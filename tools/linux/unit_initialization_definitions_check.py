#!/usr/bin/env python3
"""Initialize 1262 decoded Unit sidecars using admitted static definitions offline."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile
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
    sources=['crates/c6-data/src/unit_initialization_definitions.rs','crates/c6-data/src/loaded_actions/resources.rs',
             'crates/c6-data/src/lib.rs','crates/c6-data/src/registry.rs','crates/c6-data/src/movement.rs',
             'crates/c6-core/src/unit_initialization.rs','crates/c6-core/src/loaded_state.rs',
             'tools/linux/loaded_actions_review_load.rs','tools/linux/unit_initialization_definitions_check.rs',
             'tools/linux/unit_initialization_definitions_check.py']
    before={s:sha((ROOT/s).read_bytes()) for s in sources}
    build=subprocess.run(['cargo','build','--offline','-p','c6-data','--message-format=json'],cwd=ROOT,check=True,capture_output=True,text=True)
    libs={}
    for line in build.stdout.splitlines():
        row=json.loads(line)
        if row.get('reason')=='compiler-artifact':
            for file in row.get('filenames',[]):
                if file.endswith('.rlib'): libs[row['target']['name']]=Path(file)
    with tempfile.TemporaryDirectory(prefix='c6-unit-initialization-definitions-') as temp:
        binary=Path(temp)/'check'
        command=['rustc','--edition=2021',str(ROOT/'tools/linux/unit_initialization_definitions_check.rs'),
                 '-L','dependency='+str(libs['c6_core'].parent),'-o',str(binary)]
        for name in ['c6_core','c6_data','serde_json','sha2']:
            command+=['--extern',name+'='+str(libs[name])]
        subprocess.run(command,check=True,cwd=ROOT)
        result=subprocess.run([str(binary),str(a.manifest),str(a.target_runtime),str(a.rules),
                               str(a.rules/'report.json'),str(a.placement),str(a.manager)],capture_output=True,text=True)
        if result.returncode:
            raise RuntimeError(result.stderr)
        output=json.loads(result.stdout)
        if output['units']!=1262 or output['original_execution'] is not False:
            raise ValueError('unexpected integration boundary')
        output.update(schema='c6-linux-unit-initialization-definitions-check-v1',
                      candidate_sha256=sha(binary.read_bytes()),cargo_lock_sha256=sha((ROOT/'Cargo.lock').read_bytes()),
                      rules_report_sha256=sha((a.rules/'report.json').read_bytes()),
                      target_runtime_sha256=sha(a.target_runtime.read_bytes()),
                      target_manifest_sha256=sha(a.manifest.read_bytes()),sources=before)
    if before!={s:sha((ROOT/s).read_bytes()) for s in sources}:
        raise ValueError('tested source changed during integration check')
    a.output.parent.mkdir(parents=True,exist_ok=True)
    a.output.write_text(json.dumps(output,sort_keys=True,indent=2)+'\n')
    print(json.dumps({k:v for k,v in output.items() if k not in ['sources']},sort_keys=True))


if __name__=='__main__':
    main()
