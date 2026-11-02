#!/usr/bin/env python3
"""Compare bounded log-level metadata with the fingerprinted original host manager."""
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
    sources = ['native/bootstrap/services/logging_probe.c', 'native/bootstrap/services/service_fixture.c', 'native/bootstrap/services/service_fixture.h', 'native/bootstrap/xml-import/imports.c', 'native/bootstrap/xml-import/imports.h']
    identity = {'schema':'c6-original-service-logging-v1', 'assets':config['expected_assets'], 'sources':{p:digest(ROOT/p) for p in sources}, 'compiler':subprocess.check_output(['x86_64-w64-mingw32-gcc','--version'],text=True).splitlines()[0]}
    host = ROOT/'target/bootstrap/service_logging_probe.exe'
    subprocess.run(['x86_64-w64-mingw32-gcc','-std=c11','-O2','-Wall','-Wextra','-Werror','-municode','-static','-Wl,--image-base,0x150000000','-o',str(host),'native/bootstrap/services/logging_probe.c','native/bootstrap/xml-import/imports.c'],cwd=ROOT,check=True)
    if identity['sources'] != {p:digest(ROOT/p) for p in sources}:
        raise SystemExit('source changed during compilation')
    identity['host_sha256'] = digest(host)
    identity['wine'] = subprocess.check_output(['/usr/lib/wine/wine64','--version'],text=True).strip()
    env = dict(os.environ,WINEPREFIX=str(ROOT/'target/bootstrap/wine-prefix'),WINEARCH='win64',WINEDEBUG='-all',WINEDLLOVERRIDES='msvcp140,vcruntime140,ucrtbase=n')
    path = lambda p: 'Z:'+str(p.resolve()).replace('/','\\')
    result = subprocess.run(['/usr/lib/wine/wine64',str(host),path(assets/'Base/Binaries/Win64Steam/CivilizationVI.exe'),path(assets/'Base/Binaries/Win64Steam')],cwd=host.parent,env=env,text=True,capture_output=True,timeout=60)
    observations = [json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')]
    report = {'configuration':identity,'configuration_sha256':hashlib.sha256(json.dumps(identity,sort_keys=True,separators=(',',':')).encode()).hexdigest(),'returncode':result.returncode,'observations':observations,'stderr':result.stderr,'initialized_game':False,'gameplay_parity':False}
    destination = ROOT/'evidence/bootstrap/services-logging-validation.json'
    destination.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'output':str(destination),'returncode':result.returncode,'observations':observations},indent=2))
    if result.returncode or not observations or observations[-1].get('status') != 'ok':
        raise SystemExit(1)
if __name__ == '__main__':
    main()
