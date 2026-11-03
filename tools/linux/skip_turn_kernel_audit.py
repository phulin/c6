#!/usr/bin/env python3
"""Compare bounded Rust queue phase with immutable Linux native records.

No original code executes. Reporting-provider delivery and complete action parity
are explicitly outside this comparison; historical reporting has an ABI gap.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

from save_lifecycle_audit import Elf, GAME_SHA

ROOT = Path(__file__).resolve().parents[2]
CONFIG = 'd68604a55344b85212154f5abf20b2a109e2d65b34a582686226e1f3f520187a'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def audit(args):
    state = json.loads(args.state_report.read_text())
    core = json.loads((ROOT / 'evidence/linux/core-component-execute-3.json').read_text())
    require(state['configuration_sha256'] == core['configuration_sha256'] == CONFIG, 'configuration mismatch')
    require(state['runtime_report_sha256'] == core['runtime_report_sha256'], 'mixed runtime evidence')
    require(state['native_quiescence_proven'] and state['native_cleanup_proven'], 'native snapshot not quiescent')
    require(state['complete_serialized_byte_footprint_accounted'], 'unclassified serialized footprint')
    require(not state['historical_identity_report_wrappers_return_abi_correct'], 'new observer needs separately identified adapter')
    # Independently recheck original ELF identities and every selected method.
    elf = Elf(args.module, GAME_SHA)
    contracts = []
    for name in ('skip-turn-contracts.json', 'unit-archive-schema.json'):
        manifest = json.loads((ROOT / 'evidence/linux' / name).read_text())
        require(manifest['gamecore_sha256'] == GAME_SHA, 'contract module mismatch')
        for row in manifest['methods']:
            rva = int(row.get('address', row.get('rva')), 16)
            size = row.get('size', row.get('bytes'))
            require(sha(elf.read(rva, size)) == row['sha256'], 'original method mismatch: ' + row['name'])
            contracts.append(row)
    # MoveTo overrides Deactivate: CompleteMovement must run before BaseHandler.
    extra = [('MoveTo.Deactivate', 0x2da4b40, 0x29, '04ee26064c5370b4e2aacb053f6946f4a51d3726a3bba3e019cb4a84e6178be5'),
             ('Placement.CompleteMovement.Unit', 0x2c42ab0, 0x5c, '7b2680f933866af5406bf9ad5c1c1464be1f5bcef2262e18e095e4338550c2c1'),
             ('Unit.GetNeedsMovementCompletion', 0x2c43a70, 7, '90d52fc37d9542093734f9bc623743bc129b9656431545615a03adaf0bd77fa0')]
    for name, rva, size, fingerprint in extra:
        require(sha(elf.read(rva, size)) == fingerprint, 'MoveTo cancellation method mismatch')
        contracts.append(dict(name=name, address=hex(rva), size=size, sha256=fingerprint))
    require(elf.relocated_pointer(0x3affa30 + 16 + 0x68) == 0x2da4b40, 'MoveTo Deactivate override mismatch')
    elf.call(0x2da4b51, 0x1f63180)
    require(elf.read(0x2da4b64, 1) == b'\xe9', 'MoveTo tail binding differs')
    require(elf.read(0x2c42ac1, 2).hex() == '7441', 'CompleteMovement false fast path differs')
    before, after = args.native_dir / 'before.bin', args.native_dir / 'after.bin'
    streams = {'before': before.read_bytes(), 'after': after.read_bytes()}
    for side, data in streams.items():
        require(sha(data) == state['original_core_saves'][side]['sha256'], side + ' complete Core identity mismatch')
    selected = next(r for r in core['component_observations'] if r['stage'] == 'selected')
    conversion = next(r for r in core['component_observations'] if r['stage'] == 'conversion')
    require(selected['bAppend'] is False and selected['bFromUser'] is False, 'request boolean profile mismatch')
    require(conversion['accepted'] and conversion['bAppend'] == conversion['bFromUser'] == 0, 'original conversion profile mismatch')
    command_path = args.native_dir / 'command.bin'
    require(sha(command_path.read_bytes()) == core['command']['sha256'], 'command mismatch')
    rows = {r['owner']: r for r in state['changed_owner_regions']}
    phase = next(r for r in core['component_observations'] if r['stage'] == 'before')
    subprocess.run(['cargo', 'build', '-p', 'c6-core', '--release', '--example', 'skip_turn_kernel'], cwd=ROOT, check=True)
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--format-version=1', '--no-deps'], cwd=ROOT))
    candidate = Path(metadata['target_directory']) / 'release/examples/skip_turn_kernel'
    with tempfile.TemporaryDirectory(prefix='c6-skip-kernel-') as temp:
        directory = Path(temp)
        for name in ('unit', 'formation'):
            for side in streams:
                lo, hi = rows[name][side + '_range']
                record = streams[side][lo:hi]
                require(sha(record) == rows[name][side + '_sha256'], 'record identity mismatch')
                (directory / (side + '-' + name)).write_bytes(record)
        output = subprocess.check_output([str(candidate), str(directory / 'before-unit'), str(directory / 'before-formation'),
                                         str(command_path), str(phase['turn']), str(phase['local']),
                                         str(directory / 'rust-unit'), str(directory / 'rust-formation'), str(args.iterations)], text=True)
        effects = [json.loads(line) for line in output.splitlines()]
        comparisons = []
        for name in ('unit', 'formation'):
            actual = (directory / ('rust-' + name)).read_bytes()
            expected = (directory / ('after-' + name)).read_bytes()
            require(actual == expected, 'Rust full typed ' + name + ' record mismatch')
            comparisons.append(dict(owner=name, bytes=len(actual), sha256=sha(actual), exact_record_equal=True))
    reports = [r for r in effects if r['kind'] == 'report']
    historical = []
    for r in state['reporting_events']:
        historical.append(dict(kind='report', type_bits=r['type_bits'], component_bits=r['component_bits'],
                               first=r.get('first', r.get('value')), second=r.get('second')))
    require(reports == historical, 'selected historical report arguments/order mismatch')
    native_before, native_after = (state['bounded_live_observations'][n] for n in (0, 2))
    dirty_requests = [r['index'] for r in effects if r['kind'] == 'unit_dirty']
    require(set(native_before['dirty_indexes']) | set(dirty_requests) == set(native_after['dirty_indexes']), 'Unit dirty membership mismatch')
    require(dirty_requests == [11, 32, 72, 32, 72, 11, 11, 72, 32], 'original ordered tracked-edit sequence mismatch')
    formation_requests = [r['index'] for r in effects if r['kind'] == 'formation_dirty']
    require(formation_requests == [14, 15] * 3, 'three ordered formation path edits missing')
    check = next(r for r in effects if r['kind'] == 'checks')
    source_paths = ['crates/c6-core/src/skip_turn.rs', 'crates/c6-core/examples/skip_turn_kernel.rs',
                    'crates/c6-core/src/unit.rs', 'crates/c6-core/src/unit/state.rs', 'crates/c6-core/src/unit/variants.rs',
                    'tools/linux/skip_turn_kernel_audit.py']
    return dict(schema='c6-linux-skip-turn-rust-state-phase-v1', checks_passed=True, original_execution_performed=False,
                gamecore_sha256=GAME_SHA, configuration_sha256=CONFIG,
                state_report_sha256=sha(args.state_report.read_bytes()), native_runtime_sha256=state['runtime_report_sha256'],
                native_startup_profile=core['main_thread_startup_profile'],
                native_component_checker_report_sha256=sha((ROOT / 'evidence/linux/core-component-execute-3.json').read_bytes()),
                request_profile=dict(insert_mode=0, append=False, from_user=False, source=core['source']),
                original_core_saves=state['original_core_saves'], original_command_sha256=core['command']['sha256'],
                candidate_sha256=sha(candidate.read_bytes()), source_sha256={p:sha((ROOT / p).read_bytes()) for p in source_paths},
                original_method_fingerprints_checked=len(contracts), additional_cancellation_methods=contracts[-3:],
                movement_completion_false_required=True, typed_record_comparisons=comparisons,
                full_unit_wire_fields_compared=73, complete_formation_wire_record_compared=True,
                unit_dirty_registration_requests=dirty_requests, unit_dirty_membership_matches_native=True,
                unit_native_dirty_iteration_order_modeled=False, formation_dirty_registration_requests=formation_requests,
                formation_native_dirty_observed=False, ordered_effects=effects[:-1],
                selected_historical_report_arguments_equal=True, historical_report_return_abi_correct=False,
                corrected_observer_event_parity=False, reporting_provider_delivery_executed=False,
                required_live_inputs=dict(turn_local='original component observation', numeric_handler='actual original command',
                                          can_move='true branch inferred from original OPERATION activity before handler',
                                          busy='false branch inferred from original handler reaching Start',
                                          activation_can_start='true branch inferred from original handler reaching Start',
                                          unit_archive_attachments='required by observed newly dirty activity/timer/queue indexes',
                                          formation_archive_attachments='assumed attached for static edit-request comparison; no native dirty capture'),
                synthetic_contract_checks=check, synthetic_cases_have_native_fixtures=False,
                benchmark_scope='Rust typed clone/queue transition/effect allocation/drop only; excludes codec, initialization, provider delivery, full legality, diplomacy, notification/player aftermath, turns and native original performance.',
                raw_core_difference=state['raw_core_difference'], historical_raw_save_load_failure_bytes=89198,
                full_action_acceptance=0, gameplay_parity=False, complete_live_affected_state=False,
                complete_rng_inventory=False, engine_gameplay_apply_supported=False,
                limits='Coherent Unit/Formation wire-state phase and static ordered effect requests for an actual positive-path native fixture. Reporting dispatcher/provider calls synchronous callbacks whose actual registrations are uncaptured, so a live operation cannot accept this projection as complete. Full CanStart, archive pointer-tree state, omitted live state, diplomacy and player/notification aftermath remain required; all player decisions yield externally.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--state-report', type=Path, default=ROOT / 'evidence/linux/skip-turn-state-execute-3.json')
    parser.add_argument('--native-dir', type=Path, default=ROOT.parent / 'c6-ghidra/evidence/linux-core-component/native-execute-3')
    parser.add_argument('--module', type=Path, default=ROOT.parent / 'c6-assets-linux/libGameCore_XP2.so')
    parser.add_argument('--iterations', type=int, default=10000)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    require(0 <= args.iterations <= 10000000, 'iteration resource limit')
    result = audit(args)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({k:result[k] for k in ('checks_passed', 'full_unit_wire_fields_compared', 'full_action_acceptance', 'gameplay_parity')}))


if __name__ == '__main__':
    main()
