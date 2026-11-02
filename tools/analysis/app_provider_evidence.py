#!/usr/bin/env python3
"""Extract exact original App/provider instruction evidence privately."""
import hashlib
from pathlib import Path
import subprocess
from bootstrap_services_evidence import IDENTITIES, pe_fields

path = Path('../c6-assets/Base/Binaries/Win64Steam/CivilizationVI.exe')
blob = path.read_bytes()
if hashlib.sha256(blob).hexdigest() != IDENTITIES[path.name]:
    raise SystemExit('unsupported original EXE identity')
base = pe_fields(blob)[0]
output = Path('../c6-ghidra/evidence/script/app-provider.asm')
output.parent.mkdir(parents=True, exist_ok=True)
ranges = [(0x79c20, 0x79cd0), (0x5cb40, 0x5cbb7), (0x5ccb0, 0x5d066),
          (0x71898, 0x7190a), (0x64650, 0x6466c), (0xf02860, 0xf02865),
          (0xbf070, 0xbf116), (0xbf6a0, 0xbf768), (0xbf980, 0xbf9f1),
          (0xbfc11, 0xbfcb3), (0x1c9780, 0x1c984f)]
with output.open('w') as stream:
    for start, end in ranges:
        subprocess.run(['objdump', '-d', '-M', 'intel',
                        f'--start-address={base + start:#x}',
                        f'--stop-address={base + end:#x}', str(path)],
                       stdout=stream, check=True)
print(f'{len(ranges)} fingerprinted native instruction ranges: {output}')
