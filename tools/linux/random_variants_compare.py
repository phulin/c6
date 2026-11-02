#!/usr/bin/env python3
"""Compare original bounded Linux RNG components to a freshly built Rust adapter.

Acquire the shared native execution lease before invoking this runner.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import re
import subprocess
import time
from random_variants_audit import ROOT, audit, digest

SOURCES = ['native/linux/random_variants.c', 'native/linux/random_variants.h',
           'crates/c6-core/src/random_variants.rs', 'crates/c6-core/examples/random_variants.rs',
           'tools/linux/random_variants_compare.py', 'tools/linux/random_variants_audit.py']


def fixtures():
    edges32 = [0, 1, 0x7fffffff, 0x80000000, 0xfffffffe, 0xffffffff]
    edges64 = edges32 + [0x7fffffffffffffff, 0x8000000000000000,
                         0xfffffffffffffffe, 0xffffffffffffffff]
    bounds = [0, 1, 2, 32767, 32768, 65534, 65535, 65536, 0xffffffff]
    rows = []
    for op in range(7):
        for state in (edges32 if op <= 2 else edges64):
            for arg in (bounds if op == 2 else edges32):
                for inc in (edges64 if op == 6 else [0]):
                    rows.append([op, len(rows) % 4, arg, state, inc, 0])
    # All 32 rotations plus full-width synthetic increments; zero/even increments
    # remain raw input rather than being normalized to a standard PCG stream.
    chooser = random.Random(0xc6322026)
    for i in range(2048):
        rows.append([6, i % 4, 0, (i % 32 << 59) | chooser.getrandbits(59),
                     chooser.getrandbits(64), 0])
    # Eight real objects retain independent state across constructor/seed/draws.
    for stream in range(4):
        rows.append([0, stream, edges32[stream], 0, 0, 1])
        rows.append([4, stream, edges32[-stream-1], 0, 0, 1])
    for i in range(1024):
        op = chooser.choice([1, 2, 2, 2, 5, 6, 6, 6])
        rows.append([op, chooser.randrange(4), chooser.choice(bounds if op == 2 else edges32), 0, 0, 1])
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent/'c6-assets-linux')
    parser.add_argument('--sysroot', type=Path, default=ROOT/'target/linux/sysroot')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--expected-configuration')
    args = parser.parse_args()
    assets = args.assets.resolve()
    contract = audit(assets)
    gate = json.loads((ROOT/'tools/linux/runtime-gate.json').read_text())
    for name, sha in gate.items():
        if digest(assets/name) != sha:
            raise ValueError('original runtime asset hash mismatch: ' + name)
    work = ROOT/'target/linux/random-variants'
    work.mkdir(parents=True, exist_ok=True)
    probe = work/'oracle.so'
    subprocess.run(['cc', '-shared', '-fPIC', '-Wall', '-Wextra', '-Werror',
                    str(ROOT/SOURCES[0]), '-ldl', '-o', str(probe)], check=True)
    subprocess.run(['cargo', 'build', '-p', 'c6-core', '--example', 'random_variants'], cwd=ROOT, check=True)
    candidate = ROOT/'target/debug/examples/random_variants'
    subprocess.run(['cargo', 'build', '--release', '-p', 'c6-core', '--example', 'random_variants'], cwd=ROOT, check=True)
    release_candidate = ROOT/'target/release/examples/random_variants'
    env = {k:v for k,v in os.environ.items() if not k.startswith(('LD_', 'C6_'))}
    env['LD_LIBRARY_PATH'] = f'{args.sysroot.resolve()}/usr/lib/x86_64-linux-gnu:{assets}'
    listing = subprocess.check_output(['ldd', str(assets/'Civ6')], env=env, text=True)
    if 'not found' in listing:
        raise ValueError('host dependency unavailable')
    dependencies = {}
    for line in listing.splitlines():
        match = re.search(r'(?:=>\s+)?(/\S+)\s+\(', line)
        if match:
            path = Path(match[1]).resolve()
            dependencies[str(path)] = digest(path)
    config = dict(schema='c6-linux-random-variants-isolated-v1', original_assets=gate,
                  runtime_dependencies=dependencies, contract=contract,
                  source_sha256={name:digest(ROOT/name) for name in SOURCES},
                  native_adapter_sha256=digest(probe), candidate_sha256=digest(candidate),
                  release_candidate_sha256=digest(release_candidate),
                  compiler=subprocess.check_output(['cc','--version'], text=True).splitlines()[0],
                  initialized_game=False, rules_database=None, enabled_dlc_mods=None,
                  fields='Original constructors then synthetic raw-state boundaries; retained interleaved objects',
                  logging_service='None required by these original methods')
    identity = hashlib.sha256(json.dumps(config,sort_keys=True,separators=(',',':')).encode()).hexdigest()
    if args.expected_configuration and identity != args.expected_configuration:
        raise ValueError('native configuration identity mismatch')
    rows = fixtures()
    corpus = ''.join(' '.join(map(str,row))+'\n' for row in rows)
    expected = subprocess.check_output([str(candidate)], input=corpus, text=True).splitlines()
    release_expected = subprocess.check_output([str(release_candidate)], input=corpus, text=True).splitlines()
    if release_expected != expected:
        raise ValueError('debug and release Rust observations differ')
    if len(expected) != len(rows):
        raise ValueError('Rust candidate fixture count differs')
    env.update(LD_PRELOAD=str(probe), C6_RANDOM_GAMECORE=str(assets/'libGameCore_XP2.so'))
    started = time.monotonic()
    native = subprocess.run([str(assets/'Civ6')], input=corpus, text=True, capture_output=True,
                            cwd=work, env=env, timeout=45)
    elapsed = time.monotonic()-started
    # Original host startup may print messages before application main. Preserve
    # those bytes, and accept only exact machine observation records.
    stdout_path = args.output.with_suffix('.stdout.txt')
    stderr_path = args.output.with_suffix('.stderr.txt')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    stdout_path.write_text(native.stdout)
    stderr_path.write_text(native.stderr)
    actual = [line for line in native.stdout.splitlines() if line.startswith('C6_RNG_VARIANT')]
    startup_output = [line for line in native.stdout.splitlines() if not line.startswith('C6_RNG_VARIANT')]
    if startup_output != ['Shutdown handler: initalize.']:
        raise ValueError('unclassified original host startup output')
    for ordinal, (actual_row, expected_row) in enumerate(zip(actual, expected)):
        for row in [actual_row, expected_row]:
            if not re.fullmatch(r'C6_RNG_VARIANT [0-9]+ [0-9]+ [0-9]+ [0-9]+', row) or int(row.split()[1]) != ordinal:
                raise ValueError('malformed, duplicate or out-of-order observation')
    if native.returncode or len(actual) != len(rows):
        raise ValueError(f'native oracle unavailable: exit={native.returncode} rows={len(actual)}/{len(rows)} stderr={native.stderr[-1000:]}')
    mismatches = [dict(index=i,input=rows[i],native=a,rust=e)
                  for i,(a,e) in enumerate(zip(actual,expected)) if a != e]
    result = dict(configuration=config, configuration_sha256=identity, fixture_count=len(rows),
                  retained_interleaved_steps=1032, native_exit_code=native.returncode,
                  candidate_builds=['debug', 'release'], candidate_build_outputs_identical=True,
                  startup_stdout_lines=startup_output,
                  native_process_seconds=elapsed, mismatches=mismatches,
                  input_sha256=hashlib.sha256(corpus.encode()).hexdigest(),
                  native_stdout_sha256=hashlib.sha256(native.stdout.encode()).hexdigest(),
                  native_stderr_sha256=hashlib.sha256(native.stderr.encode()).hexdigest(),
                  corpus=[dict(input=row,expected=e) for row,e in zip(rows,expected)],
                  limits=['No initialized game, stream ownership/lifecycle, save semantics, actions or turns',
                          'No fRand, bounded Lua wrapper or real-time seeding; no RNG exclusions inferred',
                          'Process time includes startup and I/O; not a throughput or turn benchmark'])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(dict(configuration_sha256=identity, fixture_count=len(rows), mismatches=len(mismatches), native_process_seconds=elapsed)))
    if mismatches:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
