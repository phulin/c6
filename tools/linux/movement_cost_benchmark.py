#!/usr/bin/env python3
"""Build and measure accepted prepared-fact arithmetic; never execute game code."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
BIN_SOURCE = 'crates/c6-differential/src/bin/c6-movement-cost-bench.rs'
DRIVER_SOURCE = 'tools/linux/movement_cost_benchmark.py'
FROZEN = 'd4dddd41f8f157a951c5b43720e6a217fef1b6600439b307ee01e300c7ce6718'
SCALAR = '676f9803bec5bb7157513044850dee7e3892b30fb8c6abedf40d40cbcdf77652'
NATIVE = '0314b8153d2d6e0bb63f70ec22fb4513988fa0a3bfa08c6d9da237c22241657f'


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def run(command, **kwargs):
    return subprocess.run(command, cwd=ROOT, check=True, capture_output=True,
                          text=True, **kwargs).stdout


def sources():
    paths = [Path('Cargo.lock'), Path('Cargo.toml'), Path('crates/c6-core/Cargo.toml'),
             Path('crates/c6-differential/Cargo.toml'), Path('crates/c6-data/Cargo.toml'),
             Path(BIN_SOURCE), Path(DRIVER_SOURCE)]
    for directory in ['crates/c6-core/src', 'crates/c6-data/src']:
        paths += sorted(p.relative_to(ROOT) for p in (ROOT / directory).rglob('*.rs'))
    return {str(p): digest(ROOT / p) for p in paths}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--native', type=Path, required=True)
    parser.add_argument('--control', type=Path, required=True)
    parser.add_argument('--accepted-report', type=Path, required=True)
    parser.add_argument('--accepted-report-sha256', required=True)
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--source-manifest', type=Path, required=True)
    parser.add_argument('--contract', type=Path, default=ROOT / 'evidence/linux/movement-cost-probe-xp2-contract.json')
    parser.add_argument('--rounds', type=int, default=10000)
    parser.add_argument('--samples', type=int, default=7)
    parser.add_argument('--warmup-rounds', type=int, default=1000)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    require(1 <= args.rounds <= 1000000 and 1 <= args.samples <= 100
            and 0 <= args.warmup_rounds <= 1000000, 'invalid iteration bounds')
    require(platform.system() == 'Linux' and platform.machine() == 'x86_64', 'Linux x86_64 only')
    started = time.perf_counter_ns()
    accepted_hash = digest(args.accepted_report)
    require(accepted_hash == args.accepted_report_sha256, 'accepted report identity differs')
    report = json.loads(args.accepted_report.read_text())
    require(report['schema'] == 'c6-linux-movement-cost-results-v1', 'unexpected report')
    require(report['scalar_comparison_equal'] is True and report['candidate_exit'] == 0
            and report['phase_state_mismatches'] == 0 and report['compared_cases'] == 1536
            and report['compared_phase_states'] == 13824, 'scalar comparison not accepted')
    require(report['fresh_control_facts_equal'] is True and report['registries_equal'] is True
            and report['provided_raw_snapshot_pairs_equal'] is True
            and report['known_rng_equal'] == {'cost': True, 'control': True}, 'control/state gates failed')
    require(len(report['raw_snapshot_pairs']) >= 10 and all(
        p['different_bytes'] == 0 and p['masked_bytes'] == 0
        for p in report['raw_snapshot_pairs'].values()), 'raw snapshots not accepted')
    expected = [(args.native, 'native_corpus_sha256'), (args.control, 'control_corpus_sha256'),
                (args.candidate, 'candidate_sha256'), (args.source_manifest, 'source_manifest_sha256'),
                (args.contract, 'source_contract_sha256')]
    for path, key in expected:
        require(digest(path) == report[key], 'identity mismatch: ' + key)
    require(report['native_corpus_sha256'] == NATIVE and report['candidate_sha256'] == FROZEN,
            'unexpected accepted artifacts')
    require(digest(ROOT / 'crates/c6-core/src/movement.rs') == SCALAR, 'accepted arithmetic source changed')
    manifest = json.loads(args.source_manifest.read_text())
    require(manifest['candidate_sha256'] == FROZEN
            and manifest['source_reference_commit'] == report['candidate_source_revision'], 'manifest identity differs')
    for path, expected_hash in manifest['sources_at_reference_commit'].items():
        data = subprocess.check_output(['git', 'show', manifest['source_reference_commit'] + ':' + path], cwd=ROOT)
        require(hashlib.sha256(data).hexdigest() == expected_hash, 'immutable source mismatch: ' + path)
    frozen_rows = [json.loads(line) for line in run([
        str(args.candidate.resolve()), str(args.native.resolve())], timeout=120).splitlines()]
    require(len(frozen_rows) == report['compared_cases'] + 1
            and all(r.get('equal') is True for r in frozen_rows[:-1]), 'frozen candidate output mismatch')
    require(frozen_rows[-1] == report['frozen_rust_candidate'], 'frozen candidate summary differs')
    gating_ns = time.perf_counter_ns() - started
    source_hashes = sources()
    build_command = ['cargo', 'build', '--release', '--locked', '-p', 'c6-differential',
                     '--bin', 'c6-movement-cost-bench']
    began = time.perf_counter_ns()
    run(build_command, timeout=300)
    build_ns = time.perf_counter_ns() - began
    require(sources() == source_hashes, 'source changed during build')
    metadata = json.loads(run(['cargo', 'metadata', '--no-deps', '--format-version', '1']))
    binary = Path(metadata['target_directory']) / 'release/c6-movement-cost-bench'
    binary_hash = digest(binary)
    require(binary_hash != FROZEN, 'benchmark must be a distinct artifact')
    measured = json.loads(run([str(binary), str(args.native.resolve()), str(args.rounds),
                              str(args.samples), str(args.warmup_rounds)], timeout=300))
    require(measured['schema'] == 'c6-linux-movement-cost-bench-run-v1'
            and measured['validated_cases'] == report['compared_cases']
            and measured['input_sha256'] == NATIVE
            and measured['profile_sha256'] == report['native_profile'], 'benchmark validation differs')
    require([m['mode'] for m in measured['measurements']] == ['no_zoc', 'zoc_aware'], 'missing benchmark modes')
    for mode in measured['measurements']:
        require(len(mode['samples']) == args.samples
                and mode['edges_per_sample'] == args.rounds * report['compared_cases'], 'sample count differs')
        key = 'rust_full' if mode['mode'] == 'zoc_aware' else 'rust_no_zoc'
        expected_checksum = (sum(r[key] & 0xffffffff for r in frozen_rows[:-1])
                             * args.rounds * args.samples) & 0xffffffffffffffff
        require(mode['checksum'] == expected_checksum, 'timed output checksum differs')
        values = [s['ns_per_edge'] for s in mode['samples']]
        require(all(s['elapsed_ns'] > 0 and s['ns_per_edge'] == s['elapsed_ns'] / mode['edges_per_sample']
                    for s in mode['samples']), 'invalid timing output')
        mode['median_ns_per_edge'] = statistics.median(values)
        mode['min_ns_per_edge'] = min(values)
        mode['max_ns_per_edge'] = max(values)
    require(digest(binary) == binary_hash and sources() == source_hashes
            and digest(args.accepted_report) == accepted_hash, 'artifacts changed during measurement')
    result = dict(schema='c6-linux-movement-cost-benchmark-v1',
        measured_at_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),
        accepted_report_sha256=accepted_hash, accepted_candidate_sha256=FROZEN,
        accepted_candidate_source_revision=report['candidate_source_revision'],
        source_manifest_sha256=report['source_manifest_sha256'],
        native_corpus_sha256=NATIVE, control_corpus_sha256=report['control_corpus_sha256'],
        source_contract_sha256=report['source_contract_sha256'],
        source_hashes=source_hashes, checkout_head=run(['git', 'rev-parse', 'HEAD']).strip(),
        benchmark_binary_sha256=binary_hash, build_command=build_command,
        rustc=run(['rustc', '-Vv']).strip(), cargo=run(['cargo', '-V']).strip(),
        build_environment={k: os.environ.get(k) for k in ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS',
            'CARGO_TARGET_DIR', 'RUSTC_WRAPPER', 'CARGO_PROFILE_RELEASE_OPT_LEVEL', 'CARGO_PROFILE_RELEASE_LTO']},
        machine=dict(system=platform.system(), machine=platform.machine(), kernel=platform.release(),
                     cpu_model=next((s.split(':', 1)[1].strip() for s in Path('/proc/cpuinfo').read_text().splitlines()
                                     if s.startswith('model name')), 'unknown'),
                     logical_cpus=os.cpu_count(), affinity=sorted(os.sched_getaffinity(0))),
        untimed_driver_gating_ns=gating_ns, untimed_build_ns=build_ns, run=measured,
        coverage=dict(formations=256, edges=report['compared_cases'],
            truncated=report['native_scan_truncated'], formation_layers=report['formation_layers'],
            actual_variations=report['actual_variations']),
        timed_work='reset accumulator, prepared-fact rules, optional supplied-ZOC minimum, final cost, black_box and checksum',
        exclusions=['JSON/preparation/validation', 'world/runtime query resolution', 'formation aggregation',
            'rules registry lookup', 'codec/save transport', 'native calls', 'movement legality/actions',
            'gameplay state mutation', 'RNG', 'turns', 'external policies'],
        original_code_speedup=None, full_movement_parity=False, complete_gameplay_state=False,
        complete_turns=False, save_load_89198_byte_failure='separate unresolved failure; no bytes excluded')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(dict(output=str(args.output), benchmark_binary_sha256=binary_hash,
        medians_ns_per_edge={m['mode']: m['median_ns_per_edge'] for m in measured['measurements']})))


if __name__ == '__main__':
    main()
