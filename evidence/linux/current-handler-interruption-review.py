#!/usr/bin/env python3
"""Independent current-handler source/capture review; executes archived Rust only."""
import argparse
from collections import Counter
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools/linux'))
from save_lifecycle_audit import Elf,GAME_SHA,HOST_SHA
from current_handler_interruption_audit import METHODS

FACTS_SHA='3fd5b6e64b5f81e06fc06187b9dae53691ddb536a6fc0b03f6f728d759dea585'
RUNTIME_SHA='5b68cfff7a3c88033694df0ef5f882a43dc780700e5ca4b194e312ca115b3740'
REGISTRY_SHA='d8a43667523e9385bfa9ac35594027609b26d7755551c0bab46d226917899901'
HARNESS=r'''
use c6_core::unit_operation_interruption::*;
use c6_core::messages::{HandlerId,OperationHash};
fn main(){
 // Original-image projection only; runtime slot contents were not captured.
 let identity=ObservationIdentity {gamecore_sha256:"a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa",configuration_sha256:"CONFIG",capture_sha256:"CAPTURE"};
 let slots=vec![SLOTS];
 for (index,slot) in slots.iter().enumerate(){
  let slot=slot.unwrap();let rva=match slot.interruption_target{InterruptionTarget::OriginalImagePrediction(rva)=>rva,_=>panic!("prediction promoted to live")};
  let mut one=vec![None;slots.len()];one[index]=Some(slot);
  assert_eq!(ObservedRegistry::new(identity,&one,&[]).unwrap_err(),Error::UnobservedLiveBinding(rva));
 }
 // Explicit diagnostic evidence shapes below are synthetic, not new live observations.
 let diagnostic=ObservationIdentity {gamecore_sha256:identity.gamecore_sha256,
  configuration_sha256:"0000000000000000000000000000000000000000000000000000000000000000",
  capture_sha256:"1111111111111111111111111111111111111111111111111111111111111111"};
 let mut slots=vec![None;0x800];slots[0]=Some(ObservedHandler {runtime_id:HandlerId(0),operation_hash:OperationHash(0x800),interruption_target:InterruptionTarget::LiveSlotObservation(0x2d95660)});
 let hashes=[
  ObservedHashResolution{hash:OperationHash(0x7ff),runtime_id:Some(HandlerId(0))},
  ObservedHashResolution{hash:OperationHash(0x800),runtime_id:Some(HandlerId(0))},
  ObservedHashResolution{hash:OperationHash(0x80000000),runtime_id:Some(HandlerId(0))},
  ObservedHashResolution{hash:OperationHash(0x900),runtime_id:None},
  ObservedHashResolution{hash:OperationHash(0x901),runtime_id:Some(HandlerId(1))},
 ];
 let registry=ObservedRegistry::new(diagnostic,&slots,&hashes).unwrap();
 assert_eq!(registry.handler(HandlerId(0x7ff)),Ok(None));
 assert_eq!(registry.saved_hash_runtime_id(OperationHash(0x7ff)),Ok(HandlerId(0)));
 assert_eq!(registry.handler(HandlerId(0x800)).unwrap().unwrap().identity().runtime_id,HandlerId(0));
 assert!(registry.handler(HandlerId(i32::MIN)).unwrap().unwrap().can_be_interrupted());
 assert_eq!(registry.handler(HandlerId(0x801)),Err(Error::UnobservedHash(OperationHash(0x801))));
 assert_eq!(registry.handler(HandlerId(0x900)),Ok(None));
 assert_eq!(registry.saved_hash_runtime_id(OperationHash(0x900)),Err(Error::UnresolvedSavedHash(OperationHash(0x900))));
 assert_eq!(registry.handler(HandlerId(0x901)),Ok(None));
 assert_eq!(registry.saved_hash_runtime_id(OperationHash(0x901)),Ok(HandlerId(1)));
 let too_short=[None];let short=ObservedRegistry::new(diagnostic,&too_short,&[]).unwrap();
 assert_eq!(short.handler(HandlerId(1)),Err(Error::NumericSlotOutOfBounds(HandlerId(1))));
 let invalid=[ObservedHashResolution{hash:OperationHash(0x900),runtime_id:Some(HandlerId(0x800))}];
 assert_eq!(ObservedRegistry::new(diagnostic,&slots,&invalid).unwrap_err(),Error::InvalidHashResolution(OperationHash(0x900)));
 let mut malformed=diagnostic;malformed.capture_sha256="unobserved";
 assert_eq!(ObservedRegistry::new(malformed,&[],&[]).unwrap_err(),Error::InvalidEvidenceIdentity);
 let mismatched=[Some(ObservedHandler{runtime_id:HandlerId(1),operation_hash:OperationHash(9),interruption_target:InterruptionTarget::LiveSlotObservation(0x2d95660)})];
 assert_eq!(ObservedRegistry::new(diagnostic,&mismatched,&[]).unwrap_err(),Error::SlotIdentity(0));
 println!("source_predictions_rejected=63;independent_boundary_groups=8");
}
'''

def sha(b):return hashlib.sha256(b).hexdigest()
def need(v,m):
 if not v:raise ValueError(m)
def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--export-commit',required=True)
 p.add_argument('--source-commit',required=True)
 p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
 p.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra')
 p.add_argument('--output',type=Path,required=True);a=p.parse_args()
 source_commit=subprocess.check_output(['git','rev-parse',a.source_commit+'^{commit}'],cwd=ROOT,text=True).strip();files={}
 paths=['crates/c6-core/src/unit_operation_interruption.rs','tools/linux/current_handler_interruption_audit.py','docs/linux-current-handler-interruption.md','evidence/linux/current-handler-interruption.json']
 for path in paths:
  data=subprocess.check_output(['git','show',source_commit+':'+path],cwd=ROOT);need(data==(ROOT/path).read_bytes(),'source differs from immutable interruption commit: '+path);files[path]=sha(data)
 game=Elf(a.assets/'libGameCore_XP2.so',GAME_SHA);Elf(a.assets/'Civ6',HOST_SHA)
 for name,address,size,digest in METHODS:need(sha(game.read(address,size))==digest,'full target method differs: '+name)
 manager_bindings=[(0x88,0x2da1650),(0x98,0x2da16f0),(0xa0,0x2da1720),(0xd8,0x2da2a60)]
 for slot,target_rva in manager_bindings:need(game.relocated_pointer(0x3aff880+slot)==target_rva,'static Manager delegated slot differs')
 need(game.read(0x2d95660,3)==bytes.fromhex('b001c3') and game.read(0x2da0920,3)==bytes.fromhex('31c0c3'),'constant leaves differ')
 capture=a.private_root/'evidence/linux-unit-operation-manager-metadata/native-1';facts=(capture/'facts.jsonl').read_bytes();runtime=(capture/'runtime.json').read_bytes()
 need(sha(facts)==FACTS_SHA and facts==(capture/'facts-repeat.jsonl').read_bytes(),'closed repeated observation identity differs')
 need(sha(runtime)==RUNTIME_SHA,'closed runtime identity differs');runtime=json.loads(runtime)
 configuration=runtime['configuration_sha256'];need(sha(json.dumps(runtime['configuration'],sort_keys=True,separators=(',',':')).encode())==configuration,'configuration canonical identity differs')
 rows=[json.loads(line)for line in facts.splitlines()];profile=next(r for r in rows if r['kind']=='profile');definitions=next(r for r in rows if r['kind']=='operation_definitions')
 entries=definitions['entries'];need(definitions['count']==definitions['handler_slots']==len(entries)==63,'observed registry extent differs')
 registry_raw=(a.private_root/'evidence/linux-operation-registry/native-1.json').read_bytes();need(sha(registry_raw)==REGISTRY_SHA,'earlier registry identity differs');previous=json.loads(registry_raw)
 inventory=[]
 for ordinal,(row,old)in enumerate(zip(entries,previous['mapping']['entries'],strict=True)):
  need(row['ordinal']==row['runtime_id']==row['handler_id']==ordinal,'observed numeric slot/runtime identity differs')
  need(row['handler_pointer']!=0 and row['hash']==row['handler_hash']==old['hash']==old['handler_hash'],'observed actual handler/hash differs')
  need(old['id']==old['resolved_id']==old['handler_id']==ordinal and old['handler_present'] and old['lookup_matches'],'independent observed registry numeric identity differs')
  vptr=row['handler_vptr']-profile['module_base'];need(vptr==old['vtable_rva'],'independent handler vptr profile differs')
  target=game.relocated_pointer(vptr+0x20);need(target in [0x2d95660,0x2da0920],'unimplemented actual interruption binding')
  inventory.append(dict(runtime_id=row['runtime_id'],operation_hash=row['hash'],observed_vptr_rva=hex(vptr),source_image_target_rva=hex(target)))
 need(Counter(row['source_image_target_rva']for row in inventory)=={'0x2d95660':62,'0x2da0920':1},'original image predicted families differ')
 census=[];by_hash={f"{row['operation_hash']:08x}":row['source_image_target_rva']for row in inventory}
 for sample in previous['corpus']:
  counts=Counter()
  for key,count in sample['queue_counts'].items():
   if key=='empty':counts['empty']+=count
   else:counts['predicted_false_leaf'if by_hash[key]=='0x2da0920'else'predicted_true_leaf']+=count
  need(counts=={'empty':954,'predicted_false_leaf':185,'predicted_true_leaf':123},'saved census source-family attribution differs')
  census.append(dict(save_sha256=sample['save_sha256'],counts=dict(counts),actual_runtime_dispatch_observed=False))
 export_commit=subprocess.check_output(['git','rev-parse',a.export_commit+'^{commit}'],cwd=ROOT,text=True).strip()
 exported_lib=subprocess.check_output(['git','show',export_commit+':crates/c6-core/src/lib.rs'],cwd=ROOT)
 need(b'pub mod unit_operation_interruption;'in exported_lib,'actual production export absent from committed source')
 subprocess.run(['git','merge-base','--is-ancestor','ee021ff',export_commit],cwd=ROOT,check=True)
 exported_source=subprocess.check_output(['git','show',export_commit+':'+paths[0]],cwd=ROOT);need(sha(exported_source)==files[paths[0]],'export tree contains a different interruption implementation')
 archive=subprocess.check_output(['git','archive',export_commit],cwd=ROOT)
 slot_expr=','.join('Some(ObservedHandler{runtime_id:HandlerId('+str(r['runtime_id'])+'),operation_hash:OperationHash('+str(r['operation_hash'])+'),interruption_target:InterruptionTarget::OriginalImagePrediction('+r['source_image_target_rva']+')})'for r in inventory)
 harness=HARNESS.replace('CONFIG','0'*64).replace('CAPTURE','1'*64).replace('SLOTS',slot_expr)
 env=os.environ.copy()
 for k in ['CARGO_TARGET_DIR','CARGO_BUILD_TARGET','RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER']:env.pop(k,None)
 with tempfile.TemporaryDirectory(prefix='c6-interruption-independent-review-')as temp:
  folder=Path(temp);source=folder/'source';source.mkdir()
  with tarfile.open(fileobj=io.BytesIO(archive))as tar:tar.extractall(source,filter='data')
  target=folder/'target'
  subprocess.run(['cargo','build','--offline','--locked','-p','c6-core','--target-dir',str(target)],cwd=source,env=env,check=True,stdout=subprocess.PIPE)
  tests=subprocess.check_output(['cargo','test','--offline','--locked','-p','c6-core','--target-dir',str(target),'unit_operation_interruption'],cwd=source,env=env,text=True);need('2 passed;'in tests,'archived module source tests incomplete')
  fixture=folder/'review.rs';candidate=folder/'review';fixture.write_text(harness)
  subprocess.run(['rustc','--edition=2021',str(fixture),'--extern','c6_core='+str(target/'debug/libc6_core.rlib'),'-L','dependency='+str(target/'debug/deps'),'-o',str(candidate)],cwd=source,env=env,check=True)
  output=subprocess.check_output([str(candidate)],env=env,text=True).strip();need(output=='source_predictions_rejected=63;independent_boundary_groups=8','prediction rejection or independent boundary fixture failed');candidate_sha=sha(candidate.read_bytes())
  with tarfile.open(fileobj=io.BytesIO(archive))as tar:
   for member in tar.getmembers():
    if member.isfile():need((source/member.name).read_bytes()==tar.extractfile(member).read(),'committed source changed during archived build: '+member.name)
 result=dict(schema='c6-linux-current-handler-interruption-independent-review-v1',checks_passed=True,
  source_commit=source_commit,pinned_files=files,production_export_commit=export_commit,exported_lib_sha256=sha(exported_lib),complete_build_archive_sha256=sha(archive),
  gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,configuration_sha256=configuration,
  complete_target_methods=[dict(name=n,rva=hex(r),bytes=s,sha256=h)for n,r,s,h in METHODS],
  static_manager_bindings=[dict(slot=hex(s),target_rva=hex(t))for s,t in manager_bindings],
  closed_facts_sha256=FACTS_SHA,closed_runtime_sha256=RUNTIME_SHA,independent_registry_sha256=REGISTRY_SHA,
  source_projected_handlers=inventory,observed_handler_count=63,observed_null_slots=0,observed_small_hashes=sum(r['operation_hash']<0x800 for r in inventory),original_image_predicted_families=dict(Counter(r['source_image_target_rva']for r in inventory)),
  saved_queue_source_family_census=census,
  immutable_source_tests_passed=2,audit_rejection_tests_passed=2,independent_boundary_groups=8,source_prediction_admission_rejections=63,
  boundary_candidate_sha256=candidate_sha,harness_sha256=sha(harness.encode()),
  synthetic_boundaries=['unsigned0x7ff index before hash;0x800 hash path','high-bit negative integer hash bits','observed hash miss vs unobserved hash','observed null slot vs mapped null slot vs invalid numeric extent','saved hash resolution separate from handler small-index lookup','non-index reverse answer rejected','malformed evidence identity rejected','slot identity mismatch rejected'],
  census_source='Saved queue corpus attribution through original-image predicted families only:185trade/123base/954empty per captured Save; runtime targets and native interruption outcomes unobserved.',
  evidence_limit='Identity strings are syntax/provenance references, not authentication of pointer ownership or snapshot freshness. Retain actual owners and verify live delegated Manager slots before admission.',
  integration='Same-ID bypass, empty-front and actually-null-current-handler semantics remain owned by existing Manager orchestration. This module does not resolve live queue fronts or install a ManagerQueries provider.',
  manager_script_lock_repair='7d3c026 reviewed separately; interruption orchestration unchanged. This review builds actual committed export archive without injected exports.',
  actual_runtime_handler_slot20_captured=False,native_target_binding_admitted=False,source_image_projection_uses_synthetic_test_provenance=True,live_manager_vptr_captured=False,same_context_native_queue_front_captured=False,native_predicate_comparison=False,
  native_execution_during_review=False,full_manager_supported=False,complete_state_action_rng_turns_supported=False,
  unknown_save_bytes_masked=False,raw_save_load_failure_bytes=89198,driver_sha256=sha(Path(__file__).read_bytes()))
 a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(result,indent=2)+'\n')
 print(json.dumps(dict(checks_passed=True,source_predictions_rejected=63,archived_source_tests=2,independent_boundary_groups=8,native_execution=False)))
if __name__=='__main__':main()
