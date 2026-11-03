#!/usr/bin/env python3
"""Exercise state-backed Unit MaxMoves/HasMoved over 1262 decoded Units offline."""
import argparse
import json
from pathlib import Path
import subprocess
import os
import uuid
from definition_ids_audit import ROOT, sha


def main():
    p=argparse.ArgumentParser(description=__doc__)
    private=ROOT.parent/'c6-ghidra/evidence'
    p.add_argument('--manifest',type=Path,default=private/'linux-military-initialization/native-3/candidate-paths-v2.json')
    p.add_argument('--target-runtime',type=Path,default=private/'linux-military-initialization/native-3/runtime.json')
    p.add_argument('--rules',type=Path,default=private/'linux-rules/mod-activation-xp2-constructor-final-1')
    p.add_argument('--placement',type=Path,default=private/'linux-placement-metadata/native-1')
    p.add_argument('--manager',type=Path,default=private/'linux-unit-operation-manager-metadata/native-1')
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args()
    # Copy once, then build and execute the immutable public-source candidate.
    # Concurrent owners may continue editing the checkout during the long decode.
    candidate=private/'linux-unit-movement-state'/('candidate-'+uuid.uuid4().hex)
    snapshot=candidate/'source'
    files=[ROOT/'Cargo.toml',ROOT/'Cargo.lock']
    files += sorted((ROOT/'crates').rglob('*.rs'))
    files += sorted((ROOT/'crates').rglob('*.toml'))
    files += sorted((ROOT/'crates').rglob('*.json'))
    files += [ROOT/'evidence'/p for p in ['linux/rules-base-snapshot.json',
              'linux/rules-xp2-request-none-snapshot.json','assets/xp2-rules-snapshot.json']]
    files += [ROOT/'tools/linux'/p for p in ['loaded_actions_review_load.rs',
              'unit_movement_state_check.rs','unit_movement_state_check.py']]
    payloads={str(path.relative_to(ROOT)):path.read_bytes() for path in files}
    # Short copy-time consistency check, not a workspace freeze for the run.
    if any((ROOT/name).read_bytes()!=value for name,value in payloads.items()):
        raise ValueError('source changed while taking candidate snapshot; retry')
    sources={name:sha(value) for name,value in payloads.items()}
    for name,value in payloads.items():
        path=snapshot/name;path.parent.mkdir(parents=True,exist_ok=True)
        path.write_bytes(value);path.chmod(0o444)
    revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
    status=subprocess.check_output(['git','status','--short'],cwd=ROOT,text=True)
    wip=[line for line in status.splitlines() if line[3:] in sources]
    manifest=dict(source_revision=revision,uncommitted_source_paths=wip,sources=sources)
    (candidate/'source-manifest.json').write_text(json.dumps(manifest,sort_keys=True,indent=2)+'\n')
    env=dict(os.environ,CARGO_TARGET_DIR=str(ROOT/'target'))
    build=subprocess.run(['cargo','build','--locked','--offline','-p','c6-data','--message-format=json'],
                         cwd=snapshot,env=env,check=True,capture_output=True,text=True)
    libs={}
    for line in build.stdout.splitlines():
        row=json.loads(line)
        if row.get('reason')=='compiler-artifact':
            for file in row.get('filenames',[]):
                if file.endswith('.rlib'): libs[row['target']['name']]=Path(file)
    binary=candidate/'check'
    command=['rustc','--edition=2021',str(snapshot/'tools/linux/unit_movement_state_check.rs'),
             '-L','dependency='+str(libs['c6_core'].parent),'-o',str(binary)]
    for name in ['c6_core','c6_data','serde_json','sha2']:
        command+=['--extern',name+'='+str(libs[name])]
    subprocess.run(command,check=True,cwd=snapshot)
    result=subprocess.run([str(binary),str(a.manifest),str(a.target_runtime),str(a.rules),
                           str(a.rules/'report.json'),str(a.placement),str(a.manager)],capture_output=True,text=True)
    if result.returncode: raise RuntimeError(result.stderr)
    output=json.loads(result.stdout)
    if output['units']!=1262 or output['original_execution'] is not False:
        raise ValueError('unexpected integration boundary')
    if sources!={name:sha((snapshot/name).read_bytes()) for name in sources}:
        raise ValueError('immutable candidate source changed')
    output.update(schema='c6-linux-unit-movement-state-check-v1',
                  candidate_sha256=sha(binary.read_bytes()),cargo_lock_sha256=sources['Cargo.lock'],
                  rules_report_sha256=sha((a.rules/'report.json').read_bytes()),
                  target_runtime_sha256=sha(a.target_runtime.read_bytes()),
                  target_manifest_sha256=sha(a.manifest.read_bytes()),sources=sources,
                  candidate_source_directory=str(snapshot),source_revision=revision,
                  uncommitted_source_paths=wip,immutable_source_snapshot=True)
    a.output.parent.mkdir(parents=True,exist_ok=True)
    a.output.write_text(json.dumps(output,sort_keys=True,indent=2)+'\n')
    print(json.dumps({k:v for k,v in output.items() if k not in ['sources']},sort_keys=True))


if __name__=='__main__':
    main()
