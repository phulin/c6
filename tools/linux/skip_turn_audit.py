#!/usr/bin/env python3
"""Verify bounded SKIP_TURN semantics in the exact original Linux XP2 ELF.

Static evidence only: never executes native code or establishes action parity.
"""
import argparse
import hashlib
import json
from pathlib import Path

from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA


METHODS = [
    (0x2DAC8E0, 0x69, 'SkipTurn.Start'),
    (0x2DAC950, 0xDA, 'SkipTurn.CanStart'),
    (0x2DACA30, 0x18, 'SkipTurn.MakeRequest'),
    (0x2DA1840, 0x63, 'Manager.SetActivityType'),
    (0x2DA1A20, 0x292, 'Manager.Add'),
    (0x2DA1CC0, 0xB0, 'Manager.PopFront'),
    (0x2DA20F0, 0x243, 'Manager.CanStart'),
    (0x2DA2340, 0xF9, 'Manager.Start'),
    (0x2DA25E0, 0x94, 'Manager.InsertAtEnd'),
    (0x2DA2680, 0x88, 'Manager.DeleteNode'),
    (0x2DA2710, 0x73, 'Manager.Clear'),
    (0x2DA2790, 0x222, 'Manager.Activate'),
    (0x2DA29C0, 0x60, 'Manager.Deactivate'),
    (0x2D964C0, 0x5C, 'BaseHandler.Deactivate'),
    (0x2D96520, 0x77, 'IsReadyToSelect'),
    (0x2D96720, 0xF6, 'BaseHandler.MakeRequest'),
    (0x2DA4010, 0x3C, 'Activity.CopyAssignmentDo'),
    (0x2D28A00, 0xB0, 'UnitOperation.Execute'),
    (0x2913510, 0xB2, 'Diplomacy.ProcessPending'),
    (0x2911900, 0xA6, 'Diplomacy.ProcessPendingSessionStatement'),
    (0x29119B0, 0x3A1, 'Diplomacy.ProcessStatement'),
    (0x2917970, 0x41, 'Diplomacy.Session.Process'),
    (0x291B9D0, 0x24C, 'Diplomacy.ProposeAction.Process'),
    (0x2B5BD70, 0x9B, 'Player.UpdateEndTurnBlockingState'),
    (0x2B5BE10, 0x52, 'Player.SetEndTurnBlockingState'),
    (0x2D2F360, 0x67, 'Notifications.Update'),
]
SLOTS = [
    (0x3AFF870, 0x50, 0x2DA20F0), (0x3AFF870, 0x68, 0x2DA1A20),
    (0x3AFF870, 0x70, 0x2DA2710), (0x3AFF870, 0xC0, 0x2DA17E0),
    (0x3AFF870, 0xC8, 0x2DA1840), (0x3AFF870, 0x130, 0x2DA19E0),
    (0x3AFF870, 0x138, 0x2DA2340), (0x3AFF870, 0x158, 0x2DA1CC0),
    (0x3AFF870, 0x168, 0x2DA25E0), (0x3AFF870, 0x170, 0x2DA2680),
    (0x3AFF870, 0x178, 0x2DA2790), (0x3AFF870, 0x180, 0x2DA29C0),
    (0x3B00DA0, 0x10, 0x2D95650), (0x3B00DA0, 0x18, 0x2DAC950),
    (0x3B00DA0, 0x48, 0x2DAC8E0), (0x3B00DA0, 0x60, 0x2D956C0),
    (0x3B00DA0, 0x68, 0x2D964C0), (0x3AF62F0, 0x98, 0x2D2F360),
    (0x3AF62F0, 0xA0, 0x2D2F330), (0x3AF62F0, 0x108, 0x2D2F4A0),
    (0x3AF62F0, 0x110, 0x2D2F4B0), (0x3AF62F0, 0x118, 0x2D2F5A0),
    (0x3AF62F0, 0x120, 0x2D2F640),
]
CHECKS = [
    (0x2DACA50, 'b80819682cc3', 'operation_hash'),
    (0x265B300, '8b8790020000c3', 'activity_value_offset'),
    (0x2DAC92A, 'ba8b047c04', 'skip_sets_hold'),
    (0x2DAC93B, '41c60601', 'start_data_bDelete_true'),
    (0x2DAC9F3, '772a', 'stack_count_unsigned_above_limit_rejects'),
    (0x213CAD9, '66c7431c0000', 'default_request_append_and_user_false'),
    (0x2DA30A0, '66c7070000c3', 'start_data_delete_and_action_false'),
    (0x2DA185B, '488dbb80020000', 'activity_archive_wrapper_offset'),
    (0x2DA2A25, '488dbe18050000', 'timer_archive_wrapper_offset'),
    (0x2DA2A50, '488d86100c0000c3', 'queue_value_offset'),
    (0x2B5BE24, '899fa8070000', 'player_end_turn_blocking_offset'),
    (0x2DA2778, 'e923090fff', 'clear_formation_path_cache_tail'),
    (0x2DA2703, 'e998090fff', 'delete_final_node_formation_path_cache_tail'),
]
CALLS = [
    (0x2DAC8FF, 0x1EE2720, 'operation_start_event_type'),
    (0x2DAC91D, 0x1FB5910, 'operation_start_event_send'),
    (0x2DA1867, 0x1EB6B60, 'activity_tracked_assignment'),
    (0x2DA1897, 0x1FB5910, 'activity_changed_event_send'),
    (0x2DA229B, 0x1F41E00, 'legality_script_CallTestAll'),
    (0x2D28A8C, 0x1F2E780, 'message_pending_diplomacy'),
    (0x2913590, 0x1F04C20, 'pending_session_statement_loop'),
    (0x291197C, 0x1F48180, 'remove_pending_statement'),
    (0x291199D, 0x1F3CCF0, 'process_removed_statement'),
    (0x291798B, 0x1F8CAD0, 'session_append_statement_history'),
    (0x2917998, 0x1EBDB70, 'session_remove_matching_pending_statement'),
    (0x291BA3A, 0x1EF0400, 'propose_action_calls_diplomatic_response_policy'),
    (0x291BA51, 0x1E4C710, 'policy_result_sends_gameplay_action'),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    game = Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    host = Elf(args.assets / 'Civ6', HOST_SHA)
    if game.read(0x26B0D10, 5) != bytes.fromhex('488d4708c3'):
        raise ValueError('Unit.GetAutoArchive mismatch')
    if host.read(0x3C46FCA, 9) != bytes.fromhex('48837f48000f95c0c3'):
        raise ValueError('FAutoArchive.hasDeltas mismatch')
    if host.read(0x3C45F41, 6) != bytes.fromhex('4989f54989fe'):
        raise ValueError('getDeltaIndexes hidden return forwarding mismatch')
    for address, expected, name in CHECKS:
        raw = bytes.fromhex(expected)
        if game.read(address, len(raw)) != raw:
            raise ValueError('instruction mismatch: ' + name)
    for address, target, _ in CALLS:
        game.call(address, target)
    for table, slot, target in SLOTS:
        if game.relocated_pointer(table + 16 + slot) != target:
            raise ValueError(f'virtual slot mismatch: {table:#x}+{slot:#x}')
    report = dict(
        schema='c6-linux-skip-turn-contracts-v1', gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA,
        checks_passed=True, native_execution=False, action_executed=False,
        rust_action_implemented=False, full_affected_state_contract_complete=False,
        methods=[dict(address=hex(a), size=s, name=n, sha256=hashlib.sha256(game.read(a, s)).hexdigest())
                 for a, s, n in METHODS],
        instruction_contracts=[dict(address=hex(a), contract=n) for a, _, n in CHECKS],
        direct_calls=[dict(address=hex(a), target=hex(t), contract=n) for a, t, n in CALLS],
        virtual_slots=[dict(vtable=hex(t), offset=hex(s), target=hex(a)) for t, s, a in SLOTS],
        archive_observer=dict(unit_get_auto_archive='0x26b0d10',
                              hidden_return_storage_bytes=24, hidden_return_register='RDI', archive_register='RSI',
                              order='Native dirty-pointer set iteration mapped to original variable indexes',
                              methods=[dict(address=hex(a), size=z, name=n, sha256=hashlib.sha256(host.read(a,z)).hexdigest())
                                       for a,z,n in [(0x3c46fca,9,'FAutoArchive.hasDeltas'),
                                                     (0x3c45f34,0xd9,'FAutoArchive.getDeltaIndexes'),
                                                     (0x3c45d9e,0xe4,'FAutoArchive.calculateDeltaHash'),
                                                     (0x327ee90,4,'Vector.uint.size'),
                                                     (0x327eea0,0xf,'Vector.uint.const_index'),
                                                     (0x3283350,0x2c,'Vector.uint.destructor')]]),
        typed_fields=[
            dict(owner='Unit.CInstance', name='m_eAutomation', wrapper='0x268', value='0x278', size=4),
            dict(owner='Unit.CInstance', name='m_eActivity', wrapper='0x280', value='0x290', size=4),
            dict(owner='Unit.Instance', name='m_iOperationTimer', wrapper='0x518', value='0x528', size=4),
            dict(owner='Unit.Instance', name='m_kOperations', wrapper='0xc00', value='0xc10', size=200),
            dict(owner='Player.Instance', name='end_turn_blocking_value', value='0x7a8', size=4,
                 naming='Processor setter symbol and native store; enclosing member DWARF not audited'),
            dict(owner='Operations.StartData', name='bDelete', value='0x0', size=1),
            dict(owner='Operations.StartData', name='bAction', value='0x1', size=1),
        ],
        rng='No direct RNG call in audited SkipTurn leaf; callbacks/diplomacy not proven RNG-neutral.',
        unresolved=['Native action and deterministic branch restore', 'All reporting/cache subscribers and policy reachability',
                    'Complete diplomacy response subtype contracts and RNG; ProposeAction response policy is proven reachable', 'Notification subtype callbacks and complete mutations',
                    'Formation path-cache representation and archive/cache notification ownership'],
        private_evidence='../c6-ghidra/evidence/linux-skip-turn/',
        scope='Bounded static contracts, not full Civ VI state/action/turn parity.')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, native_execution=False, action_executed=False)))


if __name__ == '__main__':
    main()
