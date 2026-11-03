#!/usr/bin/env python3
"""Independent raw Manager scope, exact accepted guard and legacy projections."""
import hashlib
import json
from pathlib import Path
import subprocess
ROOT=Path(__file__).resolve().parents[2]
BASELINE='b6b3143'

def original(path):
    return subprocess.check_output(['git','show',BASELINE+':'+path],cwd=ROOT,text=True)

def project_previous_lifecycle(text):
    text=text.replace('    QUERY_PLACEMENT_HANDLER_METADATA,\n    QUERY_UNIT_OPERATION_MANAGER_METADATA\n','    QUERY_PLACEMENT_HANDLER_METADATA\n',1)
    text=text.replace('const char *marker=profile==QUERY_UNIT_OPERATION_MANAGER_METADATA?\n        "C6_LINUX_UNIT_OPERATION_MANAGER_METADATA_LIFECYCLE":profile==QUERY_PLACEMENT_HANDLER_METADATA?', 'const char *marker=profile==QUERY_PLACEMENT_HANDLER_METADATA?',1)
    text=text.replace('    if(profile==QUERY_UNIT_OPERATION_MANAGER_METADATA)\n        fprintf(stderr,"C6_LINUX_UNIT_OPERATION_MANAGER_METADATA_LIFECYCLE mode unit_operation_manager_metadata read_only 1\\n");\n','',1)
    for suffix in ('quiescent','probe','admission_begin','admission_end'):
        callback='fixture_probe' if suffix=='probe' else suffix
        token='dlsym(RTLD_DEFAULT,profile==QUERY_UNIT_OPERATION_MANAGER_METADATA?\n                "c6_linux_unit_operation_manager_metadata_'+callback+'":profile==QUERY_PLACEMENT_HANDLER_METADATA?'
        assert text.count(token)==1
        text=text.replace(token,'dlsym(RTLD_DEFAULT,profile==QUERY_PLACEMENT_HANDLER_METADATA?',1)
    text=text.replace('profile==QUERY_UNIT_OPERATION_MANAGER_METADATA?"missing_unit_operation_manager_metadata_callback":','',1)
    tail='\nint c6_linux_action_prepare_unit_operation_manager_metadata(void *module,void *context) {\n    return prepare(module,context,false,false,false,false,QUERY_UNIT_OPERATION_MANAGER_METADATA);\n}\n'
    assert text.endswith(tail);text=text[:-len(tail)]
    assert text==original('native/linux/action_lifecycle.c'),'previous lifecycle differs'
    return text

def project_previous_loader(text):
    block='''        if(!result && getenv("C6_UNIT_OPERATION_MANAGER_METADATA_OUTPUT")) {
            int (*metadata)(void *,void *)=dlsym(RTLD_DEFAULT,"c6_linux_action_prepare_unit_operation_manager_metadata");
            if(!metadata){
                fprintf(stderr,"C6_LINUX_UNIT_OPERATION_MANAGER_METADATA_LIFECYCLE fatal missing_lifecycle process_discarded 1\\n");
                fflush(stderr);_exit(184);
            }
            result=metadata(game,context);
        } else '''
    assert text.count(block)==1;text=text.replace(block,'        ',1)
    assert text==original('native/linux/game_load.c'),'previous loader differs'
    return text

def audit():
    old=original('native/linux/placement_handler_metadata_fixture.c')
    expected=old.replace('placement_handler','unit_operation_manager').replace('PLACEMENT_HANDLER','UNIT_OPERATION_MANAGER').replace('PlacementHandler','UnitOperationManager').replace('placement handler','unit operation manager')
    expected=expected.replace('#include <unistd.h>','#include <unistd.h>\n#include <stdlib.h>\n#include "context_stream.h"\n#include "unit_operation_manager_metadata.h"')
    new=(ROOT/'native/linux/unit_operation_manager_metadata_fixture.c').read_text()
    assert new.split('/* Caller-side controls')[0]==expected+'\n','idle guard changed beyond namespace'
    project_previous_lifecycle((ROOT/'native/linux/action_lifecycle.c').read_text())
    project_previous_loader((ROOT/'native/linux/game_load.c').read_text())
    names=['native/linux/unit_operation_manager_metadata_fixture.c','native/linux/unit_operation_manager_metadata_fixture.h','tools/linux/unit_operation_manager_metadata_fixture_audit.py']
    return {'schema':'c6-linux-unit-operation-manager-fixture-v1','baseline_commit':subprocess.check_output(['git','rev-parse',BASELINE],cwd=ROOT,text=True).strip(),'prior_profiles_preserved':True,'independent_admission_scope':True,'immutable_binding_count':12,'raw_metadata_passes':2,'internal_core_saves':4,'known_rng_bytes':64,'complete_rng_inventory':False,'full_manager_called':False,'script_callbacks_called':False,'native_execution_performed':False,'sources':{n:hashlib.sha256((ROOT/n).read_bytes()).hexdigest() for n in names}}

if __name__=='__main__':print(json.dumps(audit(),indent=2))
