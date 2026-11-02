#!/usr/bin/env python3
"""Recover saved enabled/required mod identities from the pinned XP2 benchmark.

Configuration key meanings are evidenced by original EXE static initializers and
accessors, independently of differently named outer save-metadata keys.
"""
import argparse
import json
from pathlib import Path
import re
import sys
import zlib

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'saves'))
from inspect_save import Reader, inspect

GUID = re.compile(r'^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$', re.I)
KEYS = {'ENABLED_MODS': '1b8cd1c8', 'REQUIRED_MODS': 'fed47f44'}


def config_key(name):
    # Original 7a0da0 -> 7992c0: reflected CRC seed FFFFFFFF, no final XOR.
    return f'{zlib.crc32(name.encode("ascii")) ^ 0xffffffff:08x}'


def collections(node, location='configuration'):
    if isinstance(node, dict):
        if node.get('key') in KEYS.values():
            if 'items' not in node:
                raise ValueError('mod configuration key is not a collection')
            records = []
            for item in node['items']:
                entries = item.get('entries', [])
                guids = [entry['text'].lower() for entry in entries
                         if GUID.fullmatch(entry.get('text', ''))]
                if len(guids) != 1:
                    raise ValueError('expected one GUID per saved mod record')
                records.append(guids[0])
            yield {'location': location, 'key': node['key'],
                   'offset': node['offset'], 'mod_ids': records}
        for name, child in node.items():
            yield from collections(child, location + '.' + name)
    elif isinstance(node, list):
        for index, child in enumerate(node):
            yield from collections(child, location + f'[{index}]')


def recover(path):
    result = inspect(path)  # Exact save hash gate; no gameplay decompression needed.
    if path.name != 'XP2Benchmark.Civ6Save':
        raise ValueError('configuration framing only established for XP2 benchmark')
    reader = Reader(path.read_bytes())
    reader.pos = result['metadata_end']
    if (reader.u32(), reader.u32()) != (3, 1):
        raise ValueError('unsupported benchmark configuration versions')
    reader.map()
    blocks = []
    for expected in (67, 0, 0):
        if reader.u32() != 1:
            raise ValueError('unsupported configuration block version')
        block = reader.map()
        if len(block) != expected:
            raise ValueError('unexpected configuration block count')
        blocks.append(block)
    for expected in (15, 67):
        block = reader.map()
        if len(block) != expected:
            raise ValueError('unexpected configuration map count')
        blocks.append(block)
    if reader.pos != 103901:
        raise ValueError('unexpected end of benchmark configuration')
    found = list(collections(blocks))
    identities = {}
    for name, key in KEYS.items():
        if config_key(name) != key:
            raise ValueError('configuration key CRC identity mismatch')
        matches = [item for item in found if item['key'] == key]
        if len(matches) != 2 or matches[0]['mod_ids'] != matches[1]['mod_ids']:
            raise ValueError('saved mod configuration copies are missing or disagree')
        identities[name] = matches[0]['mod_ids']
    return {'format': 'c6.saved-mod-identity.v1', 'save_sha256': result['sha256'],
            'saved_config_mod_ids': identities, 'observed_collections': found,
            'initialized_game': False,
            'native_evidence': {'enabled_key_initializer_rva': '0x5390',
                                'required_key_initializer_rva': '0x5b90',
                                'hash_wrapper_rva': '0x7a0da0', 'crc_rva': '0x7992c0',
                                'enabled_count_rva': '0x100520', 'required_count_rva': '0x100ea0',
                                'save_metadata_population_rva': '0x16baf0',
                                'outer_enabled_metadata_key': '88305ebb',
                                'outer_required_metadata_key': '8427ae5c'},
            'limits': ['Saved configuration identity; current host content activation has not run.',
                       'No rules database hash, native save-load compatibility, or turn parity established.']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('save', type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    result = recover(args.save)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps(result['saved_config_mod_ids'], sort_keys=True))


if __name__ == '__main__':
    main()
