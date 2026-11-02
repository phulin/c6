#!/usr/bin/env python3
"""Preserve exact native job-set and DLL-wrapper instruction evidence privately."""
import hashlib
from pathlib import Path
import subprocess
from bootstrap_services_evidence import IDENTITIES, pe_fields

ROOT = Path(__file__).resolve().parents[2]
TARGETS = {
    'CivilizationVI.exe': ('Base/Binaries/Win64Steam', [(0x241a0, 0x24222), (0xe9469c, 0xe94760), (0xe94854, 0xe948bb), (0xe9a71a, 0xe9a72c), (0x985170, 0x9851c2), (0x985490, 0x985550), (0x985550, 0x9855c2), (0x985660, 0x985728), (0x9857b0, 0x9858a8), (0x985b20, 0x985d2e), (0x985d70, 0x985e63), (0x986010, 0x986085)]),
    'GameCore_XP2_FinalRelease.dll': ('DLC/Expansion2/Binaries/Win64', [(0x49bcda, 0x49bcf2), (0x992190, 0x9921d0), (0x9922c0, 0x992310)]),
}

def main():
    output = ROOT.parent / 'c6-ghidra/evidence/jobs'
    output.mkdir(parents=True, exist_ok=True)
    for name, (directory, ranges) in TARGETS.items():
        binary = ROOT.parent / 'c6-assets' / directory / name
        data = binary.read_bytes()
        if hashlib.sha256(data).hexdigest() != IDENTITIES[name]:
            raise SystemExit(f'unsupported native identity: {name}')
        base = pe_fields(data)[0]
        with (output / (name + '.asm')).open('w') as stream:
            for start, end in ranges:
                subprocess.run(['objdump', '-d', '-M', 'intel', f'--start-address={base + start:#x}', f'--stop-address={base + end:#x}', str(binary)], stdout=stream, check=True)
    print(f'Private bounded instruction evidence: {output}')

if __name__ == '__main__':
    main()
