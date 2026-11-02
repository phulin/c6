#!/usr/bin/env python3
"""Gate and prepare a read-only original Linux Base English localization fixture."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def prepare(assets, work, userdata=None):
    assets, work = Path(assets).resolve(), Path(work).resolve()
    gate = json.loads((ROOT / 'tools/linux/localization-sources.json').read_text())
    if digest(assets / 'Civ6') != gate['host_sha256']:
        raise ValueError('localization host SHA-256 mismatch')
    actual = {str(p.relative_to(assets)) for p in
              (assets / 'Base/assets/text/en_us').rglob('*') if p.is_file()}
    expected = {name for name in gate['files'] if '/en_us/' in name}
    if actual != expected:
        raise ValueError('localization English source inventory mismatch')
    for name, expected_hash in gate['files'].items():
        if digest(assets / name) != expected_hash:
            raise ValueError(f'localization source SHA-256 mismatch: {name}')
    text = work / 'Base/Assets/Text'
    text.mkdir(parents=True, exist_ok=True)
    aliases = {
        'localizationdatabase_schema.sql': 'LocalizationDatabase_Schema.sql',
        'localizationdatabase_data.sql': 'LocalizationDatabase_Data.sql',
        'localizationdatabase_postprocessing.sql': 'LocalizationDatabase_PostProcessing.sql',
    }
    fixture_paths = set()
    for name in gate['files']:
        relative = Path(name).relative_to('Base/assets/text')
        if relative.name == 'icudt56l.dat':
            continue
        target = text / (aliases[relative.name] if relative.name in aliases else relative)
        target.parent.mkdir(parents=True, exist_ok=True)
        source = assets / name
        if target.is_symlink() and target.resolve() != source:
            raise ValueError(f'localization fixture symlink mismatch: {target}')
        if target.exists() and not target.is_symlink():
            raise ValueError(f'localization fixture contains non-symlink file: {target}')
        if not target.is_symlink():
            target.symlink_to(source)
        fixture_paths.add(target)
    actual_fixture = {p for p in text.rglob('*') if p.is_file() or p.is_symlink()}
    if actual_fixture != fixture_paths:
        raise ValueError('localization fixture inventory mismatch')
    # Original Bootstrap paths start ../../../Base/Assets/Text/. No original
    # path literal is patched and no original asset is copied or modified.
    cwd = work / 'launch/a/b'
    cwd.mkdir(parents=True, exist_ok=True)
    userdata = Path(userdata).resolve() if userdata else ROOT / 'target/linux-runtime/userdata'
    absent = [work / 'Base/Assets/UI/Text', work / 'Base/Assets/Gameplay/UI/Text',
              userdata / "Sid Meier's Civilization VI/Text",
              cwd / 'c6-no-localization-cache.sqlite']
    for path in absent:
        if path.exists() or path.is_symlink():
            raise ValueError(f'localization fixture requires absent optional source/cache: {path}')
    gate['required_absent_paths'] = [str(path) for path in absent]
    return gate, {'C6_LOCALIZATION_BOOTSTRAP_CWD': str(cwd),
                  'C6_LOCALIZATION_ICU': str(assets / 'Base/assets/text/icudt56l.dat')}
