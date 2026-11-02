#!/usr/bin/env python3
"""Identity-gated read-only actual RequirementSystem codec check; no engine run."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

from save_normalization_gameeffects import (
    MODULE_SHA, INITIAL_SHA, RELOADED_SHA, REQUIREMENT_START, REQUIREMENT_END, requirements,
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--canonical', type=Path, required=True)
    parser.add_argument('--codec', type=Path, required=True)
    parser.add_argument('--iterations', type=int, default=200)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.iterations <= 0:
        parser.error('iterations must be positive')
    manifest = json.loads((args.canonical / "manifest.json").read_text())
    if manifest["module_sha256"] != MODULE_SHA:
        raise ValueError("canonical native module identity mismatch")
    boundary_evidence = Path(__file__).resolve().parents[2] / 'evidence/linux/save-normalization-gameeffects.json'
    evidence_bytes = boundary_evidence.read_bytes()
    evidence = json.loads(evidence_bytes)
    if (evidence['module_sha256'] != MODULE_SHA
            or evidence['requirements']['start'] != REQUIREMENT_START
            or evidence['requirements']['end'] != REQUIREMENT_END
            or evidence['requirements']['count'] != 28304):
        raise ValueError('native owner evidence mismatch')
    results = []
    with tempfile.TemporaryDirectory(prefix='c6-requirements-') as temporary:
        directory = Path(temporary)
        for identity in (INITIAL_SHA, RELOADED_SHA):
            with gzip.open(args.canonical / (identity + '.bin.gz'), 'rb') as stream:
                data = stream.read(80657463)
            if len(data) != 80657462 or hashlib.sha256(data).hexdigest() != identity:
                raise ValueError('canonical stream identity mismatch')
            records = requirements(data)
            section = data[REQUIREMENT_START:REQUIREMENT_END]
            source, output, boundaries = [directory / name for name in ('input', 'output', 'boundaries')]
            source.write_bytes(section)
            process = subprocess.run([str(args.codec.resolve()), str(source), str(output),
                                      str(boundaries), str(args.iterations)],
                                     check=True, capture_output=True, text=True)
            if output.read_bytes() != section:
                raise ValueError('Rust roundtrip mismatch')
            rust = [tuple(map(int, line.split(',')))
                    for line in boundaries.read_text().splitlines()]
            expected = [(record['offset'] - REQUIREMENT_START,
                         record['contextId_offset'] - REQUIREMENT_START,
                         record['contextId'], record['subjectId'], record['state'])
                        for record in records]
            if rust != expected:
                raise ValueError('typed field boundary/value mismatch')
            benchmark = json.loads(process.stdout)
            if benchmark['records'] != len(records) or benchmark['bytes'] != len(section):
                raise ValueError('Rust scope mismatch')
            results.append({'native_save_sha256': identity,
                            'section_sha256': hashlib.sha256(section).hexdigest(),
                            'exact_roundtrip': True, 'all_record_field_boundaries_equal': True,
                            'states_raw': sorted({record['state'] for record in records}),
                            'benchmark': benchmark})
    report = {
        'schema': 'c6-linux-requirements-codec-v1',
        'target_gamecore_sha256': 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa',
        'codec_sha256': hashlib.sha256(args.codec.read_bytes()).hexdigest(),
        'native_boundary_evidence': 'evidence/linux/save-normalization-gameeffects.json',
        'native_boundary_evidence_sha256': hashlib.sha256(evidence_bytes).hexdigest(),
        'owner_start': REQUIREMENT_START, 'owner_end': REQUIREMENT_END,
        'fixtures': results,
        'scope': 'Extracted RequirementSystem v3 records only; no engine execution, lookup, evaluation, allocator or complete state/action/turn parity.',
        'raw_save_load_different_bytes_retained': 89198,
    }
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
