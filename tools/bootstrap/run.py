#!/usr/bin/env python3
"""Fingerprint-gated, real Windows loader/context lifetime probe (no game init)."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
EXE_HASH = 'e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63'
DLL_HASH = '324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c'


def sha(path):
    return hashlib.file_digest(path.open('rb'), 'sha256').hexdigest()


def pe_imports(path):
    output = subprocess.check_output(['objdump', '-p', str(path)], text=True)
    return re.findall(r'DLL Name: (.+)', output)


def windows(path):
    return 'Z:' + str(path.resolve()).replace('/', '\\')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--inventory-only', action='store_true')
    parser.add_argument('--initialize', action='store_true', help='run explicitly bounded host service diagnostic')
    parser.add_argument('--content-recipe', choices=('base', 'xp2'), help='populate real original database connections using pinned original SQL/XML recipe')
    parser.add_argument('--load-stream', type=Path, help='after original StartupGame, probe original Load with fingerprinted private extracted stream')
    parser.add_argument('--startup-game', action='store_true', help='probe original StartupGame with explicit SQL-only schema fixture')
    parser.add_argument('--raw-sql-bytes', action='store_true', help='preserve UTF-8 BOM for reproducible original SQL failure boundary')
    parser.add_argument('--sql-schema-bootstrap', action='store_true', help='execute seven hash-gated original SQL schema files through original DatabaseDLL')
    parser.add_argument('--empty-databases', action='store_true', help='initialize original context with real empty named-memory DatabaseDLL fixtures')
    parser.add_argument('--expected-configuration', help='refuse configuration identity mismatch')
    args = parser.parse_args()
    if args.raw_sql_bytes and not args.sql_schema_bootstrap:
        parser.error('--raw-sql-bytes requires --sql-schema-bootstrap')
    save_stream = None
    if args.load_stream:
        args.startup_game = True
        if args.load_stream.stat().st_size != 76556938 or sha(args.load_stream) != 'a66736f0423e9a59b11c77b1879b471d936fe3887342df5008940f763dbcc5aa':
            raise SystemExit('save stream SHA256/size mismatch: refusing execution')
        save_stream = {'sha256': sha(args.load_stream), 'size': args.load_stream.stat().st_size, 'full_save_loader': False}
    if args.startup_game and not args.content_recipe:
        args.sql_schema_bootstrap = True
    if args.content_recipe:
        args.empty_databases = True
        if args.sql_schema_bootstrap or args.raw_sql_bytes:
            parser.error('--content-recipe cannot combine with seven-source SQL diagnostic')
    if args.sql_schema_bootstrap:
        args.empty_databases = True
    if args.empty_databases:
        args.initialize = True
    localization = None
    if args.startup_game:
        localization = json.loads((ROOT / 'evidence/bootstrap/localization.json').read_text())['configuration']
        localization_sources = ROOT / 'tools/analysis/localization_sources.json'
        if sha(localization_sources) != localization['sources_manifest_sha256']:
            raise SystemExit('localization source manifest SHA256 mismatch')
        localization['sources'] = json.loads(localization_sources.read_text())['sources']
        for source in localization['sources']:
            if sha(args.assets / source['path']) != source['sha256']:
                raise SystemExit('localization source SHA256 mismatch: ' + source['path'])
        if sha(args.assets / 'Base/Assets/Text/icudt56l.dat') != localization['icu_sha256']:
            raise SystemExit('ICU SHA256 mismatch: refusing execution')
    recipe = None
    recipe_path = None
    if args.content_recipe:
        recipe_path = ROOT / ('evidence/assets/base-content-recipe.json' if args.content_recipe == 'base' else 'evidence/assets/xp2-declared-content-recipe.json')
        recipe = json.loads(recipe_path.read_text())
        if recipe['host_executable_sha256'] != EXE_HASH:
            raise SystemExit('content recipe executable identity mismatch')
        for step in recipe['steps']:
            source = (args.assets / step['path']).resolve()
            if not source.is_relative_to(args.assets.resolve()) or source.stat().st_size != step['bytes'] or sha(source) != step['sha256']:
                raise SystemExit('content recipe source SHA256/size mismatch: ' + step['path'])
    sql_sources = []
    if args.sql_schema_bootstrap:
        sql_sources = json.loads((ROOT / "evidence/assets/sql-schema-sources.json").read_text())["sources"]
        for source in sql_sources:
            if sha(args.assets / source["path"]) != source["sha256"]:
                raise SystemExit("SQL schema SHA256 mismatch: refusing execution")
    dll = args.assets / 'DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll'
    deps = args.assets / 'Base/Binaries/Win64Steam'
    original_executable = deps / 'CivilizationVI.exe'
    if sha(original_executable) != EXE_HASH:
        raise SystemExit('original executable SHA256 mismatch: refusing execution')
    asset_gate = json.loads((ROOT / 'tools/bootstrap/config.json').read_text())
    runtime_files = [args.assets / name for name in sorted(asset_gate['expected_assets']) if name.startswith('LaunchPad/')]
    host = ROOT / 'target/bootstrap/context_host.exe'
    if sha(dll) != DLL_HASH:
        raise SystemExit('target DLL SHA256 mismatch: refusing execution')
    for name, expected in asset_gate['expected_assets'].items():
        if sha(args.assets / name) != expected:
            raise SystemExit('bootstrap asset SHA256 mismatch: refusing execution: ' + name)
    wine = shutil.which('wine64') or shutil.which('wine') or '/usr/lib/wine/wine64'
    wine_version = subprocess.check_output([wine, '--version'], text=True).strip()
    compiler_version = subprocess.check_output(['x86_64-w64-mingw32-gcc', '--version'], text=True).splitlines()[0]
    sources = {name: sha(ROOT / name) for name in (
        'native/bootstrap/context_host.c',
        'native/bootstrap/services/service_fixture.c',
        'native/bootstrap/services/service_fixture.h',
        'native/bootstrap/save/save_probe.c',
        'native/bootstrap/save/save_probe.h',
        'native/bootstrap/xml-import/imports.c',
        'native/bootstrap/xml-import/imports.h',
        'native/bootstrap/xml-import/xml.c',
        'native/bootstrap/xml-import/xml.h',
        'native/bootstrap/xml-import/content.c',
        'native/bootstrap/xml-import/content.h',
        'native/bootstrap/localization/localization.c',
        'native/bootstrap/localization/localization.h',
    )}
    sources['tools/bootstrap/config.json'] = sha(ROOT / 'tools/bootstrap/config.json')
    sources['native/bootstrap/Makefile'] = sha(ROOT / 'native/bootstrap/Makefile')
    sources['tools/bootstrap/run.py'] = sha(Path(__file__))
    inventory = []
    for path in [dll] + [deps / name for name in ('DatabaseDLL_FinalRelease.dll', 'Localization_FinalRelease.dll', 'HavokScript_FinalRelease.dll')]:
        inventory.append({'path': str(path.relative_to(args.assets)), 'sha256': sha(path), 'imports': pe_imports(path)})
    supplied = {p.name.lower() for p in deps.glob('*.dll')}
    external = sorted({name for item in inventory for name in item['imports'] if name.lower() not in supplied})
    configuration = {'schema': 'c6-wine-context-lifetime-v1', 'target_sha256': DLL_HASH,
                     'sources': sources, 'dependencies': inventory,
                     'wine_version': wine_version, 'compiler_version': compiler_version,
                     'original_executable_sha256': EXE_HASH,
                     'runtime_providers': {str(p.relative_to(args.assets)): sha(p) for p in runtime_files},
                     'wine_dll_overrides': 'msvcp140,vcruntime140,ucrtbase=n',
                     'runtime_imports': external, 'service_fixture': 'bounded_diagnostic' if args.initialize else 'none',
                     'startup_game_requested': args.startup_game, 'save_stream': save_stream,
                     'content_recipe_sha256': sha(recipe_path) if recipe_path else None,
                     'content_recipe': recipe,
                     'localization': ({k: localization[k] for k in ('language', 'selection', 'sources', 'icu_sha256', 'dll_sha256')} if localization else None),
                     'sql_schema_sources': sql_sources,
                     'sql_text_transform': ('none' if args.raw_sql_bytes else 'strip_utf8_bom_if_present') if args.sql_schema_bootstrap else None,
                     'rules_database': 'original_sql_only_schema_diagnostic' if args.sql_schema_bootstrap else ('empty_named_memory_diagnostic' if args.empty_databases else None), 'enabled_dlc_mods': None,
                     'initialized_game': False}
    if recipe:
        configuration['rules_database'] = 'original_declared_content_recipe_diagnostic'
        configuration['enabled_dlc_mods'] = recipe['selected_mod_ids']
        configuration['ruleset'] = recipe['ruleset']
    identity = hashlib.sha256(json.dumps(configuration, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    if args.expected_configuration and args.expected_configuration != identity:
        raise SystemExit('configuration SHA256 mismatch: refusing execution')
    result = {'configuration_sha256': identity, 'configuration': configuration}
    if not args.inventory_only:
        subprocess.run(['make', '-C', str(ROOT / 'native/bootstrap')], check=True, stdout=subprocess.DEVNULL)
        if any(sha(ROOT / name) != expected for name, expected in sources.items()):
            raise SystemExit('bootstrap source changed during build: refusing execution')
        for runtime in runtime_files:
            shutil.copyfile(runtime, host.parent / runtime.name)
        env = dict(os.environ, WINEPREFIX=str(ROOT / 'target/bootstrap/wine-prefix'), WINEARCH='win64', WINEDEBUG='-all', WINEDLLOVERRIDES='msvcp140,vcruntime140,ucrtbase=n')
        if localization:
            localization_manifest = host.parent / 'localization-context-paths.tsv'
            localization_manifest.write_text(''.join('\t'.join((Path(source['path']).suffix[1:], source['sha256'], windows(args.assets / source['path']))) + '\n' for source in localization['sources']), encoding='utf-8')
            env['C6_BOOTSTRAP_LOCALIZATION_MANIFEST'] = windows(localization_manifest)
            env['C6_BOOTSTRAP_ICU_PATH'] = windows(args.assets / 'Base/Assets/Text/icudt56l.dat')
        command = [wine, str(host), windows(dll), windows(deps), windows(original_executable)]
        if args.initialize:
            if recipe:
                mode = '--' + args.content_recipe + '-content' + ('-load' if args.load_stream else ('-startup' if args.startup_game else ''))
            else:
                mode = '--load-stream' if args.load_stream else ('--startup-game' if args.startup_game else ('--raw-sql-databases' if args.raw_sql_bytes else ('--empty-databases' if args.empty_databases else '--initialize')))
            command.append(mode)
        if recipe:
            manifest = host.parent / 'content-paths.tsv'
            manifest.write_text(''.join('\t'.join((step['database'], step['format'], step['phase'], step['sha256'], windows(args.assets / step['path']))) + ('\t' + step['action_id'] if step.get('action_id') else '') + '\n' for step in recipe['steps']), encoding='utf-8')
            command.append(windows(manifest))
        elif args.sql_schema_bootstrap:
            manifest = host.parent / 'sql-schema-paths.tsv'
            manifest.write_text(''.join(('1' if '/Gameplay/' in item['path'] or item['path'].startswith('DLC/') else '0') + '\t' + windows(args.assets / item['path']) + '\n' for item in sql_sources), encoding='utf-8')
            command.append(windows(manifest))
        if args.load_stream:
            command.append(windows(args.load_stream))
        run = subprocess.run(command, env=env, text=True, capture_output=True, timeout=120)
        observations = [json.loads(line) for line in run.stdout.splitlines() if line.startswith('{')]
        result.update({'host_sha256': sha(host), 'wine_version': wine_version,
                       'returncode': run.returncode, 'observations': observations,
                       'stderr': run.stderr, 'real_context_created': any(item.get('real_context_created') for item in observations),
                       'context_initialized': any(item.get('phase') == 'Initialize_returned' and item.get('result') and item.get('is_initialized_after') for item in observations),
                       'initialized_game': False, 'gameplay_parity': False})
    rendered = json.dumps(result, indent=2) + '\n'
    if args.output:
        args.output.write_text(rendered)
    else:
        print(rendered, end='')
    return result.get('returncode', 0)


if __name__ == '__main__':
    raise SystemExit(main())
