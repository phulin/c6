#!/usr/bin/env python3
"""Audit closed passive current-handler facts and controls; no original calls."""
import argparse
import copy
from collections import Counter
import hashlib
import json
from pathlib import Path
import struct
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
ROOT=Path(__file__).resolve().parents[2]
PROFILE='321c39581fd268b9e75d2efaa2a672e8662a3468573bc1cad8bfbec8e15e7045'
RUNTIME='ce40c14a3eb5882ef04d2524e2231d33b77737a1fc8c542a8ede1cf921184553'
FACTS='77c31dd6dda9736588ee71c06637eb92ec002b0f7acacd780b30739b9ebef942'
CORE='ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884'
SAVES=['save.bin','before.bin','control.bin','after.bin','after-repeat.bin','repeat.bin']
def sha(raw):return hashlib.sha256(raw).hexdigest()
def require(value,reason):
 if not value:raise ValueError(reason)
def unique(pairs):
 result={}
 for k,v in pairs:
  require(k not in result,'duplicate JSON key');result[k]=v
 return result
def load(raw):return json.loads(raw,object_pairs_hook=unique,parse_constant=lambda _:(_ for _ in ()).throw(ValueError('nonfinite JSON')))
def u32(raw,off):return struct.unpack_from('<I',raw,off)[0]
def ptr(raw,off):return struct.unpack_from('<Q',raw,off)[0]
def validate_facts(rows,elf):
 kinds=Counter(r.get('kind') for r in rows)
 require(kinds=={'profile':1,'manager':1,'handler':63,'player_manager':1,'player_slot':65,'unit_container':43,'unit_membership':1262,'unit_queue':1262,'roster_summary':1,'complete':1},'incomplete/unknown raw framing')
 require(rows[0]['kind']=='profile' and rows[-1]['kind']=='complete','missing framing endpoints')
 header=rows[0];base=header['module_base']
 require(header['schema']=='c6-linux-current-handler-interruption-metadata-v1' and header['profile_sha256']==PROFILE and header['gamecore_sha256']==GAME_SHA and header['host_sha256']==HOST_SHA,'profile/input scope')
 require(header['helper_direct_original_calls'] is False and header['predicate_results'] is False and header['caller_quiescence_callback_invoked'] is True,'scope overclaim')
 for key in ['module_loader_path_raw','module_canonical_path_raw']:require(bool(bytes.fromhex(header[key])),'missing path identity')
 methods=[]
 def method(m):
  require(m['owner_base']==base and m['actual_pointer']==base+m['rva'],'method owner/RVA identity')
  for flag in ['vtable_slot_mapping_matches_hashed_file','runtime_mapping_readable_executable','runtime_mapping_matches_hashed_file']:require(m[flag] is True,'method mapping witness absent')
  code=bytes.fromhex(m['live_code_raw']);require(bool(code) and code==elf.read(m['rva'],len(code)),'actual method bytes differ from pinned original')
  methods.append((m['rva'],len(code),sha(code)))
 manager=next(r for r in rows if r['kind']=='manager');manager_raw=bytes.fromhex(manager['manager_raw']);require(len(manager_raw)==104,'Manager header cut')
 require(ptr(manager_raw,0)==base+0x3aff880,'Manager actual class')
 require(len(manager['live_methods'])==4 and [m['slot_offset'] for m in manager['live_methods']]==[0x88,0x98,0xa0,0xd8],'Manager method coverage')
 for m in manager['live_methods']:method(m)
 handlers={}
 for r in [r for r in rows if r['kind']=='handler']:
  require(r['slot']==len(handlers) and r['slot'] not in handlers,'handler slot order/duplicate');handlers[r['slot']]=r
  if r['pointer']:
   h=bytes.fromhex(r['handler_raw']);require(len(h)==16 and ptr(h,0)==r['vptr'] and u32(h,8)==r['runtime_id']==r['slot'] and u32(h,12)==r['operation_hash'],'handler header identity')
   require(r['actual_method']['slot_offset']==0x20,'wrong handler method slot');method(r['actual_method'])
 require(ptr(manager_raw,32)-ptr(manager_raw,24)==8*len(handlers),'handler vector extent')
 slots=[r for r in rows if r['kind']=='player_slot'];require([r['query'] for r in slots]==list(range(65)),'65 slot order')
 resolved={}
 for r in slots:
  require(r['resolved']==(r['stored'] if r['mapping']!=-1 else 0),'source lookup projection')
  if r['resolved']:resolved[r['query']]=r['resolved']
 containers={r['owner']:r for r in rows if r['kind']=='unit_container'};require(len(containers)==43 and set(containers)==set(resolved),'Player/container coverage')
 members={};queues={};counts=Counter();units=set()
 for r in rows:
  if r['kind']=='unit_membership':
   key=(r['owner'],r['id']);require(key not in members and r['unit'] not in units,'duplicate Unit ID/instance');units.add(r['unit']);members[key]=r
   require(r['owner'] in containers and r['ordinal']==counts[r['owner']],'membership ordinal');counts[r['owner']]+=1
   require(ptr(bytes.fromhex(r['entry_raw']),0)==r['unit'],'entry owner pointer')
  elif r['kind']=='unit_queue':
   key=(r['owner'],r['id']);require(key in members and key not in queues and members[key]['unit']==r['unit'],'queue member identity');queues[key]=r
   q=bytes.fromhex(r['queue_raw']);block=bytes.fromhex(r['block_allocation_raw'])
   require(len(q)==200 and u32(q,192)==r['count'] and u32(q,184)==r['capacity'] and ptr(q,176)==r['block'],'queue raw field identity')
   require(r['count']<=r['capacity'] and len(block)==r['capacity']*40,'queue full capacity cut')
   if r['inline']:require(r['block']==r['queue'],'inline block owner')
   if r['count']:
    require(r['native_gethandler_return_observed'] is False and r['numeric_slot_selection_source_derived'] is True,'lookup inference relabeled as observed return')
    front=u32(block,0);require(front==r['native_front_bits'] and front in handlers,'raw native front')
    h=handlers[front];require(h['pointer']==r['observed_numeric_slot_pointer'],'source-derived numeric selection')
    if h['pointer']:require(h['operation_hash']==r['numeric_slot_handler_hash'] and h['vptr']==r['numeric_slot_handler_vptr'] and h['actual_method']['actual_pointer']==r['numeric_slot_actual_target'],'selected handler identity')
 for owner,c in containers.items():
  raw=bytes.fromhex(c['container_raw']);require(len(raw)==72 and c['parent']==resolved[owner] and u32(raw,32)==counts[owner],'container count/parent')
 require(set(members)==set(queues),'missing Unit queue')
 summary=next(r for r in rows if r['kind']=='roster_summary');require(summary=={'kind':'roster_summary','player_slots':65,'players':43,'units':1262},'roster summary')
 require(rows[-1]['manager']==manager['pointer'] and rows[-1]['handler_slots']==63 and rows[-1]['player_slots']==65,'completion mismatch')
 return dict(row_kinds=dict(kinds),players=43,units=1262,handler_slots=63,nonempty_queues=sum(bool(q['count']) for q in queues.values()),empty_queues=sum(not q['count'] for q in queues.values()),actual_method_instances=len(methods),unique_actual_method_bodies=len(set(methods)),native_gethandler_returns_observed=0,native_caninterrupt_results_observed=0,native_full_manager_results_observed=0)
def mutation_checks(rows,elf):
 tests=[]
 def mutate(kind,key,value,reason):
  changed=copy.deepcopy(rows);row=next(r for r in changed if r['kind']==kind and (kind!='unit_queue' or r['count']));row[key]=value;tests.append((changed,reason))
 mutate('profile','helper_direct_original_calls',True,'scope overclaim')
 mutate('handler','runtime_id',999,'handler header identity')
 mutate('unit_membership','ordinal',999,'membership ordinal')
 mutate('unit_queue','native_gethandler_return_observed',True,'lookup inference relabeled')
 q=next(r for r in rows if r['kind']=='unit_queue' and r['count'])
 mutate('unit_queue','native_front_bits',q['native_front_bits']+1,'raw native front')
 mutate('unit_queue','observed_numeric_slot_pointer',q['observed_numeric_slot_pointer']+8,'source-derived numeric selection')
 mutate('unit_queue','block_allocation_raw',q['block_allocation_raw'][:-2],'queue full capacity cut')
 mutate('unit_queue','queue_raw',q['queue_raw'][:-2],'queue raw field identity')
 mutate('complete','handler_slots',64,'completion mismatch')
 for field,value,reason in [('actual_pointer',0,'method owner/RVA identity'),('live_code_raw','00','actual method bytes'),('runtime_mapping_matches_hashed_file',False,'method mapping witness absent')]:
  changed=copy.deepcopy(rows);next(r for r in changed if r['kind']=='manager')['live_methods'][0][field]=value;tests.append((changed,reason))
 changed=copy.deepcopy(rows);changed.pop();tests.append((changed,'incomplete/unknown raw framing'))
 for changed,reason in tests:
  try:validate_facts(changed,elf)
  except ValueError as error:require(reason in str(error),'wrong mutation refusal: '+str(error))
  else:raise ValueError('malformed raw observation accepted')
 return len(tests)
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--capture',type=Path,default=ROOT.parent/'c6-ghidra/evidence/linux-current-handler-metadata/native-3');p.add_argument('--output',type=Path,required=True);args=p.parse_args();capture=args.capture
 runtime_raw=(capture/'runtime.json').read_bytes();require(sha(runtime_raw)==RUNTIME,'runtime identity');runtime=load(runtime_raw);config=runtime['configuration']
 require(runtime['returncode']==0 and runtime['fixture_success'] is True and runtime['framework_cleanup_proven'] is True and runtime['client_cleanup_proven'] is True,'actual terminal cleanup unavailable')
 require(runtime['configuration_sha256']==PROFILE and sha(json.dumps(config,sort_keys=True,separators=(',',':')).encode())==PROFILE,'configuration identity')
 require(config==load((capture/'configuration.json').read_bytes()) and config==load((capture/'prepared.json').read_bytes())['configuration'],'prepared/runtime config mismatch')
 require('uninitialize_return initialized 0 context_bound 0 executing 0 cleanup 1' in runtime['stderr'],'native cleanup footer')
 require(sha((capture/'current-handler-metadata-preload.so').read_bytes())==config['preload_sha256'],'preload identity')
 for name,digest in {**config['sources'],'tools/linux/mod_activation_probe.py':config['runner_sha256']}.items():require(sha((capture/'source'/name).read_bytes())==digest,'archived source mismatch '+name)
 raw=(capture/'facts.jsonl').read_bytes();require(sha(raw)==FACTS and raw==(capture/'facts-repeat.jsonl').read_bytes(),'raw repeated facts differ')
 rows=[load(line) for line in raw.splitlines()];elf=Elf(ROOT.parent/'c6-assets-linux/libGameCore_XP2.so',GAME_SHA);coverage=validate_facts(rows,elf);refusals=mutation_checks(rows,elf)
 controls=[load(line) for line in (capture/'controls.jsonl').read_bytes().splitlines()];require(len(controls)==2 and controls[0]['profile_sha256']==PROFILE and controls[0]['full_manager_called'] is False,'control framing')
 end=controls[-1];require(end['status']==0 and end['known_rng_equal'] is True and end['complete_rng_inventory'] is False,'known RNG scope')
 require(bytes.fromhex(end['before'])==bytes.fromhex(end['after']) and len(bytes.fromhex(end['before']))==64,'known RNG endpoint bytes')
 for name in SAVES:raw_core=(capture/name).read_bytes();require(len(raw_core)==80657462 and sha(raw_core)==CORE,'actual unmasked Save mismatch '+name)
 report=dict(schema='c6-linux-current-handler-capture-audit-v1',status='bounded_raw_observations_equal',profile_sha256=PROFILE,runtime_sha256=RUNTIME,facts_sha256=FACTS,facts_bytes=len(raw),raw_passes=2,coverage=coverage,whole_unmasked_save_files=6,whole_core_sha256=CORE,whole_core_bytes=80657462,unknown_bytes_masked=0,known_rng_bytes=64,known_rng_endpoints_equal=True,complete_rng_inventory=False,source_files_checked=len(config['sources'])+1,actual_native_cleanup_footer=True,framework_and_client_cleanup=True,helper_or_guard_original_calls=False,Save_continuous_lock_claimed=False,full_state_actions_turns_accepted=False,original_execution_by_audit=False,checker_sha256=sha(Path(__file__).read_bytes()),mutation_refusals=refusals,retained_raw_method_records=67,source_executed_but_unretained_witnesses=['26B anchor equality and mapped file identity','native lock owner/depth field checks','main-thread/controller/work/queue field checks'],terminal_process_scope='native subprocess normal return and observed no Civ6/GDB executable after closure; exact host starttime not retained',scope='actual raw slots/code/fronts; source-derived numeric lookup, no observed GetHandler/CanInterrupt/Manager return')
 args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')
if __name__=='__main__':main()
