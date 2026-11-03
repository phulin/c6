#!/usr/bin/env python3
"""Prepare/install a bounded read-only GDB observer; this file never launches a game."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import struct
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
GAME_SHA = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
HOST_SHA = '106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
CONFIG_SHA = 'b6ca58d53073abc2afa9176a4ceba350fafe3f91ba12dbf84fd08e25a3be9a5b'
BEFORE_SHA = 'ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884'
PLOT_YIELDS_CONTENT_BUDGET = 256 * 1024

PRELOAD_SHA = '907b006e5225ac9816231eceab9124fedb2810f483ba5135232c380d4f89e9fa'
PARENT_SHA = '504571fe4cb8f194189bccc86940b820720d2f375de74cffedb9ccdb525c2029'
GATES = [(0x2c66430, '534889fb', 'monitor_entry'),
         (0x272361e, '4883c3304c89ff', 'monitor_native_caller_return'),
         (0x2c28137, '84c07518', 'has_adjacent_move_return'),
         (0x2021490, '8b87b8000000c3', 'actual_unit_id'),
         (0x2004670, '8b8730010000c3', 'actual_unit_owner'),
         (0x20e0710, '488d87100c0000c3', 'actual_operation_queue'),
         (0x2702f10, '8b87c0000000c3', 'actual_operation_queue_count'),
         (0x2661d30, '488b8768020000c3', 'game_effects_pointer'),
         (0x23292c0, '488b87a8000000c3', 'plot_yields_pointer'),
         (0x264c075, '89f0488b0f488d04c048c1e005488b0401', 'component_entry_stride288'),
         (0x2c0411b, '488b4f10833c81ff', 'hash_occupancy_not_ffffffff'),
         (0x2bf6118, '4531c0', 'null_contribution_caller'),
         (0x2a3ed98, '4d89c6', 'preserve_contribution'),
         (0x2a3edd3, '4d89f1', 'forward_contribution'),
         (0x2a3ee5e, '4c894c2418', 'save_contribution'),
         (0x2a3f08d, '48837c241800', 'null_contribution_test'),
         (0x2a3f097, '7424', 'skip_bonus_add_remove_path')]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def cid(raw):
    return list(struct.unpack('<hhI', raw))


class ObservationReadError(ValueError):
    def __init__(self, reason, **context):
        self.context = context
        super().__init__(reason)


class Decoder:
    """Pure bounded memory reads; read callback never calls an inferior function."""
    def __init__(self, read, base):
        self.read, self.base = read, base

    def uint(self, address, size=8):
        return int.from_bytes(self.read(address, size), 'little')

    def vector(self, address, width, limit, label):
        header = self.read(address, 24)
        begin, end, capacity = struct.unpack('<QQQ', header)
        if not begin <= end <= capacity or (end - begin) % width or (end - begin) // width > limit:
            raise ObservationReadError('invalid/out-of-profile vector header', label=label,
                address=hex(address), raw24=header.hex(), begin=hex(begin), end=hex(end), capacity=hex(capacity),
                width=width, element_limit=limit, ordered_pointers=begin <= end <= capacity,
                used_bytes=end - begin, used_bytes_divisible=(end - begin) % width == 0)
        return header, begin, (end - begin) // width

    def index_list(self, header):
        pointer, capacity = struct.unpack_from('<QI', header)
        size = struct.unpack_from('<I', header, 16)[0]
        if size > capacity or capacity > 128 or (size and not pointer):
            raise ValueError('invalid/out-of-profile IndexList')
        return dict(pointer=hex(pointer), size=size, capacity=capacity,
                    header=header.hex(), values_raw=self.read(pointer, size * 4).hex() if size else '')

    def current_game(self):
        game = self.uint(self.base + 0x3c051a8)
        if not game:
            raise ValueError('actual current game is null')
        return game

    @staticmethod
    def system_vector_header(raw, address, width, alignment, label):
        begin, end, capacity = struct.unpack('<QQQ', raw)
        ordered = begin <= end <= capacity
        aligned = all(pointer % alignment == 0 for pointer in (begin, end, capacity))
        divisible = ordered and (end - begin) % width == 0 and (capacity - begin) % width == 0
        null_consistent = bool(begin) or end == capacity == 0
        extent_fits_u64 = ordered and end - begin <= 0xffffffffffffffff - begin
        if not (ordered and aligned and divisible and null_consistent and extent_fits_u64):
            raise ObservationReadError('invalid PlotYields vector header', classification='invalid_header', label=label,
                address=hex(address), raw24=raw.hex(), begin=hex(begin), end=hex(end), capacity=hex(capacity),
                width=width, alignment=alignment, ordered=ordered, aligned=aligned, divisible=divisible,
                null_consistent=null_consistent, extent_fits_u64=extent_fits_u64)
        return dict(address=hex(address), raw24=raw.hex(), begin=hex(begin), end=hex(end), capacity=hex(capacity),
                    width=width, alignment=alignment, count=(end - begin) // width,
                    used_bytes=end - begin, capacity_bytes=capacity - begin)

    def system(self, address):
        raw = self.read(address, 88)
        handles = self.system_vector_header(raw[0x28:0x40], address + 0x28, 4, 4, 'PlotYields.ObjectHandles')
        objects = self.system_vector_header(raw[0x40:0x58], address + 0x40, 16, 8, 'PlotYields.Objects')
        byte_total = handles['used_bytes'] + objects['used_bytes']
        if byte_total > PLOT_YIELDS_CONTENT_BUDGET:
            raise ObservationReadError('PlotYields content byte budget exceeded', classification='budget_exceeded',
                system_address=hex(address), raw88=raw.hex(), handles_header=handles, objects_header=objects,
                aggregate_used_bytes=byte_total, checked_sum_exceeds_u64=byte_total > 0xffffffffffffffff,
                operational_byte_budget=PLOT_YIELDS_CONTENT_BUDGET, native_invariant_violation=False)
        ids_raw = self.read(int(handles['begin'], 16), handles['used_bytes']) if handles['used_bytes'] else b''
        objects_raw = self.read(int(objects['begin'], 16), objects['used_bytes']) if objects['used_bytes'] else b''
        ids = list(struct.unpack('<' + 'I' * handles['count'], ids_raw)) if ids_raw else []
        return dict(address=hex(address), raw88=raw.hex(), counter=int.from_bytes(raw[8:12], 'little'),
                    ids=ids, objects_raw=objects_raw.hex(), objects_count=objects['count'],
                    handles_header=handles, objects_header=objects, aggregate_used_bytes=byte_total,
                    operational_byte_budget=PLOT_YIELDS_CONTENT_BUDGET,
                    handles_objects_count_equal=handles['count'] == objects['count'])

    def current_system(self):
        game = self.current_game()
        effects = self.uint(game + 0x268)
        if not effects:
            raise ValueError('actual GameEffects is null')
        system = self.uint(effects + 0xa8)
        if not system:
            raise ValueError('actual PlotYields system is null')
        return system

    def routes(self, expected):
        game = self.current_game()
        entries, end = self.uint(game + 8), self.uint(game + 16)
        index = self.uint(self.base + 0x3c00510, 4)
        if end < entries or (end - entries) % 288 or index >= (end - entries) // 288:
            raise ValueError('invalid native game component index')
        manager = self.uint(entries + index * 288)
        table = manager + 0xc8 + 0xd0
        header = self.read(table, 32)
        capacity, elements, _ = struct.unpack_from('<III', header)
        hashes, values = struct.unpack_from('<QQ', header, 16)
        if not manager or capacity > 65536 or elements > 8192:
            raise ValueError('out-of-profile Trade ID-map')
        hash_bytes, pointer_bytes = self.read(hashes, capacity * 4), self.read(values, capacity * 8)
        rows, occupied = [], 0
        for slot in range(capacity):
            if struct.unpack_from('<I', hash_bytes, slot * 4)[0] == 0xffffffff:
                continue
            occupied += 1
            edge = struct.unpack_from('<Q', pointer_bytes, slot * 8)[0]
            if not edge:
                raise ValueError('occupied native Trade slot has null edge')
            if not self.uint(edge + 0x140, 1):
                continue
            raw = self.read(edge, 536)
            path_header, path_ptr, count = self.vector(edge + 0x188, 4, 65536, 'Trade.Edge.path edge=' + hex(edge))
            path_raw = self.read(path_ptr, count * 4) if count else b''
            row = dict(slot=slot, edge_pointer=hex(edge), edge_id=struct.unpack_from('<i', raw, 0xa8)[0],
                       route_id=struct.unpack_from('<i', raw, 0xc0)[0], origin=cid(raw[0xd0:0xd8]),
                       destination=cid(raw[0xd8:0xe0]), path=list(struct.unpack('<' + 'i' * count, path_raw)) if count else [],
                       raw536=raw.hex(), path_header=path_header.hex(), path_pointer=hex(path_ptr))
            match = expected.get(str(row['edge_id']))
            row['matches_parent_typed_edge'] = bool(match and all(row[k] == match[k] for k in
                                                       ['route_id', 'origin', 'destination', 'path']))
            row['nonempty_forward_path'] = bool(count)
            rows.append(row)
        if occupied != elements:
            raise ValueError('native ID-map occupancy count differs')
        return dict(manager=hex(manager), component_index=index, table_header=header.hex(),
                    hash_bytes=hash_bytes.hex(), pointer_bytes=pointer_bytes.hex(), occupied=occupied,
                    active_routes_in_native_order=rows, actual_lookup_index_consistency_observed=False)

    def monitor(self, receiver, expected):
        header = self.read(receiver, 48)
        _, plot_ptr, plots = self.vector(receiver, 32, 10000, 'YieldMonitor.PlotYields')
        _, trade_ptr, trades = self.vector(receiver + 24, 80, 1024, 'YieldMonitor.TradeYields')
        plot_raw, trade_raw = self.read(plot_ptr, plots * 32), self.read(trade_ptr, trades * 80)
        plot_rows = [self.index_list(plot_raw[i * 32:(i + 1) * 32]) for i in range(plots)]
        trade_rows = []
        for i in range(trades):
            raw = trade_raw[i * 80:(i + 1) * 80]
            trade_rows.append(dict(origin=cid(raw[:8]), destination=cid(raw[8:16]), raw80=raw.hex(),
                                   origin_yields=self.index_list(raw[16:48]), destination_yields=self.index_list(raw[48:80])))
        routes = self.routes(expected)
        native_keys = Counter(tuple(r['origin'] + r['destination']) for r in routes['active_routes_in_native_order'])
        cache_keys = Counter(tuple(r['origin'] + r['destination']) for r in trade_rows)
        return dict(receiver=hex(receiver), raw48=header.hex(), plot_headers_raw=plot_raw.hex(),
                    trade_headers_raw=trade_raw.hex(), plots=plot_rows, trades=trade_rows, native_trade=routes,
                    cache_membership_matches_live_active_routes=cache_keys == native_keys,
                    fixture_dimensions_match=(plots, trades) == (6996, 182))


def changes(before, after):
    result = dict(plot_count_before=len(before['plots']), plot_count_after=len(after['plots']),
                  trade_count_before=len(before['trades']), trade_count_after=len(after['trades']))
    for kind in ['plots', 'trades']:
        result[kind + '_changed_rows'] = [i for i in range(max(len(before[kind]), len(after[kind])))
                                        if i >= len(before[kind]) or i >= len(after[kind]) or before[kind][i] != after[kind][i]]
    return result


def expected_routes(path):
    import subprocess
    raw = Path(path).read_bytes()
    if hashlib.sha256(raw).hexdigest() != BEFORE_SHA:
        raise ValueError('requires accepted before-Core identity')
    codec = ROOT / 'crates/c6-core/src/trade.rs'
    code = '''#[path=CODEC] mod trade;
fn main(){let b=std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
let g=trade::Graph::decode(&b,trade::Limits::default()).unwrap();assert_eq!(g.encode(trade::Limits::default()).unwrap(),b);
for o in g.origins(){for d in &o.destinations{let e=&d.edge;if e.has_active_route{let r=&e.most_recent_route;
println!("{{\\\"edge_id\\\":{},\\\"route_id\\\":{},\\\"origin\\\":[{},{},{}],\\\"destination\\\":[{},{},{}],\\\"path\\\":{:?}}}",e.id,r.id,r.origin_city.owner,r.origin_city.component_type,r.origin_city.id,r.destination_city.owner,r.destination_city.component_type,r.destination_city.id,e.most_recent_path);}}}}
'''.replace('CODEC', json.dumps(str(codec)))
    with tempfile.TemporaryDirectory(prefix='c6-effects-prepare-') as temporary:
        directory = Path(temporary)
        (directory / 'main.rs').write_text(code)
        (directory / 'graph.bin').write_bytes(raw[71027800:71497438])
        subprocess.run(['rustc', '--edition=2021', '-O', '--cap-lints', 'allow', str(directory / 'main.rs'),
                        '-o', str(directory / 'inspect')], check=True)
        rows = [json.loads(line) for line in subprocess.check_output([str(directory / 'inspect'), str(directory / 'graph.bin')], text=True).splitlines()]
    if len(rows) != 182 or any(not r['path'] for r in rows):
        raise ValueError('accepted typed forward-path fixture differs')
    return {str(r['edge_id']): r for r in rows}


def prepare(args):
    sys.path.insert(0, str(ROOT / 'tools/linux'))
    from save_lifecycle_audit import Elf
    image = Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    Elf(args.assets / 'Civ6', HOST_SHA)
    for address, expected, name in GATES:
        if image.read(address, len(bytes.fromhex(expected))).hex() != expected:
            raise ValueError('original byte gate differs: ' + name)
    if sha(args.parent_runtime) != PARENT_SHA:
        raise ValueError('accepted parent runtime artifact differs')
    parent = json.loads(args.parent_runtime.read_text())
    if parent['returncode'] != 0:
        raise ValueError('parent native run failed')
    config = parent['configuration']
    encoded = json.dumps(config, sort_keys=True, separators=(',', ':')).encode()
    if hashlib.sha256(encoded).hexdigest() != CONFIG_SHA or parent['configuration_sha256'] != CONFIG_SHA:
        raise ValueError('parent configuration differs')
    preload = Path(parent['execution_paths']['preload'])
    if sha(preload) != PRELOAD_SHA or config['preload_sha256'] != PRELOAD_SHA:
        raise ValueError('archived preload differs')
    for path, expected in config['sources'].items():
        if sha(preload.parent / 'source' / path) != expected:
            raise ValueError('archived parent source differs: ' + path)
    routes = expected_routes(parent['execution_paths']['core_component']['before'])
    selected = []
    for line in parent['stderr'].splitlines():
        if line.startswith('C6_LINUX_CORE_COMPONENT '):
            row = json.loads(line.split(' ', 1)[1])
            if row.get('stage') == 'selected':
                selected.append(dict(owner=row['owner'], id=row['id']))
    if len(selected) != 1:
        raise ValueError('parent selected actor identity is ambiguous')
    metadata = dict(schema='c6-linux-reporting-effects-observer-prepared-v1', checks_passed=True, diagnostic_only_revision=False,
        plot_yields_content_budget_bytes=PLOT_YIELDS_CONTENT_BUDGET, plot_yields_budget_is_native_invariant=False,
        original_code_executed=False, native_capture_performed=False, source_sha256=sha(__file__),
        parent_runtime_sha256=sha(args.parent_runtime), parent_configuration_sha256=CONFIG_SHA,
        gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA, preload_sha256=PRELOAD_SHA,
        gates=[dict(rva=hex(a), bytes=b, name=n) for a, b, n in GATES],
        maximum_simultaneous_hardware_breakpoints=3, readiness_hit_cap=32,
        source_and_initializer_change=False, inferior_calls=False, inferior_writes=False, software_game_breakpoints=False,
        expected_typed_active_routes=len(routes), before_core_sha256=BEFORE_SHA,
        expected_routes_sha256=hashlib.sha256(json.dumps(routes, sort_keys=True, separators=(',', ':')).encode()).hexdigest(),
        codec_sha256=sha(ROOT / 'crates/c6-core/src/trade.rs'), expected_actor=selected[0],
        tracer_fields=dict(armed_offset='0x27500', armed_bytes=1, phase_offset='0x27514', phase_bytes=4),
        hypothesis='Read actual old-empty AWAKE HasAdjacentMove result and monitor cache/counter effects. Null contribution statically skips direct Bonus/Add/Remove path; no Add/Remove runtime census.',
        scope='Armed phase5 Unit readiness provenance plus original monitor entry through matching native caller return with same thread and exact stack return.',
        limits=['No global purity, complete effects/RNG or full action acceptance',
                'Counter raw delta is not an Add count; no Add/Remove runtime coverage is supplied and complete counter-writer coverage is unproven',
                'Native route lookup-index consistency remains unobserved', 'Original helper deadline must remain unchanged'])
    if args.config_out:
        cfg = dict(metadata=metadata, preload=str(preload), expected_routes=routes, expected_actor=selected[0],
                   output_directory=str(args.raw_output.resolve()))
        args.config_out.write_text(json.dumps(cfg, indent=2) + '\n')
    args.output.write_text(json.dumps(metadata, indent=2, sort_keys=True) + '\n')


def install_gdb(config_path):
    import gdb
    cfg = json.loads(Path(config_path).read_text())
    if cfg['metadata']['source_sha256'] != sha(__file__):
        raise ValueError('observer source differs from frozen preparation')
    if (cfg['metadata']['parent_configuration_sha256'] != CONFIG_SHA or cfg['metadata']['preload_sha256'] != PRELOAD_SHA or
            cfg['metadata']['parent_runtime_sha256'] != PARENT_SHA):
        raise ValueError('observer parent/preload identity differs')
    encoded_routes = json.dumps(cfg['expected_routes'], sort_keys=True, separators=(',', ':')).encode()
    if hashlib.sha256(encoded_routes).hexdigest() != cfg['metadata']['expected_routes_sha256'] or cfg['expected_actor'] != cfg['metadata']['expected_actor']:
        raise ValueError('observer typed fixture identity differs')
    output = Path(cfg['output_directory'])
    if not output.is_dir():
        raise ValueError('driver must create fresh raw output directory')
    log = (output / 'observations.jsonl').open('x')
    state = dict(base=None, preload=None, installed=False, interval=None, sequence=0, intervals=0, readiness_hits=0, readiness_capped=False)
    def mem(p, n):
        return gdb.selected_inferior().read_memory(p, n).tobytes() if n else b''
    def reg(name):
        return int(gdb.parse_and_eval('$' + name))
    def emit(row):
        state['sequence'] += 1
        row['sequence'] = state['sequence']
        log.write(json.dumps(row, sort_keys=True) + '\n')
        log.flush()
    def phase():
        p = state['preload']
        return (bool(mem(p + 0x27500, 1)[0]), int.from_bytes(mem(p + 0x27514, 4), 'little'))
    class Point(gdb.Breakpoint):
        def __init__(self, address, kind, expected):
            self.kind, self.address, self.expected = kind, address, bytes.fromhex(expected)
            super().__init__('*' + hex(address), type=gdb.BP_HARDWARE_BREAKPOINT, internal=False)
        def stop(self):
            try:
                if mem(self.address, len(self.expected)) != self.expected:
                    raise ValueError('live checkpoint bytes differ: ' + self.kind)
                return hit(self)
            except Exception as error:
                emit(dict(stage='observer_error', checkpoint=self.kind, error=str(error), details=getattr(error, 'context', None), interval_open=bool(state['interval'])))
                return True
    def hit(point):
        decoder = Decoder(mem, state['base'])
        interval = state['interval']
        tid = list(gdb.selected_thread().ptid)
        if point.kind == 'monitor_entry':
            armed, current_phase = phase()
            if not armed or current_phase != 5:
                emit(dict(stage='monitor_outside_selected_scope', trace_armed=armed, trace_phase=current_phase))
                return False
            if interval:
                raise ValueError('nested monitor interval unsupported')
            sp, receiver = reg('rsp'), reg('rdi')
            caller = decoder.uint(sp)
            if caller != state['base'] + 0x272361e:
                raise ValueError('monitor caller differs from native cache delegate return')
            # Preserve bounded context before any deep decode. This is a prefix,
            # never a completed interval or a valid before/after observation.
            emit(dict(stage='monitor_entry_prefix', thread=tid, receiver=hex(receiver), raw48=mem(receiver, 48).hex(),
                      rsp=hex(sp), caller=hex(caller)))
            bindings = dict(stage='monitor_binding_diagnostics', thread=tid, receiver=hex(receiver))
            def probe(label, address, width):
                try:
                    raw = mem(address, width)
                    value = int.from_bytes(raw, 'little')
                    bindings[label] = dict(address=hex(address), width=width, raw=raw.hex(), value=hex(value) if width == 8 else value)
                    return value
                except Exception as error:
                    bindings[label] = dict(address=hex(address), width=width, read_error=str(error))
                    return None
            probe('local_game_global', state['base'] + 0x3c051a8, 8)
            game_slot = probe('target_GetGame_got_slot', state['base'] + 0x3b0ef18, 8)
            if game_slot:
                probe('target_GetGame_via_got', game_slot, 8)
            probe('local_trade_component_index', state['base'] + 0x3c00510, 4)
            index_slot = probe('target_TradeManager_index_got_slot', state['base'] + 0x3b09068, 8)
            if index_slot:
                probe('target_TradeManager_index_via_got', index_slot, 4)
            emit(bindings)
            snapshot = decoder.monitor(receiver, cfg['expected_routes'])
            state['intervals'] += 1
            before_name = 'monitor-%04d-before.json' % state['intervals']
            (output / before_name).write_text(json.dumps(snapshot) + '\n')
            emit(dict(stage='monitor_primary_snapshot', interval=state['intervals'], snapshot_file=before_name,
                      secondary_system_capture_pending=True, paired_interval_completed=False))
            system = decoder.current_system()
            interval = dict(number=state['intervals'], thread=tid, entry_rsp=sp, receiver=receiver, before=snapshot,
                            systems={system: decoder.system(system)}, ignored_return_hits=0)
            state['interval'] = interval
            interval['return_point'] = Point(caller, 'monitor_return', '4883c3304c89ff')
            emit(dict(stage='monitor_entry', interval=interval['number'], snapshot_file=before_name, thread=tid, rsp=hex(sp), receiver=hex(receiver),
                      trace_armed=armed, trace_phase=current_phase, systems=list(interval['systems'].values()),
                      matching_return=hex(caller), fixture_dimensions_match=snapshot['fixture_dimensions_match']))
        elif point.kind == 'monitor_return':
            if not interval:
                raise ValueError('return without active monitor interval')
            if tid != interval['thread'] or reg('rsp') != interval['entry_rsp'] + 8:
                interval['ignored_return_hits'] += 1
                return False
            after = decoder.monitor(interval['receiver'], cfg['expected_routes'])
            after_name = 'monitor-%04d-after.json' % interval['number']
            (output / after_name).write_text(json.dumps(after) + '\n')
            emit(dict(stage='monitor_primary_snapshot', interval=interval['number'], snapshot_file=after_name,
                      secondary_system_capture_pending=True, paired_interval_completed=False))
            systems = []
            current_address = decoder.current_system()
            current = decoder.system(current_address)
            for address, before in interval['systems'].items():
                same = address == current_address
                systems.append(dict(before=before, after=current, same_live_system_identity=same,
                                    raw_counter_delta_u32=((current['counter'] - before['counter']) & 0xffffffff) if same else None,
                                    delta_is_add_call_count=False, old_address_reread_when_identity_changed=False))
            emit(dict(stage='monitor_return', interval=interval['number'], snapshot_file=after_name, thread=tid, changes=changes(interval['before'], after), systems=systems,
                      ignored_return_pc_hits=interval['ignored_return_hits'], readiness_hits=state['readiness_hits'],
                      readiness_coverage_capped=state['readiness_capped'], trace_armed=phase()[0], trace_phase=phase()[1],
                      add_remove_runtime_coverage=False))
            point.delete()
            state['interval'] = None
        elif point.kind == 'has_adjacent_move_return':
            armed, current_phase = phase()
            if not armed or current_phase != 5:
                return False
            unit = reg('rbx')
            queue = mem(unit + 0xc10, 200)
            trace = gdb.execute('bt 14', to_string=True)
            owner = int.from_bytes(mem(unit + 0x130, 4), 'little', signed=True)
            identity = decoder.uint(unit + 0xb8, 4)
            state['readiness_hits'] += 1
            frames = []
            frame = gdb.newest_frame()
            while frame is not None and len(frames) < 20:
                frames.append(dict(pc=hex(frame.pc()), name=frame.name()))
                frame = frame.older()
            frame_pcs = [int(row['pc'], 16) - state['base'] for row in frames]
            provenance = (any(0x2c280f0 <= pc < 0x2c2817a for pc in frame_pcs) and
                          any(0x2c27ff0 <= pc < 0x2c28019 for pc in frame_pcs) and
                          any(0x2805100 <= pc < 0x280552c for pc in frame_pcs))
            actual = reg('al') & 0xff
            emit(dict(stage='has_adjacent_move_return', thread=tid, unit=hex(unit), owner=owner, id=identity,
                      matches_parent_actor=dict(owner=owner, id=identity) == cfg['expected_actor'],
                      location=list(struct.unpack('<ii', mem(unit + 0x100, 8))),
                      activity=hex(decoder.uint(unit + 0x290, 4)), needs_attention=decoder.uint(unit + 0x738, 1),
                      actual_return_al=actual, has_adjacent_move=bool(actual), queue_raw200=queue.hex(),
                      queue_size=int.from_bytes(queue[0xc0:0xc4], 'little'), trace_armed=armed, trace_phase=current_phase,
                      backtrace=trace, frames=frames, required_call_chain_seen=provenance,
                      exact_old_empty_awake_branch=(dict(owner=owner, id=identity) == cfg['expected_actor'] and
                          decoder.uint(unit + 0x290, 4) == 0x6b25012e and decoder.uint(unit + 0x738, 1) == 0 and
                          int.from_bytes(queue[0xc0:0xc4], 'little') == 0 and provenance),
                      hold_cycling_fallbacks_skipped=bool(actual),
                      fallback_skip_basis='Exact next conditional branch; no separate post-jump observation'))
            if state['readiness_hits'] >= 32:
                point.enabled = False
                state['readiness_capped'] = True
                emit(dict(stage='readiness_coverage_disabled', reason='32-hit bound', observed_hits=32))
        return False
    def install(event=None):
        if state['installed']:
            return
        mappings = []
        for line in Path('/proc/%d/maps' % gdb.selected_inferior().pid).read_text().splitlines():
            fields = line.split(maxsplit=5)
            if len(fields) == 6 and int(fields[2], 16) == 0:
                mappings.append((int(fields[0].split('-')[0], 16), fields[5]))
        for address, path in mappings:
            if Path(path).resolve() == Path(cfg['preload']).resolve():
                if sha(path) != PRELOAD_SHA:
                    raise ValueError('live preload file differs')
                state['preload'] = address
            if path.endswith('/libGameCore_XP2.so'):
                if sha(path) != GAME_SHA:
                    raise ValueError('live XP2 file differs')
                state['base'] = address
        if state['base'] is None or state['preload'] is None:
            return
        for rva, expected, _ in GATES:
            if mem(state['base'] + rva, len(bytes.fromhex(expected))) != bytes.fromhex(expected):
                raise ValueError('live module byte gate differs')
        Point(state['base'] + 0x2c66430, 'monitor_entry', '534889fb')
        Point(state['base'] + 0x2c28137, 'has_adjacent_move_return', '84c07518')
        state['installed'] = True
        emit(dict(stage='hardware_installed', game_base=hex(state['base']), preload_base=hex(state['preload']),
                  gamecore_sha256=GAME_SHA, preload_sha256=PRELOAD_SHA, source_sha256=sha(__file__), gates_verified=len(GATES),
                  persistent_points=2, maximum_points_with_matching_return=3))
    def exited(event):
        emit(dict(stage='inferior_exit', hardware_installed=state['installed'],
                  interval_still_open=bool(state['interval']), readiness_hits=state['readiness_hits'],
                  readiness_coverage_capped=state['readiness_capped'], exit_code=getattr(event, 'exit_code', None)))
        log.close()
    gdb.events.new_objfile.connect(install)
    gdb.events.exited.connect(exited)
    install()


def self_test():
    memory = bytearray(512)
    struct.pack_into('<QI', memory, 32, 128, 4)
    struct.pack_into('<I', memory, 48, 3)
    struct.pack_into('<iii', memory, 128, -1, 0, 123)
    decoder = Decoder(lambda p, n: bytes(memory[p:p+n]), 0)
    row = decoder.index_list(bytes(memory[32:64]))
    assert row['size'] == 3 and bytes.fromhex(row['values_raw']) == struct.pack('<iii', -1, 0, 123)
    struct.pack_into('<I', memory, 48, 5)
    try:
        decoder.index_list(bytes(memory[32:64]))
    except ValueError:
        pass
    else:
        raise AssertionError('invalid IndexList accepted')
    struct.pack_into('<QQQ', memory, 200, 300, 320, 310)
    try:
        decoder.vector(200, 4, 16, 'offline.bad_vector')
    except ObservationReadError as error:
        assert error.context['label'] == 'offline.bad_vector'
        assert error.context['raw24'] == bytes(memory[200:224]).hex()
        assert error.context['address'] == hex(200) and not error.context['ordered_pointers']
    else:
        raise AssertionError('invalid vector accepted')
    assert cid(struct.pack('<hhI', -1, 241, 0xffffffff)) == [-1, 241, 0xffffffff]
    assert changes(dict(plots=[1, 2], trades=[]), dict(plots=[1, 3], trades=[]))['plots_changed_rows'] == [1]

    sparse = {}
    def put(address, raw):
        sparse.update((address + i, value) for i, value in enumerate(raw))
    def number(address, value, size=8):
        put(address, value.to_bytes(size, 'little'))
    number(0x3c051a8, 0x1000)
    number(0x3c00510, 0, 4)
    number(0x1008, 0x2000)
    number(0x1010, 0x2120)
    number(0x2000, 0x3000)
    put(0x3198, struct.pack('<IIIIQQ', 2, 1, 0, 0, 0x4000, 0x4100))
    put(0x4000, struct.pack('<II', 0xffffffff, 0))
    put(0x4100, struct.pack('<QQ', 0, 0x5000))
    edge = bytearray(536)
    struct.pack_into('<i', edge, 0xa8, 17)
    struct.pack_into('<i', edge, 0xc0, 18)
    struct.pack_into('<hhI', edge, 0xd0, 0, 1, 2)
    struct.pack_into('<hhI', edge, 0xd8, 1, 1, 3)
    edge[0x140] = 1
    struct.pack_into('<QQQ', edge, 0x188, 0x7000, 0x7008, 0x7008)
    put(0x5000, edge)
    put(0x7000, struct.pack('<ii', 4, 5))
    fake = Decoder(lambda p, n: bytes(sparse.get(p + i, 0) for i in range(n)), 0)
    rows = fake.routes({'17': dict(route_id=18, origin=[0, 1, 2], destination=[1, 1, 3], path=[4, 5])})
    assert rows['occupied'] == 1
    assert rows['active_routes_in_native_order'][0]['slot'] == 1
    assert rows['active_routes_in_native_order'][0]['matches_parent_typed_edge']

    real_header = bytes.fromhex('400fbc0c00000000dc61bc0c00000000408fbc0c00000000')
    real = Decoder.system_vector_header(real_header, 0x38082228, 4, 4, 'real.PlotYields.ObjectHandles')
    assert (real['count'], real['used_bytes'], real['capacity_bytes']) == (5287, 21148, 32768)
    # Reproduce the old resource-cap rejection without calling it a native-invalid header.
    assert real['count'] > 4096
    raw = bytearray(88)
    raw[0x28:0x40] = real_header
    struct.pack_into('<QQQ', raw, 0x40, 0x100000, 0x100000 + 5287 * 16, 0x100000 + 8192 * 16)
    reads = []
    def used_only(address, size):
        reads.append((address, size))
        if address == 0x100 and size == 88:
            return bytes(raw)
        return bytes(size)
    system = Decoder(used_only, 0).system(0x100)
    assert system['aggregate_used_bytes'] == 105740
    assert reads == [(0x100, 88), (0xcbc0f40, 21148), (0x100000, 84592)]
    boundary = bytearray(88)
    half = PLOT_YIELDS_CONTENT_BUDGET // 2
    struct.pack_into('<QQQ', boundary, 0x28, 0x200000, 0x200000 + half, 0x200000 + half)
    struct.pack_into('<QQQ', boundary, 0x40, 0x400000, 0x400000 + half, 0x400000 + half)
    boundary_reads = []
    def boundary_memory(address, size):
        boundary_reads.append((address, size))
        return bytes(boundary) if (address, size) == (0x100, 88) else bytes(size)
    exact = Decoder(boundary_memory, 0).system(0x100)
    assert exact['aggregate_used_bytes'] == PLOT_YIELDS_CONTENT_BUDGET
    assert boundary_reads == [(0x100, 88), (0x200000, half), (0x400000, half)]
    oversized = bytearray(88)
    struct.pack_into('<QQQ', oversized, 0x28, 0x200000, 0x200000 + PLOT_YIELDS_CONTENT_BUDGET + 4,
                     0x200000 + PLOT_YIELDS_CONTENT_BUDGET + 4)
    def headers_only(address, size):
        assert (address, size) == (0x100, 88), 'payload read before aggregate budget rejection'
        return bytes(oversized)
    try:
        Decoder(headers_only, 0).system(0x100)
    except ObservationReadError as error:
        assert error.context['classification'] == 'budget_exceeded'
        assert error.context['native_invariant_violation'] is False
        assert error.context['raw88'] == oversized.hex()
    else:
        raise AssertionError('oversized used-byte total accepted')
    for malformed in [struct.pack('<QQQ', 0x1001, 0x1011, 0x1021),
                      struct.pack('<QQQ', 0xfffffffffffffff0, 0x10, 0x20)]:
        try:
            Decoder.system_vector_header(malformed, 0x100, 4, 4, 'offline.malformed')
        except ObservationReadError as error:
            assert error.context['classification'] == 'invalid_header'
        else:
            raise AssertionError('misaligned/wrapped vector header accepted')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--parent-runtime', type=Path, default=Path('/tmp/c6-core-component-event-execute-1.json'))
    parser.add_argument('--output', type=Path)
    parser.add_argument('--config-out', type=Path)
    parser.add_argument('--raw-output', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        self_test()
    if args.output:
        if args.config_out and not args.raw_output:
            parser.error('--config-out requires --raw-output')
        prepare(args)
    elif not args.self_test:
        parser.error('--output or --self-test required')


if 'C6_EFFECTS_CONFIG_PATH' in globals():
    install_gdb(C6_EFFECTS_CONFIG_PATH)
elif __name__ == '__main__':
    main()
