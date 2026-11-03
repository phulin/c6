#!/usr/bin/env python3
"""Strict closed-capture SkipTurn predicate comparison; never executes original code.

The immutable candidate receives only the old audited raw-state projection. Both
native answer streams are opened after that process exits. Historical candidate
rows retain their native_compared:false qualifier; this checker supplies a new,
separate attestation for the original handler predicate only.
"""
import argparse
import collections
import gzip
import json
from pathlib import Path
import subprocess

import placement_count_reconstruction_results as base

ROOT = base.ROOT
OPERATION_HASH = 0x2C681908
# Admission is deliberately closed until the independent runner/reviewer freeze
# an actually terminal-cleaned successor capture. No CLI override bypasses it.
ADMISSION = None


def same(left, right):
    """JSON equality with booleans distinguished from integers."""
    if isinstance(left, set):
        left = sorted(left)
    if isinstance(right, set):
        right = sorted(right)
    return base.canonical(left) == base.canonical(right)


def exact(left, right, reason):
    base.require(same(left, right), reason)


def read_rows(path):
    return [base.parse(line) for line in path.read_text().splitlines()]


def candidate_roster(request, calculated):
    """Validate complete ordered actors, count directions, bindings and handlers."""
    actors = []
    units = []
    for player in request['players']:
        for kind, name in [('unit', 'units'), ('formation', 'formations')]:
            for entry in player[name]['entries']:
                actor = dict(owner=player['owner'], ordinal=entry['ordinal'], id=entry['id'])
                actors.append((kind, actor))
                if kind == 'unit':
                    units.append(actor)
    counts = calculated['count_results']
    base.require(len(counts) == len(actors) * 7, 'candidate count coverage differs')
    current = {}
    for index, (kind, actor) in enumerate(actors):
        for offset, direction in enumerate(range(-1, 6)):
            row = counts[index * 7 + offset]
            for key, value in dict(actor, kind='count_result', member_kind=kind,
                                   direction=direction).items():
                exact(row.get(key), value, 'candidate actor/direction/type/order differs')
            base.require(type(row['plot_present']) is bool, 'nonboolean plot presence')
            base.require(('plot' in row) == row['plot_present'], 'plot discriminant differs')
            base.require(('count' in row) == row['plot_present'], 'count discriminant differs')
            base.require(('bool_argument' in row) == row['plot_present'], 'count bool discriminant differs')
            if row['plot_present']:
                base.require(row['bool_argument'] is False, 'candidate count bool argument differs')
            if direction == -1 and kind == 'unit':
                current[(actor['owner'], actor['id'])] = row['plot_present']
    bindings = calculated['unit_binding_results']
    handlers = calculated['handler_results']
    base.require(len(bindings) == len(handlers) == len(units), 'candidate Unit roster incomplete')
    for actor, binding, handler in zip(units, bindings, handlers):
        for key, value in actor.items():
            exact(binding.get(key), value, 'binding roster/order differs')
            exact(handler.get(key), value, 'handler roster/order differs')
        exact(binding.get('kind'), 'unit_binding_result', 'binding kind differs')
        base.require(type(binding.get('formation_id')) is int, 'missing Formation is unsupported (244), never false')
        exact(handler.get('kind'), 'handler_result', 'handler kind differs')
        base.require(handler.get('native_compared') is False, 'historical candidate qualifier changed')
        base.require('unsupported' not in handler, 'unsupported handler is not a false answer')
        base.require(type(handler.get('result')) is bool, 'missing/nonboolean candidate handler result')
        exact(set(handler), {'kind', 'owner', 'ordinal', 'id', 'result', 'native_compared'},
              'unexpected candidate handler fields')
    return units, current


def compare_counts(calculated, rows, profile):
    exact(rows[0], dict(kind='header', schema='c6-linux-placement-handler-count-expected-v1',
                       profile_sha256=profile, answers_are_inputs=False,
                       native_handler_query_available=True), 'count oracle header differs')
    exact(rows[-1], dict(kind='summary', status=0), 'count oracle not successful')
    counts = calculated['count_results']
    bindings = calculated['unit_binding_results']
    # Producer interleaves each Unit binding immediately before its seven count
    # rows; Formations have seven counts without a binding. Validate whole order.
    binding_map = {(r['owner'], r['id']): r for r in bindings}
    projected = []
    for row in counts:
        if row['member_kind'] == 'unit' and row['direction'] == -1:
            projected.append(dict(binding_map[(row['owner'], row['id'])], kind='unit_binding_expected'))
        projected.append(dict(row, kind='count_expected'))
    base.require(len(rows) == len(projected) + 2, 'count/binding stream coverage differs')
    mismatches = []
    for index, (candidate, native) in enumerate(zip(projected, rows[1:-1])):
        # Identity/order/type violations are rejected, not a value mismatch.
        for key in ('kind', 'owner', 'ordinal', 'id'):
            exact(native.get(key), candidate[key], 'native count/binding identity/order differs')
        if candidate['kind'] == 'count_expected':
            for key in ('member_kind', 'direction', 'plot_present', 'anchor', 'anchor_valid', 'layer'):
                exact(native.get(key), candidate[key], 'native count direction/plot/raw identity differs')
            if candidate['plot_present']:
                for key in ('bool_argument', 'plot'):
                    exact(native.get(key), candidate[key], 'native count bool/plot differs')
        exact(set(native), set(candidate), 'unexpected count/binding fields')
        if not same(native, candidate):
            mismatches.append(dict(index=index, calculated=candidate, native_expected=native))
    return mismatches


def compare_handlers(calculated, units, current, rows, profile, operation_id):
    exact(rows[0], dict(kind='header', schema='c6-linux-skip-turn-handler-expected-v1',
                       profile_sha256=profile, answers_are_inputs=False,
                       full_manager_legality=False), 'handler oracle header differs')
    base.require(len(rows) > 2, 'handler oracle empty')
    base.require(type(operation_id) is int and operation_id >= 0, 'invalid runtime operation ID')
    projected = []
    for repeat in range(2):
        for visible in (False, True):
            projected.append(dict(kind='handler_no_call_control', unit_present=False,
                                  plot_present=False, visible_only=visible, repeat=repeat,
                                  operation_id=operation_id, operation_hash=OPERATION_HASH,
                                  allowed=False, original_leaf_called=False))
        for actor, handler in zip(units, calculated['handler_results']):
            variants = ('current', 'null') if current[(actor['owner'], actor['id'])] else ('null',)
            for variant in variants:
                for visible in (False, True):
                    projected.append(dict(actor, kind='handler_expected', unit_present=True,
                                          plot_variant=variant, plot_present=variant == 'current',
                                          visible_only=visible, repeat=repeat,
                                          operation_id=operation_id, operation_hash=OPERATION_HASH,
                                          allowed=handler['result'], original_leaf_called=True))
    calls = len(projected) - 4
    exact(rows[-1], dict(kind='summary', status=0, units=len(units),
                        original_leaf_calls=calls, no_call_controls=4, repeats=2),
          'handler oracle incomplete/unsupported')
    base.require(len(rows) == len(projected) + 2, 'handler oracle call count differs')
    mismatches = []
    for index, (candidate, native) in enumerate(zip(projected, rows[1:-1])):
        exact(set(native), set(candidate), 'unexpected handler fields')
        for key in candidate:
            if key != 'allowed':
                exact(native[key], candidate[key], 'handler identity/matrix/order/operation differs: ' + key)
        base.require(type(native['allowed']) is bool, 'nonboolean native handler answer')
        if candidate['kind'] == 'handler_no_call_control':
            base.require(native['allowed'] is False, 'NULL Unit no-call control differs')
        elif not same(candidate['allowed'], native['allowed']):
            mismatches.append(dict(index=index, calculated=candidate, native_expected=native))
    answers = collections.Counter(row['allowed'] for row in rows if row.get('kind') == 'handler_expected')
    return mismatches, dict(original_leaf_calls=calls, no_call_controls=4,
                            operation_id=operation_id, operation_hash=OPERATION_HASH,
                            native_true=answers[True], native_false=answers[False],
                            placed_units=sum(current.values()), unplaced_units=len(units) - sum(current.values()))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--capture', type=Path, required=True)
    parser.add_argument('--manifest', type=Path, default=ROOT/'evidence/linux/placement-count-candidate-301f9e9.json')
    parser.add_argument('--diagnostics', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    base.require(ADMISSION is not None, 'no independently admitted terminal-cleaned handler capture yet')
    capture = args.capture.resolve()
    manifest, binary = base.candidate_gate(args.manifest)
    runtime_path, prepared_path = capture/'runtime.json', capture/'prepared.json'
    base.require(base.digest(runtime_path) == ADMISSION['runtime_sha256'], 'unreviewed runtime')
    base.require(base.digest(prepared_path) == ADMISSION['prepared_sha256'], 'unreviewed preparation')
    runtime, prepared = base.load(runtime_path), base.load(prepared_path)
    config = runtime['configuration']
    profile = ADMISSION['profile_sha256']
    base.require(base.sha(base.canonical(config)) == runtime['configuration_sha256'] == prepared['configuration_sha256'] == profile,
                 'runtime/preparation/profile identity differs')
    exact(config, prepared['configuration'], 'prepared configuration changed')
    exact(base.load(capture/'configuration.json'), config, 'durable configuration differs')
    base.require(config['assets']['libGameCore_XP2.so'] == base.GAME and config['assets']['Civ6'] == base.HOST,
                 'original module/host identity differs')
    binding = config['lower_core_placement_handler_metadata']
    base.require(binding['expected_outputs_used_as_inputs'] is False and binding['full_manager_legality'] is False
                 and binding['native_action_execution'] is False, 'handler profile boundary differs')
    base.require(base.digest(capture/'candidate-manifest.json') == binding['reconstruction_manifest_sha256'] == ADMISSION['successor_manifest_sha256'],
                 'successor manifest identity differs')
    exact(base.load(capture/'candidate-manifest.json'), binding['reconstruction_manifest'], 'successor manifest/config binding differs')
    successor = binding['reconstruction_manifest']
    for field, value in manifest.items():
        if field != 'schema':
            exact(successor[field], value, 'successor changed immutable candidate field: ' + field)
    base.require(successor['schema'] == 'c6-linux-placement-handler-candidate-v1', 'successor manifest schema differs')
    base.require(runtime['returncode'] == 0 and runtime['fixture_success'] is True
                 and runtime['framework_boundary_returned'] is True and runtime['framework_cleanup_proven'] is True
                 and runtime['client_cleanup_proven'] is True and runtime['native_boundary_result'] == 0,
                 'native capture not terminal-cleaned successfully')
    base.require('C6_LINUX_ACTION_LIFECYCLE uninitialize_return initialized 0 context_bound 0 executing 0 cleanup 1 dispatch_mode 0 requested_execution 0 core_component_mode 0' in runtime['stderr'].splitlines(),
                 'terminal cleanup footer absent')
    for name, value in config['sources'].items():
        base.require(not Path(name).is_absolute() and '..' not in Path(name).parts, 'source archive path escapes')
        base.require(base.digest(capture/'source'/name) == value, 'archived source differs: ' + name)
    base.require(base.digest(capture/'source/tools/linux/mod_activation_probe.py') == config['runner_sha256'], 'archived runner differs')
    base.require(base.digest(capture/'placement-handler-metadata-preload.so') == config['preload_sha256'], 'admitted preload differs')
    facts_path = capture/runtime['placement_handler_metadata_output']['filename']
    exact(base.file_identity(facts_path), runtime['placement_handler_metadata_output'], 'closed facts identity differs')
    facts = read_rows(facts_path)
    base.require(facts[0]['kind'] == 'header' and facts[-1]['kind'] == 'summary', 'fact framing differs')
    base.require(sum(r['kind'] == 'header' for r in facts) == sum(r['kind'] == 'summary' for r in facts) == 1
                 and all(r['kind'] in {'header', 'unit_definitions', 'relations', 'player', 'members', 'operation_identity', 'summary'} for r in facts), 'fact kinds differ')
    header, summary = facts[0], facts[-1]
    base.require(header['schema'] == 'c6-linux-placement-handler-metadata-native-v1' and header['game_sha256'] == base.GAME
                 and header['profile_sha256'] == profile and header['player_slots'] == 65
                 and header['complete_allocator_state'] is False, 'native fact profile differs')
    base.require(summary['status'] == 0 and summary['known_rng_equal'] is True
                 and summary['rng_before'] == summary['sync_rng_after'] + summary['map_rng_after']
                 and summary['complete_rng_inventory'] is False, 'known RNG/summary differs')
    base.require(len(bytes.fromhex(summary['rng_before'])) == 64 and len(bytes.fromhex(summary['sync_rng_after']))
                 == len(bytes.fromhex(summary['map_rng_after'])) == 32, 'known RNG state length differs')
    identities = [row for row in facts if row['kind'] == 'operation_identity']
    base.require(len(identities) == 1, 'independent registered operation identity missing/duplicated')
    operation_id = identities[0].get('operation_id')
    base.require(type(operation_id) is int and operation_id >= 0, 'invalid registered runtime operation ID')
    exact(identities[0], dict(kind='operation_identity', source='original_registered_handler_getters',
                              operation_id=operation_id, operation_hash=OPERATION_HASH,
                              original_leaf_called=False), 'registered operation raw fact differs')
    controls_runtime = dict(runtime, placement_metadata_saves=runtime['placement_handler_metadata_saves'])
    data, controls = base.closed_controls(capture, controls_runtime)
    trace = gzip.decompress((ROOT.parent/'c6-ghidra/evidence/linux-save-normalization/checkpoint-203-owners/trace-benchmark-1.jsonl.gz').read_bytes())
    base.source_gate(ROOT.parent/'c6-assets-linux/libGameCore_XP2.so')
    alive = base.compare_actual_alive(base.saved_player_alive(data, trace), facts)
    base.require(alive['actual_players_compared'] == 43, 'actual/saved alive roster differs')
    directory = args.diagnostics.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    request, coverage = base.make_request(data, facts, trace, directory, profile)
    input_path = directory/'input.json'
    input_path.write_bytes(base.canonical(request) + b'\n')
    process = subprocess.run([str(binary), str(input_path), profile, profile], capture_output=True, text=True)
    (directory/'candidate.stdout').write_text(process.stdout)
    (directory/'candidate.stderr').write_text(process.stderr)
    base.require(process.returncode == 0, 'candidate rejected genuine inputs; diagnostics: ' + str(directory))
    calculated = base.parse(process.stdout)
    base.require(calculated['schema'] == 'c6-linux-placement-count-results-v1' and calculated['gamecore_sha256'] == base.GAME
                 and calculated['configuration_sha256'] == calculated['profile_sha256'] == profile
                 and calculated['typed_input_state_unchanged'] is True and calculated['native_execution'] is False
                 and calculated['native_handler_comparison'] is False and calculated['complete_state_actions_turns'] is False,
                 'candidate identity/scope differs')
    units, current = candidate_roster(request, calculated)
    base.require(coverage['units'] == 1262 and coverage['formations'] == 1250 and len(calculated['count_results']) == 17584,
                 'full original membership/count coverage differs')
    # Both native answer files are first opened after candidate execution.
    answer_paths = []
    for key in ('placement_handler_count_expected_output', 'placement_handler_expected_output'):
        path = capture/runtime[key]['filename']
        exact(base.file_identity(path), runtime[key], 'closed native answer changed')
        answer_paths.append(path)
    count_rows, handler_rows = map(read_rows, answer_paths)
    count_mismatches = compare_counts(calculated, count_rows, profile)
    handler_mismatches, matrix = compare_handlers(calculated, units, current, handler_rows, profile, operation_id)
    base.require(matrix['placed_units'] == 1241 and matrix['unplaced_units'] == 21 and matrix['original_leaf_calls'] == 10012,
                 'original complete Unit matrix differs')
    report = dict(schema='c6-linux-skip-turn-handler-native-comparison-v1',
                  status='mismatched' if count_mismatches or handler_mismatches else 'equal',
                  gamecore_sha256=base.GAME, host_sha256=base.HOST, profile_sha256=profile,
                  candidate_sha256=base.CANDIDATE, candidate_source_commit=base.COMMIT,
                  historical_candidate_manifest_sha256=base.MANIFEST,
                  successor_manifest_sha256=ADMISSION['successor_manifest_sha256'],
                  runtime_sha256=base.digest(runtime_path), prepared_sha256=base.digest(prepared_path),
                  facts_sha256=base.digest(facts_path), native_count_answers_sha256=base.digest(answer_paths[0]),
                  native_handler_answers_sha256=base.digest(answer_paths[1]),
                  checker_sha256=base.digest(Path(__file__)), projection_checker_sha256=base.digest(Path(base.__file__)),
                  candidate_input_sha256=base.sha(base.canonical(request)), candidate_output_sha256=base.sha(process.stdout.encode()),
                  archived_sources_checked=len(config['sources']), archived_runner_sha256=config['runner_sha256'],
                  admitted_preload_sha256=config['preload_sha256'], coverage=coverage, matrix=matrix,
                  count_and_binding_mismatches=len(count_mismatches), handler_mismatches=len(handler_mismatches),
                  native_count_rows_compared=len(calculated['count_results']), native_binding_rows_compared=len(units),
                  computed_handler_unsupported=0, actual_core_alive_saved_header_crosscheck=alive,
                  core_controls=controls, known_game_and_map_rng_bytes=64, known_rng_equal=True,
                  complete_rng_inventory=False, typed_input_state_unchanged=True,
                  native_handler_comparison=True, candidate_native_compared_fields_changed=False,
                  original_game_execution_by_checker=False, expected_outputs_used_as_inputs=False,
                  full_manager_legality=False, gameplay_action_acceptance=False, complete_state_actions_turns=False,
                  unknown_bytes_masked=0, historical_raw_save_load_difference_preserved=89198,
                  diagnostics_path=str(directory))
    (directory/'mismatches.json').write_text(json.dumps(dict(counts_and_bindings=count_mismatches, handlers=handler_mismatches), indent=2) + '\n')
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({key: report[key] for key in ('status', 'count_and_binding_mismatches', 'handler_mismatches')}))
    if report['status'] != 'equal':
        raise SystemExit(1)


if __name__ == '__main__':
    main()
