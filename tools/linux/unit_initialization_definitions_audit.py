#!/usr/bin/env python3
"""Fingerprint source-derived Unit array definition builders; no game execution."""
import argparse
import json
from pathlib import Path
import re
import struct
import subprocess
from definition_ids_audit import ROOT, TARGET, HOST, elf, sha

COLLECTIONS = {
    'UnitAbility': ('UnitAbilities', 'UnitAbilityType', 0x221c410, 0x225f580, 6),
    'Terrain': ('Terrains', 'TerrainType', 0x2219e80, 0x225e380, 13),
    'Feature': ('Features', 'FeatureType', 0x21f9150, 0x22517e0, 12),
    'GreatPersonClass': ('GreatPersonClasses', 'GreatPersonClassType', 0x21fc580, 0x2253be0, 5),
}


def audit(assets, private):
    module = assets / 'libGameCore_XP2.so'
    read = elf(module, TARGET)
    elf(assets / 'Civ6', HOST)
    methods = {}
    rows = []
    for line in subprocess.check_output(['nm', '-D', '-S', '-C', str(module)], text=True).splitlines():
        f = line.split(maxsplit=3)
        if len(f) == 4 and re.fullmatch('[0-9a-fA-F]+', f[0]) and re.fullmatch('[0-9a-fA-F]+', f[1]):
            rows.append((int(f[0], 16), int(f[1], 16), f[3]))
    private.mkdir(parents=True, exist_ok=True)
    result = {}
    for kind, (table, column, getter, builder, type_column) in COLLECTIONS.items():
        code = read(getter, 8)
        if code[:3] != b'\x48\x8d\x05' or code[-1] != 0xc3:
            raise ValueError('SQL getter changed')
        sql = read(getter + 7 + struct.unpack_from('<i', code, 3)[0], 4096).split(b'\0')[0]
        text = sql.decode()
        match = re.fullmatch(r'SELECT (.+) FROM ' + table + r' ORDER BY rowid', text)
        if not match or match[1].split(', ')[type_column] != column:
            raise ValueError('SQL filter/order/type column changed')
        selected = []
        for address, size, symbol in rows:
            direct = f'GameCore::Definition::{kind}::'
            collection = f'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::{kind}>::'
            row_builder = f'GameCore::Definition::{kind}CollectionBuilder::'
            wanted = ((symbol.startswith(direct) and any(x in symbol for x in [f'Set{column}(', 'SetCollectionIndex(', 'GetTypeHash()']))
                      or (collection in symbol and any(x in symbol for x in ['Find(unsigned int)', 'FindIndex(unsigned int)', 'push_back(']))
                      or symbol.startswith(row_builder)
                      or (f'BuildCollection<GameCore::Definition::{kind}CollectionBuilder,' in symbol))
            if not wanted:
                continue
            selected.append(address)
            key = hex(address)
            if key not in methods:
                methods[key] = dict(bytes=size, sha256=sha(read(address, size)), symbol=symbol)
                assembly = subprocess.check_output(['objdump', '-d', '-C', f'--start-address={address}', f'--stop-address={address+size}', str(module)], text=True)
                (private / f'{address:x}.asm').write_text(assembly)
        assembly = (private / f'{builder:x}.asm').read_text()
        sequence = [f'{kind}CollectionBuilder::BuildDefinition(', '::size() const', '::SetCollectionIndex(', '::push_back(']
        offsets = [assembly.index(s) for s in sequence]
        if offsets != sorted(offsets):
            raise ValueError('append-index construction changed')
        result[kind] = dict(table=table, type_column=column, sql_sha256=sha(sql),
                            sql_bytes=len(sql), sql_getter=hex(getter), type_column_index=type_column,
                            filter='none', order='signed intrinsic rowid ascending',
                            collection_index='size before append', methods=[hex(a) for a in selected])
    return dict(schema='c6-linux-unit-initialization-definition-builders-v1',
                gamecore_sha256=TARGET, host_sha256=HOST, collections=result, methods=methods,
                native_execution=False, complete_unit_initialization=False,
                full_state_action_turn_parity=False)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-output', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    result = audit(a.assets, a.private_output)
    a.output.parent.mkdir(parents=True, exist_ok=True)
    a.output.write_text(json.dumps(result, indent=2, sort_keys=True)+'\n')


if __name__ == '__main__':
    main()
