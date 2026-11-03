#!/usr/bin/env python3
"""Independent bounded closed native-2 review; never executes original code."""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import struct
from unittest import mock

import current_handler_capture_audit as metadata
import full_manager_observer_runner as observer
from save_lifecycle_audit import Elf, GAME_SHA

PROFILE = '27ee77e3bf3aed48f957e13d41a314cf7a9c413f5ccb0e1793fe7d4826987116'
RUNTIME = '255603fc41c1768a5da113beeb94b14aa28010da837e71bb9540fd7b16028903'
SEAL = '449c875d1f45d01257ddfc59ded6b1690f10e9a2c76132f9171903a599575822'
CHECKPOINTS = ['marker'] * 4 + ['manager_entry', 'dispatch', 'ccall_entry', 'acquired',
    'resolved', 'protected_return', 'before_release', 'released', 'wrapper_return',
    'manager_return'] + ['marker'] * 5


def require(ok, why):
    if not ok:
        raise ValueError(why)


def digest(path):
    result = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            result.update(block)
    return result.hexdigest()


def rows(path):
    return [metadata.load(line) for line in Path(path).read_bytes().splitlines()]


def validate_events(events, plan, predicate):
    points = [r for r in events if r['event'] == 'checkpoint']
    require([r['checkpoint'] for r in points] == CHECKPOINTS, 'checkpoint order/coverage')
    markers = [r for r in events if r['event'] == 'fixture_marker']
    require([r['phase'] for r in markers] == list(range(1, 10)), 'marker order/coverage')
    main = [r for r in events if r['event'] == 'actual_main_entry_thread_witness']
    require(len(main) == 1, 'main witness')
    tid = main[0]['os_main_tid']
    require(all(r['thread'][:2] == [tid, tid] for r in points), 'selected main thread')
    biases = {}
    for point in points:
        require(point['frames'][0]['pc'] == point['registers']['pc'], 'stopped frame/register PC')
        for frame in point['frames']:
            if frame['module'] in plan['modules']:
                bias = frame['pc'] - frame['relative_pc']
                require(biases.setdefault(frame['module'], bias) == bias, 'retained frame load bias')
    expected = plan['caller_suffix_frames']
    byname = {r['checkpoint']: r for r in points if r['checkpoint'] != 'marker'}
    for point, marker in zip([r for r in points if r['checkpoint'] == 'marker'], markers):
        require(observer.descriptor(bytes.fromhex(marker['raw88'])) ==
                {k: v for k, v in marker.items() if k != 'event'}, 'raw descriptor projection')
        require(any(len(point['frames']) >= len(chain) and all(observer.same_frame(a, b)
                    for a, b in zip(point['frames'], chain))
                    for chain in plan['marker_parent_frames'][str(marker['phase'])]), 'actual marker own prefix')
        require(marker['actor_owner'] == 0 and marker['actor_full_id'] == 15794187 and
                marker['visible_only'] is False, 'actor identity')
    base = markers[0]['core_base']
    require(biases[plan['core']] == base and biases[plan['host']] == 0, 'original frame module bases')
    for point in points:
        name = point['checkpoint']; pc = point['registers']['pc']
        if name in observer.SITES:
            require(pc == observer.SITES[name] + (base if name in observer.GAME_SITES else 0),
                    'retained actual known checkpoint PC')
    entry, returned = byname['manager_entry'], byname['manager_return']
    require(entry['registers']['pc'] == base + 0x2da20f0, 'actual Manager entry PC')
    require(returned['registers']['pc'] == returned['frames'][0]['pc'], 'actual Manager return PC')
    guard = observer.Frames(expected, plan['core'], base, allow_unresolved_tail=True)
    guard.manager_entry(entry['frames']); guard.caller(returned['frames'])
    r = entry['registers']; marker = markers[3]
    require([r['rdi'], r['rsi'], r['rdx'], r['rcx'], r['r8'] & 255, r['r9']] ==
            [marker['manager'], marker['unit'], marker['operation'], 0, 0, 0], 'actual Manager ABI')
    require(returned['registers']['rsp'] == r['rsp'] + 8, 'Manager return stack')
    actual = [r for r in events if r['event'] == 'actual_Manager_return']
    require(len(actual) == 1 and actual[0]['allowed'] == bool(returned['registers']['rax'] & 255), 'actual AL projection')
    require(actual[0]['allowed'] is True and predicate[-1]['allowed'] is True and
            predicate[-1]['predicate_invoked'] and predicate[-1]['predicate_returned'], 'fixture versus actual return')
    require(all(m['predicate_returned'] == (m['phase'] >= 5) and
                m['allowed'] == (m['phase'] >= 5) for m in markers), 'marker return state')
    require(markers[-1]['facade'] == 0 and all(m['facade'] for m in markers[:-1]), 'facade release framing')
    call = [r for r in events if r['event'] == 'raw_actual_CallTestAll_return']
    require(len(call) == 1, 'natural wrapper return coverage')
    call = call[0]
    dispatch, ccall = byname['dispatch']['registers'], byname['ccall_entry']['registers']
    require(dispatch['rsp'] == r['rsp'] - 152 and ccall['rsp'] == dispatch['rsp'] - 72,
            'selected outer Manager/CCall stack')
    require(call['wrapper_rsp'] == dispatch['rsp'] and call['entry_rsp'] == ccall['rsp'] and
            call['state'] == ccall['rdi'] and call['context'] == ccall['rdx'] and
            call['event_pointer'] == dispatch['r12'], 'retained CCall context identity')
    require(call['return_bit'] == bool(byname['wrapper_return']['registers']['rax'] & 255), 'wrapper AL projection')
    require(call['protected_status_raw_u32'] == byname['protected_return']['registers']['rax'] & 0xffffffff,
            'protected status projection')
    require(call['return_bit'] is False and call['out_value_raw'] == '00' and
            call['protected_status_raw_u32'] == 0 and call['protected_success_raw_u8'] == 1,
            'bounded raw wrapper outcome')
    terminal = [r for r in events if r['event'] == 'raw_trace_terminal']
    require(len(terminal) == 1 and terminal[0]['protocol_complete'], 'known native protocol closure')
    require(events[-1]['event'] == 'adapter_terminal' and events[-1]['natural_Lua_calls'] == 1 and
            events[-1]['observation_mode'] == 'raw_unresolved', 'adapter terminal')
    require(not any(r.get(k, False) for r in events for k in
            ['callback_absence_proven', 'registry_decoded', 'registry_images_captured',
             'callback_effects_accepted', 'native_parity_accepted']), 'unsupported completeness claim')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--capture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args(); root = args.capture
    require(digest(root / 'closed-files.json') == SEAL, 'sealed manifest identity')
    seal = metadata.load((root / 'closed-files.json').read_bytes())
    for name, identity in seal['files'].items():
        path = root / name
        require(path.stat().st_size == identity['bytes'] and digest(path) == identity['sha256'], 'sealed file changed: ' + name)
    require(digest(root / 'runtime.json') == RUNTIME, 'runtime identity')
    runtime = metadata.load((root / 'runtime.json').read_bytes())
    prepared = metadata.load((root / 'prepared.json').read_bytes())
    require(runtime['configuration'] == prepared['configuration'] and
            runtime['configuration_sha256'] == PROFILE and runtime['returncode'] == 0, 'runtime profile/status')
    plan = metadata.load((root / 'observer-plan.json').read_bytes())
    events = rows(root / 'observer/capture/observations.jsonl'); predicate = rows(root / 'predicate.jsonl')
    validate_events(events, plan, predicate)
    locks = [r for r in predicate if r['kind'] == 'core_lock_endpoint']
    require([r['phase'] for r in locks] == list(range(1, 10)) and
            all(struct.unpack('<II', bytes.fromhex(r['owner_u32_depth_u32_raw'])) ==
                ((1, 1) if r['phase'] < 9 else (0, 0)) for r in locks), 'raw core lock endpoints')
    mutations = [lambda x: x.pop(),
        lambda x: next(r for r in x if r['event'] == 'actual_Manager_return').__setitem__('allowed', False),
        lambda x: next(r for r in x if r['event'] == 'checkpoint' and r['checkpoint'] == 'manager_return')['registers'].__setitem__('rax', 0),
        lambda x: next(r for r in x if r['event'] == 'fixture_marker').__setitem__('actor_full_id', 1),
        lambda x: next(r for r in x if r['event'] == 'checkpoint' and r['checkpoint'] == 'ccall_entry')['registers'].__setitem__('rsp', 1),
        lambda x: next(r for r in x if r['event'] == 'checkpoint' and r['checkpoint'] == 'manager_entry')['frames'][0].__setitem__('pc', 1),
        lambda x: next(r for r in x if r['event'] == 'raw_trace_terminal').__setitem__('protocol_complete', False)]
    for mutate in mutations:
        changed = copy.deepcopy(events); mutate(changed)
        try:
            validate_events(changed, plan, predicate)
        except ValueError:
            pass
        else:
            raise ValueError('malformed retained observer rows accepted')
    core = Elf(Path(plan['core']), GAME_SHA)
    streams = [root / ('metadata-' + phase + '.jsonl') for phase in ['before', 'control', 'after', 'repeat']]
    require(all(path.read_bytes() == streams[0].read_bytes() for path in streams), 'metadata differences')
    with mock.patch.object(metadata, 'PROFILE', PROFILE):
        facts = rows(streams[0]); roster = metadata.validate_facts(facts, core)
    device = facts[0]['file_device']
    require([os.major(device), os.minor(device)] == plan['modules'][plan['core']]['device'] and
            facts[0]['file_inode'] == plan['modules'][plan['core']]['inode'], 'raw metadata original file binding')
    require(runtime['framework_cleanup_proven'] and runtime['client_cleanup_proven'] and
            runtime['lifecycle_stages']['controller_uninitialized_cleanly'], 'original cleanup markers')
    require(all(r['pid_absent'] and r['original_generation_absent'] and
                not Path('/proc', str(r['pid'])).exists() for r in seal['process_generation_check']), 'closed process generations')
    result = dict(schema='c6-linux-full-manager-closed-independent-checks-v1', original_execution=False,
        runtime_sha256=RUNTIME, sealed_files=len(seal['files']), observer_rows=len(events),
        strict_roster=roster, mutation_refusals=len(mutations), actual_Manager_allowed=True,
        natural_CallTestAll_return_bit=False, complete_gameplay_or_callback_parity=False)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps(dict(checks='PASS', sealed_files=len(seal['files']), mutation_refusals=len(mutations))))


if __name__ == '__main__':
    main()
