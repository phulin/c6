#!/usr/bin/env python3
"""Frozen offline raw-binding/owned-queue source comparison, never native execution."""
import argparse
import gzip
import tempfile
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import re
import uuid
ROOT = Path(__file__).resolve().parents[2]

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args()
    paths=['crates/c6-data/src/loaded_actions.rs','crates/c6-data/src/loaded_actions/handlers.rs',
           'crates/c6-data/src/loaded_actions/resources.rs','crates/c6-data/src/unit_initialization_definitions.rs',
           'crates/c6-core/src/loaded_state.rs','crates/c6-core/src/unit_operation_interruption.rs',
           'crates/c6-core/src/lib.rs','crates/c6-data/src/lib.rs',
           'tools/linux/loaded_actions_review_load.rs','tools/linux/loaded_handler_adapter_check.rs',
           'tools/linux/loaded_handler_adapter_check.py','evidence/linux/current-handler-closed-review.json','Cargo.lock']
    files=[ROOT/'Cargo.toml',ROOT/'Cargo.lock']
    for suffix in ('*.rs','*.toml','*.json'):
        files+=sorted((ROOT/'crates').rglob(suffix))
    files += [ROOT/name for name in paths]
    for source in list(files):
        if source.suffix != '.rs': continue
        for include in re.findall(r'include_(?:str|bytes)!\(\s*"([^"]+)"',source.read_text()):
            path=(source.parent/include).resolve()
            if path.is_file() and path.is_relative_to(ROOT): files.append(path)
    payloads={str(path.relative_to(ROOT)):path.read_bytes() for path in files}
    if any((ROOT/name).read_bytes()!=data for name,data in payloads.items()):
        raise ValueError('source changed during snapshot; retry')
    before={name:hashlib.sha256(data).hexdigest() for name,data in payloads.items()}
    revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
    retained=ROOT/'target/linux/loaded-handler-adapter-candidates'/uuid.uuid4().hex
    snapshot=retained/'source'
    for name,data in payloads.items():
        path=snapshot/name;path.parent.mkdir(parents=True,exist_ok=True)
        path.write_bytes(data);path.chmod(0o444)
    (retained/'source-manifest.json').write_text(json.dumps(before,sort_keys=True,indent=2)+'\n')
    package=retained/'check';package.mkdir();manifest=package/'Cargo.toml'
    binary_name='check-'+retained.name
    manifest.write_text('[workspace]\n[package]\nname="loaded-handler-adapter-check"\nversion="0.0.0"\nedition="2021"\n'
                        '[dependencies]\nserde_json="1"\nc6-core={path='+json.dumps(str(snapshot/'crates/c6-core'))+'}\n'
                        'c6-data={path='+json.dumps(str(snapshot/'crates/c6-data'))+'}\n'
                        '[[bin]]\nname='+json.dumps(binary_name)+'\npath='+json.dumps(str(snapshot/'tools/linux/loaded_handler_adapter_check.rs'))+'\n')
    shutil.copyfile(snapshot/'Cargo.lock',package/'Cargo.lock')
    target=Path('/tmp/c6-loaded-handler-adapter-check-target')
    trace_source=ROOT.parent/'c6-ghidra/evidence/linux-save-normalization/checkpoint-203-owners/trace-benchmark-1.jsonl.gz'
    with tempfile.TemporaryDirectory(prefix='c6-notification-owner-input-') as temporary:
        trace=Path(temporary)/'owner-trace.jsonl'
        trace.write_bytes(gzip.decompress(trace_source.read_bytes()))
        trace_sha256=digest(trace)
        run=subprocess.run(['cargo','run','--release','--offline','--bin',binary_name,'--manifest-path',str(manifest),
                            '--target-dir',str(target),'--',str(ROOT),str(trace)],text=True,capture_output=True)
    (retained/'build-stderr.txt').write_text(run.stderr)
    if run.returncode:
        raise RuntimeError(run.stderr)
    result=json.loads(run.stdout)
    candidate=target/'release'/binary_name;retained_candidate=retained/'candidate'
    shutil.copyfile(candidate,retained_candidate)
    result.update(owner_trace_sha256=trace_sha256,owner_trace_compressed_sha256=digest(trace_source),candidate_sha256=digest(retained_candidate),candidate_bytes=retained_candidate.stat().st_size,
                  source_revision=revision,source_sha256=before,immutable_source_snapshot=True,
                  retained_source_snapshot=str(snapshot.relative_to(ROOT)),retained_candidate=str(retained_candidate.relative_to(ROOT)),
                  retained_manifest=str(manifest.relative_to(ROOT)),generated_manifest_sha256=digest(manifest),
                  generated_lock_sha256=digest(package/'Cargo.lock'),
                  root_changed_after_snapshot=any((ROOT/name).read_bytes()!=data for name,data in payloads.items()))
    (retained/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    a.output.parent.mkdir(parents=True,exist_ok=True)
    a.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:result[k] for k in ['schema','players','units','formations','actual_handler_slots',
                                          'numeric_fronts_compared','source_leaf_true_fronts','source_leaf_false_fronts',
                                          'successful_callback_stubs_used','original_execution','candidate_sha256']}))
if __name__=='__main__':
    main()
