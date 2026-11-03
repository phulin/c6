#!/usr/bin/env python3
"""Static original-code gates and synthetic passive callback refusal tests.

No original code execution. Synthetic memory below is parser testing only and
must never become an original receiver or a gameplay/capture acceptance input.
"""
import argparse
import copy
import io
import json
from pathlib import Path
import struct
import tempfile
import unittest
from military_initialization_observer import (
    GAME_SHA, HOST_SHA, HardwareSchedule, Memory, Observer, SITES, VECTOR_OFFSETS, digest, prepare, validate_runtime_profile,
)


class Fixture:
    def __init__(self):
        self.raw = bytearray(0x10000)
        self.bias = 0x10000000
        self.code = {self.bias + address: bytes.fromhex('9090909090909090') for address in SITES.values()}
        self.rows = []
        self.plan = dict(schema='c6-linux-military-observer-plan-v1', gamecore_sha256=GAME_SHA,
                         host_sha256=HOST_SHA, observer_sha256=digest(Path(__file__).with_name('military_initialization_observer.py')),
                         site_bytes={name: '9090909090909090' for name in SITES})
        self.military, self.player, self.stream = 0x1000, 0x2000, 0x3000
        self.put(self.military + 0x98, '<Q', self.player)
        self.put(self.player + 0xe0, '<i', 13)
        self.put(self.player + 0x278, '<B', 0)
        self.put(self.player + 0x2d8, '<i', -1)
        for i, offset in enumerate(VECTOR_OFFSETS.values()):
            data = 0x4000 + i * 0x100
            self.put(self.military + offset, '<QIIII', data, 2, 0xffffffff, 2, 0xa5a5a5a5)
            self.put(data, '<ii', -2147483648, 2147483647)
        self.put(0x7000, '<QQQ', 0x7100, 0x7110, 0x7110)
        for i in range(2):
            self.put(0x7100+i*8, '<Q', 0x7200+i*40)
            self.put(0x7200+i*40+16, '<iIIi', i+9, i, 0x81230000+i, -i)
        self.binding = dict(kind='read', lifecycle_id='load-1', owner_lifetime_id='player-13-generation-1',
                            occurrence_id='read-1', stream_pointer=self.stream,
                            rules_database_sha256=None, live_database_snapshot_unavailable=True,
                            **{key: 'a'*64 for key in ('configuration_sha256', 'rules_inputs_sha256',
                               'dlc_sha256', 'mods_sha256', 'input_save_sha256', 'input_core_sha256',
                               'runtime_profile_sha256', 'runner_source_sha256')})
        self.observer = Observer(self.read, self.rows.append, self.bias, self.plan)

    def put(self, address, fmt, *values):
        struct.pack_into(fmt, self.raw, address, *values)

    def read(self, address, size):
        if address in self.code:
            return self.code[address][:size]
        if not (0 < address and address+size <= len(self.raw)):
            raise ValueError('synthetic unreadable extent')
        return bytes(self.raw[address:address+size])

    def event(self, name, frame='root', binding=None, **registers):
        regs = dict(pc=self.bias + SITES.get(name, 0), rsp=0x8000, rax=0, rdi=0, rsi=0,
                    rdx=0, rcx=0, r8=0, rbx=0, rbp=0, r14=0)
        if name.endswith('_return') and name not in ('personality_return', 'leader_return') and self.observer.calls:
            regs['rsp'] = self.observer.current()['entry_rsp'] + 8
        regs.update(registers)
        return self.observer.observe(name, regs, [1, 1, 1], binding, frame)

    def root(self, kind):
        binding = copy.deepcopy(self.binding)
        binding.update(kind=kind, occurrence_id=kind+'-1')
        if kind == 'write':
            binding['output_id'] = 'save-2'
        self.put(0x8000, '<Q', 0x8888)
        assert not self.event(kind+'_entry', binding=binding, rdi=self.stream, rsi=self.military)

    def init(self):
        self.root('read')
        self.put(0x8000, '<Q', self.bias+SITES['read_initialized'])
        assert not self.event('initialize_entry', frame='init', rdi=self.military, rsi=self.player)

    def writer(self):
        self.root('write')
        assert not self.event('write_guard', rbx=self.military, r14=self.stream, rax=2)
        for offset, ret in zip(VECTOR_OFFSETS.values(), (0x2145526, 0x2145548, 0x214556a)):
            self.put(0x8000, '<Q', self.bias+ret)
            data = struct.unpack_from('<Q', self.raw, self.military+offset)[0]
            assert not self.event('hash_array_entry', rdi=self.stream, rsi=data, rdx=0x7000)
        assert not self.event('write_return', pc=0x8888)


class CallbackTests(unittest.TestCase):
    def test_vector_capacity_is_u32_and_signed_values(self):
        f = Fixture()
        v = Memory(f.read).vector(f.military+0x158)
        self.assertEqual(v['capacity_u32'], 2)
        self.assertEqual(v['padding_u32'], 0xffffffff)
        self.assertEqual(v['values_i32'], [-2147483648, 2147483647])

    def test_capacity_less_than_length_refused(self):
        f = Fixture()
        f.put(f.military+0x158+8, '<I', 1)
        with self.assertRaises(ValueError):
            Memory(f.read).vector(f.military+0x158)

    def test_unreadable_capacity_refused(self):
        f = Fixture()
        f.put(f.military+0x158, '<Q', 0xfffc)
        with self.assertRaises(ValueError):
            Memory(f.read).vector(f.military+0x158)

    def test_ordered_definition_fields(self):
        f = Fixture()
        rows = Memory(f.read).definitions(0x7000)['ordered']
        self.assertEqual(rows[1]['collection_index_u32'], 1)
        self.assertEqual(rows[1]['default_value_i32'], -1)
        self.assertEqual(rows[0]['operation_type_hash_u32'], 0x81230000)

    def test_definition_malformed_span_refused(self):
        f = Fixture()
        f.put(0x7000+8, '<Q', 0x710f)
        with self.assertRaises(ValueError):
            Memory(f.read).definitions(0x7000)

    def test_genuine_boundary_protocol_without_personality(self):
        f = Fixture()
        f.init()
        self.assertFalse(f.event('personality_return', frame='init', rax=0))
        self.assertFalse(f.event('initialize_return', frame='init', pc=f.bias+SITES['read_initialized']))
        self.assertFalse(f.event('read_initialized'))
        self.assertFalse(f.event('read_return', pc=0x8888))
        f.observer.close()
        self.assertFalse(f.observer.failed)
        self.assertEqual(f.rows[2]['snapshot']['vectors'], f.rows[4]['snapshot']['vectors'])
        self.assertFalse(f.rows[-1]['capture_complete'])

    def test_missing_branch_observation_refused(self):
        f = Fixture()
        f.init()
        self.assertTrue(f.event('initialize_return', frame='init', pc=f.bias+SITES['read_initialized']))
        self.assertIn('personality', f.rows[-1]['reason'])

    def test_native_query_and_count_observed(self):
        f = Fixture()
        f.init()
        self.assertFalse(f.event('personality_return', frame='init', rax=1))
        self.assertFalse(f.event('leader_return', frame='init', rax=12))
        f.put(0x8000, '<Q', f.bias+0x2145c9b)
        f.put(0x6000, '<Q', 0x9000)
        f.put(0x6000+0x140, '<i', -6)
        f.put(0x6000+0x40, '<Q', 0x6100)
        f.put(0x6100, '<QQ', 0x6200, 0x7000)
        f.put(0x6200+0x30, '<Q', f.bias+0x21141c0)
        self.assertFalse(f.event('query_entry', frame='init', rdi=0x6000, rsi=f.military+0x158, rcx=4, r8=0xffffffff))
        f.put(0x8000, '<Q', f.bias+0x203ac8b)
        self.assertFalse(f.event('count_entry', frame='count-1', rdi=0x7000, rbx=0x6000, rbp=f.military+0x158))
        self.assertFalse(f.event('count_return', frame='count-1', pc=f.bias+0x203ac8b, rax=2))
        obs = f.observer.current('initialize')['observations']
        self.assertEqual(obs['leader_query']['default_value_i32'], -6)
        self.assertEqual(obs['leader_query']['handicap_i32'], -1)
        self.assertEqual(len(obs['active_definitions']['ordered']), 2)

    def test_wrong_stream_refused(self):
        f = Fixture()
        f.put(0x8000, '<Q', 0x8888)
        self.assertTrue(f.event('read_entry', binding=f.binding, rdi=0xdead, rsi=f.military))

    def test_changed_live_site_refused(self):
        f = Fixture()
        f.code[f.bias+SITES['read_entry']] = b'badbytes'
        self.assertTrue(f.event('read_entry', binding=f.binding, rdi=f.stream, rsi=f.military))

    def test_unmatched_return_and_unwind_refused(self):
        f = Fixture()
        f.root('read')
        self.assertTrue(f.event('read_return', frame='another-frame', pc=0x8888))
        g = Fixture()
        g.root('read')
        g.observer.close()
        self.assertTrue(g.observer.failed)

    def test_write_guard_arrays_and_actual_output_binding(self):
        f = Fixture()
        f.writer()
        self.assertFalse(f.observer.bind_output('write-1', 'save-2', 'b'*64, 'c'*64, [10, 200]))
        f.observer.close()
        self.assertFalse(f.observer.failed)
        self.assertEqual(len([r for r in f.rows if r['event'] == 'hash_array_entry']), 3)
        self.assertFalse(f.rows[-1]['capture_complete'])

    def test_missing_output_or_mismatched_output_refused(self):
        f = Fixture()
        f.writer()
        f.observer.close()
        self.assertTrue(f.observer.failed)
        g = Fixture()
        g.writer()
        self.assertTrue(g.observer.bind_output('write-1', 'different-save', 'b'*64, 'c'*64, [10, 200]))

    def test_null_query_does_not_fabricate_or_require_count_callback(self):
        f = Fixture()
        f.init()
        f.event('personality_return', frame='init', rax=1)
        f.event('leader_return', frame='init', rax=3)
        f.put(0x8000, '<Q', f.bias+0x2145c9b)
        self.assertFalse(f.event('query_entry', frame='init', rdi=0x6000, rsi=f.military+0x158))
        self.assertIsNone(f.observer.current()['observations']['leader_query']['captured_definition_pointer'])
        self.assertFalse(f.event('initialize_return', frame='init', pc=f.bias+SITES['read_initialized']))

    def test_unrelated_native_count_caller_refused(self):
        f = Fixture()
        f.init()
        f.event('personality_return', frame='init', rax=1)
        f.event('leader_return', frame='init', rax=0xffffffff)
        f.put(0x8000, '<Q', 0x9999)
        self.assertTrue(f.event('count_entry', frame='unrelated-count', rdi=0x7000, rbx=f.military+0x158))
        self.assertIn('unknown-leader assignment', f.rows[-1]['reason'])

    def test_initialized_branch_missing_count_refused(self):
        f = Fixture()
        f.init()
        f.event('personality_return', frame='init', rax=1)
        f.event('leader_return', frame='init', rax=0xffffffff)
        self.assertTrue(f.event('initialize_return', frame='init', pc=f.bias+SITES['read_initialized']))
        self.assertIn('definition count', f.rows[-1]['reason'])

    def test_missing_write_guard_refused(self):
        f = Fixture()
        f.root('write')
        self.assertTrue(f.event('write_return', pc=0x8888))

    def test_explicit_passive_runtime_profile(self):
        cfg = dict(lower_core_military_initialization_observer=dict(mode='ordinary_load_save_passive'))
        for key in ('core_component_mode', 'lower_core_movement_cost', 'lower_core_domain_metadata',
                    'lower_core_placement_metadata', 'lower_core_placement_handler_metadata',
                    'lower_core_unit_operation_manager_metadata', 'full_save_request'):
            cfg[key] = None
        validate_runtime_profile(cfg)
        cfg['core_component_mode'] = 'execute'
        with self.assertRaises(ValueError):
            validate_runtime_profile(cfg)

    def test_missing_passive_runtime_profile_refused(self):
        with self.assertRaises(ValueError):
            validate_runtime_profile({})

    def test_hardware_schedule_three_lanes_plus_provenance(self):
        for personality, leader, query in ((False, None, None), (True, -1, None),
                                            (True, 4, False), (True, 4, True)):
            schedule = HardwareSchedule()
            events = [('read_entry', dict(return_pc=0x9999)), ('initialize_entry', {}),
                      ('personality_return', dict(value=personality))]
            if personality:
                events.append(('leader_return', dict(value=leader)))
                if leader != -1:
                    events.append(('query_entry', dict(value=query)))
                if leader == -1 or query:
                    events += [('count_entry', dict(return_pc=0x8888)), ('count_return', {})]
            events += [('initialize_return', {}), ('read_initialized', {}), ('read_return', {})]
            for event, arguments in events:
                self.assertLessEqual(len(schedule.advance(event, **arguments)), 3)
            self.assertEqual(schedule.phase, 'idle')
        for present in (False, True):
            schedule = HardwareSchedule()
            schedule.advance('write_entry', return_pc=0x9999)
            schedule.advance('write_guard', value=present)
            if present:
                for _ in range(3):
                    self.assertLessEqual(len(schedule.advance('hash_array_entry')), 3)
            schedule.advance('write_return')
            self.assertEqual(schedule.phase, 'idle')

    def test_hardware_schedule_refuses_missing_count(self):
        schedule = HardwareSchedule()
        schedule.advance('read_entry', return_pc=0x9999)
        schedule.advance('initialize_entry')
        schedule.advance('personality_return', value=True)
        schedule.advance('leader_return', value=-1)
        with self.assertRaises(ValueError):
            schedule.advance('initialize_return')

    def test_bad_return_rsp_refused(self):
        f = Fixture()
        f.init()
        f.event('personality_return', frame='init', rax=0)
        self.assertTrue(f.event('initialize_return', frame='init', pc=f.bias+SITES['read_initialized'], rsp=0xdead))

    def test_read_only_postload_cannot_claim_initialization(self):
        f = Fixture()
        f.root('read')
        self.assertTrue(f.event('read_initialized'))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3]/'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(CallbackTests)
    log = io.StringIO()
    result = unittest.TextTestRunner(stream=log).run(suite)
    if not result.wasSuccessful():
        raise ValueError(log.getvalue())
    with tempfile.TemporaryDirectory(prefix='c6-military-observer-') as tmp:
        plan = prepare(args.assets/'libGameCore_XP2.so', args.assets/'Civ6', Path(tmp)/'plan.json')
    report = dict(schema='c6-linux-military-observer-preparation-v1', checks_passed=True,
                  gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA,
                  observer_sha256=plan['observer_sha256'], audit_sha256=digest(__file__),
                  dwarf_sha256=plan['dwarf_sha256'], extended_dwarf_sha256=plan['extended_dwarf_sha256'],
                  static_method_checks=34, callback_protocol_tests=result.testsRun,
                  callback_schema='c6-linux-military-observation-v1',
                  source_contract_commit='78866b74ea27036a25185b3108e094eef4b7cb61',
                  vector_abi=dict(data_pointer_offset=0, capacity_u32_offset=8, size_u32_offset=16, element_type='i32'),
                  checkpoints={name: hex(address) for name, address in SITES.items()},
                  original_execution=False, hooks_installed=False, genuine_snapshot_captured=False,
                  runtime_profile_changed=False, capture_complete=False, gameplay_parity=False,
                  missing=['runner hook integration and source/profile-reviewed ordinary-load capture',
                           'genuine prior/initialized/post-Read and actual writer snapshots for every owner',
                           'actual ordered definitions/defaults under matching rules lifetime',
                           'independent saved-output/canonical Core stream attribution review'],
                  full_gameplay_state=False, actions=False, turns=False,
                  recorded_raw_save_load_difference_bytes=89198, unknown_bytes_masked=False)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True, synthetic_callback_tests=result.testsRun, original_execution=False)))


if __name__ == '__main__':
    main()
