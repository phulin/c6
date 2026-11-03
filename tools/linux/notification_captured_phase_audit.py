#!/usr/bin/env python3
"""Replay production notification phase from closed passive native control reads.

No original code runs; raw captures remain private. A complete native first-Find
wire record and full event/pool live state are not inferred from read coverage.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace

from notification_live_observe import check_capture
from notification_state_phase_audit import audit as phase_audit, ROOT, require, sha
from core_component_check import validate as validate_component

OBSERVATION_CONFIG = 'f3384ec474685c1b1cc23c1f4a7906ecf2ff7ce2f35bee433750a4046c9338f7'
PARENT_CONFIG = 'b6ca58d53073abc2afa9176a4ceba350fafe3f91ba12dbf84fd08e25a3be9a5b'


def read_json(path):
    return json.loads(path.read_text())


def logical(handle):
    require(isinstance(handle, str) and handle.startswith('object-'), 'unknown logical handle')
    return int(handle.removeprefix('object-'))


def identity(record):
    bits = int(record['identity_bits'], 16)
    owner, component = bits & 65535, (bits >> 16) & 65535
    owner = owner - 65536 if owner >= 32768 else owner
    component = component - 65536 if component >= 32768 else component
    return owner, component, bits >> 32


def serialized_control(record):
    owner, component, id_ = identity(record)
    return dict(owner=owner, component_type=component, id=id_, type_hash=int(record['type_hash'], 16),
                severity=record['severity'], dismissed=record['dismissed'], activated=record['activated'],
                native_auto_activate=record['native_auto_activate'], native_is_expired=record['native_is_expired'])


def reference_control(record):
    return dict(owner=record['owner'], component_type=record['component_type'], id=record['id'],
                type_hash=record['type_hash'], severity=record['severity'], dismissed=record['is_dismissed'],
                activated=record['has_been_activated'], native_auto_activate=bool(record['native_19d_19e'][0]),
                native_is_expired=bool(record['native_19b_19c'][1]))


def scans_from_rows(rows):
    scans, active, eligibility = [], None, None
    for row in rows:
        if row['stage'] == 'find_entry':
            active = dict(entry=row, eligibility=[])
            scans.append(active)
        elif row['stage'] == 'auto_entry':
            require(active is not None, 'eligibility outside Find')
            eligibility = dict(entry=row)
            active['eligibility'].append(eligibility)
        elif row['stage'] in ('auto_network', 'definition', 'icon', 'auto_exit', 'severity'):
            require(eligibility is not None, 'missing IsAutoActivate entry')
            eligibility[row['stage']] = row
        elif row['stage'] == 'selected_return':
            active['selected'] = row
            active = None
            eligibility = None
    return scans


def derive(capture, static):
    controls = read_json(capture / 'validation.json')
    metadata = read_json(capture / 'metadata.json')
    manifest = read_json(capture / 'manifest.json')
    require(controls['native_exit'] == controls['gdb_exit'] == 0 and controls['all_five_save_captures_and_command_match_parent'], 'native exit/control failure')
    require(read_json(capture / 'exit.json')['inferior_exitcode'] == 0, 'inferior did not exit normally')
    require(metadata['gdb_returncode'] == 0 and metadata['inferior_exit']['inferior_exitcode'] == 0, 'driver exit mismatch')
    require(metadata['observation_configuration_sha256'] == controls['observation_configuration_sha256'] == OBSERVATION_CONFIG, 'observation configuration mismatch')
    canonical = json.dumps(metadata['observation_configuration'], sort_keys=True, separators=(',', ':')).encode()
    require(sha(canonical) == OBSERVATION_CONFIG, 'observation configuration bytes mismatch')
    require(manifest['configuration_sha256'] == metadata['parent_configuration_sha256'] == controls['parent_configuration_sha256'] == PARENT_CONFIG, 'parent configuration mismatch')
    require(sha((capture / 'observations.jsonl').read_bytes()) == metadata['observations_sha256'], 'observation bytes mismatch')
    require(sha((capture / 'manifest.json').read_bytes()) == metadata['notification_manifest_sha256'], 'manifest mismatch')
    require(sha((ROOT / 'tools/linux/notification_live_observe.py').read_bytes()) == metadata['observer_sha256'], 'observer source mismatch')
    with tempfile.TemporaryDirectory(prefix='c6-native-notification-check-') as tmp:
        output = Path(tmp) / 'check.json'
        check_capture(capture / 'manifest.json', output)
        checked = read_json(output)
    component, _ = validate_component(capture / 'runtime.json')
    require(component['checks_passed'] and component['conditional_component_completed'] and component['native_cleanup_proven'], 'native component incomplete')
    require(component['configuration_sha256'] == PARENT_CONFIG, 'component parent mismatch')
    rows = [json.loads(line) for line in (capture / 'observations.jsonl').read_text().splitlines()]
    scans = scans_from_rows(rows)
    require(scans, 'no actual Find entry')
    first = scans[0]['entry']
    before = next(r['manager'] for r in rows if r['stage'] == 'update_entry')
    after = next(r['manager'] for r in rows if r['stage'] == 'update_return')
    cloud = next(r for r in rows if r['stage'] == 'cloud')
    require(before['game_present'] and not before['game_uninitializing'], 'missing actual manager game/gate')
    gate_proof = first['entering_branch_gates']
    require(gate_proof['accepts_notifications'] and gate_proof['initialized'] and gate_proof['human_configuration']
            and gate_proof['owner_turn_active'] and gate_proof['game_uninitializing'] is False, 'activation entry gate proof missing')
    require(before['player_count'] > 0 and before['selected']['owner'] == first['list']['owner'] == after['selected']['owner'], 'owner/init mismatch')
    require(cloud['game_present'] == before['game_present'], 'cloud game pointer mismatch')
    if cloud['play_by_cloud']:
        local = next(r for r in rows if r['stage'] == 'local')
        require(local['local_player'] == local['owner'] == first['list']['owner'], 'cloud activation not permitted')
    require(first['threshold'] == (3 if first['network_multiplayer'] else 0), 'network threshold mismatch')
    lines = ['mode undelivered_projection', 'gates {} {} {} {} {} {} {} {} v1'.format(
        int(before['player_count'] > 0), int(before['game_uninitializing']),
        int(first['entering_branch_gates']['human_configuration']), int(first['entering_branch_gates']['owner_turn_active']),
        int(first['network_multiplayer']), int(cloud['play_by_cloud']), int(before['game_present']), before['local_player'])]
    pools, definition_inputs, live_metadata = {}, {}, []
    profiles = {r['handle']:r['slots'] for r in first['list']['records'] if r is not None}
    observed_profile_checks = 0
    for row in rows:
        snapshots = []
        if 'manager' in row:
            snapshots += row['manager']['selected']['records']
        if 'list' in row:
            snapshots += row['list']['records']
        for key in ('instance', 'selected', 'released'):
            if isinstance(row.get(key), dict):
                snapshots.append(row[key])
        for snapshot in snapshots:
            require(snapshot is not None and snapshot['slots'] == profiles[snapshot['handle']], 'branch-time virtual profile changed')
            observed_profile_checks += 1
    for record, branch in zip(first['list']['records'], scans[0]['eligibility'], strict=True):
        require(record is not None and branch['entry']['instance'] == record, 'first Find entry/control state changed before eligibility')
        definition = '?'
        if record['activated']:
            require('definition' not in branch and 'auto_network' not in branch, 'activated short circuit unexpectedly read definition')
        else:
            require(branch['auto_network']['network'] == first['network_multiplayer'], 'network getter mismatch')
            if not first['network_multiplayer']:
                actual = branch['definition']
                definition = '-' if not actual['definition_found'] else str(int(branch['icon']['show_icon_single_player']))
                if actual['definition_found']:
                    require(actual['icon_flag_bit'] == branch['icon']['show_icon_single_player'], 'definition bit/getter mismatch')
        definition_inputs[record['handle']] = definition
        slots = record['slots']
        require(any(all(table['slots'][name] == value for name, value in slots.items()) for table in static['virtual_bindings']), 'observed virtual profile not in target contract')
        activation = 'base' if slots['activate'] == '0x2d2dd80' else 'unsupported'
        release = 'base' if slots['release'] == '0x2d2de30' else 'unsupported'
        destruction = 'unsupported' if any(table['slots']['deleting_destructor'] == slots['deleting_destructor'] and '11PromoteUnitE' in table['symbol'] for table in static['virtual_bindings']) else 'base'
        pool = record['pool']
        if pool is not None:
            if pool in pools:
                require(pools[pool] == record['pool_order'], 'inconsistent same-pool snapshots')
            pools[pool] = record['pool_order']
        owner, kind, id_ = identity(record)
        lines.append('record {} {} {} {} {} {} {} {} {} {}'.format(logical(record['handle']), record['lock_count'], '-' if pool is None else logical(pool), definition, activation, release, destruction, owner, kind, id_))
        live_metadata.append(dict(object=logical(record['handle']), lock_count=record['lock_count'],
                                  pool=None if pool is None else logical(pool), definition_input=definition,
                                  observed_slots=slots, definition_provenance='short-circuited/unobserved' if definition == '?' else 'actual first-scan getter path'))
    for pool, prefix in pools.items():
        lines.append('pool ' + ' '.join(str(v) for v in [logical(pool)] + [logical(obj) for obj in prefix]))
    return '\n'.join(lines) + '\n', dict(rows=rows, scans=scans, first=first, before=before, after=after,
                                             pools=pools, definition_inputs=definition_inputs, live_metadata=live_metadata,
                                             observer_check=checked, component=component, controls=controls, metadata=metadata,
                                             observed_profile_checks=observed_profile_checks)


def compare_replay(result, captured, reference):
    first, scans = captured['first'], captured['scans']
    expected_before = reference['notification_container']['before']
    expected_after = reference['notification_container']['after']
    require(first['list']['next_id'] == expected_before['header']['next_id'], 'first Find next ID differs from serializer baseline')
    for record, wire in zip(first['list']['records'], expected_before['records'], strict=True):
        require(serialized_control(record) == reference_control(wire), 'first Find needed serialized control input differs from Save baseline')
    # This compares only fields actually read at first Find; unread strings/map
    # payloads/group are not silently declared byte-identical at that boundary.
    observed = result['phase_observations']
    replay_eligibility = [row for row in observed if row['kind'] == 'eligibility']
    replay_selection = [row for row in observed if row['kind'] == 'selection']
    require(len(replay_selection) == len(scans), 'Find rescan count differs')
    offset, definition_reads, unknown_definitions, override_reads, unread_overrides = 0, 0, 0, 0, 0
    for index, scan in enumerate(scans):
        require(scan['entry']['entering_branch_gates'] == first['entering_branch_gates'], 'native Process entry gate proof changed')
        require(scan['entry']['list']['owner'] == first['list']['owner'] and scan['entry']['list']['next_id'] == first['list']['next_id'], 'native owner/next ID changed')
        require(scan['entry']['network_multiplayer'] == first['network_multiplayer'] and scan['entry']['threshold'] == first['threshold'], 'native external gate changed')
        for branch, entry in zip(scan['eligibility'], scan['entry']['list']['records'], strict=True):
            replay = replay_eligibility[offset]
            offset += 1
            actual = branch['entry']['instance']
            require(actual == entry and replay['scan'] == index and replay['object'] == logical(actual['handle']), 'native per-record read chronology differs')
            require({key:replay[key] for key in serialized_control(actual)} == serialized_control(actual), 'Rust/native serialized read field differs')
            require(replay['lock_count'] == actual['lock_count'] and replay['pool'] == (None if actual['pool'] is None else logical(actual['pool'])), 'Rust/native live lock or pool binding differs')
            require(replay['is_auto_activate'] == branch['auto_exit']['is_auto_activate'], 'Rust/native IsAutoActivate differs')
            if 'definition' in branch:
                definition_reads += 1
                expected = -1 if not branch['definition']['definition_found'] else int(branch['icon']['show_icon_single_player'])
                require(replay['definition'] == expected, 'Rust/native actual definition/icon input differs')
            else:
                unknown_definitions += 1
                require(actual['activated'] or first['network_multiplayer'], 'required definition remained unobserved')
            if 'severity' in branch:
                severity = branch['severity']
                override_reads += 1
                require(replay['effective_override'] == severity['effective_native_severity_override'], 'Rust wire override/native first-match conversion differs')
                require(replay['severity'] == severity['severity'] and severity['threshold'] == first['threshold'], 'native severity read differs')
            else:
                unread_overrides += 1
                require(replay['effective_override'] is None, 'unread override invented')
        selected = scan['selected']['selected']
        expected = None if selected is None else logical(selected['handle'])
        require(replay_selection[index]['object'] == expected and replay_selection[index]['locked'] == bool(selected and selected['lock_count'] > 0), 'Rust/native Find return differs')
    require(offset == len(replay_eligibility), 'extra/missing Rust eligibility reads')
    calls = [r for r in captured['rows'] if r['stage'] == 'activate_call']
    reports = [r for r in observed if r['kind'] == 'undelivered_activation']
    require(len(calls) == len(reports), 'activation count mismatch')
    selected_inputs = [scan['selected']['selected'] for scan in scans if scan['selected']['selected'] is not None and scan['selected']['selected']['lock_count'] <= 0]
    require([call['selected'] for call in calls] == selected_inputs, 'activation branch-time input changed after Find return')
    for call, report in zip(calls, reports, strict=True):
        require((report['owner'], report['component_type'], report['id']) == identity(call['selected']) and int(report['argument']) == call['activation_argument'], 'base activation target/argument mismatch')
    releases = [r for r in captured['rows'] if r['stage'] == 'release_call']
    returns = [r for r in captured['rows'] if r['stage'] == 'release_return']
    effects = [r for r in observed if r['kind'] in ('returned_to_pool', 'destroyed_base_resources')]
    require(len(releases) == len(returns) == len(effects), 'Release count mismatch')
    pools = {pool:list(order) for pool, order in captured['pools'].items()}
    for call, returned, effect in zip(releases, returns, effects, strict=True):
        instance = call['released']
        require(instance['dismissed'], 'native Cleanup released a nondismissed object')
        last_find = next(r for r in scans[-1]['entry']['list']['records'] if r['handle'] == instance['handle'])
        require({k:v for k,v in instance.items() if k != 'pool_order'} == {k:v for k,v in last_find.items() if k != 'pool_order'}, 'Release input changed after selection phase')
        require(effect['object'] == logical(instance['handle']), 'Release order/object mismatch')
        if instance['pool'] is not None:
            pool = instance['pool']
            require(effect['kind'] == 'returned_to_pool' and effect['pool'] == logical(pool), 'native pool branch differs')
            require(instance['pool_order'] == pools[pool], 'wrong pre-Release pool order')
            pools[pool].append(instance['handle'])
            require(returned['pool_order'] == pools[pool] and returned['released_pool_link_after'] is None, 'post-Release append/link-clear mismatch')
        else:
            require(effect['kind'] == 'destroyed_base_resources' and returned['pool_order'] is None, 'native destruction branch differs')
    replay_pools = {row['id']:row['objects'] for row in observed if row['kind'] == 'pool'}
    require(replay_pools == {logical(pool):[logical(obj) for obj in order] for pool, order in pools.items()}, 'resulting logical pool order differs')
    final_records = captured['after']['selected']['records']
    for actual, wire in zip(final_records, expected_after['records'], strict=True):
        require(serialized_control(actual) == reference_control(wire), 'native final captured controls differ from serialized after')
    for actual in final_records:
        require(actual['pool_order'] == pools[actual['pool']], 'final surviving object pool view differs')
        initial = next(r for r in first['list']['records'] if r['handle'] == actual['handle'])
        require(actual['lock_count'] == initial['lock_count'] and actual['pool'] == initial['pool'], 'unmodeled survivor lock/binding effect')
    # Fields not implemented in State are retained as explicit native evidence,
    # not counted as Rust live-state support.
    initial_by_handle = {r['handle']:r for r in first['list']['records']}
    require(all(r['default_blocking'] == initial_by_handle[r['handle']]['default_blocking'] for r in final_records), 'unmodeled blocking state changed')
    return dict(native_find_rescans=len(scans), native_eligibility_pairs=offset, definition_icon_paths_compared=definition_reads,
                definition_paths_short_circuited_unobserved=unknown_definitions, effective_override_reads_compared=override_reads,
                override_paths_unread=unread_overrides, activation_targets_arguments_compared=len(calls), release_pairs_compared=len(releases),
                distinct_pool_prefixes_compared=len(pools), observed_pool_order_and_link_clear_equal=True,
                branch_time_virtual_profiles_compared=captured['observed_profile_checks'],
                all_observed_serialized_control_fields_equal=True, unmodeled_default_blocking_unchanged=True)


def rejection_checks(result, captured, reference):
    checks = []
    for field, value in [('lock_count',1),('definition',-1),('effective_override',True)]:
        altered=copy.deepcopy(result)
        row=next(r for r in altered['phase_observations'] if r['kind']=='eligibility' and (field!='effective_override' or r['effective_override'] is not None))
        row[field]=value
        try:
            compare_replay(altered,captured,reference)
        except ValueError:
            checks.append(field)
        else:
            raise ValueError('negative replay accepted: '+field)
    for kind,field,value in [('selection','object',999999),('returned_to_pool','object',999999),('pool','objects',[999999])]:
        altered=copy.deepcopy(result)
        next(r for r in altered['phase_observations'] if r['kind']==kind)[field]=value
        try:
            compare_replay(altered,captured,reference)
        except ValueError:
            checks.append(kind)
        else:
            raise ValueError('negative replay accepted: '+kind)
    return dict(scope='Mutated Rust diagnostic output only; no native artifact mutation/execution', rejected=checks)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--capture-dir', type=Path, default=ROOT.parent / 'c6-ghidra/evidence/linux-notification-lifecycle/native-passive-1')
    parser.add_argument('--module', type=Path, default=ROOT.parent / 'c6-assets-linux/libGameCore_XP2.so')
    parser.add_argument('--host', type=Path, default=ROOT.parent / 'c6-assets-linux/Civ6')
    parser.add_argument('--iterations', type=int, default=1000)
    parser.add_argument('--manifest-output', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    require(0 <= args.iterations <= 10000, 'iteration limit')
    static = read_json(ROOT / 'evidence/linux/notification-lifecycle.json')
    manifest, captured = derive(args.capture_dir, static)
    args.manifest_output.parent.mkdir(parents=True, exist_ok=True)
    args.manifest_output.write_text(manifest)
    result = phase_audit(SimpleNamespace(module=args.module, host=args.host, native_dir=args.capture_dir,
                                       live_manifest=args.manifest_output, iterations=args.iterations))
    reference = read_json(ROOT / 'evidence/linux/skip-turn-state-execute-3.json')
    comparisons = compare_replay(result, captured, reference)
    negatives = rejection_checks(result, captured, reference)
    result.update(schema='c6-linux-notification-captured-control-replay-v1', captured_comparisons=comparisons,
                  negative_replay_checks=negatives,
                  baseline_reference_configuration_sha256=result['configuration_sha256'], configuration_sha256=OBSERVATION_CONFIG,
                  parent_configuration_sha256=PARENT_CONFIG, native_capture_checks_passed=True,
                  actual_live_inputs_observed=True, actual_observed_selection_cleanup_control_input_parity=True,
                  actual_native_phase_input_parity=False, complete_first_find_wire_state_observed=False,
                  default_projection_assumptions=None, live_manifest_source='DERIVED_ACTUAL_FIRST_FIND_AFTER_SEND_AUTO_NOTIFY',
                  live_manifest_derived_fields=captured['live_metadata'], observer_check=captured['observer_check'],
                  capture_artifact_sha256={name:sha((args.capture_dir/name).read_bytes()) for name in
                                          ('manifest.json','metadata.json','validation.json','runtime.json','observations.jsonl','exit.json')},
                  native_component_checker=dict(configuration_sha256=captured['component']['configuration_sha256'],
                      checks_passed=True,native_cleanup_proven=True,conditional_component_completed=True),
                  captured_audit_source_sha256=sha(Path(__file__).read_bytes()),
                  input_limit='All observed control reads used by the supported selection/base-activation/cleanup branches are compared. Full serialized first-Find payloads and complete pooled live objects were not captured; no broad phase-input or complete live-state parity is asserted.',
                  missing_broader_state=['Full first-Find message/summary/parameters and other unread serialized fields',
                      'Notification live +0x18 Group', 'Native +0x130 DefaultEndTurnBlocking captured but not modeled in Rust LiveNotification',
                      'Complete preexisting/released pooled-object live resources', 'Logical event257 queue insertion/order and later consumers'],
                  related_immediate_callback_closure=dict(review_commit='f495c9d', report_sha256=sha((ROOT/'evidence/linux/notification-lifecycle-review.json').read_bytes()),
                      same_capture_invocation=False, scope='Separate related passive reporting invocation: no notification cache delegate, one translator/publish257, null prepublish; not universal callback absence. This Rust adapter does not execute the logical queued event delivery.'),
                  limits='Actual first-Find control-input, native selection/activation arguments, logical pool ownership/order, and full 2629-byte resulting owner comparison pass. Unread first-Find wire payload and full notification/pool live state remain incomplete. Reporting is an undelivered obligation in Rust; event257 queue/delivery and complete Manager.Update/action/turn/RNG parity remain unsupported.')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps({key:result[key] for key in ('checks_passed','full_owner_wire_bytes_equal','actual_observed_selection_cleanup_control_input_parity','actual_native_phase_input_parity','gameplay_parity')}))


if __name__ == '__main__':
    main()
