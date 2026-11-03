#!/usr/bin/env python3
"""Pin internal Linux completion kernel sources; never executes original code."""
import argparse
import hashlib
import json
from pathlib import Path
import struct

from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
from turn_progression_audit import METHODS, CALLS, BYTES, defined_relocations

ROOT = Path(__file__).resolve().parents[2]
PREFIX_METHODS = {
    0x2d27b00, 0x2d27b50, 0x2d28000, 0x2d28030, 0x2d28130,
    0x2b5b840, 0x2ba17b0, 0x2a5b640, 0x2a5b570, 0x2850fc0,
    0x2661ba0,
}
ACCESSORS = [
    (0x299e450, 8, 'Player.EditTurnManager', '180ed9bda300816a5b82f1b549d7ec97314d5197a1e4755b145c5ff39231c499'),
    (0x20a8cb0, 8, 'Player.EditCities', '91d1746a30132ffc2968016e3adacc19f461ee24a3c0510f47bea2428acc1e04'),
    (0x1fe4890, 12, 'City.EditBuildQueue', '3d5c3e93320fdc74310d1c2945a3e4c3b0d74a16d7b2f77477e476bd74013d32'),
    (0x1fe5090, 15, 'BuildQueue.Front const', 'b940fc6e45643749cdd6b825dec4dfc33d24f275379e2fac270358c637d9406b'),
    (0x1fe6b80, 4, 'BuildQueue.GetQueue const', 'f84e915a39a42129d4d8c8f1c771b4a320ba7b1ab0c17754721be4ff7b835a21'),
    (0x1fe71d0, 27, 'Queue.Front const', '79980c47d2ab7952bb82adb23e27d0d63269bc5dc9445c535493a49490ed910a'),
    (0x1fcdc10, 9, 'FixedPoint8.assign_int', '289ee59e91d4f68ddbe5f570b5002092702cd7aae484d25f21ca0576db55b96f'),
]
# Includes tail transfers. Each edge resolves the original ELF-defined GOT,
# independent of any interposition/stubs in a future native fixture.
EXTRA_EDGES = [
    (0x2b5b89d, 0x299e450, 'requested player EditTurnManager'),
    (0x2ba17b6, 0x20a8cb0, 'turn manager owner EditCities'),
    (0x2ba17c1, 0x2a5b640, 'cities completion hook'),
    (0x2a5b66c, 0x2a5b570, 'human forced/timer city overflow'),
    (0x2a5b5e8, 0x1fe5090, 'city queue Front'),
    (0x2a5b5f5, 0x1fe4890, 'city EditBuildQueue archive boundary'),
    (0x2a5b5fd, 0x2850fc0, 'clear overflow'),
    (0x2850fc9, 0x1fcdc10, 'fixed point zero assignment'),
    (0x1fe5091, 0x1fe6b80, 'Front GetQueue'),
    (0x1fe509a, 0x1fe71d0, 'Front queue delegate'),
    (0x2d27e97, 0x2021030, 'same player active reread'),
    (0x2d27ebc, 0x2661ba0, 'second Game.Update tail transfer'),
]


def edge(game, relocations, address, expected):
    raw = game.read(address, 5)
    if raw[0] not in (0xe8, 0xe9):
        raise ValueError(f'not direct call/jump: {address:#x}')
    plt = address + 5 + struct.unpack_from('<i', raw, 1)[0]
    raw = game.read(plt, 6)
    if raw[:2] != b'\xff\x25':
        raise ValueError(f'not original PLT: {address:#x}')
    got = plt + 6 + struct.unpack_from('<i', raw, 2)[0]
    if relocations.get(got) != expected:
        raise ValueError(f'original target mismatch: {address:#x}, actual={relocations.get(got)}')
    return dict(address=hex(address), plt=hex(plt), got=hex(got), target=hex(expected))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    game = Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    methods = [m for m in METHODS if m[0] in PREFIX_METHODS] + ACCESSORS
    if len(methods) != len(PREFIX_METHODS) + len(ACCESSORS):
        raise ValueError('missing upstream method identity')
    for address, size, name, expected in methods:
        if hashlib.sha256(game.read(address, size)).hexdigest() != expected:
            raise ValueError('original method identity differs: ' + name)
    relocations = defined_relocations(game)
    calls = [c for c in CALLS if any(a <= c[0] < a+n for a,n,_,_ in methods)]
    edges = [dict(edge(game, relocations, a, t), name=name) for a,_,_,t,name in calls]
    edges += [dict(edge(game, relocations, a, t), name=name) for a,t,name in EXTRA_EDGES]
    branches = [b for b in BYTES if any(a <= b[0] < a+n for a,n,_,_ in methods)]
    for address, expected, name in branches:
        if game.read(address, len(bytes.fromhex(expected))).hex() != expected:
            raise ValueError('branch bytes differ: ' + name)
    # Global bool array address in SetTurnComplete's RIP-relative LEA.
    if game.read(0x2b5b857, 3) != b'\x48\x8d\x3d':
        raise ValueError('completion array LEA differs')
    array = 0x2b5b85e + struct.unpack('<i', game.read(0x2b5b85a, 4))[0]
    if array != 0x3c06210:
        raise ValueError('completion array address differs')
    paths = ['crates/c6-core/src/turn_completion.rs',
             'tools/linux/turn_completion_kernel_audit.py',
             'docs/linux-turn-completion-kernel.md']
    report = dict(
        schema='c6-linux-turn-completion-kernel-v1', checks_passed=True,
        gamecore_sha256=GAME_SHA, required_future_host_sha256=HOST_SHA,
        methods=[dict(address=hex(a), size=n, name=s, sha256=h) for a,n,s,h in methods],
        resolved_edges=edges,
        branch_contracts=[dict(address=hex(a), name=s) for a,_,s in branches],
        global_completion_array=dict(address=hex(array), length=64, element='bool',
            owner='Player::ms_abIsTurnComplete; not serialized Player field'),
        source_sha256={p: hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in paths},
        implementation=dict(
            prefix='unsigned encoded range; paused human log/report; duplicate log; setter; required Update',
            setter='write global bool; true loops all 64 numeric valid slots, rereading requested player alive and invoking same requested hook',
            forced_timer='human reread per hook; recycled city cursor/end rereads; typed BuildQueue entries/overflow',
            archive='mandatory edit_build_queue provider; exact archive/dirty effects remain unimplemented',
            continuation='mandatory first complete Game.Update, then same-player active reread, optional second complete Game.Update',
            effect_providers='no no-op defaults; errors may follow internal mutation',
            public_Engine_apply='unchanged; must reject before mutation'),
        native_execution=False, native_differential_fixture_available=False,
        acceptance=dict(full_action_acceptance=0, affected_state_parity=False,
            gameplay_rng_parity=False, next_external_decision_boundary_proven=False,
            full_gameplay_state_action_turn_supported=False),
        remaining=['exact reporting/log service effects and subscribers',
            'City FAutoVariable edit archive/dirty ownership',
            'full mandatory Game.Update and nested exact rules/upkeep',
            'all external player decisions, including NPC/offturn/diplomacy/automation',
            'complete logical state and gameplay RNG',
            'unclassified/unmasked raw Save->Load 89,198-byte failure'],
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, methods=len(methods), resolved_edges=len(edges),
                          native_execution=False, full_action_acceptance=0)))


if __name__ == '__main__':
    main()
