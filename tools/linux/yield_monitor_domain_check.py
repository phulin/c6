#!/usr/bin/env python3
"""Build the owned Trade domain checker from a retained committed source tree."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import uuid

ROOT = Path(__file__).resolve().parents[2]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    paths = ['Cargo.toml', 'Cargo.lock', 'crates',
             'evidence/linux/current-handler-closed-review.json',
             'evidence/linux/rules-base-snapshot.json',
             'evidence/linux/rules-xp2-request-none-snapshot.json',
             'evidence/assets/xp2-rules-snapshot.json',
             'tools/linux/loaded_actions_review_load.rs',
             'tools/linux/yield_monitor_domain_check.rs',
             'tools/linux/yield_monitor_domain_check.py']
    # Archive HEAD directly. Concurrent work is excluded, without requiring a
    # whole-workspace lock or making unrelated owners checkpoint their changes.
    pending = subprocess.check_output(['git', 'diff', '--name-only', 'HEAD', '--', *paths], cwd=ROOT, text=True).splitlines()
    retained = ROOT / 'target/linux/yield-monitor-domain-candidates' / uuid.uuid4().hex
    retained.mkdir(parents=True)
    archive = retained / 'source.tar'
    subprocess.run(['git', 'archive', '--format=tar', '--output', str(archive), revision, *paths], cwd=ROOT, check=True)
    source = retained / 'source'
    source.mkdir()
    with tarfile.open(archive) as contents:
        contents.extractall(source, filter='data')
    manifest = {str(path.relative_to(source)): sha(path) for path in sorted(source.rglob('*')) if path.is_file()}
    (retained / 'source-manifest.json').write_text(json.dumps(manifest, indent=2, sort_keys=True) + '\n')
    package = retained / 'check'
    package.mkdir()
    cargo = package / 'Cargo.toml'
    cargo.write_text('[workspace]\n[package]\nname="yield-monitor-domain-check"\nversion="0.0.0"\nedition="2021"\n'
                     '[dependencies]\nserde_json="1"\nc6-core={path=' + json.dumps(str(source / 'crates/c6-core')) + '}\n'
                     'c6-data={path=' + json.dumps(str(source / 'crates/c6-data')) + '}\n'
                     '[[bin]]\nname="check"\npath=' + json.dumps(str(source / 'tools/linux/yield_monitor_domain_check.rs')) + '\n')
    shutil.copyfile(source / 'Cargo.lock', package / 'Cargo.lock')
    trace = retained / 'owner-trace.jsonl'
    trace_source = ROOT.parent / 'c6-ghidra/evidence/linux-save-normalization/checkpoint-203-owners/trace-benchmark-1.jsonl.gz'
    trace.write_bytes(gzip.decompress(trace_source.read_bytes()))
    if sha(trace) != '528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae':
        raise ValueError('owner trace identity differs')
    target = ROOT / 'target/linux/yield-monitor-domain-build'
    run = subprocess.run(['cargo', 'run', '--offline', '--manifest-path', str(cargo),
                          '--target-dir', str(target), '--', str(ROOT), str(trace)], text=True, capture_output=True)
    (retained / 'build-stderr.txt').write_text(run.stderr)
    if run.returncode:
        raise RuntimeError(run.stderr)
    result = json.loads(run.stdout)
    candidate = retained / 'candidate'
    shutil.copyfile(target / 'debug/check', candidate)
    result.update(source_revision=revision, source_archive_sha256=sha(archive),
                  retained_source=str(source.relative_to(ROOT)),
                  source_sha256=manifest, candidate_sha256=sha(candidate),
                  retained_candidate=str(candidate.relative_to(ROOT)),
                  generated_manifest_sha256=sha(cargo),
                  generated_lock_sha256=sha(package / 'Cargo.lock'),
                  workspace_changed_paths_excluded=pending,
                  source_contains_uncommitted_overlay=False)
    (retained / 'result.json').write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps({key: value for key, value in result.items() if key != 'source_sha256'}))


if __name__ == '__main__':
    main()
