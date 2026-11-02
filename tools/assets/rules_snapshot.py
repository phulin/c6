#!/usr/bin/env python3
"""Read-only typed, rowid-preserving fingerprints of genuinely backed-up rules DBs.

No database is generated here. Supply native backup evidence alongside each pair.
The format fingerprints SQL values, persistent schema and rowids, not SQLite's
physical btree layout or unrecorded native connection functions/collations.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3
import struct

from inventory import digest, identity

FORMAT = 'c6.rules-snapshot.v1'
ENCODING = 'c6.sqlite-typed-rowid.v1'


class TextBytes(bytes):
    """Preserve TEXT bytes without conflating them with BLOB or decoding loss."""


def field(value):
    if value is None:
        return b'N'
    if isinstance(value, int):
        return b'I' + struct.pack('>q', value)
    if isinstance(value, float):
        return b'R' + struct.pack('>d', value)
    if isinstance(value, TextBytes):
        return b'T' + struct.pack('>Q', len(value)) + value
    if isinstance(value, bytes):
        return b'B' + struct.pack('>Q', len(value)) + value
    raise ValueError('unsupported SQLite value type: ' + type(value).__name__)


def record(row):
    return b'A' + struct.pack('>Q', len(row)) + b''.join(field(value) for value in row)


def rows_digest(rows):
    checksum = hashlib.sha256(ENCODING.encode() + b'\0')
    count = 0
    for row in rows:
        checksum.update(record(row))
        count += 1
    return checksum.hexdigest(), count


def text(value):
    if not isinstance(value, TextBytes):
        raise ValueError('expected SQLite TEXT metadata')
    return value.decode('utf-8', errors='strict')


def quote(name):
    return '"' + name.replace('"', '""') + '"'


def fingerprint(path):
    before = digest(path)
    if any(Path(str(path) + suffix).exists() for suffix in ('-wal', '-journal')):
        raise ValueError('snapshot must have no external WAL/journal state')
    with sqlite3.connect(path.resolve().as_uri() + '?mode=ro', uri=True) as connection:
        connection.text_factory = TextBytes
        connection.execute('PRAGMA query_only=ON')
        if connection.execute('PRAGMA integrity_check').fetchall() != [(TextBytes(b'ok'),)]:
            raise ValueError('SQLite snapshot integrity check failed')
        connection.execute('BEGIN')
        schema = connection.execute(
            'SELECT rowid,type,name,tbl_name,sql FROM sqlite_schema ORDER BY rowid').fetchall()
        schema_hash, schema_count = rows_digest(schema)
        table_flags = {text(row[1]): row[4] for row in connection.execute('PRAGMA table_list')
                       if text(row[0]) == 'main'}
        tables = []
        for schema_row in schema:
            if schema_row[1] != b'table':
                continue
            name = text(schema_row[2])
            ddl = text(schema_row[4]) if schema_row[4] is not None else ''
            if ddl.upper().startswith('CREATE VIRTUAL TABLE'):
                raise ValueError('virtual table extension state is unsupported: ' + name)
            columns = connection.execute('PRAGMA table_xinfo(' + quote(name) + ')').fetchall()
            if not columns or name not in table_flags:
                raise ValueError('missing required table metadata: ' + name)
            column_hash, _ = rows_digest(columns)
            without_rowid = bool(table_flags[name])
            selected = '*'
            if without_rowid:
                primary = sorted((row[5], text(row[1])) for row in columns if row[5])
                if not primary:
                    raise ValueError('WITHOUT ROWID table lacks primary key: ' + name)
                order = ','.join(quote(item[1]) for item in primary)
                row_identity = {'kind': 'without_rowid_primary_key', 'columns': [item[1] for item in primary]}
            else:
                names = {text(row[1]).lower() for row in columns}
                alias = next((alias for alias in ('rowid', '_rowid_', 'oid') if alias not in names), None)
                if alias is None:
                    raise ValueError('all intrinsic rowid aliases shadowed: ' + name)
                selected = quote(alias) + ',*'
                order = quote(alias)
                row_identity = {'kind': 'intrinsic_rowid', 'alias': alias}
            checksum, count = rows_digest(connection.execute(
                'SELECT ' + selected + ' FROM ' + quote(name) + ' ORDER BY ' + order))
            tables.append({'name': name, 'schema_rowid': schema_row[0], 'rows': count,
                           'row_identity': row_identity, 'columns_sha256': column_hash,
                           'typed_rows_sha256': checksum})
        persistent = {}
        for pragma in ('application_id', 'user_version', 'encoding'):
            value = connection.execute('PRAGMA ' + pragma).fetchone()[0]
            persistent[pragma] = text(value) if isinstance(value, TextBytes) else value
        connection.rollback()
    if digest(path) != before:
        raise ValueError('snapshot changed during read-only inspection')
    semantic = {'encoding': ENCODING, 'schema_sha256': schema_hash,
                'schema_objects': schema_count, 'persistent_pragmas': persistent,
                'tables': tables}
    return {'filename': path.name, 'bytes': path.stat().st_size, 'file_sha256': before,
            'semantic_sha256': identity(semantic), 'semantic': semantic,
            'table_count': len(tables), 'row_count': sum(table['rows'] for table in tables)}


def snapshot(directory, native_evidence):
    evidence = json.loads(native_evidence.read_text())
    events = evidence.get('observations', evidence.get('events', []))
    backups = [event for event in events if event.get('phase') == 'original_database_backup']
    if evidence.get('returncode') != 0 or len(backups) != 2 or any(
            event.get('result') != 0 or not event.get('native_success') or
            not event.get('row_counts_match') for event in backups):
        raise ValueError('successful genuine native backup evidence for both databases is required')
    imported = [event for event in events if event.get('phase') == 'original_source_import']
    if len(imported) != 213 or any(event.get('native_status') != 0 for event in imported):
        raise ValueError('the established 213-source native XP2 fixture is required')
    artifacts = evidence.get('rules_snapshots', [])
    if sorted(item.get('filename', '') for item in artifacts) != ['configuration.sqlite', 'gameplay.sqlite']:
        raise ValueError('exactly the two native snapshot artifacts are required')
    configuration = evidence.get('configuration', {})
    if evidence.get('configuration_sha256') != identity(configuration):
        raise ValueError('native configuration identity mismatch')
    if configuration.get('target_sha256') != '324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c':
        raise ValueError('unsupported original GameCore identity')
    if configuration.get('original_executable_sha256') != 'e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63':
        raise ValueError('unsupported original executable identity')
    databases = []
    for kind in ('configuration', 'gameplay'):
        report = fingerprint(directory / (kind + '.sqlite'))
        report['database'] = kind
        artifact = next((item for item in artifacts if item.get('filename') == report['filename']), None)
        if artifact is None or (artifact.get('sha256'), artifact.get('bytes')) != (report['file_sha256'], report['bytes']):
            raise ValueError('snapshot does not match native runtime artifact hash/size: ' + kind)
        native = next((event for event in backups if event.get('database') == kind), None)
        if native is None or (report['table_count'], report['row_count']) != (native['tables'], native['rows']):
            raise ValueError('native backup and inspected row/table counts disagree: ' + kind)
        databases.append(report)
    return {'format': FORMAT, 'inspector_sqlite_version': sqlite3.sqlite_version,
            'native_evidence_sha256': digest(native_evidence),
            'inspector_source_sha256': digest(Path(__file__)),
            'native_configuration_sha256': evidence.get('configuration_sha256'),
            'native_configuration': evidence.get('configuration'),
            'language_identity': {'localization': evidence.get('configuration', {}).get('localization'),
                                  'localized_text_database_included': False},
            'databases': databases, 'initialized_game': False,
            'limits': ['Original-native explicit source fixture; component activation engine not executed.',
                       'Typed semantic digest preserves schema creation rowids and intrinsic data rowids.',
                       'Host SQLite reads SQL values; native custom collations/functions and connection pragmas are not serialized.',
                       'Planner equivalence, btree scan order without ORDER BY, full startup/save/turn parity are unproven.']}


def compare(first, second, evidence_paths):
    if first['native_configuration_sha256'] != second['native_configuration_sha256']:
        raise ValueError('independent native fixture configuration identities differ')
    if first['inspector_sqlite_version'] != second['inspector_sqlite_version']:
        raise ValueError('inspector SQLite versions differ')
    for a, b in zip(first['databases'], second['databases'], strict=True):
        for key in ('database', 'file_sha256', 'semantic_sha256', 'bytes', 'table_count', 'row_count'):
            if a[key] != b[key]:
                raise ValueError('native rules snapshots differ: ' + a['database'] + ' ' + key)
    configuration = first['native_configuration']
    return {'format': 'c6.reproducible-rules-snapshot.v1',
            'inspector': {'source_sha256': first['inspector_source_sha256'],
                          'sqlite_version': first['inspector_sqlite_version'], 'encoding': ENCODING},
            'profile': {key: configuration[key] for key in (
                'target_sha256', 'original_executable_sha256', 'ruleset', 'enabled_dlc_mods',
                'content_recipe_sha256', 'localization', 'rules_database')},
            'module_identities': [{'path': item['path'], 'sha256': item['sha256']}
                                  for item in configuration['dependencies']],
            'source_sequence_sha256': configuration['content_recipe']['source_sequence_sha256'],
            'native_configuration_sha256': first['native_configuration_sha256'],
            'native_runs': [{'evidence': str(path), 'evidence_sha256': item['native_evidence_sha256']}
                            for path, item in zip(evidence_paths, (first, second), strict=True)],
            'independent_native_builds': 2, 'file_bytes_identical': True,
            'typed_schema_rowid_content_identical': True,
            'language_identity': first['language_identity'],
            'databases': first['databases'], 'initialized_game': False, 'limits': first['limits']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--native-evidence', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--compare-directory', type=Path)
    parser.add_argument('--compare-native-evidence', type=Path)
    args = parser.parse_args()
    if bool(args.compare_directory) != bool(args.compare_native_evidence):
        parser.error('both comparison inputs are required')
    result = snapshot(args.directory, args.native_evidence)
    if args.compare_directory:
        other = snapshot(args.compare_directory, args.compare_native_evidence)
        result = compare(result, other, (args.native_evidence, args.compare_native_evidence))
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps({db['database']: db['semantic_sha256'] for db in result['databases']}, sort_keys=True))


if __name__ == '__main__':
    main()
