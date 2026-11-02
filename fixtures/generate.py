#!/usr/bin/env python3
"""Generate a pinned real-oracle corpus; no expected game results are fabricated."""
import argparse
import json
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[1]

def identity(command):
    return json.loads(subprocess.check_output(command, cwd=ROOT, text=True))

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--candidate', default=str(ROOT / 'target/debug/c6-rng-candidate'))
    p.add_argument('--output', type=pathlib.Path, default=ROOT / 'fixtures/corpus/xp2-rng.json')
    args = p.parse_args()
    cases = []
    seeds = [0, 1, 0x7fffffff, 0x80000000, 0xfffffffe, 0xffffffff]
    bounds = [0, 1, 2, 3, 255, 256, 32767, 32768, 65534, 65535, 65536, 65537, 0xffffffff]
    for i, seed in enumerate(seeds):
        state = {'rng': {'seed': seed, 'calls': 0xffffffff if i%2 else 0, 'resets': 0xffffffff if i%2 else 1,
                         'lock_count': 0xffffffff if i%2 else 0, 'synchronous': bool(i%2)}}
        steps = [{'op': 'rng_get', 'bound': b} for b in bounds]
        steps += [{'op': 'rng_lock'}, {'op': 'rng_unlock'}, {'op': 'rng_unlock'},
                  {'op': 'rng_reset', 'seed': seed ^ 0xffffffff}, {'op': 'rng_get', 'bound': 65535},
                  {'op': 'rng_initialize', 'seed': seed}, {'op': 'rng_get', 'bound': 0}, {'op': 'rng_get', 'bound': 65535}]
        # Deterministic corpus selection; these values are inputs, never results.
        steps += [{'op': 'rng_get', 'bound': bounds[(j * 7 + i) % len(bounds)], 'stream': 'sync'} for j in range(512)]
        cases.append({'id': f'rng-boundaries-{seed:08x}', 'initial_state': state, 'steps': steps})
    multi_state = {'rng': {'seed': 1, 'calls': 0, 'resets': 1, 'lock_count': 0, 'synchronous': False},
                   'rng_streams': {'1': {'seed': 2, 'calls': 0xffffffff, 'resets': 0, 'lock_count': 0, 'synchronous': True},
                                   '4294967295': {'seed': 0xffffffff, 'calls': 0, 'resets': 0xffffffff, 'lock_count': 0xffffffff, 'synchronous': False}}}
    steps = []
    for i in range(512):
        sid = [0, 1, 4294967295][i % 3]
        steps.append({'op': 'rng_get', 'bound': bounds[i % len(bounds)], 'stream_id': sid, 'stream': f'instance-{sid}'})
        if i % 17 == 0:
            steps += [{'op': 'rng_reset', 'seed': i, 'stream_id': sid}, {'op': 'rng_lock', 'stream_id': sid}, {'op': 'rng_unlock', 'stream_id': sid}]
        if i % 97 == 0:
            steps.append({'op': 'rng_initialize', 'seed': i ^ 0xffffffff, 'stream_id': sid})
    cases.append({'id': 'rng-independent-instances', 'initial_state': multi_state, 'steps': steps})
    extrema = [-2147483648, -2147483647, -10000, -9999, -9998, -3, -2, -1, 0, 1, 2, 3, 2147483646, 2147483647]
    valid_steps = [{'op': 'plot_valid', 'x': x, 'y': y} for x in extrema for y in extrema]
    cases.append({'id': 'plot-sentinel-boundaries', 'initial_state': {}, 'steps': valid_steps})
    # Same-row, odd/even/negative rows, wrap directions, and signed overflow.
    widths = [-2147483648, -1, 0, 1, 2, 10, 84, 2147483647]
    for wrap in [False, True]:
        steps = []
        for i, x1 in enumerate(extrema):
            for j, y1 in enumerate(extrema):
                for k, width in enumerate(widths):
                    steps.append({'op': 'plot_distance', 'width': width, 'wrap': wrap,
                                  'x1': x1, 'y1': y1, 'x2': extrema[(j+k)%len(extrema)],
                                  'y2': extrema[(i+3*k)%len(extrema)]})
        cases.append({'id': f'plot-distance-wrap-{str(wrap).lower()}', 'initial_state': {}, 'steps': steps})
    corpus = {'schema_version': 1,
              'oracle': identity(['python3', 'tools/oracle/run.py', '--identity']),
              'candidate': identity([args.candidate, '--identity']),
              'evidence': {'level': 'isolated_original_dll',
                           'references': ['tools/oracle/config.json', 'evidence/native/xp2-random.asm', 'evidence/engine/map-distance.asm', 'evidence/engine/plot-valid.asm'],
                           'coverage': ['rng_get', 'rng_reset', 'rng_initialize', 'rng_lock', 'rng_unlock', 'plot_valid', 'plot_distance']},
              'cases': cases}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(corpus, indent=2) + '\n')
    print(f'{args.output}: {len(cases)} cases, {sum(len(c["steps"]) for c in cases)} steps')

if __name__ == '__main__':
    main()
