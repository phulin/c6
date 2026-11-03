#!/usr/bin/env python3
"""Independent actual-wrapper/caller integration with mock originals only."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
PATHS = ['native/linux/full_manager_profile.c', 'native/linux/full_manager_fixture.c',
         'native/linux/full_manager_fixture.h', 'native/linux/full_manager_profile_guard.c',
         'tools/linux/full_manager_fixture_review.c', 'tools/linux/full_manager_wrapper_review.c',
         'tools/linux/full_manager_wrapper_review.py']


def hashes():
    return {p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in PATHS}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    before = hashes()
    rows = []
    with tempfile.TemporaryDirectory(prefix='c6-wrapper-independent-') as temporary:
        work = Path(temporary)
        # Namespace the old mock callback without changing the shared API field
        # name. The wrapper and run_bound themselves remain the actual sources.
        mock = (ROOT / 'tools/linux/full_manager_fixture_review.c').read_text()
        mock = mock.replace('#include "../../native/linux/full_manager_fixture.c"',
                            '#include "' + str(ROOT / 'native/linux/full_manager_fixture.c') + '"')
        mock = mock.replace('static int observe(', 'static int caller_leaf_observe(')
        mock = mock.replace('.observe=observe', '.observe=caller_leaf_observe')
        include = work / 'mock.c'
        include.write_text(mock)
        flags = ['cc', '-std=gnu11', '-Wall', '-Wextra', '-Werror', '-Wconversion', '-Wshadow',
                 '-DCALLER_REVIEW_SOURCE="' + str(include) + '"']
        source = str(ROOT / 'tools/linux/full_manager_wrapper_review.c')
        subprocess.run(flags + ['-fanalyzer', '-c', source, '-o', str(work / 'test.o')], check=True)
        executable = work / 'test'
        subprocess.run(flags + ['-O2', '-fsanitize=undefined', '-fno-sanitize-recover=undefined',
                                source, '-ldl', '-o', str(executable)], check=True)
        executable_sha = hashlib.sha256(executable.read_bytes()).hexdigest()
        reasons = {2: 'actor_not_loaded_member', 3: 'actor_identity_unknown',
                   7: 'actor_not_loaded_member', 8: 'actor_identity_unknown'}
        labels = ['false_no_saves', 'save_payload_effect_and_error', 'save_removes_actor',
                  'save_changes_native_generation', 'save_replaces_manager',
                  'all_metadata_errors_and_removed_actor', 'controls_io_and_removed_actor',
                  'native_find_disagrees_with_raw_actor', 'native_id_disagrees_with_raw_actor']
        for index, label in enumerate(labels):
            case = work / str(index)
            case.mkdir()
            result = subprocess.run([str(executable), str(index), str(case)], capture_output=True, text=True)
            expected = 240 if index in reasons else 0
            if result.returncode != expected:
                raise ValueError(label + ': ' + repr(result))
            if index in reasons:
                if result.stderr != 'C6_FULL_MANAGER_FIXTURE fatal ' + reasons[index] + ' no_poststate_or_cleanup\n':
                    raise ValueError(label + ': wrong refusal ' + result.stderr)
            else:
                if result.stdout != 'integrated wrapper case %u passed\n' % index or result.stderr:
                    raise ValueError(label + ': missing assertions ' + repr(result))
                if index != 6:
                    controls = [json.loads(line) for line in (case / 'CONTROL_OUTPUT').read_text().splitlines()]
                    evidence = next(row for row in controls if row['kind'] == 'evidence_status')
                    if evidence['metadata_status'] != ([245] * 4 if index == 5 else [0] * 4):
                        raise ValueError('metadata status lost')
                    observations = [r for r in controls if r['kind'] == 'observation']
                    if [r['phase'] for r in observations] != list(range(1, 10)):
                        raise ValueError('actual caller observation order changed')
                    if observations[-1]['known_rng_raw'][:2] != '44' or observations[-1]['known_rng_raw'][64:66] != '84':
                        raise ValueError('callback effects lost')
            rows.append(dict(case=label, exit=expected, reason=reasons.get(index), passed=True))
    if hashes() != before:
        raise ValueError('source changed during review')
    args.output.write_text(json.dumps(dict(
        schema='c6-linux-full-manager-wrapper-independent-review-v1',
        reviewed_checkpoints=['13fe520', 'de502ec'], source_sha256=before,
        source_stable=True, mock_executable_sha256=executable_sha, cases=rows,
        actual_wrapper_and_run_bound=True, mock_originals_only=True,
        real_public_binding_resolution=False, actual_guards_or_observer_admitted=False,
        original_execution=False, native_gameplay_parity=False,
        checks=['strict C', 'GCC analyzer', 'UBSan without recovery',
                'one predicate and balanced ctor/dtor/facade on normal returns',
                'non-gating optional metadata and controls failures',
                'preserved callback RNG bytes and preSave effects',
                'fresh native actor and Manager resolution after preSave',
                'actor protected after query without stale dereference']), indent=2, sort_keys=True) + '\n')
    print('9 independent wrapper/caller integration cases passed; no original execution')


if __name__ == '__main__':
    main()
