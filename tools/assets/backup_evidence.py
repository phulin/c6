#!/usr/bin/env python3
"""Preserve hash-gated original DB backup ABI evidence outside the public repo."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess

DLL_SHA = 'e6577e873f946224df65c02f6c4bc3b385607e2a82d957b0fd613e01cbe78b0c'


def read_rva(data, rva, length):
    pe = struct.unpack_from('<I', data, 0x3c)[0]
    section = pe + 24 + struct.unpack_from('<H', data, pe + 20)[0]
    for index in range(struct.unpack_from('<H', data, pe + 6)[0]):
        size, va, raw_size, offset = struct.unpack_from('<IIII', data, section + 40 * index + 8)
        if va <= rva and rva + length <= va + raw_size:
            return data[offset + rva - va:offset + rva - va + length]
    raise ValueError('unbacked RVA')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('database_dll', type=Path)
    parser.add_argument('--output-dir', required=True, type=Path)
    args = parser.parse_args()
    data = args.database_dll.read_bytes()
    if hashlib.sha256(data).hexdigest() != DLL_SHA:
        parser.exit(1, 'original DatabaseDLL SHA-256 gate failed\n')
    if args.output_dir.resolve().is_relative_to(Path(__file__).resolve().parents[2]):
        parser.exit(1, 'raw evidence must remain outside public source\n')
    if struct.unpack('<Q', read_rva(data, 0xb8578 + 0xc0, 8))[0] != 0x180002670:
        parser.exit(1, 'original backup vtable contract changed\n')
    args.output_dir.mkdir(parents=True, exist_ok=True)
    records = []
    for start, end in ((0x2670, 0x2700), (0x1020, 0x10fb), (0x141c0, 0x14240)):
        path = args.output_dir / f'database-{start:x}-{end:x}.asm'
        with path.open('w') as stream:
            subprocess.run(['objdump', '-d', '-M', 'intel',
                            f'--start-address={0x180000000 + start:#x}',
                            f'--stop-address={0x180000000 + end:#x}',
                            str(args.database_dll)], stdout=stream, check=True)
        records.append({'start_rva': hex(start), 'end_rva_exclusive': hex(end),
                        'filename': path.name, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()})
    result = {'database_dll_sha256': DLL_SHA, 'vtable_rva': '0xb8578', 'copy_slot': '0xc0',
              'copy_rva': '0x2670', 'abi': 'bool(destination_connection, source_connection)',
              'source_raw_connection_slot': '0x130', 'sqlite_database_name': 'main',
              'sqlite_backup_init_rva': '0x141c0', 'sqlite_backup_step_rva': '0x14900',
              'sqlite_backup_finish_rva': '0x151b0', 'page_count_argument': -1,
              'return_test': 'destination sqlite error code equals zero after backup finish',
              'ranges': records}
    (args.output_dir / 'manifest.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'ranges': len(records), 'output_dir': str(args.output_dir)}))


if __name__ == '__main__':
    main()
