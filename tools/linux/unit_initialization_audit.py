#!/usr/bin/env python3
"""Fingerprint bounded current Linux Unit Read/application contracts; no execution."""
import argparse
import json
from pathlib import Path
import struct
import subprocess
from definition_ids_audit import ROOT, TARGET, HOST, elf, sha

METHODS = dict(unit_read=0x2c231f0, unit_reset=0x2c23dc0,
               initialize_definition=0x2c1c6e0, interface_post_load=0x2c1c850,
               concrete_post_load=0x2c12d00, initialize_domain=0x2c12bc0,
               definition_domain_hash=0x1fe5870, unit_hash_read=0x2d4ec80,
               abilities_overlay=0x2c2a7a0, terrain_overlay=0x2c2a8e0,
               great_person_overlay=0x2b13a80)


def audit(assets, private_output=None):
    module = assets / 'libGameCore_XP2.so'
    read = elf(module, TARGET)
    elf(assets / 'Civ6', HOST)
    symbols = {}
    for line in subprocess.check_output(['nm', '-D', '-S', str(module)], text=True).splitlines():
        fields = line.split()
        if len(fields) == 4:
            symbols.setdefault(int(fields[0], 16), []).append((fields[3], int(fields[1], 16)))
    methods = {}
    if private_output:
        private_output.mkdir(parents=True, exist_ok=True)
    for label, address in METHODS.items():
        names = symbols[address]
        sizes = {n for _, n in names}
        if len(sizes) != 1:
            raise ValueError('ambiguous function extent')
        size = sizes.pop()
        methods[label] = dict(address=hex(address), bytes=size,
                              sha256=sha(read(address, size)), symbols=sorted(s for s, _ in names))
        if private_output:
            assembly = subprocess.check_output(['objdump', '-d', '-C',
                f'--start-address={address}', f'--stop-address={address+size}', str(module)], text=True)
            (private_output / f'{label}.asm').write_text(assembly)
    # Linux vtable has two ABI prefix words and two destructor entries. Verify
    # Read's +0x28 post-load dispatch and CInstance's +0x38 domain dispatch.
    vtable = read(0x3ae5f70, 0x50)
    relocations = {}
    for line in subprocess.check_output(['readelf', '-rW', str(module)], text=True).splitlines():
        f = line.split()
        if len(f) >= 5 and f[2] == 'R_X86_64_64':
            relocations[int(f[0], 16)] = (int(f[3], 16), f[4])
    if relocations.get(0x3ae5fa8, (None,))[0] != 0x2c12d00:
        raise ValueError('Unit Read postlude virtual slot differs')
    if relocations.get(0x3ae5fb8, (None,))[0] != 0x2c12bc0:
        raise ValueError('Unit domain virtual slot differs')
    domain = read(0x2c12bc0, 0x82)
    mapping = {'0x643eada3': 0, '0x5397d46f': 1, '0xb6e1baac': 2,
               '0xc03bbd2f': 3, '0x44c22cdb': 4}
    for h in mapping:
        if struct.pack('<I', int(h, 16)) not in domain:
            raise ValueError('domain hash branch absent')
    return dict(schema='c6-linux-unit-live-initialization-v1', gamecore_sha256=TARGET,
                host_sha256=HOST, methods=methods,
                virtual_post_load=dict(vtable='0x3ae5f70', sha256=sha(vtable),
                                       read_slot='0x28', domain_slot='0x38'),
                domain_hash_to_runtime=mapping, unknown_domain=-1,
                reset_arrays=dict(abilities=0, invalid_terrain=False, ignore_terrain=False,
                                  ignore_feature=False, great_person_points=0,
                                  great_person_strength=0),
                overlay=dict(unknown='consume_without_write', duplicate='last_resolved_write',
                             untouched='preserve_destination'),
                acceptance='static-source-only', native_comparison=False,
                complete_unit_initialization=False, full_state_action_turn_parity=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--private-output', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = audit(args.assets, args.private_output)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')


if __name__ == '__main__':
    main()
