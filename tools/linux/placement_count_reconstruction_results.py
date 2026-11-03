#!/usr/bin/env python3
"""Closed-capture Linux placement comparison; no original code execution.

Facts and attributed raw state alone enter the frozen Rust candidate. Native
expected files are opened only after candidate execution, for comparison.
"""
import argparse
import collections
import gzip
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile
from core_stream_index import Reader
from player_alive_saved_crosscheck import source_gate,saved_player_alive,compare_actual_alive
ROOT=Path(__file__).resolve().parents[2]
GAME='a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
HOST='106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
CORE='ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884'
TRACE='528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'
PROFILE='9022128238f6209173eec2532b47405721780f1e2a105808ee9084306ebc2f33'
MANIFEST='e2925853cea2eaee831e1c2ba57421957a02d659e9822b750c28c2ffee1cc6c4'
CANDIDATE='58171fb1398b148cfc1e94fc882b79723a9571d193cd6d9c375c3c51fb68f59a'
ARCHIVE='560bb94b98da8a187118e6cbc82f60d279e6d008c12884bd07a1b878c1b6b559'
COMMIT='301f9e9d188f6a5ac815914218d0380f51b25938'
RUNTIME='e70d0b5f9fdd62d720f6c6929165da7a4f2e290c382b89bf595171b1f935e62a'
FACTS='b4922e0312310664d7ba59034d32b3118ab3ea78cdad45321926fa73e9cefc23'
EXPECTED='bc65088165f1b57473738f6ec5f784c86b838060d3c8f2c87a8bebcd5240c882'
def sha(data):return hashlib.sha256(data).hexdigest()
def digest(path):return sha(path.read_bytes())
def require(value,reason):
 if not value:raise ValueError(reason)
def pairs(items):
 result={}
 for k,v in items:require(k not in result,'duplicate JSON key: '+k);result[k]=v
 return result
def parse(text):return json.loads(text,object_pairs_hook=pairs)
def load(path):return parse(path.read_text())
def canonical(value):return json.dumps(value,sort_keys=True,separators=(',',':')).encode()
def file_identity(path):return dict(filename=path.name,bytes=path.stat().st_size,sha256=digest(path))
def candidate_gate(path):
 m=load(path)
 require(digest(path)==MANIFEST and m['source_commit']==COMMIT and m['candidate_sha256']==CANDIDATE and m['source_snapshot_sha256']==ARCHIVE,'unreviewed candidate/source/manifest identity')
 require(m['schema']=='c6-linux-placement-count-candidate-v1' and m['expected_outputs_used_as_inputs'] is False and m['native_execution'] is False and m['native_comparison'] is False,'candidate preparation boundary differs')
 binary=Path(m['candidate_path']);archive=Path(m['source_snapshot_path'])
 require(binary.stat().st_size==m['candidate_bytes'] and digest(binary)==m['candidate_sha256'],'immutable candidate changed')
 require(archive.stat().st_size==m['source_snapshot_bytes'] and digest(archive)==m['source_snapshot_sha256'],'source archive changed')
 require(sha(subprocess.check_output(['git','archive','--format=tar',m['source_commit']],cwd=ROOT))==m['source_snapshot_sha256'],'archive not exact complete committed source')
 for p,h in m['sources'].items():require(sha(subprocess.check_output(['git','show',m['source_commit']+':'+p],cwd=ROOT))==h,'committed source differs: '+p)
 return m,binary

def closed_controls(capture,runtime):
 roles=[('before','before.bin'),('control','control.bin'),('after','after.bin'),('repeat','after-repeat.bin')]
 controls=[];data=None
 for role,name in roles:
  p=capture/name;row=file_identity(p);require(row==runtime['placement_metadata_saves'][role],'internal Core identity differs: '+role)
  require(row['sha256']==CORE and row['bytes']==80657462,'raw Core differs from exact owner-trace source; no spans guessed')
  controls.append(dict(role='internal_'+role,**row,raw_differences_from_before=0))
  if role=='before':data=p.read_bytes()
 for role,name in [('save','save.bin'),('repeat','repeat.bin')]:
  row=file_identity(capture/name);require(row==runtime['game_saves'][role],'outer Core identity differs')
  require(row['sha256']==CORE and row['bytes']==len(data),'outer Save differs from accepted owner-trace source')
  controls.append(dict(role='outer_'+role,**row,raw_differences_from_before=0))
 return data,controls

def make_request(data,facts,trace,directory,profile):
 """No expected count/binding/handler object is accepted by this projection."""
 require(sha(data)==CORE and sha(trace)==TRACE,'exact owner-attributed source unavailable')
 rows=[parse(x) for x in trace.splitlines()];rows=[r for r in rows if r.get('event')=='owner' and r.get('save')==1]
 calls={r['call']:r for r in rows};require(len(calls)==len(rows),'duplicate owner-call identity')
 players={};units={};formations={};maps=[r for r in rows if r['owner']=='GameCore::Map::Instance']
 require(len(maps)==1,'Map writer attribution differs')
 # Header and Formation prefix are structurally parseable for all Players, even
 # while the intervening full Player payload import remains unfinished.
 formation_wire_slots={}
 for r in rows:
  require(0<=r['start']<r['end']<=len(data),'writer cut outside Core')
  if r['owner']=='GameCore::Player::Instance':
   reader=Reader(data[r['start']:r['end']]);owner=reader.first_player_prefix()
   require(owner not in players,'duplicate Player owner cut');players[owner]=r
   wire=reader.first_formations()
   for slot,record in wire['entries']:
    start=r['start']+record['start'];end=r['start']+record['end']
    fid=struct.unpack_from('<I',data,start+4)[0]
    require((owner,fid) not in formation_wire_slots,'duplicate Formation wire identity')
    formation_wire_slots[(owner,fid)]=(slot,start,end)
 for r in rows:
  if r['owner'] not in ('GameCore::Unit::Instance','GameCore::Formation::Instance'):continue
  kind='unit' if r['owner']=='GameCore::Unit::Instance' else 'formation'
  raw=data[r['start']:r['end']];version=struct.unpack_from('<I',raw)[0]
  require(version==(28 if kind=='unit' else 1),'current actor wire version differs')
  uid=struct.unpack_from('<I',raw,4)[0];owner=struct.unpack_from('<i',raw,40 if kind=='unit' else 16)[0]
  parent=calls.get(r['parent']);require(parent is not None,'actor parent writer unavailable')
  if kind=='unit':
   require(parent['owner']=='GameCore::Player::Units','Unit not owned by Units writer')
   parent=calls.get(parent['parent']);require(parent is not None,'Units owner writer unavailable')
  require(parent['owner']=='GameCore::Player::Instance' and players.get(owner)==parent,'actor/Player owner attribution differs')
  require(parent['start']<=r['start']<r['end']<=parent['end'],'actor not within original Player writer')
  target=units if kind=='unit' else formations
  require((owner,uid) not in target,'duplicate actor identity');target[(owner,uid)]=r
  if kind=='formation':require(formation_wire_slots.get((owner,uid))==(uid&65535,r['start'],r['end']),'Formation native slot/prefix/owner trace disagree')
 def blob(name,row):
  p=directory/name;p.parent.mkdir(parents=True,exist_ok=True);raw=data[row['start']:row['end']]
  with p.open('xb') as f:f.write(raw)
  return dict(path=name,bytes=len(raw),sha256=sha(raw))
 header=facts[0];p_rows=[r for r in facts if r['kind']=='player']
 require(len(p_rows)==65 and [p['query'] for p in p_rows]==list(range(65)),'65 actual player query rows missing/order differs')
 memberships=[r for r in facts if r['kind']=='members'];member_map={}
 for m in memberships:
  key=(m['owner'],m['member_kind']);require(key not in member_map and key[1] in ('unit','formation'),'duplicate/unknown membership')
  require(m['complete'] is True and m['count']==len(m['entries']),'incomplete native membership')
  member_map[key]=m
 out_players=[];used={'unit':set(),'formation':set()}
 for p in p_rows:
  q=p['query'];require(type(p['mapped_index']) is int and type(p['stored_nonnull']) is bool and type(p['valid']) is bool and type(p['resolved']) is bool,'player raw fact type differs')
  require(p['valid']==(p['mapped_index']!=-1) and p['resolved']==(p['valid'] and p['stored_nonnull']),'native player validity/pointer facts inconsistent')
  if not p['resolved']:continue
  require(p['actual_owner']==q and type(p['alive']) is bool,'actual Player identity/state differs')
  out=dict(owner=q,slot_status=p['slot_status'],team=p['team'],alive=p['alive'])
  for kind,name,index in [('unit','units',units),('formation','formations',formations)]:
   require((q,kind) in member_map,'actual complete actor collection unavailable')
   m=member_map[(q,kind)];entries=[]
   for ordinal,e in enumerate(m['entries']):
    require(e['ordinal']==ordinal and e['actual_owner']==q and e['slot']==e['id']&65535,'native member order/identity/slot differs')
    key=(q,e['id']);require(key in index and key not in used[kind],'member lacks unique original attributed wire');used[kind].add(key)
    entries.append(dict(ordinal=ordinal,id=e['id'],slot=e['slot'],actual_owner=q,wire=blob(f'{name}/{q:02d}-{ordinal:06d}.bin',index[key])))
   out[name]=dict(count=m['count'],complete=True,entries=entries)
  out_players.append(out)
 require(used['unit']==set(units) and used['formation']==set(formations),'native/wire actor roster differs; no actor exclusion')
 require(len(memberships)==2*len(out_players),'extra/missing native membership rows')
 registries=[r for r in facts if r['kind']=='unit_definitions'];relations=[r for r in facts if r['kind']=='relations']
 require(len(registries)==len(relations)==1,'complete native definition/relation facts missing')
 defs=registries[0];require(defs['complete'] is True and defs['count']==len(defs['entries']),'definition collection incomplete')
 fields=['ordinal','index','hash','domain_hash','formation_class_hash','make_trade_route','spy','combat','range']
 entries=[{k:d[k] for k in fields} for d in defs['entries']]
 # Validate captured Find observations independently, never pass them as inputs.
 by_hash={}
 for i,d in enumerate(entries):
  require(d['ordinal']==d['index']==i,'native definition collection index differs');by_hash[d['hash']]=i
 for i,d in enumerate(defs['entries']):
  expected=d['hash'] if d['hash']<defs['count'] else by_hash[d['hash']]
  require(d['find_ordinal_index']==i and d['find_hash_index']==expected,'native registry direct/hash/collision contract differs')
 rel=relations[0];require(rel['complete'] is True,'native relation state incomplete')
 request=dict(schema='c6-linux-placement-inputs-v1',gamecore_sha256=GAME,host_sha256=HOST,configuration_sha256=profile,profile_sha256=profile,
  map=blob('map.bin',maps[0]),player_mapping=[p['mapped_index'] for p in p_rows],player_slot_present=[p['stored_nonnull'] for p in p_rows],
  players=out_players,unit_definitions=dict(count=defs['count'],complete=True,entries=entries),relations={k:rel[k] for k in ['player_count','war_types','complete']},plot_unit_limit=header['plot_unit_limit'])
 coverage=dict(players=len(out_players),units=len(units),formations=len(formations),memberships=len(memberships),unit_definitions=defs['count'],war_types=len(rel['war_types']),formation_allocator_prefixes_independently_parsed=len(players),actor_wire_and_native_membership_sets_equal=True)
 return request,coverage

def compare_rows(actual,expected,result_kind,expected_kind):
 require(len(actual)==len(expected),'native/result row count differs')
 mismatches=[]
 for i,(left,right) in enumerate(zip(actual,expected)):
  require(left.get('kind')==result_kind and right.get('kind')==expected_kind,'row kind differs')
  target=dict(right,kind=result_kind)
  if left!=target:mismatches.append(dict(index=i,calculated=left,native_expected=right))
 return mismatches

def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--capture',type=Path,default=ROOT.parent/'c6-ghidra/evidence/linux-placement-metadata/native-1')
 p.add_argument('--manifest',type=Path,default=ROOT/'evidence/linux/placement-count-candidate-301f9e9.json')
 p.add_argument('--profile-sha256',default=PROFILE)
 p.add_argument('--diagnostics',type=Path)
 p.add_argument('--output',type=Path,required=True)
 a=p.parse_args();capture=a.capture.resolve();manifest,binary=candidate_gate(a.manifest)
 runtime_path=capture/'runtime.json';prepared_path=capture/'prepared.json';runtime=load(runtime_path);prepared=load(prepared_path)
 require(digest(runtime_path)==RUNTIME and a.profile_sha256==PROFILE,'unreviewed native capture/profile')
 config=runtime['configuration'];profile=a.profile_sha256
 require(sha(canonical(config))==runtime['configuration_sha256']==prepared['configuration_sha256']==profile and config==prepared['configuration'],'closed runtime/preparation/configuration differs')
 require(config['assets']['libGameCore_XP2.so']==GAME and config['assets']['Civ6']==HOST,'original module/host differs')
 binding=config['lower_core_placement_metadata']
 require(binding['reconstruction_manifest']==manifest and binding['reconstruction_manifest_sha256']==digest(a.manifest) and binding['expected_outputs_used_as_inputs'] is False,'native configuration did not bind immutable candidate')
 require(runtime['returncode']==0 and runtime['fixture_success'] is True and runtime['framework_boundary_returned'] is True and runtime['framework_cleanup_proven'] is True and runtime['client_cleanup_proven'] is True,'original capture not closed successfully')
 require(runtime['native_boundary_result']==0 and 'C6_LINUX_ACTION_LIFECYCLE uninitialize_return initialized 0 context_bound 0 executing 0 cleanup 1 dispatch_mode 0 requested_execution 0 core_component_mode 0' in runtime['stderr'].splitlines(),'original cleanup footer absent')
 require(len(config['sources'])==62,'native source set differs')
 for name,value in config['sources'].items():
  require(not Path(name).is_absolute() and '..' not in Path(name).parts,'archived source path escapes')
  require(digest(capture/'source'/name)==value,'actual archived native source differs: '+name)
 require(digest(capture/'source/tools/linux/mod_activation_probe.py')==config['runner_sha256'],'actual archived runner differs')
 require(digest(capture/'placement-metadata-preload.so')==config['preload_sha256'],'actual admitted preload differs')
 require(load(capture/'configuration.json')==config,'durable native configuration differs')
 require(digest(capture/'candidate-manifest.json')==MANIFEST,'durable native candidate manifest differs')
 facts_path=capture/'facts.jsonl';require(digest(facts_path)==FACTS and file_identity(facts_path)==runtime['placement_metadata_output'],'closed facts changed')
 facts=[parse(x) for x in facts_path.read_text().splitlines()]
 require(facts[0]['kind']=='header' and facts[-1]['kind']=='summary' and all(r['kind'] in {'header','unit_definitions','relations','player','members','summary'} for r in facts),'native fact framing/kinds differ')
 require(sum(r['kind']=='header' for r in facts)==sum(r['kind']=='summary' for r in facts)==1,'duplicate fact framing')
 header,summary=facts[0],facts[-1]
 require(header['schema']=='c6-linux-placement-metadata-native-v1' and header['game_sha256']==GAME and header['profile_sha256']==profile and header['player_slots']==65 and header['complete_allocator_state'] is False,'native fact profile differs')
 require(summary['status']==0 and summary['known_rng_equal'] is True and summary['rng_before']==summary['sync_rng_after']+summary['map_rng_after'] and summary['complete_rng_inventory'] is False,'native known RNG/summary differs')
 data,controls=closed_controls(capture,runtime)
 trace_path=ROOT.parent/'c6-ghidra/evidence/linux-save-normalization/checkpoint-203-owners/trace-benchmark-1.jsonl.gz';trace=gzip.decompress(trace_path.read_bytes());require(sha(trace)==TRACE,'owner trace differs')
 source_gate(ROOT.parent/'c6-assets-linux/libGameCore_XP2.so');alive=compare_actual_alive(saved_player_alive(data,trace),facts)
 if a.diagnostics:
  directory=a.diagnostics.resolve();directory.mkdir(parents=True,exist_ok=False)
 else:
  parent=ROOT.parent/'c6-ghidra/evidence/linux-placement-count-reconstruction';parent.mkdir(parents=True,exist_ok=True);directory=Path(tempfile.mkdtemp(prefix='native-1-',dir=parent))
 request,coverage=make_request(data,facts,trace,directory,profile)
 request_path=directory/'input.json';request_path.write_bytes(canonical(request)+b'\n')
 result=subprocess.run([str(binary),str(request_path),profile,profile],capture_output=True,text=True)
 (directory/'candidate.stderr').write_text(result.stderr);(directory/'candidate.stdout').write_text(result.stdout)
 require(result.returncode==0,'frozen candidate rejected genuine inputs; preserved '+str(directory))
 calculated=parse(result.stdout)
 require(calculated['schema']=='c6-linux-placement-count-results-v1' and calculated['gamecore_sha256']==GAME and calculated['configuration_sha256']==calculated['profile_sha256']==profile and calculated['typed_input_state_unchanged'] is True and calculated['native_execution'] is False and calculated['native_handler_comparison'] is False,'candidate output identity/scope differs')
 # Open native expected answers ONLY NOW, after the pure facts→candidate run.
 expected_path=capture/'expected-counts.jsonl';require(digest(expected_path)==EXPECTED and file_identity(expected_path)==runtime['placement_metadata_expected_output'],'closed expected answers changed')
 expected=[parse(x) for x in expected_path.read_text().splitlines()]
 require(expected[0]['schema']=='c6-linux-placement-count-expected-v1' and expected[0]['profile_sha256']==profile and expected[0]['answers_are_inputs'] is False and expected[0]['native_handler_query_available'] is False and expected[-1]=={'kind':'summary','status':0},'native expected framing/profile differs')
 require(all(r['kind'] in {'header','count_expected','unit_binding_expected','summary'} for r in expected) and sum(r['kind']=='header' for r in expected)==sum(r['kind']=='summary' for r in expected)==1,'unexpected oracle row kind/framing')
 count_rows=[r for r in expected if r['kind']=='count_expected'];binding_rows=[r for r in expected if r['kind']=='unit_binding_expected']
 require(len(count_rows)==7*(coverage['units']+coverage['formations']) and len(binding_rows)==coverage['units'],'native full actor/query coverage missing')
 mismatches=compare_rows(calculated['count_results'],count_rows,'count_result','count_expected')
 binding_mismatches=compare_rows(calculated['unit_binding_results'],binding_rows,'unit_binding_result','unit_binding_expected')
 (directory/'mismatches.json').write_text(json.dumps(dict(counts=mismatches,bindings=binding_mismatches),indent=2)+'\n')
 handler=calculated['handler_results'];require(len(handler)==coverage['units'] and all(r['native_compared'] is False for r in handler),'computed handler coverage/scope differs')
 groups=collections.Counter((r['member_kind'],'current' if r['direction']==-1 else 'neighbor','present' if r['plot_present'] else 'null') for r in count_rows)
 report=dict(schema='c6-linux-placement-count-native-comparison-v1',status='equal' if not mismatches and not binding_mismatches else 'mismatched',gamecore_sha256=GAME,host_sha256=HOST,profile_sha256=profile,
  candidate_sha256=manifest['candidate_sha256'],candidate_source_commit=manifest['source_commit'],candidate_manifest_sha256=digest(a.manifest),source_snapshot_sha256=manifest['source_snapshot_sha256'],
  runtime_sha256=digest(runtime_path),prepared_sha256=digest(prepared_path),facts_sha256=digest(facts_path),native_expected_sha256=digest(expected_path),
  actual_archived_native_sources_checked=62,archived_runner_sha256=config['runner_sha256'],admitted_preload_sha256=config['preload_sha256'],
  checker_sha256=digest(Path(__file__)),alive_checker_sha256=digest(ROOT/'tools/linux/player_alive_saved_crosscheck.py'),candidate_input_projection_sha256=sha(canonical(request)),candidate_output_sha256=sha(result.stdout.encode()),
  coverage=coverage,native_count_rows_compared=len(count_rows),count_mismatches=len(mismatches),native_binding_rows_compared=len(binding_rows),binding_mismatches=len(binding_mismatches),
  query_groups=[dict(member_kind=k[0],position=k[1],plot=k[2],rows=v) for k,v in sorted(groups.items())],
  actual_core_alive_saved_header_crosscheck=alive,computed_handler_results=len(handler),computed_handler_unsupported=sum('unsupported' in r for r in handler),native_handler_comparison=False,
  core_controls=controls,known_sync_and_map_rng_equal=True,complete_rng_inventory=False,typed_input_state_unchanged=True,
  diagnostics_path=str(directory),expected_outputs_used_as_inputs=False,original_game_execution_by_checker=False,
  gameplay_action_acceptance=False,complete_state_actions_turns=False,unknown_bytes_masked=0,historical_raw_save_load_difference_preserved=89198)
 a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps({k:report[k] for k in ['status','native_count_rows_compared','count_mismatches','native_binding_rows_compared','binding_mismatches','computed_handler_unsupported']}))
 if mismatches or binding_mismatches:raise SystemExit(1)
if __name__=='__main__':main()
