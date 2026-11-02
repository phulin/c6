#!/usr/bin/env python3
"""Audit the exact Linux XP2 average-damage boundary; no native execution."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess

from inspect_elf import sections

ROOT = Path(__file__).resolve().parents[2]
XP2 = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
HOST = '106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
METHODS = [
    ('calculate_damage', 0x2bd8980, 0x1fe),
    ('construct_parameters', 0x2be20e0, 0x3e),
    ('global_max_extra_damage', 0x2be20d0, 7),
    ('get_parameters', 0x1fe4cb0, 11),
    ('get_game', 0x1fcc320, 11),
    ('fixed_multiply', 0x1fceb20, 0x2c),
    ('fixed_subtract', 0x1fe4aa0, 0x24),
    ('fixed_divide', 0x1fe55e0, 0x2d),
    ('fixed_as_float', 0x1fe2170, 0x1f),
    ('fixed_as_int', 0x1fccdb0, 6),
    ('fixed_assign_int', 0x1fcdc10, 9),
    ('fixed_equal', 0x1fe5230, 8),
    ('fixed_greater', 0x1fe2b30, 8),
    ('game_rand_num', 0x1fe5240, 0x27),
    ('fixed_default_constructor', 0x1fceb60, 7),
    ('fixed_assign_copy', 0x1fcdc00, 8),
]
CORE_BINDINGS = [
    (0x1ebacf0, 0x3b80560, 0x1fe4cb0), (0x1f69340, 0x3bd7888, 0x2be20d0),
    (0x1f567e0, 0x3bce2d8, 0x1fcc8b0), (0x1f24c70, 0x3bb5520, 0x1fe5230),
    (0x1f0ac40, 0x3ba8508, 0x1fe4aa0), (0x1e67fc0, 0x3b56ec8, 0x1fceb20),
    (0x1f44d60, 0x3bc5598, 0x1fe2170), (0x1e6bce0, 0x3b58d58, 0x1fe2b30),
    (0x1dff260, 0x3b22818, 0x1fe55e0), (0x1e2efc0, 0x3b3a6c8, 0x1fcdc00),
    (0x1edd680, 0x3b91a28, 0x1fccdb0), (0x1e4f8f0, 0x3b4ab60, 0x1fceb60),
    (0x1f3afa0, 0x3bc06b8, 0x1fcdc10),
    (None, 0x3b20fc8, 0x3c051b0), (None, 0x3b0ef18, 0x3c051a8),
]
HOST_WEAK = [
    ('fixed_int_constructor', 0x1fcc8b0, 0x32ff470, 5),
    ('fixed_default_constructor', 0x1fceb60, 0x321e840, 7),
    ('fixed_assign_copy', 0x1fcdc00, 0x32170a0, 8),
    ('fixed_assign_int', 0x1fcdc10, 0x32ff550, 9),
    ('fixed_as_float', 0x1fe2170, 0x3242ee0, 0x1f),
    ('fixed_greater', 0x1fe2b30, 0x32ff460, 8),
]
FIELDS = [('xAttackerStrength', 0), ('xDefenderStrength', 4),
          ('iDefenderMaxHitPoints', 8), ('iOuterDefenseDamage', 12),
          ('iOuterDefenseMaxHitPoints', 16), ('eDefenseType', 20),
          ('xPowerScaling', 24), ('iMinimumDamage', 28), ('iBaseDamage', 32),
          ('iPowerDampener', 36), ('iMaxExtraDamage', 40),
          ('bUseAverage', 44), ('bBypassDefenses', 45)]


def digest(path):
    with Path(path).open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def host_dependencies(host, module, private_dir):
    if digest(host) != HOST:
        raise ValueError('unsupported original Linux host SHA-256')
    host_sections, core_sections = sections(host), sections(module)
    def read(path, layout, section, address, size):
        row = layout[section]
        with path.open('rb') as stream:
            stream.seek(address - int(row['address'], 16) + row['offset'])
            value = stream.read(size)
        if len(value) != size:
            raise ValueError('incomplete original code/constants')
        return value
    host_symbols = {}
    for line in subprocess.check_output(['nm', '-D', '-S', '--defined-only', str(host)], text=True).splitlines():
        row = line.split()
        if len(row) == 4:
            host_symbols[int(row[0], 16)] = (row[3], int(row[1], 16))
    methods = []
    for name, core, address, size in HOST_WEAK:
        symbol, actual_size = host_symbols[address]
        if actual_size != size:
            raise ValueError('original host dependency boundary differs: ' + name)
        code = read(host, host_sections, '.text', address, size)
        original = read(module, core_sections, '.text', core, size)
        if name == 'fixed_int_constructor':
            if code[0] != 0xe9 or address+5+struct.unpack('<i', code[1:])[0] != 0x32ff550:
                raise ValueError('host int constructor no longer tail-calls audited host integer assignment')
            proof = 'Direct tail jump to independently byte-identical host signed integer assignment'
        elif name == 'fixed_as_float':
            if code[:0x16] + code[0x1a:] != original[:0x16] + original[0x1a:]:
                raise ValueError('host Q8 float arithmetic differs from target instructions')
            constant = address+0x1a+struct.unpack('<i', code[0x16:0x1a])[0]
            if constant != 0x45f26c0 or struct.unpack('<f', read(host, host_sections, '.rodata', constant, 4))[0] != 1/256:
                raise ValueError('host Q8 fractional constant differs')
            proof = 'Identical instruction bytes after normalizing read-only constant displacement; exact Q8 1/256 constant'
        else:
            if code != original:
                raise ValueError('host primitive instructions differ from target: ' + name)
            proof = 'Complete instruction bytes identical to supplied XP2 helper'
        raw = subprocess.check_output(['objdump', '-d', '-C', f'--start-address={address}',
                                       f'--stop-address={address+size}', str(host)])
        (private_dir/(f'host-{name}.asm')).write_bytes(raw)
        methods.append(dict(name=name, host_absolute_va=hex(address), target_offset=hex(core), size=size,
                            symbol=symbol, sha256=hashlib.sha256(code).hexdigest(), proof=proof))
    return dict(module_sha256=HOST, image_type='ET_EXEC; VAs are absolute, not GameCore-relative offsets',
                methods=methods, runtime_binding_observed=False)


def audit(module, private_dir, host=None):
    if digest(module) != XP2:
        raise ValueError('unsupported original Linux XP2 SHA-256')
    private_dir.mkdir(parents=True, exist_ok=True)
    host_contract = host_dependencies(host or ROOT.parent/'c6-assets-linux/Civ6', module, private_dir)
    symbols = {}
    for line in subprocess.check_output(['nm', '-D', '-S', '--defined-only', str(module)], text=True).splitlines():
        values = line.split()
        if len(values) == 4:
            symbols[int(values[0], 16)] = (values[3], int(values[1], 16))
    relocations = subprocess.check_output(['readelf', '-rW', str(module)], text=True)
    bindings = []
    for name, plt, got in [('expf', 0x1f944f0, 0x3bed160), ('floorf', 0x1f22960, 0x3bb4398)]:
        if not re.search(r'^0*' + f'{got:x}' + r'\s+\S+\s+R_X86_64_JUMP_SLOT\s+\S+\s+' + name + r' \+ 0$', relocations, re.M):
            raise ValueError('original arithmetic runtime relocation differs: ' + name)
        bindings.append(dict(name=name, plt=hex(plt), got=hex(got)))
    core_bindings = []
    for plt, got, target in CORE_BINDINGS:
        name = symbols[target][0]
        kind = 'JUMP_SLOT' if plt is not None else 'GLOB_DAT'
        if not re.search(r'^0*' + f'{got:x}' + r'\s+\S+\s+R_X86_64_' + kind + r'\s+\S+\s+' + re.escape(name) + r' \+ 0$', relocations, re.M):
            raise ValueError('original core relocation differs: ' + name)
        core_bindings.append(dict(name=name, plt=hex(plt) if plt else None, got=hex(got), target=hex(target)))
    helper_source = (ROOT/'native/linux/combat_kernel.c').read_text()
    array = helper_source.split('core_bindings[] = {', 1)[1].split('};', 1)[0]
    expected_pairs = [(hex(got), hex(target)) for _, got, target in CORE_BINDINGS]
    if re.findall(r'\{(0x[0-9a-f]+), (0x[0-9a-f]+)\}', array) != expected_pairs:
        raise ValueError('native runtime core binding gate differs from audited contract')
    array = helper_source.split('host_weak_bindings[] = {', 1)[1].split('};', 1)[0]
    if re.findall(r'\{(0x[0-9a-f]+), (0x[0-9a-f]+)\}', array) != [(hex(core), hex(address)) for _, core, address, _ in HOST_WEAK]:
        raise ValueError('native runtime host dependency gate differs from independently audited contract')
    section_map = sections(module)
    text = section_map['.text']
    methods = []
    with module.open('rb') as source:
        plt_section = section_map['.plt']
        for binding in bindings + [row for row in core_bindings if row['plt']]:
            address = int(binding['plt'], 16)
            source.seek(address - int(plt_section['address'], 16) + plt_section['offset'])
            instruction = source.read(6)
            if instruction[:2] != b'\xff\x25' or address + 6 + struct.unpack('<i', instruction[2:])[0] != int(binding['got'], 16):
                raise ValueError('original PLT jump differs: ' + binding['name'])
        for label, address, size in METHODS:
            symbol, actual_size = symbols[address]
            if actual_size != size:
                raise ValueError(f'{label}: target symbol size differs: {actual_size:x}')
            source.seek(address - int(text['address'], 16) + text['offset'])
            code = source.read(size)
            if len(code) != size:
                raise ValueError('incomplete target function')
            raw = subprocess.check_output(['objdump', '-d', '-C', f'--start-address={address}',
                                           f'--stop-address={address+size}', str(module)])
            (private_dir / (label + '.asm')).write_bytes(raw)
            methods.append(dict(name=label, address=hex(address), size=size, symbol=symbol,
                                sha256=hashlib.sha256(code).hexdigest()))
        constants = []
        rodata = section_map['.rodata']
        for name, address, expected in [('q8_fraction', 0x2dfc344, 1/256), ('rounding_half', 0x2dfc354, 0.5)]:
            source.seek(rodata['offset'] + address - int(rodata['address'], 16))
            value = struct.unpack('<f', source.read(4))[0]
            if value != expected:
                raise ValueError('original floating constant differs: ' + name)
            constants.append(dict(name=name, address=hex(address), value=value))
    commands = ['gdb', '-q', '-nx', '-batch', str(module)]
    for name in ['GameCore::Rules::Combat::Instance::DamageParameters',
                 'GameCore::DefenseTypes', 'FixedPointT<8>', 'GameCore::Game::Instance']:
        commands += ['-ex', f"ptype /o '{name}'"]
    dwarf = subprocess.check_output(commands, text=True, stderr=subprocess.STDOUT)
    (private_dir / 'abi.dwarf').write_text(dwarf)
    parameter_dwarf = dwarf.split('total size (bytes):   48', 1)[0]
    for name, offset in FIELDS:
        if not re.search(r'/\*\s+' + str(offset) + r'\s+\|\s+[14]\s+\*/[^\n]*\b' + name + r';', parameter_dwarf):
            raise ValueError('matching target DWARF field differs: ' + name)
    if 'DISTRICT_DEFENSE_GARRISON = 1587009065' not in dwarf:
        raise ValueError('matching target defense enum differs')
    if not re.search(r'/\*\s+232\s+\|\s+48\s+\*/[^\n]*FAutoVariable<GameCore::Random', dwarf):
        raise ValueError('game synchronous RNG wrapper differs')
    if not re.search(r'/\*\s+248\s+\|\s+32\s+\*/\s+ObjectType m_value;', dwarf):
        raise ValueError('game synchronous RNG value differs')
    return dict(schema='c6-linux-xp2-average-damage-audit-v1', module_sha256=XP2,
                matching_dwarf_sha256=hashlib.sha256(dwarf.encode()).hexdigest(), methods=methods,
                parameter_fields=[dict(name=name, offset=offset) for name, offset in FIELDS],
                parameter_size=48, sync_random_offset=248, sync_random_size=32, constants=constants,
                arithmetic_runtime_bindings=bindings,
                module_owned_runtime_bindings=core_bindings,
                host_weak_dependency_candidates=host_contract,
                required_runtime_binding_profile=dict(xp2=9, original_host=6, direct_xp2_entries=5,
                    host_image_base='0x400000', host_va_kind='absolute ET_EXEC virtual address'),
                garrison_defense_type=1587009065, native_execution=False,
                scope='Average damage arithmetic; original helper receiver unused; genuine loaded GlobalParameters required',
                conclusions=['Signed Q8 difference wraps i32; multiply uses signed i64 product then low i32 after shift 8',
                             'AsFloat splits arithmetic integer and unsigned low-byte fractional parts',
                             'Single-precision expf, multiply, add 0.5, floorf, CVTTSS2SI',
                             'Global COMBAT_MAX_EXTRA_DAMAGE / 2 truncates toward zero in average mode',
                             'Garrison damage reduces through fixed-point remaining outer defenses unless bypassed',
                             'Zero defender strength returns maximum before zero attacker returns minimum; caps follow reduction',
                             'Parameter power dampener and maximum extra damage fields are not read by this helper',
                             'Nonaverage branch draws original game synchronous RNG before both early returns; not implemented'],
                limits=['No unit/district strength construction, modifiers, attacks, legal-action or turn parity',
                        'Static evidence only; native arithmetic acceptance requires separate loaded-context corpus comparison'])


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--module', type=Path, default=ROOT.parent/'c6-assets-linux/libGameCore_XP2.so')
    ap.add_argument('--private-dir', type=Path, default=ROOT.parent/'c6-ghidra/evidence/linux-combat-kernel')
    ap.add_argument('--host', type=Path, default=ROOT.parent/'c6-assets-linux/Civ6')
    ap.add_argument('--output', type=Path, required=True)
    args = ap.parse_args()
    result = audit(args.module, args.private_dir, args.host)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(dict(output=str(args.output), methods=len(result['methods']), native_execution=False)))


if __name__ == '__main__':
    main()
