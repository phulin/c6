#!/usr/bin/env python3
"""Read-only exact native constructors and separate conditional component audit."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

METHODS = [('gamecore', 34851536, 17, 'Request.C2', '4a8f110dd12df8a0ee7aaedc522d75dda536e424d9706e7059044a3061feaa19'), ('gamecore', 34851520, 9, 'Operation.D2', '602378b0f64b1db59b55f1d15e148f2e1fc33dec4f30cb6f94be62391d4d1251'), ('gamecore', 47352176, 87, 'UnitOperation.C1', 'fb70f24028076bb9119deaba4b4c3b71679f4c39863803e912906ad65ee16891'), ('gamecore', 41974224, 36, 'UnitOperation.D2', '196172e9e2ce538cda2ab2e96ac3a8f30062a2442f748c602ae01cd793adacaa'), ('gamecore', 47352272, 43, 'UnitOperation.Init', 'fdbe5cf384997c15cce153cf8e9b4ef0490e46f3392739c5e5c23077777d4ac2'), ('gamecore', 47320768, 44, 'BaseMessage.PutInBuffer', '45ba35a1b209ebc81f658cadd47f42dc9257266d106046c525c504571d5d3674'), ('gamecore', 47353232, 105, 'UnitOperation.CreateMessage', '58439c21466aaf8442a15a8e07d9df4db7d565b24ef32aa2f36880fb4cb62836'), ('gamecore', 47845984, 26, 'Manager.Get', '247a0c981c4169f399c86ecce00bb2cc63f7100c890337325450e592e3addcd4'), ('host', 63258740, 23, 'MemoryIStream.C1', 'ee1372ebf02a88dba5631cc87b99f29de0c7cd10224d25479ac81f6c418dd4eb'), ('host', 63258798, 23, 'MemoryIStream.D1', 'c3b703c6ab391d7e736677ab4da31b38b23b2f85fd9d4cbdd3ea550242c8e651'), ('host', 63259438, 23, 'MemoryOStream.C1.fixed', '1e99e438f390367ea3ae2b713bb866cbd1aab575a566a4d56c1e2572e6ab1fd3'), ('host', 63259802, 36, 'MemoryOStream.D0', '4cad00229cd6ced4c283ae1d3fc75e4092a7fed7b44227412ce84587357aee98'), ('host', 54320512, 35, 'Network.GetLocalPlayerID', 'f0df704c64005d2fae6fe60f23835b4d8b4dcccdbdbac202bca44c1611e75a0a')]

METHODS += [
    ('host', 0x31d4950, 0x138, 'Controller.ExecuteGameCoreUpdate', '514193e9d85d1b34d22f6053b548e0ba240de42ea22c34a4d2068265eb870dd6'),
    ('host', 0x31d4e50, 0x18, 'Thread.SetExecuting', '1fd93ce7c8f39680c4e77cf3eb5ca60fc9372c2ed28c17782985bf1ec9f694ca'),
    ('host', 0x31d11d0, 0xd9, 'Thread.Run', '11b63e8fb242e3d2ac85a2115084ac106eacfe43fd27fdff6960dd257cc54a9d'),
    ('host', 0x31d4e70, 0x13, 'Thread.SetComplete', '0e85fba1ecea919abc139064bd9b6c6c16bb5753d2ffafc67a200ab42ab4a861'),
    ('host', 0x31d18a0, 0x1a, 'Controller.IsExecuting', '116689463607c34540ee990b53566ad86b1f6ee2f010cffb8586ae97312519cc'),
    ('host', 0x31d1730, 0x2e, 'Controller.Synchronize', 'd9945d3a3834f6145e016aa41b57fbaec681e6bf3efed34dda8b7cc3ec868502'),
    ('host', 0x31d4ee0, 0x33, 'Thread.JoinOnPassComplete', '7fe50455eb5ffa67411b507d0f96d8bb6b74aa255f573990cae0a7e7fbc28d6b'),
    ('host', 0x31d1150, 0x7e, 'Thread.Constructor', '43ad2f8e5b2e867900eb05ede5c635215d95c59af32746dc8c1eb411d53ce4bd'),
    ('host', 0x31d4ec0, 0x12, 'Thread.IsCurrentThread', '442f45e7ca73b2ab79a71f17e819a08adbe03fc1a2d725af529a2eb1c5038981'),
    ('host', 0x31d4f30, 0x18, 'Thread.IsSuspended', '271bb6d24c728983de1b881ac6a37fb205371c43987a370cfd6c86972c6099c7'),
]


METHODS += [('gamecore', 46648032, 92, 'Context.GetGame', '0bee95a94a869e776ee221963d7e80aa885739a6d73aff21785121122d1a2c9f'), ('gamecore', 46685408, 92, 'Game.live.C1', '981ad8a20e2991f4cb6211bc5b349ea550efa2dbc6d59625e4d9b69897b2d519'), ('gamecore', 46685568, 56, 'Game.D1/D2', '9fd728c0f0fe56b64e0fbacaeb0c28e9f73723000c86c57ec6e708ee5f3ae1b3'), ('host', 53165376, 22, 'EngineUtility.GetGameCoreLock', '9394d1eaa3cd10aad54c138f38dbe484621b0f48d78dfd14fb0e14b1de441406'), ('host', 53165408, 22, 'EngineUtility.ReleaseGameCoreLock', '257d7c9d356de1f649b0ca2394024341abe3b88c655630f0f43df9cda22d670f'), ('gamecore', 43039648, 22, 'ICvUnknown.delete', 'ad7d214f06fbbe64670982468ad970bc59593dbb904cc8cf29764bd75e59f480'), ('gamecore', 46687088, 5, 'Game.Destroy', 'ff38a02ea4fa34d0d0258df743b207c473530c7154151d18aa114aa4d4c7afd2'), ('gamecore', 46689152, 40, 'Game.DecrementReference', '5920342050c4e1a7e3da5f1777b6c0687ba790afb7487f181e33fd95efff7755'), ('host', 52252112, 126, 'Core.EditGame', '7b21eb4636406e40ebcc7c7b75a211c0d9347403ae013d87ad91c5392ecae4e8'), ('host', 52243008, 20, 'Controller.HasLock', '6dd4a791f87419007b573e362d56f00750cf684694a909976a660b5d092ab3c4'), ('host', 52241472, 1459, 'Controller.GetLock', 'fa88d916251d8c16793b899effec00d1240e83486e2e7b5e8a153b5672079594'), ('host', 52242944, 50, 'Controller.ReleaseLock', 'bc8780bf02eda320da19787506a92b14ffbf1536ec99a10d43adb3be160b3c0d'), ('gamecore', 46711520, 92, 'Player.live.C1', 'b691f5e6f4de4881da8aad97d7ccba10c4b29a1e688a51ffa24497986d6beb56'), ('gamecore', 46711680, 56, 'Player.D1/D2', '4285b5552701212206e3a579912020c458cb7e1dbd7b0159e92fe1082a4b9162'), ('gamecore', 46712144, 5, 'Player.Destroy', '1bd077331273069ac475194e73242d22545f283bb9031d65bb23f25b1bb4d162'), ('gamecore', 46715392, 40, 'Player.DecrementReference', 'ff20560665f2d2632db5c9d9b89ebf76bfb5825a377d8144605055b5972ec4f6'), ('gamecore', 46648448, 116, 'Context.GetPlayer', '59a756c03636b0d30f3ee7b8b6b0f5e075801e81e4e7fdf1de0ef2e7a79fc56a')]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    elves = {'gamecore': Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA),
             'host': Elf(args.assets / 'Civ6', HOST_SHA)}
    for module, address, size, name, expected in METHODS:
        if hashlib.sha256(elves[module].read(address, size)).hexdigest() != expected:
            raise ValueError('original method differs: ' + name)
    game = elves['gamecore']
    host = elves['host']
    scheduling_calls = [(0x31d49cc, 0x31d4e50, 'Execute marks executing before Resume'),
                        (0x31d11d8, 0x3c402d8, 'Run enters worker base critical section'),
                        (0x31d1232, 0x31d4e70, 'Run completes after ProcessMessages'),
                        (0x31d1252, 0x3c402de, 'Run releases worker lock after completion'),
                        (0x31d4eff, 0x3c402d8, 'Join enters same worker base critical section'),
                        (0x31d4f07, 0x3c402de, 'Join releases worker base critical section')]
    for address, target, _ in scheduling_calls:
        host.call(address, target)
    for address, raw in [(0x31d1213, 'ff9038020000'),
                         (0x31d49e0, 'e96b050000'),
                         (0x31d1738, '7415'),
                         (0x31d125e, 'ff5030')]:
        if host.read(address, len(bytes.fromhex(raw))) != bytes.fromhex(raw):
            raise ValueError('original worker ordering differs')
    # RIP-relative loads in Execute and IsExecuting both name the real borrowed
    # worker at 0x6499108, exactly exported ms_pkActiveController +8.
    for address in (0x31d4965, 0x31d18a6):
        raw = host.read(address, 7)
        if raw[:3] != bytes.fromhex('488b3d') or address + 7 + int.from_bytes(raw[3:], 'little', signed=True) != 0x6499108:
            raise ValueError('original borrowed worker global differs')
    # Original factory proves allocation independently of the request stack frame.
    if game.read(0x2d28da2, 5) != bytes.fromhex('bf48000000'):
        raise ValueError('native UnitOperation allocation size differs')
    for table, slot, address in [(0x3aff870, 0x100, 0x2da2af0),
                                 (0x3aeb9c0, 0x230, 0x2c7d320),
                                 (0x3aeb9c0, 0x90, 0x2c7cae0),
                                 (0x3aecf50, 0x8, 0x2c86370)]:
        if game.relocated_pointer(table + 16 + slot) != address:
            raise ValueError('original manager/queue virtual binding differs')
    for slot, address in [(0x40, 0x32b3d40), (0x38, 0x32b3d60)]:
        if int.from_bytes(host.read(0x53a6c80 + 16 + slot, 8), 'little') != address:
            raise ValueError('original facade engine lock bridge differs')
    game.call(0x2c7cb14, 0x1f62be0)
    game.call(0x2c86b8f, 0x1e126e0)
    root = Path(__file__).resolve().parents[2]
    sources = ['native/linux/core_component.c', 'native/linux/core_component.h']
    text = (root / sources[0]).read_text()
    for forbidden in ['Context20RequestUnitOperation', 'Manager16BroadcastMessage',
                      'Manager6UpdateEf', 'Network::Manager::Update']:
        if forbidden in text:
            raise ValueError('component unexpectedly calls full-host request/transport')
    result = dict(schema='c6-linux-core-component-static-v2', checks_passed=True,
                  host_sha256=HOST_SHA, gamecore_sha256=GAME_SHA,
                  methods=[dict(module=m, address=hex(a), size=n, name=s, sha256=h)
                           for m, a, n, s, h in METHODS],
                  source_sha256={s: hashlib.sha256((root / s).read_bytes()).hexdigest() for s in sources},
                  request_storage_size=32, request_size_evidence='matching linux-skip-turn/types.dwarf and original RequestOperation stack',
                  message_storage_size=72, message_size_evidence='original CreateMessage immediate allocation at0x2d28da2',
                  stream_storage_size=40, stream_size_evidence='established original context_stream constructor contract',
                  empty_map_command_expected_size=28,
                  command_size_evidence='existing genuine original request packet, runtime must separately verify actual serialization',
                  request_ctor='C2', request_destructor='Operation D2', message_ctor='C1', message_destructor='D2',
                  component_source='fresh original MakeRequest -> UnitOperation.Init -> BaseMessage.PutInBuffer',
                  scheduling_calls=[dict(address=hex(a), target=hex(t), contract=s) for a,t,s in scheduling_calls],
                  worker_identity='borrowed original global0x6499108 = exported ms_pkActiveController0x6499100 +8; read only',
                  worker_wait='original IsExecuting false -> original Thread.JoinOnPassComplete shared-lock barrier -> original Synchronize (not credited as barrier) -> original IsSuspended true -> idle/identity recheck',
                  worker_deadline_ms=30000, timeout_disposition='fatal isolated exit240 before post-state capture or Uninitialize; original join also bounded by runner process timeout',
                  facade_handoff='both modes release all acquired game/player/unit-list/operations before any Resume; native HasLock must false; retain native parameters/identity and borrowed core refs; reacquire game/operations after completion (execute only), native HasLock must true',
                  facade_lock_evidence='fresh live Game/Player refcount1 -> ICvUnknown.delete -> Destroy -> DecrementReference -> original destructor -> EngineUtility.ReleaseGameCoreLock; no synthetic unlock',
                  native_execution=False, host_admission_proven=False, full_action_acceptance=0,
                  full_gameplay_state_supported=False, complete_rng_inventory=False,
                  policy_continuation_proven=False, gameplay_parity=False)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, native_execution=False, full_action_acceptance=0)))


if __name__ == '__main__':
    main()
