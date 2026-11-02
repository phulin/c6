#!/usr/bin/env python3
"""Preserve bounded Linux-only backup/bootstrap ABI evidence privately."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

HOST='106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
RANGES=((0x395cb6c,0x395cc06),(0x303cd40,0x303cd45),(0x3041980,0x3041985),
        (0x303ce90,0x303d1b0),(0x3041ab0,0x3041d10),(0x3c41283,0x3c412a0),
        (0x2cc7d98,0x2cc7d9d),(0x2cca898,0x2ccaa80),(0x2cae9c0,0x2caeb44))


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--host',type=Path,default=Path('../c6-assets-linux/Civ6'))
    p.add_argument('--output-dir',type=Path,default=Path('../c6-ghidra/evidence/linux-rules/abi'))
    a=p.parse_args()
    if hashlib.file_digest(a.host.open('rb'),'sha256').hexdigest()!=HOST:p.exit(1,'Linux host identity mismatch\n')
    if a.output_dir.resolve().is_relative_to(Path(__file__).resolve().parents[2]):p.exit(1,'raw output must stay private\n')
    a.output_dir.mkdir(parents=True,exist_ok=True)
    records=[]
    for start,end in RANGES:
        path=a.output_dir/f'host-{start:x}-{end:x}.asm'
        with path.open('w') as output:
            subprocess.run(['objdump','-d','-M','intel',f'--start-address={start:#x}',f'--stop-address={end:#x}',str(a.host)],stdout=output,check=True)
        records.append({'start':hex(start),'end_exclusive':hex(end),'filename':path.name,
                        'sha256':hashlib.sha256(path.read_bytes()).hexdigest()})
    (a.output_dir/'manifest.json').write_text(json.dumps({'host_sha256':HOST,'ranges':records},indent=2)+'\n')
    print(json.dumps({'ranges':len(records),'output':str(a.output_dir)}))


if __name__=='__main__':main()
