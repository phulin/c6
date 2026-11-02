#!/usr/bin/env python3
"""Fingerprint save ABI evidence and save disassembly outside the public checkout."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'analysis'))
from bootstrap_services_evidence import IDENTITIES, at_rva, pe_fields

RANGES = {
 'GameCore_XP2_FinalRelease.dll': [(0x5906e0,0x59072a),(0x463b0,0x46527),
  (0x46530,0x46679),(0x45f20,0x45f74),(0x46be0,0x46de6),(0x99e4b0,0x99e4d3),
  (0x99e750,0x99e777),(0x9a9a40,0x9a9d90),(0x9a0e50,0x9a0f31),
  (0x9a44f0,0x9a48c7),(0x9a8280,0x9a839a),(0x991940,0x9919f7),(0x9a9ec0,0x9aa2d5),
  (0x5916e0,0x5916eb),(0x472c0,0x4737e),(0x9aa2e0,0x9aa3d5)],
 'CivilizationVI.exe': [(0x169b00,0x169c23),(0x169590,0x1699d3),
  (0x169a90,0x169aff),(0x16e340,0x16f31c),(0x991c20,0x991c57),
  (0x991ca0,0x991ce1),(0x991df0,0x991edd),(0x991f20,0x9920b7)],
}

def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--assets',type=Path,default=Path('../c6-assets'))
 p.add_argument('--output',type=Path,default=Path('../c6-ghidra/evidence/saves/contracts'))
 args=p.parse_args()
 paths={'GameCore_XP2_FinalRelease.dll':args.assets/'DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll',
        'CivilizationVI.exe':args.assets/'Base/Binaries/Win64Steam/CivilizationVI.exe'}
 blobs={name:path.read_bytes() for name,path in paths.items()}
 for name,blob in blobs.items():
  if hashlib.sha256(blob).hexdigest()!=IDENTITIES[name]:
   raise SystemExit('unsupported module identity: '+name)
 # Verify actual Windows vtable entries, independently of reference Linux layouts.
 dll=blobs['GameCore_XP2_FinalRelease.dll']
 slots={hex(slot):hex(struct.unpack('<Q',at_rva(dll,0xa101e8+slot,8))[0]-0x180000000)
        for slot in [0x20,0x168,0x170]}
 if slots['0x170']!='0x5906e0' or slots['0x168']!='0x5916e0':
  raise SystemExit('unexpected original context Load slot')
 args.output.mkdir(parents=True,exist_ok=True)
 for name,ranges in RANGES.items():
  base=pe_fields(blobs[name])[0]
  with (args.output/(name+'.asm')).open('w') as out:
   for start,end in ranges:
    result=subprocess.run(['objdump','-d','-M','intel',f'--start-address={base+start:#x}',
      f'--stop-address={base+end:#x}',str(paths[name])],check=True,capture_output=True,text=True)
    if f'{base+start:x}:' not in result.stdout:
     raise ValueError('disassembly did not begin at requested instruction address')
    out.write(result.stdout)
 (args.output/'identities.json').write_text(json.dumps(dict(modules={n:IDENTITIES[n] for n in paths},
   export_context_vtable_rva='0xa101e8',verified_slots=slots),indent=2)+'\n')
 print(json.dumps(dict(output=str(args.output),verified_slots=slots)))

if __name__=='__main__':main()
