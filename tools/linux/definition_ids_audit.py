#!/usr/bin/env python3
"""Fingerprint exact Linux registry contracts and audit optional native queries."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import zlib

ROOT = Path(__file__).resolve().parents[2]
TARGET = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
HOST = '106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
METHODS = dict(yield_get=0x2214970, yield_build=0x2260c00,
               yield_build_row=0x221f850, yield_push=0x231a1a0,
               yield_push_compare=0x231a680, yield_find=0x1ff7290,
               yield_find_index=0x2b35520, yield_at=0x1fe47a0,
               yield_count=0x1fe2360, yield_type_hash=0x1ff73b0,
               yield_type_name=0x1fe47c0, yield_index=0x1fe51e0,
               yield_rowid=0x2110120, yield_set_index=0x231a190,
               yield_set_type=0x221f810, hash=0x2d577c0,
               lazy_hasher=0x2d57760, yield_hash_read=0x1ff7160,
               unit_build=0x225eec0, unit_get_sql=0x221c190)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def elf(path, expected):
    data = path.read_bytes()
    if sha(data) != expected:
        raise ValueError(f'Pinned ELF identity mismatch: {path}')
    if data[:6] != b'\x7fELF\x02\x01':
        raise ValueError('Expected little-endian ELF64')
    offset = struct.unpack_from('<Q', data, 32)[0]
    stride, count = struct.unpack_from('<HH', data, 54)
    segments = [struct.unpack_from('<IIQQQQQQ', data, offset + i * stride)
                for i in range(count)]

    def read(address, size):
        for kind, _, at, start, _, length, _, _ in segments:
            if kind == 1 and start <= address and address + size <= start + length:
                return data[at + address - start:at + address - start + size]
        raise ValueError(f'Unbacked address {address:x}')
    return read


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--runtime-report', type=Path)
    parser.add_argument('--before-save', type=Path)
    parser.add_argument('--after-save', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    module = args.assets / 'libGameCore_XP2.so'
    read = elf(module, TARGET)
    host_read = elf(args.assets / 'Civ6', HOST)
    symbols = {}
    for line in subprocess.check_output(['nm', '-D', '-S', str(module)], text=True).splitlines():
        fields = line.split()
        if len(fields) == 4:
            symbols.setdefault(int(fields[0], 16), []).append((fields[3], int(fields[1], 16)))
    methods = {}
    for label, address in METHODS.items():
        names = symbols[address]
        sizes = {size for _, size in names}
        if len(sizes) != 1:
            raise ValueError(f'Ambiguous extent: {label}')
        size = sizes.pop()
        methods[label] = dict(address=hex(address), bytes=size,
                              sha256=sha(read(address, size)), symbols=sorted(n for n, _ in names))
    sql = read(0x2e43b80, 256).split(b'\0')[0].decode()
    if sql != 'SELECT rowid, DefaultValue, IconString, Name, OccupiedCityChange, YieldType FROM Yields ORDER BY rowid':
        raise ValueError('Yield SQL contract changed')
    result = dict(schema='c6-linux-definition-ids-v1', gamecore_sha256=TARGET,
                  host_sha256=HOST, methods=methods, yield_sql=sql,
                  host_crc=dict(address='0x39508ee', bytes=40,
                                sha256=sha(host_read(0x39508ee, 40))),
                  hash_contract=dict(polynomial='0xedb88320', seed='0xffffffff',
                                     final_xor=0, nul_terminated=True, case_sensitive=True),
                  native_query=None, full_state_action_turn_parity=False)
    if args.runtime_report:
        if not args.before_save or not args.after_save:
            parser.error('Native audit requires both canonical Save captures')
        runtime = json.loads(args.runtime_report.read_text())
        config = runtime['configuration']
        canonical = json.dumps(config, sort_keys=True, separators=(',', ':')).encode()
        if sha(canonical) != runtime['configuration_sha256']:
            raise ValueError('Native configuration identity differs')
        identities = json.dumps(config.get('assets', config.get('original_assets', {})))
        if TARGET not in identities or HOST not in identities:
            raise ValueError('Missing original module/host gates')
        if 'returncode' in runtime:
            succeeded = (runtime['returncode'] == 0 and runtime.get('framework_cleanup_proven')
                         and runtime.get('client_cleanup_proven'))
            active_xp2 = any(line.startswith('C6_LINUX_GAME_LOAD module ')
                             and '/libGameCore_XP2.so context_initialized 1 ' in line
                             for line in runtime['stderr'].splitlines())
        else:
            succeeded = runtime.get('exit_code') == 0
            active_xp2 = runtime.get('active_context_module') == 'libGameCore_XP2.so'
        if not succeeded or not active_xp2:
            raise ValueError('Native runtime did not succeed with XP2')
        records = [json.loads(line.split('C6_LINUX_DEFINITION_IDS ', 1)[1])
                   for line in runtime['stderr'].splitlines() if line.startswith('C6_LINUX_DEFINITION_IDS ')]
        if len(records) != 1:
            raise ValueError('Expected exactly one native registry query')
        query = records[0]
        if query['registry'] != 'Yields' or query['count'] != len(query['entries']):
            raise ValueError('Invalid native registry envelope')
        for i, entry in enumerate(query['entries']):
            name = bytes.fromhex(entry['type_hex'])
            expected = (~zlib.crc32(name)) & 0xffffffff
            if (entry['index'], entry['hash'], entry['native_string_hash'], entry['find_index']) != (i, expected, expected, i):
                raise ValueError('Native index/hash mapping mismatch')
        if query['index_zero'] != 0 or query['at_count_is_null'] is not True or query['missing_result'] != -1:
            raise ValueError('Native lookup boundary mismatch')
        if query['empty_string_hash'] != 0xffffffff or query['lowercase_food_hash'] != (~zlib.crc32(b'yield_food')) & 0xffffffff:
            raise ValueError('Native string-hash boundary mismatch')
        before, after = args.before_save.read_bytes(), args.after_save.read_bytes()
        if 'game_saves' in runtime:
            for label, path, data in [('save', args.before_save, before),
                                       ('repeat', args.after_save, after)]:
                expected = runtime['game_saves'][label]
                if (sha(data), len(data)) != (expected['sha256'], expected['bytes']):
                    raise ValueError(f'Original Save identity differs: {path}')
        if before != after:
            raise ValueError('Original canonical core Save changed around query')
        result['native_query'] = dict(mapping=query, runtime_report_sha256=sha(args.runtime_report.read_bytes()),
                                      configuration_sha256=runtime['configuration_sha256'],
                                      source_identity=runtime['configuration'],
                                      native_activation_selection=runtime.get('native_activation_selection'),
                                      rules_snapshots=runtime.get('rules_snapshots'),
                                      audit_source_sha256=sha(Path(__file__).read_bytes()),
                                      canonical_save_sha256=sha(before), canonical_save_bytes=len(before),
                                      canonical_core_unchanged=True, full_live_state_unchanged_proven=False)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps(dict(methods=len(methods), native_query=result['native_query'] is not None)))


if __name__ == '__main__':
    main()
