#!/usr/bin/env python3
"""Audit forwarding-only original XP2 serialization-owner interfaces; no execution."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from inspect_elf import sections

ROOT = Path(__file__).resolve().parents[2]
XP2 = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
DWARF = '1c5bcad722f0b4b3aa3b1aba197523a22e9ccbf0811e297442918ee8aacf7ef8'


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def audit(module, dwarf):
    if digest(module) != XP2:
        raise ValueError('unsupported original XP2 module SHA-256')
    if digest(dwarf) != DWARF:
        raise ValueError('matching original serialization DWARF signatures differ')
    declarations = dwarf.read_text()
    candidates = []
    for line in subprocess.check_output(['nm', '-D', '-S', str(module)], text=True).splitlines():
        p = line.split()
        if len(p) == 4 and p[3].startswith(('_ZN13SerializationlsERNS_7OStreamERKN8GameCore', '_ZN13SerializationlsERNS_7OStreamERKN11GameEffects')):
            candidates.append(dict(symbol=p[3], address=int(p[0], 16), bytes=int(p[1], 16)))
    names = subprocess.check_output(['c++filt'], input='\n'.join(x['symbol'] for x in candidates)+'\n', text=True).splitlines()
    selected = []
    for row, name in zip(candidates, names):
        if not (name.startswith(('Serialization::operator<<(Serialization::OStream&, GameCore::', 'Serialization::operator<<(Serialization::OStream&, GameEffects::')) and name.endswith(' const&)')):
            continue
        if not (row['bytes'] > 128 or '::Manager const&' in name or '::Instance const&' in name or
                'GameCore::Game::' in name or 'GameCore::AI::GlobalAi ' in name or 'GameCore::Random const&' in name):
            continue
        if 'Serialization::OStream &'+name+';' not in declarations:
            raise ValueError('original OStream reference / two reference arguments not proven: '+name)
        row['owner'] = name.removeprefix('Serialization::operator<<(Serialization::OStream&, ').removesuffix(' const&)')
        selected.append(row)
    if len(selected) != 207:
        raise ValueError('exact original serializer owner inventory differs')
    lines = ['/* Generated exact XP2 serialization owner interfaces; no original game code. */']
    text = sections(module)['.text']
    with module.open('rb') as source:
        for i, row in enumerate(selected):
            lines.append(f'OWNER({i},{json.dumps(row["symbol"])},{json.dumps(row["owner"])})')
            source.seek(row['address'] - int(text['address'], 16) + text['offset'])
            row['sha256'] = hashlib.sha256(source.read(row['bytes'])).hexdigest()
            row['address'] = hex(row['address'])
    if (ROOT/'tools/linux/save_normalization_owners.inc').read_text() != '\n'.join(lines)+'\n':
        raise ValueError('forwarding wrapper interface inventory differs from native symbols')
    return dict(schema='c6-linux-save-normalization-owner-trace-v1', module_sha256=XP2,
                matching_dwarf_sha256=DWARF, owners=selected,
                forwarding_abi='OStream&(OStream&, GameCore::<owner> const&); original return and stream behavior preserved, original MemoryOStream.GetOffset reads only',
                native_execution=False, gameplay_parity=False,
                limits='Static instrumentation interface audit. Must compare original output bytes and full cleanup against uninstrumented baseline before using observed boundaries. No field semantics or full-state equivalence inferred.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--module', type=Path, default=ROOT.parent/'c6-assets-linux/libGameCore_XP2.so')
    parser.add_argument('--dwarf', type=Path, default=ROOT.parent/'c6-ghidra/evidence/linux-save-normalization/serialization-write-functions.dwarf.txt')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = audit(args.module, args.dwarf)
    args.output.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(dict(output=str(args.output), original_interfaces_checked=len(result['owners']), native_execution=False)))


if __name__ == '__main__':
    main()
