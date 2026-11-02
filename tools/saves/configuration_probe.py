#!/usr/bin/env python3
"""Fingerprint-gated original saved configuration TypedVariantMap roundtrip."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import shutil
import sys
sys.path.insert(0,str(Path(__file__).resolve().parent))
from inspect_save import Reader
ROOT = Path(__file__).resolve().parents[2]
def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--restore-configuration',action='store_true',help='experimental original singleton-reader attempt; successful restore not yet established')
    args=parser.parse_args()
    config = json.loads((ROOT/'tools/bootstrap/config.json').read_text())
    assets = ROOT.parent/'c6-assets'
    for relative, expected in config['expected_assets'].items():
        if digest(assets/relative) != expected:
            raise SystemExit(f'unsupported asset: {relative}')
    for relative in config['expected_assets']:
        if relative.startswith('LaunchPad/'):
            provider=assets/relative
            (ROOT/'target/bootstrap').mkdir(parents=True,exist_ok=True)
            shutil.copyfile(provider,ROOT/'target/bootstrap'/provider.name)
    sources = ['native/bootstrap/configuration/probe.c', 'native/bootstrap/configuration/configuration.c', 'native/bootstrap/configuration/configuration.h', 'native/bootstrap/configuration/key_initializers.h', 'native/bootstrap/job-manager/job_manager.c', 'native/bootstrap/job-manager/job_manager.h', 'native/bootstrap/xml-import/imports.c', 'native/bootstrap/xml-import/imports.h', 'tools/saves/configuration_probe.py', 'tools/saves/inspect_save.py', 'tools/bootstrap/config.json']
    identity = {'schema':'c6-original-saved-configuration-maps-v1', 'assets':config['expected_assets'],'restore_configuration_requested':args.restore_configuration, 'sources':{p:digest(ROOT/p) for p in sources}, 'compiler':subprocess.check_output(['x86_64-w64-mingw32-gcc','--version'],text=True).splitlines()[0]}
    host = ROOT/'target/bootstrap/configuration_probe.exe'
    subprocess.run(['x86_64-w64-mingw32-gcc','-std=c11','-O2','-Wall','-Wextra','-Werror','-municode','-static','-Wl,--image-base,0x150000000','-o',str(host),'native/bootstrap/configuration/probe.c','native/bootstrap/configuration/configuration.c','native/bootstrap/xml-import/imports.c','native/bootstrap/job-manager/job_manager.c','-lbcrypt'],cwd=ROOT,check=True)
    if identity['sources'] != {p:digest(ROOT/p) for p in sources}:
        raise SystemExit('source changed during compilation')
    save=assets/'Base/Assets/Benchmark/Saves/XP2Benchmark.Civ6Save'
    expected_save='a328d643201cb11ba226eee46c782f85b0df36b2a7a4e1a32372da8abe9cddb8'
    if digest(save)!=expected_save: raise SystemExit('unsupported save identity')
    identity['save_sha256']=expected_save
    identity['host_sha256'] = digest(host)
    identity['wine'] = subprocess.check_output(['/usr/lib/wine/wine64','--version'],text=True).strip()
    env = dict(os.environ,WINEPREFIX=str(ROOT/'target/bootstrap/wine-prefix'),WINEARCH='win64',WINEDEBUG='-all',WINEDLLOVERRIDES='msvcp140,vcruntime140,ucrtbase=n')
    path = lambda p: 'Z:'+str(p.resolve()).replace('/','\\')
    private=ROOT.parent/'c6-ghidra/evidence/configuration/native-roundtrip'
    private.mkdir(parents=True,exist_ok=True)
    result = subprocess.run(['/usr/lib/wine/wine64',str(host),path(assets/'Base/Binaries/Win64Steam/CivilizationVI.exe'),path(assets/'Base/Binaries/Win64Steam'),path(assets/'DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll'),path(save),path(private)]+(['--restore'] if args.restore_configuration else []),cwd=host.parent,env=env,text=True,capture_output=True,timeout=60)
    observations = [json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')]
    comparisons=[]
    save_bytes=save.read_bytes()
    for row in observations:
        if 'start' not in row: continue
        original=Reader(save_bytes); original.pos=row['start']
        native=Reader((private/f"map-{row['map']}.bin").read_bytes())
        # Compare original native typed payloads separately from byte order/flags.
        def semantic(value):
            if isinstance(value,list): return [semantic(v) for v in value]
            if not isinstance(value,dict): return value
            result={k:semantic(v) for k,v in value.items() if k!='offset'}
            if 'entries' in result: result['entries'].sort(key=lambda v:v['key'])
            return result
        expected=sorted(semantic(original.map()),key=lambda v:v['key'])
        actual=sorted(semantic(native.map()),key=lambda v:v['key'])
        normalizations=[]
        def canonical(value, location=''):
            if isinstance(value,list): return [canonical(v,location+'/'+str(i)) for i,v in enumerate(value)]
            if not isinstance(value,dict): return value
            result={k:canonical(v,location+'/'+k) for k,v in value.items()}
            before=result.get('flags')
            if before=='04000000' and 'entries' in result:
                result['flags']='05000000'
            elif before=='20000000' and result.get('unsigned')==0:
                result['flags']='00000000'
            if result.get('flags')!=before:
                normalizations.append(dict(path=location,saved_flags=before,native_flags=result['flags']))
            return result
        canonical_expected=canonical(expected)
        comparisons.append(dict(map=row['map'],raw_typed_equal=expected==actual,
            native_canonical_typed_equal=canonical_expected==actual,flag_normalizations=normalizations,
            original_end=original.pos,native_end=native.pos))
    report = {'configuration':identity,'configuration_sha256':hashlib.sha256(json.dumps(identity,sort_keys=True,separators=(',',':')).encode()).hexdigest(),'returncode':result.returncode,'observations':observations,'map_comparisons':comparisons,'stderr':result.stderr,'initialized_game':False,'gameplay_parity':False}
    destination = ROOT/('evidence/saves/configuration-native-restore-attempt.json' if args.restore_configuration else 'evidence/saves/configuration-native-maps.json')
    destination.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'output':str(destination),'returncode':result.returncode,'observations':observations},indent=2))
    if result.returncode or not comparisons or not all(c['native_canonical_typed_equal'] for c in comparisons) or not observations or observations[-1].get('status') != 'ok':
        raise SystemExit(1)
if __name__ == '__main__':
    main()
