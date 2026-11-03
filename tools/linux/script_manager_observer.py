#!/usr/bin/env python3
"""Passive checkpoints for naturally invoked Linux Manager CallTestAll.

This module neither runs originals nor installs breakpoints. The sole runner
supplies mapped-memory reads and registers from its all-stop hardware observer.
Heap images and decoded closure data must be written to private evidence only.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import unittest

from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
from script_hks_registry import Image, Table, SnapshotError, inspect_roots, inspect_named_event
from script_can_start_boundary_audit import METHODS as SCRIPT_METHODS
from script_vm_lifecycle_audit import METHODS as LIFECYCLE_METHODS

ROOT = Path(__file__).resolve().parents[2]
SITES = dict(dispatch=0x2ce79e9, ccall_entry=0x3bf22e0,
             acquired=0x3bf22fa, resolved=0x36b3c44,
             protected_return=0x3bf2308, before_release=0x3bf2329,
             released=0x3bf2335, wrapper_return=0x2ce79ff)
GAME_SITES = {'dispatch', 'wrapper_return'}


def require(condition, message):
    if not condition:
        raise SnapshotError(message)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


class CaptureImage(Image):
    """Bounded read-through image, compatible with the reviewed offline decoder.

    Reader must reject unmapped/unreadable extents in the paused inferior. Each
    request is saved; overlapping reads must agree, rather than hiding mutation.
    """
    def __init__(self, reader, budget=16 * 1024 * 1024, limit=65536):
        super().__init__([], limit)
        self.reader, self.budget, self.total = reader, budget, 0

    def read(self, address, size):
        require(0 < address < 1 << 64 and 0 <= size <= self.limit * 40
                and address + size <= 1 << 64, 'invalid/excessive capture extent')
        for start, raw in self.segments:
            if start <= address and address + size <= start + len(raw):
                return raw[address-start:address-start+size]
        require(self.total + size <= self.budget, 'capture byte budget exhausted')
        raw = bytes(self.reader(address, size))
        require(len(raw) == size, 'short passive read')
        for start, old in self.segments:
            lo, hi = max(address, start), min(address + size, start + len(old))
            require(lo >= hi or raw[lo-address:hi-address] == old[lo-start:hi-start],
                    'overlapping snapshot reads changed')
        self.segments.append((address, raw))
        self.total += size
        return raw

    def merged(self):
        result = []
        for address, raw in sorted(self.segments):
            if result and address <= result[-1][0] + len(result[-1][1]):
                start, old = result[-1]
                suffix = address + len(raw) - start - len(old)
                if suffix > 0:
                    result[-1] = (start, old + raw[-suffix:])
            else:
                result.append((address, raw))
        return result

    def write_private(self, directory, identity):
        directory = Path(directory).resolve()
        require(not directory.is_relative_to(ROOT), 'heap evidence cannot enter public checkout')
        directory.mkdir(parents=True, exist_ok=False)
        rows = []
        for index, (address, raw) in enumerate(self.merged()):
            name = 'segment-%04d.bin' % index
            (directory / name).write_bytes(raw)
            rows.append(dict(address=hex(address), size=len(raw), file=name, sha256=digest(raw)))
        manifest = dict(schema='c6-linux-script-hks-image-v1', host_load_bias=0,
                        host_sha256=HOST_SHA, gamecore_sha256=GAME_SHA,
                        segments=rows, **identity)
        (directory / 'image.json').write_text(json.dumps(manifest, indent=2) + '\n')
        return manifest


def inspect_resolved_function(image, state):
    # hksi_lua_isfunction(-1) reads top-16, bounded below by base.
    # Capture the actual lookup result even if __index returned an
    # uncached event/method different from the raw named registry.
    top, base = image.integer(state + 0x48), image.integer(state + 0x50)
    require(top >= base + 16, 'resolved function stack is empty')
    function = image.value(top - 16)
    require(function.tag in (9, 10), 'resolved value is not supported function kind')
    actual = dict(value=function.record(), closure=image.closure(function),
                  stack_top=hex(top), stack_base=hex(base))
    if function.tag == 10 and actual['closure']['function'] == '0x36b3540':
        upvalues = actual['closure']['upvalues']
        require(upvalues and upvalues[0]['tag'] == 5, 'actual TestAll group upvalue missing')
        groups = Table(image, int(upvalues[0]['payload_hex'], 0))
        actual['groups'] = groups.record()
        actual['callback_arrays'] = []
        for key, value in groups.entries():
            if value.tag != 5:
                continue
            array = Table(image, value.bits)
            length = array.length()
            require(length <= image.limit, 'actual callback array exceeds budget')
            slots = []
            for index in range(1, length + 1):
                callback = array.integer(index)
                slot = dict(index=index, value=callback.record())
                if callback.tag in (9, 10):
                    slot['closure'] = image.closure(callback)
                slots.append(slot)
            actual['callback_arrays'].append(dict(group_key=key.record(),
                table=array.record(), objlen=length, callbacks=slots))
    return actual


class Observer:
    """Runner-owned state machine; observe returns the next required site names.

    At most two hardware sites are needed simultaneously. Start with dispatch;
    after acquired keep resolved and protected_return armed. A missing lookup
    naturally skips resolved. Caller must stop all inferior threads, pass exact
    register values, and gate actual profile/mapped module/live body identities.
    No GetCurrentThreadHANDLE or Lua APIs are called here.
    """
    def __init__(self, read, emit, gamecore_base, configuration_sha256, private_directory):
        require(len(configuration_sha256) == 64 and
                all(c in '0123456789abcdef' for c in configuration_sha256), 'configuration identity')
        def bounded_read(address, size):
            require(0 < address < 1 << 64 and 0 <= size <= 65536 * 40 and
                    address + size <= 1 << 64, 'invalid observer read extent')
            raw = bytes(read(address, size))
            require(len(raw) == size, 'short observer read')
            return raw
        self.read, self.emit, self.base = bounded_read, emit, gamecore_base
        self.configuration = configuration_sha256
        self.directory = Path(private_directory)
        require(not self.directory.exists(), 'fresh private capture root required')
        self.private_root_created = False
        require(not self.directory.resolve().is_relative_to(ROOT), 'private capture directory required')
        self.call, self.ordinal = None, 0
        self.failed = False
        self.expected = ('dispatch',)

    def num(self, address):
        return struct.unpack('<Q', self.read(address, 8))[0]

    def address(self, site):
        return SITES[site] + (self.base if site in GAME_SITES else 0)

    def snapshot(self, phase):
        if not self.private_root_created:
            self.directory.mkdir(parents=True, exist_ok=False)
            self.private_root_created = True
        image = CaptureImage(self.read)
        try:
            self._snapshot(image, phase)
        except Exception as error:
            # Preserve even a truncated/unsupported graph as unavailable data.
            # Never turn a failed capture into absent callbacks.
            manifest = image.write_private(self.directory /
                ('call-%04d-%s-partial' % (self.ordinal, phase)),
                dict(configuration_sha256=self.configuration, gamecore_base=hex(self.base),
                     phase=phase, occurrence=self.ordinal, partial=True,
                     failure=str(error), observation_boundary_accepted=False))
            self.emit(dict(event='partial_snapshot', phase=phase, occurrence=self.ordinal,
                           failure=str(error), image_manifest=manifest,
                           callbacks_available=False, callback_absence_proven=False))
            raise

    def _snapshot(self, image, phase):
        roots = inspect_roots(image, self.base)
        state = int(roots['main_lua_state'], 0)
        require(state == self.call['state'] and state != 0, 'main-state identity changed/null')
        lock = image.read(0x6ec9c70, 64)
        require(struct.unpack_from('<i', lock, 12)[0] > 0 and
                struct.unpack_from('<Q', lock, 16)[0] != 0 and
                struct.unpack_from('<Q', lock, 24)[0] != 0, 'inconsistent acquired Lua lock fields')
        # These bytes are retained unclassified; no collector flag is an idle proof.
        global_state = image.integer(state + 0x10)
        image.read(state, 0x80)
        image.read(global_state + 0x58, 0x128)
        result = inspect_named_event(image, state)
        if phase == 'naturally_resolved_function_before_original_call':
            result['actual_resolved_function'] = inspect_resolved_function(image, state)
        identity = dict(configuration_sha256=self.configuration, gamecore_base=hex(self.base),
                        phase=phase, occurrence=self.ordinal,
                        observation_boundary_accepted=False)
        manifest = image.write_private(self.directory / ('call-%04d-%s' % (self.ordinal, phase)), identity)
        # Reconstruct exclusively from the saved bytes to check image completeness.
        offline = Image(image.merged(), image.limit)
        raw_result = dict(result)
        resolved = raw_result.pop('actual_resolved_function', None)
        if resolved is not None:
            require(inspect_resolved_function(offline, state) == resolved,
                    'offline actual resolved function differs')
        require(inspect_named_event(offline, state) == raw_result, 'offline reconstruction differs')
        self.emit(dict(event='snapshot', **identity, roots=roots, lua_lock_raw=lock.hex(),
                       lua_owner_kind='ASL ThreadHANDLE pointer; not core u32 thread ID',
                       registry=result, image_manifest=manifest,
                       original_acquisition_observed=True,
                       callback_effects_accepted=False))

    def fail(self, reason):
        self.failed = True
        self.emit(dict(event='observer_failure', reason=str(reason),
                       occurrence=self.ordinal, pending_sites=list(self.expected),
                       partial_call=self.call, callbacks_available=False,
                       callback_absence_proven=False))

    def close(self):
        if self.call is not None or self.expected != ('dispatch',):
            self.fail('terminal missing checkpoint/unwind/partial call')
        self.emit(dict(event='observer_terminal', occurrences=self.ordinal,
                       protocol_complete=not self.failed, native_parity_accepted=False))
        return not self.failed

    def observe(self, event, r, thread, provenance=None):
        require(not self.failed, 'observer already failed')
        try:
            return self._observe(event, r, thread, provenance)
        except Exception as error:
            self.fail(error)
            raise

    def _observe(self, event, r, thread, provenance=None):
        require(event in self.expected, 'unmatched checkpoint: ' + event)
        require(r['pc'] == self.address(event), 'wrong original checkpoint PC')
        if event == 'dispatch':
            require(isinstance(provenance, dict), 'missing runner provenance')
            require(provenance.get('configuration_sha256') == self.configuration and
                    provenance.get('scope') == 'genuine_lower_core_full_Manager' and
                    provenance.get('all_threads_stopped') is True and
                    provenance.get('active_lua_parent_frames') == 0 and
                    provenance.get('active_gc_parent_frames') == 0,
                    'unadmitted/reentrant runner provenance')
            require(self.num(r['rsp'] + 56) == self.base + 0x2da22a0,
                    'not genuine Manager CanStart continuation')
            require(bytes(self.read(r['r12'], 18)) == b'CanStartOperation\0', 'wrong event')
            require(self.num(r['r13']) == 0x53a63a8, 'overridden ScriptSystem')
            self.ordinal += 1
            self.call = dict(thread=thread, wrapper_rsp=r['rsp'], script=r['r13'],
                             provenance=provenance, event_pointer=r['r12'], args=r['r15'], out=r['r14'], core_lock_was_held=bool(r['rbx'] & 255))
            self.expected = ('ccall_entry',)
        else:
            require(thread == self.call['thread'], 'cross-thread/reentrant checkpoint')
            if event == 'ccall_entry':
                require(r['rsp'] == self.call['wrapper_rsp'] - 72, 'CCall stack frame differs')
                require(self.num(r['rsp']) == 0x36b40e1 and r['rsi'] == 0x36b3bc0,
                        'wrong protected callback/continuation')
                require(self.num(r['rsp'] + 64) == self.base + 0x2ce79ff,
                        'not concrete ScriptSystem tail path')
                context = bytes(self.read(r['rdx'], 26))
                require(struct.unpack_from('<Q', context)[0] == self.call['event_pointer'] and
                        struct.unpack_from('<Q', context, 8)[0] == 0x462e7b4 and
                        struct.unpack_from('<Q', context, 16)[0] == self.call['args'],
                        'wrong TestAll context')
                self.call.update(entry_rsp=r['rsp'], state=r['rdi'], context=r['rdx'], context_entry_raw=context.hex())
                self.expected = ('acquired',)
            elif event == 'acquired':
                require(r['rsp'] == self.call['entry_rsp'] - 24 and
                        r['rbx'] == self.call['state'] and r['r15'] == 0x36b3bc0 and
                        r['r14'] == self.call['context'], 'acquisition frame changed')
                self.snapshot('acquired_before_original_lookup')
                self.expected = ('resolved', 'protected_return')
            elif event == 'resolved':
                require(r['rbx'] == self.call['state'] and r['r14'] == self.call['context'],
                        'nested/unrelated named-event resolution')
                self.snapshot('naturally_resolved_function_before_original_call')
                self.expected = ('protected_return',)
            elif event == 'protected_return':
                require(r['rsp'] == self.call['entry_rsp'] - 24, 'nested protected return')
                self.call['protected_status_i32'] = struct.unpack('<i', struct.pack('<I', r['rax'] & 0xffffffff))[0]
                self.expected = ('before_release',)
            elif event == 'before_release':
                require(r['rsp'] == self.call['entry_rsp'] - 24, 'release frame changed')
                self.snapshot('protected_call_completed_before_original_release')
                self.call['protected_success'] = bool(r['rbx'] & 255)
                self.expected = ('released',)
            elif event == 'released':
                require(r['rsp'] == self.call['entry_rsp'] - 24, 'unmatched release return')
                self.call['lock_after_release_raw'] = bytes(self.read(0x6ec9c70, 64)).hex()
                self.expected = ('wrapper_return',)
            elif event == 'wrapper_return':
                require(r['rsp'] == self.call['wrapper_rsp'], 'wrapper return frame changed')
                self.emit(dict(event='natural_CallTestAll_return', occurrence=self.ordinal,
                               return_bit=bool(r['rax'] & 255),
                               out_value_raw=bytes(self.read(self.call['out'], 1)).hex(), **self.call))
                self.call = None
                self.expected = ('dispatch',)
        return self.expected


def audit(assets):
    host, game = Elf(assets / 'Civ6', HOST_SHA), Elf(assets / 'libGameCore_XP2.so', GAME_SHA)
    spans = [(host, name, address, size, sha) for name, address, size, sha in SCRIPT_METHODS]
    spans += [(host, name, address, size, sha) for module, name, address, size, sha in LIFECYCLE_METHODS
              if module == 'host' and ('CCallWithErrorHandling' in name or name in
                  ('EnterCriticalSection', 'LeaveCriticalSection'))]
    spans += [(game, 'Manager.CanStart', 0x2da20f0, 579,
               '573164c5e12ad82eeb6761d2492634ae376ae6de63526a3b94cf0608d8550927'),
              (game, 'XP2 utility wrapper', 0x2ce79b0, 116,
               '47ad40fd9efa7ddcbe88b6bc4de3eff8c174262c6cc031ce76d78e63094c431e')]
    for elf, name, address, size, sha in spans:
        require(digest(elf.read(address, size)) == sha, 'original body mismatch: ' + name)
    # Complete negative-index branch: top at +0x48, base at +0x50.
    require(host.read(0x343dbe3, 0x16) == bytes.fromhex(
        '79114863c648c1e00448034748483b475073c431c0c3'),
        'actual function negative-stack-index source')
    host.call(0x3bf22f5, 0x3c402d8)
    host.call(0x36b40dc, 0x3bf22e0)
    host.call(0x36b3c2e, 0x2f81f60)
    host.call(0x36b3c3b, 0x343db80)
    host.call(0x3bf2303, 0x3bc2f54)
    host.call(0x3bf2330, 0x3c402de)
    return dict(schema='c6-linux-script-manager-live-v1', source_checks_passed=True,
                host_sha256=HOST_SHA, gamecore_sha256=GAME_SHA,
                checked_body_count=len(spans), sites={k: hex(v) for k, v in SITES.items()},
                original_execution=False, native_window_hit=False,
                full_manager_supported=False, callback_effects_supported=False,
                complete_state_actions_rng_turns_supported=False,
                historical_raw_save_load_difference_bytes=89198, unknown_bytes_masked=0,
                observer_source_sha256=digest(Path(__file__).read_bytes()))


class Tests(unittest.TestCase):
    def test_budget_and_bad_extent(self):
        image = CaptureImage(lambda p, n: bytes(n), budget=4)
        with self.assertRaises(SnapshotError):
            image.read(1, 5)
        with self.assertRaises(SnapshotError):
            image.read(0, 1)
        with self.assertRaises(SnapshotError):
            image.read(1 << 64, 0)

    def test_overlap_merge_and_offline(self):
        image = CaptureImage(lambda p, n: bytes(range(p, p+n)))
        image.read(4, 8)
        image.read(1, 6)
        image.read(9, 6)
        self.assertEqual(Image(image.merged()).read(1, 14), bytes(range(1, 15)))

    def test_mutating_overlap_refused(self):
        image = CaptureImage(lambda p, n: bytes([p]) * n)
        image.read(4, 8)
        with self.assertRaisesRegex(SnapshotError, 'changed'):
            image.read(3, 4)

    def test_wrong_event_does_not_read_memory(self):
        def refuse(p, n):
            self.fail('memory read before provenance gate')
        observer = Observer(refuse, lambda r: None, 0x10000000, 'a'*64, '/tmp/c6-private-test')
        with self.assertRaisesRegex(SnapshotError, 'unmatched'):
            observer.observe('acquired', {}, 1)

    def test_exact_frame_schedule_and_original_result(self):
        memory = {}
        def put(address, raw):
            for offset, byte in enumerate(raw):
                memory[address + offset] = byte
        def q(address, value):
            put(address, struct.pack('<Q', value))
        def read(address, size):
            return bytes(memory[address + offset] for offset in range(size))
        rows = []
        observer = Observer(read, rows.append, 0x10000000, 'a'*64, '/tmp/c6-private-test')
        phases = []
        observer.snapshot = phases.append
        q(0x1000+56, 0x10000000+0x2da22a0)
        put(0x2000, b'CanStartOperation\0')
        q(0x3000, 0x53a63a8)
        q(0x1000-72, 0x36b40e1)
        q(0x1000-8, 0x10000000+0x2ce79ff)
        put(0x4000, struct.pack('<QQQBB', 0x2000, 0x462e7b4, 0x5000, 0, 0))
        put(0x6000, b'\x00')
        put(0x6ec9c70, bytes(64))
        provenance = dict(configuration_sha256='a'*64, scope='genuine_lower_core_full_Manager',
                          all_threads_stopped=True, active_lua_parent_frames=0, active_gc_parent_frames=0)
        def event(name, **registers):
            registers['pc'] = observer.address(name)
            return observer.observe(name, registers, 7, provenance)
        event('dispatch', rsp=0x1000, r12=0x2000, r13=0x3000, r15=0x5000, r14=0x6000, rbx=1)
        event('ccall_entry', rsp=0x1000-72, rdi=0x7000, rsi=0x36b3bc0, rdx=0x4000)
        self.assertEqual(event('acquired', rsp=0x1000-96, rbx=0x7000, r15=0x36b3bc0, r14=0x4000),
                         ('resolved', 'protected_return'))
        # A missing/nonfunction event skips the resolved site naturally.
        event('protected_return', rsp=0x1000-96, rax=0)
        event('before_release', rsp=0x1000-96, rbx=1)
        event('released', rsp=0x1000-96)
        self.assertEqual(event('wrapper_return', rsp=0x1000, rax=0), ('dispatch',))
        self.assertEqual(len(phases), 2)
        self.assertFalse(rows[0]['return_bit'])
        self.assertEqual(rows[0]['out_value_raw'], '00')
        self.assertTrue(observer.close())
        self.assertTrue(rows[-1]['protocol_complete'])

    def test_partial_terminal_never_reports_absence(self):
        rows = []
        observer = Observer(lambda p, n: bytes(n), rows.append, 0x10000000,
                            'a'*64, '/tmp/c6-private-test')
        observer.call = dict(state=0x1234)
        observer.expected = ('protected_return',)
        self.assertFalse(observer.close())
        self.assertFalse(rows[0]['callback_absence_proven'])
        self.assertEqual(rows[0]['partial_call']['state'], 0x1234)
        self.assertFalse(rows[-1]['protocol_complete'])

    def test_wrong_manager_continuation(self):
        observer = Observer(lambda p, n: bytes(n), lambda r: None, 0x10000000,
                            'a'*64, '/tmp/c6-private-test')
        with self.assertRaisesRegex(SnapshotError, 'genuine Manager'):
            observer.observe('dispatch', dict(pc=observer.address('dispatch'), rsp=0x1000), 1,
                dict(configuration_sha256='a'*64, scope='genuine_lower_core_full_Manager',
                     all_threads_stopped=True, active_lua_parent_frames=0, active_gc_parent_frames=0))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        unittest.main(argv=['script_manager_observer'], exit=True)
    require(args.output is not None, '--output required')
    args.output.write_text(json.dumps(audit(args.assets), indent=2) + '\n')
