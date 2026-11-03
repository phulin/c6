#!/usr/bin/env python3
"""Offline driver for the separately closed captured-input Military observation.

Reuses the immutable first-capture validator. Keeps both reports separate and
replaces that first report's fixture-specific missing-ff31-Read limit with the
actual role/lifetime scope of this fresh input. No original/candidate execution.
"""
import argparse
import json
from pathlib import Path

from military_initialization_capture_audit import (
    audit_artifacts, build_report, file_identity, mutation_tests, require,
)


def raw_difference(left, right):
    differing = 0
    with Path(left).open('rb') as before, Path(right).open('rb') as after:
        while True:
            a, b = before.read(1 << 20), after.read(1 << 20)
            if not a and not b:
                return differing
            if a != b:
                differing += sum(x != y for x, y in zip(a, b)) + abs(len(a)-len(b))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    capture = audit_artifacts(args.directory)
    gate = capture['config']['captured_core_stream_gate']
    require(gate['schema'] == 'c6-linux-game-load-captured-input-v1'
            and gate['profile'] == 'captured-initial', 'captured input gate scope')
    require(gate['sha256'] == capture['inputs']['core']['sha256']
            and gate['bytes'] == capture['inputs']['core']['bytes'], 'actual input/captured gate identity')
    frozen_gate = args.directory/'source/tools/linux/game_load_captured_gate.json'
    require(json.loads(frozen_gate.read_text()) == gate, 'captured gate source preimage')
    root = Path(__file__).resolve().parents[2]
    origin = file_identity(root/gate['origin_report'])
    require(origin['sha256'] == gate['origin_report_sha256'], 'historical origin reference identity')
    tests = mutation_tests(capture)
    report = build_report(capture, tests)
    old_limit = 'no incoming ff31 Read or initialized ff31 context captured'
    require(old_limit in report['limits'], 'first-capture validator scope changed')
    report['limits'].remove(old_limit)
    report['limits'].insert(1, 'fresh initialized input Read context only; no initialized Read of this capture\'s own output Core')
    report['classification_driver_sha256'] = file_identity(__file__)['sha256']
    report['captured_input_origin_gate'] = dict(
        source_preimage=file_identity(frozen_gate), origin_reference=origin,
        actual_input_core_sha256=capture['inputs']['core']['sha256'],
        own_output_core_sha256=sorted({item['sha256'] for item in capture['outputs'].values()}),
        historical_origin_reference_does_not_supply_initialization=True,
        initialization_source='fresh matched original Read/Initialize call witnesses in this capture',
        earlier_capture_unchanged=True)
    input_path = capture['config']['lower_core_military_initialization_observer']['input_core_artifact']['path']
    report['unmasked_original_load_save_comparison'] = dict(
        input_core_sha256=capture['inputs']['core']['sha256'],
        actual_output_differences=[dict(output_core_sha256=identity['sha256'],
            differing_bytes=raw_difference(input_path, path), unknown_bytes_masked=False)
            for path, identity in capture['outputs'].items()],
        candidate_comparison=False, gameplay_equivalence=False)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True, raw_rows=report['raw_jsonl_rows'],
                         reads=report['summary']['reads'], writes=report['summary']['writes'],
                         mutation_refusals=len(tests), candidate_comparison=False)))


if __name__ == '__main__':
    main()
