#!/usr/bin/env python3
"""Static caller preparation; synthetic refusals never load original modules."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
from save_lifecycle_audit import Elf,GAME_SHA
ROOT=Path(__file__).resolve().parents[2]
METHODS=[(46648032, 92, '0bee95a94a869e776ee221963d7e80aa885739a6d73aff21785121122d1a2c9f'), (46685408, 92, '981ad8a20e2991f4cb6211bc5b349ea550efa2dbc6d59625e4d9b69897b2d519'), (46685568, 56, '9fd728c0f0fe56b64e0fbacaeb0c28e9f73723000c86c57ec6e708ee5f3ae1b3'), (43039648, 22, 'ad7d214f06fbbe64670982468ad970bc59593dbb904cc8cf29764bd75e59f480'), (47690960, 8, 'f0c112fce3f22ba3110c9b92191ebdc9137f2f7c83a3906235e7ad9806bd2b4b'), (46687088, 5, 'ff38a02ea4fa34d0d0258df743b207c473530c7154151d18aa114aa4d4c7afd2'), (46689152, 40, '5920342050c4e1a7e3da5f1777b6c0687ba790afb7487f181e33fd95efff7755')]
def sha(data):return hashlib.sha256(data).hexdigest()
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);args=p.parse_args()
 core=Elf(ROOT.parent/'c6-assets-linux/libGameCore_XP2.so',GAME_SHA)
 for addr,size,digest in METHODS:assert sha(core.read(addr,size))==digest
 native=ROOT/'native/linux'
 with tempfile.TemporaryDirectory(prefix='c6-current-handler-caller-') as temp:
  work=Path(temp);test=work/'test.c'
  test.write_text('#include "'+str(native/'current_handler_metadata_probe.c')+'"\n'+HARNESS)
  flags=['cc','-std=gnu11','-Wall','-Wextra','-Werror','-Wconversion','-Wshadow']
  subprocess.run(flags+['-fanalyzer','-c',str(native/'current_handler_metadata_probe.c'),'-o',str(work/'probe.o')],check=True)
  subprocess.run(flags+['-O2','-fsanitize=undefined',str(test),'-ldl','-o',str(work/'test')],check=True)
  subprocess.run([str(work/'test')],check=True,capture_output=True)
  subprocess.run(['cc','-std=gnu11','-Wall','-Wextra','-Werror','-fsyntax-only',str(native/'mod_activation.c')],check=True)
 sources={name:sha((ROOT/name).read_bytes()) for name in [
  'native/linux/current_handler_metadata_probe.c','native/linux/current_handler_metadata_fixture.c',
  'native/linux/current_handler_metadata_fixture.h','native/linux/mod_activation.c',
  'native/linux/current_handler_interruption.c','native/linux/current_handler_interruption.h',
  'native/linux/context_stream.c','native/linux/context_stream.h','native/linux/stream_observation.h',
  'tools/linux/current_handler_caller_audit.py','tools/linux/current_handler_field_guard_audit.py']}
 report=dict(schema='c6-linux-current-handler-caller-source-v1',native_execution=False,
  preparation_admission=False,original_calls_in_helper_or_guard=False,
  enclosing_original_calls=['Exports.Context.GetGame facade acquisition','Context.Save observational controls','created facade Release'],
  methods=[dict(address=hex(a),bytes=n,sha256=h) for a,n,h in METHODS],sources=sources,
  checks=dict(strict_c=True,gcc_analyzer=True,ubsan_pre_original_refusals=6),
  pending=['independent acquisition/Save lifetime review','loader/lifecycle mode and source projection',
   'exact source/mapping/profile/dependency gates before any original run','two outer Save controls and actual terminal cleanup'])
 args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')
HARNESS=r"""
#include <assert.h>
#include <sys/wait.h>
int c6_linux_context_save(void *m,void *c,const char *p){(void)m;(void)c;(void)p;abort();}
int c6_linux_current_handler_interruption_capture(const C6CurrentHandlerInterruptionAdmission *a){(void)a;abort();}
int c6_linux_current_handler_metadata_fields_begin(void *m,void *c,const void *g,uintptr_t b,long t){(void)m;(void)c;(void)g;(void)b;(void)t;abort();}
int c6_linux_current_handler_metadata_quiescent(void *m,void *c,const void *g){(void)m;(void)c;(void)g;abort();}
int c6_linux_current_handler_metadata_fields_end(void *m,void *c,const void *g){(void)m;(void)c;(void)g;abort();}
int main(void){
 const char *names[]={"OUTPUT","REPEAT_OUTPUT","CONTROL_OUTPUT","SNAPSHOT_BEFORE","SNAPSHOT_CONTROL","SNAPSHOT_AFTER","SNAPSHOT_REPEAT"};
 for(unsigned test=0;test<6;test++){
  pid_t pid=fork();assert(pid>=0);
  if(!pid){
   char key[96],value[32];
   for(unsigned i=0;i<7;i++){snprintf(key,sizeof key,"C6_CURRENT_HANDLER_METADATA_%s",names[i]);snprintf(value,sizeof value,"/not-created-%u",i);assert(!setenv(key,value,1));}
   assert(!setenv("C6_CURRENT_HANDLER_METADATA_PROFILE_SHA256","0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",1));
   void *ctx=NULL;
   switch(test){
    case 0:assert(!unsetenv("C6_CURRENT_HANDLER_METADATA_OUTPUT"));break;
    case 1:assert(!setenv("C6_CURRENT_HANDLER_METADATA_REPEAT_OUTPUT","/not-created-0",1));break;
    case 2:assert(!unsetenv("C6_CURRENT_HANDLER_METADATA_PROFILE_SHA256"));break;
    case 3:assert(!setenv("C6_CURRENT_HANDLER_METADATA_PROFILE_SHA256","g123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",1));break;
    case 4:ctx=(void *)1;break;
    case 5:ctx=(void *)(UINTPTR_MAX-8);break;
   }
   c6_linux_current_handler_metadata_probe((void *)1,ctx);_exit(1);
  }
  int status;assert(waitpid(pid,&status,0)==pid);assert(WIFEXITED(status)&&WEXITSTATUS(status)==240);
 }
 return 0;
}
"""
if __name__=='__main__':main()
