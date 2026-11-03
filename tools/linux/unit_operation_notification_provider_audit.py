#!/usr/bin/env python3
"""Read-only exact Linux predicate/provider audit; never executes original code."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import xml.etree.ElementTree as ET

from save_lifecycle_audit import Elf, GAME_SHA
from notification_captured_phase_audit import derive, OBSERVATION_CONFIG

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'crates/c6-core/src/unit_operation_notification_provider.rs'
CAPTURE = ROOT.parent / 'c6-ghidra/evidence/linux-notification-lifecycle/native-passive-1'
SLOTS = {'is_expired': 0x28, 'is_valid_for_phase': 0x30, 'is_needed': 0x38,
         'auto_expires': 0x40, 'get_blocking': 0x50}



def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def methods_and_factories(game, module, static):
    # Complete supplied ELF hash gates every instruction. Full method identities
    # are included so a reader can independently reproduce dependency inspection.
    names = subprocess.check_output(['nm', '-D', '-S', '--defined-only', str(module)], text=True)
    methods = []
    for line in names.splitlines():
        parts = line.split()
        if len(parts) != 4:
            continue
        address, size, _, symbol = parts
        if not symbol.startswith('_ZN') or '12Notification' not in symbol:
            continue
        if not any(word in symbol for word in ('9IsExpired', '15IsValidForPhase', '8IsNeeded', '6Create', '8Register')):
            continue
        start, length = int(address, 16), int(size, 16)
        text = subprocess.check_output(['objdump', '-d', '--start-address=' + hex(start),
                                       '--stop-address=' + hex(start + length), str(module)], text=True)
        dependencies = [dict(rva=a, target=t, symbol=n) for a, t, n in re.findall(
            r'^\s*([0-9a-f]+):[^\n]*\b(?:call|jmp)\s+([0-9a-f]+)\s+<([^>]+)>', text, re.M)]
        methods.append(dict(symbol=symbol, rva=hex(start), bytes=length,
                            sha256=sha(game.read(start, length)), direct_dependencies=dependencies))
    for row in static['virtual_bindings']:
        address = int(row['vtable_rva'], 16) + 16
        for name, slot in SLOTS.items():
            require(game.relocated_pointer(address + slot) == int(row['slots'][name], 16), 'actual subtype slot differs: ' + name)
    return methods


def captured_replay(capture, static):
    _, observed = derive(capture, static)
    first = observed['first']['list']['records']
    reference = json.loads((ROOT / 'evidence/linux/skip-turn-state-execute-3.json').read_text())
    region = next(r for r in reference['changed_owner_regions'] if r['owner'] == 'notifications')
    raw = (capture / 'before.bin').read_bytes()
    require(sha(raw) == reference['original_core_saves']['before']['sha256'], 'original baseline save identity')
    before = raw[slice(*region['before_range'])]
    require(sha(before) == region['before_sha256'], 'original owner baseline identity')
    require(len(first) == 12, 'captured owner inventory changed')
    rows, live, inputs, pools = [], [], [], {}
    for snapshot, wire in zip(first, reference['notification_container']['before']['records'], strict=True):
        matches = [t for t in static['virtual_bindings'] if all(t['slots'][k] == v for k, v in snapshot['slots'].items())]
        require(len(matches) == 1, 'captured exact profile not uniquely identified')
        table = matches[0]
        owner_bits = int(snapshot['identity_bits'], 16)
        require(owner_bits >> 32 == wire['id'] and int(snapshot['type_hash'], 16) == wire['type_hash'], 'wire/capture object identity')
        require(snapshot['dismissed'] == wire['is_dismissed'] and snapshot['severity'] == wire['severity'], 'wire/capture predicate fields')
        object_id = int(snapshot['handle'].removeprefix('object-'))
        pool = snapshot['pool']
        pool_binding = 'None' if pool is None else f'Some(PoolId({int(pool.removeprefix("object-"))}))'
        if pool is not None:
            pools[int(pool.removeprefix('object-'))] = snapshot['pool_order']
        live.append(f'''LiveNotification {{ object: ObjectId({object_id}), lock_count: {snapshot['lock_count']},
            pool_binding: {pool_binding}, definition_icon: DefinitionIcon::Unobserved,
            activation: ActivationBinding::Base, release: ReleaseBinding::Base, destruction: DestructionBinding::BaseResources }}''')
        address = int(table['vtable_rva'], 16) + 16
        inputs.append(f'LivePredicates {{ object: ObjectId({object_id}), vtable_addresspoint: {address}, default_blocking: Some({snapshot["default_blocking"]}) }}')
        rows.append(dict(object=object_id, notification_id=wire['id'], type_hash=hex(wire['type_hash']),
                         inferred_vtable_addresspoint=hex(address), subtype_symbol=table['symbol'],
                         default_blocking=snapshot['default_blocking'], added_turn=wire['native_188_18c'][0],
                         expiry_delta=wire['native_188_18c'][1], blocks_turn_advancement=bool(wire['native_198_199_19a'][2]),
                         dismissed=snapshot['dismissed'], wire_fields_provenance='Original identified Save baseline; only observed control fields crosschecked at first Find'))
    pool_exprs = [f'Pool {{ id: PoolId({key}), retained_objects: vec![' + ','.join('ObjectId(' + obj.removeprefix('object-') + ')' for obj in value) + '], records: vec![] }' for key, value in pools.items()]
    with tempfile.TemporaryDirectory(prefix='c6-notification-predicate-') as tmp:
        folder = Path(tmp)
        (folder / 'owner.bin').write_bytes(before)
        harness = '''pub use c6_core::{notification_lifecycle, notifications, unit_operation_aftermath};
#[path = "''' + str(SOURCE) + '''"] pub mod provider;
use provider::{CapturedOwner,LivePredicates};
use notification_lifecycle::*;
use notifications::{Limits,PlayerNotifications};
fn main() {
 let args: Vec<String> = std::env::args().collect();
 let records = PlayerNotifications::decode(&std::fs::read(&args[1]).unwrap(), Limits::default()).unwrap();
 let live = vec![''' + ','.join(live) + '''];
 let state = State { notifications: records, live, pools: vec![''' + ','.join(pool_exprs) + '''] };
 let turn = if args.len() > 2 {
   let bytes = std::fs::read(&args[2]).unwrap();
   let prefix = c6_core::core_stream::CorePrefix::decode(&bytes, c6_core::map::Limits::default()).unwrap();
   println!("saved_turn|{}",prefix.current_turn);
   Some(prefix.current_turn)
 } else { None };
 let mut owner = CapturedOwner::new(state, vec![''' + ','.join(inputs) + '''], turn).unwrap();
 let objects: Vec<ObjectId> = owner.predicates().iter().map(|r| r.object).collect();
 for object in objects { println!("{}|{:?}|{:?}|{:?}|{:?}",object.0,
   owner.valid_for_phase(object),owner.is_needed(object),owner.expired(object),owner.blocking(object)); }
 println!("reduction|{:?}",owner.blocking_reduction());
}
'''
        source = folder / 'main.rs'
        source.write_text(harness)
        identities = {p: sha(p.read_bytes()) for p in [SOURCE, ROOT / 'crates/c6-core/src/unit_operation_aftermath.rs', ROOT / 'crates/c6-core/src/lib.rs']}
        subprocess.run(['cargo', 'build', '-p', 'c6-core'], cwd=ROOT, check=True, stdout=subprocess.PIPE)
        candidate = folder / 'candidate'
        subprocess.run(['rustc', '--edition=2021', str(source), '--extern', 'c6_core=' + str(ROOT / 'target/debug/libc6_core.rlib'),
                        '-L', 'dependency=' + str(ROOT / 'target/debug/deps'), '-o', str(candidate)], cwd=ROOT, check=True)
        require(all(sha(p.read_bytes()) == identity for p, identity in identities.items()), 'source changed during build')
        candidate_bytes = candidate.read_bytes()
        candidate_sha = sha(candidate_bytes)
        frozen_dir = ROOT / 'target/linux/unit-operation-notification-provider-candidates' / candidate_sha
        frozen_dir.mkdir(parents=True, exist_ok=True)
        frozen = frozen_dir / 'candidate'
        try:
            os.link(candidate, frozen)
        except FileExistsError:
            require(sha(frozen.read_bytes()) == candidate_sha, 'immutable candidate differs')
        frozen.chmod(0o500)
        candidate = frozen
        output = subprocess.check_output([str(candidate), str(folder / 'owner.bin')], text=True)
        lines = output.splitlines()
        require(len(lines) == 13, 'predicate replay row count')
        for line, row in zip(lines[:-1], rows, strict=True):
            blocking = 'Ok(0)' if row['dismissed'] else 'Err(Unsupported("actual current game turn"))'
            expected = f'{row["object"]}|Ok(true)|Ok(true)|Err(Unsupported("actual current game turn"))|{blocking}'
            require(line == expected, 'captured predicate explicit-unknown result differs')
        require(lines[-1] == 'reduction|Err(Unsupported("actual current game turn"))', 'missing turn silently defaulted')
        saved_output = subprocess.check_output([str(candidate), str(folder / 'owner.bin'), str(capture / 'before.bin')], text=True)
        saved_lines = saved_output.splitlines()
        saved_turn = int(saved_lines[0].split('|')[1])
        require(len(saved_lines) == 14, 'saved-turn conditional replay count')
        for line, row in zip(saved_lines[1:-1], rows, strict=True):
            expected = f'{row["object"]}|Ok(true)|Ok(true)|Ok(false)|Ok(0)'
            require(line == expected, 'saved-turn conditional predicate result differs')
        require(saved_lines[-1] == 'reduction|Ok(0)', 'saved-turn conditional reduction differs')
        candidate_sha = sha(candidate.read_bytes())
    return dict(configuration_sha256=OBSERVATION_CONFIG, records=rows,
                capture_sha256=sha((capture / 'observations.jsonl').read_bytes()), owner_wire_sha256=sha(before),
                rust_candidate_sha256=candidate_sha, immutable_candidate=str(candidate.relative_to(ROOT)), rust_results_missing_live_turn=lines,
                saved_initial_turn=saved_turn, rust_results_using_saved_initial_turn=saved_lines,
                saved_turn_replay_scope='Conditional on no intervening turn change. Production CorePrefix decoder supplies the saved field, not a fixed offset/592. No callback-wide no-write proof or native predicate-return comparison.', 
                current_turn_observed_at_predicate_boundary=False, native_predicate_return_comparison=False,
                wire_to_live_limit='Current turn absent. Added/delta/19a come from original Save baseline, not a new branch-time read. Do not infer full first-Find equality.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--module', type=Path, default=ROOT.parent / 'c6-assets-linux/libGameCore_XP2.so')
    parser.add_argument('--capture-dir', type=Path, default=CAPTURE)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    game = Elf(args.module, GAME_SHA)
    static = json.loads((ROOT / 'evidence/linux/notification-lifecycle.json').read_text())
    methods = methods_and_factories(game, args.module, static)
    source_profiles = [tuple(int(v, 16) for v in values) for values in re.findall(
        r'\((0x[0-9a-f]+), (0x[0-9a-f]+), (0x[0-9a-f]+), (0x[0-9a-f]+)\)', SOURCE.read_text())]
    expected = [(int(r['vtable_rva'], 16) + 16, int(r['slots']['is_expired'], 16), int(r['slots']['is_valid_for_phase'], 16), int(r['slots']['is_needed'], 16)) for r in static['virtual_bindings']]
    require(source_profiles == expected, 'production exact subtype table differs')
    captured = captured_replay(args.capture_dir, static)
    definitions = ROOT.parent / 'c6-assets-linux/Base/assets/gameplay/data/notifications.xml'
    candidates = [r.attrib['NotificationType'] for r in ET.parse(definitions).getroot().iter('Row') if r.attrib.get('AutoNotify', '').lower() in ('true', '1')]
    report = dict(schema='c6-linux-unit-operation-notification-provider-v1', checks_passed=True,
                  gamecore_sha256=GAME_SHA, source_sha256=sha(SOURCE.read_bytes()), audit_sha256=sha(Path(__file__).read_bytes()),
                  native_execution=False, original_methods=methods, verified_subtypes=39,
                  implemented_expiry_subtypes=9, captured_owner=captured,
                  auto_notify_definition_candidates=dict(source_sha256=sha(definitions.read_bytes()), names=candidates,
                      actual_live_definition_inventory=False, scope='Supplied base data discovery only; active DLC/mod/rules and iteration order are not established by this XML'),
                  factory_lookup='Manager.Create requires initialized and not uninitializing; matching nonnull registered factory returns its result if nonnull, otherwise FALL BACK to base Instance.Create.',
                  unresolved=['Actual current turn at predicate boundary', 'Branch-time added/delta/19a and exact subtype bindings +30/+38/+40/+50',
                              'All thirty concrete subtype expiry overrides and phase restrictions outside captured inherited profile',
                              'Actual ordered active definitions and current creator registry/pool state',
                              'Create initialization, true IsNeeded, duplicate/Send/Add, reports, subscriber/queue/RNG effects'],
                  broad_native_predicate_parity=False, complete_update_parity=False, full_action_turn_parity=False,
                  historical_save_load_raw_difference_unmasked_unresolved_bytes=89198,
                  attribution='docs/reference.md; public semantic contracts only; private capture unchanged')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, captured_objects=12, actual_current_turn=None, native_predicate_parity=False)))


if __name__ == '__main__':
    main()
