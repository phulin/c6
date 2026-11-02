#!/usr/bin/env python3
"""Inspect the separately audited original Linux XP2 requestNone callback fixture.

This profile is not saved Begin, an initialized gameplay session, or turn parity.
"""
import argparse
import json
from pathlib import Path
import sqlite3
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'assets'))
from inventory import digest, identity
from rules_snapshot import ENCODING, fingerprint

ROOT = Path(__file__).resolve().parents[2]
PROFILE = 'linux-xp2-native-request-none-generated-v1'
# Only frozen, independently executed fixture identities may be registered here.
CONFIGURATION = '4c392cbe97c20bed07617686d03260417dc2abe65aace7ce15988279279b43a6'
SELECTION_SHA256 = '508c782bb6e4064fb36fd9116a000dcc59f93fc14ad015ab7df73c24daed8df1'
XP2 = '4873eb62-8ccc-4574-b784-dda455e74e68'
CALLBACK = 'native LoadGameViewState constructor only; requestNone and constructor-default censorship=false; no saved Request/Begin, not saved-game parity'


def validate_report(report):
    configuration = report['configuration']
    if identity(configuration) != report['configuration_sha256']:
        raise ValueError('native Linux XP2 configuration identity mismatch')
    if CONFIGURATION is None or report['configuration_sha256'] != CONFIGURATION:
        raise ValueError('unsupported Linux XP2 requestNone fixture configuration identity')
    gate = json.loads((ROOT / 'evidence/linux/rules-content-inputs.json').read_text())
    modules = json.loads((ROOT / 'tools/linux/runtime-gate.json').read_text())
    modules.update(gate['modules'])
    if configuration.get('assets') != modules:
        raise ValueError('unsupported Linux XP2 complete module profile')
    launch = configuration.get('original_filesystem_profile', {}).get('validated_launch', {})
    if (launch.get('format') != 'c6.linux-original-asl-launch-fixture.v3' or
            launch.get('mapping_sha256') != '4768b158ee2281c1a2f54f679a06e65137c050286bdf77215b15922c7d92c0f3' or
            launch.get('validated_input_gate_sha256') != digest(ROOT / 'evidence/linux/rules-content-inputs.json') or
            launch.get('validated_source_inventory_sha256') != gate['source_inventory_sha256'] or
            launch.get('validated_native_module_identities') != gate['modules']):
        raise ValueError('unsupported Linux XP2 native source strategy')
    if (configuration.get('update_callback_profile') != CALLBACK or
            configuration.get('ui_constructor_only') is not True or
            configuration.get('configure_requested') is not True or
            configuration.get('source_view') is not None or
            configuration.get('localization_profile', {}).get('locale') != 'en_US'):
        raise ValueError('unsupported Linux XP2 callback/localization profile')
    if report.get('returncode') != 0 or report.get('framework_boundary_returned') is not True:
        raise ValueError('successful original Linux XP2 update fixture is required')
    lines = report['stderr'].splitlines()
    required = [
        'C6_LINUX_CONFIGURATION ruleset RULESET_EXPANSION_2 enabled_mods 1 required_mods 1',
        'C6_LINUX_CONFIGURATION enabled_mod 0 ' + XP2,
        'C6_LINUX_CONFIGURATION required_mod 0 ' + XP2,
        'C6_LINUX_MOD configure_content_return flags 3 code 0',
        'C6_LINUX_MOD update_constructor_callback_owner_enter request_none 1',
        'C6_LINUX_MOD update_content_return flags 3 code 0',
        'C6_LINUX_MOD rules_snapshot_return 0',
    ]
    if any(line not in lines for line in required):
        raise ValueError('required original Linux XP2 activation/backup markers missing')
    updated = [line for line in lines if line.startswith('C6_LINUX_MOD updated_active_context_module ')]
    if len(updated) != 1 or not updated[0].endswith('/libGameCore_XP2.so'):
        raise ValueError('actual native XP2 context module transition is required')
    events = [json.loads(line[len('C6_LINUX_RULES '):]) for line in lines
              if line.startswith('C6_LINUX_RULES {')]
    backups = [event for event in events if event.get('phase') == 'original_database_backup']
    if (sorted(event.get('database', '') for event in backups) != ['configuration', 'gameplay'] or
            any(event.get('native_success') is not True or event.get('row_counts_match') is not True or
                event.get('result') != 0 for event in backups)):
        raise ValueError('two successful original Linux BackupTo results are required')
    selections={}
    for line in lines:
        if line.startswith('C6_LINUX_MOD selection '):
            value=json.loads(line.removeprefix('C6_LINUX_MOD selection '))
            phase=selections.setdefault(value['phase'],{})
            if value['list'] in phase:
                raise ValueError('duplicate native Linux XP2 selection collection')
            if (value['count']!=len(value['entries']) or
                    any(entry.get('order')!=index or entry.get('resolved') is not True
                        for index,entry in enumerate(value['entries']))):
                raise ValueError('unresolved/unordered native Linux XP2 selection handles')
            phase[value['list']]=value
    if selections!=report.get('native_activation_selection'):
        raise ValueError('native Linux XP2 selection observations disagree')
    if set(selections.get('after_update',{}))!={'enabled_mods','active_mods','active_components'}:
        raise ValueError('native Linux XP2 effective selection observations missing')
    if SELECTION_SHA256 is None or identity(selections)!=SELECTION_SHA256:
        raise ValueError('unsupported Linux XP2 effective selection identity')
    if configuration.get('cleanup_requested') and (
            report.get('framework_cleanup_proven') is not True or
            'C6_LINUX_MOD cleanup_return 0' not in lines):
        raise ValueError('requested original Linux XP2 cleanup is unproven')
    if report.get('initialized_game') is not False or report.get('gameplay_parity') is not False:
        raise ValueError('unsupported Linux XP2 session/parity profile')
    return configuration, backups


def inspect(directory, report_path):
    report = json.loads(report_path.read_text())
    configuration, backups = validate_report(report)
    artifact_mapping = report['rules_snapshots']
    if set(artifact_mapping)!= {'configuration','gameplay'}:
        raise ValueError('exactly two native Linux XP2 snapshot artifacts are required')
    artifacts=list(artifact_mapping.values())
    if sorted(artifact['filename'] for artifact in artifacts) != ['configuration.sqlite', 'gameplay.sqlite']:
        raise ValueError('exactly two native Linux XP2 snapshot artifacts are required')
    databases = []
    for name in ('configuration', 'gameplay'):
        db = fingerprint(directory / (name + '.sqlite'))
        db['database'] = name
        artifact = next(item for item in artifacts if item['filename'] == db['filename'])
        if (artifact['sha256'], artifact['bytes']) != (db['file_sha256'], db['bytes']):
            raise ValueError('native Linux XP2 snapshot hash/size mismatch: ' + name)
        event = next(item for item in backups if item['database'] == name)
        if (event['tables'], event['rows']) != (db['table_count'], db['row_count']):
            raise ValueError('native Linux XP2 source/destination count mismatch: ' + name)
        databases.append(db)
    tables = {table['name']: table for table in databases[1]['semantic']['tables']}
    if any(name not in tables or not tables[name]['rows'] for name in ('Units', 'Types', 'GameSpeeds', 'Governors')):
        raise ValueError('native Linux XP2 rules remain unpopulated')
    return {'configuration': configuration, 'report_sha256': digest(report_path),
            'databases': databases, 'report': report}


def compare(a, b, reports):
    if a['configuration'] != b['configuration']:
        raise ValueError('independent Linux XP2 fixture configurations differ')
    if a['report']['native_activation_selection'] != b['report']['native_activation_selection']:
        raise ValueError('independent Linux XP2 native selection observations differ')
    if a['databases'] != b['databases']:
        raise ValueError('independent Linux XP2 typed/byte snapshots differ')
    configuration = a['configuration']
    gate = json.loads((ROOT / 'evidence/linux/rules-content-inputs.json').read_text())
    localization = configuration['localization_profile']
    return {
        'format': 'c6.reproducible-rules-snapshot.v1',
        'profile': {'id': PROFILE, 'target_sha256': gate['modules']['libGameCore_XP2.so'],
                    'original_executable_sha256': gate['modules']['Civ6'],
                    'ruleset': 'RULESET_EXPANSION_2', 'enabled_dlc_mods': [XP2],
                    'required_dlc_mods': [XP2],
                    'rules_database': 'original Linux AppModdingFramework ConfigureContent and UpdateContent, requestNone callback',
                    'coverage': 'native_XP2_rules_after_UpdateContent_diagnostic_requestNone_callback',
                    'content_recipe_sha256': gate['source_inventory_sha256'],
                    'localization': 'en_US', 'update_callback_profile': CALLBACK,
                    'callback_state': {'request': 'None', 'censorship': False,
                                       'authority': 'original LoadGameViewState constructor at 0x2fd0e0f clears word at +0x4b0; saved Begin at 0x2fd16ad is not called'}},
        'module_identities': [{'path': name, 'sha256': sha} for name, sha in configuration['assets'].items()],
        'native_configuration_sha256': CONFIGURATION,
        'source_sequence_sha256': gate['source_inventory_sha256'],
        'saved_configuration_source_sha256': configuration['saved_configuration_sha256'],
        'source_order_authority': 'Original Linux mod framework selects and applies content; installed source inventory is not an asserted action sequence.',
        'original_asl_launch_profile': configuration['original_filesystem_profile'],
        'native_activation_selection': a['report']['native_activation_selection'],
        'native_activation_selection_sha256': SELECTION_SHA256,
        'selection_order_authority': 'Original getters return ordered handle collections resolved through the actual Modding database; getter order is not asserted to be action execution order.',
        'native_lifecycle': {'framework_cleanup_proven': a['report']['framework_cleanup_proven'],
                             'client_cleanup_proven': a['report']['client_cleanup_proven']},
        'language_identity': {'localization': 'en_US', 'source_identity_sha256': identity(localization),
                              'localized_text_database_included': False},
        'native_runs': [{'evidence': str(path), 'evidence_sha256': run['report_sha256']}
                        for path, run in zip(reports, (a, b), strict=True)],
        'independent_native_builds': 2, 'file_bytes_identical': True,
        'typed_schema_rowid_content_identical': True,
        'inspector': {'source_sha256': digest(Path(__file__)),
                      'typed_inspector_sha256': digest(ROOT / 'tools/assets/rules_snapshot.py'),
                      'encoding': ENCODING, 'sqlite_version': sqlite3.sqlite_version},
        'databases': a['databases'], 'initialized_game': False,
        'limits': [
            'The genuine callback owner is constructor-initialized and requestNone; saved Request/Begin was not executed.',
            'Differences in callback fields or censorship policy versus saved Begin are not assumed irrelevant to these rules.',
            'The genuine UI manager is constructed with no graphics initialization.',
            'Lifecycle coverage is recorded separately from the immutable database backups; renderer/session teardown is not established.',
            'Enabled/required XP2 configuration is distinct from the 35 installed descriptors.',
            'No initialized game, map/player state, RNG, save deserialization, actions or turn parity established.',
            'SQLite rowids are not native DefinitionIDs; no runtime definition mapping is inferred.',
            'Native connection functions/collations/planner state and localization database bytes are outside these two backups.',
        ],
    }


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
    print(json.dumps({db['database']: db['semantic_sha256'] for db in result['databases']}))


if __name__ == '__main__':
    main()
