#!/usr/bin/env python3
"""Audit exact Linux HKS passive-image decoding ABI; no native execution."""
import argparse
import hashlib
import json
import struct
from pathlib import Path
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
from script_can_start_boundary_audit import METHODS as SCRIPT_METHODS

ROOT = Path(__file__).resolve().parents[2]
METHODS = [
    ('hks::HashTable::Create(lua_State*, unsigned int, unsigned int)', 62544810, 322, '1aa18970a349aeeb8a2349e249db56e85c6d0b95f0fc7baf31d1d0bf742c28fb'),
    ('hks::HashTable::getByString(HksObject) const', 62633650, 136, '68f8efdf570b019881f0d2cf5a66fb5bda69302d82a2f9e2ba26b99189584db6'),
    ('hks::HashTable::findKeyStringPosition(hks::InternString*) const', 62650864, 78, '5e0a58275a965fdc9a2f08c1d5e8e951ee527f924d7b96da40b2963b1c66010e'),
    ('hks::HashTable::findKeyIntegerPosition(int) const', 62651872, 67, 'b2b340751bfd9aaf24ccac580ea2bb5a13b6e113aa68b1118d181d1f5fc927c2'),
    ('hks::HashTable::getArrayValue(int) const', 62613920, 140, '59110e6d4e65a88baa49e09a86965a667908f8cb27547582cdd71656a282b211'),
    ('hks::HashTable::contiguousArraySize() const', 62615018, 326, 'e710f2086f6061d8060dbf3f86a7e648508bc68eaeabc955dcf28339cb6168f3'),
    ('hks::HashTable::getNext(HksObject*)', 62516464, 65, 'fe78c0f3233f342d6533d5474f00ff4151de54c7ee824675217facaec408005b'),
    ('hks::HashTable::getNextArray(HksObject*) const', 62652662, 110, '901924d917d65a5adc5f1c4fd2a220813e24427ac06d2172d2e84cc09869cdc3'),
    ('hks::HashTable::getNextHash(HksObject*) const', 62652772, 61, '4932d543835caa5800d1f158a6bc871051abc03f98b1a7a6c9756f22260e7e7d'),
    ('hks::HashTable::searchTableForNext(hks::HashTable::Node*, HksObject*) const', 62652834, 106, '5a81e2042c98ce96bc62066ddba12fef8f951fc74da463413dffd3d3aee27bcb'),
    ('hks_obj_next(lua_State*, HksObject const*, HksObject*, HksObject*)', 62614728, 71, '812fa4b3a9754054de2684c90d30b7f8966d738223b0de38c501fdceb26521e5'),
    ('hks_obj_objlen(lua_State*, HksObject const*)', 62468243, 132, 'fb8266d55c2cacc6d362906dd2a204418c60cac10952e777285260f6d04136e8'),
    ('hks_obj_getmetatable(lua_State*, HksObject const*, HksObject*)', 62468086, 147, 'c72c082012b27f3fe1b377ee263aefa2c330f6ee0d9bd57b17055e839441d182'),
    ('hks::StringTable::real_intern_inplace(lua_State*, hks::InternString*, char const*, unsigned long, unsigned int)', 62628770, 306, 'f2616997dbfaebfa9bf0c9d47ae9dbb717976be185cce6961656bc81932b3dff'),
    ('hks::cclosure::Create(lua_State*, int (*)(lua_State*), int, int, int)', 62655826, 191, '4ae4c03dc02f5d31f1c3d39ce1bacc4b91f2f1182189476eef015f5fb30ba073'),
    ('hks::createHksClosure(lua_State*, hks::Method*, hks::HashTable*)', 62650156, 138, '854d932e58c87f24bf563f53a85e8af830bf30d9a5f88f770eb89815df29dc9c'),
    ('hks::UpValue::close(lua_State*)', 62656096, 48, '8d5f9480dc3bbe4ad96d8fff62d86035707af53d3fd769107eb6c5ac9f2e5d82'),
    ('hksi_lua_getupvalue(lua_State*, int, int)', 62658245, 302, 'b25c7e6ba680e533536b3dbb17bcbe967d4ca2da6318faf4c50e9984e182425a'),
    ('Lua::Details::CCallWithErrorHandling(lua_State*, int (*)(lua_State*), void*)', 62857952, 93, '1b556700c4a56c9baf8757868080d3a3963a1833e30c55b7a8bf99d8a343bfd5'),
    ('hksi_lua_cpcall(lua_State*, int (*)(lua_State*), void*)', 62664366, 79, 'ff168b05152e3497d8e27b56ea1a1272a2ac3fb56d981390ad0648086c5bdecf'),
    ('AppLuaScriptSystem::AppLuaScriptSystem()', 57338192, 42, '6f7b14cdf581e217fb0196710c6ac1ca51b28b4b6974f8762d2cc7f281d1f55b'),
    ('hks::GarbageCollector::markTable_common(hks::HashTable*, hks::VisitData_Table*, unsigned long)', 62514896, 248, '8ad7eebe8eb0eae5c18c985491660f4617e9f2388ce1f445426544507d679b35'),
]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def global_data_symbol(elf, address):
    # Elf.relocated_pointer covers RELATIVE/64; these target GOT slots use
    # R_X86_64_GLOB_DAT (6). Resolve its own defined dynsym, never a live GOT.
    start = struct.unpack_from('<Q',elf.raw,40)[0]
    width,count = struct.unpack_from('<HH',elf.raw,58)
    sections = [struct.unpack_from('<IIQQQQIIQQ',elf.raw,start+i*width) for i in range(count)]
    for section in sections:
        if section[1] != 4:
            continue
        symbols = sections[section[6]]
        for pos in range(section[4],section[4]+section[5],section[9]):
            target,info,addend = struct.unpack_from('<QQq',elf.raw,pos)
            if target != address:
                continue
            if info & 0xffffffff != 6 or addend != 0:
                raise ValueError('expected zero-addend GLOB_DAT')
            symbol = symbols[4]+(info>>32)*symbols[9]
            section_index = struct.unpack_from('<H',elf.raw,symbol+6)[0]
            if not section_index:
                raise ValueError('undefined global symbol')
            return struct.unpack_from('<Q',elf.raw,symbol+8)[0]
    raise ValueError('GLOB_DAT relocation missing')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    parser.add_argument('--output',type=Path,required=True)
    args = parser.parse_args()
    host=Elf(args.assets/'Civ6',HOST_SHA)
    game=Elf(args.assets/'libGameCore_XP2.so',GAME_SHA)
    rows=[]
    seen=set()
    for name,address,size,fingerprint in METHODS+SCRIPT_METHODS:
        if (address,size) in seen:
            continue
        seen.add((address,size))
        if sha(host.read(address,size)) != fingerprint:
            raise ValueError('host method mismatch: '+name)
        rows.append(dict(name=name,address=hex(address),size=size,sha256=fingerprint))
    roots=[(0x3b12df0,0x3c051c0,'XP2 ScriptSystem global'),
           (0x3b13848,0x3c051b8,'XP2 EngineUtility global')]
    for got,global_address,name in roots:
        if global_data_symbol(game,got) != global_address:
            raise ValueError('XP2 global relocation mismatch: '+name)
    # Complete pure getters: double-load GOT pointer, then receiver pointer.
    game_methods=[('GetEngineUtility',0x2103fb0,11,'488b0591f8a001488b00c3'),
                  ('GetScriptSystem',0x2171660,11,'488b0589179a01488b00c3')]
    for name,address,size,code in game_methods:
        if game.read(address,size).hex()!=code:
            raise ValueError('XP2 pure getter mismatch: '+name)
    sources=['tools/linux/script_hks_registry.py','tools/linux/test_script_hks_registry.py',
             'tools/linux/script_hks_registry_audit.py']
    report=dict(schema='c6-linux-script-hks-registry-abi-v1',checks_passed=True,
        gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,
        original_execution=False,live_memory_read=False,actual_registry_captured=False,
        checked_host_methods=rows,
        checked_game_getters=[dict(name=n,address=hex(a),size=s,sha256=sha(game.read(a,s))) for n,a,s,c in game_methods],
        checked_global_relocations=[dict(got=hex(g),global_address=hex(a),name=n) for g,a,n in roots],
        abi=dict(value=dict(size=16,raw_type_offset=0,semantic_tag_mask='0xf',payload_offset=8),
                 table=dict(header_size=64,metatable_offset=16,hash_mask_offset=28,hash_nodes_offset=32,
                            array_pointer_offset=40,array_count_offset=48,hash_node_size=32,
                            node_key_offset=0,node_value_offset=16,
                            collision_links='u64 pointer at nodes-8*(node_index+1)',
                            empty_hash_mask='0xffffffff',
                            stable_next_order='nonnil array slots ascending; then nonnil values in ascending physical hash-node order'),
                 string=dict(length_flags_offset=8,length_mask='0x3fffffffffffffff',hash_offset=16,bytes_offset=20),
                 cclosure=dict(tag=10,header_size=48,function_offset=16,environment_offset=24,
                               signed_i16_upvalue_count_offset=32,inline_value_upvalues_offset=48),
                 luaclosure=dict(tag=9,header_size=56,method_offset=16,environment_offset=24,
                                upvalue_pointer_array_offset=56,method_u16_upvalue_count_offset=16,
                                upvalue_current_value_pointer_offset=32,closed_value_offset=16)),
        roots=dict(actual_script_global_GOT='base+0x3b12df0', actual_utility_global_GOT='base+0x3b13848',
                   expected_defined_xp2_script_global='base+0x3c051c0',expected_defined_xp2_utility_global='base+0x3c051b8',
                   host_main_vm_singleton_global='0x6ec9e68',singleton_main_state_offset=16,
                   main_state_global_value_offset=112,host_ET_EXEC_load_bias=0),
        null_semantics=dict(null_script='Manager script stage skipped only if actual initial system null',
                            null_main_singleton='concrete CallTestAll calls GetState with null receiver; unsafe native dereference',
                            null_main_state='no guard before original cclosure allocation/cpcall; unsupported unsafe native context',
                            absent_raw_event='preserved as raw absence; ordinary lookup may run __index and create event'),
        lock_semantics=dict(core='if already held, release before ScriptSystem call and reacquire afterward',
                            lua='CCallWithErrorHandling enters/leaves global Lua critical section 0x6ec9c70',
                            passive_helper_takes_locks=False,
                            existing_manager_idle_guard_proves_main_vm_quiescence=False),
        offline_component=['raw named string lookup using existing intern identity and native collision chain',
                           'raw integer lookup, stable table iteration and exact bounded native objlen search',
                           'C and Lua closure/upvalue shallow reconstruction, including open/closed value pointers',
                           'named event raw absence/type and built-in-versus-overridden TestAll provenance'],
        validation='16 source-derived immutable-memory tests; no original HKS table or registry native comparison',
        limits=['producer safe main-VM window and GC/lifetime proof remain required',
                'actual live roots, global/metatable tables and registrations remain uncaptured',
                'nil-valued string-key nodes retain unclassified raw pointers; GC does not mark those keys',
                'Lua Method code/prototype/debug and transitive callback environments remain unimplemented',
                'no callback execution, RNG exclusion, FullManager, gameplay action or turn acceptance'],
        source_sha256={p:sha((ROOT/p).read_bytes()) for p in sources},
        complete_state_actions_rng_turns_supported=False,unknown_bytes_masked=0,
        historical_raw_save_load_difference_bytes=89198)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,host_methods=len(rows),native_execution=False,
                          actual_registry_available=False)))


if __name__ == '__main__':
    main()
