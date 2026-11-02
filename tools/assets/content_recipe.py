#!/usr/bin/env python3
"""Emit source-gated base database import order recovered from the original host.

The output describes original inputs and native importer calls. It never applies
XML with a replacement parser or claims the resulting game is initialized.
"""
import argparse
import json
from pathlib import Path
import xml.etree.ElementTree as ET

from inventory import digest, identity

EXE_SHA256 = 'e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63'
XP2_ID = '4873eb62-8ccc-4574-b784-dda455e74e68'


def wide_key(value):
    """Native comparator 983b40 compares unsigned UTF-16 code units."""
    encoded = value.encode('utf-16le')
    return tuple(encoded[i] + encoded[i + 1] * 256 for i in range(0, len(encoded), 2))


def base_order(root, kind):
    directory = root / 'Base/Assets' / ('Configuration' if kind == 'configuration' else 'Gameplay') / 'Data'
    schema = directory / 'Schema'
    # Host bc330 recurses schema; bf120 does not. Both recurse *.xml content.
    schemas = [path for path in (schema.rglob('*') if kind == 'configuration' else schema.glob('*'))
               if path.is_file()]
    content = [path for path in directory.rglob('*') if path.is_file() and
               path.suffix.lower() == '.xml' and path not in schemas]
    key = lambda path: wide_key(path.relative_to(root).as_posix())
    return [('schema', path) for path in sorted(schemas, key=key)] + [
        ('content', path) for path in sorted(content, key=key)]


def recipe(root):
    root = root.resolve(strict=True)
    executable = root / 'Base/Binaries/Win64Steam/CivilizationVI.exe'
    if digest(executable) != EXE_SHA256:
        raise ValueError('original host identity mismatch')
    steps = []
    for kind in ('configuration', 'gameplay'):
        for phase, path in base_order(root, kind):
            if path.is_symlink() or not path.resolve().is_relative_to(root):
                raise ValueError('source escapes original asset root')
            suffix = path.suffix.lower()
            if suffix not in ('.xml', '.sql'):
                raise ValueError('unsupported source extension: ' + str(path))
            steps.append({'database': kind, 'phase': phase,
                          'path': path.relative_to(root).as_posix(),
                          'format': suffix[1:], 'sha256': digest(path),
                          'bytes': path.stat().st_size,
                          'utf8_bom_skip': path.read_bytes().startswith(b'\xef\xbb\xbf')})
    return {'format': 'c6.original-base-content-recipe.v1',
            'host_executable_sha256': EXE_SHA256,
            'ruleset': 'RULESET_STANDARD', 'selected_mod_ids': [],
            'initialized_game': False, 'native_import_executed': False,
            'source_sequence_sha256': identity(steps), 'steps': steps,
            'native_importer': {'construct_rva': '0xbd160', 'dispatch_rva': '0xbd1f0',
                                'sql_rva': '0xbd2c0', 'xml_rva': '0xbd320',
                                'xml_parse_rva': '0x79d3a0', 'bom_skip_rva': '0xbd53f'},
            'limits': ['Base database import only; no DLC/mod actions are enabled.',
                       'Path sort and directory flags recovered statically; native import comparison still required.',
                       'Use original XML importer and registered Make_Hash; no replacement XML interpreter is supplied.',
                       'SQL schema-only diagnostic is a different, narrower seven-source sequence.']}


def xp2_declared_recipe(root, base, saved):
    """Explicit saved package selection; mod tie order remains an unverified candidate."""
    expected_save = 'a328d643201cb11ba226eee46c782f85b0df36b2a7a4e1a32372da8abe9cddb8'
    if saved.get('save_sha256') != expected_save or saved.get('saved_config_mod_ids') != {
            'ENABLED_MODS': [XP2_ID], 'REQUIRED_MODS': [XP2_ID]}:
        raise ValueError('XP2 recipe requires the pinned saved enabled/required package identity')
    root = root.resolve(strict=True)
    descriptor = root / 'DLC/Expansion2/Expansion2.modinfo'
    mod = ET.parse(descriptor).getroot()
    if mod.get('id') != XP2_ID:
        raise ValueError('Expansion2 descriptor package identity mismatch')
    extension = []
    for phase in ('FrontEndActions', 'InGameActions'):
        actions = list(mod.findall(phase + '/UpdateDatabase'))
        actions.sort(key=lambda action: int(action.findtext('Properties/LoadOrder') or '0'))
        for action in actions:
            criterion = action.get('criteria')
            if criterion:
                definition = mod.find("ActionCriteria/Criteria[@id='" + criterion + "']")
                if definition is None or definition.findtext('GameCoreInUse') != 'Expansion2':
                    raise ValueError('unsupported selected action criterion: ' + criterion)
                # The any=1 criterion is true from GameCore alone; no leader guessing.
                if len(definition) != 1 and definition.get('any') != '1':
                    raise ValueError('unsupported all-of action criterion: ' + criterion)
            files = sorted(action.findall('File'), key=lambda item: (-int(item.get('Priority', '0')), item.text.strip().encode('utf-8')))
            for item in files:
                path = (descriptor.parent / item.text.strip()).resolve(strict=True)
                if not path.is_relative_to(root) or path.suffix.lower() not in ('.xml', '.sql'):
                    raise ValueError('unsupported selected mod source')
                extension.append({'database': 'configuration' if phase == 'FrontEndActions' else 'gameplay',
                                  'phase': 'mod_content', 'path': path.relative_to(root).as_posix(),
                                  'format': path.suffix.lower()[1:], 'sha256': digest(path),
                                  'bytes': path.stat().st_size,
                                  'utf8_bom_skip': path.read_bytes().startswith(b'\xef\xbb\xbf'),
                                  'mod_id': XP2_ID, 'action_id': action.get('id'),
                                  'criterion': criterion, 'priority': int(item.get('Priority', '0'))})
    steps = []
    for kind in ('configuration', 'gameplay'):
        steps.extend(step for step in base['steps'] if step['database'] == kind)
        steps.extend(step for step in extension if step['database'] == kind)
    result = dict(base)
    result.update(format='c6.declared-xp2-content-recipe.v1', ruleset='RULESET_EXPANSION_2',
                  gamecore='Expansion2', selected_mod_ids=[XP2_ID],
                  saved_mod_identity_sha256=identity(saved),
                  descriptor={'path': descriptor.relative_to(root).as_posix(), 'sha256': digest(descriptor)},
                  source_sequence_sha256=identity(steps), steps=steps,
                  native_mod_load_order_verified=False,
                  native_file_sort_evidence={'component_query_rva': '0x107aea0',
                                             'setting_query_rva': '0x107af40',
                                             'order': ['Priority DESC', 'Path ASC']},
                  limits=['Explicit saved XP2-only package selection; no other installed DLC is enabled.',
                          'Base order and mod file Priority DESC/Path ASC recovered statically; action row-ID tie assignment still unverified.',
                          'Action LoadOrder/dependency/row-ID ordering is only partially recovered; this remains an import candidate.',
                          'Only UpdateDatabase actions included; script/color/UI/art and other service activation is absent.',
                          'No initialized game, rules database parity, or save compatibility claimed.'])
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('asset_root', type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--xp2-saved-mods', type=Path,
                        help='Emit declared XP2-only candidate using pinned saved mod identity')
    args = parser.parse_args()
    try:
        result = recipe(args.asset_root)
        if args.xp2_saved_mods:
            result = xp2_declared_recipe(args.asset_root, result,
                                         json.loads(args.xp2_saved_mods.read_text()))
    except (ValueError, OSError) as error:
        parser.exit(1, str(error) + '\n')
    if args.output.resolve().is_relative_to(args.asset_root.resolve()):
        parser.error('output must be outside original assets')
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'source_sequence_sha256': result['source_sequence_sha256'],
                      'sources': len(result['steps']), 'native_import_executed': False}))


if __name__ == '__main__':
    main()
