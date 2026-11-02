#!/usr/bin/env python3
"""Read-only identity inventory of locally supplied Civ VI assets; never build rules."""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3
import xml.etree.ElementTree as ET

FORMAT = 'c6.asset-inventory.v1'


def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def identity(value):
    """Canonical UTF-8 JSON identity, independent of absolute path and timestamps."""
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':'),
                                     ensure_ascii=False).encode()).hexdigest()


def sqlite_summary(path):
    # mode=ro prevents accidental file creation; query_only also forbids writes.
    with sqlite3.connect(path.resolve().as_uri() + '?mode=ro', uri=True) as connection:
        connection.execute('PRAGMA query_only=ON')
        objects = connection.execute(
            "SELECT type,name,tbl_name,sql FROM sqlite_schema ORDER BY type,name").fetchall()
        tables = []
        for kind, name, _, sql in objects:
            if kind != 'table':
                continue
            quoted = '"' + name.replace('"', '""') + '"'
            columns = connection.execute('PRAGMA table_info(' + quoted + ')').fetchall()
            tables.append({'name': name, 'rows': connection.execute(
                'SELECT count(*) FROM ' + quoted).fetchone()[0],
                'columns': [list(row) for row in columns],
                'ddl_sha256': hashlib.sha256((sql or '').encode()).hexdigest()})
        return {'schema_sha256': identity(objects), 'tables': tables,
                'user_version': connection.execute('PRAGMA user_version').fetchone()[0]}


def mod_summary(path, root):
    mod = ET.parse(path).getroot()
    actions = []
    for phase in ('FrontEndActions', 'InGameActions'):
        for index, action in enumerate(mod.findall(phase + '/*')):
            files = []
            for item in action.findall('File'):
                name = (item.text or '').strip()
                candidate = (path.parent / name).resolve()
                # A malformed descriptor must not resolve or fingerprint outside the root.
                inside = candidate.is_relative_to(root)
                files.append({'name': name, 'priority': item.get('Priority'),
                              'exists': inside and candidate.is_file()})
            actions.append({'phase': phase, 'declaration_index': index,
                            'kind': action.tag, 'id': action.get('id'),
                            'criteria': action.get('criteria'),
                            'load_order': action.findtext('Properties/LoadOrder'),
                            'files': files})
    return {'path': path.relative_to(root).as_posix(), 'id': mod.get('id'),
            'version': mod.get('version'), 'actions': actions,
            'criteria_ids': [item.get('id') for item in mod.findall('ActionCriteria/Criteria')]}


def inventory(root):
    root = root.resolve(strict=True)
    if not root.is_dir():
        raise ValueError('asset root must be a directory')
    files, mods, databases, errors = [], [], [], []
    suffixes = {}
    for path in sorted(root.rglob('*')):
        if path.is_symlink():
            raise ValueError('asset symlinks are unsupported: ' + path.relative_to(root).as_posix())
        if not path.is_file():
            continue
        relative = path.relative_to(root).as_posix()
        suffix = path.suffix.lower()
        suffixes[suffix] = suffixes.get(suffix, 0) + 1
        files.append({'path': relative, 'bytes': path.stat().st_size, 'sha256': digest(path)})
        with path.open('rb') as stream:
            sqlite = stream.read(16) == b'SQLite format 3\x00'
        if sqlite:
            try:
                databases.append({'path': relative, **sqlite_summary(path)})
            except sqlite3.Error as error:
                errors.append({'path': relative, 'error': str(error)})
        if suffix == '.modinfo':
            try:
                mods.append(mod_summary(path, root))
            except ET.ParseError as error:
                errors.append({'path': relative, 'error': str(error)})
    important = [item for item in files if item['path'].lower().endswith(
        ('.civ6save', '.sqlite', '.db', '.ini')) or
        ('/binaries/' in item['path'].lower() and item['path'].lower().endswith(('.dll', '.exe')))]
    result = {'format': FORMAT, 'scope': 'all_regular_files_under_supplied_root',
              'asset_tree_sha256': identity(files), 'file_count': len(files),
              'suffix_counts': suffixes, 'important_files': important,
              'sqlite_databases': databases, 'mods': mods, 'errors': errors,
              'limits': ['Inventory is not an enabled DLC/mod list or initialized game configuration.',
                         'Declared action order is preserved; criteria and load ordering are not executed.',
                         'No rules database is assembled and no native parity is claimed.']}
    return result, files


def compact(result):
    """Keep reproducible identity and counts without per-column/action file details."""
    report = dict(result)
    report['detail'] = 'compact'
    report['sqlite_databases'] = [
        {**{key: value for key, value in database.items() if key != 'tables'},
         'table_count': len(database['tables']),
         'row_count': sum(table['rows'] for table in database['tables'])}
        for database in result['sqlite_databases']]
    report['mods'] = [
        {**{key: value for key, value in mod.items() if key != 'actions'},
         'actions': [{**{key: value for key, value in action.items() if key != 'files'},
                      'file_count': len(action['files']),
                      'missing_file_count': sum(not item['exists'] for item in action['files'])}
                     for action in mod['actions'] if action['kind'] == 'UpdateDatabase']}
        for mod in result['mods']]
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('asset_root', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--compact', action='store_true', help='Omit per-column and action file detail')
    parser.add_argument('--file-manifest', type=Path,
                        help='Optional complete hash manifest; contains no file contents')
    args = parser.parse_args()
    result, files = inventory(args.asset_root)
    for destination in (args.output, args.file_manifest):
        if destination and destination.resolve().is_relative_to(args.asset_root.resolve()):
            parser.error('outputs must be outside the asset root')
    args.output.write_text(json.dumps(compact(result) if args.compact else result,
                                     indent=2, sort_keys=True) + '\n')
    if args.file_manifest:
        args.file_manifest.write_text(json.dumps(files, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'asset_tree_sha256': result['asset_tree_sha256'],
                      'file_count': result['file_count'], 'mods': len(result['mods']),
                      'sqlite_databases': len(result['sqlite_databases']),
                      'errors': result['errors']}, sort_keys=True))
    return bool(result['errors'])


if __name__ == '__main__':
    raise SystemExit(main())
