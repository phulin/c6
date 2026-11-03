#!/usr/bin/env python3
"""Review retained ordered-reporting source and rebuild only Rust code offline.

Reuses the unchanged first reporting review. Fingerprints only the scalar
translation/operation projection additions; never executes original game code.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

ROOT=Path(__file__).resolve().parents[2]
ADDED_METHODS=[('host','Translate.OperationAdded',0x320cb30,123),
 ('host','Translate.OperationStarted',0x320cbb0,123),
 ('host','Translate.OperationDeactivated',0x320ccb0,123),
 ('host','Translate.OperationsCleared',0x320cd30,123),
 ('host','EventData1.Ctor',0x32b54f0,132),
 ('host','ActivityData.Ctor',0x321e990,29),
 ('game','Exports.Unit.GetOperationType',0x2c93be0,95),
 ('game','Exports.Unit.GetOperationParams',0x2c93c40,75)]


def sha(data):return hashlib.sha256(data).hexdigest()
def require(value,message):
    if not value:raise ValueError(message)


def review(checkpoint,assets,rebuild):
    raw=checkpoint.read_bytes();check=json.loads(raw)
    snapshot=ROOT/check['retained_source_snapshot']
    require(snapshot.resolve().is_relative_to((ROOT/'target').resolve()),'snapshot outside retained target')
    declared=json.loads((snapshot.parent/'source-manifest.json').read_bytes())
    require(declared==check['source_sha256'],'source manifest differs from checkpoint')
    for name,expected in declared.items():
        path=snapshot/name
        require(path.resolve().is_relative_to(snapshot.resolve()),'source path escapes retained snapshot')
        require(sha(path.read_bytes())==expected,'retained source preimage differs: '+name)
    build_manifest=(snapshot.parent/'check/Cargo.toml').read_bytes()
    build_lock=(snapshot.parent/'check/Cargo.lock').read_bytes()
    package=tomllib.loads(build_manifest.decode())
    for name in ('c6-core','c6-data'):
        require(Path(package['dependencies'][name]['path']).resolve()==(snapshot/'crates'/name).resolve(),
                'retained dependency path differs: '+name)
    require(Path(package['bin'][0]['path']).resolve()==(snapshot/'tools/linux/loaded_activity_reporting_check.rs').resolve(),
            'retained checker source path differs')
    candidate=ROOT/check['retained_candidate']
    require(sha(candidate.read_bytes())==check['candidate_sha256'],'retained executable differs')
    original_path=ROOT/'evidence/linux/skip-turn-state-execute-3.json'
    original=json.loads(original_path.read_bytes())['reporting_events']
    expected=[dict(type_bits=r['type_bits'],component_bits=r['component_bits'],
        first=r.get('value',r.get('first')),second=r.get('second')) for r in original]
    sends=[{k:r[k] for k in ('type_bits','component_bits','first','second')}
           for r in check['partial_actual_entrypoint_trace'] if r['kind']=='report']
    require(0<len(sends)<=len(expected) and sends==expected[:len(sends)],'original typed Send scalar prefix differs')
    require(check['original_report_prefix_compared']==len(sends) and check['original_report_prefix_equal'],
            'captured scalar prefix metadata missing')
    images={'game':Elf(assets/'libGameCore_XP2.so',GAME_SHA),'host':Elf(assets/'Civ6',HOST_SHA)}
    added=[]
    for image,name,address,size in ADDED_METHODS:
        added.append(dict(image=image,name=name,rva=hex(address),bytes=size,
                          sha256=sha(images[image].read(address,size))))
    for address,encoded in [(0x320cb7b,'8b4304'),(0x320cbb0+0x4b,'8b4304'),
                            (0x320ccb0+0x4b,'8b4350'),(0x320ccb0+0x52,'8b4354'),
                            (0x320cd30+0x4b,'8b4350'),(0x320cd30+0x52,'8b4354')]:
        require(images['host'].read(address,3)==bytes.fromhex(encoded),'original scalar source differs')
    rows=check['ordered_publications']
    require([r['host_event'] for r in rows]==[35,70,71,67,35,68,35,70,71][:len(rows)],
            'host publication ordering differs')
    for source,publication in zip(sends,rows):
        event=int(source['type_bits'],16)&0xffffffff
        subtype=int(source['type_bits'],16)>>32
        if event in (0x15c35863,0x3d4006f2):
            fields=(subtype,source['first'] or 0)
        elif event in (0xa60328c5,0x383669f1):
            fields=(source['first'] or 0,source['second'] or 0)
        else:
            # This is source-derived facade projection, not a captured native
            # event35 output comparison. Only the current SkipTurn front is closed.
            operation=0x2c681908 if source['first']==0x490cc8e1 else 0
            fields=(operation,-1)
            require(publication['activity_operation']==operation,'activity facade source projection differs')
        require((publication['operation'],publication['value'])==fields,
                'source-derived translator scalar output differs')
    independent=None
    if rebuild:
        with tempfile.TemporaryDirectory(prefix='c6-ordered-report-review-') as tmp:
            package=Path(tmp)
            manifest=package/'Cargo.toml'
            manifest.write_text('[workspace]\n[package]\nname="loaded-activity-reporting-check"\nversion="0.0.0"\nedition="2021"\n'
                '[dependencies]\nserde_json="1"\nc6-core={path='+json.dumps(str(snapshot/'crates/c6-core'))+'}\n'
                'c6-data={path='+json.dumps(str(snapshot/'crates/c6-data'))+'}\n'
                '[[bin]]\nname="check"\npath='+json.dumps(str(snapshot/'tools/linux/loaded_activity_reporting_check.rs'))+'\n')
            shutil.copyfile(snapshot.parent/'check/Cargo.lock',package/'Cargo.lock')
            trace=package/'owner-trace.jsonl'
            trace.write_bytes(gzip.decompress((ROOT.parent/'c6-ghidra/evidence/linux-save-normalization/checkpoint-203-owners/trace-benchmark-1.jsonl.gz').read_bytes()))
            target=ROOT/'target/linux/ordered-reporting-independent-target'
            run=subprocess.run(['cargo','run','--offline','--locked','--manifest-path',str(manifest),
                '--target-dir',str(target),'--',str(ROOT),str(trace)],capture_output=True,text=True)
            require(run.returncode==0,'independent Rust source rebuild/replay failed: '+run.stderr)
            actual=json.loads(run.stdout)
            require(all(check.get(key)==value for key,value in actual.items()),'independent rebuilt Rust payload differs')
            independent=dict(fields_equal=len(actual),candidate_sha256=sha((target/'debug/check').read_bytes()),
                output_sha256=sha(run.stdout.encode()),resolved_lock_sha256=sha((package/'Cargo.lock').read_bytes()),
                rebuilt_from_retained_source=True,original_execution=False)
    for name,expected_digest in declared.items():
        require(sha((snapshot/name).read_bytes())==expected_digest,'retained source changed during review')
    return dict(schema='c6-linux-loaded-ordered-reporting-independent-review-v1',
        checkpoint_sha256=sha(raw),checkpoint=str(checkpoint.relative_to(ROOT)),
        reviewer_source_sha256=sha(Path(__file__).read_bytes()),
        source_preimages_checked=len(declared),source_manifest_sha256=sha((snapshot.parent/'source-manifest.json').read_bytes()),
        retained_build_manifest_sha256=sha(build_manifest),retained_resolved_lock_sha256=sha(build_lock),
        retained_source_snapshot=check['retained_source_snapshot'],retained_candidate_sha256=check['candidate_sha256'],
        previous_review='evidence/linux/loaded-activity-reporting-independent-review.json',
        previous_266_method_body_checks_repeated=False,additional_methods=added,
        original_send_scalar_rows_compared=len(sends),original_send_scalar_rows_available=len(expected),original_send_scalar_rows_equal=True,
        original_send_evidence_sha256=sha(original_path.read_bytes()),
        source_translator_publications_compared=len(rows),native_host_publication_values_compared=False,
        publication_event_order=[r['host_event'] for r in rows],
        actual_snapshot_paths=check['snapshot_paths'],exact_remaining_consumer=check['exact_remaining_consumer'],
        exercised_host_events=[35,70,71,67][:len(rows)],
        unexercised_compiled_branches=['UnitOperationStarted host68','OPERATION activity facade projection'],
        scope_admission='stock headless profile with no added consumers is an explicit caller condition',
        producer_effects='three snapshot readiness checks, including one source-computed adjacent-move success; HoldCycling not invoked',
        mutation_authority='concrete provider queries immutable LoadedState; action prefix remains in same authoritative state on script failure',
        independent_source_rebuild=independent,original_execution=False,
        native_full_action_comparison=False,full_live_affected_state_rng_parity=False)


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--checkpoint',type=Path,required=True)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--rebuild',action='store_true')
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();result=review(a.checkpoint.resolve(),a.assets,a.rebuild)
    a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(result,indent=2,sort_keys=True)+'\n')
    print(json.dumps(dict(source_preimages=result['source_preimages_checked'],original_scalar_rows=result['original_send_scalar_rows_compared'],
        rebuilt=result['independent_source_rebuild'] is not None,original_execution=False)))


if __name__=='__main__':main()
