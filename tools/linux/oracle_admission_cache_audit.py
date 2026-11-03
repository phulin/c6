#!/usr/bin/env python3
"""Offline source equivalence checks; does not load or execute game code."""
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASELINE = 'e7d4c394c04be70ff6526e6442890d03d3eff84f'


def original(path):
    return subprocess.check_output(['git', 'show', f'{BASELINE}:{path}'], cwd=ROOT, text=True)


def main():
    path = 'native/linux/game_load.c'
    before, after = original(path), (ROOT / path).read_text()
    # Exact retained native predicate expressions, short-circuit order, facade
    # ownership and fatal exits, allowing only the binding-struct qualification.
    old_guard = before.split('    void *controller=*active,*bound=NULL;', 1)[1].split('\n}\n', 1)[0]
    new_guard = after.split('    void *controller=*b->active,*bound=NULL;', 1)[1].split('\n}\n', 1)[0]
    assert old_guard == new_guard.replace('b->', ''), 'mutable guard/call order differs'
    binding = r'MOVEMENT_(?:HOST|CORE)\("[^"]+",0x[0-9a-f]+\)'
    assert re.findall(binding, before) == re.findall(binding, after), 'binding identities/order differ'
    assert len(re.findall(binding, after)) == 12
    marker = 'static int observe_loaded_state('
    domain_branch='''        if(!result && getenv("C6_DOMAIN_METADATA_OUTPUT")) {
            int (*metadata)(void *,void *)=dlsym(RTLD_DEFAULT,"c6_linux_action_prepare_domain_metadata");
            if(!metadata){
                fprintf(stderr,"C6_LINUX_DOMAIN_METADATA_LIFECYCLE fatal missing_lifecycle process_discarded 1\\n");
                fflush(stderr);_exit(184);
            }
            result=metadata(game,context);
        } else '''
    # Remove only the exact new mutually exclusive profile dispatch.
    assert after.count(domain_branch)==1
    projected_after=after.replace(domain_branch,'        ',1)
    assert before.split(marker, 1)[1] == projected_after.split(marker, 1)[1], 'other game-load behavior differs'
    owner = before.split('    Dl_info info={0};', 1)[1].split('    uintptr_t base=', 1)[0]
    assert owner in after, 'per-call original owner validation removed'
    assert 'b->vptr!=*(void **)context||b->base!=info.dli_fbase' in after
    from domain_metadata_fixture_audit import project_lifecycle
    lifecycle = project_lifecycle((ROOT / 'native/linux/action_lifecycle.c').read_text())
    old_lifecycle = original('native/linux/action_lifecycle.c')
    start = '            int (*quiescent)(void *,void *)='
    end = '        } else {\n            result=c6_linux_action_execution'
    assert old_lifecycle.split(start, 1)[0] == lifecycle.split(start, 1)[0]
    assert old_lifecycle.split(end, 1)[1] == lifecycle.split(end, 1)[1], 'other lifecycle behavior differs'
    # Explicit synchronous lifetime order; end must precede native teardown.
    ordered = [
        'initialize_cache(controller);',
        'if(admission_begin(module,context))',
        'if(quiescent(module,context))movement_fatal("post_cache_not_quiescent");',
        'result=probe(module,context);',
        'if(quiescent(module,context))movement_fatal("post_probe_not_quiescent");',
        'movement_fatal("post_probe_identity_or_state");',
        'if(admission_end(module,context))',
        'uninitialize(controller);',
    ]
    positions = [lifecycle.index(token) for token in ordered]
    assert positions == sorted(positions), 'admission lifetime crosses teardown'
    assert 'movement_bindings=(MovementBindings){0};' in after
    print(json.dumps({'baseline_commit': BASELINE, 'bindings': 12,
                      'original_mutable_guard_and_call_order_unchanged': True,
                      'other_game_load_behavior_unchanged': True,
                      'invalidation_before_uninitialize': True,
                      'native_execution_performed': False}, indent=2))


if __name__ == '__main__':
    main()
