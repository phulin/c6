#!/usr/bin/env python3
"""Check original Linux offline session and core-load prerequisites without execution."""
import argparse
import json
from pathlib import Path
import struct
from save_lifecycle_audit import Elf, HOST_SHA, GAME_SHA


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    host = Elf(args.assets / 'Civ6', HOST_SHA)
    game = Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    calls = [
        (0x2f91d78, 0x2fa0a00, 'SetupNetwork_CreateManager'),
        (0x2f91d85, 0x33bd600, 'SetupNetwork_InitializeManager'),
        (0x2f91d92, 0x33d85e0, 'SetupNetwork_SubscribeToEvents'),
        (0x2f91da6, 0x33d7930, 'SetupNetwork_SetupFromCommandLine'),
        (0x33bd60e, 0x33e0670, 'NetworkInitialize_CreateFireWire'),
        (0x33bd613, 0x33e06f0, 'NetworkInitialize_CreateArchiveFactory'),
        (0x3a017e4, 0x3a02658, 'FireWireConstructor_CreateGameSync'),
        (0x324cb0e, 0x2fa09f0, 'AttachToSession_GetNetworkManager'),
        (0x324cb1a, 0x33bddf0, 'AttachToSession_HostGame'),
        (0x33be00b, 0x33be130, 'HostGame_InitSession'),
        (0x33be18f, 0x33b8410, 'InitSession_ConstructOriginalCallbacks'),
        (0x33be19c, 0x3a01e28, 'InitSession_FireWireCreateSession'),
        (0x3a01ff6, 0x39e4cf4, 'AllocateSession_ConstructFNetSession'),
        (0x39e554f, 0x39e2fc6, 'FNetSessionHost_CreateLocalPlayer'),
        (0x324cb90, 0x3c60bc0, 'LoadGameCoreState_ConstructZLibIStream'),
        (0x324cbae, 0x3c60bfa, 'LoadGameCoreState_DestroyZLibIStream'),
        (0x33bd337, 0x33bd420, 'NetworkDestructor_UnsubscribeEvents'),
        (0x33bd33c, 0x33e05f0, 'NetworkDestructor_FreeGameSync'),
        (0x33bd341, 0x33e0610, 'NetworkDestructor_FreeArchiveFactory'),
        (0x33bd346, 0x33e0630, 'NetworkDestructor_FreeFireWire'),
    ]
    for address, target, _ in calls:
        host.call(address, target)
    shutdown_calls = [
        (0x2fbbf23, 0x33bfdc0, 'GameViewShutdown_LeaveGame'),
        (0x2fbc0bf, 0x31d1400, 'GameViewShutdown_ControllerUninitialize'),
        (0x2fbc0c4, 0x31dd810, 'GameViewShutdown_CoreShutdownGame'),
        (0x2f98190, 0x3385460, 'GUIShutdown_FrameworkDestroy'),
        (0x2f98195, 0x31d4d80, 'GUIShutdown_ControllerShutdown'),
        (0x2f9819a, 0x2fa05c0, 'GUIShutdown_LuaDestroy'),
        (0x2f9861d, 0x3c2f1cf, 'FinalShutdown_NetworkShutdown'),
        (0x2f98622, 0x2fa0640, 'FinalShutdown_NetworkManagerFree'),
        (0x337f3cf, 0x31ead60, 'FrameworkDestructor_ShutdownConfiguration'),
        (0x337f3d4, 0x337f5a0, 'FrameworkDestructor_UnloadGameCore'),
        (0x337f5ae, 0x31d16c0, 'FrameworkUnload_ReleaseControllerContext'),
        (0x337f5b3, 0x31dd630, 'FrameworkUnload_UnloadDLL'),
        (0x31d1428, 0x31cd1e0, 'ControllerUninitialize_UninitializeAppCache'),
        (0x31d1430, 0x31d16c0, 'ControllerUninitialize_ReleaseContext'),
    ]
    for address, target, _ in shutdown_calls:
        host.call(address, target)
    checks = [
        (0x324cb13, '31f631d2', 'AttachToSession_HostGame_server0_false'),
        (0x324cb37, '31c0', 'AttachToSession_returns_OK_independent_of_host_result'),
        (0x324cb88, 'ba01000000', 'LoadGameCoreState_ZLib_framing_true'),
        (0x324cba1, 'ff9070010000', 'LoadGameCoreState_Context_Load_slot'),
        (0x33d7651, '85f6', 'CanInitSession_tests_transport'),
        (0x33d7674, 'b001', 'CanInitSession_transport0_allowed'),
        (0x39de6a8, '837b4801', 'TransportThread_created_only_for_transport1'),
        (0x33b8cb3, '83f803', 'OfflinePlayerSelection_slot_status_taken'),
        (0x33b8cda, '4183ff16', 'OfflinePlayerSelection_scan_bound22'),
    ]
    for address, expected, name in checks:
        if host.read(address, len(bytes.fromhex(expected))) != bytes.fromhex(expected):
            raise ValueError(f'original byte contract mismatch: {name}')
    if struct.unpack('<i', host.read(0x461ec30, 4))[0] != 0:
        raise ValueError('ServerType0 transport table mismatch')
    if struct.unpack('<Q', host.read(0x53a6c80 + 16 + 0xd0, 8))[0] != 0x32b4e00:
        raise ValueError('original engine local player callback mismatch')
    if game.read(0x26619f2, 6) != bytes.fromhex('ff91d0000000'):
        raise ValueError('Game InitializeLocalPlayer engine slot mismatch')
    game.call(0x2d7afbf, 0x1e0d8c0)
    load = game.relocated_pointer(0x3aeb9c0 + 16 + 0x170)
    shutdown = game.relocated_pointer(0x3aeb9c0 + 16 + 0x48)
    if shutdown != 0x2c7d300:
        raise ValueError('actual target Context ShutdownGame virtual mismatch')
    if load != 0x2c7d240:
        raise ValueError('actual target Context Load virtual mismatch')
    result = dict(schema=1, method='SHA-gated static original Linux ELF contract checks',
                  host_sha256=HOST_SHA, gamecore_sha256=GAME_SHA, checks_passed=True,
                  direct_calls=[dict(call=hex(a), target=hex(t), contract=n) for a, t, n in calls],
                  shutdown_calls=[dict(call=hex(a), target=hex(t), contract=n) for a, t, n in shutdown_calls],
                  byte_contracts=[n for _, _, n in checks], server_type0_transport=0,
                  engine_network_local_player_callback='0x32b4e00',
                  local_player_initialized_during_game_read=True,
                  game_read_initialize_local_player_call='0x2d7afbf',
                  game_initialize_local_player_plt='0x1e0d8c0',
                  context_load_vtable_offset='0x170', context_load_target=hex(load), context_shutdown_game_target=hex(shutdown),
                  attaches_via_original_host=True, adapter_selects_player_id=False,
                  native_session_executed=False, initialized_game=False,
                  complete_save_load_executed=False, action_or_turn_parity=False)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'checks_passed': True, 'direct_calls_checked': len(calls)}))


if __name__ == '__main__':
    main()
