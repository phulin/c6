#!/usr/bin/env python3
"""Check closed expanded Linux movement profiles offline; execute only frozen Rust."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from movement_cost_cache_results import (BASE, CANDIDATE, HELPER, PRELOAD,
    RUNNER, RUNNER_REVISION, SAVE_NAMES, config_diff, git_hash, load, pin, require, rng)
from movement_cost_results import (digest, known_identity_projection, phases,
    raw_difference, read_corpus, runtime_provenance)

ACCEPTED = '3d2b5681ba880fb62bcd18ab1edc959d2b4a5dacc8a74235ecd7a348052ec15e'
CANDIDATE_FLAGS = ['equal', 'initial_equal', 'final_accumulator_equal', 'no_zoc_equal',
                   'full_equal', 'formation_aggregate_equal']
PINS = {
    'expanded_control': ('ca3b7f99da6ccc334fc573d17e1bede01bffce3cf46f6f519ecda213756cc263',
        'cebf511f316685e6b0950c77accfef8e19f6548ed0ec2a799f579de2a37c34f6',
        'ddc9476db26f6f9cf94479a289d6588085f62fbf2120e18f13e28deaf7f63d87', 38374486),
    'expanded_cost': ('55cddc3140ad426931c6966deb945c11f557b0d86909b1417387e0ace2776736',
        'b6cc9418ace0264a49cf11183082b5ee18bb2fc190db44f4d505911ddab1298f',
        '5d3a5d2b9572269693484dfc439738c25887b0b4599692cf0a477b9aad791767', 41164256)}


def addresses(value):
    """Project only named process addresses; retain every native cost and phase."""
    if isinstance(value, list):
        return [addresses(v) for v in value]
    if isinstance(value, dict):
        return {k: addresses(v) for k, v in value.items()
                if not ({'address', 'resolved'} <= value.keys() and k in {'address', 'module_base'})}
    return value


def expansion_gate(old, new):
    expected = json.loads(json.dumps(old))
    require(expected['lower_core_movement_cost']['max_formations'] == 256,
            'accepted formation bound changed')
    expected['lower_core_movement_cost']['max_formations'] = 4096
    require(expected == new, 'expanded config changes beyond formation bound')
    return config_diff(old, new)


def validate_candidate(cases, rows, expected_summary):
    require(len(rows) == len(cases) + 1, 'candidate output row count differs')
    for c, r in zip(cases, rows[:-1]):
        expected = {key: c[key] for key in ['owner', 'formation_id', 'unit_id', 'direction']}
        expected.update({key: True for key in CANDIDATE_FLAGS})
        expected.update(rust_full=c['full_final'], rust_no_zoc=c['no_zoc_final'])
        require(r == expected and all(r.get(key) is True for key in CANDIDATE_FLAGS) and
                c['native_paths_agree'] is True, 'candidate ordered result row differs')
    require(rows[-1] == expected_summary, 'candidate summary differs')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for kind in ['cost', 'control']:
        parser.add_argument('--expanded-' + kind, type=Path, default=BASE / ('native-expanded-' + kind + '-1'))
        parser.add_argument('--cache-' + kind, type=Path, default=BASE / ('native-cache-' + kind + '-1'))
    parser.add_argument('--private-candidate-output', type=Path, required=True)
    parser.add_argument('--output', type=Path, default=Path('evidence/linux/movement-cost-expanded-results.json'))
    args = parser.parse_args()
    pin(Path('tools/linux/movement_cost_cache_results.py'),
        '940d96999f78c15b4aa0ce52b68bbe2c6d664f77b6db889c6b89ae20d512635a')
    accepted_path = Path('evidence/linux/movement-cost-cache-results.json')
    pin(accepted_path, ACCEPTED)
    accepted = load(accepted_path)
    contract_path = Path('evidence/linux/movement-cost-probe-xp2-contract.json')
    pin(contract_path, accepted['contract_sha256'])
    contract = load(contract_path)
    original_path = Path('evidence/linux/movement-cost-results.json')
    pin(original_path, accepted['accepted_baseline_sha256'])
    original = load(original_path)
    pin(Path('tools/linux/movement_cost_results.py'), original['offline_checker_sha256'])
    review_path = Path('evidence/linux/oracle-admission-cache-review.json')
    pin(review_path, accepted['review_sha256'])
    review = load(review_path)
    require(review['source_lifetime_review_passed'], 'cache lifetime review failed')
    for path, expected in accepted['reviewed_sources'].items():
        require(git_hash(accepted['reviewed_source_revision'], path) == expected, 'reviewed source differs: ' + path)
    require(git_hash(RUNNER_REVISION, 'tools/linux/mod_activation_probe.py') == RUNNER, 'runner revision differs')
    plan_review_path = Path('evidence/linux/movement-expanded-corpus-review.json')
    plan_path = Path('evidence/linux/movement-expanded-corpus-plan.json')
    plan_pins = {}
    for path in [plan_review_path, plan_path]:
        expected = git_hash('3ad397d725732f70fee174e04bf15de8299d78ff', str(path))
        pin(path, expected)
        plan_pins[str(path)] = expected
    frozen = Path('target/frozen/movement-cost-probe')
    candidate = frozen / CANDIDATE / 'candidate'
    manifest_path = frozen / CANDIDATE / 'source-manifest.json'
    pin(candidate, CANDIDATE)
    pin(frozen / HELPER / 'native_helper', HELPER)
    pin(manifest_path, accepted['source_manifest_sha256'])
    manifest = load(manifest_path)
    require(manifest['candidate_sha256'] == CANDIDATE and
            manifest['source_reference_commit'] == original['candidate_source_revision'], 'manifest revision differs')
    for path, expected in manifest['sources_at_reference_commit'].items():
        require(git_hash(manifest['source_reference_commit'], path) == expected, 'candidate source differs: ' + path)
    profiles = {}
    for name in ['cache_control', 'cache_cost', 'expanded_control', 'expanded_cost']:
        directory = getattr(args, name)
        corpus = directory / 'facts.jsonl'
        expanded = name.startswith('expanded')
        if expanded:
            profile_hash, runtime_hash, corpus_hash, corpus_bytes = PINS[name]
            require(corpus.stat().st_size == corpus_bytes, 'closed expanded corpus length differs')
        else:
            profile_hash = accepted['runtime_provenance'][name]['configuration_sha256']
            runtime_hash = accepted['runtime_provenance'][name]['runtime_sha256']
            corpus_hash = accepted['corpus_sha256'][name]
        pin(corpus, corpus_hash)
        pin(directory / 'runtime.json', runtime_hash)
        header, summary, cases, registries = read_corpus(corpus, name.endswith('cost'))
        require(header['profile_sha256'] == profile_hash and
                header['game_sha256'] == contract['game_sha256'] and
                header['host_sha256'] == contract['host_sha256'], 'target/profile mismatch')
        require(len(cases) == (7461 if expanded else 1536) and
                summary['formations'] == (1250 if expanded else 256) and
                summary['units_seen'] == (1262 if expanded else 258) and
                summary['skipped'] == 0 and summary['truncated'] is (not expanded), 'incomplete closed profile')
        runtime, proof = runtime_provenance(directory / 'runtime.json', header, corpus, CANDIDATE, contract)
        config = runtime['configuration']
        require(load(directory / 'configuration.json') == config, 'configuration archive differs')
        require(config['preload_sha256'] == PRELOAD and config['runner_sha256'] == RUNNER and
                proof['archived_sources_verified'] == 60, 'preload/runner/source count differs')
        movement = config['lower_core_movement_cost']
        require(movement['integrated_preload_sha256'] == PRELOAD and
                movement['frozen_artifacts']['native_helper']['sha256'] == HELPER and
                movement['candidate_source_manifest']['sha256'] == digest(manifest_path), 'frozen identities differ')
        pin(directory / 'candidate-source-manifest.json', digest(manifest_path))
        profiles[name] = dict(directory=directory, header=header, summary=summary, cases=cases,
                              registries=registries, runtime=runtime, proof=proof)
    changes = {kind: expansion_gate(profiles['cache_' + kind]['runtime']['configuration'],
                                   profiles['expanded_' + kind]['runtime']['configuration']) for kind in ['control', 'cost']}
    cost_config = profiles['expanded_cost']['runtime']['configuration']
    control_config = profiles['expanded_control']['runtime']['configuration']
    expected_control = json.loads(json.dumps(cost_config))
    expected_control['lower_core_movement_cost']['mode'] = 'control'
    require(expected_control == control_config, 'fresh profiles differ beyond mode')
    comparisons = {}
    for first, second, prefix, facts_only in [
            ('expanded_control', 'expanded_cost', False, True),
            ('cache_control', 'expanded_control', True, True),
            ('cache_cost', 'expanded_cost', True, False)]:
        a, b = profiles[first], profiles[second]
        aa, bb = a['cases'], b['cases'][:1536] if prefix else b['cases']
        project = known_identity_projection if facts_only else addresses
        require(project(aa) == project(bb) and a['registries'] == b['registries'], 'ordered comparison failed: ' + first)
        comparisons[first + '_to_' + second] = dict(cases=len(aa), ordered_cases_equal=True,
            registries_equal=True, facts_only=facts_only, prefix_only=prefix,
            cost_and_phase_fields_retained=not facts_only)
    cases = profiles['expanded_cost']['cases']
    require(all(len(c['phases']) == 9 and phases(c) == c['phases'] for c in cases), 'nine-phase replay failed')
    raw = {}
    for name, p in profiles.items():
        metadata = list(p['runtime']['movement_cost_saves'].values()) + list(p['runtime']['game_saves'].values())
        require(len(metadata) == 6 and {v['filename'] for v in metadata} == set(SAVE_NAMES), 'six saves missing')
        for value in metadata:
            path = p['directory'] / value['filename']
            pin(path, value['sha256'])
            require(path.stat().st_size == value['bytes'] == 80657462, 'save size differs')
        raw[name] = {filename: raw_difference(args.cache_cost / 'before.bin', p['directory'] / filename) for filename in SAVE_NAMES}
        require(all(v['different_bytes'] == 0 for v in raw[name].values()), 'raw snapshot changed')
    rng_results = {name: rng(p['summary']) for name, p in profiles.items()}
    owned_rng = [{k: r[k] for k in ['owned_game', 'owned_map']} for r in rng_results.values()]
    require(all(r == owned_rng[0] for r in owned_rng), 'cross-profile known RNG fields differ')
    private_output = args.private_candidate_output.resolve()
    require(private_output == (args.expanded_cost / 'expanded-candidate-comparison.jsonl').resolve(), 'candidate output must remain in closed private profile')
    run = subprocess.run([str(candidate.resolve()), str((args.expanded_cost / 'facts.jsonl').resolve())], capture_output=True, text=True, timeout=120)
    private_output.write_text(run.stdout)
    require(run.returncode == 0, 'frozen Rust failed: ' + run.stderr.strip())
    rows = [json.loads(line) for line in run.stdout.splitlines()]
    expected_summary = dict(accepted['frozen_rust_candidate'])
    expected_summary.update(compared=7461, input_sha256=digest(args.expanded_cost / 'facts.jsonl'),
                            profile_sha256=profiles['expanded_cost']['header']['profile_sha256'])
    validate_candidate(cases, rows, expected_summary)
    report = dict(schema='c6-linux-movement-cost-expanded-results-v1', offline_checker_sha256=digest(Path(__file__)),
        accepted_cache_report_sha256=ACCEPTED, accepted_cache_checker_sha256=digest(Path('tools/linux/movement_cost_cache_results.py')),
        expansion_review_pins=plan_pins, reviewed_source_revision=accepted['reviewed_source_revision'],
        reviewed_sources=accepted['reviewed_sources'], runner_revision=RUNNER_REVISION,
        candidate_sha256=CANDIDATE, helper_sha256=HELPER, source_manifest_sha256=digest(manifest_path),
        candidate_source_revision=manifest['source_reference_commit'], contract_sha256=digest(contract_path),
        exact_configuration_changes=changes, expanded_control_configuration_changes=config_diff(cost_config, control_config),
        runtime_provenance={name: p['proof'] for name, p in profiles.items()},
        corpus_sha256={name: digest(p['directory'] / 'facts.jsonl') for name, p in profiles.items()},
        ordered_comparisons=comparisons, compared_cases=7461, formations=1250, units_seen=1262, skipped=0, scan_truncated=False,
        compared_phase_states=67149, independent_phase_mismatches=0, frozen_rust_candidate=rows[-1],
        candidate_exit=run.returncode, candidate_result_rows_validated=7461, candidate_output_sha256=digest(private_output),
        candidate_equality_flags_per_row=CANDIDATE_FLAGS, raw_snapshot_pairs=raw, raw_snapshot_pair_count=24,
        raw_rng_endpoints=rng_results, known_owned_rng_fields_equal_across_profiles=True,
        rng_unclassified_bytes='vptr and padding retained in raw within-process comparisons; no classification or masking',
        all_genuine_sentinels_retained=True, null_neighbor_edges_not_fabricated=True,
        original_code_executed_by_checker=False, performance_measured=False, complete_affected_state=False,
        complete_rng_inventory=False, full_movement_parity=False,
        scope='Closed expanded unit-reachable query arithmetic and retained-state observations only; no legality, actions, full state, full RNG, turns or native query speed acceptance. Coverage classification is separate.',
        save_load_89198_byte_failure='Separate unresolved raw Save-to-Load failure; no unknown bytes masked or excluded.')
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(output=str(args.output), cases=7461, phases=67149, candidate_rows=7461, raw_pairs=24)))


if __name__ == '__main__':
    main()
