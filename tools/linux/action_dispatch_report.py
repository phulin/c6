#!/usr/bin/env python3
"""Validate one bounded original Linux SkipTurn or no-action control experiment."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import GAME_SHA, HOST_SHA


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime-report', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    run = json.loads(args.runtime_report.read_text())
    config = run['configuration']
    identity = hashlib.sha256(json.dumps(config, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    require(identity == run['configuration_sha256'], 'configuration identity mismatch')
    require(GAME_SHA in config['assets'].values() and HOST_SHA in config['assets'].values(), 'original module gates missing')
    mode = config['bounded_action_mode']
    require(mode in ('control', 'execute'), 'explicit bounded action mode missing')
    execute = mode == 'execute'
    require(run['returncode'] == 0 and run['framework_cleanup_proven'] and run['client_cleanup_proven'], 'native execution or full cleanup failed; discard isolated process')
    stderr = run['stderr']
    require('guarded_entry' not in stderr and 'failing_boundary' not in stderr, 'unexpected selected policy boundary; discard isolated process')
    prefix = 'C6_LINUX_ACTION_DISPATCH '
    rows = [json.loads(line[len(prefix):]) for line in stderr.splitlines() if line.startswith(prefix + '{')]
    candidates = [row for row in rows if row['stage'] == 'actor_candidate']
    singleton_rows = [row for row in rows if row['stage'] not in ('actor_candidate', 'protocol_pass')]
    stages = {row['stage']: row for row in singleton_rows}
    require(len(stages) == len(singleton_rows), 'duplicate dispatch stage')
    initial, final, selected = (stages[key] for key in ('before', 'after', 'selected'))
    serializer = stages['serializer_observation']
    require(serializer['request_active'] == 0 and serializer['request_in_progress'] == 0, 'actual host serialization request is active')
    require(stages['serializer_guard']['actual_request_idle'] == 1, 'underlying original serialization guard failed')
    require(all(stages['player_activation'][key] == 1 for key in ('core_player_present', 'unit_container_present', 'activation_complete')), 'original player activation readiness incomplete')
    require(selected['mode'] == mode and selected['native_queue_size'] in (0, 1), 'fresh legal bounded queue selection missing')
    require(any(row['id'] == selected['id'] and row['component_bits'] == selected['component_bits'] and row['public_can_start'] == 1 and row['core_found'] == 1 and row['queue_query_status'] == 0 and row['queue_size'] == selected['native_queue_size'] for row in candidates), 'selected actor lacks original legality/lookup/queue proof')
    require(selected['hash'] == '2c681908' and selected['bAppend'] is False and selected['bFromUser'] is False, 'original default request semantics differ')
    for flag in ('pending', 'needs_update', 'needs_advance', 'ai_processing', 'turn_processing'):
        require(initial[flag] == 0, 'initial readiness failed: ' + flag)
    for key in ('turn', 'phase', 'local'):
        require(initial[key] == final[key], 'bounded action changed ' + key)
    for flag in ('pending', 'ai_processing', 'turn_processing'):
        require(final[flag] == 0, 'message drain incomplete: ' + flag)
    for key in ('diplomacy_before', 'diplomacy_after'):
        require(stages[key]['pending_actions'] == 0 and stages[key]['open_session_map_nonempty'] == 0, 'diplomacy scan nonempty')
    require(stages['diplomacy_after']['result'] == 0 and stages['diplomacy_after']['request_issued'] == execute, 'request outcome differs')
    require(f'cleanup 1 dispatch_mode 1 requested_execution {int(execute)}' in stderr, 'original controller teardown missing')
    states = [json.loads(line.split(' ', 1)[1]) for line in stderr.splitlines() if line.startswith('C6_LINUX_ACTION_STATE {')]
    require([row['stage'] for row in states] == ['before_a', 'before_b', 'after_a', 'after_b'], 'state observation order differs')
    def payload(row):
        return {key: value for key, value in row.items() if key != 'stage'}
    require(all(row['unit_present'] and row['rng_unchanged_during_observation'] for row in states), 'observer failed or observed RNG mutation')
    require(states[0]['queue_size'] == selected['native_queue_size'] and len(states[0]['queue']) == selected['native_queue_size'], 'original pre-action queue observation differs from selected actor')
    require(payload(states[0]) == payload(states[1]) and payload(states[2]) == payload(states[3]), 'repeated live observations differ')
    if execute:
        require(states[2]['activity'] == 0x047c048b and states[2]['queue_size'] == 0 and states[2]['queue'] == [], 'SkipTurn final HOLD/empty queue effect missing')
    else:
        require(payload(states[0]) == payload(states[2]), 'no-action control changed observed live state')
    trace_path = Path(run['execution_paths']['action_trace_directory']) / 'events.jsonl'
    trace = [json.loads(line) for line in trace_path.read_text().splitlines()]
    bindings = [row for row in trace if row['event'] == 'original_PLT_binding']
    require(trace[0]['UnitOperation_Execute_virtual_interposed'] and len(bindings) in (12, 13) and all(row['interposed'] for row in bindings), 'selected guard binding proof incomplete')
    if len(bindings) == 13:
        require(any(row['event'] == 'original_virtual_binding' and row['name'] == 'UnitManager_MakeRequest' and row['interposed'] for row in trace), 'original MakeRequest virtual observation not bound')
    end = trace[-1]
    require(end['event'] == 'end' and end['queued'] == end['received'] and end['unit_executed'] == int(execute) and (end['queued'] >= 1 if execute else end['queued'] == 0), 'genuine request delivery or execution count differs')
    require(sum(row['event'] == 'unit_execute_enter' for row in trace) == int(execute) and sum(row['event'] == 'unit_execute_return' for row in trace) == int(execute), 'original UnitOperation enter/return imbalance')
    if not execute:
        require(end['reports'] == 0, 'no-action control emitted armed reporting events')
    packets = {}
    for kind in ('queue', 'receive'):
        events = [row for row in trace if row['event'] == kind]
        require(len(events) == end['queued'], 'packet trace count differs: ' + kind)
        if events:
            packets[kind] = []
        for event in events:
            path = trace_path.parent / f"{event['packet']:03d}-{kind}.bin"
            require(path.is_file(), 'original packet bytes missing: ' + str(path))
            require(path.stat().st_size == event['bytes'] and 0 <= event['offset'] <= event['bytes'], 'packet capture size/offset differs')
            packets[kind].append(dict(bytes=path.stat().st_size, sha256=digest(path), event=event))
    if execute:
        require([(row['sha256'], row['bytes']) for row in packets['queue']] == [(row['sha256'], row['bytes']) for row in packets['receive']], 'original queue/receive packet bytes differ')
    saves = {}
    for key, name in run['execution_paths']['action_snapshots'].items():
        path = Path(name)
        value = dict(bytes=path.stat().st_size, sha256=digest(path))
        require(all(run['action_saves'][key][field] == val for field, val in value.items()), 'actual original Save identity differs')
        saves[key] = value
    require(set(saves) == {'before', 'after', 'repeat'}, 'three original action Saves required')
    require(saves['after'] == saves['repeat'], 'repeated original post-action Saves differ')
    require((saves['before'] != saves['after']) == execute, 'serialized effect does not match explicit mode')
    outer_saves = {}
    for key, action_key in (('save', 'before'), ('repeat', 'after')):
        path = Path(run['execution_paths']['game_outputs'][key])
        actual = dict(bytes=path.stat().st_size, sha256=digest(path))
        require(all(run['game_saves'][key][field] == value for field, value in actual.items()), 'outer original Save identity differs')
        require(actual == saves[action_key], 'queries/lifecycle changed serialized Core outside action snapshot')
        outer_saves[key] = actual
    report = dict(schema='c6-linux-bounded-action-experiment-v1', runtime_report_sha256=digest(args.runtime_report), configuration_sha256=identity,
                  mode=mode, selected=selected, readiness_before=initial, readiness_after=final, dispatch_observations=rows,
                  live_observations=states, trace_sha256=digest(trace_path), trace_end=end, reporting_events=[row for row in trace if row['event'].startswith('report_')],
                  original_packets=packets, original_core_saves=saves, outer_original_core_saves=outer_saves, native_full_cleanup=True, message_boundary_pass=True,
                  complete_policy_boundary_coverage=False, complete_affected_state=False, complete_rng_inventory=False, gameplay_parity=False,
                  limits='Exploratory genuine original request/delivery/drain only. Two identified RNG objects and selected unit live fields/dirty ordering are observed; all affected live state and all gameplay RNG streams remain unresolved. Selected policy guards are not complete engine coverage. NeedsUpdate/NeedsTurnAdvance are reported without a full readiness claim. Guard aborts are failed possibly partial-state processes, never resumable yields.')
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(mode=mode, message_boundary_pass=True, gameplay_parity=False)))


if __name__ == '__main__':
    main()
