#!/usr/bin/env python3
"""Negative integration checks against the real adapters, never fake parity."""
import json
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[1]
COMMANDS = [['python3', 'tools/oracle/run.py'], ['target/debug/c6-rng-candidate']]
STATE = {'rng': {'seed': 1, 'calls': 0, 'resets': 0, 'lock_count': 0, 'synchronous': False}}

def main():
    checks = 0
    for command in COMMANDS:
        engine = json.loads(subprocess.check_output(command + ['--identity'], cwd=ROOT, text=True))
        for steps, wrong_identity in [([{'op': 'end_turn'}], False),
                                      ([{'op': 'rng_get', 'bound': 10}, {'op': 'rng_get', 'bound': 10, 'stream_id': 999}], False),
                                      ([{'op': 'rng_get', 'bound': 10}], True)]:
            expected = dict(engine)
            if wrong_identity:
                expected['binary_sha256'] = '0' * 64
            request = {'protocol_version': 1, 'expected_engine': expected,
                       'case': {'id': 'negative', 'initial_state': STATE, 'steps': steps}}
            output = subprocess.check_output(command, input=json.dumps(request)+'\n', cwd=ROOT, text=True)
            response = json.loads(output)
            assert response['status'] != 'ok', (command, response)
            assert response['steps'] == [], (command, response)
            checks += 1
    print(f'{checks} real-adapter rejection checks passed')

if __name__ == '__main__':
    main()
