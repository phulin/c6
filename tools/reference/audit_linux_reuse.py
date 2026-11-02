#!/usr/bin/env python3
"""Compare pinned public reference names/addresses with the identity-gated Linux host.

Read-only static comparison. It neither executes the game nor establishes an ABI.
"""
import argparse
import collections
import hashlib
import json
from pathlib import Path
import re
import subprocess

HOST_SHA256 = '106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
COMMIT = '5af389445118bed2bdebf7a8a42b219c8fba5175'


def sha256(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def audit(host, upstream):
    actual = sha256(host)
    if actual != HOST_SHA256:
        raise ValueError(f'host identity mismatch: {actual}')
    revision = subprocess.check_output(['git', '-C', str(upstream), 'rev-parse', 'HEAD'], text=True).strip()
    if revision != COMMIT:
        raise ValueError(f'reference revision mismatch: {revision}')
    paths = ['docs/exe_function_index.json', 'findings/tables/interfaces_exe_implements.txt']
    for path in paths:
        clean = subprocess.run(['git', '-C', str(upstream), 'diff', '--quiet', COMMIT, '--', path], check=False)
        if clean.returncode:
            raise ValueError(f'reference input differs from pinned commit: {path}')
    symbols = collections.defaultdict(set)
    for line in subprocess.check_output(['nm', '-D', '-C', str(host)], text=True).splitlines():
        match = re.fullmatch(r'([0-9a-f]+)\s+\S\s+(.+)', line.strip())
        if match:
            symbols[match[2]].add(int(match[1], 16))
    counts = collections.Counter()
    different = []
    for row in json.loads((upstream / paths[0]).read_text())['rows']:
        if not row[1]:
            counts['without_linux_address'] += 1
        elif not symbols.get(row[0]):
            counts['name_not_exactly_exported'] += 1
        elif int(row[1], 16) in symbols[row[0]]:
            counts['exact_name_and_address'] += 1
        else:
            counts['same_name_different_address'] += 1
            different.append({'name': row[0], 'reference_address': row[1], 'host_addresses': sorted(hex(x) for x in symbols[row[0]])})
    named_slots = []
    for line in (upstream / paths[1]).read_text().splitlines():
        match = re.search(r'slot\s+(\d+) \(\+0x[0-9a-f]+\): ([0-9a-f]+) (.+)$', line)
        if match:
            named_slots.append(int(match[2], 16) in symbols[match[3]])
    return {'schema': 1, 'host_sha256': actual, 'reference_commit': revision,
            'reference_inputs_sha256': {p: sha256(upstream / p) for p in paths},
            'method': 'nm -D -C; demangled name to set of addresses preserves constructor/destructor aliases',
            'function_index_counts': dict(counts), 'same_name_different_address': different,
            'service_table_named_rows': len(named_slots), 'service_table_named_address_matches': sum(named_slots),
            'limits': 'Static name/address agreement only. Does not compare instructions, actual vtable pointer contents, complete ABI, library identity, initialization, or gameplay. Unmatched names are not necessarily absent functions.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('host', type=Path)
    parser.add_argument('--upstream', type=Path, default=Path('evidence/reference/upstream'))
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.write_text(json.dumps(audit(args.host, args.upstream), indent=2) + '\n')


if __name__ == '__main__':
    main()
