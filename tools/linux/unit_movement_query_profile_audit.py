#!/usr/bin/env python3
"""Retained-source movement-only caller/guard review; never executes original code."""
import argparse
import hashlib
import json
import re
from pathlib import Path
import subprocess
import sys
import tarfile
from current_handler_caller_audit import METHODS as FACADE
from current_handler_field_guard_audit import METHODS as OWNER
from script_can_start_boundary_audit import METHODS as LOCKS
from save_lifecycle_audit import Elf,GAME_SHA,HOST_SHA
ROOT=Path(__file__).resolve().parents[2]
NATIVE=['native/linux/unit_movement_query_profile.c','native/linux/unit_movement_query_profile_guard.c',
 'native/linux/unit_movement_query_profile_guard.h','native/linux/unit_movement_query_fixture.c',
 'native/linux/unit_movement_query_fixture.h']
HARNESS='tools/linux/unit_movement_query_profile_test.c'
SUCCESS=['normal','rng-effects','changed-env','capture-error']
# expected reason, kind, and last actual mock-original calls before refusal
FATAL={
 'main-missing':('main_entry_unwitnessed','owner',[]),
 'main-zero':('scope_or_main_thread_unknown','owner',[]),
 'context-dladdr':('context_module_unknown','owner',[]),
 'context-unreadable':('field_unreadable','owner',[]),
 'controller-extent':('object_extent_unknown','owner',[]),
 'pre-worker':('controller_or_worker_unknown','owner',[]),
 'pre-lock':('unlocked_boundary_unknown','owner',[]),
 'without-admission':('admission_identity_changed','owner',[]),
 'duplicate-admission':('duplicate_or_missing_admission','owner',[]),
 'profile-missing':('hash_input_missing','profile',[]),
 'profile-invalid':('hash_input_invalid','profile',[]),
 'candidate-unsupported':('candidate_reference_unsupported','profile',[]),
 'path-missing':('path_input_missing','profile',[]),
 'duplicate-path':('duplicate_paths','profile',[]),
 'acquire-binding':('original_binding_changed','profile',[]),
 'release-binding':('original_binding_changed','profile',[]),
 'decrement-binding':('original_binding_changed','profile',[]),
 'acquired-facade-null':('facade_identity_unknown','profile',['acquire']),
 'acquired-facade-vptr':('facade_identity_unknown','profile',['acquire']),
 'acquired-facade-refcount':('facade_identity_unknown','profile',['acquire']),
 'zero-token':('retained_native_lock_changed','owner',['acquire']),
 'recursive-lock':('retained_native_lock_changed','owner',['acquire']),
 'wrong-lock-base':('locked_begin_identity_unknown','owner',['acquire']),
 'wrong-lock-tid':('locked_begin_identity_unknown','owner',['acquire']),
 'nested-profile':('nested_profile','profile',['acquire','capture']),
 'wrong-thread':('scope_or_main_thread_unknown','owner',['acquire','capture']),
 'worker':('controller_or_worker_unknown','owner',['acquire','capture']),
 'native-owner':('retained_native_lock_changed','owner',['acquire','capture']),
 'native-depth':('retained_native_lock_changed','owner',['acquire','capture']),
 'game-owner':('context_or_game_owner_changed','owner',['acquire','capture']),
 'controller-owner':('controller_or_worker_unknown','owner',['acquire','capture']),
 'facade-vptr':('facade_identity_unknown','profile',['acquire','capture']),
 'facade-refcount':('facade_identity_unknown','profile',['acquire','capture']),
 'facade-game':('facade_identity_unknown','profile',['acquire','capture']),
 'facade-destroy':('facade_identity_unknown','profile',['acquire','capture']),
 'utility-object':('release_utility_changed','profile',['acquire','capture']),
 'utility-vptr':('utility_dispatch_unknown','profile',['acquire','capture']),
 'utility-unlock':('utility_dispatch_unknown','profile',['acquire','capture']),
 'release-lock-remains':('unlocked_boundary_unknown','owner',['acquire','capture','release']),
 'release-context-changed':('context_or_game_owner_changed','owner',['acquire','capture','release']),
}
def sha(b):return hashlib.sha256(b).hexdigest()
def require(value,reason):
 if not value:raise ValueError(reason)
def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--directory',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
 p.add_argument('--source-commit',help='require every retained source to match this committed preimage')
 p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux');a=p.parse_args()
 a.directory=a.directory.resolve();a.directory.mkdir(parents=True,exist_ok=False)
 images={'core':Elf(a.assets/'libGameCore_XP2.so',GAME_SHA),'host':Elf(a.assets/'Civ6',HOST_SHA)}
 gates=[]
 for address,size,digest in FACADE:
  require(sha(images['core'].read(address,size))==digest,'facade source mismatch')
  gates.append(dict(image='core',address=hex(address),bytes=size,sha256=digest))
 for image,address,size,digest in OWNER:
  if address not in [0x290c910,0x2906890,0x2661530,0x31d1720,0x31d5090,0x2c7c9b0,0x2c7cae0]:continue
  require(sha(images[image].read(address,size))==digest,'owner source mismatch')
  gates.append(dict(image=image,address=hex(address),bytes=size,sha256=digest))
 for name,address,size,digest in LOCKS:
  if not name.startswith(('GameCore::EngineUtility::','GameCoreController::')):continue
  require(sha(images['host'].read(address,size))==digest,'host lock source mismatch')
  gates.append(dict(image='host',name=name,address=hex(address),bytes=size,sha256=digest))
 movement=json.loads((ROOT/'evidence/linux/unit-movement-query-abi.json').read_text())
 require(movement['gamecore_sha256']==GAME_SHA,'movement ABI target mismatch')
 for name,row in movement['methods'].items():
  require(sha(images['core'].read(int(row['address'],16),row['bytes']))==row['sha256'],'movement body differs: '+name)
 sources=NATIVE+[HARNESS,'tools/linux/unit_movement_query_profile_audit.py','evidence/linux/unit-movement-query-abi.json']
 # Keep every imported local audit dependency's complete source preimage too.
 for module in list(sys.modules.values()):
  value=getattr(module,'__file__',None)
  if value and Path(value).resolve().is_relative_to(ROOT/'tools/linux'):
   sources.append(str(Path(value).resolve().relative_to(ROOT)))
 sources=sorted(set(sources));payloads={name:(ROOT/name).read_bytes()for name in sources}
 require(all((ROOT/name).read_bytes()==raw for name,raw in payloads.items()),'source changed during snapshot')
 pinned=None
 if a.source_commit:
  pinned=subprocess.check_output(['git','rev-parse',a.source_commit+'^{commit}'],cwd=ROOT,text=True).strip()
  for name,raw in payloads.items():
   require(subprocess.check_output(['git','show',pinned+':'+name],cwd=ROOT)==raw,'source differs from committed preimage: '+name)
 source=a.directory/'source';source.mkdir()
 for name,raw in payloads.items():
  path=source/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(raw)
 with tarfile.open(a.directory/'source.tar','w') as archive:
  for name in sources:archive.add(source/name,arcname=name)
 guard=(source/NATIVE[1]).read_text();caller=(source/NATIVE[0]).read_text()
 require('static MovementQueryScope movement_scope;' in guard,'separate scope missing')
 require('c6_linux_full_manager' not in guard+caller and 'full_scope' not in guard+caller,'forbidden scope alias')
 require('c6_linux_context_save' not in caller and 'memcmp' not in caller,'unexpected Save/equality gate')
 flags=['gcc','-std=gnu11','-Wall','-Wextra','-Werror','-Wconversion','-Wshadow']
 for name in [NATIVE[0],NATIVE[1],NATIVE[3]]:
  obj=a.directory/(Path(name).stem+'.o')
  subprocess.run(flags+['-fanalyzer','-fPIC','-c',str(source/name),'-o',str(obj)],check=True)
  undefined=subprocess.check_output(['nm','-u',str(obj)],text=True)
  require('c6_linux_full_manager' not in undefined,'linked forbidden Manager dependency')
 executable=a.directory/'test'
 subprocess.run(flags+['-O2','-fsanitize=undefined',str(source/HARNESS),'-ldl','-pthread','-o',str(executable)],check=True)
 linked=subprocess.check_output(['ldd',str(executable)],text=True)
 (a.directory/'test.ldd').write_text(linked)
 dependencies={}
 for line in linked.splitlines():
  require('not found' not in line,'unresolved test dependency')
  match=re.search(r'(?:=>\s*)?(/[^\s]+)',line)
  if match:
   path=Path(match.group(1)).resolve()
   require('GameCore' not in path.name and path.name!='Civ6','original code linked into test')
   dependencies[str(path)]=sha(path.read_bytes())
 cases=[]
 for mode in SUCCESS+list(FATAL)+['capture-fatal']:
  work=a.directory/mode;work.mkdir()
  result=subprocess.run([str(executable),mode],cwd=work,capture_output=True,text=True)
  (work/'stdout').write_text(result.stdout);(work/'stderr').write_text(result.stderr)
  events=(work/'events.txt').read_text().splitlines()if(work/'events.txt').exists()else[]
  if mode in SUCCESS:
   require(result.returncode==0,mode+': '+result.stderr)
   require(events==['acquire','capture','release'],mode+': facade/capture call order')
   rows=[json.loads(line.split('C6_UNIT_MOVEMENT_QUERY_PROFILE ',1)[1])for line in result.stderr.splitlines()if line.startswith('C6_UNIT_MOVEMENT_QUERY_PROFILE {')]
   require(rows[0]['schema']=='c6-linux-unit-movement-query-profile-v1' and rows[0]['pure_query_claimed']is False and rows[0]['Save_equality_is_gate']is False,mode+': scope claims')
   phases=[r for r in rows if r['kind']=='phase']
   require([r['phase']for r in phases]==['before_acquisition','before_queries','after_queries','after_release'],mode+': phase order')
   require([(r['native_owner'],r['native_depth'])for r in phases]==[(0,0),(7,1),(7,1),(0,0)],mode+': actual lock observations')
   if mode=='rng-effects':require(phases[2]['known_rng_raw'].startswith('a5') and phases[2]['known_rng_raw'][64:66]=='5a' and phases[3]['known_rng_raw']==phases[2]['known_rng_raw'],'RNG effects lost or vetoed')
   require(rows[-1]['status']==(243 if mode=='capture-error'else 0)and rows[-1]['facade_released']is True and rows[-1]['native_query_values_compared']is False,mode+': return semantics')
   require('queued effects retained; released facade protected' in result.stdout,mode+': mutation/lifetime checks missing')
   cases.append(dict(mode=mode,mock_original_order=events,balanced=True,queued_effects_retained=True,released_facade_protected=True))
  elif mode=='capture-fatal':
   require(result.returncode==240 and events==['acquire','capture'],'capture fatal invoked cleanup/original calls')
   cases.append(dict(mode=mode,fatal_exit=240,mock_original_order=events,no_cleanup_after_fatal=True))
  else:
   reason,kind,expected=FATAL[mode]
   require(result.returncode==240 and events==expected,mode+': refusal/call count '+repr(result.stderr))
   if kind=='owner':require('"reason":"'+reason+'"' in result.stderr and '"no_further_original_calls":true' in result.stderr,mode+': owner reason')
   else:require('fatal '+reason+' no_further_original_calls\n' in result.stderr,mode+': caller reason')
   cases.append(dict(mode=mode,fatal_exit=240,targeted_reason=reason,mock_original_order=events))
 require(all((source/name).read_bytes()==raw for name,raw in payloads.items()),'retained source changed')
 for name in sources:(source/name).chmod(0o444)
 revision=pinned or subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
 status=subprocess.check_output(['git','status','--short','--',*sources],cwd=ROOT,text=True).splitlines()
 report=dict(schema='c6-linux-unit-movement-query-profile-source-v1',gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,
  source_revision=revision,all_retained_sources_match_commit=pinned is not None,uncommitted_source_paths=status,sources={name:sha(raw)for name,raw in payloads.items()},
  complete_source_preimages_retained=True,source_archive_sha256=sha((a.directory/'source.tar').read_bytes()),
  candidate_directory=str(a.directory),test_candidate_sha256=sha(executable.read_bytes()),cases=cases,
  gcc=subprocess.check_output(['gcc','--version'],text=True),python=sys.version,compiler_flags=flags,
  test_dynamic_dependencies=dependencies,
  method_gates=gates,movement_method_gates=len(movement['methods']),strict_c=True,gcc_analyzer=True,ubsan=True,
  original_execution=False,original_calls_in_guard=False,native_profile_admitted=False,
  native_query_values_compared=False,save_equality_is_gate=False,pure_query_claimed=False,
  pending=['sole-runner loader/lifecycle/CLI and actual main-entry witness integration',
   'complete exact profile/module/host/dependencies/rules/input identity and original top-level ownership review',
   'independent caller/owner source review before one fresh original execution',
   'actual1262 query inputs/answers and source-matched Rust comparison; complete affected state/RNG remains separate'])
 a.output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')
 (a.directory/'source.tar').chmod(0o444);executable.chmod(0o555)
 print(json.dumps(dict(success=len(SUCCESS),fatal_refusals=len(FATAL)+1,test_candidate_sha256=report['test_candidate_sha256'],source_archive_sha256=report['source_archive_sha256'],original_execution=False),sort_keys=True))
if __name__=='__main__':main()
