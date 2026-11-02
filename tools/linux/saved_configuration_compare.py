#!/usr/bin/env python3
"""Compare native Linux Game::Write with the pinned save's decoded GameConfig.

This is a structural/value comparison at one configuration boundary, not a save
loader or gameplay parity check. Raw native bytes remain private.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/saves'))
from inspect_save import Reader, read_xp2_configuration  # noqa: E402


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--save', type=Path, required=True)
    parser.add_argument('--native', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    source = args.save.read_bytes()
    if hashlib.sha256(source).hexdigest() != 'a328d643201cb11ba226eee46c782f85b0df36b2a7a4e1a32372da8abe9cddb8':
        raise SystemExit('pinned save identity mismatch')
    reader = Reader(source)
    reader.pos = 8842
    original = read_xp2_configuration(reader)['configuration_blocks'][0]['entries']
    raw = args.native.read_bytes()
    reader = Reader(raw)
    if reader.u32() != 1:
        raise SystemExit('unexpected native GameConfig version')
    actual = reader.map()
    if reader.pos != len(raw):
        raise SystemExit('native serialization has trailing bytes')
    changes = []
    flag_changes = []

    def compare(left, right, path=''):
        if isinstance(left, list) and isinstance(right, list):
            if all(isinstance(x, dict) and 'key' in x for x in left + right):
                a, b = {x['key']: x for x in left}, {x['key']: x for x in right}
                for key in sorted(set(a) | set(b)):
                    if key not in a or key not in b:
                        changes.append({'path': f'{path}/{key}', 'saved': a.get(key), 'native': b.get(key)})
                    else:
                        compare(a[key], b[key], f'{path}/{key}')
                return
            if len(left) != len(right):
                changes.append({'path': path, 'saved_count': len(left), 'native_count': len(right)})
                return
            for index, (a, b) in enumerate(zip(left, right)):
                compare(a, b, f'{path}/{a.get("key", index)}')
        elif isinstance(left, dict) and isinstance(right, dict):
            for key in sorted(set(left) | set(right)):
                if key == 'offset':
                    continue
                if key == 'flags' and left.get(key) != right.get(key):
                    flag_changes.append({'path': path, 'saved': left.get(key), 'native': right.get(key)})
                elif key not in left or key not in right:
                    changes.append({'path': f'{path}/{key}', 'saved': left.get(key), 'native': right.get(key)})
                else:
                    compare(left[key], right[key], f'{path}/{key}')
        elif left != right:
            changes.append({'path': path, 'saved': left, 'native': right})

    compare(original, actual)
    result = {'schema': 1, 'native_bytes': len(raw),
              'native_sha256': hashlib.sha256(raw).hexdigest(),
              'saved_game_configuration_bytes': 65937 - 8914,
              'value_differences': changes, 'flag_differences': flag_changes,
              'value_match': not changes, 'byte_match': raw == source[8914:65937],
              'initialized_game': False, 'action_or_turn_parity': False}
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'value_match': not changes, 'value_differences': len(changes),
                      'flag_differences': len(flag_changes)}))


if __name__ == '__main__':
    main()
