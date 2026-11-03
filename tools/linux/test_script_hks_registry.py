"""Source-derived immutable-memory tests; no original VM or callback execution."""
import struct
import unittest
from script_hks_registry import Image, NIL, SnapshotError, Table, Value, inspect_named_event, inspect_roots


def value(tag, payload=0, padding=b'\0'*4):
    return Value(struct.pack('<I',tag)+padding+struct.pack('<Q',payload))


def number(n):
    return Value(struct.pack('<I4xd',3,float(n)))


class Fixture:
    def __init__(self):
        self.raw = bytearray(0x20000)
        self.next = 0x1000

    def alloc(self, size):
        pointer = self.next
        self.next = (pointer+size+15)&~15
        return pointer

    def put(self, pointer, raw):
        self.raw[pointer:pointer+len(raw)] = raw

    def string(self, data):
        pointer = self.alloc(21+len(data))
        self.put(pointer+8,struct.pack('<Q',len(data)|(1<<63)))
        self.put(pointer+20,data+b'\0')
        return value(4,pointer)

    def table(self, array=(), entries=(), mask=None):
        pointer = self.alloc(0x40)
        count = 0 if not entries else 1 << ((len(entries)*2-1).bit_length())
        if mask is not None:
            count = mask+1
        nodes_allocation = self.alloc(count*40) if count else 0
        nodes = nodes_allocation+count*8 if count else 0
        data = self.alloc(len(array)*16) if array else 0
        header = bytearray(64)
        struct.pack_into('<IQQI',header,0x1c,count-1 if count else 0xffffffff,nodes,data,len(array))
        self.put(pointer,header)
        for i,item in enumerate(array):
            self.put(data+i*16,item.raw)
        used = set()
        tails = {}
        for key,item in entries:
            if key.tag == 4:
                natural = (key.bits ^ (key.bits>>5) ^ (key.bits>>8)) & (count-1)
            elif key.tag == 3:
                natural = int(key.number)&(count-1)
            else:
                raise ValueError('fixture only constructs numeric/string hash keys')
            index = natural if natural not in used else next(i for i in range(count) if i not in used)
            # The fixtures avoid displaced other main positions; this is a
            # raw layout builder, not an implementation of native insertion.
            if natural in tails:
                self.put(nodes-8*(tails[natural]+1),struct.pack('<Q',nodes+index*32))
            elif index != natural:
                raise ValueError('fixture main position occupied by another key')
            tails[natural] = index
            used.add(index)
            self.put(nodes+index*32,key.raw+item.raw)
        return pointer

    def image(self):
        return Image([(0x100,self.raw[0x100:])])


class RegistryTests(unittest.TestCase):
    def test_preserves_unknown_tag_bits_padding_and_payload(self):
        raw = value(0x123456a5,0xfeed,b'abcd')
        self.assertEqual(raw.tag,5)
        self.assertEqual(Value(bytes.fromhex(raw.record()['raw'])).raw,raw.raw)

    def test_string_length_flags_and_embedded_zero(self):
        f=Fixture(); s=f.string(b'a\0b')
        self.assertEqual(f.image().string(s.bits),b'a\0b')

    def test_iteration_uses_array_then_physical_hash_order(self):
        f=Fixture(); p=f.table([number(4),NIL,number(6)],[(number(7),number(70)),(number(4),number(40))])
        t=Table(f.image(),p)
        self.assertEqual([(k.number,v.number) for k,v in t.entries()],[(1,4),(3,6),(4,40),(7,70)])

    def test_native_hole_length_is_not_first_missing_index(self):
        f=Fixture(); p=f.table([NIL,number(2),number(3),NIL])
        self.assertEqual(Table(f.image(),p).length(),3)

    def test_length_search_crosses_array_hash_boundary(self):
        f=Fixture(); p=f.table([number(1),number(2)],[(number(3),number(3)),(number(4),number(4))])
        t=Table(f.image(),p)
        self.assertEqual(t.length(),4)
        self.assertEqual(t.integer(4).number,4)
        self.assertEqual(t.integer(5).tag,0)

    def test_reverse_collision_links_are_followed(self):
        f=Fixture(); s=f.string(b'first'); p=f.table(entries=[(s,number(1))],mask=1)
        t=Table(f.image(),p); natural=(s.bits^(s.bits>>5)^(s.bits>>8))&1
        second_pointer=next(i for i in range(0x9000,0xa000,0x10) if (i^(i>>5)^(i>>8))&1==natural)
        f.put(second_pointer+8,struct.pack('<Q',6));f.put(second_pointer+20,b'second\0')
        other=1-natural
        f.put(t.nodes+other*32,value(4,second_pointer).raw+number(2).raw)
        f.put(t.nodes-8*(natural+1),struct.pack('<Q',t.nodes+other*32))
        self.assertEqual(Table(f.image(),p).lookup_string(b'second')['value'].number,2)

    def test_nil_valued_string_key_pointer_is_not_dereferenced(self):
        f=Fixture(); s=f.string(b'nilkey'); p=f.table(entries=[(s,NIL)])
        t=Table(f.image(),p)
        # Tombstone pointers can be stale: even uncaptured string bytes must
        # not be read, and no name is assigned to this nil-valued key.
        index=next(i for i in range(t.count) if t.node(i)[0].tag==4)
        f.put(t.nodes+index*32+8,struct.pack('<Q',0xdeadbeef))
        t=Table(f.image(),p)
        self.assertEqual(t.lookup_string(b'nilkey')['status'],'absent_or_nil_raw_key')
        self.assertEqual(t.lookup_string(b'missing')['unclassified_nil_string_nodes'],[index])

    def test_missing_memory_cannot_be_zero_filled(self):
        with self.assertRaisesRegex(SnapshotError,'uncaptured'):
            Image([(0x1000,bytes(16))]).value(0x2000)

    def test_invalid_hash_extent_rejected(self):
        f=Fixture();p=f.table();f.put(p+0x1c,struct.pack('<I',2))
        with self.assertRaisesRegex(SnapshotError,'power of two'):
            Table(f.image(),p)

    def test_cyclic_hash_chain_rejected(self):
        f=Fixture();s=f.string(b'key');p=f.table(entries=[(s,number(1))]);t=Table(f.image(),p)
        index=(s.bits^(s.bits>>5)^(s.bits>>8))&t.mask
        f.put(t.nodes-8*(index+1),struct.pack('<Q',t.nodes+index*32))
        with self.assertRaisesRegex(SnapshotError,'cyclic'):
            list(Table(f.image(),p).chain(index))

    def test_open_and_closed_lua_upvalues(self):
        f=Fixture();method=f.alloc(0x50);f.put(method+0x10,struct.pack('<H',2))
        closure=f.alloc(0x48);f.put(closure+0x10,struct.pack('<Q',method))
        first=f.alloc(0x28);second=f.alloc(0x28);external=f.alloc(16)
        f.put(closure+0x38,struct.pack('<QQ',first,second))
        f.put(first+0x10,number(8).raw);f.put(first+0x20,struct.pack('<Q',first+0x10))
        f.put(second+0x20,struct.pack('<Q',external));f.put(external,number(9).raw)
        result=f.image().closure(value(9,closure))
        self.assertEqual([u['closed'] for u in result['upvalues']],[True,False])
        self.assertEqual([u['value']['payload_hex'] for u in result['upvalues']],
                         [hex(number(8).bits),hex(number(9).bits)])

    def test_raw_absent_event_does_not_create_it(self):
        f=Fixture();events=f.table();key=f.string(b'GameEvents')
        globals_table=f.table(entries=[(key,value(5,events))]);state=f.alloc(0x90)
        f.put(state+0x70,value(5,globals_table).raw)
        before=bytes(f.raw)
        result=inspect_named_event(f.image(),state)
        self.assertEqual(result['event_status'],'absent_raw_key')
        self.assertEqual(bytes(f.raw),before)
        self.assertFalse(result['metamethods_invoked'])

    def test_absent_event_preserves_factory_metatable_without_calling_indexer(self):
        f=Fixture();groups=f.table();indexer=f.alloc(0x40)
        f.put(indexer+0x10,struct.pack('<Q',0x36b4270))
        f.put(indexer+0x20,struct.pack('<h',1));f.put(indexer+0x30,value(5,groups).raw)
        index_key=f.string(b'__index')
        meta=f.table(entries=[(index_key,value(10,indexer))]);events=f.table()
        f.put(events+0x10,struct.pack('<Q',meta))
        key=f.string(b'GameEvents');global_table=f.table(entries=[(key,value(5,events))])
        state=f.alloc(0x90);f.put(state+0x70,value(5,global_table).raw)
        before=bytes(f.raw);result=inspect_named_event(f.image(),state)
        self.assertEqual(result['event_status'],'absent_raw_key')
        self.assertEqual(result['game_events']['index_closure']['function'],'0x36b4270')
        self.assertEqual(bytes(f.raw),before)
        self.assertNotIn('callback_arrays',result)

    def test_builtin_dispatch_storage_and_callback_closure_identity(self):
        f=Fixture()
        callback=f.alloc(0x30)
        f.put(callback+0x10,struct.pack('<Q',0x12345678))
        local=f.table([value(10,callback),number(7)])
        groups=f.table([value(5,local)])
        method=f.alloc(0x40)
        f.put(method+0x10,struct.pack('<Q',0x36b3540))
        f.put(method+0x20,struct.pack('<h',1))
        f.put(method+0x30,value(5,groups).raw)
        method_key=f.string(b'TestAll')
        event=f.table(entries=[(method_key,value(10,method))])
        event_key=f.string(b'CanStartOperation')
        events=f.table(entries=[(event_key,value(5,event))])
        global_key=f.string(b'GameEvents')
        globals_table=f.table(entries=[(global_key,value(5,events))])
        state=f.alloc(0x90);f.put(state+0x70,value(5,globals_table).raw)
        result=inspect_named_event(f.image(),state)
        array=result['callback_arrays'][0]
        self.assertEqual(array['objlen'],2)
        self.assertEqual(array['callbacks'][0]['closure']['function'],'0x12345678')
        self.assertEqual(array['callbacks'][1]['value']['tag'],3)
        self.assertNotIn('closure',array['callbacks'][1])
        # An overwritten method cannot be interpreted as the builtin even if
        # its upvalue happens to have the same shape.
        f.put(method+0x10,struct.pack('<Q',0x9999))
        result=inspect_named_event(f.image(),state)
        self.assertNotIn('callback_arrays',result)

    def test_null_state_and_singleton_are_unsupported_not_empty_registry(self):
        self.assertEqual(inspect_named_event(Image([]),0)['event_status'],'unavailable_null_main_lua_state')
        base=0x100000000
        segments=[(base+0x3b12df0,struct.pack('<Q',base+0x3c051c0)),
                  (base+0x3b13848,struct.pack('<Q',base+0x3c051b8)),
                  (base+0x3c051b8,bytes(16)),(0x6ec9e68,bytes(8))]
        roots=inspect_roots(Image(segments),base)
        self.assertIn('null_singleton',roots['main_vm_status'])
        self.assertEqual(roots['script_status'],'null')

    def test_actual_interposed_got_globals_are_followed(self):
        base=0x100000000
        segments=[(base+0x3b12df0,struct.pack('<Q',0x1000)),
                  (base+0x3b13848,struct.pack('<Q',0x2000)),
                  (0x1000,bytes(8)),(0x2000,bytes(8)),(0x6ec9e68,bytes(8))]
        roots=inspect_roots(Image(segments),base)
        self.assertEqual(roots['script_global_binding'],'0x1000')
        self.assertFalse(roots['script_global_is_XP2_definition'])
        self.assertEqual(roots['script_status'],'null')
        segments=[(base+0x3b12df0,bytes(8)),(base+0x3b13848,bytes(8)),(0x6ec9e68,bytes(8))]
        roots=inspect_roots(Image(segments),base)
        self.assertEqual(roots['script_status'],'null_GOT_binding_native_getter_unsafe')


if __name__ == '__main__':
    unittest.main()
