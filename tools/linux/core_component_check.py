#!/usr/bin/env python3
"""Validate a separate conditional Core component; never full-host action acceptance."""
import argparse
import hashlib
import json
import struct
from pathlib import Path

from save_lifecycle_audit import GAME_SHA, HOST_SHA


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def content_identity(path):
    path = Path(path)
    return dict(bytes=path.stat().st_size, sha256=digest(path))


def artifact(path, recorded):
    actual = content_identity(path)
    require(all(recorded[key] == value for key, value in actual.items()), 'artifact identity differs: ' + str(path))
    if 'filename' in recorded:
        require(Path(path).name == recorded['filename'], 'artifact filename differs')
    return actual


def raw_difference(before, after):
    different = common = 0
    with Path(before).open('rb') as left, Path(after).open('rb') as right:
        while True:
            a, b = left.read(1024 * 1024), right.read(1024 * 1024)
            if not a and not b:
                break
            common += min(len(a), len(b))
            different += sum(x != y for x, y in zip(a, b))
    length = abs(Path(before).stat().st_size - Path(after).stat().st_size)
    return dict(common_bytes=common, differing_common_bytes=different,
                length_difference=length, total_differing_bytes=different + length,
                masked_bytes=0, unknown_bytes_classified=False)


def payload(row):
    return {key: value for key, value in row.items() if key != 'stage'}


def typed_observation(row):
    """Explicit proven fields; raw object bytes remain separately unmasked."""
    result = {key: row[key] for key in ('unit_present', 'activity', 'automation', 'operation_timer',
              'moves_raw_u32', 'queue_size', 'has_deltas', 'dirty_indexes', 'rng_unchanged_during_observation')}
    result['queue'] = [{key: node[key] for key in ('index', 'operation', 'directive', 'flags',
                       'push_turn', 'pre_activity', 'pre_automation')} for node in row['queue']]
    for name in ('game_sync_random', 'map_generator_random'):
        result[name] = {key: row[name][key] for key in ('seed', 'calls', 'resets', 'lock', 'raw_sync')}
    return result


def raw_live_differences(left, right):
    differences = []
    pairs = [(name, left[name]['raw32'], right[name]['raw32'])
             for name in ('game_sync_random', 'map_generator_random')]
    pairs.extend((f'queue[{i}]', a['raw40'], b['raw40']) for i, (a, b) in enumerate(zip(left['queue'], right['queue'])))
    for name, a, b in pairs:
        if a != b:
            differences.append(dict(object=name, left_raw=a, right_raw=b,
                                    differing_offsets=[i for i, (x, y) in enumerate(zip(bytes.fromhex(a), bytes.fromhex(b))) if x != y],
                                    masked_bytes=0, semantic_classification_complete=False))
    return differences


def validate(path):
    run = json.loads(Path(path).read_text())
    config, paths = run['configuration'], run['execution_paths']
    identity = hashlib.sha256(json.dumps(config, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    require(identity == run['configuration_sha256'], 'configuration identity mismatch')
    require(config['assets'].get('libGameCore_XP2.so') == GAME_SHA and config['assets'].get('Civ6') == HOST_SHA, 'original module gates differ')
    mode = config['core_component_mode']
    require(mode in ('control', 'execute'), 'explicit separate component mode missing')
    execute = mode == 'execute'
    require(not config.get('bounded_action_mode') and not config.get('pre_controller_registration') and not config.get('full_save_request'), 'component mixed with original Request/registration profile')
    contract = config['core_component_contract']
    require(contract['schema'] in ('c6-linux-core-component-static-v1', 'c6-linux-core-component-static-v2') and contract['checks_passed'], 'component static contract missing')
    current_scheduler = contract['schema'] == 'c6-linux-core-component-static-v2'
    require(current_scheduler or not execute, 'initial scheduling profile cannot accept an execute result')
    require(contract['host_sha256'] == HOST_SHA and contract['gamecore_sha256'] == GAME_SHA, 'component module gates differ')
    require(contract['host_admission_proven'] is False and contract['full_action_acceptance'] == 0 and contract['gameplay_parity'] is False, 'component contract overclaims acceptance')
    canonical_name = 'core-component-static.json' if current_scheduler else 'core-component-static-initial.json'
    canonical = json.loads((Path(__file__).resolve().parents[2] / 'evidence/linux' / canonical_name).read_text())
    require(contract == canonical, 'component constructor/source contract differs from reviewed checkpoint')
    require(all(config['sources'].get(name) == value for name, value in contract['source_sha256'].items()), 'compiled component source identity differs')
    require(all(name in config['sources'] for name in ('tools/linux/action_execution_trace.c', 'native/linux/action_state.c', 'native/linux/context_stream.c')), 'native guard/observer/stream source identities missing')
    input_identity = content_identity(paths['lower_core_stream'])
    require(input_identity == config['lower_core_stream'], 'actual lower-Core input identity differs')
    require(config.get('saved_configuration_sha256') and config.get('preload_sha256') and config.get('runtime_dependencies'), 'saved configuration/preload/dependency identities missing')
    require(config.get('userdata_initially_empty') is True and config.get('userdata_policy') == 'new empty private task directory', 'fresh isolated process profile missing')
    if paths.get('saved_configuration'):
        require(digest(paths['saved_configuration']) == config['saved_configuration_sha256'], 'actual saved configuration differs')
    if paths.get('preload'):
        require(digest(paths['preload']) == config['preload_sha256'], 'actual archived preload differs')
    require(run['returncode'] == 0 and run['framework_cleanup_proven'] and run['client_cleanup_proven'], 'failed native diagnostic or incomplete cleanup')
    require(run['gameplay_parity'] is False, 'runtime report overclaims parity')
    stderr = run['stderr']
    require(not any(word in stderr for word in ('guarded_entry', 'failing_boundary', 'C6_LINUX_ACTION_DISPATCH ', 'C6_LINUX_ACTION_REGISTRATION ', 'request_enter', 'network_update_zero_enter')), 'policy guard or mixed full-host action route observed')
    readiness = [json.loads(line.split(' ', 1)[1]) for line in stderr.splitlines() if line.startswith('C6_LINUX_ACTION_EXECUTION {')]
    require([row['stage'] for row in readiness] == ['host_delivery_observation', 'cache_dispatch_observation'], 'native controller/cache readiness observations missing')
    require(all(readiness[0][key] == 1 for key in ('controller_present', 'controller_initialized', 'controller_context_matches', 'host_context_matches', 'network_gamecore_active')), 'genuine controller/context binding incomplete')
    require(all(readiness[1][key] == 1 for key in ('live_active', 'live_human', 'cache_present', 'cached_active', 'cached_human', 'request_cache_predicate_passes')), 'native live/cached player readiness incomplete')
    rows = [json.loads(line.split(' ', 1)[1]) for line in stderr.splitlines() if line.startswith('C6_LINUX_CORE_COMPONENT {')]
    candidates = [row for row in rows if row['stage'] == 'actor_candidate']
    singleton = [row for row in rows if row['stage'] != 'actor_candidate']
    stages = {row['stage']: row for row in singleton}
    require(len(stages) == len(singleton), 'duplicate component observation stage')
    if current_scheduler:
        released, reacquired = stages['facades_released'], stages['observation_facades_reacquired']
        require(released['execute'] == execute and released['original_controller_lock_held'] is False and
                all(released[key] is True for key in ('game', 'player', 'unit_list', 'operations', 'native_parameters_retained')), 'original facade lock release missing')
        require(reacquired['game'] is True and reacquired['operations'] is True and reacquired['original_controller_lock_held'] is True and
                reacquired['after_original_worker_quiescence'] == execute, 'symmetric original observation facade reacquisition differs')
        require(rows.index(released) < rows.index(reacquired) < rows.index(stages['predicate_after']), 'symmetric facade observation ordering differs')
    if execute:
        begin, quiet = stages['worker_wait_begin'], stages['worker_quiescent']
        require(begin['deadline_ms'] == contract['worker_deadline_ms'] == 30000 and begin['original_worker_present'] is True and begin['original_worker_is_current_thread'] is False, 'original worker wait identity/deadline differs')
        require(quiet['original_is_executing'] is False and all(quiet[key] is True for key in ('original_join_returned', 'original_synchronize_returned', 'original_worker_suspended', 'worker_identity_unchanged')), 'original worker barrier or suspension missing')
        lines = stderr.splitlines()
        quiet_position = next(i for i, line in enumerate(lines) if line.startswith('C6_LINUX_CORE_COMPONENT {') and json.loads(line.split(' ', 1)[1])['stage'] == 'worker_quiescent')
        after_position = next(i for i, line in enumerate(lines) if line.startswith('C6_LINUX_ACTION_STATE {') and json.loads(line.split(' ', 1)[1])['stage'] == 'after_a')
        save_positions = [i for i, line in enumerate(lines) if line.startswith('C6_LINUX_STREAM save_enter ')]
        require(len(save_positions) == 5 and quiet_position < after_position < save_positions[2], 'post-state/Save observed before original worker quiescence')
        require(rows.index(stages['facades_released']) < rows.index(begin) < rows.index(quiet) < rows.index(stages['observation_facades_reacquired']), 'original facade/worker handoff ordering differs')
    else:
        require('worker_wait_begin' not in stages and 'worker_quiescent' not in stages, 'control unexpectedly waited/executed worker')
    initial, final, selected = (stages[key] for key in ('before', 'after', 'selected'))
    require(selected['mode'] == mode and selected['native_queue_size'] in (0, 1), 'fresh bounded actor selection missing')
    require(any(all(row.get(k) == selected[k] for k in ('id', 'component_bits', 'owner')) and row['public_can_start'] == row['core_found'] == 1 and row['queue_query_status'] == 0 and row['queue_size'] == selected['native_queue_size'] for row in candidates), 'selected actor lacks genuine legality/lookup proof')
    require(selected['hash'] == '2c681908' and selected['bAppend'] is False and selected['bFromUser'] is False, 'native parameter defaults differ')
    require(stages['serializer_observation']['request_active'] == stages['serializer_observation']['request_in_progress'] == 0 and stages['serializer_guard']['actual_request_idle'] == 1, 'actual serialization request not quiescent')
    require(all(stages['player_activation'][key] == 1 for key in ('core_player_present', 'unit_container_present', 'activation_complete')), 'player activation readiness incomplete')
    host = stages['host_worker_preconditions']
    require(host['controller_present'] == 1 and host['worker_executing'] == host['worker_suspended'] == host['native_saving'] == 0, 'native worker/serializer precondition failed')
    for key in ('turn', 'phase', 'local'):
        require(initial[key] == final[key], 'bounded component changed ' + key)
    for flag in ('pending', 'needs_update', 'needs_advance', 'ai_processing', 'turn_processing'):
        require(initial[flag] == final[flag] == 0, 'unfinished component work: ' + flag)
    for key in ('diplomacy_before', 'diplomacy_after'):
        require(stages[key]['pending_actions'] == stages[key]['open_session_map_nonempty'] == 0, 'nonempty diplomacy boundary')
    outcome = stages['diplomacy_after']
    require(outcome['result'] == 0 and outcome['component_enqueued'] == execute and outcome['gameplay_parity'] is False and outcome['full_action_acceptance'] == 0, 'component completion or scope differs')
    admission, conversion, serialized = (stages[key] for key in ('admission_assumptions', 'conversion', 'serialized'))
    require(admission['source'] == 'fresh_native_make_request_component' and admission['sender'] == admission['identity_owner'] == initial['local'] == selected['owner'], 'native sender/local/encoded owner differ')
    require(all(readiness[1][key] == initial['local'] for key in ('local_player', 'live_player', 'cached_player')), 'native cached/live/local identity differs')
    require(admission['host_admission_proven'] is False and admission['full_action_acceptance'] == 0 and serialized['host_admission_proven'] is False, 'component admission claim differs')
    require(conversion['accepted'] is True and conversion['bAppend'] == conversion['bFromUser'] == 0 and conversion['source'] == 'fresh_original_constructor', 'original fresh conversion failed')
    require(serialized['length'] == 28 and serialized['sender'] == admission['sender'] and serialized['execute'] == execute, 'native serialization boundary differs')
    require(f'C6_LINUX_ACTION_LIFECYCLE uninitialize_return initialized 0 context_bound 0 executing 0 cleanup 1 dispatch_mode 0 requested_execution {int(execute)} core_component_mode 1' in stderr, 'distinct native controller teardown missing')
    states = [json.loads(line.split(' ', 1)[1]) for line in stderr.splitlines() if line.startswith('C6_LINUX_ACTION_STATE {')]
    require([row['stage'] for row in states] == ['before_a', 'before_b', 'after_a', 'after_b'], 'bounded state observation order differs')
    require(all(row['unit_present'] and row['rng_unchanged_during_observation'] for row in states), 'observer failure or observer RNG mutation')
    require(payload(states[0]) == payload(states[1]) and payload(states[2]) == payload(states[3]), 'repeated live observations differ')
    require(states[0]['queue_size'] == len(states[0]['queue']) == selected['native_queue_size'], 'selected original queue observation differs')
    if execute:
        require(states[2]['activity'] == 0x047c048b and states[2]['queue_size'] == 0 and states[2]['queue'] == [], 'bounded native SkipTurn HOLD/empty queue effect missing')
    else:
        require(payload(states[0]) == payload(states[2]), 'control changed bounded live state/RNG')
    component_paths = paths['core_component']
    command = artifact(component_paths['command'], run['core_component_command'])
    require(command['bytes'] == 28, 'actual native command length differs')
    # Original BaseMessage.PutInBuffer 0x2d20ec0 writes key first;
    # UnitOperation body 0x2d28ab0 calls ComponentID stream writer 0x2c9ac60,
    # which writes native owner i16, type i16, id u32 in that order.
    message_key, encoded_owner, encoded_type, encoded_id = struct.unpack_from('<IhhI', Path(component_paths['command']).read_bytes())
    require(message_key == 0x8c0f3f36 and encoded_owner == admission['sender'] and encoded_id == selected['id'], 'actual serialized actor/source identity differs')
    saves = {key: artifact(component_paths[key], run['core_component_saves'][key]) for key in ('before', 'after', 'repeat')}
    require(saves['after'] == saves['repeat'], 'repeated post-component Core Saves differ')
    require((saves['before'] != saves['after']) == execute, 'raw Core effect differs from mode')
    outer = {key: artifact(paths['game_outputs'][key], run['game_saves'][key]) for key in ('save', 'repeat')}
    require(outer['save'] == saves['before'] and outer['repeat'] == saves['after'], 'lifecycle/queries changed serialized Core outside component')
    trace_path = Path(paths['action_trace_directory']) / 'events.jsonl'
    trace = [json.loads(line) for line in trace_path.read_text().splitlines()]
    artifact(trace_path, run['action_trace_files']['events.jsonl'])
    require(trace[0]['event'] == 'begin' and trace[0]['UnitOperation_Execute_virtual_interposed'] and trace[0]['hook_owner_preload'] and trace[0]['target_owner_preload'], 'original handler hook owner proof missing')
    bindings = [row for row in trace if row['event'] in ('original_PLT_binding', 'original_virtual_binding')]
    expected_bindings = {'BroadcastMessage', 'GameProcessor_Update', 'GlobalAi_ProcessAi', 'ReceiveMessage',
                         'EventSend_identity', 'QueueMessage', 'EventSend_identity_two_int', 'Diplomatic_ProcessAction',
                         'AIControl_DoTurn', 'Diplomatic_ProcessDeal', 'Diplomacy_ProcessStatement', 'Game_Update', 'EventSend_identity_int'}
    plt_bindings = [row for row in bindings if row['event'] == 'original_PLT_binding']
    require(len(plt_bindings) == 13 and {row['name'] for row in plt_bindings} == expected_bindings and
            len([row for row in bindings if row.get('name') == 'UnitManager_MakeRequest']) == 1, 'selected native guard/conversion bindings incomplete')
    require(all(row['interposed'] and row['hook_owner_preload'] and row['target_owner_preload'] for row in bindings), 'guard provider differs from original trace owner')
    require(not any(row['event'] in ('broadcast_enter', 'broadcast_return', 'failing_boundary') for row in trace), 'component emitted broadcast or fatal boundary')
    requests = [row for row in trace if row['event'] in ('make_request_enter', 'make_request_return') and row['phase'] == 1]
    require([row['event'] for row in requests] == ['make_request_enter', 'make_request_return'] and requests[0]['hash'] == 0x2c681908, 'fresh original component conversion trace missing')
    require(requests[1]['accepted'] and requests[1]['operation'] == 36 and requests[1]['flags'] == requests[1]['bAppend'] == requests[1]['bFromUser'] == 0, 'native converted SkipTurn defaults differ')
    end = trace[-1]
    require(end['event'] == 'end' and end['queued'] == end['received'] == end['unit_executed'] == int(execute), 'native component message count differs')
    require(sum(row['event'] == 'unit_execute_enter' for row in trace) == sum(row['event'] == 'unit_execute_return' for row in trace) == int(execute), 'native handler execution unbalanced')
    phase = None
    ordered = []
    packets = []
    for row in trace:
        if row['event'] == 'phase':
            phase = row['value']
        if row['event'] in ('queue', 'receive'):
            require(phase == (1 if row['event'] == 'queue' else 5), 'native enqueue/receive phase differs')
            name = f"{row['packet']:03d}-{row['event']}.bin"
            packet = artifact(trace_path.parent / name, run['action_trace_files'][name])
            require(packet == command and row['bytes'] == 28 and row['offset'] == 0 and row['from'] == admission['sender'], 'native command/queue/receive bytes or sender differ')
            packets.append(dict(kind=row['event'], **packet, sender=row['from']))
        if row['event'] in ('queue', 'receive', 'unit_execute_enter', 'unit_execute_return'):
            ordered.append(row['event'])
    require(ordered == (['queue', 'receive', 'unit_execute_enter', 'unit_execute_return'] if execute else []), 'original message/handler ordering differs')
    phases = [row['value'] for row in trace if row['event'] == 'phase']
    require(all(phases.count(value) == 1 for value in (1, 2, 7)), 'component progression phase markers missing')
    if execute:
        require(phases.index(1) < phases.index(2) < phases.index(5) < phases.index(6) < phases.index(7), 'original worker/synchronize phase ordering differs')
    else:
        require(5 not in phases and 6 not in phases and end['reports'] == 0, 'control ran worker or emitted reports')
    result = dict(schema='c6-linux-conditional-core-component-v1', checks_passed=True, mode=mode,
                  runtime_report_sha256=digest(path), configuration_sha256=identity,
                  source='fresh_native_make_request_component', selected=selected, sender=admission['sender'],
                  source_sha256=config['sources'], preload_sha256=config['preload_sha256'], original_input=input_identity,
                  isolated_artifact_paths=dict(userdata=paths['userdata'], command=component_paths['command'], trace=str(trace_path)),
                  preload_artifact_rechecked=bool(paths.get('preload')), saved_configuration_artifact_rechecked=bool(paths.get('saved_configuration')),
                  readiness_before=initial, readiness_after=final, component_observations=rows,
                  native_controller_cache_readiness=readiness,
                  bounded_live_observations=states, command=command, packets=packets,
                  native_encoded_actor=dict(message_key=hex(message_key), owner=encoded_owner, component_type=encoded_type, id=encoded_id),
                  trace_sha256=digest(trace_path), trace_end=end, reporting_events=[row for row in trace if row['event'].startswith('report_')],
                  original_core_saves=saves, outer_original_core_saves=outer,
                  raw_core_difference=raw_difference(component_paths['before'], component_paths['after']),
                  native_cleanup_proven=True, conditional_component_completed=True,
                  original_worker_quiescence_proven=execute and current_scheduler,
                  host_admission_proven=False, full_action_acceptance=0, gameplay_parity=False,
                  complete_affected_state=False, complete_rng_inventory=False, complete_policy_boundary_coverage=False,
                  complete_cross_process_initial_state_equivalence_proven=False,
                  same_input_control_compared=False, isolated_repeat_compared=False,
                  limits='Conditional Core component only. Complete Core bytes and selected live unit/queue/dirty ordering plus two identified RNG objects; complete live state/RNG and all-player policy continuations unresolved. Raw Save-to-Load 89,198-byte failure remains unmasked. No full-host request acceptance or Rust action parity.')
    return result, config


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime-report', type=Path, required=True)
    parser.add_argument('--control-report', type=Path)
    parser.add_argument('--repeat-report', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    try:
        result, config = validate(args.runtime_report)
        for name, other_path, expected_mode in [('same_input_control_compared', args.control_report, 'control'), ('isolated_repeat_compared', args.repeat_report, 'execute')]:
            if other_path:
                other, other_config = validate(other_path)
                require(result['mode'] == 'execute' and other['mode'] == expected_mode, 'cross-run component modes differ')
                require(other['runtime_report_sha256'] != result['runtime_report_sha256'], 'isolated comparison reused the same runtime report')
                require(all(Path(result['isolated_artifact_paths'][key]).resolve() != Path(other['isolated_artifact_paths'][key]).resolve()
                            for key in ('userdata', 'command', 'trace')), 'cross-run comparison reused process/artifact paths')
                for key in ('assets', 'sources', 'preload_sha256', 'lower_core_stream', 'saved_configuration_sha256', 'dlc_descriptors', 'core_component_contract'):
                    require(config[key] == other_config[key], 'cross-run configuration differs: ' + key)
                require(result['selected'] == dict(other['selected'], mode='execute') and result['command'] == other['command'], 'cross-run actor/command differs')
                require(result['original_core_saves']['before'] == other['original_core_saves']['before'] and typed_observation(result['bounded_live_observations'][0]) == typed_observation(other['bounded_live_observations'][0]), 'cross-run serialized or bounded typed initial state differs')
                raw_differences = {'before': raw_live_differences(result['bounded_live_observations'][0], other['bounded_live_observations'][0])}
                if expected_mode == 'execute':
                    require(result['original_core_saves']['after'] == other['original_core_saves']['after'] and typed_observation(result['bounded_live_observations'][2]) == typed_observation(other['bounded_live_observations'][2]), 'isolated repeated serialized or bounded typed effects differ')
                    raw_differences['after'] = raw_live_differences(result['bounded_live_observations'][2], other['bounded_live_observations'][2])
                result[name] = True
                result[name + '_runtime_sha256'] = other['runtime_report_sha256']
                result[name + '_raw_live_differences'] = raw_differences
                result[name + '_scope'] = 'Matching original inputs/sources, complete serialized Core streams and explicitly selected typed fields only; raw cross-process bytes remain reported and unmasked. No complete live-state/parameter ownership equivalence claim.'
    except (ValueError, KeyError, OSError, IndexError, TypeError, StopIteration, struct.error) as error:
        result = dict(schema='c6-linux-conditional-core-component-v1', checks_passed=False,
                      runtime_report_sha256=digest(args.runtime_report), reason=str(error),
                      failed_diagnostic=True, host_admission_proven=False, full_action_acceptance=0, gameplay_parity=False)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(result, indent=2) + '\n')
        raise SystemExit(str(error)) from error
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(dict(mode=result['mode'], conditional_component_completed=True, host_admission_proven=False, full_action_acceptance=0, gameplay_parity=False)))


if __name__ == '__main__':
    main()
