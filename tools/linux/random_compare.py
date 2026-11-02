#!/usr/bin/env python3
"""Execute hash-gated original Linux Random and compare the actual Rust binaries.

This exercises isolated objects with synthetic field inputs, not a game state.
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

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    return hashlib.file_digest(Path(path).open('rb'), 'sha256').hexdigest()


def candidate(path, requests):
    identity = json.loads(subprocess.check_output([str(path), '--identity'], text=True))
    lines = [dict(r, expected_engine=identity) for r in requests]
    p = subprocess.run([str(path)], input=''.join(json.dumps(r) + '\n' for r in lines),
                       text=True, capture_output=True, check=True)
    return identity, [json.loads(s) for s in p.stdout.splitlines()]


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    ap.add_argument('--sysroot', type=Path, default=ROOT / 'target/linux/sysroot')
    ap.add_argument('--output', type=Path, required=True)
    ap.add_argument('--expected-configuration')
    args = ap.parse_args()
    assets = args.assets.resolve()
    gate = json.loads((ROOT / 'tools/linux/runtime-gate.json').read_text())
    for name, sha in gate.items():
        if digest(assets / name) != sha:
            raise SystemExit(f'original asset SHA-256 mismatch: {name}')
    work = ROOT / 'target/linux/random'
    work.mkdir(parents=True, exist_ok=True)
    # This runner owns its own preload and serializes rebuild/execution.
    import fcntl
    with (work / 'random.lock').open('w') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        execute(args, assets, gate, work)


def execute(args, assets, gate, work):
    source = ROOT / 'native/linux/random_oracle.c'
    probe = work / 'random_oracle.so'
    subprocess.run(['cc', '-shared', '-fPIC', '-Wall', '-Wextra', '-Werror', str(source),
                    '-ldl', '-o', str(probe)], check=True)
    subprocess.run(['cargo', 'build', '-p', 'c6-differential', '--bin', 'c6-rng-candidate',
                    '--bin', 'c6-serialization-candidate'], cwd=ROOT, check=True)
    env = {k: v for k, v in os.environ.items() if not k.startswith(('LD_', 'C6_'))}
    env['LD_LIBRARY_PATH'] = f'{args.sysroot.resolve()}/usr/lib/x86_64-linux-gnu:{assets}'
    listing = subprocess.check_output(['ldd', str(assets / 'Civ6')], env=env, text=True)
    if 'not found' in listing:
        raise SystemExit('native host dependency unavailable')
    dependencies = {}
    for line in listing.splitlines():
        m = re.search(r'(?:=>\s+)?(/\S+)\s+\(', line)
        if m:
            p = Path(m[1]).resolve()
            dependencies[p.name] = digest(p)
    config = {'schema': 'c6-linux-random-isolated-v1', 'original_assets': gate,
              'runtime_dependencies': dependencies, 'adapter_source_sha256': digest(source),
              'adapter_sha256': digest(probe), 'runner_source_sha256': digest(__file__),
              'compiler': subprocess.check_output(['cc', '--version'], text=True).splitlines()[0],
              'rules_database': None, 'enabled_dlc_mods': None, 'initialized_game': False,
              'logging': 'genuine host LoggingManager, disabled file output, explicit global binding',
              'fields': 'synthetic boundary fixtures after original Random constructor',
              'scope': 'isolated Random arithmetic/state and normal complete memory stream records'}
    config_sha = hashlib.sha256(json.dumps(config, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    if args.expected_configuration and args.expected_configuration != config_sha:
        raise SystemExit('native configuration identity mismatch')
    fixture_rng = random.Random(0xC6002026)  # fixture selection only, never simulated RNG
    operations = ['rng_get', 'rng_reset', 'rng_initialize', 'rng_lock', 'rng_unlock']
    cases = []
    bounds = [0, 1, 2, 32767, 32768, 65534, 65535, 65536, 0xffffffff]
    edge = [0, 1, 0x7fffffff, 0x80000000, 0xfffffffe, 0xffffffff]
    for i in range(2048):
        state = dict(zip(['seed', 'calls', 'resets', 'lock_count'],
                         [edge[(i // (6 ** n)) % 6] if i < 1296 else fixture_rng.getrandbits(32)
                          for n in range(4)]))
        state['synchronous'] = bool(i % 2)
        op = operations[i % 5]
        arg = bounds[(i // 5) % len(bounds)] if op == 'rng_get' else fixture_rng.getrandbits(32)
        step = {'op': op}
        if op == 'rng_get':
            step['bound'] = arg
        elif op in ['rng_reset', 'rng_initialize']:
            step['seed'] = arg
        cases.append({'state': state, 'step': step, 'arg': arg, 'stream': i % 4})
    requests = [{'protocol_version': 1, 'case': {'id': str(i), 'initial_state': {'rng': c['state']},
                 'steps': [c['step']]}} for i, c in enumerate(cases)]
    rng_identity, answers = candidate(ROOT / 'target/debug/c6-rng-candidate', requests)
    native_lines, expected = [], []
    for c, answer in zip(cases, answers):
        if answer['status'] != 'ok':
            raise SystemExit('Rust RNG candidate refused supported fixture')
        s = c['state']
        native_lines.append(' '.join(map(str, [operations.index(c['step']['op']), c['stream'], c['arg'],
                            s['seed'], s['calls'], s['resets'], s['lock_count'], int(s['synchronous'])])))
        observation = answer['steps'][0]
        state = observation['snapshot']['rng']
        result = observation['rng_trace'][0]['result'] if observation['rng_trace'] else 0
        expected.append([result, state['seed'], state['calls'], state['resets'], state['lock_count'],
                         int(state['synchronous']), 0, ''])
    # Four real objects retain their state through interleaved operation sequences.
    sequence_requests = []
    for stream in range(4):
        state = {'seed': stream + 1, 'calls': 0xfffffffe, 'resets': 0xffffffff,
                 'lock_count': 0xffffffff, 'synchronous': bool(stream % 2)}
        steps = [{'op': 'rng_get', 'bound': 65535}]
        for i in range(127):
            op = operations[fixture_rng.randrange(5)]
            step = {'op': op}
            if op == 'rng_get':
                step['bound'] = fixture_rng.choice(bounds)
            elif op in ['rng_reset', 'rng_initialize']:
                step['seed'] = fixture_rng.getrandbits(32)
            steps.append(step)
        sequence_requests.append({'protocol_version': 1, 'case': {'id': f'sequence-{stream}',
                                  'initial_state': {'rng': state}, 'steps': steps}})
    seq_identity, seq_answers = candidate(ROOT / 'target/debug/c6-rng-candidate', sequence_requests)
    if seq_identity != rng_identity:
        raise SystemExit('candidate changed during corpus generation')
    for ordinal in range(128):
        for stream, (request, answer) in enumerate(zip(sequence_requests, seq_answers)):
            if answer['status'] != 'ok':
                raise SystemExit('Rust candidate refused sequence')
            step = request['case']['steps'][ordinal]
            state = request['case']['initial_state']['rng']
            arg = step.get('bound', step.get('seed', 0))
            op = operations.index(step['op']) + (10 if ordinal else 0)
            native_lines.append(' '.join(map(str, [op, stream, arg, state['seed'], state['calls'],
                                state['resets'], state['lock_count'], int(state['synchronous'])])))
            observation = answer['steps'][ordinal]
            out = observation['snapshot']['rng']
            result = observation['rng_trace'][0]['result'] if observation['rng_trace'] else 0
            expected.append([result, out['seed'], out['calls'], out['resets'], out['lock_count'],
                             int(out['synchronous']), 0, ''])
    serial_requests = []
    for i in range(256):
        s = {k: fixture_rng.getrandbits(32) for k in ['seed', 'calls', 'resets', 'lock_count']}
        s['synchronous_byte'] = [0, 1, 2, 127, 255][i % 5]
        serial_requests.append({'id': f'write-{i}', 'op': 'write', 'state': s})
        payload = (i.to_bytes(4, 'little') + b''.join(fixture_rng.getrandbits(32).to_bytes(4, 'little')
                   for _ in range(3)) + bytes([s['synchronous_byte'], 0]) + b'\xca\xfe')
        serial_requests.append({'id': f'read-{i}', 'op': 'read', 'state': s, 'bytes_hex': payload.hex()})
    serial_identity, serial_answers = candidate(ROOT / 'target/debug/c6-serialization-candidate', serial_requests)
    for r, a in zip(serial_requests, serial_answers):
        if a['status'] != 'ok':
            raise SystemExit('Rust serialization candidate refused complete normal record')
        s = r['state']
        read = r['op'] == 'read'
        payload = bytes.fromhex(r['bytes_hex']) if read else b''
        native_lines.append(' '.join(map(str, [6 if read else 5, 0, len(payload), s['seed'],
                            s['calls'], s['resets'], s['lock_count'], s['synchronous_byte']]))
                            + (' ' + ' '.join(f'{b:02x}' for b in payload) if read else ''))
        out = a['state'] if read else s
        expected.append([0, out['seed'], out['calls'], out['resets'], out['lock_count'],
                         out['synchronous_byte'], 18, '' if read else a['bytes_hex']])
    corpus = '\n'.join(native_lines) + '\n'
    env.update(LD_PRELOAD=str(probe), C6_RANDOM_GAMECORE=str(assets / 'libGameCore_XP2.so'))
    started = time.monotonic()
    p = subprocess.run([str(assets / 'Civ6')], input=corpus, env=env, cwd=work,
                       capture_output=True, text=True, timeout=90)
    duration = time.monotonic() - started
    rows = [line.split() for line in p.stdout.splitlines() if re.match(r'^\d+ ', line)]
    mismatches = []
    if p.returncode != 0 or len(rows) != len(expected):
        raise SystemExit(f'native execution unavailable: exit={p.returncode}, rows={len(rows)}/{len(expected)}, stderr={p.stderr[-1000:]}')
    for i, (row, want) in enumerate(zip(rows, expected)):
        actual = list(map(int, row[:7])) + [row[7] if len(row) == 8 else '']
        if actual != want:
            mismatches.append({'index': i, 'native_input': native_lines[i], 'actual': actual, 'expected': want})
    # Preserve complete corpus + both output identities; fresh rebuilds get fresh hashes.
    artifact = {'configuration': config, 'configuration_sha256': config_sha,
                'rust_rng_candidate': rng_identity, 'rust_serialization_candidate': serial_identity,
                'native_input_sha256': hashlib.sha256(corpus.encode()).hexdigest(),
                'native_stdout_sha256': hashlib.sha256(p.stdout.encode()).hexdigest(),
                'native_stderr_sha256': hashlib.sha256(p.stderr.encode()).hexdigest(),
                'fixture_count': len(expected), 'rng_cases': len(cases) + 512, 'interleaved_sequence_steps': 512, 'serialization_cases': len(serial_requests),
                'mismatches': mismatches, 'native_exit_code': p.returncode,
                'native_process_seconds': duration,
                'timing_scope': 'includes original host startup, adapter I/O, RNG and serialization; no turns',
                'limits': ['No initialized game or gameplay stream ownership', 'Logging callbacks and file output excluded',
                           'No native partial/failing stream or legacy-vector equivalence',
                           'Rust unsupported-operation rejection is separate from native behavior'],
                'corpus': [{'input': line, 'expected': want} for line, want in zip(native_lines, expected)]}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(artifact, indent=2) + '\n')
    print(json.dumps({k: artifact[k] for k in ['configuration_sha256', 'fixture_count', 'rng_cases',
          'serialization_cases', 'native_exit_code', 'native_process_seconds']}, indent=2))
    print(f'mismatches: {len(mismatches)}')
    if mismatches:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
