#!/usr/bin/env python3
"""Read-only exact Linux worker ordering and lock-bearing facade audit."""
import argparse
import hashlib
import json
import struct
from pathlib import Path
from save_lifecycle_audit import Elf, HOST_SHA, GAME_SHA

METHODS = [('host', 0x31d4eb0, 8, 'GameCoreThread.IsExecuting', '7b7b0e671e24137d96430c4b1a0222d01e1a665cf71924a4c5c5e095daed7e7d'), ('host', 52250960, 312, 'Controller.ExecuteGameCoreUpdate', '514193e9d85d1b34d22f6053b548e0ba240de42ea22c34a4d2068265eb870dd6'), ('host', 52238128, 46, 'Controller.Synchronize', 'd9945d3a3834f6145e016aa41b57fbaec681e6bf3efed34dda8b7cc3ec868502'), ('host', 52238496, 26, 'Controller.IsExecuting', '116689463607c34540ee990b53566ad86b1f6ee2f010cffb8586ae97312519cc'), ('host', 52236624, 126, 'GameCoreThread.C1', '43ad2f8e5b2e867900eb05ede5c635215d95c59af32746dc8c1eb411d53ce4bd'), ('host', 52236752, 217, 'GameCoreThread.Run', '11b63e8fb242e3d2ac85a2115084ac106eacfe43fd27fdff6960dd257cc54a9d'), ('host', 52252240, 24, 'GameCoreThread.SetExecuting', '1fd93ce7c8f39680c4e77cf3eb5ca60fc9372c2ed28c17782985bf1ec9f694ca'), ('host', 52252272, 19, 'GameCoreThread.SetComplete', '0e85fba1ecea919abc139064bd9b6c6c16bb5753d2ffafc67a200ab42ab4a861'), ('host', 52252384, 51, 'GameCoreThread.JoinOnPassComplete', '7fe50455eb5ffa67411b507d0f96d8bb6b74aa255f573990cae0a7e7fbc28d6b'), ('host', 52252352, 18, 'GameCoreThread.IsCurrentThread', '442f45e7ca73b2ab79a71f17e819a08adbe03fc1a2d725af529a2eb1c5038981'), ('host', 52252464, 24, 'GameCoreThread.IsSuspended', '271bb6d24c728983de1b881ac6a37fb205371c43987a370cfd6c86972c6099c7'), ('host', 52252496, 49, 'GameCoreThread.Resume', '0ccff73f784314a9dcc06bffb8ce3b4010b1dba30a44601d9650feb48e0a512f'), ('host', 52255120, 4, 'GameCoreThread.NeedsCompletionEvent', 'dcdc7ecf6eeec6762f6f6d9eef71504a67bdc6711accb1f12f42106417e7fbef'), ('host', 52251792, 83, 'Controller.SendProcessingComplete', '423e83d2634f6aaca97ac99f5760c28dd3e0d0e0fc5f6568b73cafbdb0a48221'), ('host', 53165376, 22, 'EngineUtility.GetGameCoreLock', '9394d1eaa3cd10aad54c138f38dbe484621b0f48d78dfd14fb0e14b1de441406'), ('host', 53165408, 22, 'EngineUtility.ReleaseGameCoreLock', '257d7c9d356de1f649b0ca2394024341abe3b88c655630f0f43df9cda22d670f'), ('gamecore', 46685408, 92, 'Game.live.C1', '981ad8a20e2991f4cb6211bc5b349ea550efa2dbc6d59625e4d9b69897b2d519'), ('gamecore', 46685568, 56, 'Game.D1/D2', '9fd728c0f0fe56b64e0fbacaeb0c28e9f73723000c86c57ec6e708ee5f3ae1b3'), ('gamecore', 46711520, 92, 'Player.live.C1', 'b691f5e6f4de4881da8aad97d7ccba10c4b29a1e688a51ffa24497986d6beb56')]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    host = Elf(args.assets / 'Civ6', HOST_SHA)
    game = Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    for module, address, size, name, digest in METHODS:
        elf = host if module == 'host' else game
        if hashlib.sha256(elf.read(address, size)).hexdigest() != digest:
            raise ValueError('original worker/facade method differs: ' + name)
    calls = [(0x31d49bd,0x31d1150),(0x31d49cc,0x31d4e50),
             (0x31d11aa,0x3c44ac0),(0x31d11d8,0x3c402d8),
             (0x31d1232,0x31d4e70),(0x31d1252,0x3c402de),
             (0x31d4eff,0x3c402d8),(0x31d4f07,0x3c402de)]
    for address, target in calls:
        host.call(address, target)
    checks = [(host,0x31d1213,'ff9038020000','original Context.ProcessMessages slot'),
              (host,0x31d1736,'84c07415','Synchronize returns early when not executing'),
              (host,0x31d11a4,'41b801000000','original suspended thread option'),
              (game,0x2c85d31,'ff6140','live Game ctor acquires engine lock'),
              (game,0x2c85dab,'ff5138','live Game destructor releases engine lock'),
              (game,0x2c8c331,'ff6140','live Player ctor acquires engine lock')]
    for elf, address, expected, name in checks:
        if elf.read(address,len(bytes.fromhex(expected))) != bytes.fromhex(expected):
            raise ValueError(name)
    for offset, target in [(0x40,0x32b3d40),(0x38,0x32b3d60)]:
        if struct.unpack('<Q',host.read(0x53a6c80+16+offset,8))[0] != target:
            raise ValueError('actual engine lock interface binding differs')
    report = dict(schema='c6-linux-original-worker-ordering-v1',host_sha256=HOST_SHA,gamecore_sha256=GAME_SHA,checks_passed=True,
        methods=[dict(module=m,address=hex(a),size=n,name=k,sha256=d) for m,a,n,k,d in METHODS],
        calls=[dict(address=hex(a),target=hex(t)) for a,t in calls],
        counter='original Execute CAS0->1 before Resume; original Run Context.ProcessMessages then SetComplete decrements0 while holding worker lock',
        facade_lifetime='live Game and Player facades acquire controller lock; release acquired live facades before scheduling worker, reacquire after completion',
        synchronize='Controller.Synchronize returns immediately when IsExecuting=false; it is not a post-false lock barrier',
        conditional_wait='one original Execute, bounded wait on genuine executing predicate, then original worker.JoinOnPassComplete shared-lock barrier on actual borrowed worker (not current thread)',
        predicate_access='original IsExecuting performs an aligned native 32-bit comparison; SetExecuting and SetComplete use original interlocked operations, not replacement C atomic fields',
        completion_flag='first fresh constructor zeros flag45; later flag is consumer notification, not independently reset per pass by SetExecuting',
        native_execution=False,gameplay_parity=False,
        limits='Static ordering proves valid scheduling conditions; it does not prove actual OS-thread creation/start success. Non-quiescent failure captures are diagnostic, not coherent snapshots or successful cleanup.')
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,native_execution=False)))


if __name__ == '__main__':
    main()
