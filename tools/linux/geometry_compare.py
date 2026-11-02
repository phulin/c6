#!/usr/bin/env python3
"""Compare original Linux coordinate arithmetic to the Rust process adapter."""
import argparse
import fcntl
import hashlib
import itertools
import json
import os
from pathlib import Path
import random
import re
import subprocess

from random_compare import candidate, digest

ROOT = Path(__file__).resolve().parents[2]


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    ap.add_argument('--sysroot', type=Path, default=ROOT / 'target/linux/sysroot')
    ap.add_argument('--expected-configuration')
    ap.add_argument('--output', type=Path, required=True)
    args = ap.parse_args()
    assets = args.assets.resolve()
    gate = json.loads((ROOT / 'tools/linux/runtime-gate.json').read_text())
    for name, sha in gate.items():
        if digest(assets / name) != sha:
            raise SystemExit(f'original asset SHA-256 mismatch: {name}')
    work = ROOT / 'target/linux/geometry'
    work.mkdir(parents=True, exist_ok=True)
    with (work / 'geometry.lock').open('w') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        execute(args, assets, gate, work)


def execute(args, assets, gate, work):
    source = ROOT / 'native/linux/geometry_oracle.c'
    probe = work / 'geometry_oracle.so'
    subprocess.run(['cc', '-shared', '-fPIC', '-Wall', '-Wextra', '-Werror',
                    str(source), '-ldl', '-o', str(probe)], check=True)
    subprocess.run(['cargo', 'build', '-p', 'c6-differential', '--bin', 'c6-rng-candidate'],
                   cwd=ROOT, check=True)
    env = {k: v for k, v in os.environ.items() if not k.startswith(('LD_', 'C6_'))}
    env['LD_LIBRARY_PATH'] = f'{args.sysroot.resolve()}/usr/lib/x86_64-linux-gnu:{assets}'
    listing = subprocess.check_output(['ldd', str(assets / 'Civ6')], env=env, text=True)
    if 'not found' in listing:
        raise SystemExit('native host dependency unavailable')
    dependencies = {}
    for line in listing.splitlines():
        match = re.search(r'(?:=>\s+)?(/\S+)\s+\(', line)
        if match:
            p = Path(match[1]).resolve()
            dependencies[p.name] = digest(p)
    config = {'schema': 'c6-linux-stateless-geometry-v1', 'original_assets': gate,
              'runtime_dependencies': dependencies, 'adapter_source_sha256': digest(source),
              'adapter_sha256': digest(probe), 'runner_source_sha256': digest(__file__),
              'runner_helper_sha256': digest(ROOT / 'tools/linux/random_compare.py'),
              'compiler': subprocess.check_output(['cc', '--version'], text=True).splitlines()[0],
              'rules_database': None, 'enabled_dlc_mods': None, 'initialized_game': False,
              'map_object': None, 'height': 0, 'wrap_y': False,
              'scope': 'original PlotCoord constructor/IsValid and stateless GetPlotDistance overloads'}
    config_sha = hashlib.sha256(json.dumps(config, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    if args.expected_configuration and args.expected_configuration != config_sha:
        raise SystemExit('native configuration identity mismatch')
    edge = [-2147483648, -2147483647, -10000, -9999, -9998, -3, -2, -1,
            0, 1, 2, 3, 9998, 9999, 10000, 2147483646, 2147483647]
    fixtures = [{'op': 'plot_valid', 'x': x, 'y': y} for x, y in itertools.product(edge, repeat=2)]
    # Negative/equal rows, both parities, equal shortest wrapped alternatives.
    for width, wrap, x1, y1, x2, y2 in itertools.product(
            [-2147483648, -10, -1, 0, 1, 2, 10, 2147483647], [False, True],
            [-2, 0, 5], [-3, -2, 0, 1, 2], [-5, 0, 2], [-3, -2, 0, 1, 2]):
        fixtures.append(dict(op='plot_distance', width=width, wrap=wrap, x1=x1, y1=y1, x2=x2, y2=y2))
    selection = random.Random(0xc6006e0)  # fixture selection only
    for i in range(4096):
        values = [selection.choice(edge) if i < 2048 else selection.randint(-2147483648, 2147483647)
                  for _ in range(5)]
        fixtures.append(dict(zip(['width', 'x1', 'y1', 'x2', 'y2'], values),
                             op='plot_distance', wrap=bool(i % 2)))
    requests = [{'protocol_version': 1, 'case': {'id': str(i), 'initial_state': {}, 'steps': [step]}}
                for i, step in enumerate(fixtures)]
    rust_identity, answers = candidate(ROOT / 'target/debug/c6-rng-candidate', requests)
    expected = []
    native_lines = []
    for step, answer in zip(fixtures, answers):
        if answer['status'] != 'ok':
            raise SystemExit('Rust candidate refused supported geometry fixture')
        expected.append(int(answer['steps'][0]['snapshot']['result']))
        if step['op'] == 'plot_valid':
            native_lines.append(f"0 0 0 {step['x']} {step['y']} 0 0")
        else:
            native_lines.append(' '.join(map(str, [1, step['width'], int(step['wrap']),
                                step['x1'], step['y1'], step['x2'], step['y2']])))
    corpus = '\n'.join(native_lines) + '\n'
    env.update(LD_PRELOAD=str(probe), C6_GEOMETRY_GAMECORE=str(assets / 'libGameCore_XP2.so'))
    result = subprocess.run([str(assets / 'Civ6')], input=corpus, cwd=work, env=env,
                            text=True, capture_output=True, timeout=90)
    actual = [int(s) for s in result.stdout.splitlines() if re.fullmatch(r'-?\d+', s)]
    if result.returncode or len(actual) != len(expected):
        raise SystemExit(f'native oracle unavailable: exit={result.returncode}, observations={len(actual)}/{len(expected)}, stderr={result.stderr[-1000:]}')
    mismatches = [{'input': native_lines[i], 'fixture': fixtures[i], 'native': a, 'rust': b}
                  for i, (a, b) in enumerate(zip(actual, expected)) if a != b]
    # Keep reproducible synthetic observations compact, separate from profile summary.
    corpus_path = args.output.with_suffix('.jsonl')
    corpus_text = ''.join(json.dumps({'input': line, 'native': a, 'rust': b}, separators=(',', ':')) + '\n'
                          for line, a, b in zip(native_lines, actual, expected))
    artifact = {'configuration': config, 'configuration_sha256': config_sha,
                'rust_candidate': rust_identity, 'native_exit_code': result.returncode,
                'fixture_count': len(fixtures), 'validity_cases': len(edge) ** 2,
                'distance_cases': len(fixtures) - len(edge) ** 2, 'mismatches': mismatches,
                'corpus_file': corpus_path.name,
                'corpus_sha256': hashlib.sha256(corpus_text.encode()).hexdigest(),
                'native_input_sha256': hashlib.sha256(corpus.encode()).hexdigest(),
                'native_stdout_sha256': hashlib.sha256(result.stdout.encode()).hexdigest(),
                'native_stderr_sha256': hashlib.sha256(result.stderr.encode()).hexdigest(),
                'limits': ['No map instance initialization, map bounds, movement, directions or pathfinding',
                           'Explicit dimension inputs exercise arithmetic, including invalid map dimensions',
                           'Both native distance overloads checked; both original coordinate objects unchanged']}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    corpus_path.write_text(corpus_text)
    args.output.write_text(json.dumps(artifact, indent=2) + '\n')
    print(json.dumps({k: artifact[k] for k in ['configuration_sha256', 'fixture_count', 'validity_cases',
                     'distance_cases', 'native_exit_code']}, indent=2))
    print(f'mismatches: {len(mismatches)}')
    if mismatches:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
