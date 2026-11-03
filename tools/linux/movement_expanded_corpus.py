#!/usr/bin/env python3
"""Prepare a bounded full unit-reachable movement scan; audit closed facts offline.

Never executes original code or the Rust candidate. Coverage is not parity.
"""
import argparse
from collections import Counter, defaultdict
import copy
import hashlib
import json
from pathlib import Path

from movement_cost_cache_results import BASE, CANDIDATE, HELPER, require, rng
from movement_cost_results import (digest, known_identity_projection, read_corpus,
                                   runtime_provenance, variations)

ROOT = Path(__file__).resolve().parents[2]
LIMIT = 4096
GAME = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
HOST = '106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
BASELINE = {
    'control': '0a3dd621b5d00faea78a3ddfa5a0f2d17a8b0f8b81ad971bebd9ee049be83320',
    'execute': '020ed0db001c28cead8f3beaf04813a8c26796e86d6b284cbb3dd477c80a4e42',
}


def configuration_hash(config):
    return hashlib.sha256(json.dumps(config, sort_keys=True,
                                     separators=(',', ':')).encode()).hexdigest()


def expected_configurations():
    configs = {}
    for mode, name in [('control', 'control'), ('execute', 'cost')]:
        path = BASE / f'native-cache-{name}-1/runtime.json'
        require(digest(path) == BASELINE[mode], 'closed cached baseline changed')
        runtime = json.loads(path.read_text())
        config = runtime['configuration']
        require(configuration_hash(config) == runtime['configuration_sha256'],
                'baseline canonical configuration differs')
        require(config['assets']['libGameCore_XP2.so'] == GAME and
                config['assets']['Civ6'] == HOST, 'wrong target')
        movement = config['lower_core_movement_cost']
        require(movement['max_formations'] == 256 and movement['mode'] == mode,
                'baseline budget/mode differs')
        require(movement['frozen_artifacts']['native_helper']['sha256'] == HELPER and
                movement['frozen_artifacts']['candidate']['sha256'] == CANDIDATE,
                'baseline frozen artifacts differ')
        configs[mode] = copy.deepcopy(config)
        configs[mode]['lower_core_movement_cost']['max_formations'] = LIMIT
    control = copy.deepcopy(configs['execute'])
    control['lower_core_movement_cost']['mode'] = 'control'
    require(control == configs['control'], 'baseline pair differs beyond mode')
    return configs


def coverage(header, summary, cases, attestation):
    groups = defaultdict(list)
    for case in cases:
        groups[(case['owner'], case['formation_id'])].append(case)
    missing = 0
    direction_histogram = Counter()
    for (owner, _), rows in groups.items():
        require(0 <= owner < 64, 'owner outside original scan bounds')
        directions = [r['direction'] for r in rows]
        require(directions == sorted(set(directions)) and
                all(0 <= d < 6 for d in directions), 'invalid direction order')
        require(all(r['from'] == rows[0]['from'] and
                    r['unit_id'] == rows[0]['unit_id'] and
                    [(m['id'], m['found']) for m in r['members']] ==
                    [(m['id'], m['found']) for m in rows[0]['members']] for r in rows),
                'formation identity/position changed within scan')
        missing += 6 - len(rows)
        direction_histogram[str(len(rows))] += 1
    observed = len(groups)
    require(observed <= summary['formations'] <= LIMIT and header['formation_limit'] == LIMIT,
            'formation accounting/budget differs')
    result = dict(formations_discovered=summary['formations'],
        formations_with_cases=observed,
        discovered_formations_without_cases=summary['formations'] - observed,
        units_seen=summary['units_seen'], cases=len(cases),
        skipped_unclassified=summary['skipped'], truncated=summary['truncated'],
        formations_by_case_direction_count=dict(sorted(direction_histogram.items())),
        absent_directions_for_observed_formations=missing,
        absent_direction_meaning='original adjacent returned null; no synthetic edge added',
        skipped_meaning='native aggregate: missing unit, missing formation, or unplaced source; attribution unavailable',
        owners=sorted({c['owner'] for c in cases}),
        cases_by_owner=dict(sorted(Counter(str(c['owner']) for c in cases).items())),
        cases_by_domain=dict(Counter(str(c['formation_domain']) for c in cases)),
        cases_by_layer=dict(Counter(str(c['formation_layer']) for c in cases)),
        cases_by_member_count=dict(Counter(str(len(c['members'])) for c in cases)),
        escort_member_cases=sum(any(m['found'] and m['escort'] for m in c['members']) for c in cases),
        missing_member_cases=sum(any(not m['found'] for m in c['members']) for c in cases),
        source_coordinate_edge_cases=sum(c['from']['x'] in (0, attestation['map_width'] - 1) or
                                c['from']['y'] in (0, attestation['map_height'] - 1)
                                for c in cases),
        source_open_border_cases=sum(
            (not attestation['wrap_x'] and
             c['from']['x'] in (0, attestation['map_width'] - 1)) or
            (not attestation['wrap_y'] and
             c['from']['y'] in (0, attestation['map_height'] - 1)) for c in cases))
    if header['execute']:
        result['actual_variations'] = variations(cases)
    result['all_discovered_formations_accounted_without_skips'] = (
        not summary['skipped'] and observed == summary['formations'])
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--control', type=Path, help='closed expanded control directory')
    parser.add_argument('--cost', type=Path, help='closed expanded cost directory')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    require(bool(args.control) == bool(args.cost), 'supply both closed profiles or neither')
    configs = expected_configurations()
    report = dict(schema='c6-linux-movement-expanded-corpus-plan-v1',
        tool_sha256=digest(Path(__file__)), game_sha256=GAME, host_sha256=HOST,
        helper_sha256=HELPER, candidate_sha256=CANDIDATE,
        expected_configuration_sha256={mode: configuration_hash(config)
                                       for mode, config in configs.items()},
        configuration_delta={'/lower_core_movement_cost/max_formations':
                             {'before': 256, 'after': LIMIT}},
        runner_arguments=['--movement-cost-max-formations', str(LIMIT)],
        timeout_seconds=configs['control']['timeout_seconds'],
        scan_bounds={'owners': [0, 63], 'formation_budget': LIMIT,
                     'unit_id_vector_hard_bound_per_owner': 1000000,
                     'directions_per_placed_formation': [0, 5]},
        original_code_execution_performed=False, coverage_only=True,
        complete_gameplay_state_actions_turns=False,
        full_rng_inventory=False, native_expanded_acceptance_pending=True)
    if args.control:
        profiles = {}
        contract = json.loads((ROOT / 'evidence/linux/movement-cost-probe-xp2-contract.json').read_text())
        for mode, directory in [('control', args.control), ('execute', args.cost)]:
            path = directory / 'facts.jsonl'
            header, summary, cases, registries = read_corpus(path, mode == 'execute')
            require(header['game_sha256'] == GAME and header['host_sha256'] == HOST,
                    'expanded corpus wrong target')
            runtime, proof = runtime_provenance(directory / 'runtime.json', header,
                                                path, CANDIDATE, contract)
            require(runtime['configuration'] == configs[mode],
                    'expanded configuration differs beyond budget')
            require(not summary['truncated'], 'expanded scan still capped; preserve and report prefix')
            rows = [json.loads(line) for line in path.read_text().splitlines()]
            attestation = [r for r in rows if r['kind'] == 'attestation']
            require(len(attestation) == 1, 'missing map attestation')
            profiles[mode] = (cases, registries)
            report[mode] = dict(provenance=proof, corpus_sha256=digest(path),
                coverage=coverage(header, summary, cases, attestation[0]),
                observed_rng=rng(summary))
        require(known_identity_projection(profiles['control'][0]) ==
                known_identity_projection(profiles['execute'][0]) and
                profiles['control'][1] == profiles['execute'][1],
                'ordered expanded control/cost facts differ')
        report['ordered_control_cost_facts_and_registries_equal'] = True
        report['schema'] = 'c6-linux-movement-expanded-corpus-coverage-v1'
        del report['native_expanded_acceptance_pending']
        report['native_expanded_coverage_completed'] = True
        report['arithmetic_acceptance_assessed_by_this_tool'] = False
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
