#!/usr/bin/env python3
"""Check original UnitOperation body writes; this never executes a game action."""
import argparse
import ctypes
import hashlib
import itertools
import json
from pathlib import Path
import random
import struct
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / 'evidence/actions/skip-turn-contract.json'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dll', type=Path, default=Path('/home/phulin/c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll'))
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    contract = json.loads(CONTRACT.read_text())
    if digest(args.dll) != contract['dll_sha256']:
        raise SystemExit('Unsupported DLL SHA-256; original code was not loaded')
    host = ROOT / 'native/oracle/image_host.c'
    adapter = Path(__file__).with_name('actions_message_host.c')
    with tempfile.TemporaryDirectory(prefix='c6-action-message-') as temporary:
        library = Path(temporary) / 'message.so'
        subprocess.run(['cc', '-O2', '-Wall', '-Wextra', '-Werror', '-fPIC', '-shared', '-DC6_SOURCE_SHA256="' + digest(host) + '"', '-o', str(library), str(host), str(adapter)], check=True)
        lib = ctypes.CDLL(str(library))
        lib.c6_load.argtypes = [ctypes.c_char_p]
        if lib.c6_load(str(args.dll).encode()):
            raise SystemExit('Original image mapping failed')
        lib.c6_action_body.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
        rng = random.Random(0x2c681908)
        edges = [0, 1, 0x7fff, 0x8000, 0xffff]
        cases = itertools.chain(
            ((a, b, c, c, c, c, c) for a, b, c in itertools.product(edges, edges, [0, 1, 0x7fffffff, 0x80000000, 0xffffffff])),
            ((rng.getrandbits(16), rng.getrandbits(16), *(rng.getrandbits(32) for _ in range(5))) for _ in range(10000)))
        count = 0
        for fields in cases:
            message = bytearray(rng.getrandbits(8) for _ in range(80))
            struct.pack_into('<HHI', message, 0x18, *fields[:3])
            struct.pack_into('<II', message, 0x20, *fields[3:5])
            message[0x28:0x30] = bytes(8)  # bounded null parameter branch
            struct.pack_into('<I', message, 0x38, fields[5])
            struct.pack_into('<I', message, 0x48, fields[6])
            source = ctypes.create_string_buffer(bytes(message))
            output = ctypes.create_string_buffer(24)
            status = lib.c6_action_body(source, output)
            expected = struct.pack('<HHIIIII', *fields)
            if status or output.raw != expected or source.raw[:80] != message:
                raise AssertionError({'fields': fields, 'status': status, 'actual': output.raw.hex(), 'expected': expected.hex()})
            count += 1
        # Reject unsupported polymorphic parameters before calling original code.
        invalid = bytearray(80)
        invalid[0x28] = 1
        assert lib.c6_action_body(ctypes.create_string_buffer(bytes(invalid)), ctypes.create_string_buffer(24)) == -2
        report = {'schema': 1, 'dll_sha256': contract['dll_sha256'], 'configuration_sha256': digest(CONTRACT), 'host_source_sha256': digest(host), 'adapter_source_sha256': digest(adapter), 'adapter_binary_sha256': digest(library), 'cases': count, 'mismatches': 0, 'writes_per_body': [2, 2, 4, 4, 4, 4, 4], 'body_bytes': 24, 'source_unchanged': True, 'scope': 'Original UnitOperation::PutInBufferBody with synthetic object and null parameters; no CanStart, Start, queue, message envelope, initialized unit, or gameplay parity tested'}
    rendered = json.dumps(report, indent=2) + '\n'
    if args.output:
        args.output.write_text(rendered)
    print(rendered, end='')


if __name__ == '__main__':
    main()
