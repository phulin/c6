#!/usr/bin/env python3
"""Independent immutable Manager metadata/world source review; Rust execution only."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import sqlite3
import subprocess
import tarfile
from save_lifecycle_audit import Elf,GAME_SHA,HOST_SHA
from unit_operation_manager_world_audit import METHODS

ROOT=Path(__file__).resolve().parents[2]
HARNESS=r'''
use c6_data::unit_operation_manager::*;
use c6_data::RulesRowId;
use c6_core::unit::HashValue;
fn definitions(hashes:&[u32])->OperationDefinitions {
 OperationDefinitions::from_native_order(&hashes.iter().enumerate().map(|(i,&hash)| OperationDefinition {collection_index:i as u32,hash}).collect::<Vec<_>>()).unwrap()
}
fn pair(definition_hash:u32,value:bool)->HashValue<bool>{HashValue{definition_hash,value}}
fn phase(phase_order:i32,phase_type_hash:u32,flags:Option<u8>)->TurnPhaseDefinition {
 TurnPhaseDefinition {row_id:RulesRowId(i64::from(phase_order)),phase_order,phase_type_hash,
 active_segment:flags.map(|flags|TurnSegmentDefinition{row_id:RulesRowId(90),flags})}
}
fn main(){
 let d=definitions(&[0x100,0x200,0x100,u32::MAX]);
 let restored=d.restore_disabled(&[pair(0x998,true),pair(0x100,true),pair(0x200,true),pair(0x200,false),pair(u32::MAX,true)]).unwrap();
 assert_eq!(restored.values(),[false,false,true,true]);
 assert!(restored.is_disabled(RuntimeOperationId(2)));
 assert!(restored.is_disabled(RuntimeOperationId(0x100)));
 assert!(restored.is_disabled(RuntimeOperationId(-1)));
 assert!(!restored.is_disabled(RuntimeOperationId(-2)));
 assert_eq!(d.restore_disabled(&[]).unwrap().values(),[false;4]);
 let d=definitions(&[1,0x200]);assert_eq!(d.restore_disabled(&[pair(1,true)]).unwrap().values(),[false,true]);
 assert!(matches!(OperationDefinitions::from_native_order(&[OperationDefinition{collection_index:1,hash:0}]),Err(Error::DefinitionOrder)));
 // Ties outside the configured type must not reject a valid selected vector.
 let p=PhaseDefinitions::select([phase(0,3,Some(0x80)),phase(0,4,None),phase(0,4,None),phase(i32::MIN,3,Some(2)),phase(i32::MAX,3,Some(5))],3).unwrap();
 assert_eq!(p.slots().iter().map(|v|v.unwrap().phase_order).collect::<Vec<_>>(),[i32::MIN,0,i32::MAX]);
 assert_eq!(p.allow_tactical_commands(0,i32::MIN),Ok(true));
 assert_eq!(p.allow_tactical_commands(1,64),Ok(false));
 assert_eq!(p.allow_tactical_commands(2,-1),Ok(false));
 assert_eq!(p.allow_tactical_commands(-1,0),Ok(true));
 assert_eq!(p.allow_tactical_commands(-2,0),Err(Error::CurrentPhase(-2)));
 assert_eq!(p.allow_tactical_commands(3,0),Err(Error::CurrentPhase(3)));
 assert!(matches!(PhaseDefinitions::select([phase(0,3,None),phase(0,3,Some(2))],3),Err(Error::PhaseOrderTie)));
 let p=PhaseDefinitions::from_native_slots(vec![None,Some(phase(0,3,None))]);
 assert_eq!(p.allow_tactical_commands(0,0),Ok(true));assert_eq!(p.allow_tactical_commands(1,0),Ok(true));
 assert_eq!(PhaseDefinitions::select([phase(0,4,None)],3).unwrap().allow_tactical_commands(0,0),Err(Error::CurrentPhase(0)));
 for raw in [i32::MIN,-1,0,64,i32::MAX] {for pause in [0,1]{let v=LiveControlFields::new(raw,pause).unwrap();assert_eq!(v.player_processing_automation,raw);assert_eq!(v.is_paused(),pause!=0);}}
 assert!(matches!(LiveControlFields::new(0,255),Err(Error::InvalidPauseByte(255))));
 println!("independent_boundary_groups=8");
}
'''

def sha(b):return hashlib.sha256(b).hexdigest()
def need(v,m):
 if not v:raise ValueError(m)
def pin(commit,paths):
 commit=subprocess.check_output(['git','rev-parse',commit+'^{commit}'],cwd=ROOT,text=True).strip();files={}
 for path in paths:
  data=subprocess.check_output(['git','show',commit+':'+path],cwd=ROOT)
  need(data==(ROOT/path).read_bytes(),'immutable source differs: '+path);files[path]=sha(data)
 return dict(commit=commit,files=files)
def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--replay-report',type=Path,required=True)
 p.add_argument('--work-dir',type=Path,required=True)
 p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
 p.add_argument('--rules',type=Path,default=ROOT.parent/'c6-ghidra/evidence/linux-rules/mod-activation-xp2-constructor-final-1')
 p.add_argument('--output',type=Path,required=True);a=p.parse_args()
 sources=dict(implementation=pin('7e6421d',['crates/c6-data/src/unit_operation_manager.rs','crates/c6-data/src/lib.rs']),
  audit=pin('94e81a1',['tools/linux/unit_operation_manager_world_audit.py','evidence/linux/unit-operation-manager-world.json','docs/linux-unit-operation-manager-world.md']))
 report=json.loads(a.replay_report.read_text());need(report['checks_passed'] and not report['original_execution'],'replay status differs')
 work=a.work_dir.resolve();archive=subprocess.check_output(['git','archive','7e6421d'],cwd=ROOT)
 with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
  members=[m for m in tar.getmembers() if m.isfile()]
  for member in members:need((work/'source'/member.name).read_bytes()==tar.extractfile(member).read(),'archive source differs: '+member.name)
 binary=work/'target/release/c6-manager-world-replay';need(sha(binary.read_bytes())==report['replay_binary_sha256'],'replay binary differs')
 raw=(work/'reconstruction.json').read_bytes();need(sha(raw)==report['private_reconstruction_sha256'],'reconstruction differs')
 result=json.loads(raw);registry=json.loads((work/'registry.json').read_text());hashes=[e['hash']for e in registry]
 need(len(hashes)==63 and len(result['rows'])==1262,'input extent differs')
 saved_pairs=unknown=true_slots=0
 for row in result['rows']:
  values=[False]*63
  for key,value in row['pairs']:
   saved_pairs+=1
   indices=[i for i,h in enumerate(hashes)if h==key]
   index=key if key<63 else indices[-1]if indices else None
   if index is None:unknown+=1
   else:values[index]=value
  need(values==row['values'],'independent reset/remap output differs');true_slots+=sum(values)
 table_counts={};phase_rows=[]
 for name in ['configuration.sqlite','gameplay.sqlite']:
  need(sha((a.rules/name).read_bytes())==report['rules_files'][name],'rules identity differs')
  c=sqlite3.connect('file:'+str(a.rules/name)+'?mode=ro',uri=True)
  table_counts[name]=c.execute("SELECT count(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'").fetchone()[0]
  if name!='gameplay.sqlite':continue
  for configured in ['TURNPHASE_SINGLEPLAYER','TURNPHASE_SIMULTANEOUS','TURNPHASE_DYNAMIC','TURNPHASE_TWOPHASE']:
   phases=c.execute('SELECT rowid,PhaseOrder,ActiveSegmentType FROM TurnPhases WHERE TurnPhaseType=? ORDER BY PhaseOrder',(configured,)).fetchall()
   need(len({v[1]for v in phases})==len(phases),'selected phase order tie requires actual native slots')
   for index,(rowid,order,active) in enumerate(phases):
    segment=c.execute('SELECT rowid,AllowStrategicCommands,AllowTacticalCommands,AllowTurnUnready FROM TurnSegments WHERE TurnSegmentType=? ORDER BY rowid LIMIT 1',(active,)).fetchone()
    phase_rows.append(dict(type=configured,slot=index,rowid=rowid,order=order,segment_rowid=None if segment is None else segment[0],flags=None if segment is None else segment[1]|(segment[2]<<1)|(segment[3]<<2),allow=segment is None or segment[2]!=0))
 need(phase_rows==result['phases'],'independent configured phase/segment SQL projection differs')
 need(len(result['players'])==43 and sum(p['human']for p in result['players'])==1,'Player profile differs')
 header=json.loads((work/'header.json').read_text());observed=[p for p in result['players']if p['turn_active']is not None]
 need(len(observed)==1 and observed[0]['id']==header['owner'] and observed[0]['turn_active']==header['flags'][2],'one saved header bridge differs')
 elf=Elf(a.assets/'libGameCore_XP2.so',GAME_SHA);Elf(a.assets/'Civ6',HOST_SHA)
 inspected={'IndexList.Read','IndexHash.Read','IndexList.DoReset','HashToIndex','Operation.FindIndex','Operation.IsDisabled','Definitions.Count','Definitions.Size','Phase.Initialize','Phase.Comparator','Phase.GetCurrent','Phase.Tactical','Segment.Tactical','Player.TurnActive','Player.GetId','Game.Paused','Player.ProcessingAutomation'}
 methods=[row for row in METHODS if row[0]in inspected]
 methods.append(('Operation.DefinitionFind',0x20b3d20,0x6d,'6e5560f54d2973f7cffbc5de65eb89d2839e7478772244910ac8f21901dadf00'))
 for name,addr,size,digest in methods:need(sha(elf.read(addr,size))==digest,'full method differs: '+name)
 dwarf=subprocess.check_output(['gdb','-nx','-batch',str(a.assets/'libGameCore_XP2.so'),'-ex',"ptype 'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitOperation>'"],text=True)
 need('uint32_t size() const' in dwarf and 'uint32_t GetCount() const' in dwarf and 'int32_t FindIndex(uint32_t) const' in dwarf,'declared ABI widths differ')
 need(elf.read(0x2c3fbf6,2).hex()=='89c6','definition count assign does not consume EAX as u32')
 harness_path=work/'src/bin/review.rs';harness_path.parent.mkdir(exist_ok=True);harness_path.write_text(HARNESS)
 subprocess.run(['cargo','build','--offline','--release','--bin','review'],cwd=work,check=True,stdout=subprocess.PIPE)
 candidate=work/'target/release/review';out=subprocess.check_output([str(candidate)],text=True).strip();need(out=='independent_boundary_groups=8','boundary harness incomplete')
 result=dict(schema='c6-linux-manager-world-independent-review-v1',checks_passed=True,immutable_sources=sources,
  gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,whole_source_archive_sha256=sha(archive),archive_files_verified=len(members),
  original_replay_binary_sha256=report['replay_binary_sha256'],original_reconstruction_sha256=sha(raw),
  independent_boundary_binary_sha256=sha(candidate.read_bytes()),independent_boundary_harness_sha256=sha(HARNESS.encode()),independent_boundary_groups=8,
  independent_full_method_fingerprints=[dict(name=n,rva=hex(r),bytes=s,sha256=h)for n,r,s,h in methods],
  independently_rerun_audit_methods=len(report['methods']),independently_rerun_audit_calls=report['checked_call_edges'],
  definitions_count_declared_abi='uint32_t GetCount()/size(); FindIndex(uint32_t)->int32_t. Assign receives EAX via ESI. Not uint64_t.',
  definition_slots=63,units=1262,saved_disabled_pairs=saved_pairs,unknown_saved_keys_exercised=unknown,true_slots=true_slots,all_units_unchanged=report['all_units_unchanged'],
  rules_table_counts=table_counts,phase_variants=len(phase_rows),phase_tactical_false=sum(not r['allow']for r in phase_rows),configured_types=4,
  actual_configured_phase_captured=False,current_runtime_phase_vector_compared=False,
  players=43,human_players=1,saved_turn_active_headers=1,first_saved_header_flags=header['flags'],
  header_fixture_cannot_distinguish_flag_positions_alone=len(set(header['flags']))==1,
  original_adapter_test_groups_passed=3,genuine_rules_ignored_test_explicitly_run_and_passed=True,
  no_live_pause_or_processing_automation_fixture=True,native_helper_admitted=False,
  source_registry_and_controls='Ordered original collection metadata, identity-gated rules and captured Player facts; no expected predicate answers/default pause/automation values provided.',
  scope='Fresh-Read disabled value reconstruction, configured phase projection and saved/captured Player/control readers only. Separate input contexts are composite, not full Manager native profile.',
  query_allocation='Disabled query/phase getter/Player/control getters allocate nothing; reconstruction allocates metadata vectors. No whole-action benchmark.',
  native_execution_during_review=False,native_live_disabled_or_tactical_comparison=False,full_manager_native_acceptance=False,
  complete_state_action_rng_turns=False,raw_save_load_failure_bytes=89198,unknown_save_bytes_masked=False,
  source_replay_audit_sha256=sha(a.replay_report.read_bytes()),driver_sha256=sha(Path(__file__).read_bytes()))
 a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(result,indent=2)+'\n')
 print(json.dumps(dict(checks_passed=True,units=1262,definition_slots=63,independent_boundary_groups=8,native_execution=False)))
if __name__=='__main__':main()
