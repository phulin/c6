#!/usr/bin/env python3
"""Fingerprint and execute the bounded original ScriptSystem child probe."""
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
    config = json.loads((ROOT/'tools/bootstrap/config.json').read_text())
    assets = ROOT.parent/'c6-assets'
    for relative, expected in config['expected_assets'].items():
        if digest(assets/relative) != expected:
            raise SystemExit(f'unsupported asset: {relative}')
    host = ROOT/'target/bootstrap/script_system_probe.exe'
    sources = ['native/bootstrap/script-system/probe.c', 'native/bootstrap/script-system/script_system.c', 'native/bootstrap/xml-import/imports.c']
    subprocess.run(['x86_64-w64-mingw32-gcc','-std=c11','-O2','-Wall','-Wextra','-Werror','-municode','-static','-Wl,--image-base,0x150000000','-o',str(host),*sources,'-lbcrypt'],cwd=ROOT,check=True)
    identity = {'schema':'c6-original-script-system-probe-v1', 'assets':config['expected_assets'], 'sources':{s:digest(ROOT/s) for s in sources}, 'host_sha256':digest(host)}
    identity_hash = hashlib.sha256(json.dumps(identity,sort_keys=True,separators=(',',':')).encode()).hexdigest()
    env = dict(os.environ,WINEPREFIX=str(ROOT/'target/bootstrap/wine-prefix'),WINEARCH='win64',WINEDEBUG='-all',WINEDLLOVERRIDES='msvcp140,vcruntime140,ucrtbase=n')
    path = lambda p: 'Z:'+str(p.resolve()).replace('/','\\')
    cases = []
    for mode in ["--lifecycle", None]:
        command = ['/usr/lib/wine/wine64', str(host), path(assets/'Base/Binaries/Win64Steam/CivilizationVI.exe'), path(assets/'Base/Binaries/Win64Steam')]
        if mode:
            command.append(mode)
        result = subprocess.run(command, cwd=host.parent, env=env, text=True, capture_output=True, timeout=60)
        cases.append({'mode':mode or 'full-script-state', 'child_exit_code':result.returncode, 'stdout':result.stdout, 'stderr':result.stderr})
    report = {'configuration':identity,'configuration_sha256':identity_hash,'cases':cases,'initialized_game':False,'full_turn_parity':False}
    (ROOT/'evidence/bootstrap/script-system.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({k:v for k,v in report.items() if k!='configuration'},indent=2))
if __name__ == '__main__':
    main()
