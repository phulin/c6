#!/usr/bin/env python3
"""Run independent sidecar checks using one private Unit solely as a decode seed."""
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
    parser.add_argument('--unit-seed', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    harness = ROOT / 'tools/linux/live_initialization_review.rs'
    with tempfile.TemporaryDirectory(prefix='c6-live-initialization-review-') as temporary:
        project = Path(temporary)
        manifest = project / 'Cargo.toml'
        manifest.write_text('[package]\nname="live-initialization-review"\nversion="0.0.0"\n'
                            'edition="2021"\n[dependencies]\nc6-core={path='
                            + json.dumps(str(ROOT / 'crates/c6-core')) + '}\n'
                            '[[bin]]\nname="review"\npath='
                            + json.dumps(str(harness)) + '\n')
        command = ['cargo', 'run', '--offline', '--manifest-path', str(manifest),
                   '--target-dir', '/tmp/c6-live-initialization-review-target',
                   '--', str(args.unit_seed.resolve())]
        result = subprocess.run(command, text=True, capture_output=True)
        if result.returncode:
            raise RuntimeError(result.stderr)
        audits = {}
        for name in ['unit', 'formation']:
            audit_report = project / (name + '-audit.json')
            subprocess.run(['python3', str(ROOT / 'tools/linux' / (name + '_initialization_audit.py')),
                            '--output', str(audit_report)], text=True, capture_output=True, check=True)
            record = json.loads(audit_report.read_text())
            audits[name] = dict(report_sha256=digest(audit_report),
                                gamecore_sha256=record['gamecore_sha256'],
                                host_sha256=record['host_sha256'],
                                methods=record['methods'])
    paths = ['crates/c6-core/src/unit_initialization.rs',
             'crates/c6-core/src/formation_initialization.rs',
             'crates/c6-core/src/world.rs', 'tools/linux/live_initialization_review.rs',
             'tools/linux/live_initialization_review.py']
    report = dict(schema='c6-linux-live-initialization-review-v1',
                  formation_source_commit='5809ff8323ecd8b473698f9c77cf8bf7fb893a4b',
                  original_unit_source_commit='10d404c',
                  unit_overlay_fix_commit='3760ee7',
                  source_sha256={path: digest(ROOT / path) for path in paths},
                  pinned_static_audits=audits,
                  unit_seed_sha256=digest(args.unit_seed),
                  seed_role='decode seed only; inputs and expected values are modified source fixtures',
                  stdout=result.stdout.strip(), public_api_checks_passed=True,
                  checks=['wire hash retained and distinct runtime definition index',
                          'changing lookup answers are cached exactly once per pair',
                          'all six reset/overlay arrays plus disabled operations',
                          'ordered duplicates, ignored unknown keys, absent defaults',
                          'owner and profile mismatch; missing definition and invalid index',
                          'no original owned Unit mutation on success or errors',
                          'actual owner Formation low16 lookup preserves both generations',
                          'missing collection errors; known empty, sentinel, missing slot null',
                          'Unit dirty baseline remains unavailable',
                          'Formation local dirty trace and wrong-object errors before mutation'],
                  original_execution=False, native_parity=False,
                  initialized_world_acceptance=False, action_turn_acceptance=False)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
    print(result.stdout.strip())


if __name__ == '__main__':
    main()
