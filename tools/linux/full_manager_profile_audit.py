#!/usr/bin/env python3
"""Audit the full-Manager profile wrapper using synthetic state and mock callbacks.

No original module is opened, loaded, executed, or queried. This audit does not
admit a native profile, Lua/VM window, live owner, predicate answer, or gameplay.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
SOURCE = 'native/linux/full_manager_profile.c'
HARNESS = 'tools/linux/full_manager_profile_audit.c'
SOURCES = [SOURCE, HARNESS, 'tools/linux/full_manager_profile_audit.py',
           'native/linux/full_manager_profile_guard.c',
           'native/linux/full_manager_profile_guard.h',
           'native/linux/full_manager_fixture.c', 'native/linux/full_manager_fixture.h',
           'native/linux/current_handler_interruption.c',
           'native/linux/current_handler_interruption.h',
           'native/linux/action_lifecycle.c', 'native/linux/action_lifecycle.h']
SUCCESS = ['normal', 'answer-false', 'metadata-before', 'metadata-control',
           'metadata-after', 'metadata-repeat', 'metadata-all', 'saves',
           'list-prefix', 'numeric-boundaries', 'fixture-error', 'controls-write-error']
UNSUPPORTED = ['actor-unmapped', 'actor-player-id', 'actor-owner', 'actor-generation',
               'actor-vptr', 'actor-parent', 'actor-block-range', 'actor-block-null',
               'actor-block-overflow', 'actor-list-missing', 'actor-list-not-member',
               'actor-link-previous', 'actor-count', 'actor-vector-shape', 'actor-unreadable']
FATAL = {
    'missing-owner': 'numeric_input_missing', 'empty-owner': 'numeric_input_missing',
    'negative-owner': 'numeric_input_invalid', 'invalid-owner': 'numeric_input_invalid',
    'owner-range': 'numeric_input_range', 'id-range': 'numeric_input_range',
    'visible-range': 'numeric_input_range', 'profile-missing': 'profile_missing',
    'profile-short': 'profile_missing', 'profile-invalid': 'profile_invalid',
    'output-missing': 'required_path_missing', 'metadata-missing': 'required_path_missing',
    'duplicate-control': 'duplicate_path', 'duplicate-metadata': 'duplicate_path',
    'duplicate-save': 'duplicate_path', 'partial-saves': 'optional_save_paths_incomplete',
    'controls-existing': 'controls_unavailable', 'nested': 'nested_profile',
    'observation-identity': 'observation_identity', 'rng-unreadable': 'known_rng_unreadable',
}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def source_hashes():
    return {name: sha((ROOT / name).read_bytes()) for name in SOURCES}


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def read_controls(work):
    return [json.loads(line) for line in (work / 'CONTROL_OUTPUT.jsonl').read_text().splitlines()]


def check_observations(mode, work):
    rows = read_controls(work)
    header = rows[0]
    require(header['kind'] == 'header' and header['pure_query_claimed'] is False and
            header['Save_equality_is_gate'] is False and header['complete_rng_inventory'] is False,
            mode + ': evidence/equality scope changed')
    phases = [row for row in rows if row['kind'] == 'observation']
    require([row['phase'] for row in phases] == list(range(1, 10)), mode + ': phase order or predicate missing')
    for row in phases:
        phase = row['phase']
        expected = bytes([phase]) + bytes(31) + bytes([0x80 + phase]) + bytes(31)
        require(row['known_rng_raw'] == expected.hex(), mode + ': known RNG effect was lost')
        require(bool(row['unit_address_token']) == (phase in (4, 5)), mode + ': actor token lifetime changed')
        require(row['predicate_returned'] == (phase >= 5), mode + ': predicate endpoint changed')
        require(row['allowed'] == (None if phase < 5 else mode != 'answer-false'), mode + ': answer was altered')
    statuses = [row for row in rows if row['kind'] == 'metadata_status']
    require([row['phase'] for row in statuses] == [2, 3, 6, 7], mode + ': four metadata boundaries missing')
    expected_status = [0] * 4
    if mode.startswith('metadata-'):
        if mode == 'metadata-all':
            expected_status = [245] * 4
        else:
            expected_status[['metadata-before', 'metadata-control', 'metadata-after', 'metadata-repeat'].index(mode)] = 245
    require([row['status'] for row in statuses] == expected_status and
            all(row['predicate_admission_veto'] is False for row in statuses), mode + ': passive errors not retained separately')
    evidence = next(row for row in rows if row['kind'] == 'evidence_status')
    require(evidence['metadata_status'] == expected_status and evidence['predicate_admission_veto'] is False and
            evidence['io_status'] == 0, mode + ': passive evidence summary changed')
    summary = next(row for row in rows if row['kind'] == 'summary')
    require(summary['predicate_invoked'] is True and summary['predicate_returned'] is True and
            summary['facade_released'] is True and summary['observation_status'] == 0 and
            summary['complete_affected_state_claimed'] is False and
            summary['native_action_requested_by_fixture'] is False,
            mode + ': one-predicate/lifetime/scope summary changed')
    require(summary['status'] == (249 if mode == 'fixture-error' else 0), mode + ': fixture status not propagated')
    suffixes = ['METADATA_BEFORE', 'METADATA_CONTROL', 'METADATA_AFTER', 'METADATA_REPEAT']
    for ordinal, suffix in enumerate(suffixes):
        data = json.loads((work / (suffix + '.jsonl')).read_text())
        require(data == {'mock_snapshot': ordinal, 'status': expected_status[ordinal]}, mode + ': snapshot status/path lost')
    return {'phases': len(phases), 'metadata_status': expected_status,
            'one_mock_predicate': True, 'known_rng_changes_retained': True,
            'protected_actor_after_enter': True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    before = source_hashes()
    fixture = (ROOT / 'native/linux/full_manager_fixture.c').read_text()
    require('if(!r->observation_status)invoke(&b,a,r,&o,out);' in fixture and
            'if(r->observation_status){' in fixture,
            'mock observation veto no longer matches the reviewed fixture; refresh the audit')
    cases = []
    with tempfile.TemporaryDirectory(prefix='c6-full-profile-audit-') as temporary:
        directory = Path(temporary)
        flags = ['cc', '-std=gnu11', '-Wall', '-Wextra', '-Werror', '-Wconversion', '-Wshadow']
        subprocess.run(flags + ['-fanalyzer', '-c', str(ROOT / SOURCE), '-o', str(directory / 'profile.o')], check=True)
        executable = directory / 'test'
        subprocess.run(flags + ['-O2', '-fsanitize=undefined', str(ROOT / HARNESS), '-o', str(executable)], check=True)
        for mode in SUCCESS + UNSUPPORTED + list(FATAL):
            work = directory / mode
            work.mkdir()
            result = subprocess.run([str(executable), mode, str(work)], capture_output=True, text=True)
            if mode in FATAL:
                expected = 'C6_FULL_MANAGER_PROFILE fatal ' + FATAL[mode] + ' no_further_original_calls\n'
                require(result.returncode == 240 and result.stderr == expected,
                        mode + ': targeted refusal failed: ' + repr(result.stderr))
                cases.append({'mode': mode, 'fatal_exit': 240, 'targeted_reason': FATAL[mode]})
                if mode == 'controls-existing':
                    require((work / 'CONTROL_OUTPUT.jsonl').read_bytes() == b'', 'existing controls were overwritten')
                continue
            require(result.returncode == 0, mode + ': synthetic harness failed: ' + result.stderr)
            counts = json.loads(result.stdout)
            if mode in UNSUPPORTED:
                require(counts['status'] == 244 and counts['fixture_calls'] == 0 and
                        counts['predicate_calls'] == 0 and counts['metadata_calls'] == 0,
                        mode + ': unsupported actor reached fixture')
                unsupported = next(row for row in read_controls(work) if row['kind'] == 'unsupported')
                require(unsupported['reason'] == 'raw_actor_expectation_unavailable' and
                        unsupported['predicate_invoked'] is False, mode + ': unsupported reason changed')
                cases.append({'mode': mode, 'status': 244, 'fixture_not_invoked': True})
            else:
                require(counts['fixture_calls'] == 1 and counts['predicate_calls'] == 1 and counts['metadata_calls'] == 4,
                        mode + ': optional evidence suppressed or repeated predicate')
                if mode == 'controls-write-error':
                    require(counts['status'] == 247, 'controls write failure was not propagated')
                    checks = {'controls_io_error_retained': 247, 'one_mock_predicate': True}
                else:
                    checks = check_observations(mode, work)
                cases.append({'mode': mode, 'counts': counts, 'checks': checks})
    require(source_hashes() == before, 'source changed during audit; rerun after owner checkpoint settles')
    report = {
        'schema': 'c6-linux-full-manager-profile-source-v1',
        'native_execution': False, 'original_modules_opened_or_loaded': False,
        'source_sha256': before,
        'checks': {'strict_c': True, 'gcc_analyzer': True, 'ubsan_synthetic_wrapper': True,
                   'cases': len(cases), 'targeted_fatal_refusals': len(FATAL),
                   'raw_actor_identity_refusals': len(UNSUPPORTED),
                   'per_process_main_tid_witness': True, 'fixture_callback_abi_compiled': True,
                   'four_snapshot_paths_and_known_rng_effects_retained': True,
                   'passive_metadata_failure_does_not_veto_requested_predicate': True,
                   'optional_save_arguments_all_or_none': True,
                   'actor_protected_at_predicate_return_without_later_dereference': True,
                   'raw_owner_full_id_block_active_entry_expectation_checked': True,
                   'fixture_status_and_controls_io_errors_preserved': True},
        'cases': cases,
        'limits': ['synthetic objects and mocked identity/owner/fixture/metadata services only',
                   'fresh native Find, original facade/lock, actual guards and original predicate are not executed here',
                   'separate pinned source/caller/guard/observer/loader profile admission remains necessary',
                   'known 64 RNG bytes are partial observations; differing bytes are retained without equality gates',
                   'optional Save arguments are tested; no original Save or Save equality is required',
                   'no native policy boundary, Lua/GC window, affected gameplay state/order or complete RNG acceptance'],
        'raw_actor_pointer_is_untrusted_address_expectation': True,
        'native_fresh_find_executed': False,
        'native_profile_admitted': False, 'full_manager_predicate_parity': False,
        'complete_state_actions_rng_turns': False,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'checks_passed': True, 'cases': len(cases), 'native_execution': False}))


if __name__ == '__main__':
    main()
