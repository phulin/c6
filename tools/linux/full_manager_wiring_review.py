#!/usr/bin/env python3
"""Offline FULL CLI/driver review. Never launches GDB or original game code."""
import argparse
import ast
import contextlib
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/linux'))
import full_manager_driver as driver
import mod_activation_probe as probe


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cli_cases():
    """Execute actual main's pure validation prefix, before asset/compile work."""
    tree = ast.parse((ROOT / 'tools/linux/mod_activation_probe.py').read_text())
    function = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'main')
    boundary = next(i for i, n in enumerate(function.body)
                    if isinstance(n, ast.Assign) and any(isinstance(t, ast.Name) and t.id == 'gate' for t in n.targets))
    function.body = function.body[:boundary] + [ast.Return(value=ast.Call(func=ast.Name(id='locals', ctx=ast.Load()), args=[], keywords=[]))]
    module = ast.fix_missing_locations(ast.Module(body=[function], type_ignores=[]))
    namespace = dict(vars(probe))
    exec(compile(module, '<actual-CLI-validation-prefix>', 'exec'), namespace)
    rows = []
    labels = ['prepare_no_saves', 'prepare_all_saves', 'build_only_without_plan', 'execution_without_expected',
              'prepare_without_plan', 'partial_internal_saves', 'partial_outer_saves', 'outer_path_alias',
              'actor_bool_owner', 'actor_extra_answer', 'actor_duplicate_key', 'actor_invalid_id',
              'mixed_action_mode', 'passive_observer_mode', 'legacy_requires_outer_saves', 'orphan_full_arguments']
    for label in labels:
        with tempfile.TemporaryDirectory(prefix='c6-full-cli-review-') as temporary:
            root = Path(temporary);project = root / 'c6';project.mkdir()
            private = root / 'c6-ghidra';private.mkdir();observer = private / 'observer';observer.mkdir()
            trace = private / 'trace';trace.mkdir();game = root / 'core';game.touch()
            with game.open('r+b') as stream:stream.truncate(76556938)
            actor = root / 'actor.json'
            actor.write_text(json.dumps(dict(schema='c6-linux-full-manager-predicate-input-v1', actor_owner=7,
                                            actor_full_id=1179649, visible_only=True)))
            if label == 'actor_bool_owner':actor.write_text(actor.read_text().replace('"actor_owner": 7', '"actor_owner": true'))
            if label == 'actor_extra_answer':actor.write_text(actor.read_text()[:-1] + ',"expected":true}')
            if label == 'actor_duplicate_key':actor.write_text(actor.read_text()[:-1] + ',"actor_owner":8}')
            if label == 'actor_invalid_id':actor.write_text(actor.read_text().replace('1179649', '4294967295'))
            args = ['probe', '--output', str(root / 'report'), '--launch-host', '/not-executed',
                    '--game-load-stream', str(game), '--prepare-main-thread', '--cleanup', '--attach-saved-session',
                    '--network', '--saved-configuration', '/unused', '--update-constructor-callback', '--ui-constructor',
                    '--configure', '--localization', '--content-paths', '--art-manager', '--action-trace-directory', str(trace),
                    '--full-manager-fixture', '--full-manager-input', str(actor), '--full-manager-observer-directory', str(observer)]
            for name in ['output', 'controls', 'metadata-before', 'metadata-control', 'metadata-after', 'metadata-repeat']:
                args += ['--full-manager-' + name, str(private / name)]
            if label != 'execution_without_expected':args += ['--prepare-only']
            if label not in ('prepare_without_plan', 'build_only_without_plan'):args += ['--full-manager-observer-plan', str(root / 'plan')]
            if label == 'build_only_without_plan':args += ['--full-manager-build-only']
            if label in ('prepare_all_saves', 'partial_internal_saves'):
                for name in (['before'] if label == 'partial_internal_saves' else ['before', 'control', 'after', 'repeat']):
                    args += ['--full-manager-snapshot-' + name, str(private / ('save-' + name))]
            if label in ('prepare_all_saves', 'partial_outer_saves', 'outer_path_alias'):
                args += ['--game-save-output', str(private / ('output' if label == 'outer_path_alias' else 'outer-before'))]
                if label != 'partial_outer_saves':args += ['--game-save-repeat-output', str(private / 'outer-after')]
            if label == 'mixed_action_mode':args += ['--action-skip-turn']
            if label == 'passive_observer_mode':args += ['--game-load-observe']
            if label == 'legacy_requires_outer_saves':
                start = args.index('--full-manager-fixture');args = args[:start]
                args.remove('--action-trace-directory');args.remove(str(trace))
            if label == 'orphan_full_arguments':args.remove('--full-manager-fixture')
            namespace['ROOT'] = project
            namespace['digest'] = lambda path: 'a66736f0423e9a59b11c77b1879b471d936fe3887342df5008940f763dbcc5aa'
            accepted = False
            with patch.object(sys, 'argv', args), contextlib.redirect_stderr(io.StringIO()):
                try:
                    result = namespace['main']();accepted = True
                    assert result['args'].prepare_only
                    assert len(result['game_outputs']) == (2 if label == 'prepare_all_saves' else 0)
                except (SystemExit, ValueError):pass
            expected = label in ('prepare_no_saves', 'prepare_all_saves', 'build_only_without_plan')
            if accepted != expected:raise AssertionError((label, accepted))
            rows.append(dict(case=label, accepted=accepted, passed=True))
    return rows


def driver_cases():
    rows = []
    for mode in ['clean', 'nonzero_inferior', 'observer_failed', 'no_footer', 'malformed_footer',
                 'timeout', 'remaining', 'zombie_only', 'bool_exitcode', 'gdb_failure',
                 'plan_drift', 'unsafe_environment', 'capture_exists']:
        with tempfile.TemporaryDirectory(prefix='c6-full-driver-review-') as temporary:
            root = Path(temporary);out = root / 'observer';out.mkdir()
            plan = root / 'plan';plan.write_text('{}');prepared = root / 'prepared'
            prepared.write_text(json.dumps(dict(configuration=dict(sources={'tools/linux/full_manager_driver.py':'frozen'},
                runner_sha256='runner', lower_core_full_manager_observer=dict(static_plan={}, static_plan_sha256=driver.sha(plan))),
                configuration_sha256='profile')))
            if mode == 'plan_drift':plan.write_text('{"changed":true}')
            if mode == 'capture_exists':(out / 'capture').mkdir()
            identity = dict(pid=123, starttime=44, session=123, exe='gdb')
            disposals = [];launches = [];session_calls = [0]
            class Child:
                pid = 123;args = ['gdb'];returncode = 1 if mode == 'gdb_failure' else 0
                def wait(self, timeout):
                    if mode == 'timeout':raise subprocess.TimeoutExpired(self.args, timeout)
                    return self.returncode
            def launch(*args, **kwargs):
                launches.append(True)
                assert kwargs['start_new_session'] is True
                assert not any(k.startswith(('C6_', 'LD_')) for k in kwargs['env'])
                command = (out / 'capture.gdb').read_text()
                assert repr(str(out / 'capture')) in command and 'starti' in command
                if mode != 'no_footer':
                    (out / 'capture').mkdir()
                    (out / 'capture/exit.json').write_text('{' if mode == 'malformed_footer' else json.dumps(dict(
                        observer_failed=mode == 'observer_failed', inferior_exitcode=True if mode == 'bool_exitcode' else 17 if mode == 'nonzero_inferior' else 0)))
                return Child()
            def dispose(process, actual, reason, members):
                assert actual == identity;disposals.append(reason);return dict(no_executing_processes=True)
            def members(pid):
                session_calls[0] += 1
                if mode in ('remaining', 'zombie_only') and session_calls[0] >= 2:
                    return [dict(pid=125, starttime=46, state='Z' if mode == 'zombie_only' else 'S')]
                return []
            exception = None;code = None
            with patch.object(driver.subprocess, 'Popen', launch), patch.object(driver, 'process_identity', lambda pid: identity), \
                 patch.object(driver, 'session_processes', members), patch.object(driver, 'dispose_session', dispose):
                try:code, _, _ = driver.run('/not-executed', {'C6_TEST':'bad\nvalue' if mode == 'unsafe_environment' else '1'}, root,
                                            0 if mode == 'timeout' else 10, out, prepared, plan)
                except (ValueError, json.JSONDecodeError) as error:exception = type(error).__name__
            preflight = mode in ('plan_drift', 'unsafe_environment', 'capture_exists')
            if preflight:assert exception and not launches and not disposals
            elif mode == 'malformed_footer':assert exception and disposals == ['caller_exception']
            else:
                expected = 0 if mode in ('clean', 'zombie_only') else 17 if mode == 'nonzero_inferior' else 124 if mode == 'timeout' else 239
                assert code == expected, (mode, code)
                assert bool(disposals) == (mode in ('observer_failed', 'no_footer', 'timeout', 'remaining', 'gdb_failure'))
            rows.append(dict(case=mode, code=code, exception=exception, disposal=disposals, passed=True))
    return rows


def launch_barrier_cases():
    """Execute only the generated supervisor Python barrier in a harmless child."""
    rows = []
    real_popen = subprocess.Popen
    real_identity = driver.process_identity
    for mode in ('valid_identity', 'missing_identity', 'wrong_session'):
        with tempfile.TemporaryDirectory(prefix='c6-full-barrier-review-') as temporary:
            root = Path(temporary);out = root / 'observer';out.mkdir()
            plan = root / 'plan';plan.write_text('{}');prepared = root / 'prepared'
            prepared.write_text(json.dumps(dict(configuration=dict(sources={}, runner_sha256='runner',
                lower_core_full_manager_observer=dict(static_plan={}, static_plan_sha256=driver.sha(plan))),
                configuration_sha256='profile')))
            children = []
            def launch(*args, **kwargs):
                lines = (out / 'capture.gdb').read_text().splitlines()
                assert lines[0] == 'python' and lines.index('end') < lines.index('starti')
                block = '\n'.join(lines[1:lines.index('end')])
                block += '\nfrom pathlib import Path\n'
                block += 'Path(' + repr(str(out / 'past-barrier')) + ').write_text("authorized")\n'
                block += 'Path(' + repr(str(out / 'capture')) + ').mkdir()\n'
                block += 'Path(' + repr(str(out / 'capture/exit.json')) + ').write_text(\'{"observer_failed":false,"inferior_exitcode":0}\')\n'
                child = real_popen([sys.executable, '-c', block], **kwargs)
                children.append(child);return child
            def identity(pid):
                if mode == 'missing_identity':return None
                result = real_identity(pid)
                if mode == 'wrong_session' and result:result['session'] += 1
                return result
            refused = False;code = None
            try:
                with patch.object(driver.subprocess, 'Popen', launch), patch.object(driver, 'process_identity', identity):
                    try:code, _, _ = driver.run('/not-executed', {}, root, 5, out, prepared, plan)
                    except ValueError:refused = True
                assert len(children) == 1 and children[0].poll() is not None
                if mode == 'valid_identity':
                    assert code == 0 and not refused and (out / 'past-barrier').exists()
                    approval = json.loads((out / 'launch-authorized.json').read_text())
                    assert approval['pid'] == children[0].pid and approval['session'] == children[0].pid
                else:
                    assert refused and not (out / 'past-barrier').exists()
                    assert not (out / 'launch-authorized.json').exists()
                    assert json.loads((out / 'disposal.json').read_text())['original_launch_authorized'] is False
                rows.append(dict(case=mode, child_waited=True, passed_barrier=mode == 'valid_identity', passed=True))
            finally:
                for child in children:
                    if child.poll() is None:child.kill();child.wait()
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__);parser.add_argument('--output', type=Path, required=True);args = parser.parse_args()
    paths = ['tools/linux/mod_activation_probe.py', 'tools/linux/full_manager_driver.py',
             'tools/linux/military_observer_runner.py', 'tools/linux/full_manager_observer_runner.py',
             'tools/linux/full_manager_wiring_review.py']
    before = {p:digest(ROOT / p) for p in paths}
    cli = cli_cases();supervisor = driver_cases();barrier = launch_barrier_cases()
    after = {p:digest(ROOT / p) for p in paths};assert before == after
    args.output.write_text(json.dumps(dict(schema='c6-linux-full-manager-wiring-independent-review-v1', source_sha256=after,
        reviewed_checkpoints=['91b0159', 'ddb293a'], source_stable=True, cli_validation_prefix_cases=cli,
        driver_cases=supervisor, harmless_python_launch_barrier_cases=barrier, original_execution=False,
        gdb_launched=False, full_runtime_admission=False, complete_gameplay_parity=False), indent=2, sort_keys=True) + '\n')
    print('16 CLI, 13 mock driver and 3 harmless Python launch-barrier cases passed')


if __name__ == '__main__':main()
