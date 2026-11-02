#!/usr/bin/env python3
"""Read-only target identity audit; does not authorize or execute a core ingress."""
import argparse
import hashlib
import json
from pathlib import Path

from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA


METHODS = [
    (0x2c7d310, 0xa, 'Context.ReceiveMessage', 'b52610eb5ea18672e649718a5c83b81327e336999be775d3c73240a626a011b7'),
    (0x2c7d320, 0xa, 'Context.QueueMessage', '7c686b7138469f0f44944a5f8cdd9d289096a77a532355c5ef85fa10b3015feb'),
    (0x2d243c0, 0x2d5, 'Manager.ReceiveMessage', '52072c90d2067de7900f6512bd21b5cda7e05b29725cfbf9884ff9942c47b40f'),
    (0x2d246a0, 0x193, 'Manager.QueueMessage', '5407b3ef656046402f404e86df714d784558f6bb4bf4cb493f373144727ab342'),
    (0x2d248a0, 0xd2, 'Manager.ProcessMessages', '6301f5545748c2ef4283c4b74c5eeb0d834231b49eabc4ccb37975e7da4303b2'),
    (0x2d28a00, 0xb0, 'UnitOperation.Execute', '5855bdbd2ae7837445bd71020d7b9f11f713997b487f3ba8b5dfc65c7d046e26'),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    core = Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    host = Elf(args.assets / 'Civ6', HOST_SHA)
    for address, size, name, expected in METHODS:
        if hashlib.sha256(core.read(address, size)).hexdigest() != expected:
            raise ValueError('method identity differs: ' + name)
    for offset, target in [(0x228, 0x2c7d310), (0x230, 0x2c7d320)]:
        if core.relocated_pointer(0x3aeb9c0 + 16 + offset) != target:
            raise ValueError('Context virtual binding differs')
    calls = [
        (0x2d24446, 0x1e26c70, 'Receive deserializes registered message'),
        (0x2d246cc, 0x1eb3bc0, 'Queue native push_back'),
        (0x2d24702, 0x1e28dc0, 'Queue copies native stream bytes'),
        (0x2d24916, 0x1e43120, 'Process receives native front'),
        (0x2d2491e, 0x1f392d0, 'Process pops native front'),
        (0x2d24935, 0x1ef19d0, 'Process sends native reporting event'),
        (0x2d28a8c, 0x1f2e780, 'UnitOperation processes pending diplomacy'),
    ]
    for address, target, _ in calls:
        core.call(address, target)
    # These wrappers tail-jump (E9); Elf.call deliberately accepts only calls.
    tail_jumps = [(0x2c7d315, 0x1e43120), (0x2c7d325, 0x1ecb330)]
    for address, target in tail_jumps:
        raw = core.read(address, 5)
        if raw[0] != 0xe9 or address + 5 + int.from_bytes(raw[1:], 'little', signed=True) != target:
            raise ValueError('Context forwarding tail jump differs')
    host.call(0x33af874, 0x33afab0)
    for address, expected in [(0x33afab0, '8b4728c3'), (0x33af86a, '4c8bb830020000')]:
        if host.read(address, len(bytes.fromhex(expected))) != bytes.fromhex(expected):
            raise ValueError('host sender/queue forwarding contract differs')
    result = dict(
        schema='c6-linux-core-oracle-boundary-static-v1', checks_passed=True,
        host_sha256=HOST_SHA, gamecore_sha256=GAME_SHA,
        method='SHA-gated read-only original ELF bytes; no native execution',
        methods=[dict(address=hex(a), size=n, name=s, sha256=h) for a, n, s, h in METHODS],
        direct_calls=[dict(address=hex(a), target=hex(t), contract=s) for a, t, s in calls],
        tail_jumps=[dict(address=hex(a), target=hex(t)) for a, t in tail_jumps],
        context_receive_slot='0x228', context_queue_slot='0x230',
        net_message_from_id_getter='0x33afab0; i32 this+0x28',
        receive_source_id_role='debug/log formatting only in this exact Manager.ReceiveMessage body',
        native_execution=False, action_executed=False, host_acceptance_proven=False,
        replacement_provider_equivalence_proven=False, gameplay_parity=False,
        limitations=['No universal source authorization established',
                     'No live sender captured by this audit',
                     'No proof host deferral is presentation-only',
                     'No all-player policy continuation or complete affected-state proof'])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, native_execution=False,
                          replacement_provider_equivalence_proven=False)))


if __name__ == '__main__':
    main()
