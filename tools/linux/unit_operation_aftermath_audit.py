#!/usr/bin/env python3
"""Pin the Rust post-Add control-flow boundary against exact Linux XP2 bytes.

Static inspection only; never executes original code or certifies provider parity.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct

from save_lifecycle_audit import Elf, GAME_SHA
from skip_turn_state_aftermath_audit import BRANCHES

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'crates/c6-core/src/unit_operation_aftermath.rs'
METHODS = [
    ('UnitOperation.Execute', 0x2d28a00, 0xb0),
    ('Diplomacy.ProcessPending', 0x2913510, 0xb2),
    ('Diplomacy.ProcessPendingSessionStatement', 0x2911900, 0xa6),
    ('Session.RemoveNextPendingStatement', 0x2917270, 0xe3),
    ('Player.UpdateEndTurnBlocking', 0x2b5bd70, 0x9b),
    ('Player.SetEndTurnBlocking', 0x2b5be10, 0x52),
    ('Notification.Update', 0x2d2f360, 0x67),
    ('Notification.SendAutoNotify', 0x2d2f4b0, 0xec),
    ('Notification.ProcessAutoActivate', 0x2d2f5a0, 0x97),
    ('PlayerNotifications.GetBlocking', 0x2d31060, 0xc9),
    ('PlayerNotifications.DismissExpired', 0x2d31130, 0xa1),
    ('Instance.IsExpired', 0x2d2e250, 0x32),
]
EXPECTED_METHOD_SHA256 = {'UnitOperation.Execute': '5855bdbd2ae7837445bd71020d7b9f11f713997b487f3ba8b5dfc65c7d046e26', 'Diplomacy.ProcessPending': 'ed5dc2a1bf712220260dc946a9e16358d750f0b35b407e563edf790c9f3e2f57', 'Diplomacy.ProcessPendingSessionStatement': '6fd2f96ae52b08c464dc928bc908bb511d6f8f15294a467a46e609c010316b7e', 'Session.RemoveNextPendingStatement': '248c35ba6a613b87fb070882bdc1e2d0fd03f2a2b77fbd358d264ab158ad74f1', 'Player.UpdateEndTurnBlocking': '8f538f3279686211e26ca7d445cb225beb6a031bba7198eb033ca942cce207f5', 'Player.SetEndTurnBlocking': 'e20044c8a217e29d4f7fc297bffbfe836f656c7b99b7ed7db376bef55f18d521', 'Notification.Update': '58ea20bc985c8329a6026aba44f6f333bbac22709a3b308e93667c772fb73d2a', 'Notification.SendAutoNotify': 'f5fb1fb0d34e558a55fd16682cdbcdb260abaea93799e353155e51cfeddf89ab', 'Notification.ProcessAutoActivate': 'b920adcb274714c96a02dbb4498d7348a2b39849645faffc124dba1fe542b294', 'PlayerNotifications.GetBlocking': '261606a6e330578b124b6d16e439e042ab07ed740c1761f14d657bb338f795d4', 'PlayerNotifications.DismissExpired': '0b3f5b2ecb7b0630c573b8a2ed6c3c37cf8a65da5f6f630a1d0a471f807dca45', 'Instance.IsExpired': '7abbd8a56f6b5af7410114d7cd39d0002d468c4bec00ff3683fab8640d8d66a7'}
CHECKS = [
    (0x2d2f5b1, "4989c6", "Retain configuration before accepts callback"),
    (0x2d2f5e9, "4989c7", "Retain collection before network getter"),
    (0x2d2f5ec, "4c89f7", "Network getter uses retained configuration"),
    (0x2d2f60e, "4c89ff", "Each Find uses retained collection"),
    (0x2d2f543, "4989c4", "Retain collection before redundancy and InitializeText"),
    (0x2d2f56c, "4c89e6", "Add uses same captured collection after InitializeText"),
    (0x2b5bd74, "4989fe", "Retain actual player receiver for Update"),
    (0x2b5be24, "899fa8070000", "Store same player receiver before GetID"),
    (0x2913584, 'beffffffff', 'ProcessPending other-player sentinel -1'),
    (0x2913589, 'ba01000000', 'ProcessPending OUTGOING direction 1'),
    (0x2913595, 'ebe9', 'Drain repeats without another IsAI getter'),
    (0x2911920, '83fb01', 'Manager outgoing direction check'),
    (0x2911941, '4183feff', 'Manager optional other-player sentinel'),
    (0x2911984, '7514', 'Removed statement enters ProcessStatement'),
    (0x2917287, '83f901', 'Removal supports outgoing direction only'),
    (0x29172e3, '4439f8', 'Removal requires statement source player'),
    (0x29172ec, '7443', 'Any destination short-circuits GetToPlayer'),
    (0x2917351, 'ebcc', 'Return removed pointer after list erase'),
    (0x2d2f51a, 'b9010000004531c0', 'Create(hash,owner,true,null)'),
    (0x2d2f52a, 'ff93d0000000', 'Actual manager Create virtual'),
    (0x2d2f536, '7448', 'Null factory result advances'),
    (0x2d2f556, '7408', 'Redundancy selects release versus InitializeText/Add'),
    (0x2d2f55b, 'ff5120', 'Redundant release virtual'),
    (0x2d2f563, 'ff9180000000', 'Nonredundant InitializeText virtual before Add'),
    (0x2d310c1, 'ff5030', 'Blocking calls actual IsValidForPhase'),
    (0x2d310ce, 'ff5050', 'Blocking checks actual blocking getter'),
    (0x2d310ee, '7603', 'Unsigned severity ties retain first candidate'),
    (0x2d31115, 'ff5050', 'Selected blocking getter re-evaluated'),
    (0x2d2e26b, '8b8b8c010000038b88010000', 'Expiry wrapping delta plus added'),
    (0x2d2e279, '0f9dc0', 'Expiry signed current-turn comparison'),
]
CALLS = [
    (0x2d2f5ac, 0x1e37ab0, "Capture configuration before accepts"),
    (0x2d2f5e4, 0x1e6ecd0, "Capture collection before network query"),
    (0x2d2f5ef, 0x1f4b370, "Retained configuration network getter"),
    (0x2d2f53e, 0x1e6ecd0, "Capture collection before redundancy/InitializeText/Add"),
    (0x2b5bd77, 0x1f85f10, "Capture Manager Get before owner GetID"),
    (0x2b5bd8d, 0x1fb0810, "Owner GetID for initial blocking"),
    (0x2b5bd9e, 0x1fb0810, "Owner GetID for real turn-complete lookup"),
    (0x2b5bdb7, 0x1f359d0, "Capture Manager Edit before owner GetID"),
    (0x2b5bdcd, 0x1fb0810, "Owner GetID for Update"),
    (0x2b5bdd9, 0x1f85f10, "Capture Manager Get before owner GetID after Update"),
    (0x2b5bdef, 0x1fb0810, "Owner GetID for final blocking"),
    (0x2b5be2a, 0x1fb0810, "Owner GetID after owner blocking store"),
    (0x2d28a8c, 0x1f2e780, 'Mandatory ProcessPending after Add'),
    (0x2913533, 0x1f07a60, 'Copy ever-alive player handle vector'),
    (0x2913579, 0x1f5f8e0, 'Read current IsAI once per player'),
    (0x2913590, 0x1f04c20, 'Repeated ProcessPendingSessionStatement'),
    (0x291197c, 0x1f48180, 'Remove before processing statement'),
    (0x291199d, 0x1f3ccf0, 'ProcessStatement provider'),
    (0x29172de, 0x1f70bb0, 'Statement GetFromPlayer'),
    (0x29172f1, 0x1f72ac0, 'Statement GetToPlayer'),
    (0x291734c, 0x1fae2d0, 'Erase pending list node'),
    (0x2d2f572, 0x1e19180, 'Add after InitializeText'),
    (0x2b5be54, 0x1e8db60, 'Blocking changed Type,int,int overload'),
]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def audit(module):
    game = Elf(module, GAME_SHA)
    for name, address, size in METHODS:
        if sha(game.read(address, size)) != EXPECTED_METHOD_SHA256[name]:
            raise ValueError("complete method fingerprint differs: " + name)
    for address, expected, meaning in BRANCHES + CHECKS:
        if game.read(address, len(bytes.fromhex(expected))).hex() != expected:
            raise ValueError('exact target branch differs: ' + meaning)
    for address, target, _ in CALLS:
        game.call(address, target)
    tail = game.read(0x2d28a9f, 5)
    if tail[0] != 0xe9 or 0x2d28aa4 + struct.unpack_from('<i', tail, 1)[0] != 0x1f93950:
        raise ValueError('mandatory owner blocking tail differs')
    return dict(
        schema='c6-linux-unit-operation-aftermath-v2', checks_passed=True,
        gamecore_sha256=GAME_SHA, static_only=True, native_execution=False,
        source_sha256=sha(SOURCE.read_bytes()),
        retained_receivers=['Configuration captured before accepts; collection before network query; retained on every Find',
                            'Collection captured after Create, retained for redundancy and Add across InitializeText',
                            'Manager Get/Edit/Get captured before each owner ID read; actual Owner retained for store/report',
                            'Definition collection and per-pass notification collection retained across callbacks',
                            'Native node cursor distinct from object identity; retain end cursor across callbacks'], audit_sha256=sha(Path(__file__).read_bytes()),
        inherited_branches_sha256=sha((ROOT / 'tools/linux/skip_turn_state_aftermath_audit.py').read_bytes()),
        methods=[dict(name=n, rva=hex(a), bytes=z, sha256=sha(game.read(a, z))) for n, a, z in METHODS],
        branch_checks=[dict(rva=hex(a), bytes=b, meaning=n) for a, b, n in BRANCHES + CHECKS],
        calls=[dict(rva=hex(a), target=hex(t), meaning=n) for a, t, n in CALLS],
        implemented=['Outgoing pending-statement first matching removal and ordered live session drain',
                     'Full notification Update control order including factory/null/redundancy branches',
                     'Live post-callback cloud gate and activation rescan control',
                     'Expiration scan with dismissed short circuit and next-link read after callback',
                     'Unsigned severity first-tie blocking reduction and final virtual getter reread',
                     'Owner complete gate, post-update reduction, store-before-local-report ordering',
                     'Verified base IsExpired wrapping signed arithmetic helper'],
        providers_required=['Actual ever-alive player handle inventory and mutable IsAI state',
                            'Full statement identities/payload/ownership, ProcessStatement routing/history/subtypes/RNG and external decisions',
                            'Actual definition array, factory registration and subtype Create/InitializeText/Add/Release',
                            'Actual notification list links, virtual predicates and activation/dismissal effects',
                            'Actual manager/configuration/player live gates and owner blocking storage',
                            'Reporting/cache/translator/prepublish/queued-event consumers and complete state/RNG effects'],
        reuse='Selection and logical cleanup providers may use notification_lifecycle only for its verified profile; combined activation_and_cleanup cannot replace Update.',
        next_provider_closure='Bind captured owner notification objects to concrete expiry/phase-valid/blocking virtual methods with live +130 blocking and actual current-turn input, then close SendAutoNotify factory inventory.',
        rollback='Only after_add with fully isolated owned Clone; lower level functions can mutate before error. Native/global/shared effects are outside clone isolation.',
        request_order='Prior TurnUnready and manager Add belong to caller. This boundary always follows Add when Execute found a unit, including rejected Add. No whole request/action implemented.',
        native_boundary_matrix_complete=False, actual_provider_parity=False,
        complete_affected_state_rng_parity=False, full_action_turn_parity=False,
        historical_save_load_raw_difference_unresolved_bytes=89198,
        attribution='docs/reference.md; exact target bytes authoritative; no original code executed.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--module', type=Path, default=ROOT.parent / 'c6-assets-linux/libGameCore_XP2.so')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = audit(args.module)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, static_only=True, native_execution=False)))


if __name__ == '__main__':
    main()
