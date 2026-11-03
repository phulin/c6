#!/usr/bin/env python3
"""Prepare passive current-handler metadata helper; never executes original code."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
from unit_operation_manager_metadata_audit import METHODS as METADATA
from current_handler_interruption_audit import METHODS as INTERRUPTION

ROOT = Path(__file__).resolve().parents[2]
SOURCE = 'native/linux/current_handler_interruption.c'
HEADER = 'native/linux/current_handler_interruption.h'
SCRIPT = 'tools/linux/current_handler_interruption_metadata_audit.py'
# Additional exact field-access/membership bodies. These are evidence only: the
# helper does not call the original functions, their PLT entries or any predicate.
EXTRA = [
    ('CoreComponentContainer.GetByIndex', 0x264c060, 42),
    ('QueueVector.size', 0x2702f10, 7),
    ('QueueVector.at.const', 0x2da11e0, 26),
    ('NodePointer.conversion', 0x20e35f0, 4),
    ('RecycledUnit.Begin.const', 0x2021b30, 19),
    ('RecycledUnit.End.const', 0x2021b60, 17),
    ('RecycledUnit.Find.const', 0x2baa650, 84),
    ('PlayerManager.IsValidRange', 0x2005000, 7),
    ('QueueInline.GetAlignedBuffer', 0x2c3ebe0, 9),
    ('QueueInline.AllocLinearBlock', 0x2c3ef20, 172),
    ('QueueVector.ctor', 0x2da6bb0, 66),
]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def self_tests(assets):
    with tempfile.TemporaryDirectory(prefix='c6-current-handler-passive-') as temporary:
        work = Path(temporary)
        harness = work / 'test.c'
        harness.write_text('#include "' + str(ROOT / SOURCE) + '"\n' + r'''
#include <assert.h>
#include <sys/mman.h>
static int accept(void *module,void *context,const void *game){(void)module;(void)context;(void)game;return 0;}
static int reject(void *module,void *context,const void *game){(void)module;(void)context;(void)game;return 1;}
static void put32(unsigned char *p,size_t offset,uint32_t v){memcpy(p+offset,&v,4);}
static void putptr(unsigned char *p,size_t offset,uintptr_t v){memcpy(p+offset,&v,8);}
static void hash_text(const char *input,const char *expected){Sha s=sha_init();char result[65];sha_add(&s,input,strlen(input));sha_finish(&s,result);assert(!strcmp(result,expected));}
int main(int argc,char **argv){
 if(argc==4&&!strcmp(argv[1],"--hash")){int fd=open(argv[2],O_RDONLY);assert(fd>=0);bool result=file_hash(fd,argv[3]);assert(!close(fd));return result?0:9;}
 if(argc==2&&!strcmp(argv[1],"--fatal")){C6CurrentHandlerInterruptionAdmission a={.quiescent=reject};idle(&a);return 10;}
 hash_text("","e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
 hash_text("abc","ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
 assert(digest(GAME_SHA)&&!digest(NULL)&&!digest("abc"));
 assert(c6_linux_current_handler_interruption_capture(NULL)==241);
 C6CurrentHandlerInterruptionAdmission incomplete={.module=(void *)1};
 assert(c6_linux_current_handler_interruption_capture(&incomplete)==241);
 unsigned char copied[4]={0},source[]={0,1,2,255};
 assert(read_self((uintptr_t)source,copied,sizeof copied)&&!memcmp(copied,source,sizeof copied));
 assert(!read_self(1,copied,sizeof copied)&&!read_self(UINTPTR_MAX,copied,sizeof copied));
 size_t n=9;assert(span(8,40,8,4,&n)&&n==4);assert(!span(40,8,8,4,&n));
 assert(!span(0,8,8,4,&n)&&!span(8,41,8,4,&n)&&!span(8,48,8,4,&n)&&!span(8,8,0,0,&n));
 _Alignas(16) unsigned char q[QUEUE_BYTES]={0};uintptr_t qaddr=(uintptr_t)q,block;size_t allocation;bool inlined;
 assert(queue_shape(qaddr,q,&allocation,&block,&inlined)&&allocation==0&&block==0&&!inlined);
 putptr(q,176,qaddr);put32(q,184,4);put32(q,192,1);
 assert(queue_shape(qaddr,q,&allocation,&block,&inlined)&&allocation==160&&block==qaddr&&inlined);
 putptr(q,176,qaddr+16);assert(!queue_shape(qaddr,q,&allocation,&block,&inlined));
 putptr(q,176,(qaddr+16));assert(queue_shape(qaddr+1,q,&allocation,&block,&inlined));
 putptr(q,176,qaddr);put32(q,184,1);assert(!queue_shape(qaddr,q,&allocation,&block,&inlined));
 put32(q,184,5);assert(!queue_shape(qaddr,q,&allocation,&block,&inlined));
 put32(q,184,4);putptr(q,176,qaddr+17);assert(!queue_shape(qaddr,q,&allocation,&block,&inlined));
 putptr(q,176,UINTPTR_MAX-3);assert(!queue_shape(qaddr,q,&allocation,&block,&inlined));
 putptr(q,176,qaddr);put32(q,192,5);assert(!queue_shape(qaddr,q,&allocation,&block,&inlined));
 unsigned char *unit=calloc(1,4096);assert(unit);unsigned char *queue=unit+0xc10;
 putptr(queue,176,(uintptr_t)queue);put32(queue,184,4);put32(queue,192,1);put32(queue,0,18);
 HandlerSnapshot handlers[20]={0};uintptr_t native_slots[20]={0};unsigned char native_manager[104]={0};
 putptr(native_manager,24,(uintptr_t)native_slots);putptr(native_manager,32,(uintptr_t)(native_slots+20));putptr(native_manager,40,(uintptr_t)(native_slots+20));
 for(unsigned i=0;i<20;i++){handlers[i].slot_address=(uintptr_t)(native_slots+i);}
 Image image={0};FILE *out=tmpfile();assert(out);
 assert(unit_queue(out,&image,(uintptr_t)unit,13,0xa5a50012,(uintptr_t)native_manager,handlers,20)==0);
 rewind(out);char text[4096]={0};assert(fread(text,1,sizeof text-1,out)>0);
 assert(strstr(text,"\"native_front_bits\":18")&&strstr(text,"\"observed_numeric_slot_pointer\":0"));
 native_slots[18]=1;assert(unit_queue(out,&image,(uintptr_t)unit,13,0xa5a50012,(uintptr_t)native_manager,handlers,20)==247);native_slots[18]=0;
 put32(queue,0,20);assert(unit_queue(out,&image,(uintptr_t)unit,13,0xa5a50012,(uintptr_t)native_manager,handlers,20)==247);
 put32(queue,0,0x8374d954);assert(unit_queue(out,&image,(uintptr_t)unit,13,0xa5a50012,(uintptr_t)native_manager,handlers,20)==247);
 assert(unit_queue(out,&image,UINTPTR_MAX-3,13,1,(uintptr_t)native_manager,handlers,20)==247);
 assert(!fclose(out));free(unit);
 /* Full mapping/list/block source-contract fixture, never native game state. */
 unsigned char fake_game[0x250]={0},fake_manager[7416]={0},fake_player[0x6d8]={0},fake_units[232]={0};
 unsigned char fake_unit[4096]={0},block_memory[192]={0};uintptr_t player_slots[65]={0},block_slots[3]={0};int32_t mapping[65];
 for(unsigned i=0;i<65;i++){mapping[i]=-1;}mapping[13]=13;player_slots[13]=(uintptr_t)fake_player;
 putptr(fake_game,0x248,(uintptr_t)fake_manager);
 putptr(fake_manager,0x20,(uintptr_t)player_slots);putptr(fake_manager,0x28,(uintptr_t)(player_slots+65));putptr(fake_manager,0x30,(uintptr_t)(player_slots+65));
 putptr(fake_manager,0x1a08,(uintptr_t)mapping);putptr(fake_manager,0x1a10,(uintptr_t)(mapping+65));putptr(fake_manager,0x1a18,(uintptr_t)(mapping+65));
 put32(fake_player,0xe0,13);putptr(fake_player,0x6d0,(uintptr_t)fake_units);putptr(fake_units,152,(uintptr_t)fake_player);
 unsigned char *container=fake_units+160,*entry=block_memory+48;block_slots[2]=(uintptr_t)block_memory;
 putptr(container,0,(uintptr_t)block_slots);putptr(container,8,(uintptr_t)(block_slots+3));putptr(container,16,(uintptr_t)(block_slots+3));
 put32(container,32,1);putptr(container,56,(uintptr_t)entry);putptr(container,64,(uintptr_t)entry);putptr(entry,0,(uintptr_t)fake_unit);
 putptr(fake_unit,0,0x3ae5f80);put32(fake_unit,0xb8,0xa5a50012);put32(fake_unit,0x130,13);
 C6CurrentHandlerInterruptionAdmission admission={.retained_game=fake_game,.quiescent=accept};
 out=tmpfile();assert(out);Image no_native={0};
 assert(players(out,&no_native,&admission,(uintptr_t)native_manager,handlers,20)==0);
 put32(fake_unit,0xb8,0xa5a50013);assert(players(out,&no_native,&admission,(uintptr_t)native_manager,handlers,20)==246);
 put32(fake_unit,0xb8,0xa5a50012);putptr(fake_units,152,1);assert(players(out,&no_native,&admission,(uintptr_t)native_manager,handlers,20)==246);
 putptr(fake_units,152,(uintptr_t)fake_player);putptr(entry,16,(uintptr_t)entry);assert(players(out,&no_native,&admission,(uintptr_t)native_manager,handlers,20)==246);
 assert(!fclose(out));
 unsigned char code[]={0xb0,1,0xc3},live[3];FILE *file=tmpfile();assert(file);
 assert(fwrite(code,1,sizeof code,file)==sizeof code&&!fflush(file));
 image=(Image){.fd=fileno(file),.count=1,.segments={{.lo=(uintptr_t)code,.hi=(uintptr_t)code+3,.offset=0,.flags=PF_R|PF_X}}};
 assert(!runtime_code_mapping(&image,(uintptr_t)code,3));
 assert(original_code(&image,(uintptr_t)code,3,live)&&!memcmp(live,code,3));
 code[1]=0;assert(!original_code(&image,(uintptr_t)code,3,live));
 image.segments[0].flags=PF_R;assert(!original_code(&image,(uintptr_t)code,3,live));
 void *mapped=mmap(NULL,4096,PROT_READ|PROT_EXEC,MAP_PRIVATE,fileno(file),0);assert(mapped!=MAP_FAILED);
 image=(Image){.fd=fileno(file),.count=1,.segments={{.lo=(uintptr_t)mapped,.hi=(uintptr_t)mapped+3,.offset=0,.flags=PF_R|PF_X}}};
 assert(runtime_code_mapping(&image,(uintptr_t)mapped,3));
 assert(!mprotect(mapped,4096,PROT_READ));assert(!runtime_code_mapping(&image,(uintptr_t)mapped,3));
 assert(runtime_file_mapping(&image,(uintptr_t)mapped,3,false));
 assert(!munmap(mapped,4096));assert(!fclose(file));return 0;
}
''')
        executable = work / 'test'
        common = ['cc', '-std=c11', '-Wall', '-Wextra', '-Werror', '-Wconversion', '-Wshadow']
        subprocess.run(common + ['-O2', '-fsanitize=undefined', str(harness), '-ldl', '-o', str(executable)], check=True)
        subprocess.run([str(executable)], check=True)
        fatal = subprocess.run([str(executable), '--fatal'], capture_output=True)
        require(fatal.returncode == 240 and b'no_poststate_or_cleanup' in fatal.stderr,
                'unknown quiescence did not fail closed')
        for size in [1, 55, 56, 63, 64, 65, 32767, 32768, 32769, 1000000]:
            data = bytes((i * 29 + 255) % 256 for i in range(size))
            path = work / 'hash-input'
            path.write_bytes(data)
            subprocess.run([str(executable), '--hash', str(path), sha(data)], check=True)
            result = subprocess.run([str(executable), '--hash', str(path), '0' * 64])
            require(result.returncode == 9, 'wrong original file identity accepted')
        for name, digest in [('libGameCore_XP2.so', GAME_SHA), ('Civ6', HOST_SHA)]:
            subprocess.run([str(executable), '--hash', str(assets / name), digest], check=True)
        subprocess.run(common + ['-fanalyzer', '-c', str(ROOT / SOURCE), '-o', str(work / 'helper.o')], check=True)
    return dict(strict_compiler=True, gcc_analyzer=True, ubsan_synthetic_harness=True,
                hash_standard_vectors=2, streamed_hash_boundary_sizes=10,
                wrong_hash_refusals=10, original_asset_files_hashed_without_execution=2,
                unreadable_overflow_refused=True, queue_shape_and_native_identity_boundaries=True,
                complete65_slot_synthetic_roster_and_block_membership=True,
                wrong_slot_parent_and_active_list_tail_refused=True,
                live_code_mismatch_and_nonexecuting_segment_refused=True,
                actual_runtime_code_map_permissions_and_file_owner_required=True,
                actual_vtable_slot_map_readability_and_file_owner_required=True,
                unknown_idle_fatal_exit=240, original_execution=False,
                synthetic_fixtures_are_not_native_binding_acceptance=True)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    game, host = Elf(a.assets / 'libGameCore_XP2.so', GAME_SHA), Elf(a.assets / 'Civ6', HOST_SHA)
    methods = []
    needed = {0x1fcc320, 0x2da1260, 0x20e0670, 0x2003940, 0x2004fc0, 0x2004680,
              0x1fcc140, 0x1fccc70, 0x1fe4ee0, 0x2021450, 0x2021470, 0x2021480,
              0x2021490, 0x20214a0, 0x2ba5580}
    for image, address, size, name, fingerprint in METADATA:
        if image == 'gamecore' and address in needed:
            require(sha(game.read(address, size)) == fingerprint, 'original complete field method mismatch: ' + name)
            methods.append(dict(name=name, rva=hex(address), bytes=size, sha256=fingerprint))
            needed.remove(address)
    require(not needed, 'missing reusable field evidence')
    for name, address, size, fingerprint in INTERRUPTION:
        require(sha(game.read(address, size)) == fingerprint, 'original interruption method mismatch: ' + name)
        methods.append(dict(name=name, rva=hex(address), bytes=size, sha256=fingerprint))
    for name, address, size in EXTRA:
        methods.append(dict(name=name, rva=hex(address), bytes=size, sha256=sha(game.read(address, size))))
    # Full module identity pins these additional bodies. Record their complete
    # method fingerprints, then check the layout-bearing instructions separately.
    for address, expected in [(0x2702f10, '8b87c0000000c3'),
                              (0x20e0710, '488d87100c0000c3'),
                              (0x20e35f0, '488b07c3'),
                              (0x2005000, '83fe410f92c0c3'),
                              (0x2c3ebe0, '488d470f4883e0f0c3'),
                              (0x2c3ef45, 'c744240804000000'),
                              (0x2da11f0, '488d0c9b488d04c8')]:
        require(game.read(address, len(expected) // 2).hex() == expected, 'field instruction mismatch')
    for offset, target in [(0x88, 0x2da1650), (0x98, 0x2da16f0), (0xa0, 0x2da1720), (0xd8, 0x2da2a60)]:
        require(game.relocated_pointer(0x3aff880 + offset) == target, 'static Manager slot prediction mismatch')
    symbols = subprocess.run(['nm', '-S', '-C', str(a.assets / 'libGameCore_XP2.so')], check=True, capture_output=True, text=True).stdout
    require('0000000003c051a8 0000000000000008 B GameCore::Context::Globals::ms_pkCurrentGame' in symbols,
            'current Game global identity differs')
    require('0000000003c00684 0000000000000004 D GameCore::Unit::Operations::Manager::ms_uiComponentIndex' in symbols,
            'Manager component index identity differs')
    types = ['GameCore::Unit::Operations::OperationQueue',
             'GameCore::Unit::Operations::Node',
             'Types::Vector<GameCore::Unit::Operations::Node, Types::LinearAlloc::DefaultInline<GameCore::Unit::Operations::Node, 4, 51, 0> >',
             'Types::Vector<GameCore::Unit::Operations::Node, Types::LinearAlloc::DefaultInline<GameCore::Unit::Operations::Node, 4, 51, 0> >::LINEAR_BLOCK',
             'RecycledIDContainer<GameCore::Unit::Instance, 8, 16>',
             'RecycledIDContainer<GameCore::Unit::Instance, 8, 16>::BlockEntry',
             'RecycledIDContainer<GameCore::Unit::Instance, 8, 16>::BlockHeader']
    cmd = ['gdb', '-nx', '-q', '-batch', str(a.assets / 'libGameCore_XP2.so')]
    for name in types:
        cmd += ['-ex', "ptype /o '" + name + "'"]
    dwarf = subprocess.run(cmd, check=True, capture_output=True, text=True).stdout
    for marker in ['total size (bytes):  200', 'total size (bytes):   40',
                   '/*    176      |      16 */    LINEAR_BLOCK m_kBlock;',
                   '/*    192      |       4 */    unsigned int m_nCurrSize;',
                   '/*      8      |       4 */    unsigned int nElements;',
                   '/*     56      |       8 */', 'm_pHeadAllocatedEntry;',
                   '/*     64      |       8 */', 'm_pTailAllocatedEntry;',
                   '/*     16      |       8 */', 'm_pNext;',
                   'm_aEntries[8];', 'total size (bytes):  192']:
        require(marker in dwarf, 'target DWARF layout mismatch: ' + marker)
    tests = self_tests(a.assets)
    sources = [SOURCE, HEADER, SCRIPT, 'tools/linux/unit_operation_manager_metadata_audit.py',
               'tools/linux/current_handler_interruption_audit.py']
    report = dict(schema='c6-linux-current-handler-interruption-metadata-preparation-v1',
        checks_passed=True, gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA,
        original_execution_performed=False, runner_integrated=False, native_outputs_available=False,
        source_prediction_report_preserved_at='1dae236',
        methods=methods, target_dwarf_sha256=sha(dwarf.encode()),
        layout=dict(game_current_global='0x3c051a8', manager_component_index='0x3c00684',
            game_components_offset=8, component_stride=288, manager_bytes=104,
            handler_vector_offset=24, handler_base_bytes=16, handler_id_offset=8,
            handler_hash_offset=12, interruption_slot=32, player_manager_offset=584,
            player_id_to_player_offset=32, player_id_to_index_offset=6664,
            player_id_offset=224, player_units_offset=1744,
            units_parent_offset=152, recycled_container_offset=160,
            recycled_bytes=72, block_entries=8, block_entry_bytes=24,
            unit_vptr_rva='0x3ae5f80', unit_id_offset=184, unit_owner_offset=304,
            unit_queue_offset=3088, queue_bytes=200, queue_block_offset=176,
            queue_capacity_offset=184, queue_count_offset=192, node_bytes=40,
            initialized_empty_block=dict(pointer=0, capacity=0),
            inline_block=dict(pointer="align_up(queue_address,16)", capacity=4),
            out_of_line_block=dict(pointer_alignment=16, minimum_capacity=5)),
        helper_original_calls=[], caller_callback='required strict retained-context idle admission; separately source/profile gated',
        runtime_requirements=['whole actual module file and /proc/self/exe hashes match pinned target',
            'actual live Manager slots and supported complete code bytes match hashed original file',
            'actual live handler +0x20 target belongs to mapped executable original module and matches complete leaf bytes',
            'all65 mapped Player slots and complete original active Unit container membership',
            'native front bits select a bounded observed vector slot by source-derived numeric indexing; no native GetHandler return observed',
            'unmasked native queue header and entire native block capacity bytes retained; no parameter dereference',
            'same-context callback and retained original owner lock/lifetime; no original getters or predicates called by helper'],
        tests=tests,
        open_integration=['separate metadata profile and caller guard/source identities',
            'retained Context/Game/root/session lifetimes and main-thread native lock',
            'two full passes, six unmasked Saves, known64 RNG endpoint bytes and terminal cleanup',
            'same-context serializer ownership/hash evidence and independently observed reverse-map answers',
            'live transitive PLT/GOT destinations and original getter equivalence remain unvalidated; matching Manager bodies alone is insufficient',
            'independent pre-answer checker/profile admission before sole future native capture'],
        full_manager_supported=False, complete_state_action_rng_turns_supported=False,
        historical_raw_save_load_failure_bytes=89198,
        source_sha256={x: sha((ROOT / x).read_bytes()) for x in sources})
    a.output.parent.mkdir(parents=True, exist_ok=True)
    a.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, methods=len(methods), original_execution=False,
                         runner_integrated=False, strict_compiler=True, analyzer=True)))


if __name__ == '__main__':
    main()
