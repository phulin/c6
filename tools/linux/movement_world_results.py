#!/usr/bin/env python3
"""Reproduce control3 typed movement-fact comparison using immutable offline code.

Reads original artifacts; never builds a candidate, executes game code, masks
save bytes, reconstructs runtime domain masks, or advances gameplay.
"""
import argparse
import collections
import hashlib
import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CANDIDATE = '63d15a41b4f87a6472f3a456479266ac078829a1709796dc15c1c11d466cd909'
CHECKER = 'e8251cde6f119704b31c3ae17a0ff83d1ba5752f4704776837193def17d946ed'
SOURCE = '0db5d51a3af1edab7c30afa138225ccf82da0f34'
NATIVE = 'b6f4243ee2fde1ae020e55cf5e5e3e853c87b7c268dcd5fada88393254bef276'
PROFILE = 'dbe92853390379d3c268b6baf2e75d8feb02b7733e5383c846466cb3af325641'
CORE = 'ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884'
TRACE = '528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'
CONTROL = 'c175591afd4339019007382609ed5ad5bd12ba72ab6c87de891c7635fa3710b6'


def require(test, reason):
    if not test:
        raise ValueError(reason)


def digest(path):
    result = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            result.update(chunk)
    return result.hexdigest()


def coverage(cases):
    plots = [p for c in cases for p in [c['from'], c['to']]]
    coordinate = lambda p: (p['x'], p['y'])
    return dict(
        formations=len({(c['owner'], c['formation_id']) for c in cases}),
        owners=sorted({c['owner'] for c in cases}),
        member_observations=sum(len(c['members']) for c in cases),
        unique_resolved_members=len({(c['owner'], m['actual_unit_id']) for c in cases
                                     for m in c['members'] if m['found']}),
        absent_member_observations=sum(not m['found'] for c in cases for m in c['members']),
        stale_unit_requests=sum(m['id'] != m['actual_unit_id'] for c in cases
                                for m in c['members'] if m['found']),
        unique_plots=len({coordinate(p) for p in plots}),
        city_attribute_endpoints=sum(p['has_city_attributes'] for p in plots),
        unique_city_attribute_plots=len({coordinate(p) for p in plots if p['has_city_attributes']}),
        domain_mask_required_endpoints=sum(p['is_water'] and p['has_city_attributes'] for p in plots),
        unique_domain_mask_required_plots=len({coordinate(p) for p in plots
                                               if p['is_water'] and p['has_city_attributes']}),
        resolved_district_endpoints=sum(p['district_resolved'] for p in plots),
        unique_resolved_districts=len({(p['district_owner'], p['actual_district_id'])
                                      for p in plots if p['district_resolved']}),
        stale_district_requests=sum(p['requested_district_id'] != p['actual_district_id']
                                    for p in plots if p['district_resolved']),
        adjacent=sum(c['edge']['adjacent'] for c in cases),
        river=sum(c['edge']['river_crossing'] for c in cases),
        bridge_between=sum(c['edge']['bridge_between'] for c in cases),
        route_pairs=sum(c['from']['route'] is not None and c['to']['route'] is not None for c in cases),
        city_destination=sum(c['to']['is_city'] for c in cases),
        hill_destination=sum(c['to']['is_hills'] for c in cases),
        feature_destination=sum(c['to']['definitions'][1]['present'] for c in cases),
        water_destination=sum(c['to']['is_water'] for c in cases),
        water_transition=sum(c['to']['is_water'] != c['from']['is_water'] for c in cases),
        can_embark=sum(c['formation']['can_embark'] for c in cases),
        multi_member=sum(len(c['members']) > 1 for c in cases),
        ignore_terrain=sum(c['formation']['ignore_terrain'] for c in cases),
        ignore_feature=sum(c['formation']['ignore_feature'] for c in cases),
        ignore_rivers=sum(c['formation']['ignore_rivers'] for c in cases),
        ignore_shores=sum(c['formation']['ignore_shores'] for c in cases),
        escort_mobility=sum(any(m.get('escort') for m in c['members']) for c in cases))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binding', type=Path,
                        default=ROOT / 'evidence/linux/movement-world-control-3-binding.json')
    parser.add_argument('--adapter-report', type=Path,
                        default=ROOT / 'evidence/linux/movement-world-adapter.json')
    parser.add_argument('--diagnostics', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    binding = json.loads(args.binding.read_text())
    freeze = json.loads(args.adapter_report.read_text())
    require(freeze['source_commit'] == SOURCE and freeze['candidate_sha256'] == CANDIDATE,
            'unsupported candidate/source checkpoint')
    candidate = Path(freeze['frozen_candidate_path'])
    checker = Path(freeze['frozen_source_snapshot_path']) / 'tools/linux/movement_world_check.py'
    require(digest(candidate) == CANDIDATE and digest(checker) == CHECKER,
            'immutable candidate/checker changed')
    for name, expected in freeze['sources'].items():
        require(digest(Path(freeze['frozen_source_snapshot_path']) / name) == expected,
                f'frozen source/input changed: {name}')
    base = args.binding.parent
    runtime_path = base / binding['native_runtime_report']['path']
    require(digest(runtime_path) == CONTROL == binding['native_runtime_report']['sha256'],
            'closed native runtime report identity mismatch')
    runtime = json.loads(runtime_path.read_text())
    profile = hashlib.sha256(json.dumps(runtime['configuration'], sort_keys=True,
                                        separators=(',', ':')).encode()).hexdigest()
    require(profile == PROFILE == runtime['configuration_sha256'] == binding['profile_sha256'],
            'actual native configuration identity mismatch')
    require(runtime['returncode'] == 0 and runtime['fixture_success']
            and runtime['framework_cleanup_proven'] and runtime['client_cleanup_proven'],
            'native control did not close successfully')
    require(binding['native']['sha256'] == NATIVE and binding['core']['sha256'] == CORE
            and binding['trace']['sha256'] == TRACE, 'unsupported source artifact identities')
    for record in binding['core_controls']:
        path = base / record['path']
        require(path.stat().st_size == 80657462 and digest(path) == CORE == record['sha256'],
                'raw original Core controls differ; retain failure without masking')
    native_path = base / binding['native']['path']
    require(digest(native_path) == NATIVE, 'native fact bytes changed')
    records = [json.loads(line) for line in native_path.read_text().splitlines()]
    header, native_summary = records[0], records[-1]
    require(header['execute'] is False and header['probe_contract_revision'] == 'xp2-map-and-terrain-water-2',
            'this report is bound to corrected fact-only control3')
    require(native_summary['known_rng_equal'] is True
            and native_summary['rng_before'] == native_summary['rng_after'] + native_summary['map_rng_after'],
            'known original RNG stream controls differ')
    spec = importlib.util.spec_from_file_location('immutable_movement_world_checker', checker)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    result = module.compare(args.binding, candidate)
    diagnostic_bytes = (json.dumps(result, indent=2) + '\n').encode()
    if args.diagnostics is None:
        args.diagnostics = ROOT / 'target/linux/movement-world-control3/comparisons' / (digest(args.binding) + '.json')
    args.diagnostics.parent.mkdir(parents=True, exist_ok=True)
    if args.diagnostics.exists():
        require(args.diagnostics.read_bytes() == diagnostic_bytes,
                'existing original comparison diagnostics differ; preserve both')
    else:
        args.diagnostics.write_bytes(diagnostic_bytes)
    cases = [r for r in records if r['kind'] == 'case']
    groups = {}
    for key in ['typed_member_and_formation_equal', 'definition_identities_equal', 'plot_and_edge_equal']:
        values = collections.Counter(row[key] for row in result['comparisons'])
        groups[key] = dict(equal=values[True], mismatches=values[False], unavailable=values[None])
    first = [r for r in result['comparisons'] if r['equal'] is not True or r['unsupported'] is not None][:5]
    report = dict(schema='c6-linux-movement-world-control3-v1',
        game_sha256=header['game_sha256'], host_sha256=header['host_sha256'],
        profile_sha256=PROFILE, native_corpus_sha256=NATIVE, core_sha256=CORE,
        native_runtime_report_sha256=CONTROL, owner_trace_sha256=TRACE,
        binding_sha256=digest(args.binding), adapter_report_sha256=digest(args.adapter_report),
        comparison_tool_sha256=digest(Path(__file__)), candidate_source_commit=SOURCE,
        frozen_candidate_sha256=CANDIDATE, frozen_checker_sha256=CHECKER,
        original_code_executed_by_this_check=False, native_fact_control_executed_by_separate_runner=True,
        native_cost_calls=False, candidate_exit_status=result['candidate_exit_status'],
        summary=result['summary'], groups=groups, first_problem_rows=first,
        original_owner_ranges=dict(map=1, units=1262, formations=1250),
        registry_counts={r['name']: r['count'] for r in records if r['kind'] == 'registry'},
        sampled_coverage=coverage(cases), native_discovery_truncated=native_summary['truncated'],
        actual_runtime_side_inputs=['targeted original district lookup/type/completion observations',
                                   'actual signed i16 CityAttributes validDomains masks'],
        domain_valid_comparison_scope='water endpoints with city attributes; unreachable land/absent-attribute placeholder values excluded from predicate comparison, not save-byte masking',
        world_from_wire_scope=['Map4/Plot6 current supported schema', 'Unit28 ignore arrays and serialized MovementModifiers',
                               'Formation1 membership/owner/domain/location/CanEmbark/remaining',
                               'native-order observed definition hashes and properties',
                               'wrapped adjacency/direction and guarded river/bridge facts'],
        unresolved=['runtime district and domain-mask reconstruction not independently imported here',
                    'no nonadjacent/nonempty portal fixtures', 'no native scalar rule/cost calls in fact-only control',
                    'incomplete branch/formation discovery and known RNG inventory only',
                    'movement legality/actions/complete state and turns unimplemented'],
        local_comparison_diagnostics=str(args.diagnostics),
        comparison_diagnostics_sha256=hashlib.sha256(diagnostic_bytes).hexdigest(),
        raw_core_controls_equal=True, raw_core_control_streams=len(binding['core_controls']),
        known_rng_streams_equal=True, known_rng_streams=2,
        complete_affected_state=False, complete_rng_inventory=False,
        native_differential_acceptance=False, full_movement_parity=False,
        raw_save_load_different_bytes_retained=89198, unknown_bytes_masked=0)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(cases=len(cases), groups=groups, first_problem_rows=first,
                          candidate_exit_status=result['candidate_exit_status'], native_acceptance=False)))
    raise SystemExit(result['candidate_exit_status'])


if __name__ == '__main__':
    main()
