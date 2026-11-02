#!/usr/bin/env python3
"""Materialize pinned original SQL-only schemas for native connection diagnostics.

This does not populate game rules, import XML, resolve enabled content, or initialize
an actual game. Host SQLite execution is not parity with original DatabaseDLL.
"""
import argparse
import json
from pathlib import Path
import sqlite3
import tempfile

from inventory import digest, identity, sqlite_summary

GROUPS = {
    'configuration': [
        'Base/Assets/Configuration/Data/Schema/AdditionalTables.sql',
        'Base/Assets/Configuration/Data/Schema/HallofFame.sql',
        'Base/Assets/Configuration/Data/Schema/InputConfiguration.sql',
        'Base/Assets/Configuration/Data/Schema/SetupParameters.sql',
    ],
    'gameplay': [
        'Base/Assets/Gameplay/Data/Schema/01_GameplaySchema.sql',
        'Base/Assets/Gameplay/Data/Schema/02_AddTriggers.sql',
        'DLC/Expansion2/Data/Expansion2_Schema.sql',
    ],
}


def materialize(root, output, gate):
    root = root.resolve(strict=True)
    output = output.resolve()
    public_repo = Path(__file__).resolve().parents[2]
    if output.is_relative_to(root) or output.is_relative_to(public_repo):
        raise ValueError('generated original-derived databases must be outside source/assets roots')
    expected = {item['path']: item['sha256'] for item in gate['sources']}
    required = [name for names in GROUPS.values() for name in names]
    # Validate every source before creating anything. No unknown source fallback.
    if set(expected) != set(required):
        raise ValueError('schema identity gate must list exactly the required sources')
    for name in required:
        source = (root / name).resolve(strict=True)
        if not source.is_relative_to(root) or digest(source) != expected[name]:
            raise ValueError('schema source identity mismatch: ' + name)
    if output.exists():
        raise ValueError('output already exists; choose a new directory')
    output.parent.mkdir(parents=True, exist_ok=True)
    report = {'format': 'c6.sql-schema-bootstrap.v1',
              'phase': 'original_sql_only_schema_diagnostic',
              'initialized_game': False, 'native_database_parity': False,
              'source_identity_sha256': identity(gate['sources']),
              'host_sqlite_version': sqlite3.sqlite_version,
              'omitted': ['All XML schema descriptors and all rules/configuration content',
                          'Enabled DLC/mod criteria resolution and engine load-order verification',
                          'Make_Hash implementation required by Types/Kinds insert triggers'],
              'databases': []}
    # Atomic directory publication avoids a partially built bootstrap if SQL fails.
    with tempfile.TemporaryDirectory(prefix='c6-schema-', dir=output.parent) as temporary:
        stage = Path(temporary) / 'result'
        stage.mkdir()
        for group, names in GROUPS.items():
            path = stage / (group + '.sqlite')
            with sqlite3.connect(path) as connection:
                for name in names:
                    connection.executescript((root / name).read_text(encoding='utf-8-sig'))
            report['databases'].append({'kind': group, 'filename': path.name,
                                        'sha256': digest(path), 'sources': names,
                                        **sqlite_summary(path)})
        (stage / 'manifest.json').write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
        stage.rename(output)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('asset_root', type=Path)
    parser.add_argument('--output-dir', type=Path, required=True)
    parser.add_argument('--identity', type=Path, default=Path(__file__).resolve().parents[2] /
                        'evidence/assets/sql-schema-sources.json')
    args = parser.parse_args()
    try:
        report = materialize(args.asset_root, args.output_dir,
                             json.loads(args.identity.read_text()))
    except (ValueError, OSError, sqlite3.Error) as error:
        parser.exit(1, str(error) + '\n')
    print(json.dumps({'phase': report['phase'], 'initialized_game': False,
                      'databases': [{'kind': db['kind'], 'tables': len(db['tables']),
                                     'uri': (args.output_dir.resolve() / db['filename']).as_uri()}
                                    for db in report['databases']]}, sort_keys=True))


if __name__ == '__main__':
    main()
