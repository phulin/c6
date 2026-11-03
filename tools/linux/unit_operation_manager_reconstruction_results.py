#!/usr/bin/env python3
"""Pre-answer admitted, terminal-only Manager raw-state comparison; no native execution."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import unittest
import unit_operation_manager_reconstruction as base
ROOT=base.ROOT
ADMISSION_PATH=ROOT/'evidence/linux/unit-operation-manager-reconstruction-admission.json'
CAPTURE=ROOT.parent/'c6-ghidra/evidence/linux-unit-operation-manager-metadata/native-1'
EXPECTED=dict(profile_sha256='aa38b5edfe6f4c5ffc8fc25d079e9739ab7e3d75d3998c723dbbdaece5e08aff',prepared_sha256='87f3d32b10a7edc2af76bf317b2acf8187692e811b9c24edee9d430df0e061ab',configuration_file_sha256='6b34093bbf7b7407972758daeba9e7d2f2885472a48cd6ba7097a1d2d2b78cdf',preload_sha256='ca4a1212111b0bcd3bd708bd4c6d9c3f2ec7f6ff59ccf732ea33996e68f79e5f',candidate_manifest_sha256='348ee77d2d1ea9c8eaaf74d1a5abc5e1228baca16bee8536b2fc56ad2604f186')
FILES=dict(output='facts.jsonl',repeat_output='facts-repeat.jsonl',control_output='controls.jsonl',before='before.bin',control_save='control.bin',after='after.bin',repeat='after-repeat.bin')
SAVES=[('before','before.bin'),('control_save','control.bin'),('after','after.bin'),('repeat','after-repeat.bin'),('outer_save','save.bin'),('outer_repeat','repeat.bin')]
TRACE=ROOT.parent/'c6-ghidra/evidence/linux-save-normalization/checkpoint-203-owners/trace-benchmark-1.jsonl.gz'
RULES=ROOT.parent/'c6-ghidra/evidence/linux-rules/mod-activation-xp2-constructor-final-1'
MANIFEST=ROOT/'evidence/linux/unit-operation-manager-candidate.json'
PRELOAD='unit-operation-manager-metadata-preload.so'
def exact(a,b,reason):
 if isinstance(a,set):a=sorted(a)
 if isinstance(b,set):b=sorted(b)
 base.require(base.canonical(a)==base.canonical(b),reason)
def identity(path,filename=True):
 p=Path(path);v=dict(sha256=base.digest(p),bytes=p.stat().st_size)
 if filename:v['filename']=p.name
 return v
def json_rows(path):return [base.parse(line)for line in Path(path).read_bytes().splitlines()]
def imported_sources():
 found={}
 for module in list(sys.modules.values()):
  p=getattr(module,'__file__',None)
  if p:
   p=Path(p).resolve()
   if p.is_relative_to(ROOT/'tools/linux')and p.suffix=='.py':found[str(p.relative_to(ROOT))]=base.digest(p)
 return found
def check_change(before,after,terminal_required=True):
 base.require(before['runtime_sha256']is None,'pre-answer baseline already contains runtime')
 altered=copy.deepcopy(after);runtime=altered.get('runtime_sha256');altered['runtime_sha256']=None
 exact(altered,before,'admission changed outside sole terminal runtime hash')
 if terminal_required:base.require(type(runtime)is str and len(runtime)==64 and all(c in '0123456789abcdef'for c in runtime),'terminal-cleaned runtime not yet admitted')
 return runtime
def baseline(commit):
 raw=subprocess.check_output(['git','show',commit+':'+str(ADMISSION_PATH.relative_to(ROOT))],cwd=ROOT)
 value=base.parse(raw);base.require(value['runtime_sha256']is None,'baseline is not pre-answer admission');return value,base.sha(raw)
def check_prepared(a):
 for k,v in EXPECTED.items():exact(a[k],v,'admission constant differs: '+k)
 exact(base.digest(CAPTURE/'prepared.json'),a['prepared_sha256'],'prepared identity differs')
 exact(base.digest(CAPTURE/'configuration.json'),a['configuration_file_sha256'],'configuration file differs')
 prepared=base.load(CAPTURE/'prepared.json');config=base.load(CAPTURE/'configuration.json')
 exact(prepared['schema'],'c6-linux-native-profile-prepared-v1','prepared schema');base.require(prepared['native_execution_performed']is False and prepared['returncode']is None,'prepare-only stage differs')
 exact(config,prepared['configuration'],'prepared/durable configuration differs');exact(base.sha(base.canonical(config)),a['profile_sha256'],'profile identity differs');exact(prepared['configuration_sha256'],a['profile_sha256'],'prepared profile differs')
 exact(config['assets']['libGameCore_XP2.so'],base.GAME,'module identity');exact(config['assets']['Civ6'],base.HOST,'host identity')
 binding=config['lower_core_unit_operation_manager_metadata']
 exact(binding['mode'],'read_only_manager_raw_metadata','metadata mode')
 for k,v in dict(raw_passes=2,internal_core_saves=4,expected_outputs_used_as_inputs=False,disabled_vectors_are_reconstruction_targets=True,phase_and_controls_are_raw_inputs=True,full_manager_called=False,script_callbacks_called=False,complete_rng_inventory=False,native_acceptance_pending=True).items():exact(binding[k],v,'metadata boundary '+k)
 for k,v in dict(bounded_action_mode=None,core_component_mode=None,pre_controller_registration=False,cleanup_requested=True).items():exact(config[k],v,'native action/lifecycle scope '+k)
 exact(base.digest(MANIFEST),a['candidate_manifest_sha256'],'candidate manifest differs')
 exact(base.digest(CAPTURE/'candidate-manifest.json'),a['candidate_manifest_sha256'],'durable candidate manifest differs')
 manifest,binary=base.candidate_gate(MANIFEST);exact(binding['reconstruction_manifest'],manifest,'configuration candidate binding');exact(binding['reconstruction_manifest_sha256'],a['candidate_manifest_sha256'],'configuration manifest hash')
 exact(base.digest(CAPTURE/PRELOAD),a['preload_sha256'],'preload differs');exact(config['preload_sha256'],a['preload_sha256'],'configuration preload differs')
 base.require(len(config['sources'])==60,'native source extent')
 for name,h in config['sources'].items():
  p=Path(name);base.require(not p.is_absolute()and '..'not in p.parts,'source path escape');exact(base.digest(CAPTURE/'source'/p),h,'archived native source differs: '+name)
 exact(base.digest(CAPTURE/'source/tools/linux/mod_activation_probe.py'),config['runner_sha256'],'archived runner differs')
 exact(prepared['execution_paths']['preload'],str(CAPTURE/PRELOAD),'prepared preload path')
 exact(prepared['execution_paths']['unit_operation_manager_metadata'],{k:str(CAPTURE/v)for k,v in FILES.items()},'prepared artifact paths')
 for name,h in a['checker_sources'].items():exact(base.digest(ROOT/name),h,'frozen checker/import differs: '+name)
 exact(base.sha(base.read(TRACE)),base.TRACE,'attributed owner trace differs');exact(base.digest(TRACE),a['trace_file_sha256'],'compressed trace differs')
 for name,h in a['rules_files'].items():exact(base.digest(RULES/name),h,'genuine rules changed')
 return prepared,config,manifest,binary

def check_rng(rows,profile):
 base.require(len(rows)==2,'control framing')
 exact(rows[0],dict(kind='header',schema='c6-linux-unit-operation-manager-controls-v1',profile_sha256=profile,full_manager_called=False,script_callbacks_called=False),'control profile/scope')
 s=rows[1];exact(set(s),set(['kind','status','known_rng_equal','complete_rng_inventory','rng_before','sync_rng_after','map_rng_after']),'control summary fields')
 exact(s['kind'],'summary','control terminal');exact(s['status'],0,'control failed');exact(s['known_rng_equal'],True,'known RNG changed');exact(s['complete_rng_inventory'],False,'RNG inventory overstated')
 for name,n in [('rng_before',128),('sync_rng_after',64),('map_rng_after',64)]:base.require(type(s[name])is str and len(s[name])==n and all(c in '0123456789abcdef'for c in s[name]),'RNG raw extent/encoding')
 exact(s['rng_before'],s['sync_rng_after']+s['map_rng_after'],'RNG bytes changed')

def check_policy(rows):
 expected=[dict(event='begin',reporting_observer_profile='proven_skipturn_callers_v2',UnitOperation_Execute_virtual_interposed=True,hook_owner_preload=True,target_owner_preload=True),dict(event='original_virtual_binding',name='UnitManager_MakeRequest',interposed=True,hook_owner_preload=True,target_owner_preload=True)]
 for name in ['BroadcastMessage','GameProcessor_Update','GlobalAi_ProcessAi','ReceiveMessage','EventSend_identity','QueueMessage','EventSend_identity_two_int','Diplomatic_ProcessAction','AIControl_DoTurn','Diplomatic_ProcessDeal','Diplomacy_ProcessStatement','Game_Update','EventSend_identity_int','EventSend_value_value','EventSend_type','EventSend_two_int']:expected.append(dict(event='original_PLT_binding',name=name,interposed=True,hook_owner_preload=True,target_owner_preload=True))
 expected.append(dict(event='end',queued=0,received=0,unit_executed=0,reports=0));exact(rows,expected,'unexpected/missing action or policy observation')

def terminal(runtime,config,a):
 for key in ['fixture_success','framework_boundary_returned','framework_cleanup_proven','client_cleanup_proven','initialized_game']:exact(runtime[key],True,'native terminal stage failed: '+key)
 exact(runtime['returncode'],0,'native return code');exact(runtime['native_boundary_result'],0,'native boundary result');exact(runtime['configuration'],config,'runtime configuration changed');exact(runtime['configuration_sha256'],a['profile_sha256'],'runtime profile')
 stages=runtime['lifecycle_stages']
 for key in ['native_context_initialized_observed','native_core_load_completed','game_startup_returned','game_shutdown_returned','controller_cache_initialized','controller_uninitialized_cleanly','client_unpublished_observed']:exact(stages[key],True,'lifecycle stage incomplete: '+key)
 exact(stages['native_core_load_result'],0,'Core load failed');exact(stages['framework_cleanup_result'],0,'framework cleanup failed')
 base.require('C6_LINUX_ACTION_LIFECYCLE uninitialize_return initialized 0 context_bound 0 executing 0 cleanup 1 dispatch_mode 0 requested_execution 0 core_component_mode 0'in runtime['stderr'].splitlines(),'terminal original cleanup footer missing')
 exact(runtime['execution_paths']['preload'],str(CAPTURE/PRELOAD),'runtime preload path')
 exact(runtime['execution_paths']['unit_operation_manager_metadata'],{k:str(CAPTURE/v)for k,v in FILES.items()},'runtime metadata paths')
 exact(set(runtime['unit_operation_manager_metadata_artifacts']),set(FILES),'missing/extra raw/control/internal artifacts')
 for label,name in FILES.items():exact(identity(CAPTURE/name),runtime['unit_operation_manager_metadata_artifacts'][label],'raw artifact changed: '+name)
 exact(set(runtime['game_saves']),{'save','repeat'},'outer Save coverage');exact(set(runtime['execution_paths']['game_outputs']),{'save','repeat'},'outer Save paths')
 controls=[]
 for label,name in SAVES:
  row=identity(CAPTURE/name);exact(row['sha256'],base.CORE,'raw Save differs; no byte masking');exact(row['bytes'],80657462,'raw Save size differs')
  if label.startswith('outer_'):
   role=label.removeprefix('outer_');exact(runtime['execution_paths']['game_outputs'][role],str(CAPTURE/name),'outer Save path');exact(row,runtime['game_saves'][role],'outer Save report')
  controls.append(dict(role=label,**row,raw_differences_from_before=0))
 check_rng(json_rows(CAPTURE/'controls.jsonl'),a['profile_sha256'])
 exact(set(runtime['action_trace_files']),{'events.jsonl'},'policy trace coverage')
 exact(runtime['execution_paths']['action_trace_directory'],str(CAPTURE/'trace'),'trace path')
 exact(identity(CAPTURE/'trace/events.jsonl',False),runtime['action_trace_files']['events.jsonl'],'policy trace hash/bytes')
 check_policy(json_rows(CAPTURE/'trace/events.jsonl'))
 return controls

def strict_pass(path):
 f=base.facts(path)
 defs=f['definitions'];base.require(defs['count']==len(defs['entries'])and defs['handler_slots']==defs['count'],'definition slot/count coverage')
 for i,d in enumerate(defs['entries']):
  exact(d['ordinal'],i,'definition order');exact(d['collection_index'],i,'definition index')
  for key in ['hash','numeric_find_index','hash_find_index','numeric_hash_to_index','hash_hash_to_index','runtime_id','handler_pointer']:base.require(type(d[key])is int,'definition scalar type')
 for unit in f['units']:
  for key in ['owner','ordinal','id','size','capacity','default_raw']:base.require(type(unit[key])is int,'unit raw scalar type')
  base.require(0<=unit['size']<=unit['capacity']<=65536 and 0<=unit['default_raw']<=255,'native vector extent/default byte')
  for name,n in [('list_raw',64),('values_raw',2*unit['size'])]:base.require(type(unit[name])is str and len(unit[name])==n and all(c in '0123456789abcdef'for c in unit[name]),'raw vector hex extent')
 for p in f['players']:base.require(type(p['turn_active_raw'])is int and p['turn_active_raw']in (0,1),'noncanonical live turn-active byte')
 return f

def freeze_admission():
 base.require(not ADMISSION_PATH.exists(),'admission already exists')
 absent=[*FILES.values(),'save.bin','repeat.bin','runtime.json','trace/events.jsonl']
 base.require(all(not (CAPTURE/n).exists()for n in absent),'original output already present; cannot claim pre-answer freeze')
 a=dict(schema='c6-linux-unit-operation-manager-reconstruction-admission-v1',**EXPECTED,capture_path=str(CAPTURE),runtime_sha256=None,checker_sources=imported_sources(),trace_file_sha256=base.digest(TRACE),rules_files={n:base.digest(RULES/n)for n in ['configuration.sqlite','gameplay.sqlite']},native_outputs_present_at_freeze=False,post_capture_allowed_changed_fields=['runtime_sha256'],expected_outputs_used_as_inputs=False,full_manager_called=False,complete_state_actions_rng_turns=False)
 check_prepared(a)
 with ADMISSION_PATH.open('x')as stream:json.dump(a,stream,indent=2);stream.write('\n')
 print(json.dumps(dict(admission_sha256=base.digest(ADMISSION_PATH),runtime_sha256=None,original_outputs_absent=True)))

def pin_runtime(a):
 admission=base.load(ADMISSION_PATH);before,before_hash=baseline(a.frozen_admission_commit);check_change(before,admission,False);base.require(admission['runtime_sha256']is None,'runtime already pinned')
 _,config,_,_=check_prepared(admission);runtime=base.load(CAPTURE/'runtime.json');terminal(runtime,config,admission)
 updated=copy.deepcopy(admission);updated['runtime_sha256']=base.digest(CAPTURE/'runtime.json');check_change(before,updated)
 ADMISSION_PATH.write_text(json.dumps(updated,indent=2)+'\n')
 print(json.dumps(dict(runtime_sha256=updated['runtime_sha256'],pre_capture_admission_sha256=before_hash,sole_changed_field='runtime_sha256')))

def compare_capture(a):
 admission=base.load(ADMISSION_PATH);before,before_hash=baseline(a.frozen_admission_commit);runtime_hash=check_change(before,admission)
 # Fail closed on pending runtime BEFORE opening original facts or runtime.
 exact(base.digest(CAPTURE/'runtime.json'),runtime_hash,'unadmitted terminal runtime')
 _,config,manifest,binary=check_prepared(admission);runtime=base.load(CAPTURE/'runtime.json');controls=terminal(runtime,config,admission)
 directory=a.diagnostics.resolve();directory.mkdir(parents=True,exist_ok=False)
 first_bytes=(CAPTURE/'facts.jsonl').read_bytes();second_bytes=(CAPTURE/'facts-repeat.jsonl').read_bytes();exact(first_bytes.hex(),second_bytes.hex(),'complete raw metadata passes differ; no masking')
 trace=base.read(TRACE);data=(CAPTURE/'before.bin').read_bytes();passes=[];input_hashes=[];output_hashes=[]
 for number,name in enumerate(['facts.jsonl','facts-repeat.jsonl']):
  f=strict_pass(CAPTURE/name);work=directory/f'pass-{number}';work.mkdir()
  request=base.make_request(data,trace,f,work,admission['profile_sha256'],RULES);request_bytes=base.canonical(request);request_path=work/'input.json';request_path.write_bytes(request_bytes)
  process=subprocess.run([str(binary),str(request_path)],capture_output=True,check=False);(work/'candidate.stdout').write_bytes(process.stdout);(work/'candidate.stderr').write_bytes(process.stderr)
  base.require(process.returncode==0,'candidate unsupported/malformed; diagnostics retained: '+str(work))
  calculated=base.parse(process.stdout)
  exact(calculated['configuration_sha256'],admission['profile_sha256'],'candidate profile');exact(calculated['profile_sha256'],admission['profile_sha256'],'candidate source profile');exact(calculated['core_sha256'],base.CORE,'candidate Core identity');exact(calculated['trace_sha256'],base.TRACE,'candidate trace identity')
  for key in ['expected_outputs_used_as_inputs','native_execution','native_predicate_comparison','full_manager_support','complete_state_actions_rng_turns']:exact(calculated[key],False,'candidate scope differs: '+key)
  phase=calculated['phases'];base.require(phase['status']=='supported_unique_order'and phase['tactical_permission']['status']=='source_only','unsupported phase rows cannot pass')
  comparison=base.compare(calculated,strict_pass(CAPTURE/name));(work/'comparison.json').write_text(json.dumps(comparison,indent=2)+'\n');passes.append(comparison);input_hashes.append(base.sha(request_bytes));output_hashes.append(base.sha(process.stdout))
 exact(input_hashes[0],input_hashes[1],'independent raw passes project different inputs');exact(output_hashes[0],output_hashes[1],'candidate independent repeats differ');exact(base.digest(binary),manifest['candidate_sha256'],'candidate changed during comparison')
 report=dict(schema='c6-linux-unit-operation-manager-native-reconstruction-v1',status='equal',profile_sha256=admission['profile_sha256'],runtime_sha256=runtime_hash,pre_capture_admission_sha256=before_hash,current_admission_sha256=base.digest(ADMISSION_PATH),sole_post_capture_admission_change='runtime_sha256',checker_sha256=base.digest(Path(__file__)),candidate_manifest_sha256=admission['candidate_manifest_sha256'],candidate_sha256=manifest['candidate_sha256'],candidate_source_commit=manifest['source_commit'],source_snapshot_sha256=manifest['source_snapshot_sha256'],raw_passes=2,complete_raw_pass_bytes_equal=True,passes=passes,input_sha256=input_hashes,output_sha256=output_hashes,raw_core_controls=controls,known_rng_bytes=64,known_rng_equal=True,complete_rng_inventory=False,owner_trace_sha256=base.TRACE,native_sources_checked=len(config['sources']),script_callbacks_executed=False,native_numerical_predicate_comparison=False,capacity_and_raw_list_storage_reconstructed=False,unknown_bytes_masked=0,original_game_execution_by_checker=False,full_manager_can_start=False,gameplay_action_acceptance=False,complete_state_actions_rng_turns=False,historical_raw_save_load_difference_bytes=89198,diagnostics_path=str(directory))
 with a.output.open('x')as stream:json.dump(report,stream,indent=2);stream.write('\n')
 print(json.dumps(dict(status='equal',raw_passes=2,players_per_pass=43,units_per_pass=1262,full_manager_can_start=False)))

class RejectionTests(unittest.TestCase):
 def test_runtime_pending_and_only_data_change(self):
  before=dict(runtime_sha256=None,profile='fixed');after=copy.deepcopy(before)
  with self.assertRaises(ValueError):check_change(before,after)
  after['runtime_sha256']='a'*64;check_change(before,after)
  after['profile']='changed'
  with self.assertRaises(ValueError):check_change(before,after)
 def test_rng_changed_short_or_overclaimed(self):
  rows=[dict(kind='header',schema='c6-linux-unit-operation-manager-controls-v1',profile_sha256='p',full_manager_called=False,script_callbacks_called=False),dict(kind='summary',status=0,known_rng_equal=True,complete_rng_inventory=False,rng_before='00'*64,sync_rng_after='00'*32,map_rng_after='00'*32)]
  check_rng(rows,'p')
  for field,value in [('rng_before','00'),('map_rng_after','01'*32),('complete_rng_inventory',True),('status',1),('known_rng_equal',1)]:
   changed=copy.deepcopy(rows);changed[1][field]=value
   with self.assertRaises(ValueError):check_rng(changed,'p')
 def test_raw_pass_not_normalized(self):
  with self.assertRaises(ValueError):exact('00','01','raw mismatch')
  with self.assertRaises(ValueError):exact(True,1,'typed mismatch')
 def test_candidate_targets_and_unsupported_are_rejected(self):
  result=dict(schema='c6-linux-unit-operation-manager-reconstruction-v1',native_predicate_comparison=True,full_manager_support=False)
  with self.assertRaises(ValueError):base.compare(result,{})
  with self.assertRaises(ValueError):base.parse('{"kind":"complete","kind":"complete"}')
 def test_truncated_policy_and_nonzero_actions(self):
  with self.assertRaises(ValueError):check_policy([])
  with self.assertRaises(ValueError):check_policy([dict(event='end',queued=1,received=0,unit_executed=0,reports=0)])
 def test_lookup_vector_phase_turnactive_and_roster_rejections(self):
  definition=dict(ordinal=0,collection_index=0,hash=99,numeric_find_index=0,hash_find_index=0,numeric_hash_to_index=0,hash_hash_to_index=0)
  slots=[dict(query=i,mapping_valid=i<43,stored_player=int(i<43),resolved_player=int(i<43))for i in range(65)]
  players=[dict(query=i,actual_id=i,turn_active_raw=1)for i in range(43)]
  units=[dict(owner=0,ordinal=i,id=i,size=1,default_raw=0,values_raw='00')for i in range(1262)]
  phase=dict(slot=0,pointer=1,rowid=1,type_hash=99,phase_order=0,active_segment=1,segment_rowid=1,segment_flags_raw=2)
  native=dict(definitions=dict(entries=[definition]),slots=slots,players=players,units=units,phases=dict(slots=[phase]))
  result=dict(schema='c6-linux-unit-operation-manager-reconstruction-v1',native_predicate_comparison=False,full_manager_support=False,complete_player_slots=65,resolved_players=43,units_reconstructed=1262,operation_definitions=[copy.deepcopy(definition)],player_slots=[dict(query=s['query'],mapping_valid=s['mapping_valid'],stored_player_present=bool(s['stored_player']),resolved_player_present=bool(s['resolved_player']))for s in slots],players=[dict(query=p['query'],actual_id=p['actual_id'],saved_turn_active_raw=p['turn_active_raw'])for p in players],units=[dict(u,status='supported_fresh_read')for u in units],phases=dict(status='supported_unique_order',slots=[dict(slot=0,rowid=1,type_hash=99,phase_order=0,active_segment_present=True,segment_rowid=1,segment_flags_raw=2)]))
  base.compare(result,native)
  mutations=[lambda r:r['operation_definitions'][0].update(hash_find_index=-1),lambda r:r['units'][17].update(default_raw=1),lambda r:r['units'][17].update(values_raw='01'),lambda r:r['units'][17].update(size=0),lambda r:r['units'][17].update(status='unsupported'),lambda r:r['phases'].update(status='unsupported'),lambda r:r['phases']['slots'][0].update(phase_order=1),lambda r:r['phases']['slots'][0].update(segment_flags_raw=0),lambda r:r['players'][9].update(saved_turn_active_raw=0),lambda r:r['players'].pop(),lambda r:r['units'].pop(),lambda r:r['player_slots'].pop()]
  for change in mutations:
   changed=copy.deepcopy(result);change(changed)
   with self.assertRaises(ValueError):base.compare(changed,native)

def main():
 p=argparse.ArgumentParser(description=__doc__);sub=p.add_subparsers(dest='mode',required=True);sub.add_parser('freeze-admission');sub.add_parser('self-test')
 for mode in ['pin-runtime','compare']:
  q=sub.add_parser(mode);q.add_argument('--frozen-admission-commit',required=True)
  if mode=='compare':q.add_argument('--diagnostics',type=Path,required=True);q.add_argument('--output',type=Path,required=True)
 a=p.parse_args()
 if a.mode=='self-test':unittest.main(argv=[sys.argv[0]],exit=True)
 elif a.mode=='freeze-admission':freeze_admission()
 elif a.mode=='pin-runtime':pin_runtime(a)
 else:compare_capture(a)
if __name__=='__main__':main()
