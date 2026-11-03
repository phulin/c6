#!/usr/bin/env python3
"""Offline controls for one sealed original Manager observation, not parity."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import tarfile


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def rows(path):
    return [json.loads(line) for line in path.read_text().splitlines()]


def one(records, kind):
    selected = [row for row in records if row.get('kind') == kind]
    require(len(selected) == 1, 'unique ' + kind + ' required')
    return selected[0]


def audit(directory, expected_runtime, expected_closed):
    root = Path(directory)
    require(sha(root/'closed-files.json') == expected_closed, 'closed manifest identity')
    closed = json.loads((root/'closed-files.json').read_text())
    require(sha(root/'runtime.json') == expected_runtime == closed['runtime_sha256'], 'runtime identity')
    for name, identity in closed['files'].items():
        path = root/name
        require(path.is_relative_to(root) and '..' not in Path(name).parts, 'artifact name')
        require(path.stat().st_size == identity['bytes'] and sha(path) == identity['sha256'], 'artifact identity: ' + name)
    prepared = json.loads((root/'prepared.json').read_text())
    runtime = json.loads((root/'runtime.json').read_text())
    cfg = prepared['configuration']
    require(runtime['configuration'] == cfg and runtime['configuration_sha256'] == prepared['configuration_sha256'], 'prepared/runtime binding')
    require(prepared['native_execution_performed'] is False and prepared['returncode'] is None, 'unexecuted preparation')
    sources = dict(cfg['sources'])
    sources['tools/linux/mod_activation_probe.py'] = cfg['runner_sha256']
    with tarfile.open(root/'sources.tar') as archive:
        require(len(archive.getnames()) == len(sources) and set(archive.getnames()) == set(sources), 'source archive inventory')
        for name, expected in sources.items():
            raw = (root/'source-preimages'/name).read_bytes()
            require(hashlib.sha256(raw).hexdigest() == expected and archive.extractfile(name).read() == raw, 'source preimage: ' + name)
    predicate = rows(root/'predicate.jsonl')
    controls = rows(root/'controls.jsonl')
    header = one(predicate, 'header')
    summary = one(predicate, 'summary')
    control_summary = one(controls, 'summary')
    require(header['profile_sha256'] == prepared['configuration_sha256'], 'predicate profile')
    require(header['actor_owner'] == 0 and header['actor_full_id'] == 15794187 and header['visible_only'] is False, 'actor')
    require(summary['predicate_invoked'] is True and summary['predicate_returned'] is True and summary['facade_released'] is True, 'one requested predicate completed')
    require(type(summary['allowed']) is bool and summary['operation_id'] == 36 and summary['operation_hash'] == 0x2c681908, 'actual operation/result type')
    require(all(control_summary[key] == summary[key] for key in ('predicate_invoked', 'predicate_returned', 'facade_released', 'observation_status')), 'control result binding')
    phases = [row for row in controls if row.get('kind') == 'observation']
    require([row['phase'] for row in phases] == list(range(1, 10)), 'nine phase observations')
    require(all(row['actor_owner'] == header['actor_owner'] and row['actor_full_id'] == header['actor_full_id'] for row in phases), 'phase actor identity')
    require(all(row['predicate_returned'] is (row['phase'] >= 5) for row in phases), 'phase return ordering')
    require(all(row['allowed'] == summary['allowed'] for row in phases if row['phase'] >= 5), 'observed return binding')
    rng = [bytes.fromhex(row['known_rng_raw']) for row in phases]
    require(all(len(raw) == 64 for raw in rng), 'two known 32-byte RNG objects')
    native_rng = [row for row in predicate if row.get('kind') == 'known_rng']
    require([row['phase'] for row in native_rng] == ['before_predicate', 'after_predicate'], 'caller RNG endpoints')
    require(bytes.fromhex(native_rng[0]['raw']) == rng[3] and bytes.fromhex(native_rng[1]['raw']) == rng[4], 'independent endpoint binding')
    metadata = []
    statuses = [row for row in controls if row.get('kind') == 'metadata_status']
    require([row['phase'] for row in statuses] == [2, 3, 6, 7], 'four metadata attempts')
    for status, label in zip(statuses, ('before', 'control', 'after', 'repeat')):
        require(status['predicate_admission_veto'] is False, 'optional metadata gate')
        path = root/('metadata-' + label + '.jsonl')
        entry = dict(phase=status['phase'], status=status['status'], present=path.exists())
        if path.exists():
            records = rows(path)
            entry.update(bytes=path.stat().st_size, sha256=sha(path), records=len(records))
            if status['status'] == 0:
                require(records[-1]['kind'] == 'complete', 'complete successful metadata')
                entry['complete_counts'] = {key: records[-1][key] for key in ('handler_slots', 'player_slots')}
        metadata.append(entry)
    saves = []
    first = None
    for name in ('save.bin', 'before.bin', 'control.bin', 'after.bin', 'after-repeat.bin', 'repeat.bin'):
        path = root/name
        entry = dict(name=name, present=path.exists())
        if path.exists():
            raw = path.read_bytes()
            if first is None:
                first = raw
            entry.update(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest(), byte_equal_to_first=raw == first)
        saves.append(entry)
    exit_status = json.loads((root/'observer/capture/exit.json').read_text())
    supervisor = json.loads((root/'observer/supervisor.json').read_text())
    require(runtime['returncode'] == 0 and exit_status['inferior_exitcode'] == 0 and exit_status['observer_failed'] is False, 'normal original exit')
    require(supervisor['terminal']['gdb_returncode'] == 0 and supervisor['terminal']['remaining_session_processes'] == [], 'debugger/session terminal')
    require(all(row['original_generation_absent'] for row in closed['process_generation_check']), 'observed process generations terminal')
    stderr = runtime['stderr']
    markers = {'controller': r'C6_LINUX_ACTION_LIFECYCLE uninitialize_return initialized 0 context_bound 0 executing 0 cleanup 1',
               'framework': r'C6_LINUX_MOD cleanup_return 0',
               'client': r'C6_LINUX_MOD client_shutdown_only_return unpublished 1'}
    require(all(re.search(r'^' + pattern + r'(?: |$)', stderr, re.M) for pattern in markers.values()), 'actual cleanup footers')
    trace = rows(root/'trace/events.jsonl')
    require(trace[-1]['event'] == 'end', 'known trace terminal')
    return {'schema': 'c6-linux-full-manager-controls-result-v1', 'configuration_sha256': prepared['configuration_sha256'],
            'runtime_sha256': expected_runtime, 'closed_manifest_sha256': expected_closed,
            'source_preimages_verified': len(sources), 'requested_actor': {'owner': 0, 'full_id': 15794187},
            'classification_written_after_capture': True,
            'actual_predicate': summary, 'observation_phases': len(phases), 'known_rng_bytes_per_phase': 64,
            'known_rng_endpoint_records': len(phases) + len(native_rng), 'known_rng_objects': 2,
            'known_rng_all_phase_bytes_equal': all(raw == rng[0] for raw in rng), 'complete_rng_inventory': False,
            'metadata': metadata, 'optional_Save_diagnostics': saves, 'Save_equality_is_gate': False,
            'metadata_evidence_status': one(controls, 'evidence_status'), 'known_trace_end': trace[-1],
            'all_policy_activity_observed': False, 'original_cleanup_footers_verified': True,
            'observed_original_process_generations_absent': True, 'Rust_comparison_performed': False,
            'unreadable_unrelated_procfs_pids': len(closed['procfs_unreadable_pids']), 'unrestricted_global_process_absence_claimed': False,
            'registry_or_callback_absence_inferred': False, 'complete_affected_state_claimed': False,
            'full_action_or_turn_parity_claimed': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', required=True, type=Path)
    parser.add_argument('--expected-runtime', required=True)
    parser.add_argument('--expected-closed', required=True)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    report = audit(args.directory, args.expected_runtime, args.expected_closed)
    args.output.write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
