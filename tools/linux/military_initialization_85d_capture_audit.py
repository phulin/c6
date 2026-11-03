#!/usr/bin/env python3
"""Offline classification of the third, separately closed Military capture.

The observed 85d input gets its own native Read/Initialize witnesses. Its actual
outputs remain Write-only facts. Does not execute native code, load a candidate,
read prediction artifacts, or alter either earlier capture packet.
"""
import argparse
import json
from pathlib import Path

from military_initialization_capture_audit import (
    audit_artifacts, build_report, file_identity, mutation_tests, require,
)
from military_initialization_ff31_capture_audit import raw_difference


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    capture = audit_artifacts(args.directory)
    gate = capture['config']['captured_core_stream_gate']
    require(gate['schema'] == 'c6-linux-game-load-closed-output-v1'
            and gate['profile'] == 'captured-second', 'separate closed-output input gate scope')
    require(gate['sha256'] == capture['inputs']['core']['sha256']
            and gate['bytes'] == capture['inputs']['core']['bytes'], 'actual input/gate identity')
    frozen_gate = args.directory/'source/tools/linux/game_load_second_capture_gate.json'
    require(json.loads(frozen_gate.read_text()) == gate, 'closed-output selector source preimage')
    root = Path(__file__).resolve().parents[2]
    origin_runtime_path = root/gate['origin_runtime']
    origin_runtime = file_identity(origin_runtime_path)
    origin_summary = file_identity(root/gate['origin_summary'])
    require(origin_runtime['sha256'] == gate['origin_runtime_sha256']
            and origin_summary['sha256'] == gate['origin_summary_sha256'], 'closed origin runtime/summary identities')
    require(json.loads(origin_runtime_path.read_text())['configuration_sha256']
            == gate['origin_configuration_sha256'], 'closed origin configuration identity')
    preload = file_identity(capture['runtime']['execution_paths']['preload'])
    require(preload['sha256'] == capture['config']['preload_sha256'], 'compiled preload identity')
    tests = mutation_tests(capture)
    report = build_report(capture, tests)
    old_limit = 'no incoming ff31 Read or initialized ff31 context captured'
    require(old_limit in report['limits'], 'first-capture validator scope changed')
    report['limits'].remove(old_limit)
    report['limits'].insert(1, 'fresh initialized input Read context only; no initialized Read of this capture\'s own output Core')
    report['classification_driver_sha256'] = file_identity(__file__)['sha256']
    report['raw_difference_helper_sha256'] = file_identity(Path(__file__).with_name('military_initialization_ff31_capture_audit.py'))['sha256']
    report['compiled_preload'] = preload
    report['captured_input_origin_gate'] = dict(
        source_preimage=file_identity(frozen_gate), origin_runtime=origin_runtime,
        origin_summary=origin_summary, origin_configuration_sha256=gate['origin_configuration_sha256'],
        actual_input_core_sha256=capture['inputs']['core']['sha256'],
        own_output_core_sha256=sorted({item['sha256'] for item in capture['outputs'].values()}),
        origin_reference_does_not_supply_initialization=True,
        initialization_source='fresh matched original Read/Initialize call witnesses in this capture',
        earlier_capture_packets_unchanged=True, prediction_artifacts_read=False)
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
