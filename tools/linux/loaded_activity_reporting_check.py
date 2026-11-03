#!/usr/bin/env python3
"""Offline actual loaded activity reporting check, never original execution."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import shutil
import re
import subprocess
import tempfile
import uuid
ROOT = Path(__file__).resolve().parents[2]

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args()
    paths=['crates/c6-data/src/loaded_actions.rs','crates/c6-data/src/loaded_actions/handlers.rs',
           'crates/c6-data/src/loaded_actions/reporting.rs','crates/c6-data/src/unit_movement_state.rs','crates/c6-data/src/game_relations.rs','crates/c6-data/src/loaded_actions/resources.rs','crates/c6-data/src/unit_initialization_definitions.rs',
           'crates/c6-core/src/loaded_state.rs','crates/c6-core/src/unit_operation_interruption.rs',
           'crates/c6-core/src/lib.rs','crates/c6-data/src/lib.rs',
           'tools/linux/loaded_actions_review_load.rs','tools/linux/loaded_activity_reporting_check.rs',
           'tools/linux/loaded_activity_reporting_check.py','evidence/linux/current-handler-closed-review.json','Cargo.lock']
    files = [ROOT/'Cargo.toml', ROOT/'Cargo.lock']
    for suffix in ('*.rs', '*.toml', '*.json'):
        files += sorted((ROOT/'crates').rglob(suffix))
    files += [ROOT/name for name in paths]
    files += [ROOT/'evidence'/name for name in ('linux/rules-base-snapshot.json', 'linux/rules-xp2-request-none-snapshot.json', 'assets/xp2-rules-snapshot.json')]
    for source in list(files):
        if source.suffix != '.rs': continue
        for include in re.findall(r'include_(?:str|bytes)!\(\s*"([^"]+)"', source.read_text()):
            path = (source.parent/include).resolve()
            if path.is_file() and path.is_relative_to(ROOT): files.append(path)
    payloads={str(path.relative_to(ROOT)):path.read_bytes() for path in files}
    if any((ROOT/name).read_bytes()!=data for name,data in payloads.items()):
        raise ValueError('source changed while snapshotting; retry')
    before={name:hashlib.sha256(data).hexdigest() for name,data in payloads.items()}
    revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
    with tempfile.TemporaryDirectory(prefix='c6-handler-adapter-') as temporary:
        work=Path(temporary);manifest=work/'candidate.toml'
        retained=ROOT/'target/linux/loaded-activity-reporting-candidates'/uuid.uuid4().hex
        snapshot=retained/'source'
        for name,data in payloads.items():
            path=snapshot/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(data);path.chmod(0o444)
        (retained/'source-manifest.json').write_text(json.dumps(before,sort_keys=True,indent=2)+'\n')
        package=retained/'check';package.mkdir();manifest=package/'Cargo.toml' 
        manifest.write_text('[workspace]\n[package]\nname="loaded-activity-reporting-check"\nversion="0.0.0"\nedition="2021"\n'
                            '[dependencies]\nserde_json="1"\nc6-core={path='+json.dumps(str(snapshot/'crates/c6-core'))+'}\n'
                            'c6-data={path='+json.dumps(str(snapshot/'crates/c6-data'))+'}\n'
                            '[[bin]]\nname="check"\npath='+json.dumps(str(snapshot/'tools/linux/loaded_activity_reporting_check.rs'))+'\n')
        shutil.copyfile(snapshot/'Cargo.lock',package/'Cargo.lock')
        trace=work/'owner-trace.jsonl'
        trace.write_bytes(gzip.decompress((ROOT.parent/'c6-ghidra/evidence/linux-save-normalization/checkpoint-203-owners/trace-benchmark-1.jsonl.gz').read_bytes()))
        target=Path('/tmp/c6-loaded-activity-reporting-check-target')
        run=subprocess.run(['cargo','run','--offline','--manifest-path',str(manifest),
                            '--target-dir',str(target),'--',str(ROOT),str(trace)],text=True,capture_output=True)
        if run.returncode:
            raise RuntimeError(run.stderr)
        result=json.loads(run.stdout)
        original=json.loads((ROOT/'evidence/linux/skip-turn-state-execute-3.json').read_text())['reporting_events']
        expected=[dict(type_bits=r['type_bits'],component_bits=r['component_bits'],first=r.get('value',r.get('first')),second=r.get('second')) for r in original]
        attempted=[{k:r[k] for k in ('type_bits','component_bits','first','second')} for r in result['partial_actual_entrypoint_trace'] if r['kind']=='report']
        if attempted!=expected[:len(attempted)]: raise ValueError('source report prefix differs from original captured scalar rows')
        result['original_report_prefix_compared']=len(attempted)
        result['original_report_prefix_equal']=True
        candidate=target/'debug/check'
        retained_candidate=retained/'candidate'
        shutil.copyfile(candidate,retained_candidate)
        result.update(source_revision=revision,generated_manifest_sha256=digest(manifest),generated_lock_sha256=digest(package/'Cargo.lock'),retained_manifest=str(manifest.relative_to(ROOT)),candidate_sha256=digest(candidate),candidate_bytes=candidate.stat().st_size,
                      retained_source_snapshot=str(snapshot.relative_to(ROOT)),retained_candidate=str(retained_candidate.relative_to(ROOT)),source_sha256=before,immutable_source_snapshot=True,root_changed_after_snapshot=any((ROOT/name).read_bytes()!=data for name,data in payloads.items()))
        (retained/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    a.output.parent.mkdir(parents=True,exist_ok=True)
    a.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:result[k] for k in ['schema','synchronous_activity_publications','exact_remaining_consumer','original_execution','candidate_sha256']}))
if __name__=='__main__':
    main()
