#!/usr/bin/env python3
"""Static target gates and synthetic-only validation for the optional passive helper."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
from save_lifecycle_audit import Elf, GAME_SHA

ROOT = Path(__file__).resolve().parents[2]
EXTRA = [
    ('Player.Manager.Initialize', 0x2b51770, 0xbd),
    ('Session.GetFromPlayer', 0x27814b0, 3),
    ('Session.GetToPlayer', 0x27814c0, 4),
    ('EASTL.pending_list.base_constructor', 0x2917ed0, 0x23),
    ('EASTL.pending_list.DoInit', 0x2917f00, 8),
    ('EASTL.pending_list.begin', 0x2917ba0, 0x11),
    ('EASTL.pending_list.end', 0x2917bd0, 0xe),
    ('EASTL.pending_list.iterator_dereference', 0x2917be0, 8),
    ('EASTL.pending_list.iterator_increment', 0x2917bf0, 0xd),
    ('EASTL.pending_list.size', 0x2917c00, 0x34),
]
PATHS = ['native/linux/diplomacy_manager_capture.c',
         'native/linux/diplomacy_manager_capture.h',
         'tools/check/diplomacy_manager_capture_test.c',
         'tools/linux/diplomacy_manager_capture_decode.py',
         'tools/check/test_diplomacy_manager_capture_decode.py',
         'tools/linux/diplomacy_manager_capture_audit.py',
         'docs/linux-diplomacy-manager-capture.md',
         'evidence/linux/diplomacy-manager-source.json']


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--module', type=Path, default=ROOT.parent / 'c6-assets-linux/libGameCore_XP2.so')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    preimages = {name: (ROOT / name).read_bytes() for name in PATHS}
    elf = Elf(args.module, GAME_SHA)
    contract = json.loads(preimages['evidence/linux/diplomacy-manager-source.json'])
    methods = contract['methods'].copy()
    for row in methods:
        if sha(elf.read(int(row['rva'], 16), row['bytes'])) != row['sha256']:
            raise ValueError('source target method differs: ' + row['name'])
    methods.extend(dict(name=name, rva=hex(address), bytes=size,
                        sha256=sha(elf.read(address, size))) for name, address, size in EXTRA)
    # Nested EASTL header: first word is next; +16 is allocator, not a count.
    checks = [(0x2917eda, '488d7b10'), (0x2917f00, '48893f48897f08c3'),
              (0x2917ba4, '488b36'), (0x2917bf3, '488b00'),
              (0x2b517e3, 'be41000000'), (0x2b51804, 'be41000000')]
    for address, expected in checks:
        if elf.read(address, len(bytes.fromhex(expected))) != bytes.fromhex(expected):
            raise ValueError('layout instruction differs: ' + hex(address))
    helper = preimages['native/linux/diplomacy_manager_capture.c'].decode()
    for forbidden in ['dlopen(', 'dlsym(', 'mprotect(', 'ptrace(', 'raise(', 'signal(']:
        if forbidden in helper:
            raise ValueError('forbidden helper operation: ' + forbidden)
    with tempfile.TemporaryDirectory(prefix='c6-diplomacy-passive-audit-') as directory:
        directory = Path(directory)
        obj = directory / 'capture.o'
        binary = directory / 'synthetic-test'
        flags = ['cc', '-std=c11', '-Wall', '-Wextra', '-Werror']
        subprocess.run(flags + ['-c', str(ROOT / PATHS[0]), '-o', str(obj)], check=True, capture_output=True)
        imports = sorted(line.split()[-1] for line in subprocess.check_output(['nm', '-u', str(obj)], text=True).splitlines())
        allowed = {'fprintf', 'fputs', 'fputc', 'ferror', 'strlen', 'strchr', 'strcmp',
                   'syscall', 'process_vm_readv', 'getpid', 'strnlen', 'fwrite',
                   '__stack_chk_fail', '_GLOBAL_OFFSET_TABLE_'}
        if set(imports) - allowed:
            raise ValueError('unexpected direct helper imports: ' + str(set(imports) - allowed))
        subprocess.run(flags + [str(ROOT / PATHS[0]), str(ROOT / PATHS[2]), '-o', str(binary)], check=True, capture_output=True)
        test = subprocess.run([str(binary)], check=True, capture_output=True, text=True)
        python_test = subprocess.run(['python3', str(ROOT / PATHS[4])], check=True, capture_output=True, text=True)
        executable = sha(binary.read_bytes())
    if any((ROOT / name).read_bytes() != data for name, data in preimages.items()):
        raise ValueError('source changed during audit')
    result = dict(schema='c6-linux-diplomacy-passive-capture-audit-v1', gamecore_sha256=GAME_SHA,
                  sources={name: sha(data) for name, data in preimages.items()}, methods=methods,
                  instruction_gates=[dict(rva=hex(address), bytes=value) for address, value in checks],
                  helper_direct_imports=imports, synthetic_binary_sha256=executable,
                  compiler=subprocess.check_output(['cc', '--version'], text=True).splitlines()[0],
                  synthetic_c_tests=test.stdout.strip(), synthetic_decoder_tests=python_test.stderr.strip(),
                  original_execution=False, native_capture_performed=False,
                  active_loader_or_cli_modified=False, optional_future_profile_only=True,
                  pending_payloads='unknown', current_is_ai='unknown_configuration_schema',
                  semantic_completeness=False)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'target_methods': len(methods), 'synthetic_tests_passed': True, 'native_capture': False}))


if __name__ == '__main__':
    main()
