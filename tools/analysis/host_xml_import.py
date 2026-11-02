#!/usr/bin/env python3
"""Run exact original XML importer in an isolated, fingerprinted Wine process."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def windows(path):
    return 'Z:' + str(path.resolve()).replace('/', '\\')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets')
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--base-recipe', action='store_true', help='also execute fingerprinted original Base source sequence')
    parser.add_argument('--recipe', type=Path, help='execute an explicitly supplied fingerprinted source recipe; ordering limits remain in evidence')
    args = parser.parse_args()
    gate = json.loads((ROOT / 'tools/bootstrap/config.json').read_text())['expected_assets']
    providers = {name: expected for name, expected in gate.items()
                 if name.startswith('LaunchPad/') or name.endswith(
                     ('CivilizationVI.exe', 'DatabaseDLL_FinalRelease.dll'))}
    for name, expected in providers.items():
        if sha(args.assets / name) != expected:
            raise SystemExit('asset SHA256 mismatch: ' + name)
    recipe = None
    if args.base_recipe and args.recipe:
        parser.error('choose --base-recipe or --recipe')
    if args.base_recipe or args.recipe:
        recipe = json.loads((args.recipe or ROOT / 'evidence/assets/base-content-recipe.json').read_text())
        if recipe['host_executable_sha256'] != providers['Base/Binaries/Win64Steam/CivilizationVI.exe']:
            raise SystemExit('recipe executable identity mismatch')
        for step in recipe['steps']:
            if sha(args.assets / step['path']) != step['sha256']:
                raise SystemExit('recipe source SHA256 mismatch: ' + step['path'])
    source_dir = ROOT / 'native/bootstrap/xml-import'
    subprocess.run(['make', '-C', str(source_dir)], check=True, stdout=subprocess.DEVNULL)
    host = ROOT / 'target/bootstrap/xml_import_probe.exe'
    for name in providers:
        if name.startswith('LaunchPad/'):
            shutil.copyfile(args.assets / name, host.parent / Path(name).name)
    wine = shutil.which('wine64') or shutil.which('wine') or '/usr/lib/wine/wine64'
    configuration = {
        'format': 'c6-original-xml-probe-configuration.v1',
        'providers': providers,
        'sources': {str(path.relative_to(ROOT)): sha(path) for path in sorted(source_dir.iterdir()) if path.is_file()},
        'runner_sha256': sha(Path(__file__)),
        'wine_version': subprocess.check_output([wine, '--version'], text=True).strip(),
        'compiler_version': subprocess.check_output(['x86_64-w64-mingw32-gcc', '--version'], text=True).splitlines()[0],
        'overrides': 'msvcp140,vcruntime140,ucrtbase=n',
        'exe_startup': False, 'iat_provider_classes': ['CRT', 'KERNEL32', 'DatabaseDLL'],
        'fixture': 'C6Probe CHECK(Value=7), one native XML Row Value=7',
        'rules_database': 'original_source_recipe_diagnostic' if recipe else None,
        'enabled_dlc_mods': None, 'initialized_game': False,
        'base_recipe': recipe,
    }
    configuration_sha256 = hashlib.sha256(json.dumps(configuration, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    deps = args.assets / 'Base/Binaries/Win64Steam'
    env = dict(os.environ, WINEPREFIX=str(ROOT / 'target/bootstrap/wine-prefix'),
               WINEARCH='win64', WINEDEBUG='-all', WINEDLLOVERRIDES=configuration['overrides'])
    command = [wine, str(host), windows(deps / 'CivilizationVI.exe'), windows(deps)]
    if recipe:
        manifest = host.parent / 'original-base-recipe.tsv'
        manifest.write_text(''.join('\t'.join([step['database'], step['format'], step['phase'],
                                             step['sha256'], windows(args.assets / step['path']),
                                             step.get('action_id', '')]) + '\n'
                                    for step in recipe['steps']), encoding='utf-8')
        command.append(windows(manifest))
    run = subprocess.run(command,
                         env=env, capture_output=True, text=True, timeout=120)
    observations = [json.loads(line) for line in run.stdout.splitlines() if line.startswith('{')]
    success = run.returncode == 0 and any(item.get('original_xml_import') for item in observations)
    recipe_passed = None
    if recipe:
        imports = [item for item in observations if item.get('phase') == 'original_source_import']
        recipe_passed = (len(imports) == len(recipe['steps']) and
                         all(item.get('native_status') == 0 and item.get('source_index') == index and
                             item.get('database') == source['database'] and item.get('format') == source['format']
                             for index, (item, source) in enumerate(zip(imports, recipe['steps']))) and
                         any(item.get('phase') == 'original_recipe_complete' and
                             item.get('source_count') == len(recipe['steps']) for item in observations))
        success = success and recipe_passed
    result = {'configuration': configuration, 'configuration_sha256': configuration_sha256,
              'host_sha256': sha(host), 'returncode': run.returncode, 'observations': observations,
              'stderr': run.stderr, 'native_xml_probe_passed': success,
              'native_source_recipe_passed': recipe_passed,
              'initialized_game': False, 'gameplay_parity': False}
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'returncode': run.returncode, 'native_xml_probe_passed': success,
                      'configuration_sha256': configuration_sha256}))
    return 0 if success else (run.returncode or 1)


if __name__ == '__main__':
    raise SystemExit(main())
