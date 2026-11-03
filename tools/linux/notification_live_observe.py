#!/usr/bin/env python3
"""Prepare/source a passive GDB notification observer; never launch an inferior.

CLI prepares a private manifest from an already source-gated runtime report.
GDB usage: runpy.run_path(FILE, init_globals={'C6_NOTIFICATION_MANIFEST': PATH}).
Observations are private JSONL, not gameplay acceptance or a replacement oracle.
"""
import argparse
import hashlib
import json
from pathlib import Path

GAME_SHA = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
HOST_SHA = '106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
CONFIG = 'b6ca58d53073abc2afa9176a4ceba350fafe3f91ba12dbf84fd08e25a3be9a5b'
ROOT = Path(__file__).resolve().parents[2]
PROFILES = {
    'accepted-skip-turn': ['update_entry', 'find_entry', 'auto_entry', 'release_call'],
}
# Exact target instruction boundaries; all bytes are verified offline and live.
SITES = {
    'update_entry': (0x2d2f360, '555350'),
    'update_return': (0x2b5bdd9, 'e832a142ff'),
    'find_entry': (0x2d30ec0, '554157'),
    'selected_return': (0x2d2f618, '4889c3'),
    'activate_call': (0x2d2f608, 'ff9088000000'),
    'release_call': (0x2d31252, 'ff5020'),
    'release_return': (0x2d31255, '4c89ff'),
    'cloud': (0x2d2f37e, '84c0'),
    'local': (0x2d2f390, '39e8'),
    'auto_entry': (0x2d2e1f0, '555350'),
    'auto_network': (0x2d2e20f, '84c0'),
    'definition': (0x2d2e223, '4885c0'),
    'icon': (0x2d2e230, '84c0'),
    'auto_exit': (0x2d2e23f, '89e8'),
    'severity': (0x2d30f6c, '84db'),
}
SLOTS = {'deleting_destructor': 8, 'release': 0x20, 'is_expired': 0x28,
         'activate': 0x88, 'dismiss': 0x98}
GETTERS = {
    'game_local_player': (0x2661af0, '488bbf48020000e994678dff'),
    'player_manager_local_player': (0x2667920, '8b87681c0000c3'),
    'game_uninitializing': (0x2167500, '8a87ae020000c3'),
    'definition_icon_bit': (0x2d2e8b0, '8a87900000002410c0e804c3'),
}


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def require(value, message):
    if not value:
        raise ValueError(message)


def prepare(args):
    # Imports only in normal Python: GDB does not need ELF parser dependencies.
    from save_lifecycle_audit import Elf
    runtime = json.loads(args.runtime_report.read_text())
    cfg = runtime['configuration']
    canonical = json.dumps(cfg, sort_keys=True, separators=(',', ':')).encode()
    require(hashlib.sha256(canonical).hexdigest() == runtime['configuration_sha256'], 'configuration gate')
    require(runtime['configuration_sha256'] == CONFIG, 'only frozen accepted SkipTurn profile supported')
    require(args.phase == [5], 'only original worker phase 5 is in scope')
    require(runtime['returncode'] == 0 and cfg['core_component_mode'] == 'execute', 'closed execution required')
    require(cfg['prepare_main_thread'] and not cfg['pre_controller_registration'], 'startup profile differs')
    require(cfg['lower_core_cache_dispatch_observer']['reporting_observer_profile'] == 'proven_skipturn_callers_v2', 'corrected observer required')
    preload = Path(runtime['execution_paths']['preload']).resolve()
    require(digest(preload) == cfg['preload_sha256'], 'preload gate')
    import subprocess
    symbols = {}
    for line in subprocess.check_output(['nm', '-an', str(preload)], text=True).splitlines():
        fields = line.split()
        if len(fields) == 3 and fields[2] in ('armed', 'phase'):
            symbols[fields[2]] = int(fields[0], 16)
    require(symbols == {'armed': args.armed_rva, 'phase': args.phase_rva}, 'preload observation symbols differ')
    for name, expected in cfg['sources'].items():
        require(digest(preload.parent / 'source' / name) == expected, 'archived source gate: ' + name)
    contract_path = ROOT / 'evidence/linux/notification-lifecycle.json'
    contract = json.loads(contract_path.read_text())
    require(contract['checks_passed'] and contract['gamecore_sha256'] == GAME_SHA, 'lifecycle contract gate')
    elf = Elf(args.module, GAME_SHA)
    require(digest(args.host) == HOST_SHA, 'host gate')
    for name, (rva, expected) in {**SITES, **GETTERS}.items():
        require(elf.read(rva, len(bytes.fromhex(expected))).hex() == expected, 'instruction gate: ' + name)
    for row in contract['virtual_bindings']:
        table = int(row['vtable_rva'], 16) + 16
        for slot, offset in SLOTS.items():
            require(elf.relocated_pointer(table + offset) == int(row['slots'][slot], 16), 'vtable gate')
    # Manifest intentionally has no inferred current values or accepted result.
    manifest = dict(schema='c6-linux-notification-passive-plan-v1', profile=args.profile,
                    runtime_report=str(args.runtime_report.resolve()), runtime_report_sha256=digest(args.runtime_report),
                    configuration_sha256=runtime['configuration_sha256'], sources=cfg['sources'],
                    module=str(args.module.resolve()), module_sha256=GAME_SHA,
                    host=str(args.host.resolve()), host_sha256=HOST_SHA,
                    preload=str(preload), preload_sha256=digest(preload),
                    observer=str(Path(__file__).resolve()), observer_sha256=digest(__file__),
                    lifecycle_contract_sha256=digest(contract_path), virtual_bindings=contract['virtual_bindings'],
                    armed_rva=args.armed_rva, phase_rva=args.phase_rva, phases=args.phase,
                    output=str(args.observations.resolve()), max_records=4096, max_hits=10000,
                    native_execution_performed=False, gameplay_parity=False)
    args.output.write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'prepared': True, 'profile': args.profile, 'native_execution_performed': False,
                      'manifest_sha256': digest(args.output), 'observer_sha256': digest(__file__)}))


def install(manifest_path):
    import gdb
    require(not gdb.parameter('non-stop'), 'all-stop debugger mode required for coherent reads')
    m = json.loads(Path(manifest_path).read_text())
    require(m['schema'] == 'c6-linux-notification-passive-plan-v1', 'manifest schema')
    require(digest(__file__) == m['observer_sha256'], 'observer changed')
    require(digest(m['runtime_report']) == m['runtime_report_sha256'], 'runtime report changed')
    for name in ('module', 'host', 'preload'):
        require(digest(m[name]) == m[name + '_sha256'], name + ' changed')
    output = Path(m['output'])
    require(not output.exists(), 'refuse to append to existing observation stream')
    state = dict(sequence=0, base=None, thread=None, handles={}, lanes=[], pending=None, failed=False,
                 find_active=False)

    def mappings():
        rows = []
        pid = gdb.selected_inferior().pid
        for line in Path('/proc/%d/maps' % pid).read_text().splitlines():
            f = line.split(maxsplit=5)
            a, b = (int(x, 16) for x in f[0].split('-'))
            rows.append((a, b, f[1], int(f[2], 16), f[5] if len(f) > 5 else ''))
        return rows

    def read(p, n):
        require(p > 0 and 0 < n <= 1 << 20, 'invalid read bounds')
        maps = state.get('maps') or mappings()
        require(any(a <= p and p + n <= b and 'r' in perms for a, b, perms, _, _ in maps), 'unreadable range')
        return gdb.selected_inferior().read_memory(p, n).tobytes()

    def num(p, n=8, signed=False):
        return int.from_bytes(read(p, n), 'little', signed=signed)

    def reg(name):
        return int(gdb.parse_and_eval('$' + name))

    def signed32(value):
        value &= 0xffffffff
        return value - (1 << 32) if value & (1 << 31) else value

    def boolean(p):
        v = num(p, 1)
        require(v in (0, 1), 'noncanonical bool')
        return bool(v)

    def returned_bool():
        value = reg('al') & 255
        require(value in (0, 1), 'noncanonical returned bool')
        return bool(value)

    def handle(p):
        if not p:
            return None
        return state['handles'].setdefault(p, 'object-' + str(len(state['handles']) + 1))

    def vector(p):
        begin, end, cap = (num(p + i * 8) for i in range(3))
        require(begin <= end <= cap and (end - begin) % 8 == 0, 'vector bounds')
        require((cap - begin) // 8 <= m['max_records'], 'vector capacity limit')
        return [num(i) for i in range(begin, end, 8)]

    def instance(p, owner=None):
        require(p % 8 == 0, 'unaligned notification')
        raw = read(p, 416)
        table = num(p)
        match = next((r for r in m['virtual_bindings'] if state['base'] + int(r['vtable_rva'], 16) + 16 == table), None)
        require(match is not None, 'unrecognized notification vtable')
        slots = {}
        for name, offset in SLOTS.items():
            target = num(table + offset)
            require(target == state['base'] + int(match['slots'][name], 16), 'live virtual target mismatch')
            slots[name] = hex(target - state['base'])
        identity = int.from_bytes(raw[8:16], 'little')
        actual_owner = int.from_bytes(raw[8:10], 'little', signed=True)
        require(owner is None or owner == actual_owner, 'notification owner mismatch')
        require(int.from_bytes(raw[10:12], 'little', signed=True) == 38, 'not notification ComponentID')
        parent = num(p + 0x138)
        return dict(handle=handle(p), identity_bits=f'{identity:016x}', owner=actual_owner,
                    type_hash=hex(num(p + 0x10, 4)), severity=num(p + 0x14, 4, True),
                    lock_count=num(p + 0x194, 4, True), default_blocking=num(p + 0x130, 4, True),
                    dismissed=boolean(p + 0x198), native_is_expired=boolean(p + 0x19c),
                    native_auto_activate=boolean(p + 0x19d),
                    activated=boolean(p + 0x19e), slots=slots,
                    pool=handle(parent), pool_order=[handle(v) for v in vector(parent)] if parent else None)

    def listing(p):
        owner = num(p, 4, True)
        sentinel = p + 8
        node, previous, seen, records = num(sentinel), sentinel, set(), []
        while node != sentinel:
            require(node not in seen and len(seen) < m['max_records'], 'list cycle/count bound')
            require(num(node + 8) == previous, 'list backlink mismatch')
            seen.add(node)
            obj = num(node + 16)
            records.append(instance(obj, owner) if obj else None)
            previous, node = node, num(node)
        require(num(sentinel + 8) == previous, 'list tail mismatch')
        return dict(owner=owner, next_id=num(p + 4, 4), records=records)

    def manager(p, owner):
        require(num(p) in [state['base'] + x + 16 for x in (0x3af62f0, 0x3afb650)], 'manager vtable')
        players = vector(p + 0x18)
        require(0 <= owner < len(players) and players[owner] != 0, 'missing owner list')
        game = num(p + 8)
        return dict(player_count=len(players), game_present=game != 0, selected=listing(players[owner]),
                    game_uninitializing=boolean(game + 0x2ae) if game else None,
                    local_player=num(num(game + 0x248) + 0x1c68, 4, True) if game else None,
                    getter_provenance='pinned pure Game and PlayerManager getter bodies; branch-time memory reads')

    def record(row):
        state['sequence'] += 1
        require(state['sequence'] <= m['max_hits'], 'hit limit')
        row.update(sequence=state['sequence'], thread=gdb.selected_thread().ptid)
        with output.open('a') as f:
            f.write(json.dumps(row, sort_keys=True) + '\n')

    def active():
        bias = next((a for a, _, _, off, path in state['maps'] if path == m['preload'] and off == 0), None)
        require(bias is not None, 'source-gated preload not mapped')
        armed, phase = boolean(bias + m['armed_rva']), num(bias + m['phase_rva'], 4)
        state['trace_armed'], state['trace_phase'] = armed, phase
        return armed and phase in m['phases']

    class Lane(gdb.Breakpoint):
        def __init__(self, name, previous=None, address=None):
            self.name = name
            self.context = previous.context if previous else {}
            self.address = state['base'] + SITES[name][0]
            require(address is None or address == self.address, 'unexpected original return address')
            self.expected = bytes.fromhex(SITES[name][1])
            require(read(self.address, len(self.expected)) == self.expected, 'live site bytes')
            if previous:
                previous.delete()
            super().__init__('*' + hex(self.address), type=gdb.BP_HARDWARE_BREAKPOINT, internal=False)

        def move(self, name, address=None):
            # Rotation happens after stop() unwinds, in the drain command.
            require(state['pending'] is None, 'overlapping breakpoint rotation')
            state['pending'] = (self, name, address)

        def stop(self):
            try:
                state['maps'] = mappings()
                if not active():
                    return False
                thread = gdb.selected_thread().ptid
                if state['thread'] is None:
                    state['thread'] = thread
                require(thread == state['thread'], 'concurrent notification execution unsupported')
                require(read(self.address, len(self.expected)) == self.expected, 'checkpoint bytes changed')
                row = dict(stage=self.name, pc=hex(reg('pc') - state['base']))
                row.update(trace_armed=state['trace_armed'], trace_phase=state['trace_phase'])
                self.observe(row)
                record(row)
                return True
            except Exception as error:
                state['failed'] = True
                record(dict(stage=self.name, observation_error=str(error), capture_complete=False))
                return True  # Leave inferior stopped; never synthesize missing data.

        def observe(self, row):
            n, c = self.name, self.context
            if n == 'update_entry':
                c.update(manager=reg('rdi'), owner=signed32(reg('esi')), return_pc=num(reg('rsp')))
                row['manager'] = manager(c['manager'], c['owner'])
                require(c['return_pc'] == state['base'] + SITES['update_return'][0], 'unexpected Update caller')
                self.move('cloud')
            elif n == 'update_return':
                row['manager'] = manager(c['manager'], c['owner'])
                row['boundary'] = 'original Update including tail Cleanup returned'
                self.move('update_entry')
            elif n == 'find_entry':
                # Restrict entry provenance to the proven ProcessAutoActivate caller.
                require(num(reg('rsp')) == state['base'] + 0x2d2f618, 'unexpected Find caller')
                row.update(list=listing(reg('rdi')), threshold=signed32(reg('esi')),
                           entering_branch_gates=dict(accepts_notifications=True, owner_turn_active=True,
                                                      initialized=True, game_uninitializing=False, human_configuration=True,
                                                      provenance='original ProcessAutoActivate passed branches 2d2f5c2 and 2d2f5dd'))
                require(row['threshold'] in (0, 3), 'unsupported severity threshold')
                row['network_multiplayer'] = row['threshold'] == 3
                state.update(find_active=True, find_threshold=row['threshold'],
                             find_handles={r['handle'] for r in row['list']['records'] if r is not None})
                self.move('selected_return')
            elif n == 'selected_return':
                require(state['find_active'], 'Find return without observed entry')
                state['find_active'] = False
                p = reg('rax')
                row['selected'] = instance(p) if p else None
                if p and row['selected']['lock_count'] <= 0:
                    self.move('activate_call')
                else:
                    self.move('find_entry')
            elif n == 'activate_call':
                row.update(selected=instance(reg('rdi')), activation_argument=signed32(reg('esi')))
                require(num(reg('rax') + 0x88) == state['base'] + int(row['selected']['slots']['activate'], 16), 'actual call target mismatch')
                self.move('find_entry')
            elif n == 'release_call':
                c['pool'] = num(reg('rdi') + 0x138)
                c['released'] = reg('rdi')
                row['released'] = instance(reg('rdi'))
                self.move('release_return')
            elif n == 'release_return':
                # Do not dereference the potentially destroyed notification.
                row['pool_order'] = [handle(v) for v in vector(c['pool'])] if c['pool'] else None
                if c['pool']:
                    row['released_pool_link_after'] = handle(num(c['released'] + 0x138))
                self.move('release_call')
            elif n == 'cloud':
                row.update(play_by_cloud=returned_bool(), owner=signed32(reg('ebp')), game_present=num(reg('rbx') + 8) != 0)
                if row['play_by_cloud'] and row['game_present']:
                    self.move('local')
                else:
                    self.move('update_return', c['return_pc'])
            elif n == 'local':
                row.update(local_player=signed32(reg('eax')), owner=signed32(reg('ebp')))
                self.move('update_return', c['return_pc'])
            elif n == 'auto_entry':
                c['instance'] = reg('rdi')
                require(state['find_active'] and num(reg('rsp')) == state['base'] + 0x2d30f38, 'unexpected IsAutoActivate caller')
                require(handle(c['instance']) in state['find_handles'], 'candidate absent from observed Find list')
                row['instance'] = instance(c['instance'])
                self.move('auto_exit' if row['instance']['activated'] else 'auto_network')
            elif n == 'auto_network':
                row.update(instance=handle(c['instance']), network=returned_bool())
                self.move('auto_exit' if row['network'] else 'definition')
            elif n == 'definition':
                definition = reg('rax')
                row.update(instance=handle(c['instance']), definition_found=definition != 0)
                if definition:
                    require(num(definition + 24, 4) == num(c['instance'] + 0x10, 4), 'definition type mismatch')
                    row.update(definition=handle(definition), icon_flag_bit=bool(num(definition + 144, 1) & 16))
                self.move('icon' if definition else 'auto_exit')
            elif n == 'icon':
                row.update(instance=handle(c['instance']), show_icon_single_player=returned_bool())
                self.move('auto_exit')
            elif n == 'auto_exit':
                row.update(instance=handle(c['instance']), is_auto_activate=bool(reg('bpl') & 255))
                self.move('severity' if row['is_auto_activate'] else 'auto_entry')
            elif n == 'severity':
                require(state['find_active'] and reg('rbp') == c['instance'], 'severity branch invocation mismatch')
                require(signed32(reg('r14d')) == state['find_threshold'], 'severity threshold mismatch')
                override = reg('bl') & 255
                require(override in (0, 1), 'noncanonical native bool conversion')
                row.update(instance=instance(c['instance']), effective_native_severity_override=bool(override),
                           severity=signed32(reg('eax')), threshold=signed32(reg('r14d')),
                           provenance='actual original first-matching FindVariant and bool conversion result')
                self.move('auto_entry')

    def load(event=None):
        if state['base'] is not None:
            return
        bias = next((a for a, _, _, off, path in mappings() if path == m['module'] and off == 0), None)
        if bias is None:
            return
        state['base'] = bias
        state['maps'] = mappings()
        for rva, expected in GETTERS.values():
            require(read(bias + rva, len(bytes.fromhex(expected))).hex() == expected, 'live pure getter differs')
        for name in PROFILES[m['profile']]:
            state['lanes'].append(Lane(name))
        record(dict(stage='installed', manifest_sha256=digest(manifest_path), profile=m['profile'],
                    configuration_sha256=m['configuration_sha256'], module_sha256=GAME_SHA,
                    hardware_lanes=len(PROFILES[m['profile']]), inferior_calls=False, inferior_writes=False,
                    native_capture_complete=False, gameplay_parity=False))

    gdb.events.new_objfile.connect(load)
    load()

    class Drain(gdb.Command):
        def __init__(self):
            super().__init__('c6-notification-drain', gdb.COMMAND_RUNNING)

        def invoke(self, argument, from_tty):
            require(not argument, 'drain takes no arguments')
            while gdb.selected_inferior().pid and not state['failed']:
                gdb.execute('continue')
                if not gdb.selected_inferior().pid or state['failed']:
                    break
                pending, state['pending'] = state['pending'], None
                if pending is None:
                    # Preserve unexpected signals/breakpoints for the runner.
                    record(dict(stage='unexpected_stop', capture_complete=False))
                    break
                previous, name, address = pending
                state['lanes'].append(Lane(name, previous, address))

    Drain()


def check_capture(manifest_path, output):
    """Emit small public hash metadata, never the private object observations."""
    m = json.loads(manifest_path.read_text())
    require(digest(m['observer']) == m['observer_sha256'], 'observer changed')
    rows = [json.loads(line) for line in Path(m['output']).read_text().splitlines()]
    require(rows and rows[0]['stage'] == 'installed', 'missing installation')
    require(rows[0]['manifest_sha256'] == digest(manifest_path), 'mixed manifest')
    require([r['sequence'] for r in rows] == list(range(1, len(rows) + 1)), 'sequence gap')
    require(not any('observation_error' in r for r in rows), 'capture contains read/provenance error')
    require(not any(r['stage'] == 'unexpected_stop' for r in rows), 'unexpected inferior stop')
    require(len(rows) > 2 and rows[1]['stage'] == 'update_entry' and rows[-1]['stage'] == 'update_return', 'outer boundary order')
    expected = list(PROFILES[m['profile']])
    thread = rows[1]['thread']
    find_open, auto_handle, selected_handle = False, None, None
    for row in rows[1:]:
        stage = row['stage']
        require(row['thread'] == thread and row['trace_armed'] is True and row['trace_phase'] == 5, 'mixed thread/phase')
        require(stage in expected, 'unexpected lane stage order: ' + stage)
        require(int(row['pc'], 16) == SITES[stage][0], 'stage PC mismatch')
        lane = expected.index(stage)
        if stage == 'find_entry':
            require(not find_open, 'nested Find')
            find_open = True
        elif stage == 'selected_return':
            require(find_open and expected[2] == 'auto_entry', 'Find returned during eligibility observation')
            find_open = False
            selected_handle = row['selected']['handle'] if row['selected'] else None
        elif stage == 'activate_call':
            require(row['selected']['handle'] == selected_handle, 'different activation receiver')
        elif stage == 'auto_entry':
            require(find_open, 'eligibility outside Find')
            auto_handle = row['instance']['handle']
        elif stage in ('auto_network', 'definition', 'icon', 'auto_exit', 'severity'):
            actual_handle = row['instance']['handle'] if isinstance(row['instance'], dict) else row['instance']
            require(find_open and actual_handle == auto_handle, 'mixed eligibility receiver')
        next_stage = {
            'update_entry': 'cloud', 'local': 'update_return', 'update_return': 'update_entry',
            'find_entry': 'selected_return', 'activate_call': 'find_entry',
            'release_call': 'release_return', 'release_return': 'release_call',
            'icon': 'auto_exit', 'severity': 'auto_entry',
        }.get(stage)
        if stage == 'cloud':
            next_stage = 'local' if row['play_by_cloud'] and row['game_present'] else 'update_return'
        elif stage == 'selected_return':
            next_stage = 'activate_call' if row['selected'] and row['selected']['lock_count'] <= 0 else 'find_entry'
        elif stage == 'auto_entry':
            next_stage = 'auto_exit' if row['instance']['activated'] else 'auto_network'
        elif stage == 'auto_network':
            next_stage = 'auto_exit' if row['network'] else 'definition'
        elif stage == 'definition':
            next_stage = 'icon' if row['definition_found'] else 'auto_exit'
        elif stage == 'auto_exit':
            next_stage = 'severity' if row['is_auto_activate'] else 'auto_entry'
        require(next_stage is not None, 'unknown stage')
        expected[lane] = next_stage
    require(expected == PROFILES[m['profile']], 'unfinished lane state')
    counts = {stage: sum(r['stage'] == stage for r in rows) for stage in {r['stage'] for r in rows}}
    require(counts.get('update_entry') == counts.get('update_return') == 1, 'expected one complete Update')
    for stage in ('find_entry', 'selected_return', 'auto_entry', 'auto_exit', 'cloud'):
        require(counts.get(stage, 0) > 0, 'missing required stage: ' + stage)
    require(counts['find_entry'] == counts['selected_return'], 'incomplete Find')
    require(counts['auto_entry'] == counts['auto_exit'], 'incomplete IsAutoActivate')
    require(counts.get('release_call', 0) == counts.get('release_return', 0), 'incomplete Release')
    entered = next(r['manager'] for r in rows if r['stage'] == 'update_entry')
    returned = next(r['manager'] for r in rows if r['stage'] == 'update_return')
    require(entered['selected']['owner'] == returned['selected']['owner'], 'mixed owner')
    report = dict(schema='c6-linux-notification-passive-capture-check-v1',
                  manifest_sha256=digest(manifest_path), observations_sha256=digest(m['output']),
                  observations_bytes=Path(m['output']).stat().st_size,
                  configuration_sha256=m['configuration_sha256'], module_sha256=m['module_sha256'],
                  host_sha256=m['host_sha256'], preload_sha256=m['preload_sha256'],
                  observer_sha256=m['observer_sha256'], lifecycle_contract_sha256=m['lifecycle_contract_sha256'],
                  stage_counts=counts, balanced_observation_boundaries=True,
                  inferior_calls=False, inferior_writes=False, hardware_breakpoint_lanes=4,
                  original_run_exit_cleanup_checked=False, same_core_streams_checked=False,
                  complete_live_inventory=False, gameplay_parity=False, full_action_acceptance=0,
                  limits='Checks passive read/boundary completeness only. Runner must separately check original exit/cleanup and command/Core identities. Unexecuted definition branches remain unobserved; callbacks, expiration and full live state are not certified.')
    output.write_text(json.dumps(report, indent=2) + '\n')


def main():
    import sys
    if len(sys.argv) == 4 and sys.argv[1] == 'check':
        check_capture(Path(sys.argv[2]), Path(sys.argv[3]))
        return
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--runtime-report', type=Path, required=True)
    p.add_argument('--module', type=Path, required=True)
    p.add_argument('--host', type=Path, required=True)
    p.add_argument('--profile', choices=PROFILES, required=True)
    p.add_argument('--armed-rva', type=lambda x: int(x, 0), required=True)
    p.add_argument('--phase-rva', type=lambda x: int(x, 0), required=True)
    p.add_argument('--phase', type=int, action='append', required=True)
    p.add_argument('--observations', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    prepare(p.parse_args())


if 'C6_NOTIFICATION_MANIFEST' in globals():
    install(globals()['C6_NOTIFICATION_MANIFEST'])
elif __name__ == '__main__':
    main()
