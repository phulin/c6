#!/usr/bin/env python3
"""Extract fingerprint-gated ScriptSystem instruction evidence into private tree."""
import hashlib
import json
from pathlib import Path
import struct
import subprocess
from bootstrap_services_evidence import at_rva

ROOT = Path(__file__).resolve().parents[2]
EXPECTED = 'e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63'
RANGES = [(0x1615e0,0x161674),(0x161810,0x161879),(0x161a90,0x161ca7),(0x161d00,0x161dbc),(0x1c9880,0x1c98f8),(0x4d4c30,0x4d4ced),(0x4d4cf0,0x4d4d3f),(0x971a40,0x971b2f),(0x971e60,0x971f62),(0x972af0,0x972c3a),(0x974390,0x9744a8),(0xf607c0,0xf607e8),(0x2a0b40,0x2a0d00),(0x2a3440,0x2a3535),(0x64660,0x6466c)]
def main():
    binary = ROOT.parent/'c6-assets/Base/Binaries/Win64Steam/CivilizationVI.exe'
    blob = binary.read_bytes()
    if hashlib.sha256(blob).hexdigest() != EXPECTED:
        raise SystemExit('unsupported original executable')
    output = ROOT.parent/'c6-ghidra/evidence/bootstrap/script-system'
    output.mkdir(parents=True, exist_ok=True)
    with (output/'original-exe.asm').open('w') as sink:
        for begin,end in RANGES:
            subprocess.run(['objdump','-d','-M','intel',f'--start-address={0x140000000+begin:#x}',f'--stop-address={0x140000000+end:#x}',str(binary)],stdout=sink,check=True)
    vtables = {hex(rva):[hex(value-0x140000000) for value in struct.unpack('<12Q',at_rva(blob,rva,96))] for rva in [0xfc0618,0xfe9cf8]}
    (output/'identities.json').write_text(json.dumps({'sha256':EXPECTED,'vtables':vtables,'ranges':[[hex(a),hex(b)] for a,b in RANGES]},indent=2)+'\n')
    print(output)
if __name__ == '__main__':
    main()
