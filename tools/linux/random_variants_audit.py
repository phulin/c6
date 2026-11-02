#!/usr/bin/env python3
"""Hash-gate and audit bounded Linux ScopedRandom and host PCG32 methods."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
from inspect_elf import sections

ROOT = Path(__file__).resolve().parents[2]
METHODS = {
    'libGameCore_XP2.so': [('scoped_ctor', 0x2d4d030, 11), ('scoped_reset', 0x2d4d040, 3),
                         ('scoped_get', 0x2d4d050, 23), ('scoped_seed', 0x2d4d070, 3)],
    'Civ6': [('pcg_ctor', 0x3c087b4, 11), ('pcg_seeded_ctor', 0x3c087c0, 41),
             ('pcg_seed', 0x3c087ea, 41), ('pcg_rand', 0x3c08816, 57)]}


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read_va(path, address, count):
    for row in sections(path).values():
        start = int(row['address'], 16)
        if start <= address and address + count <= start + row['bytes']:
            with path.open('rb') as stream:
                stream.seek(row['offset'] + address - start)
                value = stream.read(count)
            if len(value) == count:
                return value
    raise ValueError('unmapped original address')


def audit(assets, private_dir=None):
    gate = json.loads((ROOT/'tools/linux/runtime-gate.json').read_text())
    methods = []
    for module, entries in METHODS.items():
        path = assets/module
        if digest(path) != gate[module]:
            raise ValueError('original module SHA-256 mismatch: ' + module)
        for name, address, count in entries:
            code = read_va(path, address, count)
            methods.append(dict(name=name, module=module, address=hex(address), size=count,
                                code_sha256=hashlib.sha256(code).hexdigest()))
            if private_dir:
                private_dir.mkdir(parents=True, exist_ok=True)
                output = subprocess.check_output(['objdump', '-d', '-C',
                            f'--start-address={address}', f'--stop-address={address+count}', str(path)])
                (private_dir/(name+'.asm')).write_bytes(output)
    constants = struct.unpack('<QQ', read_va(assets/'Civ6', 0x4679360, 16))
    if constants != (0x08f5dc87e5c07d8a, 3):
        raise ValueError('host default PCG state differs')
    return dict(schema='c6-linux-random-variants-contract-v1',
                modules={name: gate[name] for name in METHODS}, methods=methods,
                default_pcg_state=constants[0], default_pcg_increment=constants[1],
                scope='Isolated ScopedRandom and host PCG32 integer constructors, seed, draw and raw state',
                native_execution=False)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent/'c6-assets-linux')
    parser.add_argument('--private-dir', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(audit(args.assets, args.private_dir), indent=2)+'\n')
