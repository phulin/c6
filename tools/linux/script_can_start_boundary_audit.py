#!/usr/bin/env python3
"""Fingerprint-gated Linux CanStartOperation script backend audit; never executes originals."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import unittest
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

ROOT = Path(__file__).resolve().parents[2]
METHODS = [
    ('GameCore::ScriptSystem::CreateArgs()', 52715952, 88, 'a77d2f00390f0c09098fb7021926cc054c05cfed9c642644573cabd98164ec68'),
    ('GameCore::ScriptSystem::DestroyArgs(GameCore::Interfaces::IScriptSystemArgs1*)', 52716144, 63, '1111df7c72cc44b37659f9cf8e512c26edde0ce2de2e8096b4da4e87539d2bb8'),
    ('GameCore::ScriptSystemArgs::ScriptSystemArgs(GameCore::ScriptSystem*)', 52712960, 69, 'e6f42250b4b79a502dc96b48666152f084852cc3949142ca16f79e8202439966'),
    ('GameCore::ScriptSystemArgs::~ScriptSystemArgs()', 52712592, 323, 'e55933dd0923833d98a47152a16e24969b1b6341c1291b535dabab0db553f88d'),
    ('GameCore::ScriptSystemArgs::Push(int)', 52713488, 108, '90c97cecd098e06fa25101c391bdbc071f56a4e377b82863681784d6db314d47'),
    ('GameCore::ScriptSystem::CallTestAll(char const*, GameCore::Interfaces::IScriptSystemArgs1*, bool&)', 52716496, 49, 'be3fc7a69a5eeecc2c73918f4fb0b154eb49365ff2097b36d7d89396c202f542'),
    ('LuaSystem::LuaScriptSystem::Get()', 49794496, 11, '1588ce89ab6b8cc228d2a5820b2830ddb86e26a36e68841068300c53ca68f142'),
    ('LuaSystem::LuaScriptSystem::GetState()', 49947200, 5, '54fd6cfb47b3e150944357d067e1c7fcbe9bc8b8b20527db61840cc59da2b262'),
    ('GameCore::EngineUtility::EngineUtility()', 53164944, 150, 'fdb95cf9751a43fecb8c78083c77ff6e56ad2f329b2ecb84e737b2344b7852a7'),
    ('GameCore::EngineUtility::HasGameCoreLock()', 53165472, 24, '8048736dcb7c1f33746aeab7e54b92df6f825c06512c37847f13c3f958498776'),
    ('GameCore::EngineUtility::ReleaseGameCoreLock()', 53165408, 22, '257d7c9d356de1f649b0ca2394024341abe3b88c655630f0f43df9cda22d670f'),
    ('GameCore::EngineUtility::GetGameCoreLock()', 53165376, 22, '9394d1eaa3cd10aad54c138f38dbe484621b0f48d78dfd14fb0e14b1de441406'),
    ('GameCoreController::HasLock() const', 52243008, 20, '6dd4a791f87419007b573e362d56f00750cf684694a909976a660b5d092ab3c4'),
    ('GameCoreController::ReleaseLock() const', 52242944, 50, 'bc8780bf02eda320da19787506a92b14ffbf1536ec99a10d43adb3be160b3c0d'),
    ('GameCoreController::GetLock() const', 52241472, 1459, 'fa88d916251d8c16793b899effec00d1240e83486e2e7b5e8a153b5672079594'),
    ('LuaSupport::GameEventLibrary::CallTestAll(lua_State*, char const*, GameCore::Interfaces::IScriptSystemArgs1*, bool&)', 57360560, 78, 'd339afefb9a19e0b00295b78990b5e61baa4eb775dddaab7b58f61240d782a38'),
    ('LuaSupport::GameEventLibrary::CreateLuaOnlyEventFactory(lua_State*)', 57360784, 216, '5aa515e6a7a3f8f9abd5b0598542462175709b3a6d2cbc0061ff945f25265add'),
    ('LuaSupport::GameEventLibrary::lEventFactoryIndexer(lua_State*)', 57361008, 274, '90a59440449d66bfb4bbf934f3e1e0f450b07715964be87946fbc4131aac7990'),
    ('LuaSupport::GameEvent::lCreate(lua_State*)', 57355104, 140, 'ccee098874e716e0639545d0deb3fbdef4a8a3a0500a6babbd5c79e724fcfee2'),
    ('LuaSupport::GameEvent::RegisterMethods(lua_State*, int, int, int)', 57355248, 268, 'a34637bb2b0d42e6dbceee996570468bd9a0d7012300c8ebc8f0f7b1289547cc'),
    ('LuaSupport::GameEvent::RegisterMethod(lua_State*, int (*)(lua_State*), char const*, int, int)', 57358688, 84, '364e8dc14f3d97db5d2c2980f7fbdd33e79d7ca81b36c334528fb2e54673d2a2'),
    ('LuaSupport::GameEvent::lAdd(lua_State*)', 57355632, 50, '1eac14761be72aeb111c8d1c84f4c0d41d746dff735de3764b8cbdbcc1b5ee9e'),
    ('LuaSupport::GameEvent::lTestAll(lua_State*)', 57357632, 539, '4e44233d6ccbea44aee875458297a1d4a4bb8efed4ad2649b7bc89964195c934'),
    ('hksi_lua_getupvalue(lua_State*, int, int)', 62658245, 302, 'b25c7e6ba680e533536b3dbb17bcbe967d4ca2da6318faf4c50e9984e182425a'),
    ('GameEventLibrary.protected_named_event_body', 57359296, 225, 'e4a1ed2a09e18c21f82b023210d4eeb35f4f9574319ec11e2468e5a48f4ea6af'),
]


CAPTURE_FACTS_SHA = '3fd5b6e64b5f81e06fc06187b9dae53691ddb536a6fc0b03f6f728d759dea585'
RUNTIME_SHA = '5b68cfff7a3c88033694df0ef5f882a43dc780700e5ca4b194e312ca115b3740'
PROFILE_SHA = 'aa38b5edfe6f4c5ffc8fc25d079e9739ab7e3d75d3998c723dbbdaece5e08aff'
BINDINGS = [
    (0x53a63a8, 0x38, 0x32461b0, 'CreateArgs'),
    (0x53a63a8, 0x48, 0x3246270, 'DestroyArgs'),
    (0x53a63a8, 0x80, 0x32463d0, 'CallTestAll'),
    (0x53a6300, 0x20, 0x3245810, 'PushInt'),
    (0x53a6c90, 0x30, 0x32b3da0, 'HasGameCoreLock'),
    (0x53a6c90, 0x38, 0x32b3d60, 'ReleaseGameCoreLock'),
    (0x53a6c90, 0x40, 0x32b3d40, 'GetGameCoreLock'),
]
STRINGS = [(0x462e7b4, b'TestAll\0'), (0x462e7cf, b'GameEventFactory\0'),
           (0x462e7e0, b'GameEvents\0'), (0x45e8d3a, b'__index\0'),
           (0x4604a63, b'__mode\0'), (0x469fdf7, b'v\0')]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require_equal(actual, expected, boundary):
    if actual != expected:
        raise ValueError('boundary mismatch: ' + boundary)


def checked_capture(raw, repeat):
    require_equal(sha(raw), CAPTURE_FACTS_SHA, 'closed capture identity')
    require_equal(repeat, raw, 'complete repeated raw facts')
    globals_rows = [json.loads(line) for line in raw.splitlines()
                    if json.loads(line).get('kind') == 'globals']
    require_equal(len(globals_rows), 1, 'globals row count')
    row = globals_rows[0]
    require_equal(row['script_vptr'], 0x53a63a8, 'captured concrete ScriptSystem vptr')
    require_equal(row['script_callbacks_inspected'], False, 'registry availability')
    if not row['script_system']:
        raise ValueError('present ScriptSystem required; no no-script admission')
    return row


class BoundaryTests(unittest.TestCase):
    def test_wrong_identity_rejected(self):
        with self.assertRaisesRegex(ValueError, 'identity'):
            checked_capture(b'{}\n', b'{}\n')

    def test_repeat_mismatch_rejected(self):
        # Exercise the shared exact comparison used before any semantic report.
        with self.assertRaisesRegex(ValueError, 'repeated'):
            require_equal(b'first', b'second', 'complete repeated raw facts')

    def test_binding_mismatch_rejected(self):
        with self.assertRaisesRegex(ValueError, 'binding'):
            require_equal(0x32b3d40, 0x32b3d60, 'utility binding')

    def test_release_reacquire_slots_are_distinct(self):
        # These are source facts, not callback simulation or native acceptance.
        require_equal(b'TestAll\0', STRINGS[0][1], 'named method')
        require_equal([x[3] for x in BINDINGS[-3:]],
                      ['HasGameCoreLock', 'ReleaseGameCoreLock', 'GetGameCoreLock'],
                      'release/call/reacquire ordering')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--capture', type=Path, default=ROOT.parent / 'c6-ghidra/evidence/linux-unit-operation-manager-metadata/native-1')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        result = unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(BoundaryTests))
        if not result.wasSuccessful():
            raise SystemExit(1)
        return
    if args.output is None:
        parser.error('--output is required unless --self-test')
    host = Elf(args.assets / 'Civ6', HOST_SHA)
    game = Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    # Host is ET_EXEC. Its symbol addresses are already absolute virtual addresses.
    require_equal(struct.unpack_from('<H', host.raw, 16)[0], 2, 'host ET_EXEC')
    methods = []
    for name, address, size, fingerprint in METHODS:
        require_equal(sha(host.read(address, size)), fingerprint, name)
        methods.append(dict(name=name, address=hex(address), size=size, sha256=fingerprint))
    require_equal(sha(game.read(0x2ce79b0,116)),
                  '47ad40fd9efa7ddcbe88b6bc4de3eff8c174262c6cc031ce76d78e63094c431e', 'XP2 wrapper')
    for vptr, slot, target, name in BINDINGS:
        require_equal(struct.unpack('<Q',host.read(vptr + slot,8))[0], target, name + ' binding')
    for address, value in STRINGS:
        require_equal(host.read(address,len(value)), value, 'event string')
    require_equal(game.read(0x2f0bac6,18), b'CanStartOperation\0', 'Manager event')
    raw_runtime = (args.capture/'runtime.json').read_bytes()
    require_equal(sha(raw_runtime), RUNTIME_SHA, 'closed runtime identity')
    runtime = json.loads(raw_runtime)
    require_equal(runtime['configuration_sha256'], PROFILE_SHA, 'closed profile identity')
    if 'C6_LINUX_HOST utility_construct_return nonnull' not in runtime['stderr']:
        raise ValueError('closed utility construction marker missing')
    paths = runtime['execution_paths']['unit_operation_manager_metadata']
    # Read the actual recorded paths, rather than assume local capture filenames.
    raw = Path(paths['output']).read_bytes()
    repeat = Path(paths['repeat_output']).read_bytes()
    observed = checked_capture(raw, repeat)
    report = dict(
        schema='c6-linux-script-can-start-boundary-v1', checks_passed=True,
        host_sha256=HOST_SHA, gamecore_sha256=GAME_SHA,
        profile_sha256=PROFILE_SHA, runtime_sha256=RUNTIME_SHA,
        facts_sha256=sha(raw), repeated_facts_equal=True,
        original_execution_performed=False,
        captured_script=dict(pointer=hex(observed['script_system']), vptr=hex(observed['script_vptr']),
                             concrete_type='GameCore::ScriptSystem', callbacks_inspected=False),
        utility=dict(source_concrete_type='GameCore::EngineUtility', address_point='0x53a6c90',
                     captured_receiver_pointer_available=False, captured_receiver_vptr_available=False,
                     host_constructor_returned=True,
                     wrapper_order=['query current utility HasGameCoreLock',
                                    'if held: refetch current utility and ReleaseGameCoreLock',
                                    'call retained ScriptSystem CallTestAll',
                                    'if initially held: refetch current utility and GetGameCoreLock']),
        checked_methods=methods,
        checked_bindings=[dict(vptr=hex(v),slot=hex(s),target=hex(t),name=n) for v,s,t,n in BINDINGS],
        source_closed=['host argument allocation/destruction and signed integer append',
                       'actual host vtable bindings and release/reacquire direction',
                       'main Lua singleton/state forwarding',
                       'built-in factory lazy creation and callback-group closure storage',
                       'built-in TestAll iteration and protected-result aggregation'],
        registration_storage=dict(global_name='GameEvents', event='CanStartOperation', method='TestAll',
                                  lazy_factory='GameEventFactory',
                                  builtin_testall_upvalue_1='shared event group table; each table value is a per-context callback array',
                                  add_upvalue_1='local per-context callback array',
                                  actual_current_tables_and_closures_captured=False),
        builtin_dispatch=dict(outer_order='actual lua_next order, not a sorted list',
                              inner_order='raw numeric slots 1 through captured array objlen',
                              short_circuit=False, aggregate_initial=True,
                              successful_result='Lua truthiness AND; nil/false false, zero/empty string true',
                              protected_callback_error='continue without changing aggregate',
                              no_nonempty_group='nil; outer called=false and caller out-value unchanged',
                              state_effects='callbacks, new Lua threads, environments, GC/allocation, error handling and live registry mutation remain material'),
        next_capture='Quiescent read-only snapshot of actual XP2 engine-utility receiver/vptr; controller lock owner/count and current thread identity; main Lua singleton/state; raw main global GameEvents and GameEventFactory entries and metatables; raw CanStartOperation entry (including absent/type); raw TestAll method closure kind/code/upvalues/environment; exact outer callback-group table storage/iteration order and every raw callback-array entry and objlen; callback closure code/prototype/upvalues/environment plus script provenance. Do not call lookup/indexer, Count, Add, TestAll, callbacks or VM APIs merely to inspect. Preserve unknown raw storage; prove HKS layouts before implementing the raw reader.',
        limits=['actual live utility pointer and registry missing',
                'built-in bindings cannot be assumed for overwritten Lua tables/methods',
                'callback scripts, gameplay effects, errors and all RNG remain required',
                'no Lua/math.random versus Game.Random exclusion is established',
                'no native AI policy execution fills missing callbacks or decisions',
                'current-operation handler CanBeInterrupted identity/effects remain a separate full Manager provider obligation'],
        rust_gameplay_implementation_added=False, full_manager_can_start_supported=False,
        complete_state_actions_rng_turns_supported=False,
        unknown_bytes_masked=0, historical_raw_save_load_difference_bytes=89198,
        source_sha256=sha(Path(__file__).read_bytes()))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,host_methods=len(methods),original_execution=False,
                          callback_registry_available=False,full_manager_supported=False)))


if __name__ == '__main__':
    main()
