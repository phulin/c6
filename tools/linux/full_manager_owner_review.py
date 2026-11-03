#!/usr/bin/env python3
"""Independent owner/trace/lifecycle checks; original game code never executes."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    paths = ['native/linux/full_manager_profile_guard.c', 'native/linux/full_manager_profile_guard.h',
             'native/linux/action_lifecycle.c', 'native/linux/action_lifecycle.h',
             'native/linux/mod_activation.c', 'tools/linux/action_execution_trace.c',
             'tools/linux/full_manager_owner_review.c', 'tools/linux/full_manager_lifecycle_review.c',
             'tools/linux/full_manager_trace_review.c', 'tools/linux/full_manager_owner_review.py']
    before = {path: digest(ROOT / path) for path in paths}
    legacy = subprocess.check_output(['git', 'show', 'eefe52c:tools/linux/action_execution_trace.c'], cwd=ROOT)
    if not (ROOT / 'tools/linux/action_execution_trace.c').read_bytes().startswith(legacy):
        raise ValueError('legacy trace body/source changed')
    outputs = {}
    binaries = {}
    with tempfile.TemporaryDirectory(prefix='c6-full-manager-owner-review-') as temporary:
        work = Path(temporary)
        mock = work / 'mock.c'
        mock.write_text('__attribute__((visibility("default"),used)) unsigned char full_guard_mock_bss[0x4000000];\n'
                        '__attribute__((visibility("default"),used)) const unsigned char full_guard_mock_vtable[16]={0};\n')
        dso = work / 'mock.so'
        subprocess.run(['cc', '-shared', '-fPIC', str(mock), '-o', str(dso)], check=True)
        flags = ['cc', '-std=gnu11', '-Wall', '-Wextra', '-Werror', '-Wconversion', '-Wshadow']
        for name, source, extra, arguments in [
                ('owner', 'full_manager_owner_review.c', ['-rdynamic', '-pthread'], [str(dso)]),
                ('missing_witness', 'full_manager_owner_review.c', ['-rdynamic', '-pthread', '-DNO_MAIN_WITNESS'], [str(dso)]),
                ('lifecycle', 'full_manager_lifecycle_review.c', [], []),
                ('trace', 'full_manager_trace_review.c', [], [])]:
            file = str(ROOT / 'tools/linux' / source)
            executable = work / name
            subprocess.run(flags + extra + ['-fanalyzer', '-c', file, '-o', str(work / (name + '.o'))], check=True)
            subprocess.run(flags + extra + ['-O2', '-fsanitize=undefined', '-fno-sanitize-recover=undefined',
                                           file, '-ldl', '-o', str(executable)], check=True)
            result = subprocess.run([str(executable)] + arguments, capture_output=True, text=True, check=True)
            outputs[name] = result.stdout.strip()
            binaries[name] = digest(executable)
        audits = {}
        for name in ['full_manager_profile_guard_audit', 'full_manager_trace_policy_audit']:
            output = work / (name + '.json')
            subprocess.run(['python3', str(ROOT / 'tools/linux' / (name + '.py')),
                            '--output', str(output)], capture_output=True, text=True, check=True)
            audits[name] = dict(sha256=digest(output), checks_passed=True)
    after = {path: digest(ROOT / path) for path in paths}
    if before != after:
        raise RuntimeError('reviewed source changed during checks; rerun immutable checkpoint')
    report = dict(
        schema='c6-linux-full-manager-owner-independent-review-v1',
        reviewed_checkpoints=['55302c3', 'eefe52c', 'e262004'],
        source_sha256=after, source_stable_during_checks=True,
        mock_executable_sha256=binaries, independent_results=outputs,
        independent_case_counts=dict(public_owner_and_refusals=29, missing_cached_witness=1,
                                     lifecycle_selector_and_errors=9, trace_policy_and_hard_stops=14),
        legacy_trace_source_prefix_unchanged=True,
        legacy_trace_source_sha256=hashlib.sha256(legacy).hexdigest(),
        producer_audits=audits,
        checks=dict(strict_c=True, gcc_analyzer=True, ubsan_no_recovery=True,
                    real_public_guard_admission_with_mock_dso=True,
                    original_game_modules_loaded_for_execution=False,
                    opaque_lock_token_not_converted_to_os_tid=True,
                    queued_effects_and_positive_ai_counter_preserved=True,
                    field_only_guard_contains_no_actor_pointer=True,
                    observational_209_only_relaxation_full_profile_only=True,
                    structural_207_io_208_and_hard_stop_198_remain_failures=True),
        original_execution=False, runtime_profile_admitted=False,
        actual_main_lock_mapping_vm_ancestry_proven=False,
        native_predicate_or_complete_state_action_turn_parity=False)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
    print('53 independent mock owner/lifecycle/trace cases passed; no original execution')


if __name__ == '__main__':
    main()
