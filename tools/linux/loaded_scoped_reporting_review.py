#!/usr/bin/env python3
"""Review scoped reporting capabilities and isolated source projections.

Reuses the earlier retained-source rebuild for unchanged files. Replays only the
hash-verified new Rust checker and compiles small safe-API capability probes;
never executes original game code or repeats the original method-body census.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import tomllib

ROOT=Path(__file__).resolve().parents[2]
COMMIT='6bdb67d28a6bb2757aab1388c95e035724656256'
CANDIDATE='a7c6f701aa904906a5999b7d77c42bfb75a3fd5581f3b38be3dc64b93f4546db'
CHECKPOINT='10ac27dc3b1212f683bc68249e3d4ca7e1e7b5fed0fa63a4db62c2434df0f35b'
CHANGED={'crates/c6-data/src/loaded_actions.rs','crates/c6-data/src/loaded_actions/reporting.rs',
         'tools/linux/loaded_activity_reporting_check.py','tools/linux/loaded_activity_reporting_check.rs'}
PROBES={
 'allowed_read_then_actor_activity':('let _ = c.state().revision(); let _ = c.reference(); c.actor_mut().unwrap().e_activity = 0;',None),
 'allowed_actor_identity_write_is_runtime_checked':('c.actor_mut().unwrap().id ^= 1;',None),
 'deny_other_unit_mutation':('c.state().unit_mut(1, 0).unwrap().e_activity = 0;','E0596'),
 'deny_other_player_mutation':('let _ = c.state().world().player_mut(1);','E0596'),
 'deny_reference_rebinding':('c.reference = c.reference();','E0616'),
 'deny_live_mutable_actor_and_world_alias':('let actor = c.actor_mut().unwrap(); let state = c.state(); actor.e_activity = state.revision() as i32;','E0502'),
}


def sha(data):return hashlib.sha256(data).hexdigest()
def require(value,message):
    if not value:raise ValueError(message)


def review(checkpoint,replay,probes):
    raw=checkpoint.read_bytes();require(sha(raw)==CHECKPOINT,'scoped checkpoint changed')
    check=json.loads(raw);snapshot=ROOT/check['retained_source_snapshot'];base=snapshot.parent
    declared=json.loads((base/'source-manifest.json').read_bytes())
    require(declared==check['source_sha256'],'retained source manifest differs')
    for name,digest in declared.items():
        path=snapshot/name
        require(path.resolve().is_relative_to(snapshot.resolve()),'source path escapes')
        require(sha(path.read_bytes())==digest,'retained source differs: '+name)
        committed=subprocess.check_output(['git','show',COMMIT+':'+name],cwd=ROOT)
        require(sha(committed)==digest,'retained source differs from coherent commit: '+name)
    previous_path=ROOT/'evidence/linux/loaded-ordered-reporting-check.json'
    previous=json.loads(previous_path.read_bytes())
    changed={name for name,digest in declared.items() if previous['source_sha256'].get(name)!=digest}
    require(changed==CHANGED,'unexpected changed source boundary')
    prior_review_path=ROOT/'evidence/linux/loaded-ordered-reporting-independent-review.json'
    prior_review=json.loads(prior_review_path.read_bytes())
    require(prior_review['checkpoint_sha256']==sha(previous_path.read_bytes())
            and prior_review['independent_source_rebuild']['fields_equal']==25,
            'previous independent rebuild identity differs')
    for key in ['partial_actual_entrypoint_trace','ordered_publications','snapshot_paths','exact_remaining_consumer']:
        require(check[key]==previous[key],'genuine action prefix changed: '+key)
    require(check['original_report_prefix_compared']==4 and check['original_report_prefix_equal'],
            'native scalar prefix changed')
    require(check['isolated_cases_are_action_continuation'] is False
            and check['host_publication_fields_have_native_capture_comparison'] is False,
            'isolated/native scope relabeled')
    original_path=ROOT/'evidence/linux/skip-turn-state-execute-3.json'
    original=json.loads(original_path.read_bytes())['reporting_events']
    cases=check['isolated_source_projection_cases'];require(len(cases)==len(original)==9,'isolated coverage differs')
    hosts={0xe23b112d:35,0xa60328c5:70,0x383669f1:71,0x15c35863:67,0x3d4006f2:68}
    for index,(source,case) in enumerate(zip(original,cases)):
        bits=int(source['type_bits'],16);event=bits&0xffffffff;subtype=bits>>32
        first=source.get('value',source.get('first',0));second=source.get('second',0)
        host=hosts[event];activity_operation=None;parameters=None
        if host==35:
            activity_operation=0x2c681908 if first==0x490cc8e1 else 0
            parameters=[-1]*4;fields=(activity_operation,-1)
        elif host in (67,68):fields=(subtype,first)
        else:fields=(first,second)
        require(case['captured_row_index']==index and case['host_event']==host
                and (case['operation'],case['value'])==fields
                and case['activity_operation']==activity_operation
                and case['activity_parameters']==parameters,'source projection differs at '+str(index))
        require(case['context_setup']=='explicit isolated current-Unit queue/activity fixture; not action continuation',
                'isolated setup relabeled')
    candidate=ROOT/check['retained_candidate'];require(sha(candidate.read_bytes())==CANDIDATE==check['candidate_sha256'],
                                                       'retained scoped executable differs')
    manifest=(base/'check/Cargo.toml').read_bytes();lock=(base/'check/Cargo.lock').read_bytes()
    require(sha(manifest)==check['generated_manifest_sha256'] and sha(lock)==check['generated_lock_sha256'],
            'retained generated build inputs differ')
    parsed=tomllib.loads(manifest.decode());expected_source=snapshot/'tools/linux/loaded_activity_reporting_check.rs'
    require(Path(parsed['bin'][0]['path'])==expected_source,'retained checker path differs')
    replay_result=None;probe_results=[]
    with tempfile.TemporaryDirectory(prefix='c6-scoped-report-review-') as tmp:
        work=Path(tmp)
        if replay:
            trace=work/'owner-trace.jsonl'
            trace.write_bytes(gzip.decompress((ROOT.parent/'c6-ghidra/evidence/linux-save-normalization/checkpoint-203-owners/trace-benchmark-1.jsonl.gz').read_bytes()))
            executable=work/'retained-rust-check'
            executable.write_bytes(candidate.read_bytes());executable.chmod(0o700)
            require(sha(executable.read_bytes())==CANDIDATE,'executable copy differs')
            run=subprocess.run([str(executable),str(ROOT),str(trace)],text=True,capture_output=True)
            require(run.returncode==0,'retained Rust checker failed: '+run.stderr)
            actual=json.loads(run.stdout)
            require(all(check.get(key)==value for key,value in actual.items()),'scoped retained replay differs')
            replay_result=dict(executable_sha256=CANDIDATE,fields_equal=len(actual),output_sha256=sha(run.stdout.encode()),
                               source_rebuild=False,original_execution=False)
        if probes:
            probe=work/'probe.rs';probe_manifest=work/'Cargo.toml'
            probe_manifest.write_text(manifest.decode().replace(json.dumps(str(expected_source)),json.dumps(str(probe))))
            (work/'Cargo.lock').write_bytes(lock)
            for name,(body,error) in PROBES.items():
                code='#![forbid(unsafe_code)]\nuse c6_data::loaded_actions::ReportingContext;\nfn probe(c: &mut ReportingContext<\'_>) {'+body+'}\nfn main(){}\n'
                probe.write_text(code)
                run=subprocess.run(['cargo','check','--offline','--locked','--quiet','--manifest-path',str(probe_manifest),
                    '--target-dir',str(ROOT/'target/linux/scoped-reporting-capability-target')],text=True,capture_output=True)
                require((run.returncode==0) if error is None else (run.returncode!=0 and error in run.stderr),
                        'safe capability probe differs: '+name+'\n'+run.stderr)
                probe_results.append(dict(name=name,source_sha256=sha(code.encode()),compiles=run.returncode==0,expected_error=error))
    for name,digest in declared.items():require(sha((snapshot/name).read_bytes())==digest,'source changed during review')
    return dict(schema='c6-linux-loaded-scoped-reporting-independent-review-v1',reviewed_commit=COMMIT,
        checkpoint_sha256=CHECKPOINT,reviewer_source_sha256=sha(Path(__file__).read_bytes()),
        source_preimages_checked=len(declared),source_preimages_matching_commit=len(declared),
        changed_preimages=sorted(changed),unchanged_preimages_reusing_previous_build=len(declared)-len(changed),
        reused_previous_review_sha256=sha(prior_review_path.read_bytes()),original_method_census_repeated=False,
        retained_candidate_sha256=CANDIDATE,retained_manifest_sha256=sha(manifest),retained_resolved_lock_sha256=sha(lock),
        retained_binary_replay=replay_result,safe_capability_compile_probes=probe_results,
        real_native_send_scalar_prefix_compared=4,real_native_send_scalar_rows_available=9,
        isolated_source_projection_cases_compared=9,isolated_cases_are_action_continuation=False,
        native_host_publication_values_compared=False,
        mutation_boundary=dict(private_context_state_and_reference=True,immutable_world_reads=True,
            mutable_addressed_actor_only=True,full_unit_mutation_requires_runtime_guard=True,
            unsupported_actor_mutations_retained=True,world_index_repair_implemented=False,
            actor_identity_changes_stop_continuation=True,stale_runtime_identity_or_dependencies_reject=True),
        wider_world_rng_callbacks_supported=False,complete_action_state_rng_parity=False,original_execution=False,
        exact_remaining_consumer=check['exact_remaining_consumer'])


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--checkpoint',type=Path,default=ROOT/'evidence/linux/loaded-ordered-reporting-scoped-check.json')
    p.add_argument('--replay',action='store_true');p.add_argument('--capability-probes',action='store_true')
    p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    result=review(a.checkpoint,a.replay,a.capability_probes)
    a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(result,indent=2,sort_keys=True)+'\n')
    print(json.dumps(dict(source_preimages=result['source_preimages_checked'],isolated_source_cases=9,
        capability_probes=len(result['safe_capability_compile_probes']),replayed=result['retained_binary_replay'] is not None,original_execution=False)))


if __name__=='__main__':main()
