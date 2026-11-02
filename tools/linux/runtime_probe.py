#!/usr/bin/env python3
"""Hash-gated native ELF context lifetime diagnostic; no initialized game."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    h = hashlib.sha256()
    with open(path, 'rb') as source:
        for data in iter(lambda: source.read(1024 * 1024), b''):
            h.update(data)
    return h.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--sysroot', type=Path, default=ROOT / 'target/linux/sysroot')
    parser.add_argument('--expected-configuration')
    parser.add_argument('--initialize', action='store_true',
                        help='construct genuine host services and enter original Context Initialize')
    parser.add_argument('--startup', action='store_true',
                        help='also enter genuine StartupGame after Context Initialize')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.initialize = args.initialize or args.startup
    assets = args.assets.resolve()
    work = ROOT / 'target/linux/runtime'
    work.mkdir(parents=True, exist_ok=True)
    # Only genuine supplied libraries and genuine system libraries are providers.
    # No game assets or original executables are copied or modified.
    gate = json.loads((ROOT / 'tools/linux/runtime-gate.json').read_text())
    for name, expected in gate.items():
        if digest(assets / name) != expected:
            raise SystemExit(f'original asset SHA-256 mismatch: {name}')
    content_gate = None
    if args.initialize:
        content_gate = json.loads((ROOT / 'tools/linux/runtime-content-gate.json').read_text())
        for name, expected in content_gate['files'].items():
            if digest(assets / name) != expected:
                raise SystemExit(f'content source SHA-256 mismatch: {name}')
        actual_sources = {str(p.relative_to(assets)) for directory in
                          ['Base/assets/configuration/data', 'Base/assets/gameplay/data']
                          for p in (assets / directory).rglob('*') if p.is_file()}
        if actual_sources != set(content_gate['files']):
            raise SystemExit('content source inventory mismatch')
    library_path = f'{args.sysroot.resolve()}/usr/lib/x86_64-linux-gnu:{assets}'
    env = os.environ.copy()
    # Do not inherit arbitrary loader hooks into the diagnostic child.
    for name in list(env):
        if name.startswith('LD_') or name.startswith('C6_'):
            del env[name]
    env['LD_LIBRARY_PATH'] = library_path
    listing = subprocess.run(['ldd', str(assets / 'Civ6')], cwd=work,
                             env=env, capture_output=True, text=True, check=True).stdout
    if 'not found' in listing:
        raise SystemExit(f'unavailable native runtime dependency:\n{listing}')
    dependencies = {}
    for line in listing.splitlines():
        match = re.search(r'(?:=>\s+)?(/\S+)\s+\(', line)
        if match:
            path = Path(match[1]).resolve()
            dependencies[path.name] = {'path': str(path), 'sha256': digest(path)}
    source = ROOT / 'native/linux/host_probe.c'
    probe = work / 'host_probe.so'
    compiler = subprocess.check_output(['cc', '--version'], text=True).splitlines()[0]
    extra_sources = [ROOT / 'native/linux/context_initialize.c',
                     ROOT / 'native/linux/host_services.c'] if args.initialize else []
    subprocess.run(['cc', '-shared', '-fPIC', '-Wall', '-Wextra', '-Werror',
                    str(source), *map(str, extra_sources), '-ldl', '-o', str(probe)], check=True)
    configuration = {
        'schema': 'c6-linux-context-lifetime-v1',
        'original_assets': gate,
        'runtime_dependencies': dependencies,
        'probe_source_sha256': digest(source),
        'probe_sha256': digest(probe),
        'compiler': compiler,
        'mode': ('host-crt-init-original-host-services-context-initialize' if args.initialize
                 else 'host-crt-init-intercept-main-whole-gamecore-factory-3-cycles'),
        'rules_database': None, 'enabled_dlc_mods': None,
        'initialized_game': False,
        'host_application_main_called': False,
        'host_application_shutdown_called': False,
    }
    if args.initialize:
        configuration['content_profile'] = content_gate
        configuration['startup_game_requested'] = args.startup
        configuration['host_userdata'] = str(ROOT / 'target/linux-runtime/userdata')
        configuration['host_adapter_sources'] = {
            str(p.relative_to(ROOT)): digest(p) for p in
            [*extra_sources, ROOT / 'native/linux/host_services.h']}
    identity = hashlib.sha256(json.dumps(configuration, sort_keys=True,
                              separators=(',', ':')).encode()).hexdigest()
    if args.expected_configuration and args.expected_configuration != identity:
        raise SystemExit('configuration identity mismatch; original execution refused')
    env['LD_PRELOAD'] = str(probe)
    env['C6_GAMECORE'] = str(assets / 'libGameCore_XP2.so')
    if args.initialize:
        env['C6_INITIALIZE'] = '1'
        if args.startup:
            env['C6_STARTUP'] = '1'
        env['C6_HOST_ASSETS'] = str(assets)
        userdata = ROOT / 'target/linux-runtime/userdata'
        userdata.mkdir(parents=True, exist_ok=True)
        env['C6_HOST_USERDATA'] = str(userdata)
    try:
        run = subprocess.run([str(assets / 'Civ6')], cwd=work, env=env,
                             capture_output=True, text=True, timeout=30)
        code, out, err = run.returncode, run.stdout, run.stderr
    except subprocess.TimeoutExpired as exc:
        code, out, err = 124, str(exc.stdout or ''), str(exc.stderr or '')
    expected = ['C6_LINUX host_main_intercepted', 'C6_LINUX whole_gamecore_loaded']
    for i in range(3):
        expected += [f'C6_LINUX create_enter {i}', f'C6_LINUX create_return {i} nonnull']
        if i == 0 and args.initialize:
            expected += ['C6_LINUX host_services_enter', 'C6_LINUX host_services_return',
                         'C6_LINUX set_engine_enter', 'C6_LINUX set_engine_return',
                         'C6_LINUX is_initialized_before 0', 'C6_LINUX initialize_enter',
                         'C6_LINUX initialize_return', 'C6_LINUX is_initialized_after 1']
        if i == 0 and args.startup:
            expected += ['C6_LINUX startup_enter', 'C6_LINUX startup_return']
        expected += [f'C6_LINUX destroy_return {i}']
    if args.initialize:
        expected += ['C6_LINUX host_services_destroy_enter',
                     'C6_LINUX host_services_destroy_return 0']
    expected += ['C6_LINUX dlclose_return 0']
    observed = [line for line in err.splitlines()
                if not (args.initialize and line.startswith('C6_LINUX_HOST '))]
    passed = code == 0 and observed == expected
    result = {'configuration_sha256': identity, 'configuration': configuration,
              'exit_code': code, 'stdout': out, 'stderr': err,
              'whole_gamecore_loaded': 'C6_LINUX whole_gamecore_loaded' in err,
              'context_lifetime_cycles_passed': 3 if passed else 0,
              'diagnostic_passed': passed, 'initialized_game': False,
              'startup_game_returned': 'C6_LINUX startup_return' in err.splitlines(),
              'context_initialize_returned': 'C6_LINUX initialize_return' in err.splitlines(),
              'context_initialized_flag': 'C6_LINUX is_initialized_after 1' in err.splitlines(),
              'save_loaded': False, 'action_or_turn_parity': False,
              'limits': ['host App main and shutdown intentionally bypassed',
                         'no initialized gameplay session, map, players or rules fixture',
                         'separate Linux lineage; Windows parity is not implied']}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'diagnostic_passed': passed, 'configuration_sha256': identity,
                      'exit_code': code, 'output': str(args.output)}))
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
