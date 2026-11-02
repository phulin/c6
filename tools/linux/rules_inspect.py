#!/usr/bin/env python3
"""Fingerprint genuinely exported Linux base rules; never infer XP2 mod coverage."""
import argparse
import json
import sqlite3
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'assets'))
from inventory import digest, identity
from rules_snapshot import ENCODING, fingerprint

ROOT = Path(__file__).resolve().parents[2]
HOST = '106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
GAME = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
PROFILE = 'linux-base-native-generated-v1'
BASE_CONFIGURATION = '499dc9856ba562523ec70268224729232f751663988e44f4f9b0d6208e7f9328'


def validate_profile(configuration, configuration_hash):
    expected_assets = json.loads((ROOT / 'tools/linux/runtime-gate.json').read_text())
    expected_content = json.loads((ROOT / 'tools/linux/runtime-content-gate.json').read_text())
    if configuration.get('original_assets') != expected_assets:
        raise ValueError('unsupported Linux complete module profile')
    if configuration.get('content_profile') != expected_content:
        raise ValueError('unsupported Linux base source content profile')
    expected_mapping = [
        {'source': name, 'staged': '/'.join('Schema' if part == 'schema' else part
                                           for part in Path(name).parts), 'sha256': sha}
        for name, sha in sorted(expected_content['files'].items())]
    view = configuration.get('rules_source_view', {})
    if (view.get('format') != 'c6.linux-native-base-source-view.v1' or
            view.get('original_source_gate_sha256') != digest(ROOT / 'tools/linux/runtime-content-gate.json') or
            view.get('mapping') != expected_mapping or
            view.get('source_view_sha256') != identity(expected_mapping)):
        raise ValueError('unsupported Linux base source-view strategy')
    if (configuration.get('mode') != 'host-crt-init-original-host-services-context-initialize' or
            configuration.get('enabled_dlc_mods') is not None or configuration.get('rules_database') is not None or
            configuration.get('startup_game_requested') is not False or
            configuration.get('initialized_game') is not False or
            configuration.get('host_application_main_called') is not False):
        raise ValueError('unsupported Linux base activation/state profile')
    forbidden = ('saved_configuration_profile', 'localization_profile', 'localization_environment',
                 'load_stream_fixture', 'native_stream_adapter', 'native_save_requested', 'lua_profile')
    if any(name in configuration for name in forbidden):
        raise ValueError('unsupported Linux base optional service/session profile')
    if configuration.get('rules_snapshot_profile') != {
            'requested': True, 'header_sha256': digest(ROOT / 'native/linux/rules_snapshot.h')}:
        raise ValueError('unsupported Linux base native backup adapter')
    # Fixed named baseline, not permission to relabel arbitrary self-consistent
    # reports. Changes require a newly audited/registered fixture identity.
    if configuration_hash != BASE_CONFIGURATION:
        raise ValueError('unsupported Linux base fixture configuration identity')


def inspect(directory, report_path):
    report = json.loads(report_path.read_text())
    configuration = report['configuration']
    if identity(configuration) != report['configuration_sha256']:
        raise ValueError('native Linux configuration identity mismatch')
    assets = configuration['original_assets']
    if assets.get('Civ6') != HOST or assets.get('libGameCore_XP2.so') != GAME:
        raise ValueError('unsupported Linux native module identity')
    validate_profile(configuration, report['configuration_sha256'])
    if report.get('exit_code') != 0 or not report.get('diagnostic_passed'):
        raise ValueError('a passing genuine Linux fixture is required')
    events = [json.loads(line[len('C6_LINUX_RULES '):]) for line in report['stderr'].splitlines()
              if line.startswith('C6_LINUX_RULES {')]
    backups = [event for event in events if event.get('phase') == 'original_database_backup']
    if len(backups) != 2 or any(event.get('native_success') is not True or
                               event.get('row_counts_match') is not True or event.get('result') != 0 for event in backups):
        raise ValueError('successful native Linux backups are required')
    artifacts = report['rules_snapshots']
    if sorted(artifact['filename'] for artifact in artifacts) != ['configuration.sqlite', 'gameplay.sqlite']:
        raise ValueError('exactly two native Linux snapshot artifacts are required')
    databases = []
    for name in ('configuration', 'gameplay'):
        db = fingerprint(directory / (name + '.sqlite'))
        db['database'] = name
        artifact = next(artifact for artifact in artifacts if artifact['filename'] == db['filename'])
        if (artifact['sha256'], artifact['bytes']) != (db['file_sha256'], db['bytes']):
            raise ValueError('native runtime snapshot hash/size mismatch: ' + name)
        event = next(event for event in backups if event['database'] == name)
        if (event['tables'], event['rows']) != (db['table_count'], db['row_count']):
            raise ValueError('native source/destination count mismatch: ' + name)
        databases.append(db)
    gameplay = next(db for db in databases if db['database'] == 'gameplay')
    config = next(db for db in databases if db['database'] == 'configuration')
    tables = {table['name']: table for table in gameplay['semantic']['tables']}
    if not config['table_count'] or not config['row_count'] or any(
            name not in tables or tables[name]['rows'] <= 0 for name in ('Units', 'Types', 'GameSpeeds')):
        raise ValueError('native base rules remain unpopulated; Initialize boolean is insufficient')
    content = configuration['content_profile']
    return {'configuration_sha256': report['configuration_sha256'],
            'configuration': configuration, 'report_sha256': digest(report_path),
            'source_sequence_sha256': identity(content), 'databases': databases}


def compare(a, b, report_paths):
    if a['configuration_sha256'] != b['configuration_sha256']:
        raise ValueError('independent Linux fixture configurations differ')
    for x, y in zip(a['databases'], b['databases'], strict=True):
        if x != y:
            raise ValueError('independent native Linux DB snapshots differ: ' + x['database'])
    configuration = a['configuration']
    content = configuration['content_profile']
    return {'format': 'c6.reproducible-rules-snapshot.v1',
            'profile': {'id': PROFILE, 'target_sha256': GAME, 'original_executable_sha256': HOST,
                        'ruleset': 'unselected-base-content', 'enabled_dlc_mods': [],
                        'rules_database': 'original_linux_ConfigurationDatabase_and_GameplayDatabase_Initialize',
                        'coverage': 'base_sources_only_full_xp2_mod_activation_pending',
                        'content_recipe_sha256': identity(content), 'localization': None},
            'module_identities': [{'path': name, 'sha256': sha}
                                  for name, sha in configuration['original_assets'].items()],
            'native_configuration_sha256': a['configuration_sha256'],
            'source_sequence_sha256': a['source_sequence_sha256'],
            'source_order_authority': 'original Linux ConfigurationDatabase/GameplayDatabase Bootstrap; source gate is inventory identity, not inferred execution order',
            'source_view': configuration.get('rules_source_view'),
            'language_identity': {'localization': None, 'localized_text_database_included': False},
            'native_runs': [{'evidence': str(path), 'evidence_sha256': run['report_sha256']}
                            for path, run in zip(report_paths, (a,b), strict=True)],
            'independent_native_builds': 2, 'file_bytes_identical': True,
            'typed_schema_rowid_content_identical': True,
            'inspector': {'source_sha256': digest(Path(__file__)),
                          'typed_inspector_sha256': digest(ROOT/'tools/assets/rules_snapshot.py'),
                          'encoding': ENCODING, 'sqlite_version': sqlite3.sqlite_version},
            'databases': a['databases'], 'initialized_game': False,
            'limits': ['Linux profile only; no Windows source order or rules parity inferred.',
                       'Only genuine original Linux base DB Initialize executed; XP2 mod activation and native definition setup remain pending.',
                       'No selected gameplay session/options, RNG, player/map state, save load or turn parity established.',
                       'Typed SQL snapshot does not serialize native connection functions/collations/planner state.']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--native-evidence', required=True, type=Path)
    parser.add_argument('--compare-directory', required=True, type=Path)
    parser.add_argument('--compare-native-evidence', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    a = inspect(args.directory, args.native_evidence)
    b = inspect(args.compare_directory, args.compare_native_evidence)
    result = compare(a, b, (args.native_evidence, args.compare_native_evidence))
    args.output.write_text(json.dumps(result, sort_keys=True, indent=2) + '\n')
    print(json.dumps({db['database']: db['semantic_sha256'] for db in result['databases']}, sort_keys=True))


if __name__ == '__main__':
    main()
