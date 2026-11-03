#!/usr/bin/env python3
"""Read-only bounded Rust notification state-phase audit.

Default scenario uses explicitly unobserved live inputs. Byte agreement under
that scenario is not actual native lifecycle/input/callback parity.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

ROOT = Path(__file__).resolve().parents[2]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(value, message):
    if not value:
        raise ValueError(message)


def hypothetical_manifest(records):
    # Deliberately named diagnostic scenario, never interpreted as observed live
    # locks, definitions, vtables, pool ownership or native manager configuration.
    lines = ['mode undelivered_projection', 'gates 1 0 1 1 0 0 0 0 v1']
    for ordinal, record in enumerate(records):
        lines.append('record {} 0 - - base base base {} {} {}'.format(
            ordinal, record['owner'], record['component_type'], record['id']))
    return '\n'.join(lines) + '\n'


def audit(args):
    static_path = ROOT / 'evidence/linux/notification-lifecycle.json'
    static = json.loads(static_path.read_text())
    require(static['checks_passed'] and static['static_only'], 'missing static lifecycle contract')
    game, host = Elf(args.module, GAME_SHA), Elf(args.host, HOST_SHA)
    require(static['gamecore_sha256'] == GAME_SHA and static['host_sha256'] == HOST_SHA, 'static module mismatch')
    for method in static['methods']:
        elf = game if method['module'] == 'game' else host
        require(sha(elf.read(int(method['rva'], 16), method['bytes'])) == method['sha256'], 'original method mismatch: ' + method['name'])
    slots = {'deleting_destructor': 8, 'release': 0x20, 'activate': 0x88}
    for row in static['virtual_bindings']:
        for name, offset in slots.items():
            require(game.relocated_pointer(int(row['vtable_rva'], 16) + 16 + offset) == int(row['slots'][name], 16), 'virtual binding mismatch')
    state_report_path = ROOT / 'evidence/linux/skip-turn-state-execute-3.json'
    native = json.loads(state_report_path.read_text())
    region = next(row for row in native['changed_owner_regions'] if row['owner'] == 'notifications')
    streams = {side: (args.native_dir / (side + '.bin')).read_bytes() for side in ('before', 'after')}
    records = {}
    for side, data in streams.items():
        require(sha(data) == native['original_core_saves'][side]['sha256'], 'complete native Core hash mismatch')
        lo, hi = region[side + '_range']
        records[side] = data[lo:hi]
        require(sha(records[side]) == region[side + '_sha256'], 'notification owner extent mismatch')
    source_paths = ['crates/c6-core/src/notification_lifecycle.rs', 'crates/c6-core/src/notifications.rs',
                    'crates/c6-core/src/lib.rs', 'crates/c6-core/src/unit/variants.rs',
                    'crates/c6-core/examples/notification_state_phase.rs', 'tools/linux/notification_state_phase_audit.py']
    source_hashes = {name:sha((ROOT / name).read_bytes()) for name in source_paths}
    subprocess.run(['cargo', 'test', '-p', 'c6-core', 'notification_lifecycle'], cwd=ROOT, check=True, stdout=subprocess.PIPE)
    subprocess.run(['cargo', 'build', '--release', '-p', 'c6-core', '--example', 'notification_state_phase'], cwd=ROOT, check=True)
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--format-version=1', '--no-deps'], cwd=ROOT))
    built_candidate = Path(metadata['target_directory']) / 'release/examples/notification_state_phase'
    require(source_hashes == {name:sha((ROOT / name).read_bytes()) for name in source_paths}, 'source changed during build')
    with tempfile.TemporaryDirectory(prefix='c6-notification-phase-') as tmp:
        directory = Path(tmp)
        # Freeze one executable before running or comparing; never reopen a shared
        # target executable to identify an already completed comparison.
        frozen_bytes = built_candidate.read_bytes()
        candidate_sha = sha(frozen_bytes)
        frozen_dir = Path(metadata['target_directory']) / 'linux/notification-state-phase-candidates' / candidate_sha
        frozen_dir.mkdir(parents=True, exist_ok=True)
        candidate = frozen_dir / 'candidate'
        # Publish a complete immutable file without replacing a same-hash copy.
        with tempfile.NamedTemporaryFile(dir=frozen_dir, delete=False) as staging:
            staging.write(frozen_bytes)
            staging_path = Path(staging.name)
        try:
            staging_path.chmod(0o500)
            try:
                os.link(staging_path, candidate)
            except FileExistsError:
                pass
        finally:
            staging_path.unlink()
        require(sha(candidate.read_bytes()) == candidate_sha, 'candidate freeze mismatch')
        source = directory / 'before.bin'
        source.write_bytes(records['before'])
        manifest = directory / 'live-manifest'
        manifest.write_bytes(args.live_manifest.read_bytes() if args.live_manifest else
                             hypothetical_manifest(native['notification_container']['before']['records']).encode())
        expected_manifest_sha = sha(manifest.read_bytes())
        output = directory / 'rust-after.bin'
        observations = subprocess.check_output([str(candidate), str(source), str(manifest), str(output), str(args.iterations)], text=True)
        require(sha(candidate.read_bytes()) == candidate_sha, 'frozen candidate changed during comparison')
        require(sha(manifest.read_bytes()) == expected_manifest_sha, 'frozen live manifest changed')
        result = [json.loads(line) for line in observations.splitlines()]
        actual = output.read_bytes()
        require(actual == records['after'], 'complete typed native notification owner bytes mismatch')
        summary = next(row for row in result if row['kind'] == 'summary')
        require(summary['before_next_id'] == summary['after_next_id'], 'next ID changed unexpectedly')
    require(source_hashes == {name:sha((ROOT / name).read_bytes()) for name in source_paths}, 'source changed during comparison')
    return dict(schema='c6-linux-notification-rust-state-phase-v1', checks_passed=True,
                gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA, configuration_sha256=native['configuration_sha256'],
                static_lifecycle_contract_sha256=sha(static_path.read_bytes()), original_methods_checked=len(static['methods']),
                original_virtual_tables_checked=len(static['virtual_bindings']),
                original_state_report_sha256=sha(state_report_path.read_bytes()), original_core_saves=native['original_core_saves'],
                original_notification_before_sha256=sha(records['before']), original_notification_after_sha256=sha(records['after']),
                full_owner_wire_bytes_equal=True, resulting_owner_bytes=len(actual),
                candidate_sha256=candidate_sha, frozen_candidate_path=str(candidate), candidate_frozen_before_execution=True, live_manifest_sha256=expected_manifest_sha,
                live_manifest_source=str(args.live_manifest) if args.live_manifest else 'EXPLICIT_HYPOTHETICAL_UNPOOLED_BASE_PROFILE',
                actual_live_inputs_observed=False, actual_native_phase_input_parity=False,
                input_limit='A supplied manifest is hashed, not independently native-certified by this audit. Native lock/definition/pool/manager/subtype provenance requires the passive live-observer checker and separate integration.',
                default_projection_assumptions=None if args.live_manifest else dict(lock_counts=0, pool_binding='none',
                    definition_lookup='missing', activation_and_release='base', nonpool_destruction='base resources',
                    manager_initialized=True, game_uninitializing=False, human=True, turn_active=True, network=False, cloud=False),
                phase_observations=result, synchronous_reporting_delivery_executed=False,
                reporting_obligations_undelivered=True, complete_manager_update_implemented=False,
                excluded_update_stages=['SendAutoNotify', 'DismissExpired', 'GetEndTurnBlocking', 'owner blocking state'],
                synthetic_contract_test_groups=8, synthetic_boundary_native_fixtures=False,
                performance_scope='component_copy_ns includes notification component/delivery clone, transition, effect allocation and drop; local_mutation_ns excludes prebuilt component-clone setup and final state drop, includes preflight/local transition/effect allocation/drop. Both exclude codecs, native code/I/O, actual reporting callbacks, SendAutoNotify, DismissExpired, owner blocking, diplomacy, full actions and turns. No whole-world copy.',
                source_sha256=source_hashes,
                full_action_acceptance=0, gameplay_parity=False, engine_gameplay_apply_supported=False,
                historical_raw_save_load_failure_bytes=89198, unknown_save_bytes_classified=False,
                limits='Exact-target branch-derived typed phase with explicit live inputs and logical ownership. Native owner byte agreement under the named projection does not establish actual native locks, pools, definitions, virtual bindings or synchronous provider/subscriber effects. No approximate notification IDs or fixture selection rules; no complete gameplay state/action/turn claim.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--module', type=Path, default=ROOT.parent / 'c6-assets-linux/libGameCore_XP2.so')
    parser.add_argument('--host', type=Path, default=ROOT.parent / 'c6-assets-linux/Civ6')
    parser.add_argument('--native-dir', type=Path, default=ROOT.parent / 'c6-ghidra/evidence/linux-core-component/native-execute-3')
    parser.add_argument('--live-manifest', type=Path)
    parser.add_argument('--iterations', type=int, default=1000)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    require(0 <= args.iterations <= 10000, 'iteration limit')
    result = audit(args)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({key:result[key] for key in ('checks_passed', 'full_owner_wire_bytes_equal', 'actual_native_phase_input_parity', 'gameplay_parity')}))


if __name__ == '__main__':
    main()
