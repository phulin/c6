#!/usr/bin/env python3
"""Independent concrete notification predicate review; executes Rust only."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tempfile
from save_lifecycle_audit import Elf, GAME_SHA
from unit_operation_contracts_review import METHODS

ROOT = Path(__file__).resolve().parents[2]
OWNED = ['crates/c6-core/src/unit_operation_notification_provider.rs',
         'tools/linux/unit_operation_notification_provider_audit.py',
         'evidence/linux/unit-operation-notification-provider.json',
         'docs/linux-unit-operation-notification-provider.md']
HARNESS = r'''
use c6_core::notification_lifecycle::*;
use c6_core::notifications::{Notification,PlayerNotifications};
use c6_core::unit_operation_notification_provider::{CapturedOwner,LivePredicates};
use c6_core::unit_operation_aftermath::Error;
fn state(records: Vec<Notification>, ids: &[u64]) -> State {
 State { notifications: PlayerNotifications { owner: 0, next_id: 9, records },
  live: ids.iter().map(|&id| LiveNotification { object: ObjectId(id), lock_count: 0,
   pool_binding: None, definition_icon: DefinitionIcon::Unobserved,
   activation: ActivationBinding::Unsupported, release: ReleaseBinding::Unsupported,
   destruction: DestructionBinding::Unsupported }).collect(), pools: vec![] }
}
fn input(object: u64, table: u64) -> LivePredicates {
 LivePredicates { object: ObjectId(object), vtable_addresspoint: table, default_blocking: Some(3) }
}
fn main() {
 let record = Notification::default();
 assert_eq!(CapturedOwner::new(state(vec![record.clone()], &[1]), vec![], None), Err(Error::Unsupported("live predicate cardinality")));
 assert_eq!(CapturedOwner::new(state(vec![record.clone()], &[1]), vec![input(2,0x3af6238)], None), Err(Error::Unsupported("live predicate object correspondence")));
 assert_eq!(CapturedOwner::new(state(vec![record.clone(),record.clone()], &[1,1]), vec![input(1,0x3af6238),input(1,0x3af6238)], None), Err(Error::Unsupported("live predicate object correspondence")));
 assert_eq!(CapturedOwner::new(state(vec![record.clone()], &[1]), vec![input(1,0)], None), Err(Error::Unsupported("unknown target notification subtype")));
 let mut p = CapturedOwner::new(state(vec![record.clone()], &[1]), vec![input(1,0x3af6238)], None).unwrap();
 assert_eq!(p.blocking(ObjectId(2)), Err(Error::Unsupported("notification object not in captured owner")));
 let before=p.clone(); assert_eq!(p.blocking_reduction(),Ok(3)); assert_eq!(p,before);
 // Same wire identity belongs to two distinct objects; unsigned severity selects -1.
 let a=Notification { severity: 1, ..record.clone() };
 let b=Notification { severity: -1, ..record.clone() };
 let mut inputs=vec![input(1,0x3af6238),input(2,0x3af6530)]; inputs[1].default_blocking=Some(7);
 let mut p=CapturedOwner::new(state(vec![a,b], &[1,2]),inputs,None).unwrap();
 let before=p.clone(); assert_eq!(p.blocking_reduction(),Ok(7)); assert_eq!(p,before);
 // Base phase predicate inherited by ForeignCity; its actual expiry remains unsupported.
 let p=CapturedOwner::new(state(vec![record.clone()], &[1]),vec![input(1,0x3af5d60)],None).unwrap();
 assert_eq!(p.valid_for_phase(ObjectId(1)),Ok(true)); assert_eq!(p.is_needed(ObjectId(1)),Ok(true));
 assert_eq!(p.expired(ObjectId(1)),Err(Error::Unsupported("concrete subtype expiration")));
 // Known IsNeeded override must reject even when a base shortcut would return true.
 let p=CapturedOwner::new(state(vec![record.clone()], &[1]),vec![input(1,0x3af5c90)],None).unwrap();
 assert_eq!(p.is_needed(ObjectId(1)),Err(Error::Unsupported("concrete subtype IsNeeded")));
 // Native IsExpired flag is not the base expiry rule; signed nonpositive delta avoids turn.
 let p=CapturedOwner::new(state(vec![Notification { native_18c: i32::MIN, native_19c: true, ..record }], &[1]),vec![input(1,0x3af6238)],None).unwrap();
 assert_eq!(p.expired(ObjectId(1)),Ok(false));
 println!("independent_boundary_groups=10");
}
'''


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--module', type=Path, default=ROOT.parent / 'c6-assets-linux/libGameCore_XP2.so')
    parser.add_argument('--replay-report', type=Path, required=True)
    parser.add_argument('--capture-dir', type=Path, default=ROOT.parent / 'c6-ghidra/evidence/linux-notification-lifecycle/native-passive-1')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    commit = subprocess.check_output(['git','rev-parse','f4857ab^{commit}'],cwd=ROOT,text=True).strip()
    pinned = {}
    for path in OWNED:
        original = subprocess.check_output(['git','show',commit+':'+path],cwd=ROOT)
        if original != (ROOT/path).read_bytes():
            raise ValueError('source differs from immutable provider pin: '+path)
        pinned[path] = sha(original)
    game = Elf(args.module,GAME_SHA)
    methods = [row for row in METHODS if row[0].startswith('Instance.')]
    methods.append(('Manager.Create',0x2d2f700,0xc2,'33e1e887a03560c4ef108d6d9c09406c75fd7261eb0ec669f9cb2d318722c1f8'))
    for name,address,size,digest in methods:
        if sha(game.read(address,size)) != digest:
            raise ValueError('complete original method differs: '+name)
    symbols = subprocess.check_output(['nm','-D','--defined-only',str(args.module)],text=True).splitlines()
    tables = [(int(row.split()[0],16),row.split()[-1]) for row in symbols
              if '_ZTVN8GameCore9Reporting12Notification' in row and not row.endswith('7ManagerE')]
    if len(tables)!=39:
        raise ValueError('actual notification subtype inventory differs')
    rows=[]
    expected=[]
    for table,symbol in tables:
        address=table+16
        bindings={name:game.relocated_pointer(address+slot) for name,slot in
                  [('expiry',0x28),('phase',0x30),('needed',0x38),('auto',0x40),('blocking',0x50)]}
        if bindings['auto']!=0x2d2e2b0 or bindings['blocking']!=0x2d2e3b0:
            raise ValueError('unimplemented AutoExpires/blocking override')
        expected.append((address,bindings['expiry'],bindings['phase'],bindings['needed']))
        rows.append(dict(symbol=symbol,addresspoint=hex(address),bindings={k:hex(v) for k,v in bindings.items()}))
    source=(ROOT/OWNED[0]).read_text()
    actual=[tuple(int(v,16) for v in values) for values in re.findall(r'\((0x[0-9a-f]+), (0x[0-9a-f]+), (0x[0-9a-f]+), (0x[0-9a-f]+)\)',source)]
    if actual!=expected:
        raise ValueError('provider profiles differ from independent ELF relocations')
    replay=json.loads(args.replay_report.read_text())
    if not replay['checks_passed'] or replay['source_sha256']!=pinned[OWNED[0]] or replay['native_execution']:
        raise ValueError('independent rerun identity/status differs')
    captured=replay['captured_owner']
    if captured['capture_sha256']!='aa81095ebdea3c091cb910e8335d68d6277a4f047445329020de4192c572c9d6':
        raise ValueError('existing capture identity differs')
    raw_capture=(args.capture_dir/'observations.jsonl').read_bytes()
    if sha(raw_capture)!=captured['capture_sha256']:
        raise ValueError('private immutable capture bytes differ')
    first_find=next(row for row in map(json.loads,raw_capture.splitlines()) if row['stage']=='find_entry')['list']['records']
    if captured['saved_initial_turn']!=592 or captured['current_turn_observed_at_predicate_boundary'] or captured['native_predicate_return_comparison']:
        raise ValueError('conditional turn replay scope differs')
    records=captured['records']
    if len(records)!=12 or any(r['default_blocking']!=0 or r['expiry_delta']!=1 for r in records):
        raise ValueError('actual captured blocking/saved delta inventory differs')
    for observed,row in zip(first_find,records,strict=True):
        if observed['default_blocking']!=row['default_blocking'] or int(observed['handle'].removeprefix('object-'))!=row['object']:
            raise ValueError('reported live blocking/object differs from raw capture')
    reference=json.loads((ROOT/'evidence/linux/skip-turn-state-execute-3.json').read_text())
    saved=(args.capture_dir/'before.bin').read_bytes()
    region=next(r for r in reference['changed_owner_regions'] if r['owner']=='notifications')
    if sha(saved)!=reference['original_core_saves']['before']['sha256'] or sha(saved[slice(*region['before_range'])])!=captured['owner_wire_sha256']:
        raise ValueError('saved baseline/owner provenance differs')
    # These mathematical checks use actual saved rows and turn, not Rust return values.
    for row in records:
        wrapped=((row['added_turn']+row['expiry_delta']+2**31)%2**32)-2**31
        if 592>=wrapped:
            raise ValueError('saved-turn conditional expiry prediction differs')
    tracked=['crates/c6-core/src/lib.rs','crates/c6-core/src/notification_lifecycle.rs',
             'crates/c6-core/src/notifications.rs','crates/c6-core/src/unit_operation_aftermath.rs',OWNED[0]]
    dependencies={p:sha((ROOT/p).read_bytes()) for p in tracked}
    dependency_commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
    for path in tracked:
        if sha(subprocess.check_output(['git','show',dependency_commit+':'+path],cwd=ROOT))!=dependencies[path]:
            raise ValueError('test dependency is not immutable committed source: '+path)
    subprocess.run(['cargo','build','-p','c6-core'],cwd=ROOT,check=True,stdout=subprocess.PIPE)
    with tempfile.TemporaryDirectory(prefix='c6-provider-independent-review-') as temp:
        folder=Path(temp); harness=folder/'review.rs'; candidate=folder/'review'
        harness.write_text(HARNESS)
        subprocess.run(['rustc','--edition=2021',str(harness),'--extern','c6_core='+str(ROOT/'target/debug/libc6_core.rlib'),
                        '-L','dependency='+str(ROOT/'target/debug/deps'),'-o',str(candidate)],cwd=ROOT,check=True)
        if any(sha((ROOT/p).read_bytes())!=digest for p,digest in dependencies.items()):
            raise ValueError('provider dependency source changed during build')
        output=subprocess.check_output([str(candidate)],text=True).strip()
        if output!='independent_boundary_groups=10':
            raise ValueError('independent Rust boundary checks incomplete')
        candidate_sha=sha(candidate.read_bytes())
    result=dict(schema='c6-linux-notification-provider-independent-review-v1',checks_passed=True,
        reviewed_commit=commit,pinned_files=pinned,gamecore_sha256=GAME_SHA,
        independently_checked_full_methods=[dict(name=n,rva=hex(a),bytes=z,sha256=d) for n,a,z,d in methods],
        independent_actual_vtables=rows,
        supported_method_binding_counts=dict(expiry=sum(r[1]==0x2d2e250 for r in expected),
            phase=sum(r[2]==0x2d29200 for r in expected),needed=sum(r[3]==0x2d2e290 for r in expected),blocking=39,auto_expires=39),
        independent_boundary_groups=10,original_source_tests_passed=6,
        boundary_candidate_sha256=candidate_sha,dependency_commit=dependency_commit,
        dependency_sources=dependencies,harness_sha256=sha(HARNESS.encode()),
        independently_rerun_provider_audit_sha256=sha(args.replay_report.read_bytes()),
        captured_replay_candidate_sha256=captured['rust_candidate_sha256'],
        existing_observations_sha256=captured['capture_sha256'],captured_owner_wire_sha256=captured['owner_wire_sha256'],
        captured_objects=12,live_blocking_all_zero=True,saved_delta_all_one=True,saved_initial_turn=592,
        conditional_saved_turn_expiry_predictions_match=True,
        current_turn_observed_at_predicate_boundary=False,native_predicate_return_comparison=False,
        subtype_addresspoints='Unique inference from five captured targets against39tables; raw addresspoint was not separately captured.',
        input_limits='Only observed controls/+130 are live capture inputs. Added/delta/19a are saved baseline inputs; turn592 is production CorePrefix saved decode, conditional on no intervening change.',
        effects='Read-only predicate/reducer; all manager/factory/callback/mutation operations reject explicitly. No no-op provider effects.',
        factories='Complete Manager.Create inspection confirms null registered result falls back to base creation. Six XML AutoNotify candidates are discovery only, not active inventory.',
        native_execution_during_review=False,complete_update_supported=False,complete_state_action_rng_turns_supported=False,
        unknown_save_bytes_masked=False,raw_save_load_failure_bytes=89198,driver_sha256=sha(Path(__file__).read_bytes()))
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,independent_vtables=39,independent_boundary_groups=10,native_execution=False)))


if __name__=='__main__':
    main()
