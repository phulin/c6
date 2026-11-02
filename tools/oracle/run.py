#!/usr/bin/env python3
"""Execute verified original XP2 RNG methods; line-oriented differential protocol v1."""
import argparse
import ctypes as C
import hashlib
import json
import pathlib
import platform
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
CONFIG_PATH = pathlib.Path(__file__).with_name('config.json')
CONFIG = json.loads(CONFIG_PATH.read_text())
CONFIG_SHA = hashlib.sha256(CONFIG_PATH.read_bytes()).hexdigest()
FIELDS = ('seed', 'calls', 'resets', 'lock_count', 'synchronous')
COVERAGE = ['rng_get', 'rng_reset', 'rng_initialize', 'rng_lock', 'rng_unlock', 'plot_distance', 'plot_valid']

class Random(C.Structure):
    _fields_ = [('vptr', C.c_uint64), ('seed', C.c_uint32), ('calls', C.c_uint32),
                ('resets', C.c_uint32), ('lock_count', C.c_uint32),
                ('synchronous', C.c_uint8), ('padding', C.c_uint8 * 7)]

def snapshot(r):
    return {'rng': {k: bool(getattr(r, k)) if k == 'synchronous' else getattr(r, k) for k in FIELDS}}

def unsigned(value):
    if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= 0xffffffff:
        raise ValueError('expected unsigned 32-bit integer')
    return value

def signed(value):
    if isinstance(value, bool) or not isinstance(value, int) or not -(1 << 31) <= value < (1 << 31):
        raise ValueError('expected signed 32-bit integer')
    return value

def boolean(value):
    if not isinstance(value, bool): raise ValueError('expected boolean')
    return value

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--dll', type=pathlib.Path, default=ROOT.parent / 'c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll')
    p.add_argument('--identity', action='store_true')
    args = p.parse_args()
    engine = {'kind': 'original_dll', 'binary_sha256': CONFIG['dll_sha256'], 'configuration_sha256': CONFIG_SHA}
    if args.identity:
        print(json.dumps(engine)); return
    failure = None
    try:
        if platform.system() != 'Linux' or platform.machine() != 'x86_64':
            raise RuntimeError('native image host requires Linux x86_64')
        if hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest() != CONFIG['adapter_source_sha256']:
            raise RuntimeError('adapter source identity differs from versioned configuration')
        actual = hashlib.sha256(args.dll.read_bytes()).hexdigest()
        if actual != CONFIG['dll_sha256']:
            raise RuntimeError('DLL SHA256 mismatch: refusing version-specific RVAs')
        host = C.CDLL(str(ROOT / 'native/oracle/build/libc6_oracle.so'))
        host.c6_host_source_sha256.restype = C.c_char_p
        source_sha = hashlib.sha256((ROOT/'native/oracle/image_host.c').read_bytes()).hexdigest()
        if source_sha != CONFIG['host_source_sha256'] or host.c6_host_source_sha256().decode() != source_sha:
            raise RuntimeError('native host source/build identity mismatch; update config and rebuild')
        if C.sizeof(Random) != CONFIG['random_layout']['size'] or any(getattr(Random,k).offset != CONFIG['random_layout'][k] for k in FIELDS):
            raise RuntimeError('ctypes Random layout differs from versioned contract')
        host.c6_load.argtypes = [C.c_char_p]
        result = host.c6_load(str(args.dll).encode())
        if result:
            raise RuntimeError(f'native PE image mapping failed ({result})')
        host.c6_plot_distance.argtypes = [C.c_int32,C.c_uint8,C.c_int32,C.c_int32,C.c_int32,C.c_int32]
        host.c6_plot_distance.restype = C.c_int32
        host.c6_plot_valid.argtypes = [C.c_int32,C.c_int32]
        host.c6_plot_valid.restype = C.c_uint8
        for name in ('construct','get','reset','initialize','lock','unlock'):
            fn = getattr(host, 'c6_' + name)
            fn.argtypes = [C.POINTER(Random)] + ([C.c_uint32] if name in ('get','reset','initialize') else [])
            fn.restype = C.c_uint16 if name == 'get' else None
    except (OSError, RuntimeError) as e:
        failure = str(e)
    for line in sys.stdin:
        response = {'protocol_version': 1, 'engine': engine, 'coverage': COVERAGE, 'steps': []}
        try:
            req = json.loads(line)
            case = req['case']
            response['case_id'] = case['id']
            if req.get('protocol_version') != 1:
                raise ValueError('expected protocol_version 1')
            if req.get('expected_engine') != engine:
                raise ValueError('expected_engine must match --identity exactly')
            if failure:
                response.update(status='unavailable', message=failure)
            elif any(step.get('op') not in COVERAGE for step in case['steps']):
                response.update(status='unsupported', message='Only isolated RNG operations are supported; game actions/end-turn require a real initialized game context')
            else:
                r = Random()
                host.c6_construct(C.byref(r))
                state = case['initial_state'].get('rng', {})
                for key in FIELDS:
                    if key in state:
                        if key == 'synchronous':
                            if not isinstance(state[key], bool): raise ValueError('synchronous must be boolean')
                            setattr(r,key,state[key])
                        else:
                            setattr(r,key,unsigned(state[key]))
                for i, step in enumerate(case['steps']):
                    trace = []
                    op = step['op']
                    if op == 'plot_distance':
                        value = host.c6_plot_distance(signed(step['width']), boolean(step['wrap']),
                            *[signed(step[k]) for k in ('x1','y1','x2','y2')])
                        response['steps'].append({'step_index':i,'snapshot':{'result':value},'rng_trace':[]})
                        continue
                    elif op == 'plot_valid':
                        value = bool(host.c6_plot_valid(signed(step['x']),signed(step['y'])))
                        response['steps'].append({'step_index':i,'snapshot':{'result':value},'rng_trace':[]})
                        continue
                    elif op == 'rng_get':
                        bound = unsigned(step['bound'])
                        value = host.c6_get(C.byref(r),bound)
                        trace = [{'stream': step.get('stream','sync'), 'ordinal':r.calls, 'operation':'get',
                                  'arguments':{'bound':bound,'effective_bound':bound & 65535}, 'result':value}]
                    elif op in ('rng_reset','rng_initialize'):
                        getattr(host,'c6_'+op[4:])(C.byref(r),unsigned(step['seed']))
                    else:
                        getattr(host,'c6_'+op[4:])(C.byref(r))
                    response['steps'].append({'step_index':i,'snapshot':snapshot(r),'rng_trace':trace})
                response['status'] = 'ok'
        except (ValueError, KeyError, TypeError) as e:
            response.update(status='error', message=str(e), steps=[])
        print(json.dumps(response,sort_keys=True),flush=True)

if __name__ == '__main__':
    main()
