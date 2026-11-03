#!/usr/bin/env python3
"""Source/ABI and synthetic lifecycle checks; never loads original game code."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tempfile
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
from current_handler_caller_audit import METHODS as FACADE
from skip_turn_handler_query_audit import EXTRA
from unit_operation_manager_audit import METHODS as MANAGER
from movement_probe_audit import ENTRYPOINTS as MOVEMENT

ROOT=Path(__file__).resolve().parents[2]
SOURCE='native/linux/full_manager_fixture.c'
HEADER='native/linux/full_manager_fixture.h'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    core=Elf(args.assets/'libGameCore_XP2.so',GAME_SHA)
    Elf(args.assets/'Civ6',HOST_SHA)
    methods=[('facade_%x'%a,a,n,h) for a,n,h in FACADE]
    methods += [row for row in EXTRA if row[0] in
                ('component_at','map_find','map_end','operation_ctor','operation_dtor','shared_dtor')]
    methods += MANAGER
    methods += [(name,address,size,h) for name,address,size,symbol,h in MOVEMENT if name in
                ('units','player_manager','find_unit')]
    # Existing source-audited original roster lookup and full recycled Find.
    # Size/name symbols are checked independently against the pinned ELF.
    extra=[('GetValidPlayer',0x2003940,39),('PlayerManager.IsValid',0x2004fc0,51),
           ('Unit.GetID',0x2021490,7),('Unit.GetFormation',0x20214e0,8),
           ('RecycledUnit.Find',0x2baa650,84)]
    # Fingerprints below are frozen source facts, not runtime oracle answers.
    fingerprints={
      'GetValidPlayer':'58bb7297d06455fd64617c3472f7087054b4aa5fa1b28ab20ce94cb74db41648',
      'PlayerManager.IsValid':'31c2b843f442ca71b8fd27edeccb2888940a331a9f50686c2a1e078cd25be038',
      'Unit.GetID':'317ef4dac77c526b04292f52727705f4b2857f4e0c5620bf48414d69903402e5',
      'Unit.GetFormation':'0851dd84ca94b57f3265451fe81d1a7b19c6f7d57018eb10d0ea8a35255485d0',
      'RecycledUnit.Find':'23187b5ffe8639ccf9e9f356f40d40c171a4e81ef1d4488b412aa1262744a899',
    }
    methods += [(name,address,size,fingerprints[name]) for name,address,size in extra]
    for name,address,size,h in methods:
        if sha(core.read(address,size))!=h:
            raise ValueError('source body mismatch: '+name)
    symbols={}
    for line in subprocess.check_output(['nm','-D','--defined-only',str(args.assets/'libGameCore_XP2.so')],text=True).splitlines():
        fields=line.split()
        if len(fields)==3:
            symbols[fields[2]]=int(fields[0],16)
    bindings=re.findall(r'BIND\((\w+),"([^"]+)",(0x[0-9a-f]+)\)',(ROOT/SOURCE).read_text())
    if len(bindings)!=14:
        raise ValueError('complete callable binding list required')
    for field,symbol,address in bindings:
        if symbols.get(symbol)!=int(address,16):
            raise ValueError('original symbol mismatch: '+field+' '+symbol)
    with tempfile.TemporaryDirectory(prefix='c6-full-manager-audit-') as temp:
        work=Path(temp)
        test=work/'test.c'
        test.write_text('#include "'+str(ROOT/SOURCE)+'"\n'+HARNESS)
        flags=['cc','-std=gnu11','-Wall','-Wextra','-Werror','-Wconversion','-Wshadow']
        subprocess.run(flags+['-fanalyzer','-c',str(ROOT/SOURCE),'-o',str(work/'fixture.o')],check=True)
        marker_contracts=[]
        for optimization in ('-O0','-O2'):
            marker_object=work/('marker'+optimization+'.o')
            subprocess.run(flags+[optimization,'-fcf-protection=branch','-c',str(ROOT/SOURCE),'-o',str(marker_object)],check=True)
            listing=subprocess.check_output(['objdump','-d',str(marker_object)],text=True)
            body=listing.split('<c6_linux_full_manager_fixture_marker>:',1)[1].split('\n\n',1)[0]
            raw=bytes.fromhex(' '.join(match.group(1) for match in re.finditer(
                r'^\s*[0-9a-f]+:\s+((?:[0-9a-f]{2}\s+)+)',body,re.M)))
            if not (raw.startswith(bytes.fromhex('f30f1efac3')) or raw.startswith(b'\xc3')):
                raise ValueError('compiled own marker is not return-only at '+optimization)
            marker_contracts.append(dict(optimization=optimization,CET_branch=True,raw=raw.hex(),sha256=sha(raw)))
        subprocess.run(flags+['-O2','-fsanitize=undefined',str(test),'-ldl','-o',str(work/'test')],check=True)
        result=subprocess.run([str(work/'test')],capture_output=True,text=True)
        if result.returncode:
            raise ValueError('synthetic lifecycle failed: '+result.stderr)
    sources={p:sha((ROOT/p).read_bytes()) for p in [SOURCE,HEADER,
        'tools/linux/full_manager_fixture_audit.py','native/linux/context_stream.c',
        'native/linux/context_stream.h','tools/linux/script_manager_observer.py']}
    for module in list(__import__('sys').modules.values()):
        path=getattr(module,'__file__',None)
        if path and Path(path).resolve().is_relative_to(ROOT/'tools/linux'):
            relative=str(Path(path).resolve().relative_to(ROOT))
            sources[relative]=sha(Path(path).read_bytes())
    report=dict(schema='c6-linux-full-manager-fixture-source-v1',native_execution=False,
        host_sha256=HOST_SHA,gamecore_sha256=GAME_SHA,sources=sources,
        checked_methods=[dict(name=n,address=hex(a),size=z,sha256=h) for n,a,z,h in methods],
        checked_exports=[dict(field=f,symbol=s,address=a) for f,s,a in bindings],
        checks=dict(strict_c=True,gcc_analyzer=True,ubsan_synthetic_lifetime=True,
                    synthetic_summary=result.stdout.strip(),compiled_marker_contracts=marker_contracts),
        original_calls=['genuine Context.GetGame facade acquisition/release',
                        'original Manager registry and Player/Unit membership lookup',
                        'original Operation construction/complete Manager.CanStart/destruction',
                        'optional original Save diagnostics'],
        guard_original_calls=False,predicate_invocations_per_fixture_at_most=1,
        native_profile_admitted=False,full_manager_native_parity=False,
        save_identity_acceptance_gate=False,complete_state_actions_rng_turns=False,
        pending=['distinct runner-owned main-entry/ownership guard source and exact profile',
                 'external hardware Lua observer integration and safe-window provenance',
                 'independent caller source/lifetime review before sole-runner execution',
                 'all affected gameplay/VM state, callback effects, complete RNG/order comparison'])
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')


HARNESS=r'''
#include <assert.h>
#include <sys/mman.h>
#include <sys/wait.h>
static unsigned calls,destructions,observations;
static bool answer,observer_error,held;
static unsigned saves_done,phase_count;
static unsigned char facade[0x1c],utility_object[8],controller[0x4a8];
static const unsigned char *test_unit;
static uintptr_t test_base;
static unsigned char manager[104],handler[16],unit[0x134],slots[8],game[0x120];
static void pword(void *p,size_t off,uint32_t v){memcpy((unsigned char *)p+off,&v,4);}
static void pptr(void *p,size_t off,uintptr_t v){memcpy((unsigned char *)p+off,&v,8);}
int c6_linux_context_save(void *m,void *c,const char *p){assert(m==(void *)1&&c==(void *)2&&p&&held);saves_done++;return saves_done==1?13:0;}
static int unlocked(void *m,void *c,const void *g){assert(m==(void *)1&&c==(void *)2&&g==game&&!held);return 0;}
static int begin(void *m,void *c,const void *g,uintptr_t b,long t){assert(m==(void *)1&&c==(void *)2&&g==game&&held&&b==test_base&&t==syscall(SYS_gettid));return 0;}
static int endpoints(const C6FullManagerObservation *o){assert(o->game==game);assert((unsigned)o->phase==++phase_count);assert(held==(o->phase!=C6_FULL_MANAGER_RELEASED));return 0;}
static void *acquire(void *c){assert(c==(void *)2&&!held);held=true;return facade;}
static void release(void *f){assert(f==facade&&held);held=false;}
static int owner_guard(void *m,void *c,const void *g){assert(m==(void *)1&&c==(void *)2&&g==game);return 0;}
static int owned(void *m,void *c,const void *g){assert(held);return owner_guard(m,c,g);}
static int observe(const C6FullManagerObservation *o){observations++;assert(o->unit==unit);return observer_error?19:0;}
static const void *get_manager(void){return manager;}
static int32_t resolve(const void *m,uint32_t h){assert(m==manager&&h==0x2c681908);return 0;}
static const void *get_handler(const void *m,uint32_t h){assert(m==manager&&h==0x2c681908);return handler;}
static const void *get_players(const void *g){assert(g==game);return (void *)3;}
static const void *get_player(const void *p,int32_t o){assert(p==(void *)3&&o==7);return (void *)4;}
static const void *get_units(const void *p){assert(p==(void *)4);return (void *)5;}
static const void *find_unit(const void *p,uint32_t id){assert(p==(void *)5&&id==0x120001);return test_unit;}
static uint32_t get_id(const void *u){assert(u==test_unit);return 0x120001;}
static const void *get_formation(const void *u){assert(u==test_unit);return (void *)6;}
static void construct(void *p,int32_t id){assert(id==0);memset(p,0,32);pword(p,0,(uint32_t)id);pword(p,4,UINT32_MAX);}
static void destroy(void *p){assert(word((uintptr_t)p)==0);destructions++;}
static bool can_start(const void *m,const void *u,const void *op,const void *plot,bool visible,void *results){
 assert(m==manager&&u==test_unit&&plot==NULL&&results==NULL&&visible);
 assert(word((uintptr_t)op)==0&&word((uintptr_t)op+4)==UINT32_MAX);
 assert(!ptr((uintptr_t)op+8)&&!ptr((uintptr_t)op+16));
 calls++;
 /* Model removal as an adversarial effect: fixture must not dereference or
  * re-query this previously retained Unit after the complete predicate. */
 test_unit=(const void *)1;return answer;
}
static void setup(void){
 void *image=mmap(NULL,0x3c08000,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);assert(image!=MAP_FAILED);
 test_base=(uintptr_t)image;
 assert(mmap((void *)0x53a6000,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS|MAP_FIXED_NOREPLACE,-1,0)==(void *)0x53a6000);
 assert(mmap((void *)0x6499000,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS|MAP_FIXED_NOREPLACE,-1,0)==(void *)0x6499000);
 pptr((void *)0x6499100,0,(uintptr_t)controller);
 pptr((void *)(test_base+0x3b13848),0,test_base+0x3c051b8);
 pptr((void *)(test_base+0x3c051b8),0,(uintptr_t)utility_object);
 pptr(utility_object,0,0x53a6c90);
 pptr((void *)0x53a6c90,0x30,0x32b3da0);pptr((void *)0x53a6c90,0x38,0x32b3d60);pptr((void *)0x53a6c90,0x40,0x32b3d40);
 pptr(facade,0,test_base+0x3aecf60);pptr(facade,8,(uintptr_t)game);pword(facade,0x18,1);
 pptr((void *)(test_base+0x3aecf68),0,test_base+0x2c86370);
 pptr(manager,0,test_base+0x3aff880);pptr(manager,24,(uintptr_t)slots);pptr(manager,32,(uintptr_t)slots+8);
 pptr((void *)(test_base+0x3aff880),0xa0,test_base+0x2da1720);pptr((void *)(test_base+0x3aff880),0x88,test_base+0x2da1650);
 pptr(handler,0,test_base+0x3b00db0);pword(handler,8,0);pword(handler,12,0x2c681908);
 pptr((void *)(test_base+0x3b00db0),0x18,test_base+0x2dac950);
 pptr(unit,0,test_base+0x3ae5f80);pword(unit,0x130,7);
}
int main(void){
 setup();
 Bindings b={.acquire=acquire,.release=release,.manager_get=get_manager,.id_for_hash=resolve,.handler_for_hash=get_handler,
  .construct=construct,.destroy=destroy,.can_start=can_start,.players=get_players,.player=get_player,
  .units=get_units,.find_unit=find_unit,.unit_id=get_id,.formation=get_formation};
 C6FullManagerFixtureAdmission a={.module=(void *)1,.context=(void *)2,.expected_game=game,.expected_unit=unit,
  .actor_owner=7,.actor_full_id=0x120001,.visible_only=true,.retained_owner=owner_guard,.observe=observe};
 for(unsigned pass=0;pass<3;pass++){
  calls=destructions=observations=0;answer=pass==1;observer_error=pass==2;test_unit=unit;
  C6FullManagerFixtureResult r={0};
  C6FullManagerObservation o={.game=game,.core_base=test_base};
  FILE *out=tmpfile();assert(out);invoke(&b,&a,&r,&o,out);assert(!fclose(out));
  assert(destructions==1&&o.operation==NULL&&o.unit==NULL);
  if(observer_error){assert(calls==0&&!r.predicate_invoked&&!r.predicate_returned&&r.observation_status==19);}
  else{assert(calls==1&&r.predicate_invoked&&r.predicate_returned&&r.allowed==answer);}
 }
 /* Exercise the entire concrete owned-facade sequence with mocked originals.
  * A diagnostic Save error is retained and does not block the one predicate. */
 calls=destructions=observations=saves_done=phase_count=0;answer=true;observer_error=false;test_unit=unit;held=false;
 a.profile_sha256="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
 char output[]="/tmp/c6-full-manager-lifetime-XXXXXX";int fd=mkstemp(output);assert(fd>=0);assert(!close(fd));assert(!unlink(output));
 a.output_path=output;a.main_entry_tid=syscall(SYS_gettid);a.pre_acquisition=unlocked;a.locked_begin=begin;
 a.retained_owner=owned;a.locked_end=owned;a.released_owner=unlocked;a.observe=endpoints;
 for(unsigned i=0;i<4;i++)a.diagnostic_save_paths[i]="unused-mock-path";
 C6FullManagerFixtureResult complete={0};
 assert(run_bound(&b,&a,&complete,test_base)==0);
 assert(calls==1&&destructions==1&&saves_done==4&&phase_count==9&&!held);
 assert(complete.predicate_returned&&complete.allowed&&complete.facade_released&&complete.save_status[0]==13);
 assert(!unlink(output));
 a.retained_owner=owner_guard;a.observe=observe;
 /* Refuse wrong loaded membership/runtime binding before constructor/query. */
 const char *reasons[]={"actor_not_loaded_member","runtime_id_unbound","admission_incomplete"};
 for(unsigned test=0;test<3;test++){
  FILE *errors=tmpfile();assert(errors);pid_t pid=fork();assert(pid>=0);
  if(!pid){
   assert(dup2(fileno(errors),STDERR_FILENO)>=0);test_unit=unit;
   if(test==0)a.expected_unit=(void *)1;
   if(test==1)pptr(manager,32,(uintptr_t)slots);
   C6FullManagerFixtureResult r={0};C6FullManagerObservation o={.game=game,.core_base=test_base};FILE *out=tmpfile();assert(out);
   if(test==2)c6_linux_full_manager_fixture_run(NULL,&r);else invoke(&b,&a,&r,&o,out);
   _exit(1);
  }
  int status;assert(waitpid(pid,&status,0)==pid);assert(WIFEXITED(status)&&WEXITSTATUS(status)==240);
  rewind(errors);char msg[512]={0};assert(fread(msg,1,sizeof msg-1,errors)>0);assert(strstr(msg,reasons[test]));assert(!fclose(errors));
 }
 puts("4 synthetic genuine-ABI lifetime branches including complete facade acquire/release; 3 exact fatal refusals; no original execution");
 return 0;
}
'''
if __name__=='__main__':
    main()
