#!/usr/bin/env python3
"""Source-only full Manager ownership guards; no original code execution."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
from current_handler_field_guard_audit import METHODS
from save_lifecycle_audit import Elf,GAME_SHA,HOST_SHA
ROOT=Path(__file__).resolve().parents[2]
SOURCES=['native/linux/full_manager_profile_guard.c','native/linux/full_manager_profile_guard.h','native/linux/mod_activation.c','tools/linux/full_manager_profile_guard_audit.py']
def sha(raw):return hashlib.sha256(raw).hexdigest()
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);args=p.parse_args()
 images={'core':Elf(ROOT.parent/'c6-assets-linux/libGameCore_XP2.so',GAME_SHA),'host':Elf(ROOT.parent/'c6-ghidra/evidence/linux-rules/asl-launch-xp2-2/Civ6',HOST_SHA)}
 wanted={0x290c910,0x2906890,0x2661530,0x31d1720,0x31d5090,0x2c7c9b0,0x2c7cae0};methods=[]
 for owner,rva,size,digest in METHODS:
  if rva in wanted:
   assert sha(images[owner].read(rva,size))==digest
   methods.append(dict(owner=owner,address=hex(rva),bytes=size,sha256=digest))
 assert len(methods)==7
 with tempfile.TemporaryDirectory(prefix='c6-full-owner-') as temp:
  work=Path(temp);source=work/'test.c';source.write_text('#include "'+str(ROOT/'native/linux/full_manager_profile_guard.c')+'"\n'+HARNESS)
  flags=['cc','-std=gnu11','-Wall','-Wextra','-Werror','-Wconversion','-Wshadow']
  subprocess.run(flags+['-fanalyzer','-c',str(ROOT/'native/linux/full_manager_profile_guard.c'),'-o',str(work/'guard.o')],check=True)
  subprocess.run(flags+['-O2','-fsanitize=undefined',str(source),'-ldl','-o',str(work/'test')],check=True)
  subprocess.run([str(work/'test')],check=True,capture_output=True)
 source=(ROOT/'native/linux/mod_activation.c').read_text()
 assert 'getenv("C6_CURRENT_HANDLER_METADATA")||getenv("C6_FULL_MANAGER_FIXTURE")' in source
 report=dict(schema='c6-linux-full-manager-profile-owner-source-v1',native_execution=False,integrated_profile_admission=False,original_calls_in_guard=False,methods=methods,source_sha256={n:sha((ROOT/n).read_bytes()) for n in SOURCES},synthetic_cases=dict(accepted_unlocked_before_and_released_after=True,accepted_genuine_caller_mock_lock_cycle=True,queued_work_mutations_retained=True,actor_memory_protected_without_guard_dereference=True,targeted_fatal_refusals=10,strict_c=True,gcc_analyzer=True,ubsan=True),limits=['raw token/depth require separately proven original facade acquisition','Lua/GC and original caller ancestry require actual all-stop observer','no native decision boundary inferred from idle','no actor pointer retained or post-query membership equality requirement','no complete affected-state/RNG or predicate acceptance'])
 args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')
HARNESS=r'''
#include <assert.h>
#include <sys/mman.h>
#include <sys/wait.h>
static void put(uintptr_t p,uintptr_t v){memcpy((void *)p,&v,8);}
static void put32(uintptr_t p,uint32_t v){memcpy((void *)p,&v,4);}
int main(void){
 void *host=mmap((void *)0x6499000,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS|MAP_FIXED_NOREPLACE,-1,0);assert(host==(void *)0x6499000);
 void *core=mmap(NULL,0x4000000,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);assert(core!=MAP_FAILED);uintptr_t base=(uintptr_t)core;
 unsigned char controller[0x500]={0},context[16]={0},native[0x40]={0},game[0x300]={0};
 uintptr_t c=(uintptr_t)controller,x=(uintptr_t)context,n=(uintptr_t)native,g=(uintptr_t)game;
 put(0x6499100,c);controller[0xb]=1;put(c+0x450,x);put(x,123);put(x+8,n);native[8]=1;put(n+0x28,g);game[0x2ac]=1;put(base+0x3c051a8,g);
 full_scope=(FullManagerScope){.module=(void *)1,.context=context,.base=base,.context_vptr=123,.native_context=n,.game=g,.controller=c,.main_tid=syscall(SYS_gettid)};
 assert(!c6_linux_full_manager_pre_acquisition((void *)1,context,game));
 put32(c+0x4a0,7);put32(c+0x4a4,1);assert(!c6_linux_full_manager_locked_begin((void *)1,context,game,base,syscall(SYS_gettid)));
 /* Queued processing state may change, and no actor address belongs to guard. */
 *(unsigned char *)(base+0x3c05cb8)=1;*(unsigned char *)(base+0x3c05cb9)=1;*(unsigned char *)(base+0x3c05cbb)=1;put(base+0x3c07348,0x1234);
 void *actor=mmap(NULL,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);assert(actor!=MAP_FAILED);assert(!mprotect(actor,4096,PROT_NONE));
 assert(!c6_linux_full_manager_retained_owner((void *)1,context,game));assert(*(unsigned char *)(base+0x3c05cb8)==1&&*(unsigned char *)(base+0x3c05cb9)==1&&*(unsigned char *)(base+0x3c05cbb)==1&&full_ptr(base+0x3c07348)==0x1234);assert(!munmap(actor,4096));
 const char *reasons[]={"controller_or_worker_unknown","retained_native_lock_changed","retained_native_lock_changed","context_or_game_owner_changed","context_or_game_owner_changed","object_extent_unknown","object_extent_unknown","scope_or_main_thread_unknown","admission_identity_changed","unlocked_boundary_unknown"};
 for(unsigned i=0;i<10;i++){
  FILE *error=tmpfile();assert(error);pid_t pid=fork();assert(pid>=0);
  if(!pid){assert(dup2(fileno(error),STDERR_FILENO)>=0);full_scope.main_tid=syscall(SYS_gettid);
   switch(i){case 0:put(0x6499108,1);break;case 1:put32(c+0x4a0,8);break;case 2:put32(c+0x4a4,2);break;case 3:native[8]=0;break;case 4:put(n+0x28,0);break;case 5:full_scope.native_context=UINTPTR_MAX-8;break;case 6:full_scope.controller=UINTPTR_MAX-8;break;case 7:full_scope.main_tid++;break;case 8:c6_linux_full_manager_retained_owner((void *)2,context,game);break;case 9:full_scope.locked=0;full_check(0);break;}
   c6_linux_full_manager_retained_owner((void *)1,context,game);_exit(1);
  }
  int status;assert(waitpid(pid,&status,0)==pid&&WIFEXITED(status)&&WEXITSTATUS(status)==240);rewind(error);char text[512]={0};assert(fread(text,1,sizeof text-1,error)>0&&strstr(text,reasons[i]));assert(!fclose(error));
 }
 assert(!c6_linux_full_manager_locked_end((void *)1,context,game));put32(c+0x4a0,0);put32(c+0x4a4,0);assert(!c6_linux_full_manager_released_owner((void *)1,context,game));
 assert(!c6_linux_full_manager_profile_admission_end((void *)1,context));assert(!full_scope.module);
 assert(!munmap(core,0x4000000));assert(!munmap(host,4096));return 0;
}
'''
if __name__=='__main__':main()
