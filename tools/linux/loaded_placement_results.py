#!/usr/bin/env python3
"""Compare loaded-World queries to the closed native snapshot, without execution."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

EXPECTED = 'bc65088165f1b57473738f6ec5f784c86b838060d3c8f2c87a8bebcd5240c882'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require(value, reason):
    if not value:
        raise RuntimeError(reason)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--freeze', type=Path, required=True)
    p.add_argument('--manifest', type=Path, required=True)
    p.add_argument('--target-runtime', type=Path, required=True)
    p.add_argument('--placement', type=Path, required=True)
    p.add_argument('--manager', type=Path, required=True)
    p.add_argument('--diagnostics', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    freeze = json.loads(a.freeze.read_text())
    binary = Path(freeze['candidate_path'])
    require(digest(binary) == freeze['candidate_sha256'], 'candidate identity changed')
    require(digest(Path(freeze['source_archive'])) == freeze['source_archive_sha256'],
            'committed source archive changed')
    require(freeze['expected_answers_used_as_inputs'] is False, 'invalid preparation scope')
    require(freeze['checker_sha256'] == digest(Path(__file__)), 'checker identity changed')
    a.diagnostics.mkdir(parents=True, exist_ok=False)
    command = [str(binary), str(a.manifest), str(a.target_runtime),
               str(a.placement), str(a.manager)]
    r = subprocess.run(command, capture_output=True, timeout=300)
    (a.diagnostics / 'stdout.json').write_bytes(r.stdout)
    (a.diagnostics / 'stderr.txt').write_bytes(r.stderr)
    require(r.returncode == 0, 'candidate failed; preserved diagnostics')
    actual = json.loads(r.stdout)
    require(actual['candidate_sha256'] == freeze['candidate_sha256'], 'output identity differs')
    require(actual['schema'] == 'c6-loaded-placement-results-v1'
            and actual['world_revision'] == 0
            and actual['expected_answers_used_as_inputs'] is False
            and actual['native_read_equivalence'] is False, 'candidate scope differs')
    # Native answers are opened only after the facts -> loaded World -> query run.
    expected_path = a.placement / 'expected-counts.jsonl'
    require(digest(expected_path) == EXPECTED, 'native expected answers changed')
    expected = [json.loads(line) for line in expected_path.read_text().splitlines()]
    require(expected[0]['schema'] == 'c6-linux-placement-count-expected-v1'
            and expected[0]['answers_are_inputs'] is False
            and expected[-1] == {'kind': 'summary', 'status': 0}, 'expected framing differs')
    differences = {}
    totals = {}
    for key, output_kind, native_kind in [
            ('count_results', 'count_result', 'count_expected'),
            ('unit_binding_results', 'unit_binding_result', 'unit_binding_expected')]:
        calculated = actual[key]
        original = [row for row in expected if row['kind'] == native_kind]
        require(len(calculated) == len(original), 'query coverage differs')
        differences[key] = [dict(index=i, calculated=x, expected=y)
                            for i, (x, y) in enumerate(zip(calculated, original))
                            if x != dict(y, kind=output_kind)]
        totals[key] = len(original)
    (a.diagnostics / 'mismatches.json').write_text(json.dumps(differences, indent=2) + '\n')
    result = dict(schema='c6-loaded-placement-comparison-v1', source_commit=freeze['source_commit'],
                  candidate_sha256=freeze['candidate_sha256'], checker_sha256=digest(Path(__file__)),
                  input_manifest_sha256=digest(a.manifest), target_runtime_sha256=digest(a.target_runtime),
                  native_expected_sha256=EXPECTED, output_sha256=hashlib.sha256(r.stdout).hexdigest(),
                  counts=totals, mismatches={k: len(v) for k, v in differences.items()},
                  decode_role=actual['decode_role'], snapshot_source_role=actual['snapshot_source_role'],
                  source_core_sha256=actual['source_core_sha256'],
                  native_execution=False, full_action_parity=False, native_read_equivalence=False,
                  accepted=not any(differences.values()))
    a.output.write_text(json.dumps(result, sort_keys=True, indent=2) + '\n')
    require(result['accepted'], 'query differences preserved')


if __name__ == '__main__':
    main()
