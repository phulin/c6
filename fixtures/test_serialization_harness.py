"""Transport/comparison tests only: these do not claim native codec parity."""
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import serialization as harness


class HarnessTests(unittest.TestCase):
    def test_first_difference_in_state_and_prefix_cursor(self):
        self.assertEqual(harness.difference({'state': {'lock_count': 1}}, {'state': {'lock_count': 2}}),
                         ('/state/lock_count', 1, 2))
        self.assertEqual(harness.difference({'consumed': 18}, {'consumed': 19}), ('/consumed', 18, 19))
        self.assertEqual(harness.difference({'bytes_hex': '01'}, {'bytes_hex': '02'}), ('/bytes_hex', '01', '02'))
        self.assertIsNone(harness.difference({'a': 1, 'b': 2}, {'b': 2, 'a': 1}))
        self.assertIsNotNone(harness.difference({}, {'state': None}))

    def test_bounded_stdout_and_stderr(self):
        for stream in ['stdout', 'stderr']:
            with self.subTest(stream=stream), self.assertRaisesRegex(ValueError, stream + ' output limit'):
                harness.bounded_process([sys.executable, '-c', f'import sys;sys.{stream}.write("x"*65536)'], output_limit=1024)

    def test_deadline_kills_inherited_pipe_holder(self):
        with self.assertRaisesRegex(ValueError, 'deadline'):
            harness.bounded_process([sys.executable, '-c', 'import subprocess;subprocess.Popen(["sleep","10"])'], timeout=0.1)

    def test_duplex_input_does_not_deadlock(self):
        script = 'import sys;sys.stdout.write("x"*65536);sys.stdout.flush();print(len(sys.stdin.read()))'
        stdout, stderr = harness.bounded_process([sys.executable, '-c', script], data='y'*262144)
        self.assertTrue(stdout.endswith('262144\n'))
        self.assertEqual(stderr, '')

    def test_missing_process_and_nonzero_exit_rejected(self):
        with self.assertRaises(OSError):
            harness.bounded_process(['/definitely/missing/c6-serialization'])
        with self.assertRaises(harness.subprocess.CalledProcessError):
            harness.bounded_process([sys.executable, '-c', 'raise SystemExit(3)'])


if __name__ == '__main__':
    unittest.main()
