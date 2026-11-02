#!/usr/bin/env python3
"""Read-only exact native constructors and separate conditional component audit."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

METHODS = [('gamecore', 34851536, 17, 'Request.C2', '4a8f110dd12df8a0ee7aaedc522d75dda536e424d9706e7059044a3061feaa19'), ('gamecore', 34851520, 9, 'Operation.D2', '602378b0f64b1db59b55f1d15e148f2e1fc33dec4f30cb6f94be62391d4d1251'), ('gamecore', 47352176, 87, 'UnitOperation.C1', 'fb70f24028076bb9119deaba4b4c3b71679f4c39863803e912906ad65ee16891'), ('gamecore', 41974224, 36, 'UnitOperation.D2', '196172e9e2ce538cda2ab2e96ac3a8f30062a2442f748c602ae01cd793adacaa'), ('gamecore', 47352272, 43, 'UnitOperation.Init', 'fdbe5cf384997c15cce153cf8e9b4ef0490e46f3392739c5e5c23077777d4ac2'), ('gamecore', 47320768, 44, 'BaseMessage.PutInBuffer', '45ba35a1b209ebc81f658cadd47f42dc9257266d106046c525c504571d5d3674'), ('gamecore', 47353232, 105, 'UnitOperation.CreateMessage', '58439c21466aaf8442a15a8e07d9df4db7d565b24ef32aa2f36880fb4cb62836'), ('gamecore', 47845984, 26, 'Manager.Get', '247a0c981c4169f399c86ecce00bb2cc63f7100c890337325450e592e3addcd4'), ('host', 63258740, 23, 'MemoryIStream.C1', 'ee1372ebf02a88dba5631cc87b99f29de0c7cd10224d25479ac81f6c418dd4eb'), ('host', 63258798, 23, 'MemoryIStream.D1', 'c3b703c6ab391d7e736677ab4da31b38b23b2f85fd9d4cbdd3ea550242c8e651'), ('host', 63259438, 23, 'MemoryOStream.C1.fixed', '1e99e438f390367ea3ae2b713bb866cbd1aab575a566a4d56c1e2572e6ab1fd3'), ('host', 63259802, 36, 'MemoryOStream.D0', '4cad00229cd6ced4c283ae1d3fc75e4092a7fed7b44227412ce84587357aee98'), ('host', 54320512, 35, 'Network.GetLocalPlayerID', 'f0df704c64005d2fae6fe60f23835b4d8b4dcccdbdbac202bca44c1611e75a0a')]


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
    # Original factory proves allocation independently of the request stack frame.
    if game.read(0x2d28da2, 5) != bytes.fromhex('bf48000000'):
        raise ValueError('native UnitOperation allocation size differs')
    for table, slot, address in [(0x3aff870, 0x100, 0x2da2af0),
                                 (0x3aeb9c0, 0x230, 0x2c7d320)]:
        if game.relocated_pointer(table + 16 + slot) != address:
            raise ValueError('original manager/queue virtual binding differs')
    root = Path(__file__).resolve().parents[2]
    sources = ['native/linux/core_component.c', 'native/linux/core_component.h']
    text = (root / sources[0]).read_text()
    for forbidden in ['Context20RequestUnitOperation', 'Manager16BroadcastMessage',
                      'Manager6UpdateEf', 'Network::Manager::Update']:
        if forbidden in text:
            raise ValueError('component unexpectedly calls full-host request/transport')
    result = dict(schema='c6-linux-core-component-static-v1', checks_passed=True,
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
                  native_execution=False, host_admission_proven=False, full_action_acceptance=0,
                  full_gameplay_state_supported=False, complete_rng_inventory=False,
                  policy_continuation_proven=False, gameplay_parity=False)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, native_execution=False, full_action_acceptance=0)))


if __name__ == '__main__':
    main()
