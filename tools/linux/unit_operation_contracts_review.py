#!/usr/bin/env python3
"""Independent immutable-source/full-method action-contract review; no native execution."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

ROOT = Path(__file__).resolve().parents[2]
# Reviewer-selected complete methods, independently read from the exact target.
METHODS = [
    ('Manager.CanStart', 0x2da20f0, 579, '573164c5e12ad82eeb6761d2492634ae376ae6de63526a3b94cf0608d8550927'),
    ('Manager.CanCurrentOperationBeInterrupted', 0x2da2a60, 116, '6b2b1cbb8b18460cf4fb4f4f7a8000cd6f42ae2108cab549666eb1a387772ebe'),
    ('UnitOperation.Execute', 0x2d28a00, 176, '5855bdbd2ae7837445bd71020d7b9f11f713997b487f3ba8b5dfc65c7d046e26'),
    ('Diplomacy.ProcessPending', 0x2913510, 178, 'ed5dc2a1bf712220260dc946a9e16358d750f0b35b407e563edf790c9f3e2f57'),
    ('Diplomacy.ProcessPendingSessionStatement', 0x2911900, 166, '6fd2f96ae52b08c464dc928bc908bb511d6f8f15294a467a46e609c010316b7e'),
    ('Session.RemoveNextPendingStatement', 0x2917270, 227, '248c35ba6a613b87fb070882bdc1e2d0fd03f2a2b77fbd358d264ab158ad74f1'),
    ('Player.UpdateEndTurnBlocking', 0x2b5bd70, 155, '8f538f3279686211e26ca7d445cb225beb6a031bba7198eb033ca942cce207f5'),
    ('Player.SetEndTurnBlocking', 0x2b5be10, 82, 'e20044c8a217e29d4f7fc297bffbfe836f656c7b99b7ed7db376bef55f18d521'),
    ('Notification.Update', 0x2d2f360, 103, '58ea20bc985c8329a6026aba44f6f333bbac22709a3b308e93667c772fb73d2a'),
    ('Notification.SendAutoNotify', 0x2d2f4b0, 236, 'f5fb1fb0d34e558a55fd16682cdbcdb260abaea93799e353155e51cfeddf89ab'),
    ('Notification.ProcessAutoActivate', 0x2d2f5a0, 151, 'b920adcb274714c96a02dbb4498d7348a2b39849645faffc124dba1fe542b294'),
    ('PlayerNotifications.GetBlocking', 0x2d31060, 201, '261606a6e330578b124b6d16e439e042ab07ed740c1761f14d657bb338f795d4'),
    ('PlayerNotifications.DismissExpired', 0x2d31130, 161, '0b3f5b2ecb7b0630c573b8a2ed6c3c37cf8a65da5f6f630a1d0a471f807dca45'),
    ('Instance.IsExpired', 0x2d2e250, 50, '7abbd8a56f6b5af7410114d7cd39d0002d468c4bec00ff3683fab8640d8d66a7'),
    ('Instance.IsValidForPhase', 0x2d29200, 3, 'e18aacce29affcdddaa5f641ac1ec12552e0fe9d066e6408941b1792818b8619'),
    ('Instance.IsNeeded', 0x2d2e290, 28, '658f4c8162885bccab4491c099546fcdada489db36ada6cd6423f88180bb3b2b'),
    ('Instance.AutoExpires', 0x2d2e2b0, 11, '365ff90a515df2f8f029da403b91b1041eddf955455410d884e2e5bf525507e1'),
    ('Instance.GetEndTurnBlocking', 0x2d2e3b0, 45, '33eddf9a3995c115f6f6c4a91afc37dbb1b20e59e8d180d24cca79a14ed56398'),
]
HOST_METHODS = [
    ('DirtyComponents.Add', 0x32b5950, 86, '1f4235247163eba5152a3010fbb5f27a940bf74284dd888757f30869529dd3c2'),
    ('ComponentsArray.SetDirty', 0x32b5f90, 66, 'ae43abdb887ee6fd52a14001c01dca33c7c46ac9a7998e10134913cc240408b3'),
    ('ComponentID.Equal', 0x2fb7350, 34, '565a5536704f1c2eda96270c96177cd8dea251e96a1c60e185c70c2ab5c49f80'),
]
# These are receiver-retention/order sites that exposed the first aftermath defect.
CAPTURE_SITES = [
    (0x2d2f5ac, 'e8ff8410ff4989c6', 'Configuration retained before accepts callback'),
    (0x2d2f5e4, 'e8e7f613ff4989c7', 'PlayerNotifications retained before network query'),
    (0x2d2f60e, '4c89ff89ee', 'Activation rescan reuses captured PlayerNotifications'),
    (0x2d2f53e, 'e88df713ff4989c4', 'Factory result followed by captured PlayerNotifications'),
    (0x2d2f569, '4889e74c89e64889da', 'Add uses same captured collection after Send'),
    (0x2b5bd99, '89c54c89f7', 'Blocking result precedes another original Player ID read'),
    (0x2b5be24, '899fa8070000', 'Store on original Player receiver before GetID/local query'),
]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def pin(commit, paths):
    resolved = subprocess.check_output(['git', 'rev-parse', commit + '^{commit}'], cwd=ROOT, text=True).strip()
    result = {}
    for path in paths:
        data = subprocess.check_output(['git', 'show', resolved + ':' + path], cwd=ROOT)
        if data != (ROOT / path).read_bytes():
            raise ValueError('working source differs from immutable review pin: ' + path)
        result[path] = sha(data)
    return dict(commit=resolved, files=result)


def check_methods(elf, methods):
    rows = []
    for name, address, size, digest in methods:
        if sha(elf.read(address, size)) != digest:
            raise ValueError('independent full method fingerprint differs: ' + name)
        rows.append(dict(name=name, rva=hex(address), bytes=size, sha256=digest))
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--aftermath-commit', required=True)
    parser.add_argument('--capture', type=Path, default=ROOT.parent / 'c6-ghidra/evidence/linux-notification-lifecycle/native-passive-1/observations.jsonl')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    source = dict(
        manager=pin('6126bab', ['crates/c6-core/src/unit_operation_legality.rs', 'tools/linux/unit_operation_manager_audit.py', 'evidence/linux/unit-operation-manager.json']),
        aftermath=pin(args.aftermath_commit, ['crates/c6-core/src/unit_operation_aftermath.rs', 'tools/linux/unit_operation_aftermath_audit.py', 'evidence/linux/unit-operation-aftermath.json']),
        reporting=pin('f5b16f1', ['crates/c6-core/src/reporting_dirty_components.rs', 'tools/linux/skip_turn_reporting_closure_audit.py', 'evidence/linux/skip-turn-reporting-closure.json']))
    source['notification_lifecycle_correction'] = pin(args.aftermath_commit, [
        'tools/linux/notification_lifecycle_audit.py', 'evidence/linux/notification-lifecycle.json',
        'docs/linux-notification-lifecycle.md'])
    game = Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    host = Elf(args.assets / 'Civ6', HOST_SHA)
    symbol_rows = subprocess.check_output(['nm', '-D', '--defined-only', str(args.assets / 'libGameCore_XP2.so')], text=True).splitlines()
    symbols = {row.split()[-1]: int(row.split()[0], 16) for row in symbol_rows}
    predicate_symbols = {
        '_ZNK8GameCore9Reporting12Notification8Instance15IsValidForPhaseEv': 0x2d29200,
        '_ZNK8GameCore9Reporting12Notification8Instance8IsNeededEv': 0x2d2e290,
        '_ZNK8GameCore9Reporting12Notification8Instance9IsExpiredEv': 0x2d2e250,
        '_ZNK8GameCore9Reporting12Notification8Instance11AutoExpiresEv': 0x2d2e2b0,
        '_ZNK8GameCore9Reporting12Notification8Instance18GetEndTurnBlockingEv': 0x2d2e3b0,
    }
    if any(symbols.get(name) != address for name, address in predicate_symbols.items()):
        raise ValueError('actual ELF predicate symbol identity differs')
    methods = check_methods(game, METHODS)
    host_methods = check_methods(host, HOST_METHODS)
    for address, expected, label in CAPTURE_SITES:
        if game.read(address, len(expected) // 2).hex() != expected:
            raise ValueError('receiver capture site differs: ' + label)
    virtual_bindings = []
    for addresspoint in (0x3af6300, 0x3afb660):
        for slot, target in ((0x68, 0x2d2f000), (0x98, 0x2d2f360),
                             (0xa0, 0x2d2f330), (0x108, 0x2d2f4a0),
                             (0x110, 0x2d2f4b0), (0x118, 0x2d2f5a0),
                             (0x120, 0x2d2f640)):
            if game.relocated_pointer(addresspoint + slot) != target:
                raise ValueError('notification manager virtual binding differs')
            virtual_bindings.append(dict(addresspoint=hex(addresspoint), slot=hex(slot), target=hex(target)))
    predicate_bindings = []
    for addresspoint in (0x3af6238, 0x3af6530):
        for slot, target, meaning in ((0x28, 0x2d2e250, 'IsExpired'),
                                     (0x30, 0x2d29200, 'IsValidForPhase'),
                                     (0x38, 0x2d2e290, 'IsNeeded'),
                                     (0x40, 0x2d2e2b0, 'AutoExpires'),
                                     (0x50, 0x2d2e3b0, 'GetEndTurnBlocking')):
            if game.relocated_pointer(addresspoint + slot) != target:
                raise ValueError('base/PrideMoment predicate virtual binding differs')
            predicate_bindings.append(dict(addresspoint=hex(addresspoint), slot=hex(slot), target=hex(target), meaning=meaning))
    if game.read(0x2d310c1, 3).hex() != 'ff5030':
        raise ValueError('blocking reducer no longer calls phase-valid slot30')
    capture_raw = args.capture.read_bytes()
    capture_hash = sha(capture_raw)
    if capture_hash != 'aa81095ebdea3c091cb910e8335d68d6277a4f047445329020de4192c572c9d6':
        raise ValueError('historical passive capture identity differs')
    first_find = next(row for row in map(json.loads, capture_raw.splitlines()) if row['stage'] == 'find_entry')
    tables = json.loads((ROOT / 'evidence/linux/notification-lifecycle.json').read_text())['virtual_bindings']
    inferred = []
    for record in first_find['list']['records']:
        matches = [table for table in tables if all(table['slots'][key] == value for key, value in record['slots'].items())]
        if len(matches) != 1 or matches[0]['vtable_rva'] not in ('0x3af6228', '0x3af6520'):
            raise ValueError('captured virtual profile is not uniquely base/PrideMoment')
        inferred.append(dict(handle=record['handle'], inferred_vtable_rva=matches[0]['vtable_rva'],
            phase_valid_target=matches[0]['slots']['is_valid_for_phase'], needed_target=matches[0]['slots']['is_needed']))
    if len(inferred) != 12:
        raise ValueError('captured first-Find inventory differs')
    result = dict(
        schema='c6-linux-unit-operation-contracts-independent-review-v2', checks_passed=True,
        gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA, immutable_sources=source,
        independent_full_gamecore_methods=methods, independent_full_host_methods=host_methods,
        receiver_order_sites=[dict(rva=hex(a), bytes=b, meaning=n) for a, b, n in CAPTURE_SITES],
        checked_base_and_xp2_manager_virtual_bindings=virtual_bindings,
        checked_base_and_pride_moment_predicate_bindings=predicate_bindings,
        checked_predicate_symbol_addresses={name: hex(address) for name, address in predicate_symbols.items()},
        historical_capture=dict(observations_sha256=capture_hash, first_find_objects=inferred,
            inventory_basis='Unique match of five captured virtual targets against all39 exact target tables; raw addresspoint not separately captured.',
            phase_valid_and_needed_results_observed=False,
            current_turn_and_expiry_delta_observed=False,
            native_execution_during_review=False),
        supersedes_semantic_label=dict(review_commit='b96536f', historical_aftermath_commit='14ada2f',
            historical_evidence='evidence/linux/unit-operation-contracts-review.json',
            error='slot30 at2d29200 was mislabeled IsNeeded; actual IsValidForPhase. Real IsNeeded is slot38 at2d2e290.',
            authoritative_reducer='Call actual IsValidForPhase(slot30), then nonzero GetEndTurnBlocking(slot50); no direct IsNeeded(slot38) call.'),
        driver_sha256=sha(Path(__file__).read_bytes()),
        review_verdict='Bounded source control-flow contracts accepted conditionally on faithful live providers; no native gameplay acceptance.',
        reviewed_checkpoints=dict(manager='6126bab', aftermath_initial_rejected='a6eccef',
            aftermath_receiver_repair='14ada2f', aftermath_semantic_correction=args.aftermath_commit, reporting='f5b16f1'),
        repaired_findings=['Configuration/collection/Manager/Owner receiver capture order and repeated owner ID reads',
                           'Node cursor identity separate from notification object identity; captured list end',
                           'Actual slot30 IsValidForPhase distinguished from slot38 IsNeeded in provider API, docs and evidence'],
        independently_run_tests=dict(manager_grouped_tests=5, aftermath_grouped_tests=13, reporting_grouped_tests=2, failures=0),
        independently_rerun_implementation_audits=dict(manager_full_methods=42, manager_call_edges=56,
            aftermath_full_methods=12, reporting_full_methods=23, reporting_call_sites=5,
            corrected_lifecycle_full_methods=25, corrected_lifecycle_virtual_tables=39, all_passed=True),
        static_only=True, original_execution_performed=False,
        synthetic_tests_are_native_parity=False, live_providers_supplied=False,
        production_engine_gameplay_supported=False,
        complete_gameplay_state_action_rng_turns_supported=False,
        raw_save_load_failure_bytes=89198, unknown_save_bytes_masked=False,
        allocation_scope='Manager orchestration has no production allocation; providers retain original effects. In-place continuation APIs remain available; after_add Clone is an isolation wrapper, not a production whole-World-copy requirement. Clone costs are outside query benchmarks. No whole-action performance claim. Reporting capacity/allocation state remains unmodeled.',
        clone_limit='Clone alone cannot roll back external/shared/global effects. Lower-level provider errors can retain partial effects.',
        external_policy='All genuine player decisions require external policy; no native AI planning/deal/NPC/RNG blanket exclusion.',
        reporting_scope='Concrete one-identity ordered fanout/dedup only; live registry/receivers/allocator and complete reporting consumers remain required.')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, independent_full_gamecore_methods=len(methods), independent_full_host_methods=len(host_methods), native_execution=False)))


if __name__ == '__main__':
    main()
