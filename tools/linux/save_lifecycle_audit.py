#!/usr/bin/env python3
"""Verify bounded save lifecycle contracts in pinned original Linux ELF bytes.

Read-only static evidence; it never executes game code or asserts game loading.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct

HOST_SHA = '106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
GAME_SHA = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'


class Elf:
    def __init__(self, path, digest):
        self.raw = path.read_bytes()
        if hashlib.sha256(self.raw).hexdigest() != digest:
            raise ValueError(f'original Linux identity mismatch: {path.name}')
        if self.raw[:6] != b'\x7fELF\x02\x01':
            raise ValueError('requires little-endian ELF64')
        self.loads = []
        start = struct.unpack_from('<Q', self.raw, 32)[0]
        width, count = struct.unpack_from('<HH', self.raw, 54)
        for i in range(count):
            kind, _, offset, address, _, size, _, _ = struct.unpack_from('<IIQQQQQQ', self.raw, start + i * width)
            if kind == 1:
                self.loads.append((address, address + size, offset))

    def read(self, address, size):
        for lo, hi, offset in self.loads:
            if lo <= address and address + size <= hi:
                return self.raw[offset + address - lo:offset + address - lo + size]
        raise ValueError(f'not file-backed: {address:#x}')

    def relocated_pointer(self, address):
        section_start = struct.unpack_from('<Q', self.raw, 40)[0]
        section_width, section_count = struct.unpack_from('<HH', self.raw, 58)
        sections = [struct.unpack_from('<IIQQQQIIQQ', self.raw, section_start + i * section_width)
                    for i in range(section_count)]
        for section in sections:
            if section[1] != 4:  # SHT_RELA
                continue
            symbols = sections[section[6]]
            for pos in range(section[4], section[4] + section[5], section[9]):
                target, info, addend = struct.unpack_from('<QQq', self.raw, pos)
                if target != address:
                    continue
                if info & 0xffffffff == 8:  # R_X86_64_RELATIVE
                    return addend
                if info & 0xffffffff == 1:  # R_X86_64_64
                    symbol = symbols[4] + (info >> 32) * symbols[9]
                    return struct.unpack_from('<Q', self.raw, symbol + 8)[0] + addend
                raise ValueError('unsupported relocation type')
        raise ValueError('pointer relocation missing')

    def call(self, address, target):
        raw = self.read(address, 5)
        if raw[0] != 0xe8 or address + 5 + struct.unpack_from('<i', raw, 1)[0] != target:
            raise ValueError(f'call contract mismatch: {address:#x}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    host = Elf(args.assets / 'Civ6', HOST_SHA)
    game = Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    stages = [0x2fd21b7, 0x2fd2208, 0x2fd2226, 0x2fd239d,
              0x2fd2834, 0x2fd29ab, 0x2fd2b22, 0x2fd2b46,
              0x2fd2b56, 0x2fd2d03, 0x2fd2e7a, 0x2fd3038]
    table = 0x45effb0
    actual = [table + value for value in struct.unpack('<12i', host.read(table, 48))]
    if actual != stages:
        raise ValueError('native stage dispatch mismatch')
    calls = [
        (0x2fd35b2, 0x324aa40, 'stage4_LoadCurrentRequest'),
        (0x2fd37ef, 0x324ab80, 'stage5_GetCurrentRequestResult'),
        (0x2fd4cc4, 0x324ce20, 'stage5_ApplyModdingState'),
        (0x2fd335b, 0x337f600, 'stage2_ConfigureContent'),
        (0x2fd4c7e, 0x3381860, 'stage2_UpdateContent'),
        (0x2fd38d8, 0x31d1880, 'stage9_SetInitializingGameState'),
        (0x2fd38e2, 0x31d1700, 'stage9_DisableCacheUpdates'),
        (0x2fd3922, 0x324b3a0, 'stage9_Unserialize'),
        (0x2fd3b17, 0x31d1660, 'stage9_SetContext'),
        (0x2fd3b1f, 0x31d4bc0, 'stage9_InitializeCache'),
        (0x2fd500d, 0x3424990, 'stage9_PostGameInitialization'),
        (0x2fd5bd4, 0x31d1700, 'stage9_EnableCacheUpdates'),
        (0x2fd2e87, 0x31fe800, 'stage10_ApplyPending'),
        (0x2fd2ea5, 0x31cd2c0, 'stage10_GetLocalPlayer'),
        (0x2fd2eaf, 0x31fbce0, 'stage10_SetActiveUser'),
        (0x324cf6e, 0x324ca80, 'ApplyModdingState_LoadHeader'),
        (0x324bb27, 0x324ca80, 'UnserializeGameState_LoadHeader'),
        (0x324bd3d, 0x324cac0, 'UnserializeGameState_AttachToSession'),
        (0x324bf66, 0x324cb70, 'UnserializeGameState_LoadGameCoreState'),
        (0x324c17a, 0x324cc10, 'UnserializeGameState_LoadAudioState'),
        (0x324c38e, 0x324cc80, 'UnserializeGameState_LoadUIState'),
        (0x324c7d7, 0x324cce0, 'UnserializeGameState_LoadRecorderState'),
    ]
    for address, target, _ in calls:
        host.call(address, target)
    # Exact original caller immediates, not reconstructed C++ object fields.
    if host.read(0x2fd2e7f, 5) != bytes.fromhex('be01000000'):
        raise ValueError('ApplyPending true argument mismatch')
    if host.read(0x2fd391e, 4) != bytes.fromhex('31ff31f6'):
        raise ValueError('Unserialize null/default arguments mismatch')
    # Actual target Context vtable (address point follows offset/typeinfo).
    startup = game.relocated_pointer(0x3aeb9c0 + 16 + 0x40)
    if startup != 0x2c7d2f0:
        raise ValueError('target Context StartupGame slot mismatch')
    postinitialize = game.relocated_pointer(0x3aeb9c0 + 16 + 0x160)
    if postinitialize != 0x2c7d0c0 or host.read(0x2fd5254, 5) != bytes.fromhex('be01000000'):
        raise ValueError('target PostInitialize(true) contract mismatch')
    if host.read(0x3381c55, 7) != bytes.fromhex('4883782000741a'):
        raise ValueError('optional content callback guard mismatch')
    seek = struct.unpack('<Q', host.read(0x53cc710 + 16 + 0x28, 8))[0]
    if seek != 0x3c5412a or host.read(0x324cf79, 2) != bytes.fromhex('31f6'):
        raise ValueError('ApplyModdingState seek-zero contract mismatch')
    report = dict(schema=1, method='SHA-gated static original Linux ELF contract checks',
                  host_sha256=HOST_SHA, gamecore_sha256=GAME_SHA,
                  checks_passed=True,
                  stage_dispatch=[{'stage': i, 'entry': hex(a)} for i, a in enumerate(stages)],
                  direct_calls=[{'call': hex(a), 'target': hex(t), 'contract': n} for a, t, n in calls],
                  pending_argument=True, apply_modding_state_rewinds_stream=True, active_player_source='original GameCore::App::Cache::GetLocalPlayer',
                  context_startup_vtable_offset='0x40', context_startup_target=hex(startup),
                  context_postinitialize_vtable_offset='0x160', context_postinitialize_target=hex(postinitialize),
                  postinitialize_argument=True, update_content_callback_optional=True,
                  complete_save_load_executed=False, initialized_game=False, action_or_turn_parity=False)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'checks_passed': True, 'direct_calls_checked': len(calls)}))


if __name__ == '__main__':
    main()
