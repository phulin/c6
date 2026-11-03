#!/usr/bin/env python3
"""Offline classification of a closed original Military observation capture.

Reads immutable artifacts only. Does not run original code, invoke a candidate,
change a frozen observer/runner, normalize native facts into an admitted Rust
snapshot, or establish complete Player/world/action/turn equivalence.
"""
import argparse
from collections import Counter, defaultdict
import copy
import hashlib
import json
from pathlib import Path
import struct
import tarfile

from military_initialization_observer import GAME_SHA, HOST_SHA, SITES, validate_binding, validate_runtime_profile

NAMES = ('allowed', 'per_war', 'current')
ENGINE = {'observer_begin', 'read_entry', 'initialize_entry', 'personality_return',
          'leader_return', 'query_entry', 'count_entry', 'count_return',
          'initialize_return', 'read_initialized', 'read_return', 'write_entry',
          'write_guard', 'hash_array_entry', 'write_return', 'saved_output_binding', 'capture_end'}


def require(value, message):
    if not value:
        raise ValueError(message)


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':')).encode()


def sha(value):
    return hashlib.sha256(value).hexdigest()


def file_identity(path):
    digest = hashlib.sha256()
    size = 0
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1 << 20), b''):
            digest.update(block)
            size += len(block)
    return dict(sha256=digest.hexdigest(), bytes=size)


def vectors(value):
    require(set(value) == set(NAMES), 'missing/extra vector')
    for name, vector in value.items():
        size, capacity = vector['size_u32'], vector['capacity_u32']
        require(type(size) is int and type(capacity) is int and 0 <= size <= capacity <= 65536,
                'vector length/capacity: ' + name)
        require(len(vector['values_i32']) == size, 'vector values/length mismatch')
        require(all(type(x) is int and -2**31 <= x < 2**31 for x in vector['values_i32']), 'vector i32')
        require(int(vector['data_pointer'], 16) > 0 or capacity == 0, 'null backing')
        for key in ('padding_u32', 'tail_padding_u32'):
            require(type(vector[key]) is int and 0 <= vector[key] < 2**32, 'padding width')
    return value


def definition_order(value):
    begin, end, capacity = (int(value[k], 16) for k in ('begin_pointer', 'end_pointer', 'capacity_pointer'))
    require(begin <= end <= capacity and (end-begin) % 8 == 0 and (capacity-begin) % 8 == 0,
            'definition pointer span')
    require(len(value['ordered']) == (end-begin)//8 <= 65536, 'definition count/span')
    for row in value['ordered']:
        require(int(row['pointer'], 16) > 0, 'null definition')
        for key in ('database_row_i32', 'default_value_i32'):
            require(type(row[key]) is int and -2**31 <= row[key] < 2**31, 'definition i32')
        for key in ('collection_index_u32', 'operation_type_hash_u32'):
            require(type(row[key]) is int and 0 <= row[key] < 2**32, 'definition u32')
    return value


def snapshot(row, military=None, player=None):
    value = row['snapshot']
    require(value is not None, 'missing snapshot')
    require(military is None or int(value['military_pointer'], 16) == military, 'snapshot Military mismatch')
    require(player is None or int(value['player_pointer'], 16) == player, 'snapshot Player mismatch')
    require(type(value['player_id_i32']) is int and -2**31 <= value['player_id_i32'] < 2**31, 'owner i32')
    require(type(value['was_ever_alive']) is bool, 'WasEverAlive type')
    vectors(value['vectors'])
    return value


def values_digest(value):
    return sha(canonical({name: value[name]['values_i32'] for name in NAMES}))


def validate_events(rows, config, profile, source_digest, outputs):
    """Validate role-local raw facts; return diagnostic summaries, never admission."""
    counts = Counter(row['event'] for row in rows)
    allowed = ENGINE | {'installed', 'passive_owner_guard', 'marker_checkpoint', 'stream_marker',
                        'stream_destroyed', 'checkpoint', 'owner_installation_witness',
                        'original_stream_range', 'known_rng_after_load', 'final_live_original_method_fingerprints_equal'}
    require(set(counts) <= allowed, 'unsupported observation event')
    require(rows[0]['event'] == 'observer_begin' and rows[-1]['event'] == 'capture_end', 'capture endpoints')
    require(rows[-1]['observer_failed'] is False and rows[-1]['capture_complete'] is False,
            'raw capture completion/observer status')
    require(not any(row['event'] in ('capture_refused', 'runner_error') for row in rows), 'capture refusal')
    engine_rows = [row for row in rows if 'sequence' in row]
    require(all(row['event'] in ENGINE for row in engine_rows), 'unknown sequenced event')
    require([row['sequence'] for row in engine_rows] == list(range(1, len(engine_rows)+1)), 'engine sequence')
    require(counts['observer_begin'] == counts['capture_end'] == counts['installed'] == 1, 'duplicate endpoints')
    installed = next(row for row in rows if row['event'] == 'installed')
    require(installed['profile_sha256'] == profile and installed['hardware_target_lanes'] == 3
            and installed['provenance_lanes'] == 1 and installed['software_game_breakpoints'] is False,
            'hardware/source profile')
    active, pending, group = None, None, None
    streams, groups, checkpoints, frames = {}, {}, defaultdict(list), {}
    ranges, output_bindings, definitions = {}, {}, []
    thread, bias, guard_count = None, None, 0
    witness = None
    descriptor = None
    for index, row in enumerate(rows):
        event = row['event']
        if event == 'passive_owner_guard':
            require(row['worker_pointer'] == 0 and row['controller_initialized_raw'] == 0,
                    'passive worker/controller guard')
            guard_count += 1
            if thread is None:
                thread = row['thread']
            require(row['thread'] == thread, 'guard thread mismatch')
        elif event == 'marker_checkpoint':
            require(row['thread'] == thread, 'marker thread mismatch')
            raw = bytes.fromhex(row['descriptor_raw80'])
            require(len(raw) == 80, 'marker descriptor extent')
            descriptor = dict(zip(('schema', 'event', 'occurrence', 'context', 'stream', 'buffer',
                                   'size', 'path', 'result', 'offset'), struct.unpack('<10Q', raw)))
            require(descriptor['schema'] == 1 and descriptor['stream'] > 0 and descriptor['context'] > 0,
                    'marker schema/pointers')
            # The frozen runner emits bind_output inside marker(13), before its
            # trailing human-readable stream_marker row. The actual descriptor
            # is already witnessed at this stop after disk close.
            if descriptor['event'] == 13:
                require(active is not None and active['kind'] == 'write'
                        and active['phases'][-1] == 12, 'file-close marker order')
                require(descriptor['occurrence'] == active['occurrence']
                        and descriptor['stream'] == active['ready']['stream']
                        and descriptor['offset'] == descriptor['size']
                        == outputs[active['ready']['path']]['bytes'] and descriptor['result'] == 0,
                        'file-close descriptor identity/extent')
                active['closed'] = True
        elif event == 'stream_marker':
            code, occurrence = row['marker_event'], row['occurrence']
            require(descriptor is not None, 'marker lacks register descriptor')
            for key in ('schema', 'context', 'stream', 'buffer', 'size', 'result', 'offset', 'occurrence'):
                require(row[key] == descriptor[key], 'marker descriptor mismatch: ' + key)
            require(code == descriptor['event'] and row['result'] == 0, 'marker result/event')
            descriptor = None
            if code in (1, 11):
                require(active is None and pending is None and group is None and occurrence not in streams,
                        'overlapping/reused stream lifetime')
                require(row['offset'] == 0 and row['size'] > 0, 'ready stream bounds')
                kind = 'read' if code == 1 else 'write'
                expected_path = config['lower_core_military_initialization_observer']['input_core_artifact']['path'] if kind == 'read' else row['path']
                require(row['path'] == expected_path, 'wrong input path')
                require(kind == 'read' or row['path'] in outputs, 'unknown output path')
                active = dict(occurrence=occurrence, kind=kind, ready=row, phases=[code], groups=[], closed=False)
                streams[occurrence] = active
            else:
                require(active is not None and group is None and occurrence == active['occurrence'], 'inactive stream marker')
                for key in ('context', 'stream', 'buffer', 'path'):
                    require(row[key] == active['ready'][key], 'stream lifetime identity changed')
                expected = [1, 2, 3] if active['kind'] == 'read' else [11, 12, 13, 14]
                require(code == expected[len(active['phases'])], 'stream phase order')
                require(row['offset'] == row['size'], 'terminal stream offset/size')
                active['phases'].append(code)
                if code == 2:
                    require(row['size'] == active['ready']['size'], 'input extent changed')
                elif code in (12, 13):
                    require(row['size'] == outputs[row['path']]['bytes'], 'output full-file extent')
                    if code == 13:
                        active['closed'] = True
                if code in (3, 14):
                    if code == 14:
                        require(all(g['binding']['occurrence_id'] in output_bindings for g in active['groups']),
                                'writer output binding absent before retirement')
                    pending, active = active, None
        elif event == 'stream_destroyed':
            require(pending is not None and active is None and descriptor is not None, 'unmatched destruction')
            value = row['marker']
            require(value == descriptor, 'destruction descriptor mismatch')
            require(value['event'] == pending['phases'][-1] + 1, 'wrong destruction event')
            for key in ('stream', 'buffer', 'context', 'occurrence'):
                require(value[key] == pending['ready'][key], 'destroyed lifetime mismatch')
            pending['destroyed'] = True
            descriptor, pending = None, None
        elif event == 'checkpoint':
            require(active is not None and row['thread'] == thread, 'checkpoint outside active stream/thread')
            key = (row['checkpoint'], row['frame_id'])
            require(not checkpoints[key] or row['checkpoint'] == 'hash_array_entry', 'duplicate checkpoint')
            checkpoints[key].append(row)
            registers, name = row['registers'], row['checkpoint']
            if name in SITES or name == 'initialize_return':
                site = SITES['read_initialized'] if name == 'initialize_return' else SITES[name]
                candidate = registers['pc'] - site
                require(bias is None or candidate == bias, 'original module bias/site mismatch')
                bias = candidate
            if name in ('read_entry', 'write_entry', 'initialize_entry', 'count_entry'):
                require(row['frame_id'] not in frames and row['actual_entry_return_pc'] > 0, 'frame entry identity')
                frames[row['frame_id']] = row
            elif name in ('read_return', 'write_return', 'initialize_return', 'count_return'):
                entry = frames.get(row['frame_id'])
                require(entry is not None and entry['checkpoint'] == name.replace('_return', '_entry'), 'return frame hierarchy')
                require(registers['pc'] == entry['actual_entry_return_pc']
                        and registers['rsp'] == entry['registers']['rsp'] + 8, 'native return PC/RSP')
        elif event == 'owner_installation_witness':
            require(group is not None and group['binding']['kind'] == 'read' and witness is None,
                    'owner installation outside Read')
            require(row['read_frame'] == group['root_frame'] and row['actual_Player_argument'] == row['stored_Player_pointer'],
                    'owner installation identity')
            witness = row
        elif event in ENGINE - {'observer_begin', 'capture_end', 'saved_output_binding'}:
            require(active is not None and row['thread'] == thread, 'engine event outside stream/thread')
            if event in ('read_entry', 'write_entry'):
                require(group is None, 'overlapping root calls')
                binding = row['binding']
                validate_binding(binding, event.split('_')[0])
                require(binding['occurrence_id'] not in groups, 'duplicate Military occurrence')
                require(binding['stream_occurrence'] == active['occurrence']
                        and binding['stream_pointer'] == active['ready']['stream']
                        and binding['kind'] == active['kind'], 'root stream binding')
                require(binding['lifecycle_id'] == 'context-' + hex(active['ready']['context']), 'context binding')
                require(binding['owner_lifetime_kind'] == 'selected_original_Read_or_Write_frame_only', 'unexpected lifetime scope')
                require(binding['configuration_sha256'] == binding['runtime_profile_sha256'] == profile
                        and binding['runner_source_sha256'] == source_digest, 'source/config binding')
                require(binding['rules_inputs_sha256'] == config['original_filesystem_profile']['full_content_input_gate_sha256']
                        and binding['rules_database_sha256'] is None and binding['live_database_snapshot_unavailable'] is True,
                        'rules scope identity')
                require(binding['input_core_sha256'] == config['lower_core_stream']['sha256']
                        and binding['input_save_sha256'] == config['saved_configuration_sha256']
                        and binding['input_core_kind'] == 'actual_MemoryIStream_payload'
                        and binding['input_save_kind'] == 'whole_file_supplied_to_saved_configuration_loader'
                        and binding['save_to_core_extraction_binding_proven'] is False, 'Save/Core scope')
                require(binding['dlc_sha256'] == sha(canonical(config['dlc_descriptors'])), 'DLC identity')
                expected_mods = {'full_content_input_gate_sha256': binding['rules_inputs_sha256'],
                                 'saved_configuration_sha256': binding['input_save_sha256']}
                require(binding['mods_sha256'] == sha(canonical(expected_mods)), 'mod/input identity')
                group = dict(binding=binding, root_frame=row['frame_id'], rows=[], checkpoints=[], annotations=[])
                groups[binding['occurrence_id']] = group
                active['groups'].append(group)
                vectors(row['prior_vectors'])
            require(group is not None, 'engine event without root')
            if 'binding' in row:
                require(row['binding'] == group['binding'], 'within-call provenance changed')
            if event != 'read_initialized':
                checkpoint_rows = checkpoints.get((event, row['frame_id']))
                require(checkpoint_rows is not None, 'engine event lacks actual checkpoint')
                group['checkpoints'].append(checkpoint_rows[-1])
            group['rows'].append(row)
            if event == 'initialize_entry':
                require(witness is not None and witness['initialize_frame'] == row['frame_id'], 'missing installed owner witness')
                require(witness['military_pointer'] == group['binding']['military_pointer'], 'installed Military mismatch')
                group['annotations'].append(witness)
                witness = None
            if event in ('count_entry', 'count_return', 'hash_array_entry'):
                definition_order(row['definitions'])
                definitions.append(row['definitions'])
            if event in ('read_return', 'write_return'):
                require(row['normal_native_return'] is True and row['frame_id'] == group['root_frame'], 'normal root return')
                group = None
        elif event == 'original_stream_range':
            key = row['binding']['occurrence_id']
            require(key in groups and key not in ranges and row['binding'] == groups[key]['binding'], 'range occurrence')
            record = groups[key]
            require(row['frame_id'] == record['root_frame'] and row['kind'] == record['binding']['kind'], 'range role/frame')
            require(record['rows'][-1]['event'] == row['kind']+'_return', 'range before return')
            start, end = row['core_range']
            cp_entry, cp_return = record['checkpoints'][0], record['checkpoints'][-1]
            require(start == record['binding']['core_start'] == cp_entry['stream_offset']
                    and end == cp_return['stream_offset'] and 0 <= start < end <= active['ready']['size'], 'range offsets')
            record['range'], ranges[key] = row['core_range'], row
        elif event == 'saved_output_binding':
            key = row['occurrence_id']
            require(active is not None and active['kind'] == 'write' and active['closed'], 'output bound before file close')
            require(key in ranges and key not in output_bindings, 'missing/duplicate output occurrence')
            record = groups[key]
            require(record['binding']['kind'] == 'write' and row['output_id'] == record['binding']['output_id'] == active['ready']['path'],
                    'wrong output stream occurrence')
            require(row['saved_output_sha256'] == row['output_core_sha256'] == outputs[row['output_id']]['sha256']
                    and row['military_core_range'] == ranges[key]['core_range'], 'output artifact/range hash mismatch')
            output_bindings[key] = row
        elif event == 'final_live_original_method_fingerprints_equal':
            require(active is pending is group is None and row['method_count'] == 34, 'final original fingerprints')
    require(active is pending is group is witness is descriptor is None, 'unfinished native lifetime/call')
    require(counts['final_live_original_method_fingerprints_equal'] == 1, 'missing final method witness')
    require(guard_count == counts['checkpoint'] + counts['marker_checkpoint'], 'unguarded stop')
    require(len(ranges) == len(groups), 'missing original stream ranges')
    require(all(stream.get('destroyed') for stream in streams.values()), 'missing physical stream destruction')
    summaries = []
    for record in groups.values():
        summaries.append(validate_group(record, checkpoints, bias))
    for stream in streams.values():
        ordered = [record['range'] for record in stream['groups']]
        require(all(left[1] <= right[0] for left, right in zip(ordered, ordered[1:])), 'overlapping/out-of-order Military ranges')
        owners = [item['owner_i32'] for item in summaries if item['stream_occurrence'] == stream['occurrence']]
        require(len(owners) == len(set(owners)), 'duplicate saved owner within stream')
    return dict(counts=dict(counts), streams=streams, records=summaries, definitions=definitions,
                guarded_stops=guard_count, engine_rows=len(engine_rows), thread=thread)


def validate_group(record, checkpoints, bias):
    rows, binding = record['rows'], record['binding']
    by_event = defaultdict(list)
    for row in rows:
        by_event[row['event']].append(row)
    def one(event):
        require(len(by_event[event]) == 1, 'missing/duplicate group event: '+event)
        return by_event[event][0]
    def checkpoint(event, frame):
        values = checkpoints[(event, frame)]
        require(len(values) == 1, 'nonunique checkpoint')
        return values[0]
    military = binding['military_pointer']
    kind = binding['kind']
    first, last = one(kind+'_entry'), one(kind+'_return')
    require(first['frame_id'] == last['frame_id'] == record['root_frame'], 'root frame changed')
    entry_cp = checkpoint(kind+'_entry', first['frame_id'])
    require(entry_cp['registers']['rdi'] == binding['stream_pointer']
            and entry_cp['registers']['rsi'] == military, 'root registers/receiver')
    owner_state = snapshot(last, military)
    player = int(owner_state['player_pointer'], 16)
    summary = dict(role=kind, stream_occurrence=binding['stream_occurrence'],
                   military_occurrence=binding['occurrence_id'], owner_i32=owner_state['player_id_i32'],
                   core_range=record['range'], frame_id=record['root_frame'],
                   owner_lifetime_scope=binding['owner_lifetime_kind'])
    if kind == 'read':
        init_entry = one('initialize_entry')
        init_return = one('initialize_return')
        initialized = one('read_initialized')
        prior = snapshot(init_entry, military, player)
        result = snapshot(init_return, military, player)
        require(prior['player_id_i32'] == result['player_id_i32'] == owner_state['player_id_i32'], 'saved owner identity changed')
        require(result == snapshot(initialized, military, player), 'paused init-return/read-initialized mismatch')
        require(first['prior_vectors'] == prior['vectors'], 'prior vectors changed before InitializeLists')
        require(initialized['frame_id'] == first['frame_id'], 'Read initialized parent frame')
        init_cp = checkpoint('initialize_entry', init_entry['frame_id'])
        require(init_cp['actual_entry_return_pc'] == bias+SITES['read_initialized']
                and init_cp['registers']['rdi'] == military and init_cp['registers']['rsi'] == player,
                'original InitializeLists caller/owner')
        require(init_cp['registers']['rsp'] == entry_cp['registers']['rsp'] - 0xf80, 'Read/Initialize nested stack extent')
        owner_witness = record['annotations'][0]
        require(owner_witness['actual_Player_argument'] == player, 'installed Player snapshot mismatch')
        personality = one('personality_return')['observations']['has_ai_personality']
        expected = ['read_entry', 'initialize_entry', 'personality_return']
        current_observations = init_return['observations']
        require(current_observations['has_ai_personality'] == personality, 'personality observations changed')
        require((checkpoint('personality_return', init_entry['frame_id'])['registers']['rax'] & 255) == int(personality),
                'native personality result mismatch')
        require(checkpoint('personality_return', init_entry['frame_id'])['registers']['rsp']
                == init_cp['registers']['rsp'] - 0x1b8, 'personality checkpoint stack extent')
        count, default, leader = None, None, None
        if personality:
            leader_row = one('leader_return')
            leader = leader_row['observations']['leader_i32']
            actual = checkpoint('leader_return', init_entry['frame_id'])['registers']['rax'] & 0xffffffff
            require(actual == (leader & 0xffffffff) and current_observations['leader_i32'] == leader, 'native leader result')
            expected.append('leader_return')
            query = None
            if leader != -1:
                query_row = one('query_entry')
                query = query_row['observations']['leader_query']
                require(current_observations['leader_query'] == query, 'query observations changed')
                query_cp = checkpoint('query_entry', init_entry['frame_id'])
                registers = query_cp['registers']
                require(registers['rdi'] == int(query['handler_pointer'], 16)
                        and registers['rsi'] == military+0x158, 'actual allowed-query receiver/destination')
                default = query['default_value_i32']
                require(type(default) is int and -2**31 <= default < 2**31, 'handler default i32')
                expected.append('query_entry')
            if leader == -1 or int(query['query_pointer'], 16) != 0:
                count_entry, count_return = one('count_entry'), one('count_return')
                require(count_entry['frame_id'] == count_return['frame_id'], 'count frame mismatch')
                require(count_entry['definitions'] == count_return['definitions'] == current_observations['active_definitions'],
                        'count definitions changed')
                count = count_return['count_u32']
                require(count == len(count_entry['definitions']['ordered']), 'native count/ordered definitions')
                cp = checkpoint('count_entry', count_entry['frame_id'])
                require(cp['registers']['rdi'] == int(count_entry['definitions']['collection_pointer'], 16), 'actual count receiver')
                if leader == -1:
                    require(cp['registers']['rsp'] == init_cp['registers']['rsp'] - 0x1c0, 'direct count nested stack extent')
                    require(cp['actual_entry_return_pc'] == bias+0x2145dd2
                            and cp['registers']['rbx'] == military+0x158, 'unknown-leader count caller')
                    for name in ('allowed', 'per_war'):
                        require(result['vectors'][name]['size_u32'] == count
                                and result['vectors'][name]['values_i32'] == [0]*count, 'unknown-leader zero assignment')
                else:
                    require(cp['registers']['rsp'] == query_cp['registers']['rsp'] - 0x40, 'query/count nested stack extent')
                    require(cp['actual_entry_return_pc'] == bias+0x203ac8b
                            and cp['registers']['rbx'] == int(query['handler_pointer'], 16)
                            and cp['registers']['rbp'] == military+0x158
                            and query['captured_definition_pointer'] == count_entry['definitions']['collection_pointer'],
                            'allowed-query count callback chain')
                require(result['vectors']['allowed']['size_u32'] == count, 'actual initialized allowed count')
                expected += ['count_entry', 'count_return']
            else:
                require(result['vectors']['allowed'] == prior['vectors']['allowed'], 'null-query prior allowed vector')
            size = result['vectors']['allowed']['size_u32']
            require(result['vectors']['current']['size_u32'] == size
                    and result['vectors']['current']['values_i32'] == [0]*size, 'current zero assignment')
        else:
            require(result['vectors'] == prior['vectors'], 'no-personality prior vectors')
        expected += ['initialize_return', 'read_initialized', 'read_return']
        require([row['event'] for row in rows] == expected, 'read event order/coverage')
        summary.update(allowed_initialized_size=result['vectors']['allowed']['size_u32'],
                       operation_type_arrays_present=result['vectors']['allowed']['size_u32'] != 0,
                       actual_has_ai_personality=personality, actual_leader_i32=leader,
                       actual_definition_count=count, observed_handler_default_i32=default,
                       initialized_vector_values_sha256=values_digest(result['vectors']),
                       post_read_vector_values_sha256=values_digest(owner_state['vectors']),
                       prior_vector_values_sha256=values_digest(prior['vectors']),
                       changed_after_read={name: result['vectors'][name]['values_i32'] != owner_state['vectors'][name]['values_i32'] for name in NAMES},
                       prior_sizes={name: prior['vectors'][name]['size_u32'] for name in NAMES},
                       initialized_sizes={name: result['vectors'][name]['size_u32'] for name in NAMES},
                       per_war_query_not_observed=True)
    else:
        guard = one('write_guard')
        entry = snapshot(first, military, player)
        require(entry == snapshot(guard, military, player) == owner_state, 'writer entry/guard/return snapshots changed')
        require(binding['player_pointer_at_entry'] == player, 'writer entry owner pointer')
        require(first['prior_vectors'] == entry['vectors'], 'writer entry prior snapshot mismatch')
        present = guard['operation_type_arrays_present']
        require(type(present) is bool and present == (entry['vectors']['allowed']['size_u32'] != 0), 'actual writer presence')
        guard_cp = checkpoint('write_guard', first['frame_id'])
        require(guard_cp['registers']['rbx'] == military
                and guard_cp['registers']['r14'] == binding['stream_pointer']
                and guard_cp['registers']['rax'] & 0xffffffff == entry['vectors']['allowed']['size_u32'], 'actual writer guard registers')
        arrays = by_event['hash_array_entry']
        require(len(arrays) == (3 if present else 0), 'writer array coverage')
        # One target PC/frame repeats three times; checkpoints need ordered keys.
        if arrays:
            require([row['name'] for row in arrays] == list(NAMES), 'writer array ordering')
            require(len(checkpoints[('hash_array_entry', first['frame_id'])]) == len(arrays), 'hash array checkpoint coverage')
            for row, cp in zip(arrays, checkpoints[('hash_array_entry', first['frame_id'])]):
                require(cp['registers']['rsp'] == entry_cp['registers']['rsp'] - 0x30, 'writer helper nested stack extent')
                require(cp['registers']['rdi'] == binding['stream_pointer']
                        and cp['registers']['rsi'] == int(row['vector']['data_pointer'], 16)
                        and cp['registers']['rdx'] == int(row['definitions']['collection_pointer'], 16),
                        'hash array actual stream/data/definition registers')
                require(row['vector'] == entry['vectors'][row['name']], 'writer actual helper values')
                require(row['definitions'] == arrays[0]['definitions'], 'writer actual definitions changed')
                require(row['vector']['size_u32'] >= len(row['definitions']['ordered']), 'short writer vector')
        require([row['event'] for row in rows] == ['write_entry', 'write_guard'] + ['hash_array_entry']*len(arrays) + ['write_return'],
                'write event order/coverage')
        summary.update(operation_type_arrays_present=present,
                       allowed_writer_size=entry['vectors']['allowed']['size_u32'],
                       writer_vector_values_sha256=values_digest(entry['vectors']))
    return summary


def audit_artifacts(directory):
    directory = Path(directory)
    raw = (directory/'observations.jsonl').read_bytes()
    rows = [json.loads(line) for line in raw.splitlines()]
    runtime = json.loads((directory/'runtime.json').read_text())
    prepared = json.loads((directory/'runtime-prepared.json').read_text())
    plan = json.loads((directory/'runtime-plan.json').read_text())
    cfg = runtime['configuration']
    profile = sha(canonical(cfg))
    require(profile == runtime['configuration_sha256'] == prepared['configuration_sha256']
            and cfg == prepared['configuration'], 'prepared/actual runtime configuration drift')
    validate_runtime_profile(cfg)
    require(runtime['returncode'] == runtime['native_boundary_result'] == 0
            and runtime['fixture_success'] is True, 'original runtime result')
    require(runtime['framework_cleanup_proven'] and runtime['client_cleanup_proven']
            and runtime['framework_boundary_returned'], 'original cleanup')
    stages = runtime['lifecycle_stages']
    require(stages['native_core_load_completed'] and stages['native_core_load_result'] == 0
            and stages['game_startup_returned'] and stages['game_shutdown_returned']
            and stages['controller_cache_initialized'] is False
            and stages['controller_uninitialized_cleanly'] is False, 'original lifecycle stages')
    exit_report = json.loads((directory/'exit.json').read_text())
    supervisor = json.loads((directory/'supervisor.json').read_text())
    require(exit_report == dict(inferior_exitcode=0, observer_failed=False), 'native exit status')
    require(supervisor['terminal'] == dict(inferior_exitcode=0, observer_failed=False,
                gdb_returncode=0, remaining_session_processes=[]), 'supervisor terminal/session state')
    artifacts = {}
    for name, expected in runtime['military_observer_artifacts'].items():
        actual = file_identity(directory/name)
        require(actual == expected, 'closed artifact identity: '+name)
        artifacts[name] = actual
    artifacts['runtime.json'] = file_identity(directory/'runtime.json')
    header = rows[0]
    require(header['gamecore_sha256'] == plan['gamecore_sha256'] == GAME_SHA
            and header['host_sha256'] == plan['host_sha256'] == HOST_SHA, 'original target identities')
    require(file_identity(plan['module'])['sha256'] == GAME_SHA
            and file_identity(plan['host'])['sha256'] == HOST_SHA, 'supplied original module/host')
    require(header['plan_sha256'] == sha(canonical(plan)) and header['runtime_gate'] == plan['runtime_gate'], 'raw source plan binding')
    gate = plan['runtime_gate']
    require(file_identity(directory/'runtime-prepared.json')['sha256'] == gate['report_sha256']
            and gate['configuration_sha256'] == profile, 'prepared source/profile gate')
    archive = directory/'source-preimages.tar.gz'
    source_map = gate['source_files']
    require(header['observer_sha256'] == plan['observer_sha256'], 'observer source identity')
    source_names = {}
    with tarfile.open(archive, 'r:gz') as package:
        members = {member.name: member for member in package.getmembers()}
        require(all(member.isfile() for member in members.values()), 'unexpected archive member type')
        for absolute, expected in source_map.items():
            # Captured gate paths are absolute checkout paths; use the immutable
            # archive preimages, never silently replace them with today's tree.
            matches = [name for name in members if str(absolute).endswith('/'+name.removeprefix('source/'))]
            require(len(matches) == 1, 'ambiguous captured source suffix')
            name = matches[0]
            relative = name.removeprefix('source/')
            require(name in members, 'missing captured source: '+relative)
            stream = package.extractfile(members[name])
            require(stream is not None and sha(stream.read()) == expected, 'captured source identity: '+relative)
            require(file_identity(directory/name)['sha256'] == expected, 'unpacked source/archive mismatch')
            source_names[relative] = expected
        require(set(members) == {'source/'+name for name in source_names}, 'unbound source preimage')
    source_digest = sha(canonical(source_map))
    require(source_names['tools/linux/military_initialization_observer.py'] == header['observer_sha256'], 'captured observer preimage')
    profile_config = cfg['lower_core_military_initialization_observer']
    inputs = {}
    for kind in ('save', 'core'):
        item = profile_config['input_'+kind+'_artifact']
        identity = file_identity(item['path'])
        require(identity == {key: item[key] for key in ('sha256', 'bytes')}, 'actual input artifact: '+kind)
        inputs[kind] = dict(**identity, scope=item['kind'])
    require(profile_config['save_to_core_extraction_binding_proven'] is False, 'unexpected extraction claim')
    outputs = {}
    for role, artifact in runtime['game_saves'].items():
        path = runtime['execution_paths']['game_outputs'][role]
        identity = file_identity(path)
        require(identity == {key: artifact[key] for key in ('sha256', 'bytes')}, 'actual full output: '+role)
        outputs[path] = identity
    facts = validate_events(rows, cfg, profile, source_digest, outputs)
    require(len(facts['streams']) == 3 and Counter(item['kind'] for item in facts['streams'].values()) == {'read': 1, 'write': 2},
            'reviewed ordinary one-Load/two-Save profile')
    rng = [row for row in rows if row['event'] == 'known_rng_after_load']
    require(len(rng) == 1 and len(bytes.fromhex(rng[0]['raw64'])) == 64, 'retained post-Load known RNG witness')
    return dict(rows=rows, runtime=runtime, plan=plan, config=cfg, profile=profile, artifacts=artifacts,
                source_names=source_names, source_archive=file_identity(archive), inputs=inputs,
                outputs=outputs, facts=facts, source_digest=source_digest,
                post_load_rng64_sha256=sha(bytes.fromhex(rng[0]['raw64'])))


def mutation_tests(capture):
    """Corrupt copies in memory only; native capture/artifacts remain untouched."""
    rows = capture['rows']
    tests = []
    def corrupt(label, change):
        candidate = copy.deepcopy(rows)
        change(candidate)
        try:
            validate_events(candidate, capture['config'], capture['profile'], capture['source_digest'], capture['outputs'])
        except (ValueError, KeyError, IndexError, TypeError):
            tests.append(label)
            return
        raise ValueError('mutation unexpectedly accepted: '+label)
    def first(values, event):
        return next(row for row in values if row['event'] == event)
    corrupt('missing initialization return', lambda values: values.remove(first(values, 'initialize_return')))
    corrupt('wrong stream identity', lambda values: first(values, 'read_entry')['binding'].__setitem__('stream_pointer', 1))
    corrupt('return stack mismatch', lambda values: first(values, 'checkpoint')['registers'].__setitem__('rsp', 1))
    corrupt('nested frame extent mismatch', lambda values: next(row for row in values if row['event'] == 'checkpoint' and row['checkpoint'] == 'query_entry')['registers'].__setitem__('rsp', 1))
    corrupt('wrong active definition count', lambda values: first(values, 'count_return').__setitem__('count_u32', 0))
    corrupt('changed initialized snapshot', lambda values: first(values, 'read_initialized')['snapshot']['vectors']['allowed']['values_i32'].__setitem__(0, -999))
    corrupt('wrong output hash', lambda values: first(values, 'saved_output_binding').__setitem__('output_core_sha256', '0'*64))
    corrupt('wrong output range', lambda values: first(values, 'saved_output_binding')['military_core_range'].__setitem__(1, 1))
    corrupt('wrong actual owner', lambda values: first(values, 'initialize_entry')['snapshot'].__setitem__('player_pointer', '0x1'))
    corrupt('worker present', lambda values: first(values, 'passive_owner_guard').__setitem__('worker_pointer', 1))
    corrupt('missing destruction', lambda values: values.remove(first(values, 'stream_destroyed')))
    corrupt('spoofed runtime profile', lambda values: first(values, 'read_entry')['binding'].__setitem__('runtime_profile_sha256', '0'*64))
    corrupt('unclassified DB identity', lambda values: first(values, 'read_entry')['binding'].__setitem__('live_database_snapshot_unavailable', False))
    return tests


def build_report(capture, tests):
    facts = capture['facts']
    defs = facts['definitions']
    portable_defs = [{key: value for key, value in row.items() if key != 'pointer'} for row in defs[0]['ordered']]
    ordered_hashes = {sha(canonical([{key: value for key, value in row.items() if key != 'pointer'} for row in item['ordered']])) for item in defs}
    require(len(ordered_hashes) == 1, 'observed active definition ordering changed')
    streams = []
    for occurrence, stream in facts['streams'].items():
        records = [record for record in facts['records'] if record['stream_occurrence'] == occurrence]
        identity = capture['inputs']['core'] if stream['kind'] == 'read' else capture['outputs'][stream['ready']['path']]
        streams.append(dict(occurrence=occurrence, role=stream['kind'], core_sha256=identity['sha256'],
                            core_bytes=identity['bytes'], physical_lifetime_destroyed=stream['destroyed'],
                            marker_phases=stream['phases'], record_count=len(records),
                            observed_owner_ids=[record['owner_i32'] for record in records],
                            records=records))
    reads = [record for record in facts['records'] if record['role'] == 'read']
    writes = [record for record in facts['records'] if record['role'] == 'write']
    report = dict(schema='c6-linux-military-initialization-capture-classification-v1', checks_passed=True,
                  audit_sha256=file_identity(__file__)['sha256'],
                  artifacts=capture['artifacts'], source_preimages_archive=capture['source_archive'],
                  frozen_source_preimages=capture['source_names'],
                  gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA, runtime_profile_sha256=capture['profile'],
                  raw_jsonl_rows=len(capture['rows']), raw_engine_rows=facts['engine_rows'], event_counts=facts['counts'],
                  guarded_paused_stops=facts['guarded_stops'], source_profile_gate_rechecked=True,
                  offline_mutation_refusals=tests, original_execution_by_this_audit=False,
                  original_capture_closed_exit_zero=True, source_witness_final_live_method_count=34,
                  raw_capture_complete_flag=False, typed_snapshot_admission_performed=False,
                  candidate_execution=False, candidate_comparison=False,
                  inputs=capture['inputs'], save_to_core_extraction_binding_proven=False,
                  streams=streams, definitions=dict(observations=len(defs), ordered=portable_defs,
                      portable_order_sha256=next(iter(ordered_hashes))),
                  summary=dict(reads=len(reads), writes=len(writes),
                      input_initialized_allowed_sizes=dict(Counter(str(row['allowed_initialized_size']) for row in reads)),
                      input_actual_leader_indices=dict(Counter(str(row['actual_leader_i32']) for row in reads)),
                      input_handler_defaults=dict(Counter(str(row['observed_handler_default_i32']) for row in reads)),
                      input_actual_personality_true=sum(row['actual_has_ai_personality'] for row in reads),
                      input_nonempty_allowed=sum(row['operation_type_arrays_present'] for row in reads),
                      writer_nonempty_allowed=sum(row['operation_type_arrays_present'] for row in writes),
                      vectors_changed_by_saved_Read={name: sum(row['changed_after_read'][name] for row in reads) for name in NAMES}),
                  known_rng=dict(post_load_raw64_sha256=capture['post_load_rng64_sha256'],
                      post_load_bytes=64, save_endpoint_independent_raw_dumps=0,
                      save_endpoint_equality='source-guard witness at each actual Save enter/return; no independently emitted endpoint dumps'),
                  limits=['role-local selected original frames, no proof of owner persistence across streams',
                          'no incoming ff31 Read or initialized ff31 context captured',
                          'per-war query execution/preparation/rows not separately observed; full actual vector snapshots retained',
                          'rules input/config/content/DLC/mod identities; live gameplay DB byte snapshot unavailable',
                          'whole Save and actual MemoryIStream payload identities remain separately scoped',
                          'no outer Manager ordinal/requested-ID mapping inferred from owner/order/count',
                          'no runtime policy-call counters; source-only ordinary Load/Save profile',
                          'complete gameplay state/RNG/actions/turns remain unsupported'],
                  recorded_raw_save_load_difference_bytes=89198, unknown_save_bytes_masked=False)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    capture = audit_artifacts(args.directory)
    tests = mutation_tests(capture)
    report = build_report(capture, tests)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True, raw_rows=report['raw_jsonl_rows'],
                         reads=report['summary']['reads'], writes=report['summary']['writes'],
                         mutation_refusals=len(tests), candidate_comparison=False)))


if __name__ == '__main__':
    main()
