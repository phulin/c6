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
    parser.add_argument('--rust-candidate', type=Path, help='Also compare this dedicated Rust transport candidate')
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
        lib.c6_action_rejected_request.argtypes = [ctypes.c_uint16]
        for owner in [64, 65, 0x7fff, 0x8000, 0xffff]:
            if lib.c6_action_rejected_request(owner):
                raise AssertionError({'rejected_request_owner': owner})
        if lib.c6_action_rejected_request(0) != -2:
            raise AssertionError('Adapter did not guard valid owner before execution')
        rng = random.Random(0x2c681908)
        edges = [0, 1, 0x7fff, 0x8000, 0xffff]
        cases = itertools.chain(
            ((a, b, c, c, c, c, c) for a, b, c in itertools.product(edges, edges, [0, 1, 0x7fffffff, 0x80000000, 0xffffffff])),
            ((rng.getrandbits(16), rng.getrandbits(16), *(rng.getrandbits(32) for _ in range(5))) for _ in range(10000)))
        count = 0
        rust_requests = []
        native_bodies = []
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
            if args.rust_candidate:
                def signed(value):
                    return value if value < 0x80000000 else value - 0x100000000
                rust_requests.append({'owner': fields[0], 'object_type': fields[1], 'id': fields[2], 'handler': signed(fields[3]), 'secondary': signed(fields[4]), 'flags': fields[5], 'mode': signed(fields[6])})
                native_bodies.append(output.raw.hex())
            count += 1
        if args.rust_candidate:
            candidate_result = subprocess.run([str(args.rust_candidate.resolve())], input=''.join(json.dumps(r) + '\n' for r in rust_requests), text=True, capture_output=True, check=True)
            candidate_lines = candidate_result.stdout.splitlines()
            if len(candidate_lines) != count:
                raise AssertionError('Rust candidate output count differs')
            for index, line in enumerate(candidate_lines):
                actual = json.loads(line)
                if actual != {'body_hex': native_bodies[index], 'decoded': rust_requests[index]}:
                    raise AssertionError({'case': index, 'request': rust_requests[index], 'actual': actual, 'native': native_bodies[index]})
        # Reject unsupported polymorphic parameters before calling original code.
        invalid = bytearray(80)
        invalid[0x28] = 1
        assert lib.c6_action_body(ctypes.create_string_buffer(bytes(invalid)), ctypes.create_string_buffer(24)) == -2
        report = {'schema': 1, 'dll_sha256': contract['dll_sha256'], 'configuration_sha256': digest(CONTRACT), 'host_source_sha256': digest(host), 'adapter_source_sha256': digest(adapter), 'adapter_binary_sha256': digest(library), 'cases': count, 'mismatches': 0, 'writes_per_body': [2, 2, 4, 4, 4, 4, 4], 'body_bytes': 24, 'source_unchanged': True, 'invalid_owner_request_cases': 5, 'scope': 'Original UnitOperation::PutInBufferBody with synthetic object and null parameters, plus invalid-owner request early return; no CanStart, Start, queue, message envelope, initialized unit, or gameplay parity tested'}
    if args.rust_candidate:
        report['rust_candidate_binary_sha256'] = digest(args.rust_candidate)
        report['rust_codec_source_sha256'] = digest(ROOT / 'crates/c6-core/src/messages.rs')
        report['rust_candidate_source_sha256'] = digest(ROOT / 'crates/c6-differential/src/bin/c6-action-message-candidate.rs')
        report['rust_differential_cases'] = count
        report['rust_mismatches'] = 0
        report['rust_decode_fields_checked'] = True
    rendered = json.dumps(report, indent=2) + '\n'
    if args.output:
        args.output.write_text(rendered)
    print(rendered, end='')


if __name__ == '__main__':
    main()
