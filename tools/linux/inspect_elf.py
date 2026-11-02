#!/usr/bin/env python3
"""Read-only ELF identity and bounded DWARF extraction. Never executes assets."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import shutil
import struct
import subprocess


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def sections(path):
    """Read section names and sizes for little-endian ELF64 only."""
    with path.open('rb') as stream:
        header = stream.read(64)
        if header[:6] != b'\x7fELF\x02\x01':
            raise ValueError('expected little-endian ELF64')
        offset = struct.unpack_from('<Q', header, 40)[0]
        size, count, names_index = struct.unpack_from('<HHH', header, 58)
        stream.seek(offset)
        rows = [struct.unpack('<IIQQQQIIQQ', stream.read(size)) for _ in range(count)]
        names_row = rows[names_index]
        stream.seek(names_row[4])
        names = stream.read(names_row[5])
        return {names[row[0]:].split(b'\0', 1)[0].decode(): {'address': hex(row[3]), 'bytes': row[5], 'offset': row[4]} for row in rows}


def run(args):
    return subprocess.run(args, check=True, text=True, capture_output=True).stdout


def inventory(root):
    source = Path(__file__).resolve().parents[1] / 'analysis/inventory_elf.py'
    spec = importlib.util.spec_from_file_location('inventory_elf', source)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    artifacts = []
    host_symbols = set()
    host = root / 'Civ6'
    if host.exists():
        for line in module.readelf(host, '--dyn-syms').splitlines():
            fields = line.split()
            if len(fields) >= 8 and fields[6] != 'UND':
                host_symbols.add(fields[7])
    for path in sorted(root.iterdir()):
        if not path.is_file() or path.is_symlink():
            continue
        with path.open('rb') as stream:
            if stream.read(4) != b'\x7fELF':
                continue
        row = module.inventory(path, root)
        undefined = row.pop('undefined_dynamic_symbols')
        row['undefined_dynamic_symbol_count'] = len(undefined)
        row['undefined_names_also_defined_by_Civ6'] = len(set(undefined) & host_symbols)
        row['sections'] = {name: value for name, value in sections(path).items() if name.startswith('.debug_') or name in ('.init_array', '.fini_array', '.symtab', '.dynsym')}
        row['program_interpreter'] = re.findall(r'Requesting program interpreter: ([^\]]+)', module.readelf(path, '--program-headers'))
        row['initialization_tags'] = [line.strip() for line in module.readelf(path, '--dynamic').splitlines() if re.search(r'\((?:INIT|INIT_ARRAY|INIT_ARRAYSZ)\)', line)]
        artifacts.append(row)
    manifest = root / 'steamapps/appmanifest_289070.acf'
    steam = manifest.read_text() if manifest.exists() else ''
    return {'schema': 1, 'method': 'readelf and direct ELF64 section parsing; no artifact execution', 'artifacts': artifacts, 'aspyr_version': json.loads((root / 'com.aspyr.civ6.version.json').read_text()), 'steam_build_id': re.findall(r'"buildid"\s+"([^"]+)"', steam), 'steam_manifest_sha256': digest(manifest) if manifest.exists() else None, 'limits': 'Host-name overlap is not proof of symbol binding or a complete host contract. Linux identity does not establish Windows version or ABI parity.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    inv = commands.add_parser('inventory')
    inv.add_argument('root', type=Path)
    inv.add_argument('--output', type=Path, required=True)
    query = commands.add_parser('dwarf')
    query.add_argument('elf', type=Path)
    query.add_argument('--sha256', required=True, help='exact expected asset hash')
    match = query.add_mutually_exclusive_group(required=True)
    match.add_argument('--name', help='exact DW_AT_name (complete scan; accelerator find may miss entries)')
    match.add_argument('--offset', help='DIE offset from a previous query')
    match.add_argument('--statistics', action='store_true')
    query.add_argument('--depth', type=int, default=1)
    query.add_argument('--output', type=Path, required=True, help='private evidence path for proprietary debug information')
    args = parser.parse_args()
    if args.command == 'inventory':
        args.output.write_text(json.dumps(inventory(args.root.resolve()), indent=2) + '\n')
    else:
        actual = digest(args.elf)
        if actual != args.sha256:
            parser.error(f'identity mismatch: actual sha256={actual}')
        if not 0 <= args.depth <= 3:
            parser.error('depth must be 0..3')
        tool = shutil.which('llvm-dwarfdump') or shutil.which('llvm-dwarfdump-18')
        if not tool:
            parser.error('llvm-dwarfdump required')
        options = ['--statistics'] if args.statistics else ['--show-children', f'--recurse-depth={args.depth}', f'--name={args.name}' if args.name is not None else f'--debug-info={args.offset}']
        result = run([tool, *options, str(args.elf)])
        args.output.write_text(result)
        print(json.dumps({'elf_sha256': actual, 'query': options, 'output': str(args.output), 'output_sha256': digest(args.output), 'bytes': len(result.encode())}))


if __name__ == '__main__':
    main()
