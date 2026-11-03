#!/usr/bin/env python3
"""Build a separate offline placement benchmark from immutable committed source.

The frozen comparison candidate and native profile are never changed. The exact
candidate loader and validation are retained; an appended timing fragment runs
only after the full result set has been checked against closed native answers.
"""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parents[2]
SOURCE = '301f9e9d188f6a5ac815914218d0380f51b25938'
ARCHIVE = '560bb94b98da8a187118e6cbc82f60d279e6d008c12884bd07a1b878c1b6b559'
CANDIDATE = 'crates/c6-differential/src/bin/c6-linux-placement-count-candidate.rs'
FRAGMENT = 'tools/linux/placement_query_benchmark.rs'
PROFILE = '9022128238f6209173eec2532b47405721780f1e2a105808ee9084306ebc2f33'

def sha(data):
    return hashlib.sha256(data).hexdigest()

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--harness-commit', required=True)
    p.add_argument('--input', type=Path, required=True)
    p.add_argument('--input-sha256', required=True)
    p.add_argument('--expected', type=Path, required=True)
    p.add_argument('--comparison-report', type=Path, required=True)
    p.add_argument('--directory', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    assert not a.output.exists(), 'report already exists'
    directory = a.directory.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    source = directory / 'source'
    source.mkdir()
    archive = subprocess.check_output(['git','archive','--format=tar',SOURCE], cwd=ROOT)
    assert sha(archive) == ARCHIVE, 'immutable source archive differs'
    (directory / 'source.tar').write_bytes(archive)
    with tarfile.open(fileobj=io.BytesIO(archive)) as t:
        t.extractall(source, filter='data')
    harness_commit = subprocess.check_output(['git','rev-parse',a.harness_commit+'^{commit}'], cwd=ROOT, text=True).strip()
    fragment = subprocess.check_output(['git','show',f'{harness_commit}:{FRAGMENT}'], cwd=ROOT)
    script = subprocess.check_output(['git','show',f'{harness_commit}:tools/linux/placement_query_benchmark.py'], cwd=ROOT)
    assert script == Path(__file__).read_bytes(), 'runner is not the requested committed harness'
    original = (source / CANDIDATE).read_text()
    marker = '    require(\n        digest(&world.map().encode(map::Limits::default())?) == request.map.sha256,'
    assert original.count(marker) == 1
    generated = original.replace(marker, '    let benchmark = benchmark_queries(&world, &inputs, &counts, &bindings)?;\n'+marker)
    marker = '"count_results":counts,"unit_binding_results":bindings,'
    assert generated.count(marker) == 1
    generated = generated.replace(marker, '"benchmark":benchmark,'+marker)
    generated += '\n'+fragment.decode()
    bin_name = 'c6-linux-placement-query-benchmark'
    generated_path = source / f'crates/c6-differential/src/bin/{bin_name}.rs'
    generated_path.write_text(generated)
    env = os.environ.copy()
    for key in ['CARGO_TARGET_DIR','CARGO_BUILD_TARGET','RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','CARGO_BUILD_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER']:
        env.pop(key, None)
    env.update(CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE='true')
    build = ['cargo','build','--locked','--offline','--release','-p','c6-differential','--bin',bin_name]
    subprocess.run(build, cwd=source, env=env, check=True)
    subprocess.run(['cargo','clippy','--locked','--offline','--release','-p','c6-differential','--bin',bin_name,'--','-D','warnings'], cwd=source, env=env, check=True)
    input_bytes = a.input.read_bytes()
    assert sha(input_bytes) == a.input_sha256, 'strict projection identity differs'
    comparison_bytes = a.comparison_report.read_bytes()
    comparison = json.loads(comparison_bytes)
    canonical_input = json.dumps(json.loads(input_bytes),sort_keys=True,separators=(',',':')).encode()
    assert (comparison['schema'] == 'c6-linux-placement-count-native-comparison-v1'
        and comparison['status'] == 'equal' and comparison['count_mismatches'] == 0
        and comparison['binding_mismatches'] == 0 and comparison['candidate_source_commit'] == SOURCE
        and comparison['profile_sha256'] == PROFILE
        and comparison['candidate_input_projection_sha256'] == sha(canonical_input)), 'accepted strict projection differs'
    candidate = source / 'target/release' / bin_name
    env['C6_PLACEMENT_BENCH_EXPECTED'] = str(a.expected.resolve())
    result = subprocess.run([str(candidate),str(a.input.resolve()),PROFILE,PROFILE], cwd=source, env=env, capture_output=True, check=True)
    (directory / 'benchmark.stdout').write_bytes(result.stdout)
    (directory / 'benchmark.stderr').write_bytes(result.stderr)
    report = json.loads(result.stdout)['benchmark']
    report.update(source_commit=SOURCE, source_snapshot_sha256=sha(archive), harness_commit=harness_commit,
        harness_fragment_sha256=sha(fragment), harness_runner_sha256=sha(script),
        generated_source_sha256=sha(generated.encode()), benchmark_binary_sha256=sha(candidate.read_bytes()),
        input_sha256=sha(input_bytes), expected_sha256=sha(a.expected.read_bytes()), output_sha256=sha(result.stdout),
        accepted_comparison_report_sha256=sha(comparison_bytes),
        configuration_sha256=PROFILE, profile_sha256=PROFILE, build_command=build,
        rustc_version=subprocess.check_output(['rustc','-Vv'],text=True).strip(),
        machine=subprocess.check_output(['uname','-m'],text=True).strip(),
        cpu_model=next((line.split(':',1)[1].strip() for line in Path('/proc/cpuinfo').read_text().splitlines() if line.startswith('model name')), 'unavailable'),
        strict_release_clippy_passed=True, frozen_candidate_or_profile_modified=False)
    a.output.parent.mkdir(parents=True,exist_ok=True)
    with a.output.open('x') as f:
        json.dump(report,f,indent=2); f.write('\n')
    print(json.dumps({m['mode']:m['ns_per_case_median'] for m in report['modes']}))

if __name__ == '__main__':
    main()
