#!/usr/bin/env python3
"""Pin generic notification command-aftermath branches; static evidence only.

No transition implementation, native execution, fixture-ID rules, policy choice
or complete state/event coverage is supplied by this contract.
"""
import argparse
import json
from pathlib import Path
import struct

from core_component_check import digest, require
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
from skip_turn_state_success import original_contract, sha, ROOT


BRANCHES = [
    (0x2b5bdb5, '7546', 'Owner turn-complete skips notification Update'),
    (0x2d2f006, 'ff5050', 'AcceptsNotifications calls IsInitialized virtual slot50'),
    (0x2d2f00b, '7411', 'Uninitialized rejects notifications'),
    (0x2d2f01c, '7404', 'Non-uninitializing game continues to human configuration predicate'),
    (0x2d2f36b, 'ff9010010000', 'Update begins SendAutoNotify'),
    (0x2d2f380, '7412', 'Non-PlayByCloud permits autoactivation'),
    (0x2d2f389, '7417', 'PlayByCloud missing game skips autoactivation'),
    (0x2d2f392, '750e', 'PlayByCloud nonlocal owner skips autoactivation'),
    (0x2d2f39c, 'ff9018010000', 'Conditional ProcessAutoActivate'),
    (0x2d2f3aa, 'ff9008010000', 'Update then DismissExpired'),
    (0x2d2f3b3, '488b8020010000', 'Update tail invokes Cleanup'),
    (0x2d2f5bd, 'ff5068', 'ProcessAutoActivate requires AcceptsNotifications'),
    (0x2d2f5c2, '7468', 'Rejected notifications stop activation pass'),
    (0x2d2f5dd, '744d', 'Inactive player stops activation pass'),
    (0x2d2f5f7, '8d2c40', 'Severity threshold equals network multiplayer bool times3'),
    (0x2d2f603, '31f6', 'Virtual Activate receives false'),
    (0x2d2f608, 'ff9088000000', 'Invoke candidate actual Activate virtual slot88'),
    (0x2d2f62a, '74d4', 'Unlocked candidate activates then selection repeats; locked stops'),
    (0x2d30f44, 'be6f9762c3', 'Optional severity override keyc362976f'),
    (0x2d30f6e, '7505', 'True override bypasses signed severity comparison'),
    (0x2d30f73, '7c2b', 'Signed below-threshold severity rejects candidate'),
    (0x2d30f7f, '753d', 'Eligible locked record returns immediately'),
    (0x2d30f84, '751a', 'Keep first eligible unlocked candidate'),
    (0x2d30f90, '4c0f44ed', 'Retain candidate only when HasBeenActivated is false'),
    (0x2d30fbb, '4c89ed', 'Return retained candidate after complete scan'),
    (0x2d2e200, '753d', 'IsAutoActivate returns true for already-activated record'),
    (0x2d2e211, '7521', 'Network case uses native19d flag'),
    (0x2d2e232, '740b', 'Non-network existing definition without single-player icon returns true'),
    (0x2d2e234, '80bb9d01000000', 'Remaining IsAutoActivate cases use native bool19d'),
    (0x2d31195, '7519', 'DismissExpired skips already-dismissed record'),
    (0x2d3119d, 'ff5028', 'Call actual virtual expiration predicate'),
    (0x2d311a2, '740c', 'Not expired advances without Dismiss'),
    (0x2d311aa, 'ff9098000000', 'Expired invokes actual virtual Dismiss'),
    (0x2d31245, '7439', 'Cleanup retains non-dismissed record'),
    (0x2d31252, 'ff5020', 'Cleanup calls actual virtual Release before erase'),
    (0x2b5be22, '7435', 'Equal blocking state skips store and report'),
    (0x2b5be24, '899fa8070000', 'Store new blocking state at native7a8'),
    (0x2b5be3a, '751d', 'Nonlocal owner skips blocking-change report'),
    (0x2b5be41, 'be7b5af155', 'Blocking-change report type55f15a7b'),
]


def manager_and_reporting_contract(game, host):
    require(game.relocated_pointer(0x3affa30 + 16 + 0x68) == 0x2da4b40, 'MoveTo concrete deactivation differs')
    require(game.read(0x2c43a70, 7).hex() == '8a87d8060000c3', 'movement completion input differs')
    require(game.read(0x2c42ac1, 2).hex() == '7441', 'CompleteMovement false branch differs')
    require(game.read(0x2da4b64, 5).hex() == 'e9579618ff', 'MoveTo base deactivation tail differs')
    require(game.read(0x2da272c, 2).hex() == '744f', 'Clear empty-queue return differs')
    require(game.read(0x2da29ec, 5).hex() == 'ba2e01256b', 'Deactivate hard-coded AWAKE differs')
    require(struct.unpack('<Q', host.read(0x53a6c80 + 16 + 0xa8, 8))[0] == 0x32b4260,
            'original host engine reporting provider binding differs')
    for call, target in [(0x2da4b51, 0x1f63180), (0x2c42aba, 0x1e6f150),
                         (0x2c42ac8, 0x1f45ca0), (0x2c42aff, 0x1f302f0),
                         (0x293d8eb, 0x1e1fbe0), (0x293d8f3, 0x1e135b0), (0x293d902, 0x1ea2b90),
                         (0x2d4a4db, 0x1f148f0), (0x2d4a4e0, 0x1f310a0),
                         (0x2723565, 0x1e47f00), (0x2723619, 0x1f435f0)]:
        game.call(call, target)
    for call, target in [(0x32b4b97, 0x32b5f90), (0x32b4c03, 0x3214c50),
                         (0x32176cf, 0x2fd7db0), (0x32176df, 0x3228580),
                         (0x32176ea, 0x32285c0), (0x32176f1, 0x2fd7e20),
                         (0x321cadf, 0x2fd7db0), (0x321caef, 0x3237c40),
                         (0x321cafa, 0x3237c80), (0x321cb01, 0x2fd7e20)]:
        host.call(call, target)
    require(host.read(0x3214cbf, 2).hex() == 'ffd0', 'synchronous registered translator call differs')
    game_methods = [('MoveTo.Deactivate', 0x2da4b40, 0x29), ('Unit.CompleteMovement', 0x2c42ab0, 0x5c),
                    ('Unit.GetNeedsMovementCompletion', 0x2c43a70, 7), ('Formation.ClearPathCache', 0x293d8e0, 0x30),
                    ('Reporting.SendTypedMap', 0x2d4a4b0, 0x4f), ('Reporting.SendTypedVectorMap', 0x2d4a500, 0x8f),
                    ('Cache.Dispatcher.Send', 0x2723550, 0xee)]
    host_methods = [('EngineUtility.SendEvent', 0x32b4260, 0x9bc), ('Translate.Dispatch', 0x3214c50, 0x81),
                    ('GameCoreEvent71.Publish', 0x32176b0, 0x50), ('GameCoreEvent257.Publish', 0x321cac0, 0x50)]
    return dict(gamecore_methods=[dict(name=n,rva=hex(a),bytes=z,sha256=sha(game.read(a,z))) for n,a,z in game_methods],
                host_methods=[dict(name=n,rva=hex(a),bytes=z,sha256=sha(host.read(a,z))) for n,a,z in host_methods],
                clear_initial_empty='Return without deactivation, timer assignment or Formation clear',
                manager_deactivate='For nonempty queue: OPERATION -> hard-coded AWAKE; then tracked timer=0. Node saved activity is not restored.',
                move_to_deactivate='CompleteMovement(Unit*) then BaseHandler.Deactivate',
                complete_movement_false='Return immediately when actual Unit m_bNeedsMovementCompletion is false',
                complete_movement_true='Tracked Unit index50=false, then identity/PlotCoord event6e7ef927; unsupported until modeled',
                formation_clear=dict(ordered_dirty_fields=[14,15], effects=['Clear path', 'Set cache coordinates -9999,-9999'],
                                     evaluating_operation_directly_written=False),
                one_move_to_replacement_path_clear_calls=['Old DeleteNode final removal', 'Clear tail', 'New SkipTurn DeleteNode final removal'],
                path_clear_sequence_scope='Instruction-derived supported branch with unchanged subscriber inputs; actual invocation counts were not captured',
                native_path_clear_invocation_count_observed=False,
                reporting_order=['Enabled/not-unserializing gates', 'Synchronous matching Cache.Dispatcher delegates if updates enabled',
                                 'Original EngineUtility.SendEvent parameter-based dirty-component bookkeeping',
                                 'Synchronous registered host translator if found',
                                 'Relevant GameCoreEvent.Publish optional synchronous pre-publish delegate',
                                 'Create queued host GameCore event and Unlock'],
                original_provider_binding=dict(vtable='0x53a6c80', slot='0xa8', target='0x32b4260'),
                unresolved_profile_gates=['Actual reporting enabled and unserializing values', 'Actual Cache.AreUpdatesEnabled',
                                          'Matching Cache handler targets and receiver state', 'Actual host translator registration table',
                                          'Actual pre-publish subscriber pointers/targets', 'All transitive callback effects'],
                current_profile_pure_queued_reporting_proven=False,
                current_profile_synchronous_gameplay_mutation_proven=False,
                live_callback_inventory_observed=False,
                limits='Static synchronous dispatch and queued-event branches are both proven. Neither absence nor gameplay effects of current live callbacks are established by method names, matching Save bytes or the historical selected-event trace.')


def audit(module, host_path=None):
    contract = original_contract(module)
    elf = Elf(module, GAME_SHA)
    host = Elf(host_path or ROOT.parent / 'c6-assets-linux/Civ6', HOST_SHA)
    for address, expected, name in BRANCHES:
        require(elf.read(address, len(bytes.fromhex(expected))).hex() == expected,
                'original branch differs: ' + name)
    methods = dict(accepts=0x2d2f000, initialized=0x2d2ef40, update=0x2d2f360,
                   get_blocking=0x2d2f330, dismiss_expired=0x2d2f4a0,
                   send_auto=0x2d2f4b0, process_auto=0x2d2f5a0, cleanup=0x2d2f640)
    slots = [(0x50, 'initialized'), (0x68, 'accepts'), (0x98, 'update'),
             (0xa0, 'get_blocking'), (0x108, 'dismiss_expired'), (0x110, 'send_auto'),
             (0x118, 'process_auto'), (0x120, 'cleanup')]
    manager_tables = [('base', 0x3af62f0), ('XP2', 0x3afb650)]
    for owner, table in manager_tables:
        for offset, name in slots:
            require(elf.relocated_pointer(table + 16 + offset) == methods[name],
                    owner + ' notification manager binding differs: ' + name)
    elf.call(0x2b5be54, 0x1e8db60)
    elf.call(0x2d31269, 0x1eda6b0)
    for call, target in [(0x2d2f015, 0x1f4ba10), (0x2d2f035, 0x1f363a0)]:
        if call == 0x2d2f035:
            # Tail jump to the actual human-configuration getter.
            require(elf.read(call, 5).hex() == 'e9667320ff', 'configuration human tail differs')
        else:
            elf.call(call, target)
    return dict(schema='c6-linux-generic-command-notification-aftermath-static-v1', checks_passed=True,
                static_only=True, native_execution=False, implementation=False,
                gamecore_sha256=GAME_SHA, source_audit_sha256=digest(Path(__file__)),
                host_sha256=HOST_SHA, manager_and_reporting=manager_and_reporting_contract(elf, host),
                dependency_source_sha256=digest(ROOT / 'tools/linux/skip_turn_state_success.py'),
                original_methods=contract['methods'] + [dict(name='Player.SetEndTurnBlocking', rva='0x2b5be10', bytes=0x52,
                                                            sha256=sha(elf.read(0x2b5be10, 0x52)))],
                branch_checks=[dict(rva=hex(address), bytes=expected, meaning=name) for address,expected,name in BRANCHES],
                manager_virtual_slots=[dict(offset=hex(offset), method=name, target=hex(methods[name])) for offset,name in slots],
                verified_manager_vtables=[dict(owner=owner, rva=hex(table)) for owner,table in manager_tables],
                required_inputs=['Actual owner global turn-complete flag', 'Actual manager player-notification vector initialization',
                                 'Actual game uninitializing flag', 'Owner human configuration and turn-active state',
                                 'PlayByCloud and network multiplayer configuration', 'Manager game pointer/local owner',
                                 'Definition iteration order, IsAutoNotify and ShowIconSinglePlayer values',
                                 'Registered notification factories and subtype virtual behavior', 'Original notification list order',
                                 'Notification type/severity/native19d/HasBeenActivated and typed parameters',
                                 'Omitted live signed lock count native194', 'Expiration predicates and actual game turn',
                                 'Old blocking enum native7a8 and actual local owner', 'Reporting subscribers and external decision boundaries'],
                update_order=['SendAutoNotify', 'Conditional ProcessAutoActivate', 'DismissExpired', 'Cleanup'],
                selection=dict(scan_order='native linked-list order', threshold='network multiplayer ? 3 : 0',
                               eligibility='nonnull && IsAutoActivate && (signed severity >= threshold || bool variant c362976f)',
                               locked='signed native int194 > 0; not serialized',
                               locked_priority='First eligible locked record returns immediately, preempting any retained unlocked candidate',
                               unlocked_priority='First eligible unlocked record with HasBeenActivated=false is retained; continue scanning',
                               dismissed_filter=False, activation_argument=False,
                               rescan_after_each_virtual_activation=True,
                               is_auto_activate='HasBeenActivated || (!network && definition exists && !ShowIconSinglePlayer) || native bool19d'),
                cleanup=dict(predicate='IsDismissed()', order='native list order', effects=['virtual Release', 'erase current node'],
                             subtype_or_id_filter=False),
                conditional_unobserved_reporting=[dict(event='025f6cfd', call='0x2d2dde4', overload='Type,Value,Value',
                                                       arguments=['notification identity keyeab69e5a', 'actual activation bool key78a2f946']),
                                                  dict(event='55f15a7b', call='0x2b5be54', overload='Type,int,int', arguments=['old blocking enum', 'new blocking enum'])],
                native_live_inventory_complete=False, native_boundary_matrix_complete=False,
                all_event_subscriber_effects_known=False, complete_rng_inventory=False,
                full_host_admission=False, gameplay_parity=False,
                limits='General exact-target instruction-derived branch contract, not an accepted transition. Observed removed IDs are never cleanup rules. Wire omits live lock count; factories, expiration/activation subtypes, dynamic manager gates, reporting subscribers and policy continuations need exact live evidence. Original fixture footprint is a separately identified captured result with historical reporting ABI limits.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--module', type=Path, default=ROOT.parent / 'c6-ghidra/evidence/linux-rules/asl-launch-xp2-2/libGameCore_XP2.so')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--host', type=Path, default=ROOT.parent / 'c6-assets-linux/Civ6')
    args = parser.parse_args()
    result = audit(args.module, args.host)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True, static_only=True, branch_checks=len(BRANCHES), implementation=False)))


if __name__ == '__main__':
    main()
