#!/usr/bin/env python3
"""Run the proprietary-asset bootstrap integration contract; never a parity test."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
RUNNER = ROOT / 'tools/bootstrap/run.py'


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def invoke(arguments, expected_code, output=None):
    run = subprocess.run([sys.executable, str(RUNNER), *arguments],
                         capture_output=True, text=True, timeout=180)
    require(run.returncode == expected_code,
            f'runner returned {run.returncode}, expected {expected_code}: {run.stderr}')
    return json.loads(output.read_text()) if output else run


def inventory(arguments, directory, name):
    output = directory / f'{name}-inventory.json'
    result = invoke([*arguments, '--inventory-only', '--output', str(output)], 0, output)
    identity = hashlib.sha256(json.dumps(result['configuration'], sort_keys=True,
                             separators=(',', ':')).encode()).hexdigest()
    require(identity == result['configuration_sha256'], 'configuration digest mismatch')
    configuration = result['configuration']
    require(configuration.get('wine_version') and configuration.get('compiler_version'),
            'runtime/compiler identity missing')
    for name, expected in configuration['sources'].items():
        require(hashlib.sha256((ROOT / name).read_bytes()).hexdigest() == expected,
                f'source identity mismatch: {name}')
    gate = json.loads((ROOT / 'tools/bootstrap/config.json').read_text())['expected_assets']
    for dependency in configuration['dependencies']:
        require(gate.get(dependency['path']) == dependency['sha256'], 'dependency allowlist mismatch')
    require(configuration['runtime_providers'] ==
            {name: digest for name, digest in gate.items() if name.startswith('LaunchPad/')},
            'runtime provider allowlist mismatch')
    return identity


def observations(result):
    require(result['initialized_game'] is False and result['gameplay_parity'] is False,
            'bootstrap must not claim initialized gameplay or parity')
    require(result['real_context_created'] is True, 'original context was not constructed')
    require(any(item.get('normal_windows_loader') is True for item in result['observations']),
            'normal Windows loader observation missing')
    for item in result['observations']:
        if item.get('phase') == 'context_queries':
            require(all(item.get(key) is True for key in
                        ('real_context_created', 'accepted_guid_1', 'accepted_guid_2', 'unknown_rejected')),
                    'original QueryInterface contract failed')
    return result['observations']


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets')
    parser.add_argument('--diagnostics', action='store_true',
                        help='also exercise initialization, database lifecycle and original SQL failure boundaries')
    args = parser.parse_args()
    base = ['--assets', str(args.assets)]
    with tempfile.TemporaryDirectory(prefix='c6-bootstrap-check-') as temporary:
        directory = Path(temporary)
        identity = inventory(base, directory, 'lifetime')
        rejected = invoke([*base, '--expected-configuration', '0' * 64], 1)
        require('configuration SHA256 mismatch: refusing execution' in rejected.stderr,
                'configuration mismatch was not rejected')
        for repeat in range(2):
            output = directory / f'lifetime-{repeat}.json'
            result = invoke([*base, '--expected-configuration', identity, '--output', str(output)], 0, output)
            items = observations(result)
            require([item['cycle'] for item in items if item.get('phase') == 'context_queries'] == [0, 1, 2],
                    'expected exactly three context lifetimes')
            require(items[-1].get('status') == 'ok' and items[-1].get('lifetime_cycles') == 3,
                    'successful destruction/unload observation missing')
            print(f'lifetime process {repeat + 1}: three original contexts, GUID checks, destruction and unload')
        # Exercise the host gate directly with a small non-PE file. Both the target
        # and original-host paths must fail before any LoadLibrary/fixture activity.
        wine = shutil.which('wine64') or shutil.which('wine') or '/usr/lib/wine/wine64'
        native = ROOT / 'target/bootstrap/context_host.exe'
        invalid = directory / 'wrong-image.dll'
        invalid.write_bytes(b'c6-bootstrap-invalid-image\n')
        win = lambda path: 'Z:' + str(path.resolve()).replace('/', '\\')
        dll = args.assets / 'DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll'
        deps = args.assets / 'Base/Binaries/Win64Steam'
        original = deps / 'CivilizationVI.exe'
        env = dict(os.environ, WINEPREFIX=str(ROOT / 'target/bootstrap/wine-prefix'),
                   WINEARCH='win64', WINEDEBUG='-all', WINEDLLOVERRIDES='msvcp140,vcruntime140,ucrtbase=n')
        for target, host in [(invalid, original), (dll, invalid)]:
            run = subprocess.run([wine, str(native), win(target), win(deps), win(host)],
                                 env=env, capture_output=True, text=True, timeout=120)
            require(run.returncode == 3, 'native image hash gate failed')
            items = [json.loads(line) for line in run.stdout.splitlines() if line.startswith('{')]
            require(len(items) == 1 and items[0].get('status') == 'unavailable',
                    'native image mismatch did not fail before loader activity')
        # Symlink the trusted asset set and replace a single dependency/provider.
        # The Python gate must reject these before compilation or original execution.
        gate = json.loads((ROOT / 'tools/bootstrap/config.json').read_text())['expected_assets']
        gated_assets = directory / 'gated-assets'
        for name in gate:
            link = gated_assets / name
            link.parent.mkdir(parents=True, exist_ok=True)
            link.symlink_to((args.assets / name).resolve())
        for name in ('Base/Binaries/Win64Steam/DatabaseDLL_FinalRelease.dll', 'LaunchPad/msvcp140.dll'):
            link = gated_assets / name
            link.unlink()
            link.write_bytes(b'incorrect bootstrap asset\n')
            rejected = invoke(['--assets', str(gated_assets), '--inventory-only'], 1)
            require('bootstrap asset SHA256 mismatch: refusing execution: ' + name in rejected.stderr,
                    'changed dependency/runtime provider was not rejected')
            link.unlink()
            link.symlink_to((args.assets / name).resolve())
        print('identity gates: configuration, native images, supplied dependency and CRT provider rejected')
        if args.diagnostics:
            for mode in ('--initialize', '--empty-databases', '--sql-schema-bootstrap'):
                arguments = [*base, mode]
                diagnostic_identity = inventory(arguments, directory, mode[2:])
                require(diagnostic_identity != identity, 'diagnostic mode identity aliases lifetime mode')
                output = directory / f'{mode[2:]}.json'
                result = invoke([*arguments, '--expected-configuration', diagnostic_identity,
                                 '--output', str(output)], 86 if mode == '--initialize' else 0, output)
                items = observations(result)
                require(any(item.get('registered_parameters') == 16 for item in items),
                        'event declaration observation missing')
                returned = [item for item in items if item.get('phase') == 'Initialize_returned']
                if mode == '--initialize':
                    require('unsupported' in result['stderr'] and not returned and
                            'missing_configuration_database_fixture' in result['stderr'],
                            'missing fixture must stop before Initialize returns')
                else:
                    require(len(returned) == 1 and returned[0].get('result') is True and
                            returned[0].get('is_initialized_before') is False and
                            returned[0].get('is_initialized_after') is True and
                            result.get('context_initialized') is True,
                            'database diagnostic must observe original Initialize and IsInitialized transition')
                    require([item['cycle'] for item in items if item.get('phase') == 'context_queries'] == [0, 1, 2]
                            and items[-1].get('status') == 'ok', 'database lifecycle did not complete')
                    if mode == '--sql-schema-bootstrap':
                        sql = [item for item in items if item.get('phase') == 'original_database_sql']
                        require([item['source_index'] for item in sql] == list(range(7)) and
                                all(item.get('success') is True for item in sql), 'original schemas did not execute')
                print(f'{mode}: expected native boundary; initialized gameplay remains false')
            raw = [*base, '--sql-schema-bootstrap', '--raw-sql-bytes']
            raw_identity = inventory(raw, directory, 'raw-sql')
            output = directory / 'raw-sql.json'
            result = invoke([*raw, '--expected-configuration', raw_identity, '--output', str(output)], 15, output)
            items = observations(result)
            sql = [item for item in items if item.get('phase') == 'original_database_sql']
            require([item['source_index'] for item in sql] == list(range(6)) and
                    all(item.get('success') is True for item in sql[:5]) and
                    sql[-1].get('success') is False and sql[-1].get('utf8_bom_removed') is False,
                    'original raw-BOM SQL failure boundary changed')
            require(not any(item.get('phase') == 'Initialize_returned' or item.get('status') == 'ok'
                            for item in items), 'raw SQL failure continued to initialization/completion')
            print('raw SQL bytes: original source 5 failure preserved before Initialize')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
