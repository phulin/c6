#!/usr/bin/env python3
"""Offline checks for an observational full-Manager trace-end policy."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
ROOT=Path(__file__).resolve().parents[2]
BASE='eefe52c'
SOURCE='tools/linux/action_execution_trace.c'
def sha(b):return hashlib.sha256(b).hexdigest()
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);args=p.parse_args()
 old=subprocess.check_output(['git','show',BASE+':'+SOURCE],cwd=ROOT);new=(ROOT/SOURCE).read_bytes()
 assert new.startswith(old) and b'int c6_linux_action_trace_end_observational(void)' in new[len(old):]
 assert b'return failed?208:(q==r&&e==expected_messages&&q>=e&&(expected_messages||q==0))?0:209;' in old
 with tempfile.TemporaryDirectory(prefix='c6-full-manager-trace-') as temp:
  work=Path(temp);test=work/'test.c'
  test.write_text('#include "'+str(ROOT/SOURCE)+'"\n'+HARNESS)
  subprocess.run(['cc','-std=gnu11','-Wall','-Wextra','-Werror','-fanalyzer','-c',str(ROOT/SOURCE),'-o',str(work/'trace.o')],check=True)
  subprocess.run(['cc','-std=gnu11','-Wall','-Wextra','-Werror','-O2','-fsanitize=undefined',str(test),'-ldl','-o',str(work/'test')],check=True)
  subprocess.run([str(work/'test')],check=True,capture_output=True)
 report=dict(schema='c6-linux-full-manager-observational-trace-policy-v1',native_execution=False,legacy_source_sha256=sha(old),legacy_source_prefix_unchanged=True,source_sha256=sha(new),audit_sha256=sha(Path(__file__).read_bytes()),synthetic_cases=6,strict_c=True,gcc_analyzer=True,ubsan=True,legacy_end_abi_unchanged=True,full_manager_only_policy='actual counters retained without zero/equality expectations; no drain',structural_and_io_failures_retained=True,known_hard_stops='unavailable original run; never accepted substitute behavior',all_policies_observed=False)
 args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')
HARNESS=r'''
#include <assert.h>
static FILE *start(unsigned q,unsigned r,unsigned e){FILE *file=tmpfile();assert(file);trace_log=file;atomic_store(&armed,true);atomic_store(&queued,q);atomic_store(&received,r);atomic_store(&executed,e);atomic_store(&reported,7);return file;}
int main(void){
 atomic_store(&armed,false);trace_log=NULL;assert(c6_linux_action_trace_end_observational()==207);
 start(1,0,2);assert(c6_linux_action_trace_end(0)==209);
 start(1,1,1);assert(c6_linux_action_trace_end(1)==0);
 FILE *file=start(2,1,3);int copy=dup(fileno(file));assert(copy>=0);assert(c6_linux_action_trace_end_observational()==0);assert(!atomic_load(&armed)&&!trace_log);FILE *readback=fdopen(copy,"r");assert(readback);rewind(readback);char text[1024]={0};assert(fread(text,1,sizeof text-1,readback)>0);assert(strstr(text,"observe_callback_effects")&&strstr(text,"\"queued\":2")&&strstr(text,"\"received\":1")&&strstr(text,"\"unit_executed\":3")&&strstr(text,"\"reports\":7"));assert(!fclose(readback));
 start(0,0,0);assert(c6_linux_action_trace_end_observational()==0);
 trace_log=fopen("/dev/full","w");assert(trace_log);atomic_store(&armed,true);assert(c6_linux_action_trace_end_observational()==208);return 0;
}
'''
if __name__=='__main__':main()
