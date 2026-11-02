#!/usr/bin/env python3
"""Pinned original DLL vs Rust normal Random wire-record differential corpus.

This dedicated adapter contract is separate from action protocol v1 and C6SN.
"""
import argparse
import json
import os
import selectors
import signal
import time
import pathlib
import re
import struct
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
NATIVE = ['python3', 'tools/oracle/serialization.py']


def require(condition, message):
    if not condition:
        raise ValueError(message)


def bounded_process(command, data='', timeout=30, output_limit=16 * 1024 * 1024):
    """Bound both output streams while servicing stdin without a pipe deadlock."""
    require(os.name == 'posix', 'serialization process guard currently requires POSIX')
    process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, cwd=ROOT, start_new_session=True, bufsize=0)
    selector = selectors.DefaultSelector()
    payload = memoryview(data.encode())
    cursor = 0
    outputs = {'stdout': bytearray(), 'stderr': bytearray()}
    deadline = time.monotonic() + timeout
    try:
        for pipe, name, event in [(process.stdin, 'stdin', selectors.EVENT_WRITE),
                                  (process.stdout, 'stdout', selectors.EVENT_READ),
                                  (process.stderr, 'stderr', selectors.EVENT_READ)]:
            os.set_blocking(pipe.fileno(), False)
            if name == 'stdin' and not payload:
                pipe.close()
            else:
                selector.register(pipe, event, name)
        while selector.get_map():
            remaining = deadline - time.monotonic()
            require(remaining > 0, 'process deadline exceeded')
            for key, _ in selector.select(min(remaining, 0.05)):
                pipe, name = key.fileobj, key.data
                try:
                    if name == 'stdin':
                        cursor += os.write(pipe.fileno(), payload[cursor:cursor+65536])
                        if cursor == len(payload):
                            selector.unregister(pipe)
                            pipe.close()
                    else:
                        chunk = os.read(pipe.fileno(), 65536)
                        if not chunk:
                            selector.unregister(pipe)
                            pipe.close()
                        else:
                            require(len(outputs[name]) + len(chunk) <= output_limit, f'{name} output limit exceeded')
                            outputs[name].extend(chunk)
                except BlockingIOError:
                    continue
                except BrokenPipeError:
                    selector.unregister(pipe)
                    pipe.close()
        remaining = deadline - time.monotonic()
        require(remaining > 0, 'process deadline exceeded')
        try:
            code = process.wait(timeout=remaining)
        except subprocess.TimeoutExpired as error:
            raise ValueError('process deadline exceeded') from error
        require(cursor == len(payload), 'process closed stdin before complete request')
        if code:
            raise subprocess.CalledProcessError(code, command, bytes(outputs['stdout']), bytes(outputs['stderr']))
        return outputs['stdout'].decode(), outputs['stderr'].decode()
    finally:
        # Also kill descendants that could retain output pipes after parent exit.
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait()
        selector.close()
        for pipe in [process.stdin, process.stdout, process.stderr]:
            if not pipe.closed:
                pipe.close()


def identity(command):
    stdout, _ = bounded_process(command + ['--identity'], output_limit=4096)
    value = json.loads(stdout)
    require(set(value) == {'kind', 'binary_sha256', 'configuration_sha256'}, 'invalid engine identity fields')
    require(all(re.fullmatch('[0-9a-f]{64}', value[k]) for k in ['binary_sha256', 'configuration_sha256']), 'invalid engine hash')
    return value


def state(i):
    return {'seed': [0, 1, 0x80000000, 0xffffffff][i % 4],
            'calls': (0xffffffff - i) & 0xffffffff, 'resets': i * 0x01010101,
            'lock_count': (i * 0x10203041) & 0xffffffff, 'synchronous_byte': i}


def generate(args):
    cases = []
    versions = [0, 1, 2, 0xffffffff]
    for i in range(256):
        s = state(i)
        cases.append({'id': f'write-raw-sync-{i:02x}', 'op': 'write', 'state': s, 'expected_status': 'ok'})
        # These bytes are inputs to both engines, never expected reference output.
        data = struct.pack('<IIIIBB', versions[i % 4], s['seed'], s['calls'], s['resets'], i, 0)
        cases.append({'id': f'read-raw-sync-{i:02x}', 'op': 'read', 'state': state(255-i),
                      'bytes_hex': (data + bytes(range(i % 17))).hex(), 'expected_status': 'ok'})
    for version in versions:
        for trailer_length in [0, 1, 18, 255]:
            data = struct.pack('<IIIIBB', version, 0x12345678, 0xffffffff, 0, version & 1, 0)
            cases.append({'id': f'prefix-v{version:08x}-tail{trailer_length}', 'op': 'read',
                          'state': state(255), 'bytes_hex': (data + bytes(range(trailer_length))).hex(), 'expected_status': 'ok'})
    normal = struct.pack('<IIIIBB', 1, 0, 0, 1, 1, 0)
    for length in range(18):
        cases.append({'id': f'unsupported-short-{length}', 'op': 'read', 'state': state(1),
                      'bytes_hex': normal[:length].hex(), 'expected_status': 'unsupported'})
    for flag in range(1, 256):
        data = normal[:17] + bytes([flag]) + bytes(range(16))
        cases.append({'id': f'unsupported-legacy-{flag:02x}', 'op': 'read', 'state': state(1),
                      'bytes_hex': data.hex(), 'expected_status': 'unsupported'})
    corpus = {'schema_version': 1, 'scope': 'original_random_normal18_wire_record',
              'oracle': identity(NATIVE), 'candidate': identity([args.candidate]),
              'evidence': {'level': 'isolated_original_dll',
                           'references': ['tools/oracle/serialization-config.json', 'evidence/engine/rng-serialization.json'],
                           'coverage': ['random_write_normal18', 'random_read_prefix', 'raw_synchronous_byte',
                                        'ignored_version', 'preserved_lock_count', 'unsupported_input_preflight']},
              'cases': cases}
    args.corpus.parent.mkdir(parents=True, exist_ok=True)
    args.corpus.write_text(json.dumps(corpus, indent=2) + '\n')
    print(f'{args.corpus}: {len(cases)} cases')


def difference(a, b, path=''):
    if type(a) is not type(b):
        return path, a, b
    if isinstance(a, dict):
        for key in sorted(a.keys() | b.keys()):
            p = path + '/' + key.replace('~', '~0').replace('/', '~1')
            if key not in a or key not in b:
                return p, a.get(key, '<missing>'), b.get(key, '<missing>')
            d = difference(a[key], b[key], p)
            if d:
                return d
    elif isinstance(a, list) and len(a) == len(b):
        for i, (left, right) in enumerate(zip(a, b)):
            d = difference(left, right, path + '/' + str(i))
            if d:
                return d
    elif a != b:
        return path, a, b
    return None


def execute(command, expected_engine, cases, trace_dir, side):
    requests = []
    for case in cases:
        request = {key: value for key, value in case.items() if key != 'expected_status'}
        request['expected_engine'] = expected_engine
        requests.append(request)
    data = ''.join(json.dumps(r) + '\n' for r in requests)
    stdout, _ = bounded_process(command, data)
    responses = [json.loads(line) for line in stdout.splitlines()]
    require(len(responses) == len(cases), f'{side}: response count mismatch')
    if trace_dir:
        trace_dir.mkdir(parents=True, exist_ok=True)
        (trace_dir / f'{side}.jsonl').write_text(''.join(json.dumps({'request': req, 'response': resp})+'\n' for req, resp in zip(requests, responses)))
    for case, response in zip(cases, responses):
        require(response.get('engine') == expected_engine, f'{side}/{case["id"]}: engine identity mismatch')
        require(response.get('id') == case['id'], f'{side}: response ID mismatch')
        require(response.get('status') == case['expected_status'], f'{side}/{case["id"]}: {response}')
        if case['expected_status'] != 'ok':
            require(not {'state', 'bytes_hex', 'consumed', 'trailing_hex'} & response.keys(), f'{side}: partial unsupported observation')
    return responses


def replay(args):
    corpus = json.loads(args.corpus.read_text())
    require(corpus['schema_version'] == 1 and corpus['scope'] == 'original_random_normal18_wire_record', 'unsupported corpus')
    require(corpus['oracle']['kind'] == 'original_dll_serialization', 'wrong oracle kind')
    require(corpus['candidate']['kind'] == 'rust_serialization', 'wrong candidate kind')
    require(corpus['evidence']['level'] == 'isolated_original_dll' and corpus['evidence']['coverage'] and corpus['evidence']['references'], 'missing evidence')
    for case in corpus['cases']:
        require(case['op'] in ['write', 'read'] and case['expected_status'] in ['ok', 'unsupported'], 'unsupported case contract')
        require(set(case) <= {'id', 'op', 'state', 'bytes_hex', 'expected_status'}, 'unknown case fields')
    require(corpus['cases'] and len({c['id'] for c in corpus['cases']}) == len(corpus['cases']), 'missing or duplicate cases')
    require(identity(NATIVE) == corpus['oracle'], 'oracle identity mismatch; regenerate explicitly if intended')
    require(identity([args.candidate]) == corpus['candidate'], 'candidate identity mismatch; regenerate explicitly if intended')
    left = execute(NATIVE, corpus['oracle'], corpus['cases'], args.trace_dir, 'oracle')
    right = execute([args.candidate], corpus['candidate'], corpus['cases'], args.trace_dir, 'candidate')
    for case, a, b in zip(corpus['cases'], left, right):
        # Human refusal wording is not a semantic output. Successful payloads
        # are otherwise compared in full, including unknown/extra fields.
        a = {k: v for k, v in a.items() if k not in ['engine', 'message']}
        b = {k: v for k, v in b.items() if k not in ['engine', 'message']}
        d = difference(a, b)
        require(d is None, f'first divergence case={case["id"]} path={d[0]} oracle={d[1]!r} candidate={d[2]!r}' if d else '')
    supported = sum(c['expected_status'] == 'ok' for c in corpus['cases'])
    print(json.dumps({'cases': len(left), 'supported': supported, 'unsupported_preflight': len(left)-supported,
                      'evidence_level': corpus['evidence']['level'], 'scope': corpus['scope']}, indent=2))


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('mode', choices=['generate', 'replay'])
    p.add_argument('--corpus', type=pathlib.Path, default=ROOT/'fixtures/corpus/xp2-serialization.json')
    p.add_argument('--candidate', default=str(ROOT/'target/debug/c6-serialization-candidate'))
    p.add_argument('--trace-dir', type=pathlib.Path)
    args = p.parse_args()
    try:
        (generate if args.mode == 'generate' else replay)(args)
    except (AssertionError, KeyError, ValueError, OSError, subprocess.SubprocessError) as error:
        print(f'serialization differential failed: {error}', file=sys.stderr)
        return 1
    return 0

if __name__ == '__main__':
    raise SystemExit(main())
