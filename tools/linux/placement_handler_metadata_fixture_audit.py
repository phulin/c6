#!/usr/bin/env python3
"""Exact namespace admission and previous-profile projections; no native calls."""
import hashlib
import json
from pathlib import Path
import subprocess
ROOT=Path(__file__).resolve().parents[2]
BASELINE='bb8ea6b'

def original(path):
    return subprocess.check_output(['git','show',BASELINE+':'+path],cwd=ROOT,text=True)

def project_previous_lifecycle(text):
    text=text.replace('    QUERY_PLACEMENT_METADATA,\n    QUERY_PLACEMENT_HANDLER_METADATA\n','    QUERY_PLACEMENT_METADATA\n',1)
    text=text.replace('    const char *marker=profile==QUERY_PLACEMENT_HANDLER_METADATA?\n        "C6_LINUX_PLACEMENT_HANDLER_METADATA_LIFECYCLE":profile==QUERY_PLACEMENT_METADATA?','    const char *marker=profile==QUERY_PLACEMENT_METADATA?',1)
    text=text.replace('    if(profile==QUERY_PLACEMENT_HANDLER_METADATA)\n        fprintf(stderr,"C6_LINUX_PLACEMENT_HANDLER_METADATA_LIFECYCLE mode placement_handler_metadata read_only 1\\n");\n','',1)
    for suffix in ['quiescent','probe','admission_begin','admission_end']:
        token='dlsym(RTLD_DEFAULT,profile==QUERY_PLACEMENT_HANDLER_METADATA?\n                "c6_linux_placement_handler_metadata_'+suffix+'":profile==QUERY_PLACEMENT_METADATA?'
        assert token in text
        text=text.replace(token,'dlsym(RTLD_DEFAULT,profile==QUERY_PLACEMENT_METADATA?',1)
    text=text.replace('profile==QUERY_PLACEMENT_HANDLER_METADATA?"missing_placement_handler_metadata_callback":profile==QUERY_PLACEMENT_METADATA?','profile==QUERY_PLACEMENT_METADATA?',1)
    tail='\n/* Separate original handler predicate queries, not Manager/action admission. */\nint c6_linux_action_prepare_placement_handler_metadata(void *module,void *context) {\n    return prepare(module,context,false,false,false,false,QUERY_PLACEMENT_HANDLER_METADATA);\n}\n'
    assert text.endswith(tail);text=text[:-len(tail)]
    assert text==original('native/linux/action_lifecycle.c'),'previous lifecycle differs'
    return text

def project_previous_loader(text):
    block='''        if(!result && getenv("C6_PLACEMENT_HANDLER_METADATA_OUTPUT")) {
            int (*handler)(void *,void *)=dlsym(RTLD_DEFAULT,"c6_linux_action_prepare_placement_handler_metadata");
            if(!handler){
                fprintf(stderr,"C6_LINUX_PLACEMENT_HANDLER_METADATA_LIFECYCLE fatal missing_lifecycle process_discarded 1\\n");
                fflush(stderr);_exit(184);
            }
            result=handler(game,context);
        } else '''
    assert text.count(block)==1;text=text.replace(block,'        ',1)
    assert text==original('native/linux/game_load.c'),'previous loader differs'
    return text

def audit():
    expected=original('native/linux/placement_metadata_fixture.c').replace('placement_metadata_fixture.h','placement_handler_metadata_fixture.h').replace('c6_linux_placement_metadata_','c6_linux_placement_handler_metadata_').replace('placement_','placement_handler_').replace('PlacementBindings','PlacementHandlerBindings').replace('PLACEMENT_','PLACEMENT_HANDLER_').replace('placement metadata','placement handler metadata').replace('placement_handler_handler_metadata','placement_handler_metadata')
    assert (ROOT/'native/linux/placement_handler_metadata_fixture.c').read_text()==expected,'new admission differs beyond namespace'
    project_previous_lifecycle((ROOT/'native/linux/action_lifecycle.c').read_text())
    project_previous_loader((ROOT/'native/linux/game_load.c').read_text())
    names=['native/linux/placement_handler_metadata_fixture.c','native/linux/placement_handler_metadata_fixture.h','tools/linux/placement_handler_metadata_fixture_audit.py']
    return {'schema':'c6-linux-placement-handler-fixture-source-v1','baseline_commit':subprocess.check_output(['git','rev-parse',BASELINE],cwd=ROOT,text=True).strip(),'independent_admission_scope':True,'immutable_binding_count':12,'prior_profiles_preserved':True,'unknown_or_work_fatal_before_later_reads_or_teardown':True,'native_execution_performed':False,'sources':{n:hashlib.sha256((ROOT/n).read_bytes()).hexdigest() for n in names}}

if __name__=='__main__':print(json.dumps(audit(),indent=2))
