#!/usr/bin/env python3
"""Source equivalence for a separately admitted domain query fixture; no native calls."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
BASELINE = '21b5993'


def project_lifecycle(text):
    """Exact, bounded removal of the new profile dispatch; old source must match."""
    import re
    baseline = subprocess.check_output(['git', 'show', BASELINE + ':native/linux/action_lifecycle.c'], cwd=ROOT, text=True)
    old_fatal = baseline[baseline.index('static _Noreturn void movement_fatal'):baseline.index('static int prepare(')]
    start = text.index('typedef enum {')
    end = text.index('static int prepare(')
    expected_prefix = subprocess.check_output(['git', 'show', '16ce613:native/linux/action_lifecycle.c'], cwd=ROOT, text=True)
    assert text[start:end] == expected_prefix[expected_prefix.index('typedef enum {'):expected_prefix.index('static int prepare(')], 'new fatal/profile dispatch changed'
    text = text[:start] + old_fatal + text[end:]
    text = text.replace('QueryProfile profile) {\n    bool movement_cost=profile!=QUERY_NONE;\n    if(profile==QUERY_DOMAIN_METADATA)\n        fprintf(stderr,"C6_LINUX_DOMAIN_METADATA_LIFECYCLE mode domain_metadata read_only 1\\n");', 'bool movement_cost) {', 1)
    text = text.replace('preparation_failure(profile,', 'preparation_failure(movement_cost,').replace('query_fatal(profile,', 'movement_fatal(')
    for suffix in ['quiescent', 'probe', 'admission_begin', 'admission_end']:
        needle = 'profile==QUERY_DOMAIN_METADATA?\n                "c6_linux_domain_metadata_' + suffix + '":"c6_linux_movement_cost_' + suffix + '"'
        assert needle in text, 'missing exact domain callback selection'
        text = text.replace(needle, '"c6_linux_movement_cost_' + suffix + '"', 1)
    text = text.replace('movement_fatal(\n                profile==QUERY_DOMAIN_METADATA?"missing_domain_metadata_callback":"missing_movement_callback");', 'movement_fatal("missing_movement_callback");', 1)
    text = text.replace(',QUERY_NONE);', ',false);').replace(',QUERY_MOVEMENT_COST);', ',true);')
    marker = '\n/* Separate read-only metadata profile; independent callback/admission scope. */'
    assert text[text.index(marker):] == '\n/* Separate read-only metadata profile; independent callback/admission scope. */\nint c6_linux_action_prepare_domain_metadata(void *module,void *context) {\n    return prepare(module,context,false,false,false,false,QUERY_DOMAIN_METADATA);\n}\n'
    text = text[:text.index(marker)]
    assert text == baseline, 'legacy lifecycle source differs after exact new-profile projection'
    return text


def audit():
    source = subprocess.check_output(['git', 'show', BASELINE + ':native/linux/game_load.c'], cwd=ROOT, text=True)
    expected = source[source.index('/* A potentially active/unknown'):source.index('/* Matching target DWARF')]
    expected = expected.replace('c6_linux_movement_cost_', 'c6_linux_domain_metadata_').replace('movement_', 'domain_').replace('MovementBindings', 'DomainBindings').replace('MOVEMENT_', 'DOMAIN_').replace('movement probe', 'domain metadata probe').replace('cost call', 'metadata query')
    expected = expected.replace('    /* Only the acquired interface wrapper is owned.', '    if(work)domain_not_quiescent("game_work_detected");\n    /* Only the acquired interface wrapper is owned.', 1).replace('b->pending(context)||work)', 'b->pending(context))', 1)
    actual = (ROOT / 'native/linux/domain_metadata_fixture.c').read_text()
    assert actual[actual.index('/* A potentially active/unknown'):] == expected, 'domain fixture differs beyond exact scope names'
    project_lifecycle((ROOT / 'native/linux/action_lifecycle.c').read_text())
    return {'schema': 'c6-domain-metadata-fixture-source-v1', 'baseline_commit': subprocess.check_output(['git', 'rev-parse', BASELINE], cwd=ROOT, text=True).strip(), 'immutable_binding_count': 12, 'independent_admission_scope': True, 'original_idle_path_and_facade_call_order_unchanged': True, 'domain_work_true_fatal_before_release_or_post_reads': True, 'fatal_unknown_or_live': True, 'native_execution_performed': False, 'sources': {name: hashlib.sha256((ROOT / name).read_bytes()).hexdigest() for name in ['native/linux/domain_metadata_fixture.c', 'native/linux/domain_metadata_fixture.h', 'tools/linux/domain_metadata_fixture_audit.py']}}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    text = json.dumps(audit(), indent=2) + '\n'
    if args.output:
        args.output.write_text(text)
    print(text, end='')


if __name__ == '__main__':
    main()
