#!/usr/bin/env python3
"""Independent immutable-source/synthetic-image review; never runs original code."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import sys
import types
import unittest

ROOT = Path(__file__).resolve().parents[2]
REVIEWED = '37c9c0f'
HOST_SHA = '106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
GAME_SHA = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'


def committed(path):
    return subprocess.check_output(['git', 'show', f'{REVIEWED}:{path}'], cwd=ROOT)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def decoder():
    # This decoder dependency supplies only these two constants. Do not import
    # mutable working-tree implementation or fixture helpers into the review.
    constants = types.ModuleType('save_lifecycle_audit')
    constants.HOST_SHA, constants.GAME_SHA = HOST_SHA, GAME_SHA
    sys.modules['save_lifecycle_audit'] = constants
    module = types.ModuleType('reviewed_hks')
    sys.modules[module.__name__] = module
    raw = committed('tools/linux/script_hks_registry.py')
    exec(compile(raw, f'{REVIEWED}:script_hks_registry.py', 'exec'), module.__dict__)
    return module


D = decoder()


def obj(tag, bits=0):
    return struct.pack('<I4xQ', tag, bits)


def num(n):
    return struct.pack('<I4xd', 3, n)


class Memory:
    """Hand-packed sparse objects; no reuse of producer layout builder."""
    def __init__(self):
        self.parts = []

    def put(self, pointer, raw):
        self.parts.append((pointer, bytes(raw)))

    def string(self, pointer, raw):
        self.put(pointer, bytes(8) + struct.pack('<QI', len(raw), 0) + raw + b'\0')

    def table(self, pointer, array=(), nodes=(), links=None):
        count = len(nodes)
        header = bytearray(64)
        struct.pack_into('<I', header, 28, count-1 if count else 0xffffffff)
        struct.pack_into('<QQI', header, 32, pointer+0x1000 if count else 0,
                         pointer+0x2000 if array else 0, len(array))
        self.put(pointer, header)
        if count:
            links = links or [0]*count
            self.put(pointer+0x1000-8*count,
                     b''.join(struct.pack('<Q', v) for v in reversed(links)) + b''.join(k+v for k,v in nodes))
        if array:
            self.put(pointer+0x2000, b''.join(array))
        return pointer

    def image(self, **kwargs):
        return D.Image(self.parts, **kwargs)


class ReviewTests(unittest.TestCase):
    def test_all_small_array_hole_boundaries(self):
        checked = 0
        for size in range(9):
            for mask in range(1 << size):
                array = [num(i+1) if mask >> i & 1 else obj(0) for i in range(size)]
                mem = Memory(); mem.table(0x10000, array)
                length = D.Table(mem.image(), 0x10000).length()
                self.assertTrue(length == 0 or mask >> (length-1) & 1)
                self.assertTrue(length == size or not (mask >> length & 1))
                checked += 1
        self.assertEqual(checked, 511)

    def test_objlen_signed_overflow_counts_nil_hash_key_presence(self):
        # Main chain 0 has128,256,...,2**30; smaller powers use their
        # natural buckets. Keys1,2,3 have their own buckets;
        # key3 exists with nil value. Exponential overflow falls back to key
        # existence rather than value for hash keys, yielding4 rather than2.
        keys = [1 << i for i in range(2,31)]
        nodes = [(obj(0),obj(0)) for _ in range(128)]
        links = [0]*128
        positions = [key if key < 128 else None for key in keys]
        spare = iter(i for i in range(4,128) if i not in keys)
        positions = [next(spare) if p is None else p for p in positions]
        # The first key whose natural position is zero owns that bucket.
        positions[keys.index(128)] = 0
        for i,(key,pos) in enumerate(zip(keys,positions)):
            nodes[pos] = (num(key),num(key))
            if key >= 128 and i+1 < len(positions): links[pos] = 0x11000+32*positions[i+1]
        for key in [1,2,3]: nodes[key] = (num(key),obj(0) if key==3 else num(key))
        mem=Memory(); mem.table(0x10000,nodes=nodes,links=links)
        self.assertEqual(D.Table(mem.image(),0x10000).length(),4)

    def test_numeric_nan_does_not_equal_integer_or_stop_collision(self):
        mem=Memory(); mem.table(0x10000,nodes=[(num(float('nan')),num(1)),(num(0),num(8))],
                               links=[0x11020,0])
        self.assertEqual(D.Table(mem.image(),0x10000).integer(0).number,8)

    def test_reverse_collision_and_physical_iteration_differ(self):
        mem=Memory(); mem.table(0x10000,nodes=[(num(2),num(20)),(num(0),num(10))],
                               links=[0x11020,0])
        table=D.Table(mem.image(),0x10000)
        self.assertEqual(table.integer(0).number,10)
        self.assertEqual([v.number for _,v in table.entries()],[20,10])

    def test_invalid_link_and_cycle_fail_when_traversed(self):
        for link in [0x11001,0x11040,0x11000]:
            mem=Memory(); mem.table(0x10000,nodes=[(num(2),num(20)),(obj(0),obj(0))],links=[link,0])
            with self.assertRaises(D.SnapshotError): D.Table(mem.image(),0x10000).integer(0)

    def test_unreachable_live_string_is_rejected(self):
        mem=Memory(); s=0x40000; mem.string(s,b'key')
        natural=(s^(s>>5)^(s>>8))&1
        nodes=[(obj(0),obj(0))]*2
        nodes[1-natural]=(obj(4,s),num(9))
        mem.table(0x10000,nodes=nodes)
        with self.assertRaisesRegex(D.SnapshotError,'unreachable'):
            D.Table(mem.image(),0x10000).lookup_string(b'key')

    def test_duplicate_live_interns_fail_even_with_equal_text(self):
        mem=Memory(); mem.string(0x40000,b'key');mem.string(0x40100,b'key')
        mem.table(0x10000,nodes=[(obj(4,0x40000),num(1)),(obj(4,0x40100),num(2))])
        with self.assertRaisesRegex(D.SnapshotError,'duplicate intern'):
            D.Table(mem.image(),0x10000).lookup_string(b'key')

    def test_stale_tombstone_string_is_never_read(self):
        mem=Memory(); mem.table(0x10000,nodes=[(obj(4,0xfffffffffffffff0),obj(0))])
        table=D.Table(mem.image(),0x10000)
        self.assertEqual(table.lookup_string(b'key')['status'],'absent_or_nil_raw_key')
        self.assertEqual(list(table.entries()),[])
        self.assertIn('f0ffffffffffffff',table.record()['hash_storage_raw'])

    def test_shared_upvalue_alias_retains_identity(self):
        mem=Memory(); header=bytearray(0x48)
        struct.pack_into('<Q',header,16,0x30000)
        struct.pack_into('<QQ',header,56,0x40000,0x40000)
        mem.put(0x20000,header); mem.put(0x30010,struct.pack('<H',2))
        upvalue=bytearray(0x28);upvalue[16:32]=num(7);struct.pack_into('<Q',upvalue,32,0x40010)
        mem.put(0x40000,upvalue)
        decoded=mem.image().closure(D.Value(obj(9,0x20000)))
        self.assertEqual(decoded['upvalues'][0],decoded['upvalues'][1])
        self.assertTrue(decoded['upvalues'][0]['closed'])

    def test_negative_cclosure_count_and_missing_open_value_fail(self):
        mem=Memory(); header=bytearray(48);struct.pack_into('<h',header,32,-1);mem.put(0x20000,header)
        with self.assertRaises(D.SnapshotError): mem.image().closure(D.Value(obj(10,0x20000)))
        mem=Memory();header=bytearray(64);struct.pack_into('<Q',header,16,0x30000)
        struct.pack_into('<Q',header,56,0x40000);mem.put(0x20000,header)
        mem.put(0x30010,struct.pack('<H',1));up=bytearray(40);struct.pack_into('<Q',up,32,0xdeadbeef)
        mem.put(0x40000,up)
        with self.assertRaisesRegex(D.SnapshotError,'uncaptured'):
            mem.image().closure(D.Value(obj(9,0x20000)))

    def test_runtime_interposed_got_binding_and_null_singleton(self):
        base=0x100000000;mem=Memory()
        mem.put(base+0x3b12df0,struct.pack('<Q',0x30000))
        mem.put(base+0x3b13848,struct.pack('<Q',0x40000))
        mem.put(0x30000,struct.pack('<Q',0x50000));mem.put(0x40000,bytes(8))
        mem.put(0x50000,struct.pack('<Q',0x53a63a8));mem.put(0x6ec9e68,bytes(8))
        roots=D.inspect_roots(mem.image(),base)
        self.assertFalse(roots['script_global_is_XP2_definition'])
        self.assertEqual(roots['script_system'],'0x50000')
        self.assertTrue(roots['script_concrete_binding'])
        self.assertIn('null_singleton',roots['main_vm_status'])

    def test_null_got_and_nonnull_singleton_null_state_remain_unsafe(self):
        base=0x100000000;mem=Memory()
        mem.put(base+0x3b12df0,bytes(8));mem.put(base+0x3b13848,bytes(8))
        mem.put(0x6ec9e68,struct.pack('<Q',0x30000));mem.put(0x30000,bytes(24))
        roots=D.inspect_roots(mem.image(),base)
        self.assertIn('unsafe',roots['script_status'])
        self.assertIn('unsafe',roots['main_vm_status'])
        self.assertEqual(D.inspect_named_event(mem.image(),0)['event_status'],
                         'unavailable_null_main_lua_state')

    def test_builtin_missing_context_and_replacement_identity(self):
        for tag,function in [(10,0x36b3540),(10,0x999999),(9,0x80000)]:
            mem=Memory()
            for pointer,text in [(0x60000,b'GameEvents'),(0x61000,b'CanStartOperation'),(0x62000,b'TestAll')]:
                mem.string(pointer,text)
            mem.table(0x10000,nodes=[(obj(4,0x60000),obj(5,0x20000))])
            mem.table(0x20000,nodes=[(obj(4,0x61000),obj(5,0x30000))])
            mem.table(0x30000,nodes=[(obj(4,0x62000),obj(tag,0x70000))])
            mem.put(0x50070,obj(5,0x10000))
            closure=bytearray(0x38 if tag==9 else 0x30)
            struct.pack_into('<Q',closure,16,function);mem.put(0x70000,closure)
            if tag==9: mem.put(0x80010,bytes(2))
            if function==0x36b3540:
                with self.assertRaisesRegex(D.SnapshotError,'shared group upvalue missing'):
                    D.inspect_named_event(mem.image(),0x50000)
            else:
                result=D.inspect_named_event(mem.image(),0x50000)
                self.assertNotIn('callback_arrays',result)
                self.assertFalse(result['callbacks_executed'])
                self.assertEqual(result['test_all_closure']['kind'],'hksclosure' if tag==9 else 'cclosure')

    def test_overlapping_segments_and_truncated_table_are_rejected(self):
        with self.assertRaises(D.SnapshotError): D.Image([(0x1000,bytes(16)),(0x1008,bytes(16))])
        mem=Memory();mem.put(0x10000,bytes(63))
        with self.assertRaises(D.SnapshotError): D.Table(mem.image(),0x10000)


def elf_bytes(raw, address, size):
    assert raw[:6] == b'\x7fELF\x02\x01'
    phoff=struct.unpack_from('<Q',raw,32)[0]
    phsize,phnum=struct.unpack_from('<HH',raw,54)
    for i in range(phnum):
        kind,_,offset,va,_,filesz,_,_=struct.unpack_from('<IIQQQQQQ',raw,phoff+i*phsize)
        if kind==1 and va<=address and address+size<=va+filesz:
            return raw[offset+address-va:offset+address-va+size]
    raise ValueError('reviewed span not file-backed')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    audit=json.loads(committed('evidence/linux/script-hks-registry-abi.json'))
    originals={'host':(args.assets/'Civ6').read_bytes(),'game':(args.assets/'libGameCore_XP2.so').read_bytes()}
    assert digest(originals['host'])==HOST_SHA and digest(originals['game'])==GAME_SHA
    assert struct.unpack_from('<H',originals['host'],16)[0]==2
    spans=[]
    for category,key in [('host','checked_host_methods'),('game','checked_game_getters')]:
        for row in audit[key]:
            assert digest(elf_bytes(originals[category],int(row['address'],0),row['size']))==row['sha256']
            spans.append(dict(module=category,**row))
    result=unittest.TextTestRunner(verbosity=1).run(unittest.defaultTestLoader.loadTestsFromTestCase(ReviewTests))
    assert result.wasSuccessful()
    report=dict(schema='c6-linux-script-hks-registry-review-v1',
        reviewed_commit=subprocess.check_output(['git','rev-parse',REVIEWED],cwd=ROOT,text=True).strip(),
        reviewed_source_sha256={p:digest(committed(p)) for p in ['tools/linux/script_hks_registry.py',
            'tools/linux/test_script_hks_registry.py','tools/linux/script_hks_registry_audit.py',
            'docs/linux-script-hks-registry.md','evidence/linux/script-hks-registry-abi.json']},
        review_script_sha256=digest(Path(__file__).read_bytes()),host_sha256=HOST_SHA,gamecore_sha256=GAME_SHA,
        independent_elf_span_checks=spans,independent_tests_run=result.testsRun,array_hole_patterns=511,
        tests_passed=True,original_execution=False,live_memory_read=False,actual_registry_available=False,
        native_parity_accepted=False,capture_boundary_accepted=False,gameplay_supported=False,
        raw_save_load_difference_bytes=89198,unknown_bytes_masked=0,
        limits=['Synthetic immutable images establish only bounded decoder behavior.',
                'No main-VM/GC safe capture window or original registry image is established.',
                'No callback mutations/execution, Method graph, full Manager, state/actions/RNG/turn acceptance.'])
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(tests=result.testsRun,source_commit=REVIEWED,original_execution=False)))


if __name__=='__main__': main()
