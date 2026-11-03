#!/usr/bin/env python3
"""Source-only Linux Lua lifecycle/synchronization audit; never executes originals."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import unittest
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

ROOT=Path(__file__).resolve().parents[2]
METHODS=[
    ('host', 'AppLuaScriptSystem::Create()', 57338240, 112, 'a8cb48b7bceebb37c8daa1360069fca23072c0e84fa3e6ad27829952b91bf8ad'),
    ('host', 'AppLuaScriptSystem::AppLuaScriptSystem()', 57338192, 42, '6f7b14cdf581e217fb0196710c6ac1ca51b28b4b6974f8762d2cc7f281d1f55b'),
    ('host', 'LuaSystem::LuaScriptSystem::Setup(void* (*)(void*, void*, unsigned long, unsigned long), int, int)', 62899680, 395, '1c9314a4f764d2ca0d819f683c01ddb86721f8f5bf326bbbc35c36db62841f08'),
    ('host', 'LuaSystem::LuaScriptSystem::LuaScriptSystem(void* (*)(void*, void*, unsigned long, unsigned long), int, int)', 62900196, 129, 'ffc37e0e52b3d1c5f2a7d4a7dcf1d283188f271e4f6a95bbd40a4392e8e2492a'),
    ('host', 'LuaSystem::LuaScriptSystem::pInitializeMainState(lua_State*)', 62900326, 2082, '32650545fcc0bcf1263155270775efb6443bcdbd861f03c818853abf9d54874c'),
    ('host', 'LuaSystem::LuaScriptSystem::Destroy()', 49939904, 31, 'be935b0f163541fe249e7946cb752560a556b2d5a783262b56203eb42db2718f'),
    ('host', 'LuaSystem::LuaScriptSystem::~LuaScriptSystem()', 62902528, 89, '9a39b38a97508b7d5d7eb934ce5022d3f1d919411f43873c6e5dd5c214721ed8'),
    ('host', 'LuaSystem::LuaScriptSystem::Get()', 49794496, 11, '1588ce89ab6b8cc228d2a5820b2830ddb86e26a36e68841068300c53ca68f142'),
    ('host', 'LuaSystem::LuaScriptSystem::GetState()', 49947200, 5, '54fd6cfb47b3e150944357d067e1c7fcbe9bc8b8b20527db61840cc59da2b262'),
    ('host', 'GameCore::ScriptSystem::CreateLuaThread(char const*, bool)', 52715312, 317, '3e909d1d2ff888871e61b5098402eafc999422f62126b926ff98fb16a3cb9908'),
    ('host', 'GameCore::ScriptSystem::LoadFile(lua_State*, wchar_t const*)', 52715744, 36, '6f439a6c760ec13b523b7f8d02b07d507faa93a32207ae44499253fe4cc36863'),
    ('host', 'LuaSystem::LuaScriptSystem::LoadFile(lua_State*, wchar_t const*)', 62903708, 18, 'bfdf52d98615c93793bd50fcec3c1b03ef2d70b575a6ae2363cc82240e013717'),
    ('host', 'Lua::Details::CCallWithErrorHandling(lua_State*, int (*)(lua_State*), void*)', 62857952, 93, '1b556700c4a56c9baf8757868080d3a3963a1833e30c55b7a8bf99d8a343bfd5'),
    ('host', 'LuaSupport::GameEventLibrary::Register(lua_State*)', 57359024, 36, 'ec9ecd2cc6cda7545ba6a68f2d279c1c3d640de39e6cb169b449e3117c8ba505'),
    ('host', 'LuaSupport::GameEventLibrary::pRegister(lua_State*)', 57359072, 125, '288cabc1f47cbb6956a0ac1cac61cb6adfd39a8e314795a82e1ce672bc73c19a'),
    ('host', 'LuaSystem::LuaScriptSystem::ExposeStateToTuner(lua_State*)', 62903366, 46, '175c47d5765f76863316ae72d49e3303838b7a2048d5d0be9c3893a892d39010'),
    ('host', 'AppLuaScriptSystem::EndFrame()', 57340048, 104, '452911f495d9de9ac5f6fa9ad107c6832d81b38365728c562012432854a4e6cd'),
    ('host', 'LuaSystem::LuaScriptSystem::PumpGarbage()', 62904364, 34, 'b6677746ae5493831d062865c597f14c5caa3fd83fc1b14e0d326c9f9e8e800d'),
    ('host', 'GameCore::ScriptSystem::PumpGarbage(lua_State*)', 52716832, 54, 'e6318436b7dfb0a81bc6879018c035eab6f95971d186336b3c97793385383dc8'),
    ('host', 'Lua::Details::LockAccess()', 62858272, 12, 'dba4ce7a276912b5602417c89bda482e79534e3e71521333ace4650e59bcd83b'),
    ('host', 'Lua::Details::TryLockAccess()', 62858288, 12, 'f5c86127abc6841f2c9abc0e2225773853a5aa43d3b13e41a76ce06aa9b9253d'),
    ('host', 'Lua::Details::UnlockAccess()', 62858336, 12, '1b505f5755559492b72da3696b64f2ea2cf59c6f50d9709a0a272a2367502a36'),
    ('host', 'InitializeCriticalSection', 47057938, 105, '68e75469600c71be6f819251b8a02d19260ea7170d97b46eebe70834903521d2'),
    ('host', 'EnterCriticalSection', 47058173, 89, '68cb1f0e66514f4746c3a3fe26cc160c7112e392bedff6f417e237b48bdd40f5'),
    ('host', 'TryEnterCriticalSection', 47058298, 81, 'b08c61a09c005aa602caa65e5c9d5c52ff77e8fc4dcc21b1cff7d460ada74d45'),
    ('host', 'LeaveCriticalSection', 47058262, 36, '5bad52f202499dfbcd61791c520f1f5aa9bed3f808b33c44dc57f9b9e5c4746c'),
    ('host', 'ASL::GetCurrentThreadHANDLE()', 47053073, 119, '7a53063fd1a90ab7cf81644acfe6f0c6815edacadc1b459f6546fd5313d8bcf2'),
    ('host', 'GetCurrentThreadId', 47059039, 65, '0ceda76e51b816751d617189f9c76aeb437b4ed5b2218bc9284cf403716b5502'),
    ('host', 'hksi_lua_gc(lua_State*, int, int)', 62662436, 282, '3b121ac5d036d520523f30dc3e088ea81ebbc57815b3ea21bf3eefd37fa274af'),
    ('host', 'hks::GarbageCollector::stepIncremental(lua_State*, hks::GC_Call_Type, int)', 62511210, 103, 'fe6a68d2d486a292ee49e23530fe344b1b9fc8975758c7b8a79ecf704258da1a'),
    ('host', 'hks::GarbageCollector::stepNonIncremental(lua_State*, hks::GC_Call_Type)', 62511314, 82, '06fdffc052e4cf83c632f7e6fd0a1d645fd2cba8aef1a62a931096f2e7078a84'),
    ('host', 'GameCore::ScriptSystem::GetGameplayScriptCount()', 52715824, 33, '23b21a8360d8367340dad11a75329550326da782d400348932887c86b9a48d3d'),
    ('host', 'AppModdingFramework::GetGameplayScripts() const', 54016656, 8, '54826e5b6390995b63f86aae3ff55159c131e2459b37c7eac400c4fb02f10ac6'),
    ('host', 'AppModdingFramework::GetInstance()', 49946384, 11, '513331e1c292ffbd888fdf29449aed79d6aed91cc5da4c04fd70424891650b86'),
    ('game', 'GameCore::Exports::Context::Save(Serialization::OStream&)', 46649904, 16, 'eae3c66df15bba8f35893513dbda9de889d1d2701a3cb41869b5d017b73af27d'),
    ('game', 'GameCore::Exports::Context::Load(Serialization::IStream&)', 46649920, 50, 'cdb1fbcf4cff821b9abed6cfaf58ccfb8379db41b43442f3f16697c82d36cf1c'),
    ('game', 'GameCore::Context::Instance::Write(Serialization::OStream&) const', 43039728, 161, '96913cdfb9ded76f82d174aee6a76dda5636bac24c69532258b7d5c7e5279763'),
    ('game', 'GameCore::Context::Instance::Read(Serialization::IStream&)', 43039904, 310, '81dc994f9f6ad8bece5e2b421974aa6b3e78284d692e3c3323c417ad8ac7e371'),
    ('game', 'GameCore::Context::Instance::InitializeExistingGame()', 43043488, 72, 'a0fe9c9c41716e95ef4a00219e0f7a20d5ecb55b542317d808d03966c2eded24'),
    ('game', 'GameCore::Scripting::Manager::LoadGameplayScripts()', 47501792, 124, 'bc06013caf0e3a9a0e742ab012153b1be9cca0f7a7cb12988b651519e6eeafae'),
    ('game', 'GameCore::Scripting::Manager::LoadState(wchar_t const*, bool, bool)', 47501920, 484, 'f4df1195fee7b6cdef7fc30de07cbde99e060ecc1819db27b04d01077e3d39b9'),
    ('host', 'Lua critical-section static initializer', 46831968, 39, '7e93c2fefe216b46e08474c48e82691bc8c02c76fa4e11126562c333dd280cbc'),
    ('host', 'Lua static initialization dispatch', 46832016, 9, '1d2c3dfa5d32cbcd0894e04c37cde66540f612f94f9db7476f22dcb7fa05a530'),
    ('host', 'Platform::CriticalSection constructor', 63177388, 5, '34cd5999eea6b151143c345dcf9004f9b66b357da1e0937babac49f2a8d66388'),
    ('host', 'Platform::CriticalSection::Enter', 63177432, 5, '6a909c74ed948491ba912abc32e716f1450673ef5fea41e38956642f55f117f3'),
    ('host', 'Platform::CriticalSection::Leave', 63177438, 5, 'c602bbafb13a77f4c8391941a6a23031b35a4cb9c86c11591dd1ed029024581d'),
]

PROFILE_SHA='aa38b5edfe6f4c5ffc8fc25d079e9739ab7e3d75d3998c723dbbdaece5e08aff'
RUNTIME_SHA='5b68cfff7a3c88033694df0ef5f882a43dc780700e5ca4b194e312ca115b3740'
CLOSED_SOURCES={
    'native/linux/host_services.c':'f958b7afb65c865c061a8164e8686bb05c2bde9bda5b3a66dbc8939b5c22e3cf',
    'tools/linux/mod_activation_probe.py':'9aff7a381f5b6a1b172b737d93b5dab18bce73f8f18d0c0541a5a477136fae6a',
}
CALLS=[
    ('host',0x2f9412e,0x36ae980,'normal_GUIInit_creates_main_VM'),
    ('host',0x36ae9b2,0x36ae950,'construct_before_singleton_publication'),
    ('host',0x3bfc6a4,0x3bf2420,'Setup_original_Lua_lock'),
    ('host',0x3bfc6ae,0x3bc23a4,'Setup_creates_main_state'),
    ('host',0x3bfc6e5,0x3bc2f54,'Setup_protected_main_initializer'),
    ('host',0x3bfc6ea,0x3bf2460,'Setup_ignores_cpcall_status_then_unlocks'),
    ('host',0x3245f45,0x3bf2420,'CreateLuaThread_original_Lua_lock'),
    ('host',0x3246026,0x36b3ab0,'CreateLuaThread_registers_GameEvents'),
    ('host',0x3246037,0x3bf2460,'CreateLuaThread_original_unlock'),
    ('host',0x3bf22f5,0x3c402d8,'CCall_original_Lua_lock'),
    ('host',0x3bf2303,0x3bc2f54,'CCall_executes_original_protected_body'),
    ('host',0x3bf2330,0x3c402de,'CCall_original_unlock'),
    ('host',0x36af0c8,0x3bf2430,'EndFrame_original_trylock'),
    ('host',0x36af0df,0x3bc2f59,'EndFrame_original_GC_step'),
    ('host',0x3bfd830,0x3bf2420,'PumpGarbage_original_Lua_lock'),
    ('host',0x3bfd843,0x3bc2f59,'PumpGarbage_original_GC_step'),
    ('host',0x2ca996b,0x3c402ac,'loader_initializes_Lua_critical_section'),
    ('game',0x290cab5+7,0x1ee1db0,'Read_InitializeExistingGame_loads_gameplay_scripts'),
]


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def equal(actual,expected,name):
    if actual != expected:
        raise ValueError('boundary mismatch: '+name)


def source_window(pc, callback, continuation, gamecore_base):
    """Label exact source PCs only; this is never runtime safety admission."""
    if pc == 0x3246037 and continuation == gamecore_base+0x2d4d33c:
        return 'thread_libraries_registered_current_file_not_executed'
    if pc == 0x3bf2329 and callback == 0x3bfd5ae and continuation == gamecore_base+0x2d4d34e:
        return 'one_LoadState_file_protected_call_completed_before_tuner_exposure'
    return None


class SourceWindowTests(unittest.TestCase):
    def test_current_file_not_yet_run_is_not_later_manager(self):
        self.assertEqual(source_window(0x3246037,0,0x10000000+0x2d4d33c,0x10000000),
                         'thread_libraries_registered_current_file_not_executed')

    def test_overridden_or_other_callback_not_a_LoadFile_window(self):
        self.assertIsNone(source_window(0x3bf2329,0x1234,0x10000000+0x2d4d34e,0x10000000))

    def test_unrelated_or_post_return_pc_rejected(self):
        self.assertIsNone(source_window(0x3bf2335,0x3bfd5ae,0x10000000+0x2d4d34e,0x10000000))
        self.assertIsNone(source_window(0x3bf2329,0x3bfd5ae,0x10000000+0x2d4d33c,0x10000000))


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    parser.add_argument('--capture',type=Path,default=ROOT.parent/'c6-ghidra/evidence/linux-unit-operation-manager-metadata/native-1')
    parser.add_argument('--output',type=Path)
    parser.add_argument('--self-test',action='store_true')
    args=parser.parse_args()
    if args.self_test:
        result=unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(SourceWindowTests))
        raise SystemExit(0 if result.wasSuccessful() else 1)
    if args.output is None:
        parser.error('--output required unless --self-test')
    elves={'host':Elf(args.assets/'Civ6',HOST_SHA),'game':Elf(args.assets/'libGameCore_XP2.so',GAME_SHA)}
    methods=[]
    for module,name,address,size,fingerprint in METHODS:
        equal(sha(elves[module].read(address,size)),fingerprint,name)
        methods.append(dict(module=module,name=name,address=hex(address),size=size,sha256=fingerprint))
    for module,address,target,name in CALLS:
        elves[module].call(address,target)
    tails=[(0x3bfd5a9,0x3bf22e0),(0x32460ff,0x3bfd59c),
           (0x3c402ac,0x2ce0c12)]
    for address,target in tails:
        raw=elves['host'].read(address,5)
        equal(raw[0],0xe9,'tail opcode')
        equal(address+5+struct.unpack_from('<i',raw,1)[0],target,'tail target')
    equal(elves['host'].read(0x2ca9997,2),bytes.fromhex('ebc7'),'static initializer short tail')
    equal(struct.unpack('<Q',elves['host'].read(0x535e628,8))[0],0x2ca9990,'original init_array')
    bindings=[(0x3ac3c68+0x78,0x290bca0),(0x3ac3c68+0x80,0x290bbf0),
              (0x3ac3c68+0x38,0x290caa0)]
    for address,target in bindings:
        equal(elves['game'].relocated_pointer(address),target,'Context concrete virtual binding')
    for slot,target in [(0,0x3245f30),(0x18,0x32460e0),(0x28,0x3246130)]:
        equal(struct.unpack('<Q',elves['host'].read(0x53a63a8+slot,8))[0],target,'ScriptSystem concrete binding')
    raw_runtime=(args.capture/'runtime.json').read_bytes()
    equal(sha(raw_runtime),RUNTIME_SHA,'closed runtime identity')
    runtime=json.loads(raw_runtime)
    equal(runtime['configuration_sha256'],PROFILE_SHA,'closed profile identity')
    manifest=runtime['configuration']['lower_core_unit_operation_manager_metadata']['reconstruction_manifest']
    for path,fingerprint in CLOSED_SOURCES.items():
        equal(manifest['sources'][path],fingerprint,'closed source manifest '+path)
        equal(sha((args.capture/'source'/path).read_bytes()),fingerprint,'closed source bytes '+path)
    runner=(args.capture/'source/tools/linux/mod_activation_probe.py').read_text()
    helper=(args.capture/'source/native/linux/host_services.c').read_text()
    if "'C6_HOST_LUA':'1'" not in runner or 'getenv("C6_HOST_LUA")&&create_lua_system_and_probe()' not in helper:
        raise ValueError('closed Lua preparation route missing')
    stderr=runtime['stderr']
    ordered=['C6_LINUX_HOST lua_system_create_enter',
             'C6_LINUX_HOST lua_system_create_return main_state_nonnull',
             'C6_LINUX_HOST lua_stack_roundtrip_return 1',
             'C6_LINUX_GAME_LOAD startup_return',
             'C6_LINUX_STREAM load_return result 0 offset 76556938 size 76556938',
             'C6_LINUX_HOST lua_system_destroy_return unpublished']
    positions=[stderr.index(x) for x in ordered]
    equal(positions,sorted(positions),'closed lifecycle ordering')
    report=dict(schema='c6-linux-script-vm-lifecycle-v1',checks_passed=True,
        host_sha256=HOST_SHA,gamecore_sha256=GAME_SHA,
        profile_sha256=PROFILE_SHA,runtime_sha256=RUNTIME_SHA,
        original_execution=False,live_memory_read=False,
        closed_profile_lua_preparation=dict(C6_HOST_LUA='1',genuine_Create_called=True,
            main_state_nonnull_observed_before_Load=True,subthread_roundtrip_observed=True,
            original_GUIInit_executed_proven=False,source_sha256=CLOSED_SOURCES,
            later_main_VM_root_or_registry_captured=False),
        checked_methods=methods,
        checked_call_edges=[dict(module=m,address=hex(a),target=hex(t),name=n) for m,a,t,n in CALLS],
        checked_tail_edges=[dict(address=hex(a),target=hex(t)) for a,t in tails],
        main_VM_ownership=['original GUIInit calls AppLuaScriptSystem::Create',
            'Create constructs new state before publishing singleton',
            'Setup uses original Lua lock for newstate/GC settings/main initializer',
            'Setup does not test pInitializeMainState cpcall status before continuing',
            'Destroy closes main state under original Lua lock then clears singleton'],
        original_Lua_lock=dict(address='0x6ec9c70',size=64,
            loader_initializer='0x2ca9960 via init_array0x535e628 ->0x2ca9990',
            fields=dict(signed_lock_count=8,signed_recursion_count=12,owner_ASL_ThreadHANDLE_pointer=16,
                        pthread_mutex_pointer=24,spin_count=32),
            thread_identity_warning='owner pointer is not core u32 thread ID; GetCurrentThreadHANDLE acquires an additional registry mutex and must not be called just to inspect',
            snapshot_ownership='retain already executed original successful acquisition/owner value and original release pairing; raw counters alone are not a new acquisition'),
        conditional_source_windows=[
            dict(pc='0x3246037',original_load_continuation='XP2base+0x2d4d33c',
                 phase='thread libraries/GameEvents registration returned; current gameplay file not executed',
                 lock='original CreateLuaThread lock still held',later_Manager_registry_equivalent=False),
            dict(pc='0x3bf2329',callback='0x3bfd5ae',original_load_continuation='XP2base+0x2d4d34e',
                 phase='one LoadState file protected body/error handling returned; before Tuner exposure',
                 lock='original CCall lock still held',later_Manager_registry_equivalent=False),
            dict(pc='0x36af0d1',phase='original EndFrame successful trylock before its GC step',
                 source_condition='load-state-active; no executing controller; successful original TryLockAccess',
                 executed_by_closed_core_stream_route_proven=False,later_Manager_registry_equivalent=False)],
        GC_contract='inspected original PumpGarbage/EndFrame use Lua lock; hksi_lua_gc performs synchronous steps; no inference that GC flags alone prove quiescence or that every external mutator is excluded',
        ordinary_Read_Save_boundary=dict(native_facade_holds_Lua_lock_proven=False,
            Read_script_path='Context.Read -> InitializeExistingGame -> LoadGameplayScripts -> each actual nonempty script LoadState',
            actual_gameplay_script_count_observed=False,
            no_script_file_window_when_count_zero=True,
            manager_metadata_is_after_Read_and_save_controls=True),
        minimum_future_observation=['exact approved profile and loaded target identities; no new lifecycle calls',
            'existing original acquisition return, Lua lock receiver/owner/count/raw mutex identity and matched release',
            'actual GOT-bound ScriptSystem/utility globals; main singleton/state; raw initialization/teardown ordinals',
            'phase-specific CreateLuaThread or pLoadFile input identity and direct ordinary LoadState continuation',
            'no active/reentrant Lua/GC parent frame; captured global/GC bytes retained unclassified',
            'raw HKS image per reviewed decoder; preserve absent/ambiguous raw event keys',
            'last-file/Tuner/postload/update/GC/registration continuity needed before relating any snapshot to later Manager'],
        continuity_to_later_Manager_proven=False,passive_capture_ready=False,
        protocol_is_execution_authorization=False,
        prohibited_shortcuts=['added lock acquisition or thread pinning','Lua getter/API/indexer/callback invocation merely to inspect',
            'GetCurrentThreadHANDLE inspection call','artificial main-VM initialization','forcing an original script/GC/frame to manufacture a window',
            'saved-Begin/account-security reroute','mixing this proposal into current Military capture'],
        full_manager_supported=False,complete_state_actions_rng_turns_supported=False,
        historical_raw_save_load_difference_bytes=89198,unknown_bytes_masked=0,
        source_sha256=sha(Path(__file__).read_bytes()))
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,methods=len(methods),original_execution=False,
                          closed_profile_created_main_VM=True,passive_capture_ready=False)))


if __name__ == '__main__':
    main()
