#!/usr/bin/env python3
"""Passive Military observation callbacks. Never starts or calls an inferior.

The ordinary-load runner owns breakpoint installation and lifecycle attribution.
See docs/linux-military-initialization-observer.md for the callback protocol.
"""
import hashlib
import json
import struct
from pathlib import Path

GAME_SHA = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
HOST_SHA = '106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
WIRE_METHODS = [(0x21141c0, 9, 'standard count invoke', 'ac6880ae060f8c91a820aa152a98220739edb4a0e1aa9f07c513d7b833fbf5ed'),
                (0x2114200, 5, 'standard count wrapper', 'b1099987538d03761438c7b300c09363be11262870044edcfc1d516a4fbdbc50'),
                (0x1fce0e0, 67, 'std function count dispatch', '60e91f8f6789b72330131d897bb22b8c482889cf77fa67c653c98797f3222500'),(0x21453f0, 551, 'Military Write', '3f934803bfe326f35a629683ab2462fd0141671377b0d61797b0c40a8a489c20'),
                (0x2145620, 1472, 'Military Read', '34f26f5196c12039960ac4fb8581e560a7f30b59228cee65a8b1cdf754d01f9f')]
VECTOR_OFFSETS = {'allowed': 0x158, 'per_war': 0x180, 'current': 0x1a8}
# These are SO RVAs. The host ET_EXEC's 0x400000 mapping is not a bias.
SITES = {'read_entry': 0x2145620, 'write_entry': 0x21453f0,
         'initialize_entry': 0x2145be0, 'personality_return': 0x2145c07,
         'leader_return': 0x2145c17, 'query_entry': 0x203ac30,
         'count_entry': 0x2114220, 'hash_array_entry': 0x214d7b0, 'write_guard': 0x2145504,
         'read_initialized': 0x2145669}
RETURN_EVENTS = {'read_return', 'write_return', 'initialize_return',
                 'count_return'}
PROVENANCE = ('lifecycle_id', 'owner_lifetime_id', 'configuration_sha256',
              'rules_inputs_sha256', 'dlc_sha256', 'mods_sha256', 'input_save_sha256', 'input_core_sha256',
              'runtime_profile_sha256', 'runner_source_sha256')


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


class Memory:
    """Bounded exact scalar/vector reads; never evaluates native expressions."""
    def __init__(self, read, max_items=65536):
        self.read_bytes = read
        self.max_items = max_items

    def read(self, pointer, size):
        require(isinstance(pointer, int) and 0 < pointer < 2**64, 'invalid pointer')
        require(0 <= size <= self.max_items * 40 and pointer + size <= 2**64,
                'read extent overflow/limit')
        raw = bytes(self.read_bytes(pointer, size)) if size else b''
        require(len(raw) == size, 'short memory read')
        return raw

    def num(self, pointer, size=8, signed=False):
        return int.from_bytes(self.read(pointer, size), 'little', signed=signed)

    def vector(self, pointer):
        raw = self.read(pointer, 24)
        data, capacity, padding, size, tail_padding = struct.unpack('<QIIII', raw)
        require(size <= capacity <= self.max_items, 'invalid vector count/capacity')
        require(not capacity or data != 0, 'null vector backing')
        # Verify the full capacity is readable, but expose only initialized values.
        if capacity:
            self.read(data, capacity * 4)
        values = list(struct.unpack('<' + 'i' * size, self.read(data, size * 4))) if size else []
        return dict(data_pointer=hex(data), capacity_u32=capacity, size_u32=size,
                    padding_u32=padding, tail_padding_u32=tail_padding,
                    values_i32=values)

    def military(self, pointer, player=None):
        owner = self.num(pointer + 0x98)
        require(owner != 0 and (player is None or owner == player), 'Military owner mismatch')
        alive = self.num(owner + 0x278, 1)
        require(alive in (0, 1), 'invalid WasEverAlive byte')
        return dict(military_pointer=hex(pointer), player_pointer=hex(owner),
                    player_id_i32=self.num(owner + 0xe0, 4, True),
                    was_ever_alive=bool(alive), handicap_i32=self.num(owner + 0x2d8, 4, True),
                    vectors={name: self.vector(pointer + offset)
                             for name, offset in VECTOR_OFFSETS.items()})

    def definitions(self, pointer):
        begin, end, capacity = struct.unpack('<QQQ', self.read(pointer, 24))
        require(begin <= end <= capacity and (end-begin) % 8 == 0
                and (capacity-begin) % 8 == 0, 'definition span')
        require((capacity-begin)//8 <= self.max_items, 'definition capacity limit')
        if capacity != begin:
            self.read(begin, capacity-begin)
        rows = []
        for address in range(begin, end, 8):
            definition = self.num(address)
            raw = self.read(definition, 40)
            row, index, type_hash, default = struct.unpack_from('<iIIi', raw, 16)
            rows.append(dict(pointer=hex(definition), database_row_i32=row,
                             collection_index_u32=index, operation_type_hash_u32=type_hash,
                             default_value_i32=default))
        return dict(collection_pointer=hex(pointer), begin_pointer=hex(begin),
                    end_pointer=hex(end), capacity_pointer=hex(capacity), ordered=rows)


def validate_binding(binding, kind):
    require(isinstance(binding, dict), 'missing lifecycle binding')
    for key in PROVENANCE:
        require(isinstance(binding.get(key), str) and binding[key], 'missing binding: ' + key)
    for key in PROVENANCE[2:]:
        require(len(binding[key]) == 64 and all(c in '0123456789abcdef' for c in binding[key]),
                'invalid identity: ' + key)
    database = binding.get('rules_database_sha256')
    if database is None:
        require(binding.get('live_database_snapshot_unavailable') is True, 'unclassified missing gameplay database snapshot')
    else:
        require(isinstance(database, str) and len(database) == 64
                and all(c in '0123456789abcdef' for c in database), 'invalid gameplay database snapshot identity')
    require(binding.get('kind') == kind, 'binding kind mismatch')
    require(isinstance(binding.get('occurrence_id'), str) and binding['occurrence_id'], 'missing occurrence')
    require(isinstance(binding.get('stream_pointer'), int) and binding['stream_pointer'] > 0,
            'missing actual stream pointer')
    if kind == 'write':
        require(isinstance(binding.get('output_id'), str) and binding['output_id'], 'missing output identity')
    # Round-trip gives the observer its own immutable copy, not a runner-owned dict.
    return json.loads(json.dumps(binding))


class Observer:
    """Fail-closed callbacks from paused original instruction boundaries.

    read is a bounded inferior.read_memory adapter. emit writes private JSONL.
    Registers are integer x86-64 SysV values. Return events MUST come from a
    matching native frame-return callback, never a caller's guessed completion.
    """
    def __init__(self, read, emit, module_bias, manifest):
        require(manifest['schema'] == 'c6-linux-military-observer-plan-v1', 'plan schema')
        require(manifest['gamecore_sha256'] == GAME_SHA and manifest['host_sha256'] == HOST_SHA,
                'target identity')
        require(manifest['observer_sha256'] == digest(__file__), 'observer source changed')
        self.memory, self.emit = Memory(read), emit
        self.bias, self.manifest = module_bias, manifest
        self.calls, self.sequence, self.failed = [], 0, False
        self.last_thread = None
        self.used_frames, self.writes = set(), {}
        self.record('observer_begin', gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA,
                    observer_sha256=manifest['observer_sha256'],
                    dwarf_sha256=manifest.get('dwarf_sha256'),
                    extended_dwarf_sha256=manifest.get('extended_dwarf_sha256'),
                    capture_complete=False,
                    plan_sha256=hashlib.sha256(json.dumps(manifest, sort_keys=True, separators=(',', ':')).encode()).hexdigest(),
                    runtime_gate=manifest.get('runtime_gate'))

    def record(self, event, **fields):
        self.sequence += 1
        self.emit(dict(schema='c6-linux-military-observation-v1', sequence=self.sequence,
                       event=event, thread=self.last_thread, **fields))

    def current(self, kind=None):
        for call in reversed(self.calls):
            if kind is None or call['kind'] == kind:
                return call
        raise ValueError('missing matching original entry')

    def snapshot(self, call):
        return self.memory.military(call['military'], call.get('player'))

    def observe(self, event, registers, thread, binding=None, frame_id=None):
        """Returns False on success; True leaves inferior paused after refusal.

        frame_id must identify one dynamic entry/frame, unique in this capture;
        a return reuses its entry ID. Frame unwind/exit must call abort().
        """
        if self.failed:
            return True
        try:
            self.last_thread = json.loads(json.dumps(thread))
            require(event in SITES or event in RETURN_EVENTS, 'unknown checkpoint')
            if event in SITES:
                require(registers['pc'] == self.bias + SITES[event], 'wrong original PC')
                expected = bytes.fromhex(self.manifest['site_bytes'][event])
                require(self.memory.read(registers['pc'], len(expected)) == expected, 'live instruction mismatch')
            require(frame_id is not None, 'missing dynamic frame identity')
            if event in ('read_entry', 'write_entry', 'initialize_entry', 'count_entry'):
                require(frame_id not in self.used_frames, 'reused dynamic frame identity')
                self.used_frames.add(frame_id)
            if self.calls:
                require(thread == self.current()['thread'], 'thread changed')
            if event in ('read_entry', 'write_entry'):
                require(not self.calls, 'overlapping root Military calls')
                kind = event.split('_')[0]
                b = validate_binding(binding, kind)
                require(b['stream_pointer'] == registers['rdi'], 'stream binding mismatch')
                call = dict(kind=kind, military=registers['rsi'], stream=registers['rdi'],
                            binding=b, thread=thread, frame_id=frame_id,
                            return_pc=self.memory.num(registers['rsp']), entry_rsp=registers['rsp'],
                            initialized=False, guard=False, arrays=[])
                self.calls.append(call)
                # Read has not yet installed its saved owner; do not read stale ownership.
                self.record(event, binding=b, military_pointer=hex(call['military']),
                            stream_pointer=hex(call['stream']), frame_id=frame_id,
                            snapshot=self.snapshot(call) if kind == 'write' else None,
                            prior_owner_pointer=hex(self.memory.num(call['military'] + 0x98)),
                            prior_vectors={name: self.memory.vector(call['military'] + offset)
                                           for name, offset in VECTOR_OFFSETS.items()})
            elif event == 'initialize_entry':
                root = self.current()
                require(root['kind'] == 'read', 'initialization outside bound original Read')
                require(registers['rdi'] == root['military'], 'wrong initialized Military')
                require(self.memory.num(registers['rsp']) == self.bias + SITES['read_initialized'],
                        'initialization not from original Military.Read')
                call = dict(kind='initialize', military=root['military'], player=registers['rsi'],
                            binding=root['binding'], thread=thread, frame_id=frame_id,
                            return_pc=self.memory.num(registers['rsp']), entry_rsp=registers['rsp'], observations={})
                self.calls.append(call)
                self.record(event, binding=call['binding'], frame_id=frame_id, snapshot=self.snapshot(call))
            else:
                call = self.current()
                require(thread == call['thread'], 'thread changed')
                if event in RETURN_EVENTS:
                    require(call['kind'] + '_return' == event and frame_id == call['frame_id'],
                            'unmatched dynamic return')
                    require(registers['pc'] == call['return_pc'], 'wrong native return PC')
                    require(registers['rsp'] == call['entry_rsp'] + 8, 'unmatched native return stack')
                    if call['kind'] == 'count':
                        count = registers['rax'] & 0xffffffff
                        require(count == len(call['definitions']['ordered']), 'native count mismatch')
                        self.current('initialize')['observations']['active_definitions'] = call['definitions']
                        self.record(event, frame_id=frame_id, count_u32=count, definitions=call['definitions'])
                    else:
                        if call['kind'] == 'read':
                            require(call['initialized'], 'missing original InitializeLists return')
                        elif call['kind'] == 'write':
                            require(call['guard'], 'missing original writer guard')
                            expected_count = 3 if call['present'] else 0
                            require(len(call['arrays']) == expected_count, 'missing/extra writer hash arrays')
                            key = call['binding']['occurrence_id']
                            require(key not in self.writes, 'reused writer occurrence')
                            self.writes[key] = call['binding']
                        elif call['kind'] == 'initialize':
                            obs = call['observations']
                            require('has_ai_personality' in obs, 'missing personality observation')
                            if obs['has_ai_personality']:
                                require('leader_i32' in obs, 'missing leader observation')
                                if obs['leader_i32'] != -1:
                                    require('leader_query' in obs, 'missing actual leader query observation')
                                if obs['leader_i32'] == -1 or int(obs['leader_query']['query_pointer'], 16) != 0:
                                    require('active_definitions' in obs, 'missing actual active definition count')
                            self.current('read')['initialized'] = True
                        self.record(event, binding=call['binding'], frame_id=frame_id,
                                    snapshot=self.snapshot(call), observations=call.get('observations', {}),
                                    normal_native_return=True)
                    self.calls.pop()
                elif event == 'count_entry':
                    init = self.current('initialize')
                    obs = init['observations']
                    require(obs.get('has_ai_personality') is True and 'leader_i32' in obs,
                            'count outside actual personality/leader branch')
                    return_pc = self.memory.num(registers['rsp'])
                    if obs['leader_i32'] == -1:
                        require(return_pc == self.bias + 0x2145dd2
                                and registers['rbx'] == init['military'] + 0x158,
                                'count outside original unknown-leader assignment')
                    else:
                        query = obs.get('leader_query')
                        require(query is not None and int(query['query_pointer'], 16) != 0,
                                'count without actual nonnull allowed query')
                        require(return_pc == self.bias + 0x203ac8b
                                and registers['rbx'] == int(query['handler_pointer'], 16)
                                and registers['rbp'] == init['military'] + 0x158,
                                'count outside original allowed-query dispatch')
                        require(registers['rdi'] == int(query['captured_definition_pointer'], 16),
                                'count receiver differs from actual standard callback capture')
                    definitions = self.memory.definitions(registers['rdi'])
                    self.calls.append(dict(kind='count', thread=thread, frame_id=frame_id,
                                           return_pc=self.memory.num(registers['rsp']), entry_rsp=registers['rsp'], definitions=definitions))
                    self.record(event, frame_id=frame_id, binding=init['binding'], definitions=definitions)
                elif event in ('personality_return', 'leader_return', 'query_entry'):
                    call = self.current('initialize')
                    require(frame_id == call['frame_id'], 'branch frame mismatch')
                    if event == 'personality_return':
                        value = registers['rax'] & 255
                        require(value in (0, 1), 'invalid native personality result')
                        call['observations']['has_ai_personality'] = bool(value)
                    elif event == 'leader_return':
                        value = registers['rax'] & 0xffffffff
                        call['observations']['leader_i32'] = value if value < 2**31 else value-2**32
                    else:
                        require(self.memory.num(registers['rsp']) == self.bias + 0x2145c9b,
                                'query is not allowed-vector InitializeLists call')
                        require(registers['rsi'] == call['military'] + 0x158, 'query destination mismatch')
                        handler = registers['rdi']
                        query_pointer = self.memory.num(handler)
                        callback = self.memory.num(handler + 0x40)
                        table, captured_definitions = None, None
                        if query_pointer:
                            require(callback != 0, 'missing actual standard count callback')
                            table = self.memory.num(callback)
                            require(self.memory.num(table + 0x30) == self.bias + 0x21141c0,
                                    'nonstandard operation count callback')
                            captured_definitions = self.memory.num(callback + 8)
                        call['observations']['leader_query'] = dict(handler_pointer=hex(handler),
                            count_callback_pointer=hex(callback), count_callback_vtable_pointer=hex(table) if table is not None else None,
                            captured_definition_pointer=hex(captured_definitions) if captured_definitions is not None else None,
                            query_pointer=hex(query_pointer),
                            default_value_i32=self.memory.num(handler + 0x140, 4, True),
                            civilization_i32=signed32(registers['rcx']), handicap_i32=signed32(registers['r8']))
                    self.record(event, binding=call['binding'], frame_id=frame_id,
                                observations=dict(call['observations']))
                elif event == 'hash_array_entry':
                    call = self.current('write')
                    require(frame_id == call['frame_id'] and call['guard'] and call['present'],
                            'hash array outside attributed writer guard')
                    index = len(call['arrays'])
                    returns = (0x2145526, 0x2145548, 0x214556a)
                    require(index < 3 and self.memory.num(registers['rsp']) == self.bias + returns[index],
                            'wrong ordered writer array call')
                    require(registers['rdi'] == call['stream'], 'hash array stream mismatch')
                    name = tuple(VECTOR_OFFSETS)[index]
                    vector = self.memory.vector(call['military'] + VECTOR_OFFSETS[name])
                    require(registers['rsi'] == int(vector['data_pointer'], 16), 'hash array data mismatch')
                    definitions = self.memory.definitions(registers['rdx'])
                    require(vector['size_u32'] >= len(definitions['ordered']), 'short native vector for definitions')
                    if call['arrays']:
                        require(definitions == call['arrays'][0]['definitions'], 'definitions changed between arrays')
                    array = dict(name=name, vector=vector, definitions=definitions)
                    call['arrays'].append(array)
                    self.record(event, binding=call['binding'], frame_id=frame_id, **array)
                elif event == 'read_initialized':
                    require(call['kind'] == 'read' and frame_id == call['frame_id'] and call['initialized'], 'wrong/incomplete Read frame')
                    self.record(event, binding=call['binding'], frame_id=frame_id, snapshot=self.snapshot(call))
                elif event == 'write_guard':
                    require(call['kind'] == 'write' and frame_id == call['frame_id'], 'wrong Write frame')
                    require(registers['rbx'] == call['military'] and registers['r14'] == call['stream'],
                            'writer receiver/stream mismatch')
                    snapshot = self.snapshot(call)
                    require((registers['rax'] & 0xffffffff) == snapshot['vectors']['allowed']['size_u32'],
                            'writer size result mismatch')
                    call['guard'] = True
                    call['present'] = snapshot['vectors']['allowed']['size_u32'] != 0
                    self.record(event, binding=call['binding'], frame_id=frame_id, snapshot=snapshot,
                                operation_type_arrays_present=call['present'])
            return False
        except Exception as error:
            self.abort(str(error))
            return True

    def bind_output(self, occurrence_id, output_id, saved_output_sha256, output_core_sha256,
                    core_range):
        """Runner attests its completed saved artifact, after original Write return.

        This is a separate attribution record, not an inference from vector count.
        The capture reviewer must verify the supplied actual artifact hashes/range.
        """
        try:
            require(not self.failed, 'capture already refused')
            binding = self.writes[occurrence_id]
            require(output_id == binding['output_id'], 'wrong saved output')
            require(not binding.get('output_bound'), 'output already bound')
            for value in (saved_output_sha256, output_core_sha256):
                require(len(value) == 64 and all(c in '0123456789abcdef' for c in value), 'output digest')
            require(len(core_range) == 2 and all(type(x) is int for x in core_range)
                    and 0 <= core_range[0] < core_range[1], 'canonical Core range')
            binding['output_bound'] = True
            self.record('saved_output_binding', occurrence_id=occurrence_id, output_id=output_id,
                        saved_output_sha256=saved_output_sha256, output_core_sha256=output_core_sha256,
                        military_core_range=core_range)
            return False
        except Exception as error:
            self.abort(str(error))
            return True

    def abort(self, reason):
        self.failed = True
        self.record('capture_refused', reason=reason, capture_complete=False,
                    pending=[dict(kind=c['kind'], frame_id=c['frame_id']) for c in self.calls])

    def close(self):
        if self.calls:
            self.abort('inferior exit/unwind before matching native return')
        if any(not binding.get('output_bound') for binding in self.writes.values()):
            self.abort('normal writer return lacks actual saved output attribution')
        self.record('capture_end', capture_complete=False,
                    reason='raw observations require separate lifecycle and output attribution audit',
                    observer_failed=self.failed)


def signed32(value):
    value &= 0xffffffff
    return value if value < 2**31 else value-2**32


def prepare(module, host, output):
    """Static preparation only. Does not authorize or run an ordinary-load capture."""
    from player_wmds_audit import Elf
    from ai_military_initialization_audit import METHODS, DWARF_TYPES, DWARF_SHA
    import subprocess
    elf = Elf(Path(module), GAME_SHA)
    require(digest(host) == HOST_SHA, 'host identity')
    for address, size, name, expected in [*METHODS, *WIRE_METHODS]:
        require(hashlib.sha256(elf.read(address, size)).hexdigest() == expected, 'method bytes: ' + name)
    require(elf.read(0x1fcc140, 7).hex() == '8b87e0000000c3', 'Player ID getter bytes')
    extra_types = ['Types::Vector<int, Types::LinearAlloc::Default<1, 0> >::LINEAR_BLOCK',
                   'GameCore::Definition::AiOperationType',
                   'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::AiOperationType>']
    extra_sha = 'e78ead2c077c5ea96625681dfaa9cf0bc5736d8e93558c2c371d899d8bdfbc87'
    for names, expected in ((DWARF_TYPES, DWARF_SHA), (extra_types, extra_sha)):
        command = ['gdb', '-nx', '-batch', str(module)]
        for name in names:
            command += ['-ex', "ptype /o '" + name + "'"]
        raw = subprocess.check_output(command)
        require(hashlib.sha256(raw).hexdigest() == expected, 'matching DWARF gate')
    plan = dict(schema='c6-linux-military-observer-plan-v1', gamecore_sha256=GAME_SHA,
                host_sha256=HOST_SHA, observer_sha256=digest(__file__),
                module=str(Path(module).resolve()), host=str(Path(host).resolve()),
                site_bytes={event: elf.read(address, 8).hex() for event, address in SITES.items()},
                original_execution=False, capture_complete=False,
                dwarf_sha256=DWARF_SHA, extended_dwarf_sha256=extra_sha)
    Path(output).write_text(json.dumps(plan, indent=2) + '\n')
    return plan


class HardwareSchedule:
    """Pure drain-plan model: reserves one of four x86 HW lanes for provenance.

    This never creates/deletes/enables a breakpoint. The runner applies returned
    sites only in its outer drain, after stop() unwinds. Unexpected events refuse.
    """
    def __init__(self):
        self.phase, self.root_return, self.count_return, self.arrays = 'idle', None, None, 0
        self.points = ['read_entry', 'write_entry']

    def advance(self, event, return_pc=None, value=None):
        if event == 'read_initialized' and self.phase == 'read-return':
            return list(self.points)
        if self.phase == 'idle' and event in ('read_entry', 'write_entry'):
            require(type(return_pc) is int and return_pc > 0, 'missing actual root return')
            self.root_return = return_pc
            self.phase = 'init-entry' if event == 'read_entry' else 'write-guard'
            self.points = [('initialize_entry' if event == 'read_entry' else 'write_guard'), return_pc]
        elif self.phase == 'init-entry' and event == 'initialize_entry':
            self.phase = 'personality'
            self.points = ['personality_return', 'initialize_return', self.root_return]
        elif self.phase == 'personality' and event == 'personality_return':
            require(type(value) is bool, 'missing actual personality result')
            self.phase = 'leader' if value else 'init-return'
            self.points = (['leader_return'] if value else []) + ['initialize_return', self.root_return]
        elif self.phase == 'leader' and event == 'leader_return':
            require(type(value) is int, 'missing actual leader result')
            self.phase = 'count' if value == -1 else 'query'
            self.points = [('count_entry' if value == -1 else 'query_entry'), 'initialize_return', self.root_return]
        elif self.phase == 'query' and event == 'query_entry':
            require(type(value) is bool, 'missing actual nonnull query observation')
            self.phase = 'count' if value else 'init-return'
            self.points = (['count_entry'] if value else []) + ['initialize_return', self.root_return]
        elif self.phase == 'count' and event == 'count_entry':
            require(type(return_pc) is int and return_pc > 0, 'missing actual count return')
            self.phase, self.count_return = 'count-return', return_pc
            self.points = [return_pc, 'initialize_return', self.root_return]
        elif self.phase == 'count-return' and event == 'count_return':
            self.phase, self.points = 'init-return', ['initialize_return', self.root_return]
        elif self.phase == 'init-return' and event == 'initialize_return':
            self.phase, self.points = 'read-return', [self.root_return]
        elif self.phase == 'read-return' and event == 'read_return':
            self.phase, self.points = 'idle', ['read_entry', 'write_entry']
        elif self.phase == 'write-guard' and event == 'write_guard':
            require(type(value) is bool, 'missing actual writer presence result')
            self.phase = 'write-arrays' if value else 'write-return'
            self.arrays = 0
            self.points = (['hash_array_entry'] if value else []) + [self.root_return]
        elif self.phase == 'write-arrays' and event == 'hash_array_entry':
            self.arrays += 1
            if self.arrays == 3:
                self.phase, self.points = 'write-return', [self.root_return]
        elif self.phase == 'write-return' and event == 'write_return':
            self.phase, self.points = 'idle', ['read_entry', 'write_entry']
        else:
            raise ValueError('unexpected hardware schedule event: ' + self.phase + '/' + event)
        require(len(self.points) <= 3, 'hardware lane budget exceeded')
        return list(self.points)


def validate_runtime_profile(configuration):
    require(configuration.get('lower_core_military_initialization_observer', {}).get('mode')
            == 'ordinary_load_save_passive', 'wrong passive ordinary-load/save profile')
    for key in ('core_component_mode', 'lower_core_movement_cost', 'lower_core_domain_metadata',
                'lower_core_placement_metadata', 'lower_core_placement_handler_metadata',
                'lower_core_unit_operation_manager_metadata', 'full_save_request'):
        require(key in configuration and configuration[key] is None,
                'component/action profile enabled or unspecified: ' + key)


class GdbAdapter:
    """Read-only adapter; caller installs/owns debugger breakpoints and returns.

    Construct only after the original modules are mapped. No run/continue/native
    evaluate/call or breakpoint mutation is performed by this adapter.
    """
    def __init__(self, manifest_path, emit):
        import gdb
        self.gdb = gdb
        require(not gdb.parameter('non-stop'), 'all-stop mode required')
        manifest = json.loads(Path(manifest_path).read_text())
        # Static prepare deliberately supplies no runtime gate: a separately
        # reviewed runner must freeze its exact new profile before attachment.
        gate = manifest.get('runtime_gate')
        require(isinstance(gate, dict), 'missing reviewed runtime profile/source gate')
        require(digest(gate['report_path']) == gate['report_sha256'], 'runtime report changed')
        runtime = json.loads(Path(gate['report_path']).read_text())
        cfg = runtime['configuration']
        canonical = json.dumps(cfg, sort_keys=True, separators=(',', ':')).encode()
        require(hashlib.sha256(canonical).hexdigest() == gate['configuration_sha256']
                == runtime['configuration_sha256'], 'runtime configuration identity')
        validate_runtime_profile(cfg)
        require(gate['source_files'], 'missing frozen runner sources')
        for path, expected in gate['source_files'].items():
            require(digest(path) == expected, 'runner source changed: ' + path)
        self.runtime_gate = gate
        self.runner_source_sha256 = hashlib.sha256(json.dumps(gate['source_files'], sort_keys=True,
                                                  separators=(',', ':')).encode()).hexdigest()
        for name, expected in (('module', GAME_SHA), ('host', HOST_SHA)):
            require(digest(manifest[name]) == expected, name + ' on-disk identity')
        inferior = gdb.selected_inferior()
        require(inferior.pid > 0, 'no existing original inferior')
        self.pid = inferior.pid
        require(Path('/proc/%d/exe' % self.pid).resolve() == Path(manifest['host']).resolve(),
                'wrong mapped host executable')
        maps = self.mappings()
        module = str(Path(manifest['module']).resolve())
        bases = {lo - offset for lo, _, _, offset, path in maps if path == module and offset == 0}
        require(len(bases) == 1, 'missing/ambiguous mapped target module')
        bias = bases.pop()
        host = str(Path(manifest['host']).resolve())
        require(any(lo == 0x400000 and offset == 0 and path == host
                    for lo, _, _, offset, path in maps), 'host ET_EXEC mapping')
        self.observer = Observer(self.read, emit, bias, manifest)
        # Live full instruction bodies must still match the pinned file bytes.
        from ai_military_initialization_audit import METHODS
        for address, size, name, expected in [*METHODS, *WIRE_METHODS]:
            require(hashlib.sha256(self.read(bias + address, size)).hexdigest() == expected,
                    'live original method gate: ' + name)

    def mappings(self):
        rows = []
        for line in Path('/proc/%d/maps' % self.pid).read_text().splitlines():
            fields = line.split(maxsplit=5)
            lo, hi = (int(x, 16) for x in fields[0].split('-'))
            rows.append((lo, hi, fields[1], int(fields[2], 16), fields[5] if len(fields) > 5 else ''))
        return rows

    def read(self, pointer, size):
        if not size:
            return b''
        end = pointer + size
        cursor = pointer
        for lo, hi, permissions, _, _ in self.mappings():
            if lo <= cursor < hi and 'r' in permissions:
                cursor = min(hi, end)
            if cursor == end:
                return self.gdb.selected_inferior().read_memory(pointer, size).tobytes()
        raise ValueError('unreadable paused inferior extent')

    def checkpoint(self, event, binding=None, frame_id=None):
        frame = self.gdb.newest_frame()
        registers = {name: int(frame.read_register(name)) for name in
                     ('pc', 'rsp', 'rax', 'rdi', 'rsi', 'rdx', 'rcx', 'r8', 'rbx', 'rbp', 'r14')}
        if binding is not None:
            require(binding.get('runtime_profile_sha256') == self.runtime_gate['configuration_sha256'],
                    'binding differs from source-gated runtime profile')
            require(binding.get('runner_source_sha256') == self.runner_source_sha256,
                    'binding differs from frozen runner sources')
        return self.observer.observe(event, registers, list(self.gdb.selected_thread().ptid),
                                     binding, frame_id)


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--module', type=Path, required=True)
    parser.add_argument('--host', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    prepare(args.module, args.host, args.output)
