#!/usr/bin/env python3
"""Exact-byte Linux ASL filesystem bootstrap diagnostic; never replaces path APIs."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pwd
import re
import subprocess

from rules_launch_fixture import validate as validate_launch_fixture

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    with Path(path).open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def directory_state(path):
    if not path.exists():
        return {'exists': False}
    return {'exists': True, 'entries': sorted(p.name for p in path.iterdir()),
            'directory': path.is_dir()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--launch-host', type=Path, required=True)
    parser.add_argument('--sysroot', type=Path, default=ROOT / 'target/linux/sysroot')
    parser.add_argument('--expected-configuration')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    assets = args.assets.resolve(strict=True)
    host = args.launch_host.absolute()
    if host.is_symlink() or host.resolve().is_relative_to(assets):
        parser.error('launch host must be an exact byte copy outside original assets')
    modules = json.loads((ROOT / 'tools/linux/runtime-gate.json').read_text())
    for name, expected in modules.items():
        if digest(assets / name) != expected:
            raise SystemExit(f'original module gate mismatch: {name}')
    if digest(host) != modules['Civ6']:
        raise SystemExit('launch host identity mismatch')
    launch_profile = validate_launch_fixture(assets, host.parent)
    mappings = {'steamassets/base': 'Base', 'steamassets/dlc': 'DLC',
                'aspyrassets': 'aspyrassets'}
    for staged, original in mappings.items():
        link = host.parent / staged
        if not link.is_symlink() or link.resolve(strict=True) != (assets / original).resolve():
            raise SystemExit(f'original launch directory mapping mismatch: {staged}')
    metadata = json.loads((ROOT / 'tools/linux/filesystem_gate.json').read_text())
    actual_metadata = {str(p.relative_to(assets)) for p in (assets / 'aspyrassets').rglob('*')
                       if p.is_file()}
    if actual_metadata != set(metadata['files']):
        raise SystemExit('ASL metadata inventory mismatch')
    for name, expected in metadata['files'].items():
        if digest(assets / name) != expected:
            raise SystemExit(f'ASL metadata gate mismatch: {name}')
    work = ROOT / 'target/linux-filesystem'
    work.mkdir(parents=True, exist_ok=True)
    lock = (work / 'probe.lock').open('w')
    fcntl.flock(lock, fcntl.LOCK_EX)
    env = {k: v for k, v in os.environ.items() if not k.startswith(('LD_', 'C6_'))}
    env['LD_LIBRARY_PATH'] = f'{args.sysroot.resolve()}/usr/lib/x86_64-linux-gnu:{assets}'
    listing = subprocess.check_output(['ldd', str(host)], env=env, text=True)
    if 'not found' in listing:
        raise SystemExit('unavailable original native dependency')
    dependencies = {}
    for line in listing.splitlines():
        match = re.search(r'(?:=>\s+)?(/\S+)\s+\(', line)
        if match:
            path = Path(match[1]).resolve()
            dependencies[path.name] = {'path': str(path), 'sha256': digest(path)}
    sources = [ROOT / 'tools/linux/filesystem_probe.c', ROOT / 'native/linux/filesystem.c']
    preload = work / 'filesystem_probe.so'
    subprocess.run(['cc', '-shared', '-fPIC', '-Wall', '-Wextra', '-Werror',
                    *map(str, sources), '-ldl', '-o', str(preload)], check=True)
    home = Path(pwd.getpwuid(os.getuid()).pw_dir)
    native_data = home / '.local/share/aspyr-media'
    before_data = directory_state(native_data)
    if before_data['exists']:
        allowed = {"Sid Meier's Civilization VI", "Sid Meier's Civilization VI/Aspyr"}
        descendants = list(native_data.rglob('*'))
        if any(p.is_symlink() or not p.is_dir() or
               str(p.relative_to(native_data)) not in allowed for p in descendants):
            raise SystemExit('existing native ASL user configuration preserved; isolate before probing')
    configuration = {'schema': 'c6-linux-original-asl-filesystem-v1',
                     'original_modules': modules, 'runtime_dependencies': dependencies,
                     'launch_host': str(host), 'launch_host_sha256': digest(host),
                     'launch_mappings': mappings, 'launch_fixture': launch_profile,
                     'ASL_metadata_gate': metadata,
                     'native_account_home': str(home), 'native_data_root': str(native_data),
                     'native_data_before': before_data,
                     'sources': {str(p.relative_to(ROOT)): digest(p) for p in
                                 [*sources, ROOT / 'native/linux/filesystem.h', Path(__file__),
                                  ROOT / 'tools/linux/rules_launch_fixture.py',
                                  ROOT / 'tools/assets/inventory.py']},
                     'preload_sha256': digest(preload),
                     'compiler': subprocess.check_output(['cc', '--version'], text=True).splitlines()[0]}
    identity = hashlib.sha256(json.dumps(configuration, sort_keys=True,
                                       separators=(',', ':')).encode()).hexdigest()
    if args.expected_configuration and identity != args.expected_configuration:
        raise SystemExit('configuration identity mismatch before native execution')
    env['LD_PRELOAD'] = str(preload)
    try:
        child = subprocess.run([str(host)], cwd=work, env=env, text=True,
                               capture_output=True, timeout=30)
        stderr, stdout, returncode = child.stderr, child.stdout, child.returncode
    except subprocess.TimeoutExpired as failure:
        stderr = str(failure.stderr or '')
        stdout, returncode = str(failure.stdout or ''), None
    observed = []
    for phase, index, exists, path in re.findall(
            r'C6_LINUX_FILESYSTEM normalized (\w+) (\d+) exists ([01]) path ([^\n]+)', stderr):
        observed.append({'phase': phase, 'index': int(index), 'exists': exists == '1', 'path': path})
    after = {row['index']: row for row in observed if row['phase'] == 'after'}
    passed = (returncode == 0 and len(observed) == 12 and len(after) == 6 and
              all(after[i]['exists'] for i in range(3)) and not after[3]['exists'] and
              'C6_LINUX_FILESYSTEM shutdown_return' in stderr)
    result = {'configuration': configuration, 'configuration_sha256': identity,
              'exit_code': returncode, 'diagnostic_passed': passed,
              'observed_paths': observed, 'native_data_after': directory_state(native_data),
              'temporary_path_removed': bool(after.get(5)) and not Path(after[5]['path']).exists(),
              'stderr': stderr, 'stdout': stdout,
              'limits': ['Original ASL path/init lifetime only; no Base/DLC database content proof.',
                         'Expected installed root directory links are an explicit fixture adaptation.',
                         'Original getpwuid-based ASL data-root creation is recorded; HOME is unchanged.']}
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'configuration_sha256': identity, 'exit_code': returncode,
                      'diagnostic_passed': passed, 'output': str(args.output)}))
    raise SystemExit(0 if passed else 1)


if __name__ == '__main__':
    main()
