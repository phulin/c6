#!/usr/bin/env python3
"""Read-only exact Linux worker ordering and lock-bearing facade audit."""
import argparse
import hashlib
import json
import struct
from pathlib import Path
from save_lifecycle_audit import Elf, HOST_SHA, GAME_SHA

METHODS = [('host', 0x31d4eb0, 8, 'GameCoreThread.IsExecuting', '7b7b0e671e24137d96430c4b1a0222d01e1a665cf71924a4c5c5e095daed7e7d'), ('host', 52250960, 312, 'Controller.ExecuteGameCoreUpdate', '514193e9d85d1b34d22f6053b548e0ba240de42ea22c34a4d2068265eb870dd6'), ('host', 52238128, 46, 'Controller.Synchronize', 'd9945d3a3834f6145e016aa41b57fbaec681e6bf3efed34dda8b7cc3ec868502'), ('host', 52238496, 26, 'Controller.IsExecuting', '116689463607c34540ee990b53566ad86b1f6ee2f010cffb8586ae97312519cc'), ('host', 52236624, 126, 'GameCoreThread.C1', '43ad2f8e5b2e867900eb05ede5c635215d95c59af32746dc8c1eb411d53ce4bd'), ('host', 52236752, 217, 'GameCoreThread.Run', '11b63e8fb242e3d2ac85a2115084ac106eacfe43fd27fdff6960dd257cc54a9d'), ('host', 52252240, 24, 'GameCoreThread.SetExecuting', '1fd93ce7c8f39680c4e77cf3eb5ca60fc9372c2ed28c17782985bf1ec9f694ca'), ('host', 52252272, 19, 'GameCoreThread.SetComplete', '0e85fba1ecea919abc139064bd9b6c6c16bb5753d2ffafc67a200ab42ab4a861'), ('host', 52252384, 51, 'GameCoreThread.JoinOnPassComplete', '7fe50455eb5ffa67411b507d0f96d8bb6b74aa255f573990cae0a7e7fbc28d6b'), ('host', 52252352, 18, 'GameCoreThread.IsCurrentThread', '442f45e7ca73b2ab79a71f17e819a08adbe03fc1a2d725af529a2eb1c5038981'), ('host', 52252464, 24, 'GameCoreThread.IsSuspended', '271bb6d24c728983de1b881ac6a37fb205371c43987a370cfd6c86972c6099c7'), ('host', 52252496, 49, 'GameCoreThread.Resume', '0ccff73f784314a9dcc06bffb8ce3b4010b1dba30a44601d9650feb48e0a512f'), ('host', 52255120, 4, 'GameCoreThread.NeedsCompletionEvent', 'dcdc7ecf6eeec6762f6f6d9eef71504a67bdc6711accb1f12f42106417e7fbef'), ('host', 52251792, 83, 'Controller.SendProcessingComplete', '423e83d2634f6aaca97ac99f5760c28dd3e0d0e0fc5f6568b73cafbdb0a48221'), ('host', 53165376, 22, 'EngineUtility.GetGameCoreLock', '9394d1eaa3cd10aad54c138f38dbe484621b0f48d78dfd14fb0e14b1de441406'), ('host', 53165408, 22, 'EngineUtility.ReleaseGameCoreLock', '257d7c9d356de1f649b0ca2394024341abe3b88c655630f0f43df9cda22d670f'), ('gamecore', 46685408, 92, 'Game.live.C1', '981ad8a20e2991f4cb6211bc5b349ea550efa2dbc6d59625e4d9b69897b2d519'), ('gamecore', 46685568, 56, 'Game.D1/D2', '9fd728c0f0fe56b64e0fbacaeb0c28e9f73723000c86c57ec6e708ee5f3ae1b3'), ('gamecore', 46711520, 92, 'Player.live.C1', 'b691f5e6f4de4881da8aad97d7ccba10c4b29a1e688a51ffa24497986d6beb56')]

METHODS += [('host', 63196348, 358, 'WinThread.Initialize', '45639250796014ae3218421a827a8a988870038064786d22aaafdafaeb589834'), ('host', 63196706, 35, 'WinThread.StartSuspendedRoutine', 'f6b9454f1fe499fed4a1ed67f3b3a039a1c06000d1c808617e01ddccb8a39150'), ('host', 63196748, 209, 'WinThread.StartRoutine', 'ed309d8a643bf6e94894f4e49ec3ba7d2693ea123d46cb3fc5fd62ef0f623cbd'), ('host', 63196958, 12, 'WinThread.IsValid', '5a6d239ca7e789cc02ab2a5b4df68b73cf377df9ded7a99870e59b400e09886e'), ('host', 63196970, 34, 'WinThread.IsRunning', 'e3e08586655ccf6ae401fc949fd0fca73c4b7d56db8945c5963397796f194301'), ('host', 63197004, 97, 'WinThread.Start', 'ae2b851ac7e0f68b02945bb2b839f8941df58146093378651479f7f7d5d02572'), ('host', 63197448, 7, 'WinThread.IsStarted', 'd41288f39d5006fe9812e7a4cafe92b16184b1dcf072bfbc9cae4961e2ef7038'), ('host', 63177410, 4, 'CriticalSection.GetLockCount', '4628fb538250bfc88b327da2acefeb54924dfb050ac98dcea105347fb07d6367'), ('host', 63177414, 4, 'CriticalSection.GetRecursionCount', '123844b3d139f0b75833926319edcaa7dacf7529e5ca1b5d6afa59806bc75758')]

METHODS += [('host', 47061732, 5, 'ASL_WinCreateThreadWithName.forward', '771c44122d32595cefdb5326128eb103233da727601f043f32d7acc398477687'), ('host', 47053323, 155, 'ASL.WinCreateThreadWithName', '250b3e5649fe0e7e19aefa2ee1066b7d1ddc86d1381226691bd3f3c17b89ed54'), ('host', 47053478, 596, 'ThreadHANDLE.C1', '407c89860f010120275d87ecc26fa6e96164b25cc4a2b84b46f47a67e92a2f6c'), ('host', 47053024, 20, 'ASL.PrepareMainThread', 'c0ad799f23f9919f8ec5f96436586c6e44ff81e684fa6bcb0ce590630116a3b3'), ('host', 47754877, 21, 'GetLastError', '84e2989c3b214b2db7831416e7c6aa625e51969cdb4d20d3405cda197ee8ee8f'), ('host', 63195840, 140, 'Platform.CreateThread.normal_path', '456b15680cd5575be59fb232facbf9a58c96334f6e8339b6671ea03860dd2ad9')]

METHODS += [('host', 47052416, 607, 'ASL.OnCreateThread', '0fee3329532cd3c2ad9db6890bb61bb8f7ad976ae8c045e8c28458826ea3d008'), ('host', 47045129, 5, 'ASL.EmuPreMain', '2c03e9d7be424baca419d2b2d2635f8a31710ee6f5979d7728c59ba31b3ddd0d'), ('host', 46862714, 588, 'ASL.AppBase.AfterConstruct', '89ccd68257de66f4af936e6b804fe9cbd83f4f6528cd6977c228a203a4520e29'), ('host', 46857552, 231, 'ASL.AppBase.Run', '8f9c2316d7ca03ecf4e322a553d7a4e90f1be4ffe744b4849a5ef7f5ddcb4adc'), ('host', 47045134, 718, 'ASL.EmuMain', '8332507ada6e9c23e21533dad22660d64c6279ade704811865caff1b4096c160'), ('host', 50041424, 166, 'WinMain', '25036f2ce475c9bd937512c02412d6a0e54743b077de34306ea25649afb3abbb'), ('host', 69946064, 913, 'AppHost.SetupFXSPlatform', '39f46bb0156f8340e691a7fd1468029aebec1a5862c033b56e5706b8ca628da5'), ('host', 47751051, 159, 'ASL.DLLMainOnThread', 'c418678caaaedc8408461d4ecf2b4864dcc288daf9cccbb19867bf532f343843'), ('host', 47045852, 17, 'ASL.EmuQuit', 'ba2ae3b23ab924c12f03833c271e6a32d7bacc47611932a7721142265267a6f2'), ('host', 46857133, 419, 'ASL.Exit', '3300064d9d3a0c27fea8520ab2cf9408c8e60796220c65c95e26db39f82674dc')]


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
             (0x31d4eff,0x3c402d8),(0x31d4f07,0x3c402de),(0x2cb1373,0x2cdda09),
             (0x2cafde4,0x2cdda0e),(0x2fb9291,0x2f8bcd0),
             (0x2fb92a7,0x42b9800),(0x42b4d94,0x3c3ee86)]
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
        startup_diagnostics=dict(worker_global='0x6499108',worker_thread_pointer_offset='0x48',thread_os_handle_offset='0xb0',thread_start_event_offset='0xc0',thread_id_offset='0xc8',thread_started_offset='0xcd',thread_initialized_offset='0xce',native_create_return='0x31d11af',os_create_return='0x3c44df4',worker_run='0x31d11d0',delegate_call='0x3c44eac',pre_start_callback='Platform.JobManager.RegisterMasterThread'),
        early_lifecycle=dict(abi='PrepareMainThread void(void); OnCreateThread void(char const*)',ordering='ASL App.AfterConstruct -> EmuPreMain -> PrepareMainThread; later App.Run -> EmuMain creates game-entry thread -> WinMain -> Civ6App and AppHost setup/TempHeap',duplicate='not an idempotent setter: every invocation calls OnCreateThread and DLLMainOnThread(2), even with existing TLS identity',lifetime='process scoped main-thread preparation; no paired main-thread reset found in audited EmuQuit/Exit; never use worker endthread on caller',prospective_profile='fresh original startup before Civ6App/TempHeap/XP2 activation, after actual gate observation; not mid-loaded-Core'),
        creation_failure=dict(admission_byte='host0x544cc70 bit0',admission_branch='0x2cdfa2d; clear returnsNULL without pthread call',native_admission_lifecycle='ASL.PrepareMainThread exchanges admission byte then original OnCreateThread; not executed by this audit',pthread_attr_init_return='0x2cdfbd5',pthread_detach_return='0x2cdfbe5',pthread_stack_return='0x2cdfc0b; return unchecked by original',pthread_create_return='0x2cdfc25',requested_stack='original worker option1 yields ASL request0; original ThreadHANDLE normalizes to max(request,1048576)',affinity='no affinity operation in audited constructor path',error_propagation='admission rejection does not set error; pthread_create error is overwritten before later GetLastError, which reads separate TLS error'),
        lock_diagnostics=dict(worker_lock='worker+0',controller_lock='controller+0x460',controller_recorded_owner='controller+0x4a0',controller_recorded_depth='controller+0x4a4',owner_identity='Platform.GetCurrentThreadID; not assumed equal to Linux LWP'),
        native_execution=False,gameplay_parity=False,
        limits='Static ordering proves valid scheduling conditions; it does not prove actual OS-thread creation/start success. Non-quiescent failure captures are diagnostic, not coherent snapshots or successful cleanup.')
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,native_execution=False)))


if __name__ == '__main__':
    main()
