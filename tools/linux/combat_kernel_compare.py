#!/usr/bin/env python3
"""Prepare synthetic average-damage cases or compare a gated loaded native run."""
import argparse
import hashlib
import itertools
import json
from pathlib import Path
import random
import re
import subprocess
import time

from combat_kernel_audit import ROOT, XP2, digest

SOURCE_FILES = ['native/linux/combat_kernel.c', 'native/linux/combat_kernel.h',
                'tools/linux/combat_kernel_audit.py', 'tools/linux/combat_kernel_compare.py',
                'crates/c6-core/src/combat.rs', 'crates/c6-core/examples/combat_kernel.rs']
GARRISON = 0x5e97d629


def signed32(n):
    return ((n + 2**31) % 2**32) - 2**31


def fixtures():
    # Columns: attacker/defender raw Q8, defender HP cap, outer damage/max,
    # defense hash, raw Q8 power scale, minimum/base, ignored dampener/extra,
    # bypass. No Unit/Map state or random fixture selection enters gameplay.
    base = [20*256, 20*256, 100, 0, 100, 0xffffffff, 10, 1, 24, 0, 0, 0]
    result = []
    def add(**changes):
        p = base.copy()
        for index, value in changes.items():
            p[int(index)] = value
        result.append(p)
    # Dense fractional-strength differences exercise expf rounding transitions,
    # both damage caps and low/high damage ranges, including negative strengths.
    for difference, scale in itertools.product(range(-32768, 32769, 17), [0, 1, 9, 10, 11, 256]):
        add(**{'0': 20*256 + difference, '6': scale})
    edges = [-2**31, -2**31+1, -16777216, -8388608, -257, -256, -255,
             -1, 0, 1, 127, 128, 255, 256, 257, 8388607, 16777216, 2**31-1]
    for index, value in itertools.product(range(11), edges):
        add(**{str(index): value if index != 5 else value % 2**32})
    for attacker, defender, minimum, maximum in itertools.product(
            [0, 1, -1, 20*256], [0, 1, -1, 20*256], [-5, 0, 1, 101], [-1, 0, 100]):
        add(**{'0': attacker, '1': defender, '7': minimum, '2': maximum})
    for defense, bypass, outer_max, outer_damage in itertools.product(
            [GARRISON, 1839557181, 0xffffffff, 0], [0, 1],
            [-1, 0, 1, 3, 100, 8388607, 8388608, 16777216, 2**31-1],
            [-2**31, -1, 0, 1, 2, 99, 100, 101, 2**31-1]):
        add(**{'3': outer_damage, '4': outer_max, '5': defense, '11': bypass})
    # Broad deterministic full-width arithmetic; Python RNG only selects cases.
    chooser = random.Random(0xc606da6a)
    for _ in range(8192):
        p = [chooser.randint(-2**31, 2**31-1) for _ in range(11)] + [chooser.randrange(2)]
        p[5] = chooser.choice([GARRISON, 1839557181, 0xffffffff])
        result.append(p)
    safe, omitted = [], 0
    for p in result:
        remaining = signed32((p[4] - p[3]) << 8)
        # Preserve native SIGFPE boundary as an explicit unsupported fixture;
        # do not crash a combined oracle run before observing all valid cases.
        invalid = (p[0] != 0 and p[1] != 0 and p[5] == GARRISON and p[11] == 0
                   and remaining > 0 and p[4] > 0 and signed32(p[4] << 8) == 0)
        if invalid:
            omitted += 1
        else:
            safe.append(p)
    return safe, omitted


def prepare(args):
    cases, omitted = fixtures()
    corpus = ''.join(' '.join(map(str, row)) + '\n' for row in cases)
    args.input.parent.mkdir(parents=True, exist_ok=True)
    if args.input.exists() and args.input.read_text() != corpus:
        raise ValueError('refusing to overwrite a different immutable corpus')
    args.input.write_text(corpus)
    manifest = dict(schema='c6-linux-xp2-average-damage-input-v1', input_sha256=digest(args.input),
                    input_bytes=args.input.stat().st_size, case_count=len(cases),
                    omitted_native_divide_by_zero=omitted, use_average=True,
                    source_sha256={path: digest(ROOT/path) for path in SOURCE_FILES},
                    scope='Synthetic damage parameter records; real loaded global maximum extra damage required')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps(manifest, indent=2))


def compare(args):
    manifest = json.loads(args.manifest.read_text())
    if digest(args.input) != manifest['input_sha256']:
        raise ValueError('frozen synthetic input digest differs')
    for path, sha in manifest['source_sha256'].items():
        if digest(ROOT/path) != sha:
            raise ValueError('frozen source digest differs: ' + path)
    profile = json.loads(args.native_profile.read_text())
    if profile.get('returncode') != 0 or not profile.get('framework_cleanup_proven'):
        raise ValueError('native loaded-context profile did not exit successfully')
    configuration = profile['configuration']
    identity = hashlib.sha256(json.dumps(configuration, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    if identity != profile['configuration_sha256']:
        raise ValueError('native configuration identity differs')
    kernel = configuration['lower_core_average_damage_kernel']
    if (kernel['input_sha256'] != manifest['input_sha256'] or kernel['input_bytes'] != manifest['input_bytes']
            or kernel['case_count'] != manifest['case_count'] or kernel['use_average'] is not True
            or kernel['contract']['module_sha256'] != XP2):
        raise ValueError('native arithmetic configuration does not match frozen corpus/target')
    for path in SOURCE_FILES[:3]:
        if configuration['sources'][path] != manifest['source_sha256'][path]:
            raise ValueError('native arithmetic source identity differs: ' + path)
    artifact = profile['combat_kernel_output']
    if artifact['sha256'] != digest(args.native_output) or artifact['bytes'] != args.native_output.stat().st_size:
        raise ValueError('native arithmetic output identity differs')
    saves = profile['game_saves']
    if (saves['save']['sha256'] != saves['repeat']['sha256'] or saves['save']['bytes'] <= 0
            or saves['save']['bytes'] != saves['repeat']['bytes']):
        raise ValueError('original Core Saves changed across average arithmetic probe')
    marker = f"C6_LINUX_COMBAT_KERNEL average_cases {manifest['case_count']} random_unchanged 1 parameter_unchanged 1 result 0"
    if marker not in profile['stderr'].splitlines():
        raise ValueError('complete original RNG/parameter preservation observation unavailable')
    if 'C6_LINUX_COMBAT_KERNEL core_bindings xp2 9 host 6 direct_entries 5' not in profile['stderr'].splitlines():
        raise ValueError('independently audited original code/global dependency bindings unavailable')
    native = args.native_output.read_text().splitlines()
    header = native.pop(0).split()
    if len(header) != 2 or header[0] != 'max_extra_damage':
        raise ValueError('native rule header unavailable')
    extra = int(header[1])
    libm_line = native.pop(0)
    if not libm_line.startswith('libm_path '):
        raise ValueError('original exponential runtime module unavailable')
    libm = Path(libm_line.removeprefix('libm_path '))
    if not libm.is_absolute():
        libm = Path(configuration['cwd'])/libm
    libm_sha = digest(libm)
    bindings = native.pop(0).split()
    if len(bindings) != 3 or bindings[0] != 'libm_binding_offsets':
        raise ValueError('actual original arithmetic bindings unavailable')
    expf_offset, floorf_offset = map(int, bindings[1:])
    def fp_record(line, label):
        words = line.split()
        if len(words) != 4 or words[0] != label:
            raise ValueError('original floating environment unavailable: ' + label)
        return dict(fe_round=int(words[1]), mxcsr=int(words[2]), x87_control=int(words[3]))
    fp_before = fp_record(native.pop(0), 'floating_environment_before')
    fp_after = fp_record(native.pop(), 'floating_environment_after')
    if ((fp_before['mxcsr'] & ~63) != (fp_after['mxcsr'] & ~63)
            or fp_before['fe_round'] != fp_after['fe_round'] or fp_before['x87_control'] != fp_after['x87_control']):
        raise ValueError('original arithmetic changed floating-point control state')
    observations = [line.split() for line in native]
    if len(observations) != manifest['case_count'] or any(
            len(row) != 2 or int(row[0]) != i for i, row in enumerate(observations)):
        raise ValueError('incomplete/invalid original native observations')
    expected = [int(row[1]) for row in observations]
    variants, mismatches = [], []
    input_lines = args.input.read_text().splitlines()
    for release in [False, True]:
        command = ['cargo', 'build', '-p', 'c6-core', '--example', 'combat_kernel']
        if release:
            command.append('--release')
        subprocess.run(command, cwd=ROOT, check=True)
        executable = ROOT/'target'/('release' if release else 'debug')/'examples/combat_kernel'
        started = time.perf_counter()
        answer = subprocess.run([str(executable), str(extra)], input=args.input.read_text(),
                                capture_output=True, text=True, check=True)
        elapsed = time.perf_counter() - started
        actual = [int(line) for line in answer.stdout.splitlines()]
        match = re.search(r'^C6_COMBAT_CANDIDATE_FP (\d+) (\d+)$', answer.stderr, re.M)
        if not match:
            raise ValueError('candidate floating environment unavailable')
        mxcsr, x87_control = map(int, match.groups())
        offset = re.search(r'^C6_COMBAT_CANDIDATE_EXPF_OFFSET (\d+)$', answer.stderr, re.M)
        if offset is None or int(offset[1]) != expf_offset:
            raise ValueError('actual candidate exponential implementation differs from original binding')
        if (mxcsr & ~63) != (fp_before['mxcsr'] & ~63) or x87_control != fp_before['x87_control']:
            raise ValueError('candidate inherited floating-point control differs from original context')
        candidate_libm = None
        for line in subprocess.check_output(['ldd', str(executable)], text=True).splitlines():
            found = re.search(r'libm\.so\.\d+ => (/\S+)', line)
            if found:
                candidate_libm = Path(found[1])
        if candidate_libm is None or digest(candidate_libm) != libm_sha:
            raise ValueError('candidate exponential runtime module differs from original context')
        if len(actual) != len(expected):
            raise ValueError('Rust candidate refused supported cases')
        for i, (a, b) in enumerate(zip(actual, expected)):
            if a != b:
                mismatches.append(dict(release=release, index=i, native=b, rust=a,
                                       input=input_lines[i]))
        variants.append(dict(release=release, executable_sha256=digest(executable),
                             mxcsr=mxcsr, x87_control=x87_control, libm_sha256=digest(candidate_libm), expf_offset=expf_offset,
                             process_seconds=elapsed, stdout_sha256=hashlib.sha256(answer.stdout.encode()).hexdigest()))
        if release:
            measured = subprocess.run([str(executable), str(extra), '100'], input=args.input.read_text(),
                                      capture_output=True, text=True, check=True)
            benchmark = json.loads(measured.stdout)
            if benchmark['checksum'] != sum(expected) * 100:
                raise ValueError('timed arithmetic checksum differs from native observations')
    report = dict(schema='c6-linux-xp2-average-damage-comparison-v1',
                  input_manifest=manifest, native_profile_file=str(args.native_profile),
                  candidate_input_manifest_sha256=digest(args.manifest),
                  original_input_manifest_sha256=configuration['sources']['evidence/linux/combat-kernel-input.json'],
                  native_profile_sha256=digest(args.native_profile), native_output_sha256=digest(args.native_output),
                  global_max_extra_damage=extra, comparisons=len(expected)*len(variants),
                  native_fp_before=fp_before, native_fp_after=fp_after, native_libm_sha256=libm_sha,
                  native_expf_offset=expf_offset, native_floorf_offset=floorf_offset,
                  exponential_symbol_version='GLIBC_2.2.5',
                  math_profile_scope='Measured Linux x86-64 GNU/libm implementation and FP controls only; not generic cross-platform reproducibility',
                  consecutive_native_saves_equal=True, original_sync_random_unchanged=True,
                  original_dependency_binding_counts=dict(xp2=9, host=6, direct_xp2_entries=5),
                  variants=variants, mismatches=mismatches, rust_release_benchmark=benchmark,
                  limits=['Average damage arithmetic only; no combat actors/modifiers/attacks or randomized damage',
                          'Runtime profile separately proves module/rules/configuration and unchanged Core Save controls',
                          'Process timings include I/O and parsing; not pure kernel throughput or turn performance',
                          'Original divide-by-zero inputs are excluded and fail explicitly in Rust'])
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(comparisons=report['comparisons'], mismatches=len(mismatches), global_max_extra_damage=extra)))
    if mismatches:
        raise SystemExit(1)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    sub = ap.add_subparsers(dest='command', required=True)
    prep = sub.add_parser('prepare')
    prep.add_argument('--input', type=Path, required=True)
    prep.add_argument('--output', type=Path, required=True)
    comp = sub.add_parser('compare')
    for flag in ['input', 'manifest', 'native-output', 'native-profile', 'output']:
        comp.add_argument('--'+flag, type=Path, required=True)
    args = ap.parse_args()
    prepare(args) if args.command == 'prepare' else compare(args)


if __name__ == '__main__':
    main()
