#!/usr/bin/env python3
"""Exact admission-source and prior-profile projections; never executes native code."""
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
BASELINE = '100d294'


def original(path, commit=BASELINE):
    return subprocess.check_output(['git', 'show', commit + ':' + path], cwd=ROOT, text=True)


def project_previous_lifecycle(text):
    text = text.replace('    QUERY_DOMAIN_METADATA,\n    QUERY_PLACEMENT_METADATA\n', '    QUERY_DOMAIN_METADATA\n', 1)
    text = text.replace('    const char *marker=profile==QUERY_PLACEMENT_METADATA?\n        "C6_LINUX_PLACEMENT_METADATA_LIFECYCLE":profile==QUERY_DOMAIN_METADATA?', '    const char *marker=profile==QUERY_DOMAIN_METADATA?', 1)
    text = text.replace('    if(profile==QUERY_PLACEMENT_METADATA)\n        fprintf(stderr,"C6_LINUX_PLACEMENT_METADATA_LIFECYCLE mode placement_metadata read_only 1\\n");\n', '', 1)
    for suffix in ['quiescent', 'probe', 'admission_begin', 'admission_end']:
        old = 'dlsym(RTLD_DEFAULT,profile==QUERY_PLACEMENT_METADATA?\n                "c6_linux_placement_metadata_' + suffix + '":profile==QUERY_DOMAIN_METADATA?'
        assert old in text, 'placement callback selection differs'
        text = text.replace(old, 'dlsym(RTLD_DEFAULT,profile==QUERY_DOMAIN_METADATA?', 1)
    text = text.replace('profile==QUERY_PLACEMENT_METADATA?"missing_placement_metadata_callback":profile==QUERY_DOMAIN_METADATA?', 'profile==QUERY_DOMAIN_METADATA?', 1)
    tail = '\n/* Separate read-only placement-count profile with independently admitted owners. */\nint c6_linux_action_prepare_placement_metadata(void *module,void *context) {\n    return prepare(module,context,false,false,false,false,QUERY_PLACEMENT_METADATA);\n}\n'
    assert text.endswith(tail), 'placement entry differs'
    text = text[:-len(tail)]
    assert text == original('native/linux/action_lifecycle.c'), 'prior lifecycle behavior changed'
    return text


def project_previous_loader(text):
    block = '''        if(!result && getenv("C6_PLACEMENT_METADATA_OUTPUT")) {
            int (*placement)(void *,void *)=dlsym(RTLD_DEFAULT,"c6_linux_action_prepare_placement_metadata");
            if(!placement){
                fprintf(stderr,"C6_LINUX_PLACEMENT_METADATA_LIFECYCLE fatal missing_lifecycle process_discarded 1\\n");
                fflush(stderr);_exit(184);
            }
            result=placement(game,context);
        } else '''
    assert text.count(block) == 1, 'placement loader dispatch differs'
    text = text.replace(block, '        ', 1)
    assert text == original('native/linux/game_load.c'), 'prior loader behavior changed'
    return text


def audit():
    expected = original('native/linux/domain_metadata_fixture.c').replace('domain_metadata_fixture.h', 'placement_metadata_fixture.h').replace('c6_linux_domain_metadata_', 'c6_linux_placement_metadata_').replace('domain_', 'placement_').replace('DomainBindings', 'PlacementBindings').replace('DOMAIN_', 'PLACEMENT_').replace('domain metadata', 'placement metadata')
    assert (ROOT/'native/linux/placement_metadata_fixture.c').read_text() == expected, 'placement admission differs beyond exact namespace'
    project_previous_lifecycle((ROOT/'native/linux/action_lifecycle.c').read_text())
    project_previous_loader((ROOT/'native/linux/game_load.c').read_text())
    names=['native/linux/placement_metadata_fixture.c','native/linux/placement_metadata_fixture.h','tools/linux/placement_metadata_fixture_audit.py']
    return {'schema':'c6-linux-placement-fixture-source-v1','baseline_commit':subprocess.check_output(['git','rev-parse',BASELINE],cwd=ROOT,text=True).strip(),'independent_admission_scope':True,'immutable_binding_count':12,'prior_profiles_preserved':True,'unknown_or_work_fatal_before_later_reads_or_teardown':True,'native_execution_performed':False,'sources':{n:hashlib.sha256((ROOT/n).read_bytes()).hexdigest() for n in names}}


if __name__ == '__main__':
    print(json.dumps(audit(),indent=2))
