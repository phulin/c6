#!/usr/bin/env python3
"""Static SHA-gated original Linux notification occurrence boundary checks."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import Elf, GAME_SHA

ROOT = Path(__file__).resolve().parents[2]
METHODS = [
    ('PlayerNotifications.Find', 0x2d30940, 0xac),
    ('PlayerNotifications.Add', 0x2d30b90, 0xaf),
    ('Instance.SetID', 0x2d2e3e0, 0x24),
    ('Instance.SetAddedTurn', 0x2d2e410, 0x19),
    ('PlayerNotifications.Cleanup', 0x2d311e0, 0xcb),
    ('Instance.Release', 0x2d2de30, 0x42),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    game = Elf(ROOT.parent / 'c6-assets-linux/libGameCore_XP2.so', GAME_SHA)
    checks = [
        (0x2d30bac, '83f8ff', 'added turn sentinel -1'),
        (0x2d30bc0, '8b338b5304', 'stored collection owner and old next ID'),
        (0x2d30bc5, '8d4201894304', 'wrapping increment before SetID'),
        (0x2d30be6, 'ff9090000000', 'required post-append virtual callback'),
        (0x2d30c03, 'becf3a6ff2', 'added-event type'),
        (0x2d2e421, '898388010000', 'current added turn stored'),
        (0x2d31252, 'ff5020', 'Release before list erase'),
    ]
    for address, expected, meaning in checks:
        if game.read(address, len(bytes.fromhex(expected))).hex() != expected:
            raise ValueError(meaning)
    calls = [
        (0x2d30bcb, 0x1f2ec80, 'SetID'),
        (0x2d30bd9, 0x1e7d230, 'ordered push_back'),
        (0x2d30bf9, 0x1e413a0, 'capture ID after virtual callback'),
        (0x2d30c15, 0x1eaf290, 'report captured ID'),
        (0x2d30c22, 0x1f8ad80, 'expire superseded after report'),
        (0x2d31269, 0x1eda6b0, 'erase released list occurrence'),
    ]
    for address, target, _ in calls:
        game.call(address, target)
    sha = lambda raw: hashlib.sha256(raw).hexdigest()
    result = dict(
        schema='c6-linux-notification-occurrences-source-v1', checks_passed=True,
        gamecore_sha256=GAME_SHA, original_execution=False,
        methods=[dict(name=name, rva=hex(address), size=size,
                      sha256=sha(game.read(address, size))) for name, address, size in METHODS],
        checked_sites=[dict(rva=hex(address), bytes=raw, meaning=meaning)
                       for address, raw, meaning in checks],
        checked_calls=[dict(rva=hex(address), target=hex(target), meaning=meaning)
                       for address, target, meaning in calls],
        owner_source_sha256=sha((ROOT / 'crates/c6-core/src/player_notifications_owner.rs').read_bytes()),
        checker_sha256=sha(Path(__file__).read_bytes()),
        storage_contract='stable owner-relative occurrence serials, ordered append/removal, explicit omitted runtime fields; logical live object identity remains separate',
        storage_only=True, native_add_release_implemented=False,
        original_native_comparison=False, complete_state_actions_rng_turns=False,
        open=['actual factory and InitializeText', 'source-ordered Add and synchronous callbacks',
              'actual Release/pool/destructor effects', 'omitted loaded runtime fields',
              'complete affected gameplay state/RNG comparison'])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, checked_sites=len(checks), checked_calls=len(calls), original_execution=False)))


if __name__ == '__main__':
    main()
