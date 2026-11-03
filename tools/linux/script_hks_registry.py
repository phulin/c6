#!/usr/bin/env python3
"""Read pinned Linux HKS tables from an immutable offline memory image.

No process access, native calls, Lua API calls, callbacks or gameplay execution.
Uncaptured memory and unsupported layouts raise SnapshotError.
"""
import argparse
from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import struct
from save_lifecycle_audit import GAME_SHA, HOST_SHA


class SnapshotError(ValueError):
    pass


@dataclass(frozen=True)
class Value:
    raw: bytes

    def __post_init__(self):
        if len(self.raw) != 16:
            raise SnapshotError('HksObject requires all 16 raw bytes')

    @property
    def tag(self):
        return self.raw[0] & 15

    @property
    def bits(self):
        return struct.unpack_from('<Q', self.raw, 8)[0]

    @property
    def number(self):
        if self.tag != 3:
            raise SnapshotError('number required')
        return struct.unpack_from('<d', self.raw, 8)[0]

    def record(self):
        return dict(raw=self.raw.hex(), tag=self.tag, payload_hex=hex(self.bits))


NIL = Value(bytes(16))  # Logical missing result, never substituted for captured raw bytes.


class Image:
    def __init__(self, segments, limit=1_000_000):
        self.segments = sorted((address, bytes(data)) for address, data in segments)
        self.limit = limit
        end = 0
        for address, data in self.segments:
            if address < end or address <= 0 or address + len(data) > 1 << 64:
                raise SnapshotError('overlapping or invalid memory extents')
            end = address + len(data)

    def read(self, address, size):
        if address <= 0 or size < 0 or size > self.limit * 40:
            raise SnapshotError('invalid or excessive read extent')
        for start, data in self.segments:
            if start <= address and address + size <= start + len(data):
                return data[address-start:address-start+size]
        raise SnapshotError(f'uncaptured memory: {address:#x}+{size:#x}')

    def integer(self, address, fmt='<Q'):
        return struct.unpack(fmt, self.read(address, struct.calcsize(fmt)))[0]

    def value(self, address):
        return Value(self.read(address, 16))

    def string(self, pointer):
        header = self.read(pointer, 20)
        length = struct.unpack_from('<Q', header, 8)[0] & ((1 << 62) - 1)
        if length > self.limit:
            raise SnapshotError('string exceeds snapshot budget')
        raw = self.read(pointer+20, length+1)
        if raw[-1] != 0:
            raise SnapshotError('interned string terminator mismatch')
        return raw[:-1]

    def closure(self, value):
        pointer = value.bits
        if value.tag == 10:
            header = self.read(pointer, 0x30)
            count = struct.unpack_from('<h', header, 0x20)[0]
            if count < 0 or count > self.limit:
                raise SnapshotError('unsupported C closure upvalue extent')
            raw = self.read(pointer, 0x30 + count*16)
            return dict(kind='cclosure', pointer=hex(pointer), raw=raw.hex(),
                        function=hex(struct.unpack_from('<Q', header, 0x10)[0]),
                        environment=hex(struct.unpack_from('<Q', header, 0x18)[0]),
                        upvalues=[Value(raw[0x30+i*16:0x40+i*16]).record() for i in range(count)])
        if value.tag == 9:
            header = self.read(pointer, 0x38)
            method = struct.unpack_from('<Q', header, 0x10)[0]
            count = self.integer(method+0x10, '<H')
            if count > self.limit:
                raise SnapshotError('Lua closure exceeds snapshot budget')
            raw = self.read(pointer, 0x38 + count*8)
            upvalues = []
            for i in range(count):
                upvalue = struct.unpack_from('<Q', raw, 0x38+i*8)[0]
                upraw = self.read(upvalue, 0x28)
                live = struct.unpack_from('<Q', upraw, 0x20)[0]
                upvalues.append(dict(pointer=hex(upvalue), raw=upraw.hex(),
                                     value_pointer=hex(live), closed=live == upvalue+0x10,
                                     value=self.value(live).record()))
            return dict(kind='hksclosure', pointer=hex(pointer), raw=raw.hex(),
                        method=hex(method), method_upvalue_count=count,
                        environment=hex(struct.unpack_from('<Q', header, 0x18)[0]),
                        upvalues=upvalues, method_code_and_debug_decoded=False)
        raise SnapshotError('unsupported callback kind; preserve raw value')


class Table:
    def __init__(self, image, pointer):
        self.image = image
        self.pointer = pointer
        self.header = image.read(pointer, 0x40)
        self.mask = struct.unpack_from('<I', self.header, 0x1c)[0]
        self.nodes = struct.unpack_from('<Q', self.header, 0x20)[0]
        self.array = struct.unpack_from('<Q', self.header, 0x28)[0]
        self.size = struct.unpack_from('<I', self.header, 0x30)[0]
        self.count = 0 if self.mask == 0xffffffff else self.mask + 1
        if self.count > image.limit or self.size > image.limit:
            raise SnapshotError('table exceeds snapshot budget')
        if self.count and self.count & (self.count-1):
            raise SnapshotError('hash node count must be a power of two')
        if bool(self.count) != bool(self.nodes) or (self.size and not self.array):
            raise SnapshotError('inconsistent table storage')
        # Includes the reverse-order collision links preceding the nodes.
        self.hash_raw = image.read(self.nodes-8*self.count,40*self.count) if self.count else b''
        self.array_raw = image.read(self.array,16*self.size) if self.size else b''

    def array_value(self, index):
        if not 1 <= index <= self.size:
            raise SnapshotError('physical array index outside extent')
        return Value(self.array_raw[(index-1)*16:index*16])

    def node(self, index):
        if not 0 <= index < self.count:
            raise SnapshotError('hash node outside extent')
        offset = 8*self.count + index*32
        return (Value(self.hash_raw[offset:offset+16]),
                Value(self.hash_raw[offset+16:offset+32]))

    def chain(self, index):
        seen = set()
        while True:
            if index in seen:
                raise SnapshotError('cyclic hash chain')
            seen.add(index)
            yield index, self.node(index)
            pointer = struct.unpack_from('<Q',self.hash_raw,8*(self.count-index-1))[0]
            if not pointer:
                return
            if pointer < self.nodes or (pointer-self.nodes) % 32:
                raise SnapshotError('invalid collision link')
            index = (pointer-self.nodes)//32
            if index >= self.count:
                raise SnapshotError('collision link outside hash extent')

    def find_integer_node(self, integer):
        if not -0x80000000 <= integer <= 0x7fffffff:
            raise SnapshotError('native integer key exceeds i32')
        if self.count:
            for index, (key, value) in self.chain(integer & self.mask):
                if key.tag == 3 and key.number == integer:
                    return index, value
        return None

    def integer(self, integer):
        if 1 <= integer <= self.size:
            return self.array_value(integer)
        found = self.find_integer_node(integer)
        return found[1] if found is not None else NIL

    def lookup_string(self, data):
        # Find an existing intern identity without interning/allocating a key.
        candidates = set()
        nil_string_nodes = []
        for i in range(self.count):
            key, item = self.node(i)
            if key.tag != 4:
                continue
            if item.tag == 0:
                # GC does not mark keys of nil-valued nodes. The pointer may
                # be stale; retain raw node bytes without dereferencing it.
                nil_string_nodes.append(i)
                continue
            if self.image.string(key.bits) == data:
                candidates.add(key.bits)
        if len(candidates) > 1:
            raise SnapshotError('duplicate intern identities for key bytes')
        if not candidates:
            return dict(status='absent_or_nil_raw_key' if nil_string_nodes else 'absent_raw_key',
                        value=None, unclassified_nil_string_nodes=nil_string_nodes)
        pointer = candidates.pop()
        natural = (pointer ^ (pointer >> 5) ^ (pointer >> 8)) & self.mask
        for index, (key, value) in self.chain(natural):
            if key.tag == 4 and key.bits == pointer:
                return dict(status='raw_key_with_nil_value' if value.tag == 0 else 'present_raw_key',
                            value=value, node_index=index, intern_pointer=hex(pointer))
        raise SnapshotError('matching string node is unreachable by native hash lookup')

    def entries(self):
        # lua_next from nil on an unchanged HashTable: array, then physical hash order.
        for i in range(1,self.size+1):
            value = self.array_value(i)
            if value.tag:
                key = Value(struct.pack('<I4xd',3,float(i)))
                yield key,value
        for i in range(self.count):
            key,value = self.node(i)
            if value.tag:
                if key.tag == 0:
                    raise SnapshotError('non-nil value with nil key')
                yield key,value

    def length(self):
        # Original contiguousArraySize binary/exponential search; holes are not
        # replaced by the count of entries or the first missing numeric index.
        size = self.size
        if size and self.array_value(size).tag == 0:
            lo,hi = 0,size
            while hi-lo > 1:
                mid = (lo+hi)//2
                if self.array_value(mid).tag:
                    lo = mid
                else:
                    hi = mid
            return lo
        if not self.count:
            return size
        lo,hi = size,size+1
        while True:
            found = self.find_integer_node(hi)
            if found is None or found[1].tag == 0:
                break
            lo = hi
            hi = (hi*2) & 0xffffffff
            if hi & 0x80000000:
                # Native overflow fallback tests array values, then hash key
                # presence (including a nil-valued hash node).
                for index in range(1,size+1):
                    if self.array_value(index).tag == 0:
                        return index-1
                index = size+1
                for _ in range(self.image.limit):
                    if self.find_integer_node(index) is None:
                        return index-1
                    index += 1
                raise SnapshotError('length fallback exceeds snapshot budget')
        while hi-lo > 1:
            mid = (lo+hi)//2
            found = self.find_integer_node(mid)
            if found is not None and found[1].tag:
                lo = mid
            else:
                hi = mid
        return lo

    def record(self):
        return dict(pointer=hex(self.pointer), header_raw=self.header.hex(),
                    hash_storage_raw=self.hash_raw.hex(), array_storage_raw=self.array_raw.hex(),
                    hash_node_count=self.count, array_size=self.size,
                    metatable_pointer=hex(struct.unpack_from('<Q', self.header, 0x10)[0]),
                    entries=[dict(key=k.record(),value=v.record()) for k,v in self.entries()])


def load_image(path):
    manifest = json.loads(path.read_text())
    if manifest.get('schema') != 'c6-linux-script-hks-image-v1':
        raise SnapshotError('unsupported image schema')
    if manifest.get('host_sha256') != HOST_SHA or manifest.get('gamecore_sha256') != GAME_SHA:
        raise SnapshotError('original identity mismatch')
    if manifest.get('host_load_bias') != 0:
        raise SnapshotError('host symbols require zero ET_EXEC load bias')
    configuration = manifest.get('configuration_sha256','')
    if len(configuration) != 64 or any(c not in '0123456789abcdef' for c in configuration):
        raise SnapshotError('configuration identity required')
    segments = []
    for row in manifest['segments']:
        source = path.parent/row['file']
        raw = source.read_bytes()
        if hashlib.sha256(raw).hexdigest() != row['sha256'] or len(raw) != row['size']:
            raise SnapshotError('memory segment identity mismatch')
        segments.append((int(row['address'],0),raw))
    return manifest,Image(segments)


def inspect_roots(image, gamecore_base):
    # Follow actual runtime GOT bindings exactly as the original pure getters
    # do. Defined symbol VAs alone do not rule out ELF interposition.
    script_global = image.integer(gamecore_base+0x3b12df0)
    utility_global = image.integer(gamecore_base+0x3b13848)
    script = image.integer(script_global) if script_global else 0
    utility = image.integer(utility_global) if utility_global else 0
    singleton = image.integer(0x6ec9e68)
    result = dict(script_system=hex(script), utility=hex(utility), main_vm_singleton=hex(singleton),
                  script_global_binding=hex(script_global), utility_global_binding=hex(utility_global),
                  script_global_is_XP2_definition=script_global == gamecore_base+0x3c051c0,
                  utility_global_is_XP2_definition=utility_global == gamecore_base+0x3c051b8)
    for name,pointer,expected in [('script',script,0x53a63a8),('utility',utility,0x53a6c90)]:
        result[name+'_status'] = 'null' if not pointer else 'present'
        if pointer:
            vptr = image.integer(pointer)
            result[name+'_vptr'] = hex(vptr)
            result[name+'_concrete_binding'] = vptr == expected
    if not script_global:
        result['script_status'] = 'null_GOT_binding_native_getter_unsafe'
    if not utility_global:
        result['utility_status'] = 'null_GOT_binding_native_getter_unsafe'
    if not singleton:
        result['main_vm_status'] = 'null_singleton_native_CallTestAll_would_dereference_null'
        result['main_lua_state'] = '0x0'
        return result
    result['main_vm_header_raw'] = image.read(singleton,0x18).hex()
    result['main_vm_vptr'] = hex(image.integer(singleton))
    result['main_vm_is_AppLuaScriptSystem'] = image.integer(singleton) == 0x53aaa00
    state = image.integer(singleton+0x10)
    result['main_lua_state'] = hex(state)
    result['main_vm_status'] = 'nonnull_state_not_initialization_proof' if state else 'null_state_native_call_unsafe'
    return result


def table_provenance(image, table):
    result = table.record()
    meta = struct.unpack_from('<Q',table.header,0x10)[0]
    if meta:
        if meta & 1:
            raise SnapshotError('struct metatable remains unsupported; preserve image')
        metatable = Table(image,meta)
        result['metatable'] = metatable.record()
        index = metatable.lookup_string(b'__index')
        result['index_status'] = index['status']
        if index['value'] is not None:
            result['index_value'] = index['value'].record()
            if index['value'].tag in (9,10):
                result['index_closure'] = image.closure(index['value'])
    return result


def inspect_named_event(image, lua_state):
    if not lua_state:
        return dict(event_status='unavailable_null_main_lua_state', callbacks_executed=False,
                    metamethods_invoked=False, gameplay_supported=False)
    global_value = image.value(lua_state+0x70)
    if global_value.tag != 5:
        raise SnapshotError('main global object is not a HashTable')
    globals_table = Table(image,global_value.bits)
    game_events = globals_table.lookup_string(b'GameEvents')
    result = dict(global_value=global_value.record(),game_events_status=game_events['status'],
                  callbacks_executed=False,metamethods_invoked=False, gameplay_supported=False,
                  globals=table_provenance(image,globals_table))
    factory = globals_table.lookup_string(b'GameEventFactory')
    result['factory_status'] = factory['status']
    if factory['value'] is not None:
        result['factory_value'] = factory['value'].record()
        if factory['value'].tag == 5:
            result['factory'] = table_provenance(image,Table(image,factory['value'].bits))
    if game_events['value'] is None or game_events['value'].tag != 5:
        result['event_status'] = 'unavailable_raw_GameEvents'
        return result
    events = Table(image,game_events['value'].bits)
    event = events.lookup_string(b'CanStartOperation')
    result['event_status'] = event['status']
    result['game_events'] = table_provenance(image,events)
    if event['value'] is None or event['value'].tag != 5:
        result['event_value'] = event['value'].record() if event['value'] else None
        return result
    table = Table(image,event['value'].bits)
    test = table.lookup_string(b'TestAll')
    result['event'] = table_provenance(image,table)
    result['test_all_status'] = test['status']
    if test['value'] is not None:
        result['test_all_value'] = test['value'].record()
        if test['value'].tag in (9,10):
            closure = image.closure(test['value'])
            result['test_all_closure'] = closure
            if closure['kind'] == 'cclosure' and closure['function'] == '0x36b3540':
                upvalues = closure['upvalues']
                if not upvalues or upvalues[0]['tag'] != 5:
                    raise SnapshotError('built-in TestAll shared group upvalue missing')
                groups = Table(image,int(upvalues[0]['payload_hex'],0))
                result['groups'] = table_provenance(image,groups)
                result['callback_arrays'] = []
                for key,value in groups.entries():
                    if value.tag == 5:
                        array = Table(image,value.bits)
                        length = array.length()
                        if length > image.limit:
                            raise SnapshotError('callback length exceeds snapshot budget')
                        callbacks = []
                        for i in range(1,length+1):
                            callback = array.integer(i)
                            record = dict(index=i,value=callback.record())
                            if callback.tag in (9,10):
                                record['closure'] = image.closure(callback)
                            callbacks.append(record)
                        result['callback_arrays'].append(dict(group_key=key.record(),
                            table=table_provenance(image,array), objlen=length, callbacks=callbacks))
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--image',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args = parser.parse_args()
    manifest,image = load_image(args.image)
    roots = inspect_roots(image,int(manifest['gamecore_base'],0))
    result = inspect_named_event(image,int(roots['main_lua_state'],0))
    result['roots'] = roots
    result['configuration_sha256'] = manifest['configuration_sha256']
    result['image_manifest_sha256'] = hashlib.sha256(args.image.read_bytes()).hexdigest()
    result['source_only_decoder'] = True
    result['capture_boundary_accepted'] = False
    result['limits'] = ['Snapshot identity/bytes do not prove producer quiescence or native parity.',
                        'Method code/prototype/debug, transitive environments and gameplay effects remain unsupported.']
    args.output.write_text(json.dumps(result,indent=2)+'\n')


if __name__ == '__main__':
    main()
