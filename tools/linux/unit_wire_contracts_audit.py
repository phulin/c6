#!/usr/bin/env python3
"""SHA-gated static Linux Unit nested writer call contracts; no native execution."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
from unit_archive_audit import disassemble

COMPONENTS = {
    'HealthModifiers': (0x2c241d0, 0x60), 'HealthReligionModifiers': (0x2c242a0, 0x60),
    'CombatModifiers': (0x2c24370, 0x220), 'MovementModifiers': (0x2c24bd0, 0xb0),
    'SightModifiers': (0x2c24e30, 0x40), 'AttackRangeModifiers': (0x2c24eb0, 0x30),
    'Experience': (0x2c17db0, 0xc0), 'GreatPerson': (0x2c194b0, 0x50),
    'Religion': (0x2c43ad0, 0x100), 'RockBand': (0x2c44af0, 0x160),
    'Trade': (0x2c48500, 0x50), 'Archaeology': (0x2c11e30, 0x70),
    'Espionage': (0x2c15bc0, 0x100), 'Hero': (0x2c1bb20, 0x40),
    'Sight.PlotEntry': (0x2c458e0, 0x30),
    'EspionageBonus': (0x2c15aa0, 0x50),
    'EspionageTimeReduction': (0x2c15b40, 0x40),
    'InitiationYieldPopulation': (0x2c43db0, 0x40),
}
HELPERS = {
    'AbilityArray.write': (0x2c2a070, 0x8d), 'TerrainArray.write': (0x2c2a100, 0x8a),
    'FeatureArray.write': (0x285ebb0, 0x8a), 'GreatPersonClassArray.write': (0x2b13990, 0x8d),
    'UnitOperationArray.write': (0x2c3fc60, 0x8a),
    'AbilityArray.read': (0x2c2a7a0, 0x95), 'TerrainArray.read': (0x2c2a8e0, 0x95),
    'FeatureArray.read': (0x285ef60, 0x95), 'GreatPersonClassArray.read': (0x2b13a80, 0x95),
    'UnitOperationArray.read': (0x2c3fb20, 0x95),
    'PromotionMap.write': (0x2c2a190, 0xc0),
    'Hero.read': (0x2c1bb60, 0x87), 'HeroClass.write': (0x2d4fa40, 0x24),
    'HeroClass.read': (0x2d4fa70, 0x3d), 'HeroClass.index_hash': (0x2d514a0, 0x60),
    'CityID.write': (0x2c9ad10, 0x5f), 'CityID.read': (0x2c9ad70, 0x4c),
    'Trade.StaticArray5.write': (0x2c48940, 0x90),
    'Experience.StringEncoding1.write': (0x264bed0, 0x90),
}
EXPECTED_VERSIONS = dict(HealthModifiers=2, HealthReligionModifiers=1, CombatModifiers=28,
                         MovementModifiers=8, SightModifiers=1, AttackRangeModifiers=1,
                         Experience=7, GreatPerson=1, Religion=8, RockBand=10, Trade=2,
                         Archaeology=4, Espionage=7, Hero=5)
HOST_HELPERS = {
    'TypedVariantMap.write': (0x3961e50, 0xc4),
    'TypedVariantMap.header': (0x3971510, 0x6e),
    'TypedVariantMap.member': (0x3971580, 0xd0),
    'TypedVariant.write': (0x395ef70, 0x58),
    'ArrayVariant.write': (0x3967da0, 0x11b),
    'VariantMap.write': (0x3967ec0, 0x16c),
    'ArrayVariant.HasFlag': (0x2f63c30, 7),
    'ArrayVariant.count': (0x2f63c40, 9),
    'ArrayVariant.size': (0x2f63c50, 4),
    'TypedVariantMap.read': (0x3971330, 0x1e0),
}

def writer_fields(path, elf, address, size):
    """Bounded straight-line provenance; unsupported instructions fail closed."""
    regs = {'rsi': 0}
    consts = {}
    fields = []
    for pc, ins in disassemble(path, address, size):
        if ins == 'ret':
            break
        m = re.fullmatch(r'mov\s+%(r\w+),%(r\w+)', ins)
        if m:
            regs[m[2]] = regs.get(m[1])
            consts[m[2]] = consts.get(m[1])
        m = re.match(r'lea\s+(-?0x[0-9a-f]+)?\(%(r\w+)\),%(r\w+)$', ins)
        if m:
            v = regs.get(m[2])
            regs[m[3]] = None if v is None else v + int(m[1] or '0', 16)
            consts[m[3]] = None
        m = re.match(r'(add|sub)\s+\$(0x[0-9a-f]+),%(r\w+)$', ins)
        if m:
            value = int(m[2], 16)
            if value >= 1 << 63:
                value -= 1 << 64
            old = regs.get(m[3])
            regs[m[3]] = None if old is None else old + value * (1 if m[1] == 'add' else -1)
        m = re.search(r'lea\s+[^#]*%rip\),%(r\w+)\s+# ([0-9a-f]+)', ins)
        if m:
            regs[m[1]] = None
            consts[m[1]] = int(m[2], 16)
        if ins.startswith('call'):
            name = ins.split('<', 1)[1][:-1].removesuffix('@plt') if '<' in ins else ins
            if re.match(r'^(?:bool |void )?Serialization::', name):
                item = {'call': hex(pc), 'serializer': name}
                if consts.get('rsi') is not None:
                    item['constant_i32'] = struct.unpack('<i', elf.read(consts['rsi'], 4))[0]
                elif regs.get('rsi') is not None:
                    item['value_offset'] = hex(regs['rsi'])
                else:
                    raise ValueError(f'Unresolved source at {pc:#x}')
                fields.append(item)
            result = regs.get('rdi') if '::front() const' in name else None
            for r in ['rax', 'rdi', 'rsi', 'rdx', 'rcx', 'r8', 'r9', 'r10', 'r11']:
                regs[r] = None
                consts[r] = None
            regs['rax'] = result
    return fields

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=Path('../c6-assets-linux'))
    p.add_argument('--output', type=Path, default=Path('evidence/linux/unit-wire-contracts.json'))
    p.add_argument('--private-output', type=Path)
    a = p.parse_args()
    game_path, host_path = a.assets / 'libGameCore_XP2.so', a.assets / 'Civ6'
    game, host = Elf(game_path, GAME_SHA), Elf(host_path, HOST_SHA)
    commands = ['gdb', '-q', '-nx', '-batch', str(game_path)]
    for name in COMPONENTS:
        commands += ['-ex', "ptype /o 'GameCore::Unit::" + name.replace('.', '::') + "'"]
    commands += ['-ex', "ptype /o 'GameCore::Unit::Archaeology::Variants'"]
    dwarf = subprocess.check_output(commands, stderr=subprocess.STDOUT, text=True)
    members, owner, pending = {}, None, None
    for line in dwarf.splitlines():
        if 'type = struct GameCore::UnitArchaeology::Reflection::Variants1 {' in line:
            owner = 'Archaeology.Variants'
            members[owner] = {}
        m = re.search(r'type = (?:class|struct) GameCore::Unit::([\w:]+) \{', line)
        if m:
            owner = m[1].replace('::', '.')
            members[owner] = {}
        m = re.match(r'/\*\s+(\d+)\s+\|\s+(\d+) \*/    (?! )(.*)', line)
        if m and owner:
            pending = {'value_offset': hex(int(m[1])), 'object_bytes': int(m[2]), 'cpp_declaration': m[3]}
            n = re.search(r'\b(m_\w+);$', m[3])
            if n:
                pending['name'] = n[1]
                members[owner][pending['value_offset']] = pending
                pending = None
        m = re.match(r'                               } (m_\w+);$', line)
        if m and pending is not None:
            pending['name'] = m[1]
            members[owner][pending['value_offset']] = pending
            pending = None
    components = {}
    for name, (address, size) in COMPONENTS.items():
        fields = writer_fields(game_path, game, address, size)
        for field in fields:
            if 'value_offset' in field:
                member = members.get(name, {}).get(field['value_offset'])
                if name == 'Archaeology':
                    nested_offset = hex(int(field['value_offset'], 16) - 8)
                    nested = members.get('Archaeology.Variants', {}).get(nested_offset)
                    if nested:
                        member = dict(nested, name='m_kVariants.' + nested['name'])
                if member is None:
                    raise ValueError(f'Missing DWARF member {name} {field}')
                field.update({k: v for k, v in member.items() if k != 'value_offset'})
        if name in EXPECTED_VERSIONS:
            assert fields[0]['constant_i32'] == EXPECTED_VERSIONS[name]
        serialized_names = {f.get('name', '').split('.')[0] for f in fields}
        omitted = [m for m in members.get(name, {}).values() if m['name'] not in serialized_names]
        components[name] = {'writer': hex(address), 'writer_size_bound': size,
                            'sha256': hashlib.sha256(game.read(address, size)).hexdigest(),
                            'fields': fields, 'members_absent_from_this_writer': omitted,
                            'reader_compatibility_complete': False}
    def fingerprints(elf, methods):
        return {name: {'address': hex(addr), 'size_bound': size,
                      'sha256': hashlib.sha256(elf.read(addr, size)).hexdigest()}
                for name, (addr, size) in methods.items()}
    vt = {}
    for slot in [0x1f0, 0x1f8, 0x200, 0x208]:
        vt[hex(slot)] = hex(struct.unpack('<Q', host.read(0x53b3680 + 16 + slot, 8))[0])
    assert vt == {'0x1f0': '0x3971330', '0x1f8': '0x3961e50', '0x200': '0x3971510', '0x208': '0x3971580'}
    out = {'schema': 'c6-linux-unit-wire-contracts-v1', 'gamecore_sha256': GAME_SHA,
           'host_sha256': HOST_SHA, 'static_only': True, 'native_parity_validated': False,
           'complete_unit_codec': False, 'components': components,
           'game_helpers': fingerprints(game, HELPERS), 'host_helpers': fingerprints(host, HOST_HELPERS),
           'typed_variant_map_vtable': vt,
           'limits': ['Current writer sequences only; reader version compatibility is separately required.',
                      'Container order, live fields omitted by writers and definition index/hash identity must remain distinct.',
                      'Property recursive branches require full typed validation; no opaque advance contract.',
                      'No attributed native record comparison is performed by this static audit.']}
    a.output.parent.mkdir(parents=True, exist_ok=True)
    a.output.write_text(json.dumps(out, indent=2) + '\n')
    if a.private_output:
        a.private_output.mkdir(parents=True, exist_ok=True)
        for label, path, methods in [('game', game_path, dict(COMPONENTS, **HELPERS)), ('host', host_path, HOST_HELPERS)]:
            for name, (address, size) in methods.items():
                raw = subprocess.check_output(['objdump', '-d', '-C', f'--start-address={address}', f'--stop-address={address+size}', str(path)])
                (a.private_output / f'{label}-{name}.asm').write_bytes(raw)
        (a.private_output / 'types.dwarf').write_text(dwarf)
    print(json.dumps({'components': len(components), 'field_calls': sum(len(x['fields']) for x in components.values()), 'output': str(a.output), 'native_execution': False}))

if __name__ == '__main__':
    main()
