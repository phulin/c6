#!/usr/bin/env python3
"""Run bounded original job-set cleanup and explicit unsupported-input cases."""
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
EXE_SHA256 = 'e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63'

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    binary = ROOT.parent / 'c6-assets/Base/Binaries/Win64Steam/CivilizationVI.exe'
    if digest(binary) != EXE_SHA256:
        raise SystemExit('unsupported original EXE identity')
    sources = ['native/bootstrap/job-manager/probe.c', 'native/bootstrap/job-manager/job_manager.c']
    host = ROOT / 'target/bootstrap/job_manager_probe.exe'
    host.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(['x86_64-w64-mingw32-gcc', '-std=c11', '-O2', '-Wall', '-Wextra', '-Werror', '-municode', '-static', '-Wl,--image-base,0x150000000', '-o', str(host), *sources, '-lbcrypt'], cwd=ROOT, check=True)
    configuration = {'schema': 'c6-original-empty-job-set-probe-v1', 'original_executable_sha256': EXE_SHA256,
                     'sources': {s: digest(ROOT / s) for s in sources + ['native/bootstrap/job-manager/job_manager.h']},
                     'host_sha256': digest(host), 'full_executable_crt_startup': False,
                     'job_manager_initialized': False, 'native_constructor_rva': '0x985170',
                     'native_destroy_export_rva': '0x986040', 'native_wait_export_rva': '0x985e20', 'native_free_job_list_export_rva': '0x986050'}
    environment = dict(os.environ, WINEPREFIX=str(ROOT / 'target/bootstrap/wine-prefix'), WINEARCH='win64', WINEDEBUG='-all')
    target = 'Z:' + str(binary.resolve()).replace('/', '\\')
    cases = []
    for mode, expected_exit in [('native-empty', 0), ('nonempty', 86), ('null', 86), ('wait-nonempty', 86), ('wait-null', 86), ('free-nonzero', 86)]:
        command = ['/usr/lib/wine/wine64', str(host), target]
        if mode != 'native-empty':
            command.append(mode)
        result = subprocess.run(command, cwd=host.parent, env=environment, text=True, capture_output=True, timeout=60)
        cases.append({'mode': mode, 'expected_exit_code': expected_exit, 'child_exit_code': result.returncode,
                      'stdout': result.stdout, 'stderr': result.stderr, 'passed': result.returncode == expected_exit})
    report = {'configuration': configuration,
              'configuration_sha256': hashlib.sha256(json.dumps(configuration, sort_keys=True, separators=(',', ':')).encode()).hexdigest(),
              'cases': cases, 'initialized_game': False, 'job_execution_parity': False, 'full_turn_parity': False}
    (ROOT / 'evidence/bootstrap/job-manager.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'cases': [{k: v for k, v in c.items() if k not in ['stdout', 'stderr']} for c in cases]}, indent=2))
    if not all(case['passed'] for case in cases):
        raise SystemExit(1)

if __name__ == '__main__':
    main()
