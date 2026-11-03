#!/usr/bin/env python3
"""Compile/run lifecycle mocks only; never load or execute the game module.

The complete generated C source is retained here. The baseline commit retains
legacy source preimages; each run reports compiler and generated build hashes.
"""
import hashlib
import pathlib
import re
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
BASELINE = '36a737184ef497e78382f12d852a7ef690100754'
source = (ROOT / 'native/linux/action_lifecycle.c').read_text()
old = subprocess.check_output(
    ['git', 'show', f'{BASELINE}:native/linux/action_lifecycle.c'], cwd=ROOT, text=True)
# Remove only the new selector/API arms and restore the prior observational
# selector/comment. All old branches, including trace_end(unsigned), must match.
projection = source.replace('    QUERY_FULL_MANAGER_FIXTURE,\n    QUERY_UNIT_MOVEMENT_QUERY\n', '    QUERY_FULL_MANAGER_FIXTURE\n')
projection = projection.replace('profile==QUERY_UNIT_MOVEMENT_QUERY?\n        "C6_LINUX_UNIT_MOVEMENT_QUERY_LIFECYCLE":', '')
projection = projection.replace('    if(profile==QUERY_UNIT_MOVEMENT_QUERY)\n        fprintf(stderr,"C6_LINUX_UNIT_MOVEMENT_QUERY_LIFECYCLE mode unit_movement_queries action_requested 0\\n");\n', '')
projection = projection.replace('/* These profiles observe trace counters without a zero/equality\n     * expectation; structural and I/O failures still propagate. */', '/* Only this effectful predicate profile observes trace counters without a\n     * zero/equality expectation; structural and I/O failures still propagate. */')
projection = projection.replace('(profile==QUERY_FULL_MANAGER_FIXTURE||profile==QUERY_UNIT_MOVEMENT_QUERY)', 'profile==QUERY_FULL_MANAGER_FIXTURE')
for callback in ('caller_idle', 'probe', 'admission_begin', 'admission_end'):
    projection = projection.replace(f'profile==QUERY_UNIT_MOVEMENT_QUERY?\n                "c6_linux_unit_movement_query_profile_{callback}":', '')
projection = projection.replace('profile==QUERY_UNIT_MOVEMENT_QUERY?"missing_unit_movement_query_callback":', '')
projection = projection[:projection.index('\n/* Movement queries retain their own caller scope.')]
assert projection == old, 'legacy lifecycle source projection changed'
header = (ROOT / 'native/linux/action_lifecycle.h').read_text()
old_header = subprocess.check_output(['git', 'show', f'{BASELINE}:native/linux/action_lifecycle.h'], cwd=ROOT, text=True)
assert re.sub(r'/\* Independent movement query caller scope.*?c6_linux_action_prepare_unit_movement_query\(void \*module,void \*borrowed_context\);\n', '', header, flags=re.S) == old_header

HARNESS = r'''
#include <assert.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include "action_lifecycle.h"
#include "unit_movement_query_profile_guard.h"
static int controller_token, context_token, module_token;
static void *active=&controller_token, *bound=&context_token;
static bool initialized;
static int scenario, idle_count;
static const char *prefix;
static void event(const char *s){puts(s);fflush(stdout);}
static void args(void *m,void *c){assert(m==&module_token&&c==&context_token);}
static bool is_initialized(void *c){assert(c==&controller_token);return initialized;}
static bool executing(void *c){assert(c==&controller_token);return false;}
static void context(void *out,const void *c){assert(c==&controller_token);*(void **)out=bound;if(idle_count==2)event("identity");}
static void initialize(void *c){assert(c==&controller_token);initialized=true;event("initialize");}
static void cache(void *c){assert(c==&controller_token);event("cache");}
static void uninitialize(void *c){assert(c==&controller_token);event("uninitialize");initialized=false;bound=NULL;}
static int trace_begin(void *m){assert(m==&module_token);event("trace_begin");return 0;}
/* Keep the original unsigned ABI exercised for every legacy selector. */
static int trace_end(unsigned expected){assert(expected==0);event("trace_legacy");return scenario==207?207:scenario==208?208:0;}
static int trace_observe(void){event("trace_observe");return scenario==207?207:scenario==208?208:0;}
int c6_linux_unit_movement_query_profile_admission_begin(void *m,void *c){args(m,c);event("begin");return scenario==2?1:0;}
int c6_linux_unit_movement_query_profile_caller_idle(void *m,void *c){args(m,c);idle_count++;event("idle");if(scenario==240&&idle_count==2)_exit(240);return scenario==3||(scenario==7&&idle_count==2)?1:0;}
int c6_linux_unit_movement_query_profile_probe(void *m,void *c){args(m,c);event("probe");event("facade_acquire");event("facade_release");if(scenario==5)active=NULL;if(scenario==6)bound=NULL;return scenario==243?243:0;}
int c6_linux_unit_movement_query_profile_admission_end(void *m,void *c){args(m,c);event("end");return scenario==4?1:0;}
/* These assignments compile-check against the actual frozen guard header. */
static int (*const typed_callbacks[])(void *,void *)={
 c6_linux_unit_movement_query_profile_admission_begin,
 c6_linux_unit_movement_query_profile_caller_idle,
 c6_linux_unit_movement_query_profile_probe,
 c6_linux_unit_movement_query_profile_admission_end};
void *mock_dlsym(void *handle,const char *name){
 (void)handle;
 #define MAP(n,f) if(!strcmp(name,n))return (void *)(f)
 MAP("_ZN18GameCoreController21ms_pkActiveControllerE",&active);
 MAP("_ZN18GameCoreController13IsInitializedEv",is_initialized);
 MAP("_ZNK18GameCoreController10GetContextEv",context);
 MAP("_ZN18GameCoreController10InitializeEv",initialize);
 MAP("_ZN18GameCoreController15InitializeCacheEv",cache);
 MAP("_ZN18GameCoreController12UninitializeEv",uninitialize);
 MAP("_ZN18GameCoreController11IsExecutingEv",executing);
 MAP("c6_linux_action_trace_begin",trace_begin);
 MAP("c6_linux_action_trace_end",trace_end);
 if(!strcmp(name,"c6_linux_action_trace_end_observational"))return scenario==8?NULL:(void *)trace_observe;
 const char *suffixes[]={"admission_begin","caller_idle","probe","admission_end"};
 for(unsigned i=0;i<4;i++){
  char symbol[160];snprintf(symbol,sizeof(symbol),"%s%s",prefix,suffixes[i]);
  if(!strcmp(name,symbol))return scenario==1&&i==2?NULL:(void *)typed_callbacks[i];
 }
 /* Other legacy metadata profiles use quiescent instead of caller_idle. */
 if(strstr(name,"_quiescent"))return (void *)typed_callbacks[1];
 if(strstr(name,"_fixture_probe"))return (void *)typed_callbacks[2];
 fprintf(stderr,"unexpected lookup: %s\n",name);abort();
}
int c6_linux_action_execution(void *m,void *c){args(m,c);return 0;}
int c6_linux_action_observe(void *m,void *c){args(m,c);return 0;}
int c6_linux_action_dispatch(void *m,void *c,bool x){args(m,c);assert(!x);return 0;}
int c6_linux_action_registration(void *m,void *c){args(m,c);return 0;}
int main(int argc,char **argv){
 assert(argc==3);scenario=atoi(argv[2]);int mode=atoi(argv[1]);
 int (*calls[])(void *,void *)={c6_linux_action_prepare_observe,c6_linux_action_prepare_movement_cost,c6_linux_action_prepare_domain_metadata,c6_linux_action_prepare_placement_metadata,c6_linux_action_prepare_placement_handler_metadata,c6_linux_action_prepare_unit_operation_manager_metadata,c6_linux_action_prepare_current_handler_metadata,c6_linux_action_prepare_full_manager_fixture,c6_linux_action_prepare_unit_movement_query};
 const char *prefixes[]={"","c6_linux_movement_cost_","c6_linux_domain_metadata_","c6_linux_placement_metadata_","c6_linux_placement_handler_metadata_","c6_linux_unit_operation_manager_metadata_","c6_linux_current_handler_metadata_","c6_linux_full_manager_profile_","c6_linux_unit_movement_query_profile_"};
 assert(mode>=0&&mode<9);prefix=prefixes[mode];
 int result=calls[mode](&module_token,&context_token);printf("result %d\n",result);return result;
}
'''
with tempfile.TemporaryDirectory(prefix='c6-movement-lifecycle-') as tmp:
    tmp = pathlib.Path(tmp)
    harness = tmp / 'harness.c'
    harness.write_text(HARNESS)
    exe = tmp / 'lifecycle-test'
    command = ['cc', '-std=c11', '-Wall', '-Wextra', '-Werror', '-fsanitize=undefined',
               '-Ddlsym=mock_dlsym', '-I', str(ROOT / 'native/linux'),
               str(ROOT / 'native/linux/action_lifecycle.c'), str(harness), '-o', str(exe)]
    subprocess.run(command, check=True)
    def run(mode, scenario):
        result = subprocess.run([str(exe), str(mode), str(scenario)], capture_output=True, text=True)
        return result.returncode, result.stdout.splitlines(), result.stderr
    sequence = ['trace_begin', 'initialize', 'cache', 'begin', 'idle', 'probe',
                'facade_acquire', 'facade_release', 'idle', 'identity', 'end', 'uninitialize', 'identity', 'trace_observe']
    for status in (0, 243, 207, 208):
        code, events, stderr = run(8, status)
        assert code == status and events == sequence + [f'result {status}'], (code, events, stderr)
        assert 'mode unit_movement_queries action_requested 0' in stderr
    for scenario, last, code in ((1, 'cache', 184), (2, 'begin', 184), (3, 'idle', 184),
                                  (4, 'end', 184), (5, 'idle', 184), (6, 'identity', 184),
                                  (7, 'idle', 184),
                                  (240, 'idle', 240), (8, None, 184)):
        actual, events, stderr = run(8, scenario)
        assert actual == code and (events[-1] if events else None) == last, (scenario, actual, events, stderr)
        assert 'uninitialize' not in events and 'trace_observe' not in events
    for mode in range(9):
        for status in (0, 207, 208):
            code, events, stderr = run(mode, status)
            assert code == status, (mode, status, code, stderr)
            expected = 'trace_observe' if mode in (7, 8) else 'trace_legacy'
            assert events[-2] == expected and events.count(expected) == 1, (mode, events)
    for mode in range(7):
        code, events, stderr = run(mode, 8)
        assert code == 0 and events[-2] == 'trace_legacy', (mode, code, events, stderr)
    print('PASS: legacy source/header projections; typed ABI/order; ordinary243; fatal240; ownership/admission failures; all 9 trace selectors and old207/208; mock execution only')
    print('compiler:', subprocess.check_output(['cc', '--version'], text=True).splitlines()[0])
    for path in (ROOT / 'native/linux/action_lifecycle.c', ROOT / 'native/linux/action_lifecycle.h', ROOT / 'native/linux/unit_movement_query_profile_guard.h', harness, exe):
        print(path.name, hashlib.sha256(path.read_bytes()).hexdigest())
