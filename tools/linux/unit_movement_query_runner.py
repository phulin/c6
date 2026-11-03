#!/usr/bin/env python3
"""Independent movement-query process supervision; no execution on import.

The authorization barrier follows full_manager_driver.py; only the pure /proc
identity/disposal helpers are reused. No Manager observer, Lua or target
breakpoint is installed. Process disposal never establishes native cleanup.
"""
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import time

from military_observer_runner import dispose_session, process_identity, session_processes

PROFILE_KEY = 'C6_UNIT_MOVEMENT_QUERY_PROFILE_SHA256'
IDENTITY_KEYS = ('pid', 'session', 'starttime', 'exe')
ROLES = {'host', 'preload', 'core', 'libc'}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write_json(path, value):
    with Path(path).open('x') as stream:
        stream.write(json.dumps(value, indent=2) + '\n')


def launch_paths(host, env, work, timeout, prepared):
    """Validate actual inputs before creating a debugger or authorization."""
    require(prepared.get('schema') == 'c6-linux-native-profile-prepared-v1', 'prepared schema differs')
    require(prepared.get('native_execution_performed') is False and
            'returncode' in prepared and prepared['returncode'] is None, 'report is not an unused preparation')
    cfg = prepared['configuration']
    digest = hashlib.sha256(json.dumps(cfg, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    require(prepared['configuration_sha256'] == digest, 'configuration digest differs')
    profile = cfg['lower_core_unit_movement_query']
    require(profile['mode'] == 'direct_original_Unit_movement_queries', 'movement profile mode differs')
    require(math.isfinite(timeout) and timeout > 0 and timeout == cfg['timeout_seconds'], 'timeout differs')
    require(str(Path(work).resolve()) == str(Path(cfg['cwd']).resolve()), 'actual debugger working directory differs')
    expected = dict(profile['runtime_environment'])
    require(PROFILE_KEY not in expected, 'profile digest must be derived')
    require(all(isinstance(k, str) and k.startswith(('C6_', 'LD_')) and isinstance(v, str)
                for k, v in expected.items()), 'invalid recorded environment')
    expected[PROFILE_KEY] = digest
    actual_env = {k: v for k, v in env.items() if k.startswith(('C6_', 'LD_'))}
    require(actual_env == expected, 'actual runtime environment differs')
    for key, value in actual_env.items():
        require(re.fullmatch(r'[A-Za-z_][A-Za-z0-9_]*', key) and
                not any(c in value for c in '\n\r\0') and value == value.strip(),
                'unsafe debugger environment')
    paths = profile['runtime_module_paths']
    identities = profile['runtime_module_identities']
    require(set(paths) == set(identities) == ROLES, 'exact four-module identity required')
    actual = {'host': str(Path(host).resolve()),
              'preload': str(Path(env['LD_PRELOAD']).resolve()),
              'core': str((Path(env['C6_HOST_ASSETS']) / 'libGameCore_XP2.so').resolve())}
    def validate_module(role, path):
        declared = identities[role]
        require(path == paths[role] == declared['path'], 'actual module path differs: ' + role)
        stat = Path(path).stat()
        require(sha(path) == declared['sha256'], 'actual module hash differs: ' + role)
        require(stat.st_ino == declared['inode'] and
                [os.major(stat.st_dev), os.minor(stat.st_dev)] == declared['device'],
                'actual module device/inode differs: ' + role)

    # Gate the host before even invoking ldd on it. No preload/profile variables
    # enter ldd or its interpreter; use the exact admitted loader search path.
    for role, path in actual.items():
        validate_module(role, path)
    dependency_env = {k: v for k, v in env.items() if not k.startswith(('C6_', 'LD_'))}
    dependency_env['LD_LIBRARY_PATH'] = env['LD_LIBRARY_PATH']
    listing = subprocess.check_output(['ldd', str(host)], env=dependency_env, text=True)
    matches = re.findall(r'libc\.so\.6\s+=>\s+(/\S+)\s+\(', listing)
    require(len(matches) == 1 and 'not found' not in listing, 'actual loader dependency resolution unknown')
    actual['libc'] = str(Path(matches[0]).resolve())
    validate_module('libc', actual['libc'])
    return digest


def stop_unadmitted_child(process):
    """Only the unreaped owned debugger; its PID cannot yet be recycled."""
    if process.poll() is None:
        descriptor = None
        try:
            if hasattr(os, 'pidfd_open') and hasattr(signal, 'pidfd_send_signal'):
                descriptor = os.pidfd_open(process.pid)
                signal.pidfd_send_signal(descriptor, signal.SIGKILL)
            else:
                process.kill()
        except ProcessLookupError:
            pass
        except OSError:
            if process.poll() is None:
                process.kill()
        finally:
            if descriptor is not None:
                os.close(descriptor)
    process.wait()


def debugger_script(env, directory, timeout, digest):
    """The very first command waits for parent-observed debugger identity."""
    authorization = str(directory / 'launch-authorized.json')
    output = str(directory / 'exit.json')
    lines = ['python', 'import os,json,time,gdb',
             '_approval=' + repr(authorization),
             '_deadline=time.monotonic()+' + repr(float(timeout)),
             'while not os.path.exists(_approval):',
             ' if time.monotonic()>=_deadline: raise RuntimeError("supervisor authorization absent")',
             ' time.sleep(0.01)',
             '_authorized=json.load(open(_approval))',
             '_stat=open("/proc/self/stat").read().rsplit(")",1)[1].split()',
             'assert _authorized["pid"]==os.getpid() and _authorized["session"]==os.getsid(0) and _authorized["starttime"]==int(_stat[19]) and _authorized["exe"]==os.path.realpath("/proc/self/exe"), "supervisor launch identity differs"',
             '_exits=[]',
             'def _on_exit(event):',
             ' _exits.append(getattr(event,"exit_code",None))',
             'gdb.events.exited.connect(_on_exit)', 'end',
             'set pagination off', 'set confirm off', 'set debuginfod enabled off',
             'set non-stop off', 'set startup-with-shell off']
    for key, value in sorted(env.items()):
        if key.startswith(('C6_', 'LD_')):
            lines.append('set environment ' + key + ' = ' + value)
    lines += ['run', 'python',
              '_result=dict(schema="c6-linux-unit-movement-query-exit-v1",configuration_sha256=' + repr(digest) + ',exit_event_count=len(_exits),inferior_exitcode=_exits[0] if len(_exits)==1 else None,script_complete=True)',
              'with open(' + repr(output) + ',"x") as _stream: json.dump(_result,_stream)', 'end']
    return '\n'.join(lines) + '\n'


def run(host, env, work, timeout, directory, prepared_report):
    """Return (inferior code, stdout, stderr); 124 timeout, 239 protocol failure.

    The CLI owns immutable preparation/expected-profile admission. Here actual
    paths, file identities, loader resolution, environment and cwd are rebound.
    """
    directory = Path(directory).resolve()
    require(directory.is_dir(), 'movement directory absent')
    require({p.name for p in directory.iterdir()} == {'runtime-prepared.json'}, 'movement directory must be fresh')
    prepared_path = Path(prepared_report).resolve()
    require(prepared_path == directory / 'runtime-prepared.json', 'prepared report is not the retained copy')
    prepared = json.loads(prepared_path.read_text())
    digest = launch_paths(host, env, work, timeout, prepared)
    debugger = shutil.which('gdb', path=env.get('PATH', os.defpath))
    require(debugger is not None, 'gdb unavailable')
    debugger = str(Path(debugger).resolve())
    command = directory / 'capture.gdb'
    with command.open('x') as stream:
        stream.write(debugger_script(env, directory, timeout, digest))
    transcript = directory / 'transcript.txt'
    process = None
    identity = None
    owned = {}
    supervisor = dict(schema='c6-linux-unit-movement-query-supervisor-v1',
                      configuration_sha256=digest, prepared_sha256=sha(prepared_path),
                      original_launch_authorized=False, native_cleanup_proven=False,
                      native_cleanup='unavailable; process disposal is not native cleanup')

    def record():
        supervisor['observed_session_processes'] = list(owned.values())
        (directory / 'supervisor.json').write_text(json.dumps(supervisor, indent=2) + '\n')

    def dispose(reason):
        try:
            status = dispose_session(process, identity, reason, owned.values())
        except Exception as error:
            # Ownership refusal or an OS failure is evidence, never permission
            # to retry signals using weaker identity checks.
            status = dict(reason=reason, disposal_error=str(error),
                          ownership_proven=False, no_executing_processes=False,
                          native_cleanup_proven=False)
        write_json(directory / 'disposal.json', status)
        supervisor['disposal'] = status

    try:
        with transcript.open('x') as stream:
            child_env = {k: v for k, v in env.items() if not k.startswith(('C6_', 'LD_'))}
            process = subprocess.Popen([debugger, '-nx', '-batch', '-x', str(command), '--args', str(Path(host).resolve())],
                                       cwd=work, env=child_env, stdout=stream,
                                       stderr=subprocess.STDOUT, start_new_session=True)
            candidate = process_identity(process.pid)
            supervisor['initial_debugger_observation'] = candidate
            if not (candidate and candidate['pid'] == process.pid and
                    candidate['session'] == process.pid and candidate['pgrp'] == process.pid and
                    candidate['exe'] == debugger and candidate['state'] not in ('Z', 'X') and
                    type(candidate['starttime']) is int and candidate['starttime'] > 0):
                stop_unadmitted_child(process)
                write_json(directory / 'disposal.json', dict(reason='initial_debugger_identity_unavailable_or_wrong',
                           direct_child_waited=True, original_launch_authorized=False, native_cleanup_proven=False))
                supervisor['failure'] = 'initial debugger identity unavailable or wrong'
                record()
                return 239, '', transcript.read_text(errors='replace')
            identity = candidate
            owned[(identity['pid'], identity['starttime'])] = identity
            supervisor['launched'] = identity
            record()
            temporary = directory / 'launch-authorized.tmp'
            with temporary.open('x') as approval:
                approval.write(json.dumps(identity) + '\n')
                approval.flush()
                os.fsync(approval.fileno())
            os.link(temporary, directory / 'launch-authorized.json')
            temporary.unlink()
            supervisor['original_launch_authorized'] = True
            deadline = time.monotonic() + timeout
            while True:
                current = process_identity(process.pid)
                if current is not None:
                    require(all(current[k] == identity[k] for k in IDENTITY_KEYS), 'debugger session leader changed')
                for row in session_processes(process.pid):
                    owned[(row['pid'], row['starttime'])] = row
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    dispose('supervisor_timeout')
                    supervisor['returncode'] = 124
                    record()
                    return 124, '', transcript.read_text(errors='replace')
                try:
                    process.wait(timeout=min(1.0, remaining))
                    break
                except subprocess.TimeoutExpired:
                    pass
        status = json.loads((directory / 'exit.json').read_text())
        alive = [r for r in session_processes(process.pid) if r['state'] not in ('Z', 'X')]
        for row in alive:
            owned[(row['pid'], row['starttime'])] = row
        valid = (process.returncode == 0 and not alive and
                 status.get('schema') == 'c6-linux-unit-movement-query-exit-v1' and
                 status.get('configuration_sha256') == digest and status.get('script_complete') is True and
                 type(status.get('exit_event_count')) is int and status['exit_event_count'] == 1 and
                 type(status.get('inferior_exitcode')) is int and 0 <= status['inferior_exitcode'] <= 255)
        code = status['inferior_exitcode'] if valid else 239
        if not valid:
            dispose('debugger_protocol_failure_or_remaining_inferior')
        supervisor.update(terminal=status, gdb_returncode=process.returncode, returncode=code)
        record()
        return code, '', transcript.read_text(errors='replace')
    except Exception as error:
        supervisor['failure'] = str(error)
        if identity is not None:
            dispose('debugger_or_protocol_exception')
        elif process is not None:
            stop_unadmitted_child(process)
        supervisor['returncode'] = 239
        record()
        return 239, '', transcript.read_text(errors='replace') if transcript.exists() else str(error)
    except BaseException:
        if identity is not None:
            dispose('caller_interrupted')
        elif process is not None:
            stop_unadmitted_child(process)
        record()
        raise
