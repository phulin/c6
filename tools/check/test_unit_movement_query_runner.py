#!/usr/bin/env python3
"""Process-only mocks: never launch GDB, the host or original game code."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'linux'))
import unit_movement_query_runner as runner


class Process:
    pid = 4000000
    args = ['mock-gdb']

    def __init__(self, directory, digest, exitcode=0, gdbcode=0, status=None, waiting=False):
        self.directory, self.digest = directory, digest
        self.exitcode, self.gdbcode, self.status = exitcode, gdbcode, status
        self.waiting, self.returncode, self.killed, self.waited = waiting, None, False, False

    def poll(self):
        return self.returncode

    def kill(self):
        self.killed = True
        self.returncode = -9

    def wait(self, timeout=None):
        self.waited = True
        if self.killed:
            return self.returncode
        if self.waiting:
            raise subprocess.TimeoutExpired(self.args, timeout)
        status = dict(schema='c6-linux-unit-movement-query-exit-v1',
                      configuration_sha256=self.digest, exit_event_count=1,
                      inferior_exitcode=self.exitcode, script_complete=True)
        if self.status is not None:
            status.update(self.status)
        (self.directory / 'exit.json').write_text(json.dumps(status))
        self.returncode = self.gdbcode
        return self.returncode


class RunnerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.directory = self.root / 'capture'
        self.directory.mkdir()
        self.modules = {}
        for role, filename in [('host', 'host'), ('preload', 'preload.so'),
                               ('core', 'libGameCore_XP2.so'), ('libc', 'libc.so.6')]:
            path = self.root / filename
            path.write_text(role)
            stat = path.stat()
            self.modules[role] = dict(path=str(path), sha256=runner.sha(path),
                                     device=[os.major(stat.st_dev), os.minor(stat.st_dev)], inode=stat.st_ino)
        self.env = dict(PATH='/mock/bin', LD_PRELOAD=self.modules['preload']['path'],
                        LD_LIBRARY_PATH='/admitted/search', C6_HOST_ASSETS=str(self.root), C6_MODE='movement')
        self.prepared = dict(schema='c6-linux-native-profile-prepared-v1', native_execution_performed=False,
                             returncode=None, configuration=dict(cwd=str(self.root), timeout_seconds=5,
                             lower_core_unit_movement_query=dict(mode='direct_original_Unit_movement_queries',
                             runtime_environment={k: v for k, v in self.env.items() if k.startswith(('C6_', 'LD_'))},
                             runtime_module_paths={r: d['path'] for r, d in self.modules.items()},
                             runtime_module_identities=self.modules)))
        self.freeze()
        self.debugger = str(self.root / 'gdb')
        self.identity = dict(pid=Process.pid, state='S', pgrp=Process.pid, session=Process.pid,
                             ppid=1, starttime=19, exe=self.debugger)
        self.ldd = patch.object(runner.subprocess, 'check_output', return_value='libc.so.6 => ' + self.modules['libc']['path'] + ' (0x123)')
        self.ldd_mock = self.ldd.start()
        self.addCleanup(self.ldd.stop)
        self.which = patch.object(runner.shutil, 'which', return_value=self.debugger)
        self.which.start()
        self.addCleanup(self.which.stop)
        self.popen = patch.object(runner.subprocess, 'Popen')
        self.launch = self.popen.start()
        self.addCleanup(self.popen.stop)
        self.proc_identity = patch.object(runner, 'process_identity', return_value=self.identity)
        self.observe = self.proc_identity.start()
        self.addCleanup(self.proc_identity.stop)
        self.sessions = patch.object(runner, 'session_processes', return_value=[])
        self.session = self.sessions.start()
        self.addCleanup(self.sessions.stop)
        self.disposal = patch.object(runner, 'dispose_session', return_value=dict(ownership_proven=True, native_cleanup_proven=False))
        self.dispose = self.disposal.start()
        self.addCleanup(self.disposal.stop)

    def freeze(self):
        digest = hashlib.sha256(json.dumps(self.prepared['configuration'], sort_keys=True, separators=(',', ':')).encode()).hexdigest()
        self.prepared['configuration_sha256'] = digest
        self.env[runner.PROFILE_KEY] = digest
        (self.directory / 'runtime-prepared.json').write_text(json.dumps(self.prepared))

    def run_case(self, **kwargs):
        process = Process(self.directory, self.prepared['configuration_sha256'], **kwargs)
        self.launch.return_value = process
        result = runner.run(self.modules['host']['path'], self.env, self.root, 5,
                            self.directory, self.directory / 'runtime-prepared.json')
        return result, process

    def test_normal_exit_and_plain_script(self):
        result, process = self.run_case()
        self.assertEqual(result[0], 0)
        self.assertTrue(process.waited)
        self.dispose.assert_not_called()
        script = (self.directory / 'capture.gdb').read_text()
        self.assertTrue(script.startswith('python\n'))
        self.assertLess(script.index('supervisor launch identity differs'), script.index('\nrun\n'))
        for forbidden in ('starti', 'Breakpoint', 'break ', 'observer', 'lua', 'full_manager'):
            self.assertNotIn(forbidden, script)
        launch = self.launch.call_args.kwargs
        self.assertTrue(launch['start_new_session'])
        self.assertFalse(any(k.startswith(('C6_', 'LD_')) for k in launch['env']))
        self.assertEqual(self.ldd_mock.call_args.kwargs['env']['LD_LIBRARY_PATH'], '/admitted/search')
        self.assertNotIn('LD_PRELOAD', self.ldd_mock.call_args.kwargs['env'])
        supervisor = json.loads((self.directory / 'supervisor.json').read_text())
        self.assertEqual(supervisor['observed_session_processes'], [self.identity])
        self.assertFalse(supervisor['native_cleanup_proven'])

    def test_native_nonzero_is_preserved(self):
        self.assertEqual(self.run_case(exitcode=240)[0][0], 240)
        self.dispose.assert_not_called()

    def test_timeout_retains_identity_and_disposes(self):
        with patch.object(runner.time, 'monotonic', side_effect=[0, 0, 6]):
            result, _ = self.run_case(waiting=True)
        self.assertEqual(result[0], 124)
        self.assertEqual(self.dispose.call_args.args[2], 'supervisor_timeout')
        self.assertEqual(list(self.dispose.call_args.args[3]), [self.identity])

    def test_initial_missing_identity_only_owned_child(self):
        self.observe.return_value = None
        with patch.object(runner.os, 'pidfd_open', side_effect=OSError):
            result, process = self.run_case()
        self.assertEqual(result[0], 239)
        self.assertTrue(process.killed and process.waited)
        self.assertFalse((self.directory / 'launch-authorized.json').exists())
        self.dispose.assert_not_called()
        self.session.assert_not_called()

    def test_initial_wrong_executable_only_owned_child(self):
        self.observe.return_value = dict(self.identity, exe='/unrelated')
        with patch.object(runner.os, 'pidfd_open', side_effect=OSError):
            result, process = self.run_case()
        self.assertEqual(result[0], 239)
        self.assertTrue(process.killed)
        self.dispose.assert_not_called()

    def test_leader_change_is_protocol_failure(self):
        self.observe.side_effect = [self.identity, dict(self.identity, starttime=20)]
        result, _ = self.run_case()
        self.assertEqual(result[0], 239)
        self.assertEqual(self.dispose.call_args.args[1], self.identity)

    def test_remaining_descendant_disposed_and_retained(self):
        descendant = dict(self.identity, pid=Process.pid+1, starttime=21, exe='/child')
        self.session.return_value = [descendant]
        result, _ = self.run_case()
        self.assertEqual(result[0], 239)
        supervisor = json.loads((self.directory / 'supervisor.json').read_text())
        self.assertIn(descendant, supervisor['observed_session_processes'])
        self.dispose.assert_called_once()

    def test_bad_exit_protocols(self):
        cases = [dict(gdbcode=1), dict(status={'exit_event_count': 2}),
                 dict(status={'inferior_exitcode': None}), dict(status={'inferior_exitcode': True}),
                 dict(status={'script_complete': False}), dict(status={'configuration_sha256': 'wrong'}),
                 dict(status={'schema': 'wrong'}), dict(status={'inferior_exitcode': 256})]
        for i, case in enumerate(cases):
            with self.subTest(case=case):
                if i:
                    for path in self.directory.iterdir():
                        if path.name != 'runtime-prepared.json':
                            path.unlink()
                    self.dispose.reset_mock()
                self.assertEqual(self.run_case(**case)[0][0], 239)
                self.dispose.assert_called_once()

    def test_missing_exit_file_disposes(self):
        process = Process(self.directory, self.prepared['configuration_sha256'])
        def wait(timeout=None):
            process.returncode = 0
        process.wait = wait
        self.launch.return_value = process
        result = runner.run(self.modules['host']['path'], self.env, self.root, 5,
                            self.directory, self.directory / 'runtime-prepared.json')
        self.assertEqual(result[0], 239)
        self.dispose.assert_called_once()

    def assert_refused(self):
        with self.assertRaises((ValueError, KeyError)):
            self.run_case()
        self.launch.assert_not_called()

    def test_disposal_refusal_is_retained_without_weaker_retry(self):
        self.dispose.side_effect = ValueError('ownership changed during disposal')
        result, _ = self.run_case(gdbcode=1)
        self.assertEqual(result[0], 239)
        self.dispose.assert_called_once()
        disposal = json.loads((self.directory / 'disposal.json').read_text())
        self.assertFalse(disposal['ownership_proven'])
        self.assertFalse(disposal['no_executing_processes'])

    def test_environment_whitespace_refused(self):
        self.prepared['configuration']['lower_core_unit_movement_query']['runtime_environment']['C6_MODE'] = ' movement '
        self.env['C6_MODE'] = ' movement '
        self.freeze()
        self.assert_refused()

    def test_environment_drift_refused(self):
        self.env['C6_UNRECORDED'] = '1'
        self.assert_refused()

    def test_environment_digest_drift_refused(self):
        self.env[runner.PROFILE_KEY] = 'wrong'
        self.assert_refused()

    def test_stale_directory_refused(self):
        (self.directory / 'launch-authorized.json').write_text('{}')
        self.assert_refused()

    def test_hash_drift_refused_before_ldd(self):
        Path(self.modules['host']['path']).write_text('changed')
        self.assert_refused()
        self.ldd_mock.assert_not_called()

    def test_module_identity_fields_refused(self):
        for role, field, value in [('preload', 'inode', -1), ('core', 'device', [0, 0]),
                                   ('libc', 'sha256', 'wrong'), ('host', 'path', '/wrong')]:
            with self.subTest(role=role, field=field):
                original = self.modules[role][field]
                self.modules[role][field] = value
                self.freeze()
                self.assert_refused()
                self.modules[role][field] = original

    def test_ambiguous_libc_refused(self):
        self.ldd_mock.return_value *= 2
        self.assert_refused()

    def test_missing_dependency_refused(self):
        self.ldd_mock.return_value += '\nlibmissing.so => not found'
        self.assert_refused()

    def test_cwd_drift_refused(self):
        self.prepared['configuration']['cwd'] = '/different'
        self.freeze()
        self.assert_refused()

    def test_bad_preparation_or_timeout_refused(self):
        self.prepared['configuration']['timeout_seconds'] = 6
        self.freeze()
        self.assert_refused()


if __name__ == '__main__':
    unittest.main()
