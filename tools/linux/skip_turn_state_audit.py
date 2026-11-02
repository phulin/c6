#!/usr/bin/env python3
"""Read closed conditional SkipTurn artifacts; never execute original code.

This first diagnostic profile requires byte-identical before/after Core streams.
Changed streams reject instead of borrowing stale serializer offsets. Native
execution failure remains a failure even when artifact validation succeeds.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile

from core_component_check import artifact, digest, raw_difference, require, payload
from save_lifecycle_audit import GAME_SHA, HOST_SHA
from unit_codec_check import SAVES, TRACES

ROOT = Path(__file__).resolve().parents[2]

# Generated only in a temporary directory. Uses the production typed codec,
# not ad-hoc offsets or an identity-byte search, and roundtrips every record.
DECODER = r'''
use c6_core::unit::{Unit, Formation, Limits, OperationParameters};
use std::{env, fs};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a:Vec<String> = env::args().collect();
    let owner:i32=a[2].parse()?; let id:u32=a[3].parse()?;
    let mut selected = None;
    for line in fs::read_to_string(&a[1])?.lines() {
        let p:Vec<_>=line.split_whitespace().collect();
        if p[0]!="unit" {continue;}
        let b=fs::read(p[1])?;
        let (u,r)=Unit::decode_with_ranges(&b,Limits::default())?;
        assert_eq!(u.encode(Limits::default())?,b);
        if (u.e_owner,u.id)!=(owner,id) {continue;}
        assert!(selected.is_none());
        selected=Some(u.formation_id);
        println!("{{\"kind\":\"unit\",\"record\":{},\"owner\":{},\"id\":{},\"formation_id\":{},\"activity\":{},\"automation\":{},\"operation_timer\":{},\"moves_raw\":{},\"queue_size\":{}}}",p[2],u.e_owner,u.id,u.formation_id,u.e_activity,u.e_automation,u.i_operation_timer,u.x_moves_remaining,u.k_operations.nodes.len());
        for f in r {println!("{{\"kind\":\"field\",\"name\":\"{}\",\"start\":{},\"end\":{}}}",f.name,f.start,f.end);}
        for n in u.k_operations.nodes {
            let parameters=match n.parameters {
                OperationParameters::None => "null".to_string(),
                OperationParameters::MoveTo{location,modifiers} => format!("{{\"kind\":\"MoveTo\",\"location\":[{},{}],\"modifiers\":{}}}",location.x,location.y,modifiers),
                OperationParameters::MakeTradeRoute{modifiers,location0,location1,resource_hash} => format!("{{\"kind\":\"MakeTradeRoute\",\"modifiers\":{},\"location0\":[{},{}],\"location1\":[{},{}],\"resource_hash\":{}}}",modifiers,location0.x,location0.y,location1.x,location1.y,resource_hash),
            };
            println!("{{\"kind\":\"queue\",\"operation_hash\":{},\"directive\":{},\"flags\":{},\"push_turn\":{},\"pre_activity\":{},\"pre_automation\":{},\"parameters\":{}}}",n.operation_hash,n.directive,n.flags,n.push_turn,n.previous_activity,n.previous_automation,parameters);
        }
    }
    let formation_id=selected.ok_or("selected actor absent")?;
    let mut formations=0;
    for line in fs::read_to_string(&a[1])?.lines() {
        let p:Vec<_>=line.split_whitespace().collect();
        if p[0]!="formation" {continue;}
        let b=fs::read(p[1])?; let f=Formation::decode(&b,Limits::default())?;
        assert_eq!(f.encode(Limits::default())?,b);
        if (f.owner,f.id)!=(owner,formation_id) {continue;}
        formations+=1;
        assert!(f.unit_ids.contains(&id));
        println!("{{\"kind\":\"formation\",\"record\":{},\"id\":{},\"owner\":{},\"path_nodes\":{},\"cache_location\":[{},{}],\"unit_ids\":{:?}}}",p[2],f.id,f.owner,f.last_path.len(),f.last_path_cache_location.x,f.last_path_cache_location.y,f.unit_ids);
        for (index,n) in f.last_path.iter().enumerate() {
            println!("{{\"kind\":\"formation_path_node\",\"index\":{},\"data1_raw\":{},\"data2\":{},\"flags\":{},\"x\":{},\"y\":{}}}",index,n.data1_raw,n.data2,n.flags,n.x,n.y);
        }
    }
    assert_eq!(formations,1);
    Ok(())
}
'''


def typed_initial(before, selected, private_root):
    require(digest(before) == SAVES[0], 'initial Core lacks pinned serializer ownership')
    trace_path = private_root / 'evidence/linux-save-normalization/checkpoint-203-owners' / TRACES[0][0]
    trace = gzip.decompress(trace_path.read_bytes())
    require(hashlib.sha256(trace).hexdigest() == TRACES[0][1], 'native owner trace identity differs')
    rows = [json.loads(line) for line in trace.splitlines()]
    rows = [r for r in rows if r.get('save') == 1 and r.get('owner') in
            ('GameCore::Unit::Instance', 'GameCore::Formation::Instance')]
    units = [r for r in rows if r['owner'] == 'GameCore::Unit::Instance']
    formations = [r for r in rows if r['owner'] == 'GameCore::Formation::Instance']
    require((len(units), len(formations)) == (1262, 1250), 'native record counts differ')
    subprocess.run(['cargo', 'build', '-p', 'c6-core', '--lib'], cwd=ROOT, check=True)
    meta = json.loads(subprocess.check_output(['cargo', 'metadata', '--format-version=1', '--no-deps'], cwd=ROOT))
    target = Path(meta['target_directory']) / 'debug'
    with tempfile.TemporaryDirectory(prefix='c6-skip-state-') as directory:
        temp = Path(directory)
        (temp / 'decode.rs').write_text(DECODER)
        subprocess.run(['rustc', '--edition=2021', str(temp / 'decode.rs'), '--extern',
                        'c6_core=' + str(target / 'libc6_core.rlib'), '-L',
                        'dependency=' + str(target / 'deps'), '-o', str(temp / 'decode')], check=True)
        data = Path(before).read_bytes()
        lines = []
        for kind, records in [('unit', units), ('formation', formations)]:
            for index, row in enumerate(records):
                start, end = row['start'], row['end']
                require(0 <= start < end <= len(data), 'invalid original owner range')
                path = temp / f'{kind}-{index}.bin'
                path.write_bytes(data[start:end])
                lines.append(f'{kind} {path} {index}')
        (temp / 'records').write_text('\n'.join(lines))
        decoded = [json.loads(line) for line in subprocess.check_output(
            [str(temp / 'decode'), str(temp / 'records'), str(selected['owner']), str(selected['id'])], text=True).splitlines()]
    unit = next(r for r in decoded if r['kind'] == 'unit')
    formation = next(r for r in decoded if r['kind'] == 'formation')
    unit_row, formation_row = units[unit.pop('record')], formations[formation.pop('record')]
    for result, row in [(unit, unit_row), (formation, formation_row)]:
        result.update(native_range=[row['start'], row['end']],
                      record_sha256=hashlib.sha256(data[row['start']:row['end']]).hexdigest())
    fields = [r for r in decoded if r['kind'] == 'field']
    require(len(fields) == 73, 'incomplete Unit field extents')
    for field in fields:
        field.pop('kind')
        field['native_range'] = [unit_row['start'] + field[k] for k in ('start', 'end')]
        start, end = field['native_range']
        field['sha256'] = hashlib.sha256(data[start:end]).hexdigest()
    formation['last_path'] = [r for r in decoded if r.get('kind') == 'formation_path_node']
    return dict(unit=unit, formation=formation, unit_fields=fields,
                queue=[r for r in decoded if r.get('kind') == 'queue'],
                original_trace_sha256=TRACES[0][1], original_core_sha256=SAVES[0],
                all_unit_roundtrips=len(units), all_formation_roundtrips=len(formations),
                alignment='Pinned complete stream identity and original serializer-owned ranges; typed codec consumption and exact roundtrip. No ID-byte search.')


def audit(report, private_root):
    run = json.loads(report.read_text())
    config, paths = run['configuration'], run['execution_paths']
    require(hashlib.sha256(json.dumps(config, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
            == run['configuration_sha256'], 'configuration identity differs')
    require(config['assets']['libGameCore_XP2.so'] == GAME_SHA and config['assets']['Civ6'] == HOST_SHA,
            'original module identities differ')
    require(config['core_component_mode'] == 'execute', 'not a conditional execute profile')
    contract = json.loads((ROOT / 'evidence/linux/core-component-static.json').read_text())
    require(config['core_component_contract'] == contract and contract['checks_passed'],
            'reviewed native constructor/source contract differs')
    require(all(config['sources'].get(name) == value for name, value in contract['source_sha256'].items()),
            'compiled native component sources differ from reviewed contract')
    artifact(paths['lower_core_stream'], config['lower_core_stream'])
    require(paths.get('preload') and digest(paths['preload']) == config['preload_sha256'],
            'archived native preload identity differs')
    require(not config.get('bounded_action_mode') and not config.get('pre_controller_registration'),
            'mixed full-host profile')
    components = [json.loads(line.split(' ', 1)[1]) for line in run['stderr'].splitlines()
                  if line.startswith('C6_LINUX_CORE_COMPONENT {')]
    stages = {row['stage']: row for row in components if row['stage'] != 'actor_candidate'}
    selected = stages['selected']
    require(any(row['stage'] == 'actor_candidate' and row['public_can_start'] == row['core_found'] == 1
                and all(row[k] == selected[k] for k in ('owner', 'id', 'component_bits'))
                for row in components), 'selected actor lacks genuine lookup/legality')
    command_path = Path(paths['core_component']['command'])
    command = artifact(command_path, run['core_component_command'])
    key, owner, component_type, actor = struct.unpack_from('<IhhI', command_path.read_bytes())
    require(command['bytes'] == 28 and key == 0x8c0f3f36 and (owner, actor) == (selected['owner'], selected['id']),
            'actual command actor differs')
    snapshots = {k: artifact(paths['core_component'][k], run['core_component_saves'][k])
                 for k in ('before', 'after', 'repeat')}
    differences = {k: raw_difference(paths['core_component']['before'], paths['core_component'][k])
                   for k in ('after', 'repeat')}
    require(all(value['total_differing_bytes'] == 0 for value in differences.values()),
            'changed Core stream needs a separate typed alignment analysis; stale offsets forbidden')
    trace_path = Path(paths['action_trace_directory']) / 'events.jsonl'
    artifact(trace_path, run['action_trace_files']['events.jsonl'])
    trace = [json.loads(line) for line in trace_path.read_text().splitlines()]
    queues = [row for row in trace if row['event'] == 'queue']
    require(len(queues) == 1, 'expected exactly one actual queued command')
    queue_path = trace_path.parent / f"{queues[0]['packet']:03d}-queue.bin"
    queued = artifact(queue_path, run['action_trace_files'][queue_path.name])
    require(queued == command and queue_path.read_bytes() == command_path.read_bytes()
            and queues[0]['from'] == owner, 'actual Queue bytes/sender differ')
    end = trace[-1]
    require(end == dict(event='end', queued=1, received=0, unit_executed=0, reports=0),
            'this audit profile is the closed unreceived-command diagnostic')
    require(not any(row['event'] in ('receive', 'unit_execute_enter', 'unit_execute_return', 'failing_boundary')
                    for row in trace), 'unexpected execution or guard event')
    require(run['returncode'] == stages['diplomacy_after']['result'] == 230, 'native diagnostic differs')
    require(stages['before']['pending'] == 0 and stages['after']['pending'] == 1, 'pending message transition differs')
    states = [json.loads(line.split(' ', 1)[1]) for line in run['stderr'].splitlines()
              if line.startswith('C6_LINUX_ACTION_STATE {')]
    require([row['stage'] for row in states] == ['before_a', 'before_b', 'after_a', 'after_b'], 'live capture order differs')
    require(all(payload(row) == payload(states[0]) for row in states[1:]), 'observed live state differs')
    require(all(row['rng_unchanged_during_observation'] for row in states), 'observer changed known RNG')
    typed = typed_initial(paths['core_component']['before'], selected, private_root)
    u, live = typed['unit'], states[0]
    require(all(u[k] == live[k] for k in ('activity', 'automation', 'operation_timer', 'queue_size'))
            and u['moves_raw'] & 0xffffffff == live['moves_raw_u32'], 'typed Unit/live fields differ')
    require(len(typed['queue']) == len(live['queue']), 'wire/live queue size differs')
    for wire, node in zip(typed['queue'], live['queue']):
        require(all(wire[k] & 0xffffffff == node[k] & 0xffffffff
                    for k in ('directive', 'flags', 'push_turn', 'pre_activity', 'pre_automation')),
                'wire/live queue scalar mismatch')
    teardown = [line for line in run['stderr'].splitlines()
                if line.startswith('C6_LINUX_ACTION_LIFECYCLE uninitialize_return ')]
    require(len(teardown) == 1 and 'executing 1 cleanup 0' in teardown[0], 'failed worker teardown observation missing')
    return dict(schema='c6-linux-skip-turn-state-diagnostic-v1', artifact_checks_passed=True,
                native_execution_succeeded=False, runtime_report_sha256=digest(report),
                configuration_sha256=run['configuration_sha256'], gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA,
                source_sha256=config['sources'], preload_sha256=config['preload_sha256'],
                runtime_lower_core_input=config['lower_core_stream'], selected=selected,
                native_command=command, native_encoded_actor=dict(owner=owner, component_type=component_type, id=actor),
                raw_original_core_saves=snapshots, raw_differences=differences,
                snapshot_byte_shifts=0, unknown_bytes_classified=False, masked_bytes=0,
                typed_initial_state=typed,
                typed_after_alignment='All three complete streams are byte-identical, so all attributed typed record and field ranges remain identical.',
                live_observations=states, trace_sha256=digest(trace_path), trace_end=end,
                readiness_before=stages['before'], readiness_after=stages['after'],
                diplomacy_observations=[stages['diplomacy_before'], stages['diplomacy_after']],
                ordered_execution_events=[row for row in trace if row['event'] in ('queue', 'receive', 'unit_execute_enter', 'unit_execute_return', 'phase')],
                native_failure=dict(returncode=230, first_failed_postcondition='Original message remains pending after worker execution/synchronization entry points returned; no Receive or UnitOperation execution observed.',
                                    teardown=teardown[0], cause_proven=False),
                observed_nonserialized_mutations=['Core pending-message state 0 -> 1', 'Controller executing remains 1 on failed Uninitialize observation'],
                full_affected_state=False, complete_live_state_unchanged_proven=False,
                skip_turn_effects_observed=False, rust_transition_port_enabled=False,
                host_admission_proven=False, full_action_acceptance=0, gameplay_parity=False,
                historical_raw_save_load_failure_bytes=89198,
                source_audit_sha256=digest(Path(__file__)),
                limits='Closed failed conditional component. Identical serialized Core and selected live surfaces do not prove complete live-state nonmutation. Queue replacement, formation/dirty/event/diplomacy/notification/end-turn effects were not executed. No gameplay transition or nonmutating Unsupported result.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime-report', type=Path, required=True)
    parser.add_argument('--private-root', type=Path, default=ROOT.parent / 'c6-ghidra')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = audit(args.runtime_report, args.private_root)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({k: result[k] for k in ('artifact_checks_passed', 'native_execution_succeeded', 'skip_turn_effects_observed', 'rust_transition_port_enabled')}))


if __name__ == '__main__':
    main()
