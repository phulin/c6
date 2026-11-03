#!/usr/bin/env python3
"""Synthetic helper/decoder tests. Never loads or executes original game code."""
import copy
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('diplomacy_decode', ROOT / 'tools/linux/diplomacy_manager_capture_decode.py')
decode = importlib.util.module_from_spec(spec)
spec.loader.exec_module(decode)


def encoded(rows):
    return ('\n'.join(json.dumps(row) for row in rows) + '\n').encode()


class CaptureDecoderTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory(prefix='c6-diplomacy-capture-test-')
        directory = Path(cls.temp.name)
        harness = directory / 'emit.c'
        # Reuse the helper author's synthetic byte-image fixture. It supplies a
        # bounded mock reader; pointers never refer to original game objects.
        harness.write_text('''#define main prior_synthetic_tests
#include "''' + str(ROOT / 'tools/check/diplomacy_manager_capture_test.c') + '''"
#undef main
int main(int argc,char **argv) {
    Fixture f;init(&f);
    if(argc>1 && argv[1][0]=='n')nonempty(&f);
    if(argc>1 && argv[1][0]=='p')f.reject_reads=1;
    C6DiplomacyCaptureAdmission a=admission(&f,stdout);
    (void)c6_linux_diplomacy_manager_capture(&a);
    return 0;
}
''')
        cls.binary = directory / 'emit'
        subprocess.run(['cc', '-std=c11', '-Wall', '-Wextra', '-Werror',
                        str(harness), str(ROOT / 'native/linux/diplomacy_manager_capture.c'),
                        '-o', str(cls.binary)], check=True, capture_output=True)
        cls.empty = subprocess.check_output([str(cls.binary)])
        cls.nonempty = subprocess.check_output([str(cls.binary), 'n'])
        cls.partial = subprocess.check_output([str(cls.binary), 'p'])

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def test_actual_helper_empty_format_preserves_unknown_ai_and_all_slots(self):
        result = decode.decode(self.empty, '1' * 64)
        self.assertTrue(result['complete'])
        self.assertTrue(result['inventory_consistent'])
        self.assertTrue(result['open_sessions_observed_empty'])
        self.assertEqual(len(result['players']['allocation_slots']), 65)
        self.assertEqual(len(result['players']['id_slots']), 65)
        self.assertFalse(result['native_execution_admitted'])
        self.assertEqual(result['current_is_ai'], 'unknown_configuration_schema')

    def test_nonempty_is_retained_not_converted_into_supported_statement(self):
        result = decode.decode(self.nonempty)
        self.assertFalse(result['open_sessions_observed_empty'])
        self.assertEqual(len(result['open_sessions']['entries']), 1)
        self.assertEqual(result['pending_actions'][0]['id'], (1 << 32) - 1)
        self.assertEqual(result['pending_actions'][0]['from'], -2)
        self.assertEqual(result['statement_payloads'], 'unknown')
        # Fixture deliberately repeats a handle in its everAlive export. Preserve
        # the fact and report inconsistency; never invent a canonical ordering.
        self.assertFalse(result['inventory_consistent'])
        self.assertIn('ever_alive_allocation_order_disagrees', result['inventory_issues'])

    def test_partial_reads_stay_partial_with_raw_prefix(self):
        result = decode.decode(self.partial)
        self.assertFalse(result['complete'])
        self.assertEqual(result['failure'], 'unreadable')
        self.assertNotIn('open_sessions_observed_empty', result)
        self.assertTrue(result['raw_records'])

    def test_profile_framing_and_scope_refuse_contradictions(self):
        with self.assertRaises(decode.InvalidCapture):
            decode.decode(self.empty, '2' * 64)
        rows = decode.load_records(self.empty)
        for mutate in [lambda r: r[0].update(gameplay_parity=True),
                       lambda r: r[-1].update(failure='contradiction'),
                       lambda r: r[0].update(main_tid=True),
                       lambda r: r[0].update(stage='bad"stage')]:
            bad = copy.deepcopy(rows)
            mutate(bad)
            with self.assertRaises(decode.InvalidCapture):
                decode.decode(encoded(bad))
        with self.assertRaises(decode.InvalidCapture):
            decode.decode(encoded(rows[:-1]))
        with self.assertRaises(decode.InvalidCapture):
            decode.decode(self.empty.replace(b'"complete":true', b'"complete":true,"complete":false'))

    def test_count_raw_projection_and_order_corruption_refuse(self):
        rows = decode.load_records(self.nonempty)
        edits = [('list_header', 'count', False),
                 ('list_header', 'raw', '00' * 24),
                 ('list_item', 'next', '0x30000000'),
                 ('pending_action', 'raw', '00' * 12),
                 ('pending_action', 'ordinal', 1),
                 ('vector_header', 'end', '0xffffffffffffffff')]
        for kind, key, value in edits:
            bad = copy.deepcopy(rows)
            next(row for row in bad if row['kind'] == kind)[key] = value
            with self.subTest(kind=kind, key=key), self.assertRaises(decode.InvalidCapture):
                decode.decode(encoded(bad))
        bad = copy.deepcopy(rows)
        bad[1], bad[2] = bad[2], bad[1]
        with self.assertRaises(decode.InvalidCapture):
            decode.decode(encoded(bad))

    def test_limits_and_pointer_overflow_are_bounded(self):
        with self.assertRaises(decode.InvalidCapture):
            decode.decode(b' ' * (decode.MAX_BYTES + 1))
        rows = decode.load_records(self.empty)
        rows[0]['core_base'] = '0xffffffffffffffff'
        with self.assertRaises(decode.InvalidCapture):
            decode.decode(encoded(rows))
        rows = decode.load_records(self.empty)
        rows[-1]['read_bytes'] = 8 * 1024 * 1024 + 1
        with self.assertRaises(decode.InvalidCapture):
            decode.decode(encoded(rows))


if __name__ == '__main__':
    unittest.main()
