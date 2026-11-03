#!/usr/bin/env python3
"""Static and synthetic checks only; no original code execution/admission."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
from script_can_start_boundary_audit import METHODS as LOCK_METHODS
ROOT=Path(__file__).resolve().parents[2]
METHODS=[('core', 43043088, 4, 'b8b506ba100e67d9b53e88d5932e47609176a987c2f3671cc7518998d70aa90c'), ('core', 43018384, 5, '2d832792ecfa9c618ffe1f26192ba64bd13a40ab3c84ef98c28625941ec82da2'), ('core', 40244528, 7, 'b9c0437d462a304c0c929efd584166221d343a3a1083305acc042a368f564b60'), ('core', 34620368, 8, 'c9693ada7527d268ae5471326fdb2c043cea947c3e0cd5a7c0ce00bdd65125b4'), ('core', 40270176, 11, 'a89e9b2aec9ce5626ceaddda700fd0185aa7f2f8c756352f3deba1cd7b55effe'), ('core', 40246208, 238, '6119c902f5807eb4e76248ef38b6bbfa410ee75ffc390c8bbe628ed39beb5e5c'), ('core', 44094624, 7, '733ad235cf233826592599499b3840b52fc33bd1e546160113fa54ef195b5f3e'), ('core', 44094656, 7, 'a9f5aaa3621073605ed33f3aa9914e584d4f5f294b675432d0a83f35f931f600'), ('core', 44091856, 7, '13fb2f85c99b5223fc2196c2630162fb4ac004d34a2c5c336f0d5ec46d8b00e5'), ('core', 47335808, 53, 'a84613f887d608c070c83feabe6e3f6e809b3809bef0ce19128c6853a19fd2dd'), ('core', 47335872, 7, 'a8c1e8a1c4e3665084e1aef2a384b299850f37a4889514cb0920cc03e14496d0'), ('core', 47337936, 52, 'ed844b9ff4030f5df5c60a4ed0f24909c2a054c73e7e956229b244939367998a'), ('host', 52238112, 4, '3cd8b2ec8d86bb7de69e12549d76b1a0e65dffefb2283ef3c0ed335598b4c769'), ('host', 52238496, 26, '116689463607c34540ee990b53566ad86b1f6ee2f010cffb8586ae97312519cc'), ('host', 52252816, 14, 'b20d05c730c4c95cd2e91be672eac57ad0b2bb795a563c6b581bcc88d34a1f77')]

def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--output',type=Path,required=True)
 args=p.parse_args()
 images={'core':Elf(ROOT.parent/'c6-assets-linux/libGameCore_XP2.so',GAME_SHA),
         'host':Elf(ROOT.parent/'c6-ghidra/evidence/linux-rules/asl-launch-xp2-2/Civ6',HOST_SHA)}
 methods=[]
 for kind,address,size,digest in METHODS:
  assert hashlib.sha256(images[kind].read(address,size)).hexdigest()==digest
  methods.append(dict(image=kind,address=hex(address),bytes=size,sha256=digest))
 for name,address,size,digest in LOCK_METHODS:
  if name.startswith('GameCore::EngineUtility::') or name.startswith('GameCoreController::'):
   assert hashlib.sha256(images['host'].read(address,size)).hexdigest()==digest
   methods.append(dict(image='host',name=name,address=hex(address),bytes=size,sha256=digest))
 with tempfile.TemporaryDirectory(prefix='c6-field-guard-') as temp:
  work=Path(temp)
  src=work/'test.c'
  src.write_text('#include "'+str(ROOT/'native/linux/current_handler_metadata_fixture.c')+'"\n'+HARNESS)
  cmd=['cc','-std=gnu11','-Wall','-Wextra','-Werror','-Wconversion','-Wshadow']
  subprocess.run(cmd+['-fanalyzer','-c',str(ROOT/'native/linux/current_handler_metadata_fixture.c'),'-o',str(work/'guard.o')],check=True)
  subprocess.run(cmd+['-O2','-fsanitize=undefined',str(src),'-o',str(work/'test')],check=True)
  subprocess.run([str(work/'test')],check=True,capture_output=True)
 sources={name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in
  ['native/linux/current_handler_metadata_fixture.c','native/linux/current_handler_metadata_fixture.h','tools/linux/current_handler_field_guard_audit.py']}
 report=dict(schema='c6-linux-current-handler-field-guard-source-v1',native_execution=False,
  integrated_admission=False,original_calls_in_guard=False,methods=methods,sources=sources,
  synthetic=dict(accepted_idle_scope=True,refusal_cases=17,strict_c=True,gcc_analyzer=True,ubsan=True),
  limits=['caller main-entry provenance and genuine facade acquisition still require integration',
          'file/mapping/runtime source gates and native lock ownership witness pending',
          'positive GlobalAi processing counter unsupported; both queue sentinels must be empty',
          'no current-handler/native dispatch/full Manager acceptance'])
 args.output.parent.mkdir(parents=True,exist_ok=True)
 args.output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')

HARNESS=r"""
#include <assert.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <sys/wait.h>
static void put(uintptr_t p,uintptr_t v){memcpy((void *)p,&v,8);}
static void put32(uintptr_t p,uint32_t v){memcpy((void *)p,&v,4);}
int main(void){
 void *globals=mmap((void *)0x6499000,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS|MAP_FIXED_NOREPLACE,-1,0);
 assert(globals==(void *)0x6499000);
 void *core=mmap(NULL,0x4000000,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);assert(core!=MAP_FAILED);
 unsigned char controller[0x500]={0},ctx[0x40]={0},native[0x40]={0},game[0x300]={0},ai[0x120]={0};
 uintptr_t b=(uintptr_t)core,c=(uintptr_t)controller,n=(uintptr_t)native,g=(uintptr_t)game;
 put(0x6499100,c);controller[0xb]=1;put(c+0x450,(uintptr_t)ctx);put32(c+0x4a0,7);put32(c+0x4a4,1);
 put((uintptr_t)ctx,123);put((uintptr_t)ctx+8,n);native[8]=1;put(n+0x28,g);put(b+0x3c051a8,g);game[0x2ac]=1;put(g+0x258,(uintptr_t)ai);
 for(unsigned i=0;i<2;i++){uintptr_t q=b+0x3c07348+24*i;put(q,q);put(q+8,q);}
 void *module=(void *)1;
 assert(!c6_linux_current_handler_metadata_fields_begin(module,ctx,game,b,syscall(SYS_gettid)));
 assert(!c6_linux_current_handler_metadata_quiescent(module,ctx,game));
 for(unsigned test=0;test<17;test++){
  pid_t child=fork();assert(child>=0);
  if(!child){
   switch(test){
    case 0:put(0x6499108,1);break;
    case 1:controller[0xb]=0;break;
    case 2:put(c+0x450,0);break;
    case 3:native[8]=0;break;
    case 4:game[0x2ac]=0;break;
    case 5:put32(c+0x4a0,8);break;
    case 6:put32(c+0x4a4,2);break;
    case 7:*(unsigned char *)(b+0x3c05cbb)=1;break;
    case 8:*(unsigned char *)(b+0x3c05cb9)=1;break;
    case 9:*(unsigned char *)(b+0x3c05cb8)=1;break;
    case 10:*(unsigned char *)(b+0x3c07378)=1;break;
    case 11:put32((uintptr_t)ai+0x110,1);break;
    case 12:put(b+0x3c07348,0);break;
    case 13:put(b+0x3c07368,0);break;
    case 14:put(n+0x28,0);break;
    case 15:scope.main_tid++;break;
    case 16:scope.native_context=1;break;
   }
   c6_linux_current_handler_metadata_quiescent(module,ctx,game);_exit(1);
  }
  int status;assert(waitpid(child,&status,0)==child);assert(WIFEXITED(status)&&WEXITSTATUS(status)==240);
 }
 assert(!c6_linux_current_handler_metadata_fields_end(module,ctx,game));
 assert(!scope.module);assert(!munmap(core,0x4000000));assert(!munmap(globals,4096));return 0;
}
"""
if __name__=='__main__':main()
