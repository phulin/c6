#!/usr/bin/env python3
"""Independent movement collector review. Runs synthetic C only, never originals."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parents[2]
SOURCES = [
    'tools/linux/unit_movement_query_review.c',
    'tools/linux/unit_movement_query_review.py',
    'native/linux/unit_movement_query_fixture.c',
    'native/linux/unit_movement_query_fixture.h',
    'native/linux/unit_movement_query_profile.c',
    'native/linux/unit_movement_query_profile_guard.c',
    'native/linux/unit_movement_query_profile_guard.h',
    'native/linux/action_lifecycle.c', 'native/linux/action_lifecycle.h',
    'native/linux/action_execution.h', 'native/linux/action_observer.h',
    'native/linux/action_dispatch.h', 'native/linux/action_registration.h',
    'native/linux/game_load.c', 'native/linux/mod_activation.c',
    'tools/check/unit_movement_lifecycle_test.py',
    'evidence/linux/unit-movement-query-abi.json',
    'evidence/linux/unit-movement-query-profile-source.json',
]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(value, message):
    if not value:
        raise ValueError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    args = parser.parse_args()
    directory = args.directory.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    source = directory / 'source'
    source.mkdir()
    payloads = {name: (ROOT / name).read_bytes() for name in SOURCES}
    # Production preimages are pinned independently of current shared HEAD/WIP.
    pins = {name: ('68e3ce3' if 'action_lifecycle' in name else
                   'f900235' if name.endswith(('game_load.c', 'mod_activation.c')) else
                   'bf76da7') for name in SOURCES if name.startswith('native/')}
    for name, pin in pins.items():
        require(payloads[name] == subprocess.check_output(
            ['git', 'show', pin + ':' + name], cwd=ROOT), 'production source changed: ' + name)
    for name, raw in payloads.items():
        path = source / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)
    with tarfile.open(directory / 'source.tar', 'w') as archive:
        for name in SOURCES:
            archive.add(source / name, arcname=name)
    abi = json.loads(payloads['evidence/linux/unit-movement-query-abi.json'])
    owner = json.loads(payloads['evidence/linux/unit-movement-query-profile-source.json'])
    # Read ELF load ranges only. Neither dlopen nor execution is used for assets.
    images = {}
    for kind, leaf, digest in [('core', 'libGameCore_XP2.so', abi['gamecore_sha256']),
                               ('host', 'Civ6', abi['host_sha256'])]:
        raw = (args.assets / leaf).read_bytes()
        require(sha(raw) == digest, 'wrong asset: ' + kind)
        require(raw[:6] == b'\x7fELF\x02\x01', 'ELF format')
        phoff = struct.unpack_from('<Q', raw, 32)[0]
        entsize, count = struct.unpack_from('<HH', raw, 54)
        ranges = []
        for index in range(count):
            typ, _, offset, address, _, size, _, _ = struct.unpack_from('<IIQQQQQQ', raw, phoff + index * entsize)
            if typ == 1:
                ranges.append((address, offset, size))
        images[kind] = (raw, ranges)
    gates = [('core', row['address'], row['bytes'], row['sha256']) for row in abi['methods'].values()]
    gates += [(row['image'], row['address'], row['bytes'], row['sha256']) for row in owner['method_gates']]
    for kind, address, size, digest in gates:
        raw, ranges = images[kind]
        address = int(address, 16)
        matches = [raw[offset + address - base:offset + address - base + size]
                   for base, offset, length in ranges if base <= address and address + size <= base + length]
        require(len(matches) == 1 and sha(matches[0]) == digest, 'target method identity changed')
    flags = ['gcc', '-std=gnu11', '-Wall', '-Wextra', '-Werror', '-Wconversion', '-Wshadow']
    harness = source / SOURCES[0]
    subprocess.run(flags + ['-fanalyzer', '-c', str(harness), '-o', str(directory / 'review.o')], check=True)
    executable = directory / 'review'
    subprocess.run(flags + ['-O2', '-fsanitize=undefined', str(harness), '-ldl', '-o', str(executable)], check=True)
    cases = []
    for mode in ['complete', 'count-mismatch', 'output-exists', 'guard-between-targets', 'wrong-full-id']:
        work = directory / mode
        work.mkdir()
        result = subprocess.run([str(executable), mode], cwd=work, capture_output=True, text=True)
        (work / 'stdout').write_text(result.stdout)
        (work / 'stderr').write_text(result.stderr)
        events = (work / 'events').read_text().splitlines()
        if mode in ['guard-between-targets', 'wrong-full-id']:
            require(result.returncode == 240, mode + ': wrong fatal status')
            require(events == (['max 0 abcd0000', 'fatal_guard 1'] if mode == 'guard-between-targets' else []),
                    mode + ': query occurred after fatal guard or bad full ID')
            require(('retained_owner_unknown' if mode == 'guard-between-targets' else 'unit_identity') in result.stderr,
                    mode + ': wrong refusal reason')
            cases.append(dict(mode=mode, fatal_status=240, target_calls_before_refusal=len(events) - (mode == 'guard-between-targets')))
            continue
        require(result.returncode == 0, mode + ': ' + result.stderr)
        if mode == 'output-exists':
            require(not events and 'status=242 targets=0' in result.stdout, 'existing output was overwritten or target called')
            cases.append(dict(mode=mode, returned_status=242, target_calls=0))
            continue
        units = 1261 if mode == 'count-mismatch' else 1262
        expected = [f'{target} {i} {0xabcd0000 + i:08x}' for i in range(units) for target in ['max', 'moved']]
        require(events == expected, mode + ': target input/order/cardinality')
        rows = [json.loads(line) for line in (work / 'inputs').read_text().splitlines()]
        answers = [json.loads(line) for line in (work / 'answers').read_text().splitlines()]
        status = 243 if mode == 'count-mismatch' else 0
        require(rows[-1]['status'] == status and answers[-1]['status'] == status, 'count mismatch status propagation')
        require(rows[-1]['unplaced_queried'] == rows[-1]['delayed_death_queried'] == 21, 'unplaced/dead record filtered')
        require(rows[-1]['units'] == answers[-1]['answers'] == units, 'coverage')
        for passed in [0, 2]:
            unit_rows = [row for row in rows if row['kind'] == 'unit' and row['pass'] == passed]
            require(len(unit_rows) == units, 'input pass coverage')
            require([row['id'] for row in unit_rows] == list(range(0xabcd0000, 0xabcd0000 + units)), 'full ID projection')
            if mode == 'complete':
                require(unit_rows[-1]['owner'] == 64 and unit_rows[-1]['ordinal'] == 0 and
                        unit_rows[-1]['definition'] is None, 'last slot/nullable definition omitted')
            require(sum(row['plot'] is None for row in unit_rows) == units // 2, 'nullable plot omitted')
            player_rows = [row for row in rows if row['kind'] == 'player' and row['pass'] == passed]
            require(len(player_rows) == 65 and player_rows[64]['mapped_index'] == 1 and
                    player_rows[64]['owner'] == 64, '65-slot mapping projection')
        values = answers[1:-1]
        require(all(row['id'] == 0xabcd0000 + i and row['max_moves'] == i - 7 and row['has_moved'] == bool(i % 2)
                    for i, row in enumerate(values)), 'signed result/bool ABI or answer ordering')
        globals_ = [row for row in rows if row['kind'] == 'global']
        require([row['pass'] for row in globals_] == [0, 1, 2] and
                globals_[0]['known_rng'] != globals_[2]['known_rng'], 'known RNG changes lost or gated')
        require('effects_retained=1' in result.stdout, 'effects not retained')
        cases.append(dict(mode=mode, returned_status=status, units=units, direct_target_calls=len(events),
                          unplaced_delayed_records_queried=21, known_rng_changes_preserved=True))
    # Keep the existing lifecycle test's generated harness and executable too.
    # Run its identical retained preimage from this git checkout: its legacy
    # projection deliberately asks git for a named baseline source commit.
    lifecycle_directory = directory / 'lifecycle'
    lifecycle_directory.mkdir()
    launcher = directory / 'lifecycle-launcher.py'
    launcher.write_text('import contextlib, runpy\nfrom unittest.mock import patch\n'
                        + 'with patch("tempfile.TemporaryDirectory", return_value=contextlib.nullcontext('
                        + repr(str(lifecycle_directory)) + ')):\n    runpy.run_path('
                        + repr(str(ROOT / 'tools/check/unit_movement_lifecycle_test.py')) + ', run_name="__main__")\n')
    lifecycle = subprocess.run(['python3', str(launcher)],
                               cwd=ROOT, capture_output=True, text=True)
    (directory / 'lifecycle.stdout').write_text(lifecycle.stdout)
    (directory / 'lifecycle.stderr').write_text(lifecycle.stderr)
    require(lifecycle.returncode == 0, lifecycle.stdout + lifecycle.stderr)
    dependencies = {}
    ldd = subprocess.check_output(['ldd', str(executable)], text=True)
    (directory / 'review.ldd').write_text(ldd)
    for line in ldd.splitlines():
        require('not found' not in line, 'missing dependency')
        match = re.search(r'(?:=>\s*)?(/[^\s]+)', line)
        if match:
            path = Path(match[1]).resolve()
            require('GameCore' not in path.name and path.name != 'Civ6', 'original linked into mock')
            dependencies[str(path)] = sha(path.read_bytes())
    require(all((source / name).read_bytes() == raw and (ROOT / name).read_bytes() == raw
                for name, raw in payloads.items()), 'source snapshot/checkout changed')
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    report = dict(schema='c6-linux-unit-movement-query-independent-review-v1', source_revision=revision,
                  production_pins=pins, sources={name: sha(raw) for name, raw in payloads.items()},
                  source_status=subprocess.check_output(['git', 'status', '--short', '--', *SOURCES], cwd=ROOT, text=True).splitlines(),
                  complete_source_preimages_retained=True, source_archive_sha256=sha((directory / 'source.tar').read_bytes()),
                  candidate_directory=str(directory), test_candidate_sha256=sha(executable.read_bytes()),
                  lifecycle_generated={str(path.relative_to(directory)): sha(path.read_bytes())
                                       for path in [launcher, lifecycle_directory / 'harness.c', lifecycle_directory / 'lifecycle-test']},
                  dynamic_dependencies=dependencies, gcc=subprocess.check_output(['gcc', '--version'], text=True),
                  compiler_flags=flags, strict_c=True, gcc_analyzer=True, ubsan=True, cases=cases,
                  pinned_movement_methods=len(abi['methods']), pinned_owner_facade_methods=len(owner['method_gates']),
                  reused_lifecycle_check=True, original_execution=False, native_query_parity=False,
                  full_state_action_turn_parity=False,
                  pending=['coherent runner/CLI/source profile and process supervisor review',
                           'sole-runner original execution and source-matched candidate comparison',
                           'complete affected-state/RNG comparison beyond known 64-byte diagnostic'])
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
    for name in SOURCES:
        (source / name).chmod(0o444)
    executable.chmod(0o555)
    (directory / 'source.tar').chmod(0o444)
    print(json.dumps(dict(cases=cases, source_archive_sha256=report['source_archive_sha256'],
                          test_candidate_sha256=report['test_candidate_sha256'], original_execution=False), sort_keys=True))


if __name__ == '__main__':
    main()
