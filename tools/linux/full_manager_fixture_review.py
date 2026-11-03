#!/usr/bin/env python3
"""Independent native-caller source/lifetime review using mocks only."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tempfile

from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    paths = ['native/linux/full_manager_fixture.c', 'native/linux/full_manager_fixture.h',
             'tools/linux/full_manager_fixture_audit.py',
             'tools/linux/full_manager_fixture_review.c',
             'tools/linux/full_manager_fixture_review.py']
    before = {path: digest(ROOT / path) for path in paths}
    with tempfile.TemporaryDirectory(prefix='c6-full-manager-review-') as temporary:
        work = Path(temporary)
        executable = work / 'review'
        flags = ['cc', '-std=gnu11', '-Wall', '-Wextra', '-Werror', '-Wconversion', '-Wshadow']
        source = str(ROOT / 'tools/linux/full_manager_fixture_review.c')
        subprocess.run(flags + ['-fanalyzer', '-c', source, '-o', str(work / 'review.o')], check=True)
        subprocess.run(flags + ['-O2', '-fsanitize=undefined', '-fno-sanitize-recover=undefined',
                                source, '-ldl', '-o', str(executable)], check=True)
        run = subprocess.run([str(executable)], text=True, capture_output=True, check=True)
        audit_path = work / 'producer-audit.json'
        subprocess.run(['python3', str(ROOT / 'tools/linux/full_manager_fixture_audit.py'),
                        '--output', str(audit_path)], capture_output=True, text=True, check=True)
        audit = json.loads(audit_path.read_text())
        binary_sha256 = digest(executable)
    after = {path: digest(ROOT / path) for path in paths}
    if before != after:
        raise RuntimeError('reviewed source changed while checking; rerun immutable sources')
    cases = []
    pattern = r'case (\d+): exit=(\d+) calls=(\d+) ctor=(\d+) dtor=(\d+) acquire=(\d+) release=(\d+) saves=(\d+)'
    for row in re.findall(pattern, run.stdout):
        cases.append(dict(zip(['case', 'exit', 'queries', 'constructed', 'destroyed',
                               'facades_acquired', 'facades_released', 'saves'], map(int, row))))
    if len(cases) != 18:
        raise ValueError('independent case inventory differs')
    game = Elf(ROOT.parent / 'c6-assets-linux/libGameCore_XP2.so', GAME_SHA)
    host = Elf(ROOT.parent / 'c6-assets-linux/Civ6', HOST_SHA)
    for address, expected in [(0x290BBA9, 'ff5008'), (0x2C85D31, 'ff6140'),
                              (0x2C85DAB, 'ff5138')]:
        if game.read(address, len(expected) // 2).hex() != expected:
            raise ValueError('original facade/utility dispatch differs')
    if game.relocated_pointer(0x3AECF68) != 0x2C86370:
        raise ValueError('facade Destroy relocation differs')
    for slot, target in [(0x30, 0x32B3DA0), (0x38, 0x32B3D60), (0x40, 0x32B3D40)]:
        if int.from_bytes(host.read(0x53A6C90 + slot, 8), 'little') != target:
            raise ValueError('host utility slot direction differs')
    dwarf = subprocess.run(['gdb', '-q', '-nx', '-batch',
                            str(ROOT.parent / 'c6-assets-linux/libGameCore_XP2.so'),
                            '-ex', "ptype /o 'GameCore::Unit::Operations::Operation'"],
                           text=True, capture_output=True, check=True).stdout
    if not all(text in dwarf for text in ['total size (bytes):   32', '8      |      16', '24      |       4']):
        raise ValueError('original Operation layout differs')
    report = dict(
        schema='c6-linux-full-manager-fixture-independent-review-v1',
        reviewed_checkpoints=['25646c0', '9099b51'],
        gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA,
        source_sha256=after, source_stable_during_checks=True,
        synthetic_executable_sha256=binary_sha256,
        independent_lifetime_cases=cases,
        checks=dict(strict_c=True, gcc_analyzer=True, ubsan_no_recovery=True,
                    protected_unit_page_after_query=True,
                    mutating_save_stubs_before_actor_resolution=True,
                    original_facade_dispatch_and_operation_dwarf=True),
        producer_audit=dict(checked_exports=len(audit['checked_exports']),
                            checked_method_bodies=len(audit['checked_methods']),
                            checks=audit['checks']),
        remaining=['concrete field-only guard source and exact runner/profile admission',
                   'actual mapped/live code and owner/main-entry/lock witnesses',
                   'hardware observer lifecycle and safe original script windows',
                   'actual Save/callback/VM effects and complete affected gameplay/RNG comparison'],
        original_execution=False, original_modules_loaded_for_execution=False,
        runtime_profile_admitted=False, native_manager_parity=False,
        complete_state_action_turn_acceptance=False)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
    print('18 independent mock lifetime/refusal cases and original ABI gates passed; no original execution')


if __name__ == '__main__':
    main()
