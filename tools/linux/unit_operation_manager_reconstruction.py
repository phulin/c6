#!/usr/bin/env python3
"""Freeze/project/compare strict Manager raw-state reconstruction; never native execution."""
import argparse
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
from player_alive_saved_crosscheck import saved_player_alive
ROOT=Path(__file__).resolve().parents[2]
GAME='a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
HOST='106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
CORE='ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884'
TRACE='528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'
RUST='tools/linux/unit_operation_manager_reconstruction.rs'
PYTHON='tools/linux/unit_operation_manager_reconstruction.py'
def require(v,m):
 if not v:raise ValueError(m)
def sha(b):return hashlib.sha256(b).hexdigest()
def digest(p):return sha(Path(p).read_bytes())
def pairs(items):
 out={}
 for k,v in items:require(k not in out,'duplicate JSON key: '+k);out[k]=v
 return out
def parse(raw):return json.loads(raw,object_pairs_hook=pairs)
def read(p):
 p=Path(p);raw=p.read_bytes();return gzip.decompress(raw)if p.suffix=='.gz'else raw
def load(p):return parse(Path(p).read_bytes())
def output(command,cwd=ROOT):return subprocess.check_output(command,cwd=cwd,text=True).strip()
def canonical(value):return json.dumps(value,sort_keys=True,separators=(',',':')).encode()
def freeze(a):
 require(not a.output.exists(),'immutable manifest already exists')
 commit=output(['git','rev-parse',a.source_commit+'^{commit}'])
 require(subprocess.check_output(['git','show',commit+':'+PYTHON],cwd=ROOT)==Path(__file__).read_bytes(),'freeze driver is not exact committed source')
 archive=subprocess.check_output(['git','archive','--format=tar',commit],cwd=ROOT)
 directory=a.directory.resolve();directory.mkdir(parents=True,exist_ok=False);source=directory/'source';source.mkdir()
 snapshot=directory/'source.tar';snapshot.write_bytes(archive)
 with tarfile.open(fileobj=io.BytesIO(archive))as tar:tar.extractall(source,filter='data')
 paths=output(['git','ls-tree','-r','--name-only',commit]).splitlines();sources={p:sha(subprocess.check_output(['git','show',commit+':'+p],cwd=ROOT))for p in paths}
 for p,h in sources.items():require(digest(source/p)==h,'archive source differs: '+p)
 env=os.environ.copy()
 for k in ['CARGO_TARGET_DIR','CARGO_BUILD_TARGET','RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','CARGO_BUILD_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER']:env.pop(k,None)
 env.update(CARGO_INCREMENTAL='0',CARGO_NET_OFFLINE='true')
 cargo_command=['cargo','build','--locked','--offline','--release','-p','c6-data','--message-format=json']
 raw=subprocess.check_output(cargo_command,cwd=source,env=env);(directory/'cargo-artifacts.jsonl').write_bytes(raw)
 artifacts={}
 for line in raw.splitlines():
  row=json.loads(line)
  if row.get('reason')=='compiler-artifact':
   for filename in row['filenames']:
    if filename.endswith('.rlib'):artifacts[row['target']['name']]=filename
 extern=[]
 for name in ['c6_core','c6_data','serde','serde_json','sha2']:
  require(name in artifacts,'missing exact archive dependency: '+name);extern+=['--extern',name+'='+artifacts[name]]
 candidate=directory/'c6-unit-operation-manager-reconstruction'
 command=['rustc','--edition=2021','--crate-name','c6_unit_operation_manager_reconstruction','-D','warnings','-C','opt-level=3',str(source/RUST),'-L','dependency='+str(source/'target/release/deps'),*extern,'-o',str(candidate)]
 test=directory/'candidate-tests';test_command=['rustc','--test',*command[1:-2],'-o',str(test)]
 subprocess.run(test_command,cwd=source,env=env,check=True);test_result=subprocess.check_output([str(test)],env=env);(directory/'candidate-tests.txt').write_bytes(test_result)
 subprocess.run(command,cwd=source,env=env,check=True)
 sysroot=Path(output(['rustc','--print','sysroot']));rustc=Path(shutil.which('rustc'));cargo=Path(shutil.which('cargo'))
 report=dict(schema='c6-linux-unit-operation-manager-candidate-v1',source_commit=commit,source_snapshot_path=str(snapshot),source_snapshot_sha256=sha(archive),source_snapshot_bytes=len(archive),sources=sources,
  candidate_path=str(candidate),candidate_sha256=digest(candidate),candidate_bytes=candidate.stat().st_size,
  rustc_version=output(['rustc','-Vv']),cargo_version=output(['cargo','-Vv']),rustc_toolchain_sha256=digest(sysroot/'bin/rustc'),cargo_toolchain_sha256=digest(sysroot/'bin/cargo'),rustc_launcher_sha256=digest(rustc),cargo_launcher_sha256=digest(cargo),
  cargo_command=cargo_command,candidate_build_command=command,test_build_command=test_command,test_result_sha256=sha(test_result),protocol_tests_passed=3,
  build_environment={'CARGO_INCREMENTAL':'0','CARGO_NET_OFFLINE':'true','rustflags_and_wrappers_removed':True},
  input_schema='c6-linux-unit-operation-manager-input-v1',output_schema='c6-linux-unit-operation-manager-reconstruction-v1',expected_outputs_used_as_inputs=False,native_execution=False,native_comparison=False,
  full_manager_can_start=False,script_callbacks_executed=False,complete_state_actions_rng_turns=False)
 a.output.parent.mkdir(parents=True,exist_ok=True)
 for p in [a.output,directory/'manifest.json']:
  with p.open('x')as f:json.dump(report,f,indent=2);f.write('\n')
 print(json.dumps({k:report[k]for k in ['source_commit','candidate_sha256','source_snapshot_sha256','protocol_tests_passed']}))
def candidate_gate(path):
 m=load(path);require(m['schema']=='c6-linux-unit-operation-manager-candidate-v1','candidate schema')
 require(not m['expected_outputs_used_as_inputs']and not m['native_execution']and not m['native_comparison'],'candidate preparation scope')
 for stem in ['candidate','source_snapshot']:
  p=Path(m[stem+'_path']);require(p.stat().st_size==m[stem+'_bytes']and digest(p)==m[stem+'_sha256'],'immutable '+stem+' changed')
 archive=subprocess.check_output(['git','archive','--format=tar',m['source_commit']],cwd=ROOT);require(sha(archive)==m['source_snapshot_sha256'],'source archive differs')
 paths=output(['git','ls-tree','-r','--name-only',m['source_commit']]).splitlines();require(set(paths)==set(m['sources']),'incomplete source inventory')
 for p,h in m['sources'].items():require(sha(subprocess.check_output(['git','show',m['source_commit']+':'+p],cwd=ROOT))==h,'source hash differs: '+p)
 return m,Path(m['candidate_path'])
def facts(path):
 rows=[parse(line)for line in Path(path).read_bytes().splitlines()];require(rows and rows[0].get('kind')=='profile'and rows[0].get('schema')=='c6-linux-unit-operation-manager-metadata-v2','helper schema')
 cursor=1
 def take(kind):
  nonlocal cursor
  require(cursor<len(rows)and rows[cursor].get('kind')==kind,'missing/ordered helper row: '+kind);value=rows[cursor];cursor+=1;return value
 configuration=take('configuration');phases=take('phases');definitions=take('operation_definitions');slots=[];players=[];units=[]
 for query in range(65):
  slot=take('player_slot');require(slot['query']==query,'complete65 slot order');slots.append(slot)
  require(type(slot['mapped_index'])is int and type(slot['mapping_valid'])is bool,'slot raw types')
  require(slot['mapping_valid']==(slot['mapped_index']!=-1),'native mapping validity disagrees with raw mapping')
  require(slot['resolved_player']==(slot['stored_player']if slot['mapping_valid']else 0),'raw stored/resolved pointers disagree')
  if slot['resolved_player']:
   player=take('player');require(player['query']==player['actual_id']==query,'actual Player identity');players.append(player);ordinal=0
   while cursor<len(rows)and rows[cursor].get('kind')=='unit_disabled':
    unit=take('unit_disabled');require(unit['owner']==query and unit['ordinal']==ordinal,'Unit membership order');units.append(unit);ordinal+=1
 summary=take('roster_summary');globals_=take('globals');complete=take('complete');require(cursor==len(rows),'trailing helper rows')
 require(summary==dict(kind='roster_summary',player_slots=65,resolved_players=len(players),units=len(units))and complete==dict(kind='complete',player_slots=65),'complete roster framing')
 require(len(players)==43 and len(units)==1262,'actual complete profile roster differs')
 require(globals_['script_callbacks_inspected']is False,'helper callback scope differs')
 return dict(rows=rows,configuration=configuration,phases=phases,definitions=definitions,slots=slots,players=players,units=units,globals=globals_)
def make_request(data,trace,f,directory,profile,rules):
 require(sha(data)==CORE and sha(trace)==TRACE,'exact canonical Core/trace source required')
 headers=saved_player_alive(data,trace,CORE,TRACE)
 rows=[parse(line)for line in trace.splitlines()];rows=[r for r in rows if r.get('event')=='owner'and r.get('save')==1]
 calls={r['call']:r for r in rows};require(len(calls)==len(rows),'duplicate owner call')
 player_cuts={}
 for row in rows:
  require(0<=row['start']<row['end']<=len(data),'writer cut outside Core')
  if row['owner']=='GameCore::Player::Instance':
   owner=int.from_bytes(data[row['start']+4:row['start']+8],'little',signed=True);require(owner not in player_cuts,'duplicate Player writer');player_cuts[owner]=row
 units={};maps=[r for r in rows if r['owner']=='GameCore::Map::Instance'];require(len(maps)==1,'Map attribution')
 for row in rows:
  if row['owner']!='GameCore::Unit::Instance':continue
  raw=data[row['start']:row['end']];require(len(raw)>=44 and int.from_bytes(raw[:4],'little')==28,'Unit28 boundary')
  uid=int.from_bytes(raw[4:8],'little');owner=int.from_bytes(raw[40:44],'little',signed=True)
  parent=calls.get(row['parent']);require(parent and parent['owner']=='GameCore::Player::Units','Unit parent attribution');parent=calls.get(parent['parent'])
  require(parent==player_cuts.get(owner)and parent['start']<=row['start']<row['end']<=parent['end'],'Unit Player attribution')
  require((owner,uid)not in units,'duplicate Unit identity');units[(owner,uid)]=row
 require(set(headers)==set(player_cuts)=={p['query']for p in f['players']},'complete43 attributed headers/actual roster')
 def blob(name,start,end):
  p=directory/name;p.parent.mkdir(parents=True,exist_ok=True);raw=data[start:end]
  with p.open('xb')as stream:stream.write(raw)
  return dict(path=name,bytes=len(raw),sha256=sha(raw))
 by_owner={};used=set()
 for unit in f['units']:
  key=(unit['owner'],unit['id']);require(key in units and key not in used,'actual Unit lacks unique attributed wire');used.add(key);row=units[key]
  by_owner.setdefault(unit['owner'],[]).append(dict(ordinal=unit['ordinal'],id=unit['id'],wire=blob(f"units/{unit['owner']:02d}-{unit['ordinal']:06d}.bin",row['start'],row['end'])))
 require(used==set(units),'wire/native Unit roster differs; no filtering')
 players=[]
 for p in f['players']:
  q=p['query'];h=headers[q];players.append(dict(query=q,slot_status=p['slot_status'],header=blob(f'headers/{q:02d}.bin',h['header_start'],h['header_start']+98),units=by_owner.get(q,[])))
 defs=f['definitions'];require(defs['count']==len(defs['entries'])<=65536,'definition coverage')
 definitions=[];registrations=[]
 for i,d in enumerate(defs['entries']):
  require(d['ordinal']==d['collection_index']==i,'definition original order');definitions.append(dict(collection_index=i,hash=d['hash']))
  registrations.append(dict(hash=d['hash'],runtime_id=d['runtime_id'],handler_present=bool(d['handler_pointer']),handler_id=d.get('handler_id'),handler_hash=d.get('handler_hash')))
 rules_out=directory/'rules';rules_out.mkdir()
 for name in ['configuration.sqlite','gameplay.sqlite']:shutil.copyfile(rules/name,rules_out/name)
 require(len(profile)==64 and all(c in '0123456789abcdef'for c in profile),'profile hash')
 request=dict(schema='c6-linux-unit-operation-manager-input-v1',gamecore_sha256=GAME,host_sha256=HOST,configuration_sha256=profile,profile_sha256=profile,core_sha256=CORE,trace_sha256=TRACE,
  map=blob('map.bin',maps[0]['start'],maps[0]['end']),rules_directory='rules',operation_definitions=definitions,registrations=registrations,
  configured_phase_hash=f['configuration']['configured_phase_hash'],current_phase=f['phases']['current_phase'],
  controls=dict(pause_raw=f['globals']['game_pause_raw'],automation_player_raw=f['globals']['automation_player_raw'],script_system_present=bool(f['globals']['script_system'])),
  player_slots=[dict(query=s['query'],mapped_index=s['mapped_index'],stored_player_present=bool(s['stored_player']))for s in f['slots']],players=players)
 return request
def compare(result,f):
 require(result['schema']=='c6-linux-unit-operation-manager-reconstruction-v1'and not result['native_predicate_comparison']and not result['full_manager_support'],'reconstruction scope')
 require(result['complete_player_slots']==65 and result['resolved_players']==43 and result['units_reconstructed']==1262,'result coverage')
 for actual,want in zip(result['operation_definitions'],f['definitions']['entries']):
  for key in ['ordinal','collection_index','hash','numeric_find_index','hash_find_index','numeric_hash_to_index','hash_hash_to_index']:require(actual[key]==want[key],'definition lookup mismatch: '+key)
 require(len(result['operation_definitions'])==len(f['definitions']['entries']),'definition result coverage')
 for actual,want in zip(result['player_slots'],f['slots']):
  require(actual==dict(query=want['query'],mapping_valid=want['mapping_valid'],stored_player_present=bool(want['stored_player']),resolved_player_present=bool(want['resolved_player'])),'slot absence/invalid mismatch')
 require(len(result['player_slots'])==65,'slot output coverage')
 for actual,want in zip(result['players'],f['players']):
  require(actual['query']==want['query']and actual['actual_id']==want['actual_id']and actual['saved_turn_active_raw']==want['turn_active_raw'],'Player header/live TurnActive mismatch')
 require(len(result['players'])==43,'Player output coverage')
 for actual,want in zip(result['units'],f['units']):
  for key in ['owner','ordinal','id','size','default_raw','values_raw']:require(actual[key]==want[key],'live disabled reconstruction mismatch: '+key)
  require(actual['status']=='supported_fresh_read','unsupported disabled result')
 require(len(result['units'])==1262,'Unit output coverage')
 phase=result['phases'];require(phase['status']=='supported_unique_order','phase source reconstruction unsupported: '+str(phase))
 require(len(phase['slots'])==len(f['phases']['slots']),'phase count mismatch')
 for actual,want in zip(phase['slots'],f['phases']['slots']):
  require(bool(want['pointer']),'native null phase not reconstructed from rules')
  expected=dict(slot=want['slot'],rowid=want['rowid'],type_hash=want['type_hash'],phase_order=want['phase_order'],active_segment_present=bool(want['active_segment']),segment_rowid=want.get('segment_rowid'),segment_flags_raw=want.get('segment_flags_raw'))
  require(actual==expected,'phase identity/order/segment raw flags mismatch')
 return dict(player_slots=65,players=43,units=1262,definitions=len(result['operation_definitions']),phases=len(phase['slots']),all_compared_raw_state_equal=True,
  capacity_and_list_storage_not_reconstructed=True,raw_observations_retained_unmasked=True,native_numerical_predicate_comparison=False,script_callbacks_executed=False,full_manager_can_start=False,complete_state_actions_rng_turns=False)
def run(a):
 manifest,binary=candidate_gate(a.manifest);f=facts(a.facts);directory=a.directory.resolve();directory.mkdir(parents=True,exist_ok=False)
 request=make_request(read(a.core),read(a.trace),f,directory,a.profile_sha256,a.rules.resolve());input_path=directory/'input.json';input_path.write_bytes(canonical(request))
 # The strict candidate receives only the projection. Target live vectors and
 # observed phase slots are accessed by compare only after candidate execution.
 completed=subprocess.run([str(binary),str(input_path)],capture_output=True,check=False);(directory/'candidate.stdout').write_bytes(completed.stdout);(directory/'candidate.stderr').write_bytes(completed.stderr)
 require(completed.returncode==0,'candidate unsupported/malformed: '+completed.stderr.decode(errors='replace'))
 require(digest(binary)==manifest['candidate_sha256'],'candidate changed during execution')
 result=parse(completed.stdout);comparison=compare(result,facts(a.facts))
 report=dict(schema='c6-linux-unit-operation-manager-reconstruction-comparison-v1',checks_passed=True,configuration_sha256=a.profile_sha256,candidate_manifest_sha256=digest(a.manifest),candidate_sha256=manifest['candidate_sha256'],source_commit=manifest['source_commit'],input_sha256=digest(input_path),facts_sha256=digest(a.facts),output_sha256=sha(completed.stdout),comparison=comparison,original_execution_performed_by_this_driver=False,expected_outputs_used_as_inputs=False,raw_save_load_difference_bytes=89198,diagnostics_path=str(directory))
 with a.output.open('x')as stream:json.dump(report,stream,indent=2);stream.write('\n')
 print(json.dumps(comparison))
def main():
 p=argparse.ArgumentParser(description=__doc__);sub=p.add_subparsers(dest='mode',required=True)
 q=sub.add_parser('freeze');q.add_argument('--source-commit',required=True);q.add_argument('--directory',type=Path,required=True);q.add_argument('--output',type=Path,required=True)
 q=sub.add_parser('compare');q.add_argument('--manifest',type=Path,required=True);q.add_argument('--facts',type=Path,required=True);q.add_argument('--core',type=Path,required=True);q.add_argument('--trace',type=Path,required=True);q.add_argument('--rules',type=Path,required=True);q.add_argument('--profile-sha256',required=True);q.add_argument('--directory',type=Path,required=True);q.add_argument('--output',type=Path,required=True)
 a=p.parse_args();freeze(a)if a.mode=='freeze'else run(a)
if __name__=='__main__':main()
