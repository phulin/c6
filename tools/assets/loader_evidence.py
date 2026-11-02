#!/usr/bin/env python3
"""Extract bounded original loader disassembly to the private research tree."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from content_recipe import EXE_SHA256

RANGES = [
    (0xbc330, 0xbc630), (0xbf120, 0xbf440),
    (0xbd160, 0xbd580), (0xbb850, 0xbb950),
    (0x983b40, 0x983b70), (0xc09d0, 0xc0a40),
    (0x7e91f0, 0x7e92e0), (0x987950, 0x987b20),
    (0x5390, 0x53ab), (0x5b90, 0x5bab),
    (0x100520, 0x1005bd), (0x100ea0, 0x100f3d),
    (0x16baf0, 0x16bc49), (0x7a0da0, 0x7a0e25), (0x7992c0, 0x7992fb),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', type=Path)
    parser.add_argument('--output-dir', required=True, type=Path)
    args = parser.parse_args()
    if hashlib.sha256(args.executable.read_bytes()).hexdigest() != EXE_SHA256:
        parser.exit(1, 'original executable SHA-256 gate failed\n')
    if args.output_dir.resolve().is_relative_to(Path(__file__).resolve().parents[2]):
        parser.exit(1, 'raw proprietary disassembly must stay outside public repository\n')
    args.output_dir.mkdir(parents=True, exist_ok=True)
    records = []
    for start, end in RANGES:
        path = args.output_dir / f'host-{start:x}-{end:x}.asm'
        with path.open('w') as stream:
            subprocess.run(['objdump', '-d', '-M', 'intel',
                            f'--start-address={0x140000000 + start:#x}',
                            f'--stop-address={0x140000000 + end:#x}',
                            str(args.executable)], stdout=stream, check=True)
        records.append({'start_rva': hex(start), 'end_rva_exclusive': hex(end),
                        'path': path.name, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()})
    manifest = {'executable_sha256': EXE_SHA256, 'image_base': '0x140000000',
                'ranges': records, 'purpose': 'base database source load order and original XML/SQL importer'}
    (args.output_dir / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'ranges': len(records), 'output_dir': str(args.output_dir)}))


if __name__ == '__main__':
    main()
