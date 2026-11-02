#!/usr/bin/env python3
"""SHA-gated static scalar/container contracts for current Linux Unit writers.

No native execution, save acceptance or gameplay parity is claimed.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
from unit_archive_audit import disassemble

PLAIN = ['DirectionTypes', 'GreatWorkListIndex', 'HiddenVisibilityTypes',
         'MilitaryFormationTypes', 'PlayerTypes', 'RelicSource', 'UnitAITypes',
         'UnitActivityTypes', 'UnitAutomateTypes', 'YieldTypes']
HASH = ['BuildingTypes', 'DistrictTypes', 'GreatPersonClassTypes',
        'GreatPersonIndividualTypes', 'HeroClassTypes', 'ReligionTypes',
        'TechTypes', 'UnitOperationTypes', 'UnitTypes', 'ImprovementTypes',
        'ResourceTypes', 'RouteTypes', 'WMDTypes']
HELPERS = {
    'Unit.PlayerEnumToStream': (0x2c29f30, 0x16, 'i32 payload at FAutoVariable+0x10; no wrapper header'),
    'Unit.UnitTypes.save': (0x2c1dbc0, 0xf, 'payload at FAutoVariable+0x10; direct UnitTypes writer'),
    'Unit.UnitAITypes.save': (0x2c1e5c0, 0xf, 'payload at FAutoVariable+0x10; direct UnitAITypes writer'),
    'Unit.DirectionTypes.save': (0x2c1fa00, 0xf, 'payload at FAutoVariable+0x10; direct DirectionTypes writer'),
    'Unit.PlayerTypes.save': (0x2c20400, 0xf, 'payload at FAutoVariable+0x10; direct PlayerTypes writer'),
    'Unit.MilitaryFormationTypes.save': (0x2c2d670, 0xf, 'payload at FAutoVariable+0x10; direct MilitaryFormationTypes writer'),
    'Unit.UnitAutomateTypes.save': (0x2c13080, 0xf, 'payload at FAutoVariable+0x10; direct UnitAutomateTypes writer'),
    'Unit.UnitActivityTypes.save': (0x2c13a80, 0xf, 'payload at FAutoVariable+0x10; direct UnitActivityTypes writer'),
    'Sight.PlotEntry.vector': (0x2c38680, 0x120, 'u32 count; count PlotEntry records in begin/end order'),
    'EspionageBonus.vector': (0x2c167a0, 0x60, 'u32 count; count EspionageBonus records in begin/end order'),
    'EspionageBonus.count': (0x2c16c90, 0x38, 'u32(size()), low 32 bits'),
    'EspionageTimeReduction.vector': (0x2c16800, 0x60, 'u32 count; count EspionageTimeReduction records in begin/end order'),
    'EspionageTimeReduction.count': (0x2c16d40, 0x38, 'u32(size()), low 32 bits'),
    'InitiationYieldPopulation.vector': (0x2c442d0, 0x60, 'u32 count; count InitiationYieldPopulation records in begin/end order'),
    'InitiationYieldPopulation.count': (0x2c444f0, 0x38, 'u32(size()), low 32 bits'),
    'Vector.int': (0x1feff10, 0x100, 'u32 count; count i32 in begin/end order'),
    'Vector.int.count': (0x1feffc0, 0x30, 'u32(size())'),
    'YieldIndexList': (0x2194800, 0x38, 'Yield definition hash array of i32'),
    'OperationIndexList.bool': (0x2c3fc20, 0x40, 'UnitOperation definition hash array of one-byte bool'),
    'YieldHashArray': (0x2003140, 0x8d, 'u32 definition count; (u32 hash, i32 value) for every At(i)'),
    'OperationHashArray.bool': (0x2c3fc60, 0x8a, 'u32 definition count; (u32 hash, u8 bool) for every At(i)'),
    'AbilityHashArray': (0x2c2a070, 0x8d, 'u32 definition count; (u32 hash, i32 value) for every At(i)'),
    'TerrainHashArray.bool': (0x2c2a100, 0x8a, 'u32 definition count; (u32 hash, u8 bool) for every At(i)'),
    'FeatureHashArray.bool': (0x285ebb0, 0x8a, 'u32 definition count; (u32 hash, u8 bool) for every At(i)'),
    'GreatPersonClassHashArray': (0x2b13990, 0x8d, 'u32 definition count; (u32 hash, i32 value) for every At(i)'),
    'PromotionHashTable.u8': (0x2c2a190, 0xc0, 'u32 table size; (u32 key, u8 value) in native forward-iterator order; not definition count and not bool'),
    'Unit.IndexToHash': (0x2005d70, 0x43, 'index below count -> At(index).type_hash; otherwise preserve input if Find(input) exists; otherwise 0xffffffff'),
    'Unit.HashToIndex': (0x1ffc120, 0x22, 'Find(hash).collection_index, otherwise -1'),
    'UnitID': (0x2c9adc0, 0x5f, 'i16 owner; i16 component type; u32 ID'),
    'CityID': (0x2c9ad10, 0x5f, 'i16 owner; i16 component type; u32 ID'),
    'PlotCoord': (0x2c9af20, 0x29, 'i32 x; i32 y'),
    'FixedPoint8': (0x2c9afe0, 0x2a, 'i32 raw fixed-point bits'),
    'String.Encoding1': (0x264bed0, 0x90, 'u32 GetLength(); exactly that many raw bytes from string buffer; no terminator'),
    'StaticArray.int5': (0x2c48940, 0x90, 'five i32 values, no count'),
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--gamecore', type=Path, required=True)
    parser.add_argument('--host', type=Path, required=True)
    parser.add_argument('--symbols', type=Path, help='Optional nm -C output cache')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--private-disassembly', type=Path)
    args = parser.parse_args()
    game = Elf(args.gamecore, GAME_SHA)
    host = Elf(args.host, HOST_SHA)
    symbols = (args.symbols.read_text() if args.symbols else
               subprocess.check_output(['nm', '-C', str(args.gamecore)], text=True))
    private = []

    def record(path, elf, address, size, contract):
        rows = disassemble(path, address, size)
        private.append(f'{address:#x} {contract}\n' + '\n'.join(f'{pc:x} {ins}' for pc, ins in rows))
        return {'address': hex(address), 'size_bound': size,
                'sha256': hashlib.sha256(elf.read(address, size)).hexdigest(),
                'wire': contract,
                'calls': [ins.split('<', 1)[1][:-1].removesuffix('@plt')
                          for _, ins in rows if ins.startswith(('call', 'jmp')) and '<' in ins]}

    enums = {}
    for name in sorted(PLAIN + HASH):
        full = 'Serialization::operator<<(Serialization::OStream&, GameCore::' + name + ' const&)'
        match = re.search(r'^([0-9a-f]+) [TW] ' + re.escape(full) + r'$', symbols, re.M)
        if not match:
            raise ValueError('Missing symbol: ' + full)
        address = int(match[1], 16)
        rows = disassemble(args.gamecore, address, 0x50)
        end = next(pc + 1 for pc, ins in rows if ins == 'ret')
        item = record(args.gamecore, game, address, end - address,
                      'u32 definition hash' if name in HASH else 'i32 enum ordinal')
        if name in HASH:
            assert any('WriteIndexToHash<' in call for call in item['calls'])
            getter = next(call for call in item['calls'] if 'Definitions::Get' in call)
            item['definition_getter'] = getter
        else:
            assert any('operator<< <int>' in call for call in item['calls'])
        enums[name] = item
    helpers = {name: record(args.gamecore, game, a, s, wire)
               for name, (a, s, wire) in HELPERS.items()}
    widths = {name: record(args.host, host, a, 14, wire) for name, a, wire in [
        ('bool', 0x2fe34f0, 'one raw source byte'),
        ('u8', 0x33b0420, 'one raw source byte'),
        ('i16', 0x3962650, 'two raw source bytes'),
        ('i32', 0x2fdd7c0, 'four raw source bytes'),
        ('u32', 0x2f66f20, 'four raw source bytes')]}
    for name, item in widths.items():
        size = {'bool': 1, 'u8': 1, 'i16': 2, 'i32': 4, 'u32': 4}[name]
        assert any(f'$0x{size:x},%edx' in ins for _, ins in disassemble(args.host, int(item['address'], 16), 14))
    result = {'schema': 'c6-linux-unit-wire-scalars-v1', 'gamecore_sha256': GAME_SHA,
              'host_sha256': HOST_SHA, 'static_only': True, 'native_parity_validated': False,
              'complete_unit_codec': False, 'byte_order': 'little-endian raw primitive bytes',
              'enums': enums, 'helpers': helpers, 'host_primitive_widths': widths,
              'limits': ['Current writer contracts only; historical reader version branches not covered.',
                         'Definition identities and counts require exact Linux rules inputs.',
                         'HashTable forward order is state dependent; sorting keys is not evidenced.',
                         'Static evidence does not establish complete save/state/action/turn parity.']}
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    if args.private_disassembly:
        args.private_disassembly.mkdir(parents=True, exist_ok=True)
        (args.private_disassembly / 'writers.txt').write_text('\n\n'.join(private) + '\n')
    print(json.dumps({'enums': len(enums), 'helpers': len(helpers), 'output': str(args.output)}))


if __name__ == '__main__':
    main()
