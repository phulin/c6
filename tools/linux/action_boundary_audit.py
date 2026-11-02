#!/usr/bin/env python3
"""Read-only ABI audit of pinned Linux action, queue, turn and core-save boundaries."""
import argparse
import json
from pathlib import Path
from save_lifecycle_audit import Elf, HOST_SHA, GAME_SHA


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    host = Elf(args.assets / 'Civ6', HOST_SHA)
    game = Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    calls = [
        (0x2c7d4a8, 0x1f600a0, 'RequestAction_constructs_original_empty_parameters'),
        (0x2c7d4b8, 0x1f6d610, 'RequestAction_forwards_to_parameter_overload'),
        (0x2c7dca4, 0x1f71e70, 'EndTurn_constructs_PlayerTurnComplete'),
        (0x2c7dcac, 0x1fb9798, 'EndTurn_broadcasts_original_message'),
        (0x2d27e82, 0x1e61790, 'PlayerTurnComplete_SetTurnComplete'),
        (0x2d27e8f, 0x1f906b0, 'PlayerTurnComplete_calls_GameUpdate'),
        (0x2661ba1, 0x1e0c3e0, 'GameUpdate_calls_ProcessorUpdate'),
        (0x2661ba6, 0x1f2e780, 'GameUpdate_processes_pending_diplomacy'),
        (0x2a0ccf5, 0x1f299a0, 'GameProcessor_updates_player'),
        (0x2b5c7b5, 0x1f107f0, 'InactivePlayerProcessor_calls_original_AI_DoTurn'),
        (0x2b5acbc, 0x1e232c0, 'ActivePlayerProcessor_calls_TurnManager_DoTurn'),
        (0x2ba16c7, 0x1e87800, 'TurnManager_checks_original_IsHuman'),
        (0x201ed3c, 0x1f3c7f0, 'InactiveAI_dispatches_original_offturn_signal'),
        (0x2b31542, 0x1f72ff0, 'IsHuman_resolves_actual_player_configuration'),
        (0x2a0d290, 0x1f4b450, 'GameProcessor_waits_original_AI_jobs'),
        (0x2d24916, 0x1e43120, 'QueueDrain_receives_front_message'),
        (0x2d2491e, 0x1f392d0, 'QueueDrain_pops_front_after_receive'),
        (0x2d24935, 0x1ef19d0, 'QueueDrain_sends_reporting_event'),
        (0x28061de, 0x1ee11f0, 'UnitRequest_checks_actual_cache_turn'),
        (0x2806224, 0x1eb7fb0, 'UnitRequest_initializes_original_message'),
        (0x280622e, 0x1fb9798, 'UnitRequest_broadcasts_original_message'),
        (0x290bc46, 0x1e45900, 'CoreSave_serializes_game'),
        (0x290bc55, 0x1eb94e0, 'CoreSave_serializes_tutorial'),
        (0x290bc61, 0x1e20f90, 'CoreSave_serializes_achievements'),
    ]
    for address, target, _ in calls:
        game.call(address, target)
    host_calls = [(0x31d11d8, 0x3c402d8, 'Worker_enters_original_critical_section'),
                  (0x31d1232, 0x31d4e70, 'Worker_marks_pass_complete'),
                  (0x31d1252, 0x3c402de, 'Worker_leaves_original_critical_section'),
                  (0x31d49e8, 0x31d2440, 'InlinePump_gets_controller_lock'),
                  (0x31d4a2c, 0x31d2a00, 'InlinePump_releases_controller_lock'),
                  (0x31d4abd, 0x33cee30, 'UpdateRequest_requires_session_host')]
    for address, target, _ in host_calls:
        host.call(address, target)
    checks = [
        (game, 0x2ba16cc, '84c074025bc3', 'TurnManager_human_returns_before_AI'),
        (game, 0x28ec236, '83f8030f94c0', 'Configuration_IsHuman_slot_taken'),
        (game, 0x28ebd80, 'be03000000', 'Original_SetAsHuman_uses_slot_taken'),
        (game, 0x2c7d682, '81fbb5a6c92c', 'ACTION_ENDTURN_hash'),
        (game, 0x2c7dc94, 'ba8412fab6', 'EndTurn_default_subtype_NONE'),
        (game, 0x2d27e7a, 'be01000000', 'SetTurnComplete_true'),
        (game, 0x2c7e490, '5085f67505e8163f21ff31c059c3', 'ContextUpdate_only_autoplay'),
        (game, 0x2806203, 'ff9500010000', 'UnitRequest_manager_MakeRequest_slot100'),
        (game, 0x2d2494c, '7592', 'QueueDrain_repeats_until_empty'),
        (host, 0x31d1213, 'ff9038020000', 'Worker_ProcessMessages_slot238'),
        (host, 0x31d4a0e, 'ff5030', 'InlinePump_GameUpdate_slot30'),
        (host, 0x31d4b1a, 'befbb986b1', 'Host_requests_ACTION_GAME_UPDATE'),
    ]
    for elf, address, expected, name in checks:
        raw = bytes.fromhex(expected)
        if elf.read(address, len(raw)) != raw:
            raise ValueError(f'original byte contract mismatch: {name}')
    slots = [(0x3aeb9c0, 0x168, 0x2c7d230, 'Context_Save'),
             (0x3aeb9c0, 0x238, 0x2c7d330, 'Context_ProcessMessages'),
             (0x3aeb9c0, 0x240, 0x2c7d340, 'Context_HasPendingMessages'),
             (0x3aeb9c0, 0x1f8, 0x2c7d490, 'Context_RequestAction'),
             (0x3aeb9c0, 0x200, 0x2c7d4f0, 'Context_RequestAction_parameters'),
             (0x3aeb9c0, 0x208, 0x2c7e2c0, 'Context_RequestUnitOperation'),
             (0x3aecf50, 0x30, 0x2c862e0, 'Game_Update'),
             (0x3af94c0, 0x80, 0x290bbf0, 'XP2Context_Write')]
    for table, offset, target, name in slots:
        if game.relocated_pointer(table + 16 + offset) != target:
            raise ValueError(f'original virtual contract mismatch: {name}')
    report = dict(schema=1, method='SHA-gated read-only static original Linux ELF ABI audit',
                  host_sha256=HOST_SHA, gamecore_sha256=GAME_SHA, checks_passed=True,
                  direct_calls=[dict(module=m, call=hex(a), target=hex(t), contract=n)
                                for m, rows in [('gamecore', calls), ('host', host_calls)] for a, t, n in rows],
                  byte_contracts=[n for _, _, _, n in checks],
                  virtual_contracts=[dict(table=hex(t), offset=hex(o), target=hex(a), contract=n) for t, o, a, n in slots],
                  endturn_action='0x2cc9a6b5', default_turn_subtype='0xb6fa1284',
                  message_drain_can_invoke_native_AI=True,
                  initialized_game=False, native_action_executed=False,
                  complete_affected_state_observed=False, action_or_turn_parity=False)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, direct_calls_checked=len(calls)+len(host_calls), virtual_slots_checked=len(slots))))


if __name__ == '__main__':
    main()
