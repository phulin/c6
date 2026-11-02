#!/usr/bin/env python3
"""Fingerprint and execute the bounded original Civ6App child probe."""
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
    host = ROOT/'target/bootstrap/app_provider_probe.exe'
    sources = ['native/bootstrap/app-provider/probe.c', 'native/bootstrap/app-provider/app_provider.c', 'native/bootstrap/xml-import/imports.c', 'native/bootstrap/script-system/script_system.c']
    subprocess.run(['x86_64-w64-mingw32-gcc','-std=c11','-O2','-Wall','-Wextra','-Werror','-municode','-static','-Wl,--image-base,0x150000000','-o',str(host),*sources,'-lbcrypt'],cwd=ROOT,check=True)
    identity = {'schema':'c6-original-app-provider-probe-v1', 'assets':config['expected_assets'], 'sources':{s:digest(ROOT/s) for s in sources + ['native/bootstrap/app-provider/app_provider.h', 'native/bootstrap/script-system/script_system.h', 'native/bootstrap/xml-import/imports.h']}, 'host_sha256':digest(host)}
    env = dict(os.environ,WINEPREFIX=str(ROOT/'target/bootstrap/wine-prefix'),WINEARCH='win64',WINEDEBUG='-all',WINEDLLOVERRIDES='msvcp140,vcruntime140,ucrtbase=n')
    path = lambda p: 'Z:'+str(p.resolve()).replace('/','\\')
    snapshot = ROOT.parent/'c6-ghidra/evidence/rules/run-1/gameplay.sqlite'
    metadata = json.loads((ROOT/'evidence/assets/xp2-rules-snapshot.json').read_text())
    expected_snapshot = next(item for item in metadata['databases'] if item['database'] == 'gameplay')['file_sha256']
    if digest(snapshot) != expected_snapshot:
        raise SystemExit('unsupported native rules snapshot')
    identity['native_rules_snapshot_sha256'] = expected_snapshot
    identity_hash = hashlib.sha256(json.dumps(identity,sort_keys=True,separators=(',',':')).encode()).hexdigest()
    cases = []
    for mode in [None]:
        command = ['/usr/lib/wine/wine64', str(host), path(assets/'Base/Binaries/Win64Steam/CivilizationVI.exe'), path(assets/'Base/Binaries/Win64Steam'), path(snapshot)]
        if mode:
            command.append(mode)
        result = subprocess.run(command, cwd=host.parent, env=env, text=True, capture_output=True, timeout=60)
        cases.append({'mode':mode or 'app-provider-and-original-script-state', 'child_exit_code':result.returncode, 'stdout':result.stdout, 'stderr':result.stderr})
    report = {'configuration':identity,'configuration_sha256':identity_hash,'cases':cases,'initialized_game':False,'full_turn_parity':False}
    (ROOT/'evidence/bootstrap/app-provider.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({k:v for k,v in report.items() if k!='configuration'},indent=2))
if __name__ == '__main__':
    main()
