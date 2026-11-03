#!/usr/bin/env python3
"""Offline comparison of closed native C against an immutable pre-answer Graph prediction."""
import argparse
import hashlib
import json
from pathlib import Path
import sys
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/linux'))
from save_normalization_trade import Reader, START, END, SIZE

def sha(data):
    return hashlib.sha256(data).hexdigest()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--capture', type=Path, required=True)
    parser.add_argument('--frozen', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    def need(value, message):
        if not value:
            raise ValueError(message)
    manifest_raw = (args.frozen / 'manifest.json').read_bytes()
    need(sha(manifest_raw) == '2957dce152ff0f4c284ead3f04b930c8fd298cd0ef10c20a7c86d16740d3ddf5', 'pre-answer manifest changed')
    manifest = json.loads(manifest_raw)
    need(manifest['native_c_answer_available_at_freeze'] is False, 'prediction not pre-answer')
    need(sha((args.frozen / 'candidate').read_bytes()) == manifest['candidate_sha256'], 'frozen executable changed')
    need(sha((args.frozen / 'source.tar').read_bytes()) == manifest['source_archive_sha256'], 'frozen source archive changed')
    runtime_raw = (args.capture / 'runtime.json').read_bytes()
    need(sha(runtime_raw) == 'f4ddea674a5ddc262a393e17efe049a15a1c145d3bf046a348ce1a411dd05c03', 'closed runtime identity differs')
    runtime = json.loads(runtime_raw)
    need(runtime['configuration_sha256'] == '450387b547ff8b61eaeaf24a2b52d0576b0543698aa9f5891357274296bb598c', 'capture configuration differs')
    need(runtime['configuration']['lower_core_stream']['sha256'] == manifest['inputs'][1]['canonical_core_sha256'], 'native input is not frozen B')
    need(runtime['returncode'] == 0 and runtime['fixture_success'] and runtime['framework_cleanup_proven'] and runtime['client_cleanup_proven'], 'capture not successfully closed')
    actual = (args.capture / 'save.bin').read_bytes()
    need(len(actual) == SIZE and sha(actual) == '27ca0057dc37b1e5ca8b726373d31b10bfe5eb81e493137719c8848a5fd5365c', 'native C identity differs')
    need(actual == (args.capture / 'repeat.bin').read_bytes(), 'native repeated Save differs')
    reader = Reader(actual)
    graph = reader.graph()
    need(reader.offset == END, 'typed owner not consumed exactly')
    owner = actual[START:END]
    prediction = (args.frozen / 'b-predicted-c.graph').read_bytes()
    b = (args.frozen / 'b.graph').read_bytes()
    need(sha(prediction) == '38f3e34a0004bdca3249ec82b1f96cb4f528e78b151a3fa1c2f179921012bff2', 'frozen output changed')
    need(sha(b) == manifest['inputs'][1]['graph_sha256'], 'frozen B Graph changed')
    need(owner == prediction, 'complete native Graph differs from pre-answer prediction')
    report = dict(schema='c6-linux-trade-read-order-native-c-comparison-v1', checks_passed=True,
        pre_answer_review_commit='d91d409', frozen_manifest_sha256=sha(manifest_raw),
        frozen_at_utc=manifest['frozen_at_utc'], candidate_sha256=manifest['candidate_sha256'],
        source_archive_sha256=manifest['source_archive_sha256'], native_runtime_sha256=sha(runtime_raw),
        configuration_sha256=runtime['configuration_sha256'], native_input_core_sha256=manifest['inputs'][1]['canonical_core_sha256'],
        native_c_core_sha256=sha(actual), native_c_core_bytes=len(actual), native_repeat_equal=True,
        graph_start=START, graph_end=END, graph_bytes=len(owner), actual_graph_sha256=sha(owner),
        frozen_prediction_sha256=sha(prediction), complete_graph_equal=True, prediction_difference_bytes=0,
        native_c_vs_b_graph_difference_bytes=sum(x != y for x, y in zip(owner, b)),
        origins=len(graph['origins_in_wire_order']), edges=len(graph['edges_by_vertices']),
        typed_exact_consumption_unique_keys_and_endpoint_invariants=True, sorting_or_masking_used=False,
        new_native_c_serializer_owner_trace_observed=False,
        owner_extent_basis='Existing fixture-specific boundary plus strict typed version/count/endpoint/unique-key validation and exact consumption; native C save_normalization_trace is null.',
        admitted_scope='One pre-answer complete Trade Graph wire-order prediction matched the closed B-to-C Load/Save fixture. No whole Core C prediction or general save-loader/gameplay acceptance.',
        live_got_or_interposition_proven=False, fresh_pre_read_capacities_observed=False,
        native_allocator_archive_id_map_modeled=False, original_execution_by_reviewer=False,
        frozen_candidate_or_prior_evidence_rewritten=False, complete_state_action_rng_turns_supported=False,
        historical_raw_core_difference_bytes=89198, unknown_bytes_masked=False,
        driver_sha256=sha(Path(__file__).read_bytes()))
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(complete_graph_equal=True, graph_bytes=len(owner), differences=0)))

if __name__ == '__main__':
    main()
