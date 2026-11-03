#!/usr/bin/env python3
"""Offline full-entrypoint source fixtures; no original execution or native parity."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def original_contracts():
    game = Elf(ROOT.parent / 'c6-assets-linux/libGameCore_XP2.so', GAME_SHA)
    Elf(ROOT.parent / 'c6-assets-linux/Civ6', HOST_SHA)
    instructions = {
        0x2D28A88: bytes.fromhex('41ff5068'),  # Execute -> Manager.Add
        0x2DA1A6D: bytes.fromhex('ff5070'),    # Add -> Clear before handler
        0x2DA1A94: bytes.fromhex('ff9080000000'),
        0x2DA283C: bytes.fromhex('ffd5'),      # Activate -> CanStart
    }
    for site, expected in instructions.items():
        if game.read(site, len(expected)) != expected:
            raise ValueError('original order/dispatch differs at ' + hex(site))
    slots = {0x50: 0x2DA20F0, 0x68: 0x2DA1A20, 0x70: 0x2DA2710}
    for slot, expected in slots.items():
        if game.relocated_pointer(0x3AFF870 + 16 + slot) != expected:
            raise ValueError('original Manager slot differs')
    return dict(gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA,
                dispatch_sites=[hex(site) for site in instructions],
                manager_slots={hex(slot): hex(target) for slot, target in slots.items()},
                original_execution=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    paths = ['crates/c6-data/src/loaded_actions.rs',
             'crates/c6-data/src/loaded_actions/resources.rs',
             'crates/c6-core/src/loaded_state.rs',
             'crates/c6-core/src/unit_operation_legality.rs',
             'crates/c6-core/src/lib.rs', 'crates/c6-data/src/lib.rs',
             'crates/c6-data/src/unit_initialization_definitions.rs',
             'crates/c6-data/src/unit_movement_state.rs',
             'tools/linux/loaded_actions_review.rs',
             'tools/linux/loaded_actions_review_load.rs',
             'tools/linux/loaded_actions_review.py', 'Cargo.lock']
    before = {path: digest(ROOT / path) for path in paths}
    with tempfile.TemporaryDirectory(prefix='c6-loaded-actions-review-') as temporary:
        manifest = Path(temporary) / 'Cargo.toml'
        manifest.write_text('[package]\nname="loaded-actions-review"\nversion="0.0.0"\n'
                            'edition="2021"\n[dependencies]\nserde_json="1"\n'
                            'c6-core={path=' + json.dumps(str(ROOT / 'crates/c6-core')) + '}\n'
                            'c6-data={path=' + json.dumps(str(ROOT / 'crates/c6-data')) + '}\n'
                            '[[bin]]\nname="review"\npath='
                            + json.dumps(str(ROOT / 'tools/linux/loaded_actions_review.rs')) + '\n')
        shutil.copyfile(ROOT / 'Cargo.lock', Path(temporary) / 'Cargo.lock')
        result = subprocess.run(['cargo', 'run', '--release', '--offline', '--manifest-path',
                                 str(manifest), '--target-dir', '/tmp/c6-loaded-actions-review-target',
                                 '--', str(ROOT)], text=True, capture_output=True)
        if result.returncode:
            raise RuntimeError(result.stderr)
        lock_sha256 = digest(Path(temporary) / 'Cargo.lock')
    report = json.loads(result.stdout)
    after = {path: digest(ROOT / path) for path in paths}
    if before != after:
        raise RuntimeError('tested source changed during build/run; rerun against immutable checkpoint')
    report['source_sha256'] = after
    report['source_stable_during_build_and_run'] = True
    report['candidate_sha256'] = digest(Path('/tmp/c6-loaded-actions-review-target/release/review'))
    report['candidate_lock_sha256'] = lock_sha256
    report['original_source_contracts'] = original_contracts()
    report['schema'] = 'c6-linux-loaded-actions-independent-review-v1'
    report['reviewed_original_candidates'] = ['b59e39d', '15f4a90']
    report['corrected_checkpoints'] = ['c5baaeb', '44c2225', '1ffc015', '2ec623f', '40c9775']
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
    print('independent full loaded entrypoint source fixtures passed; no native parity')


if __name__ == '__main__':
    main()
