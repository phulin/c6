#!/usr/bin/env python3
"""Offline comparison for the reviewed movement admission-cache revision only.

Never loads original code. Executes only the immutable, SHA-gated Rust candidate.
The older supervisor-only acceptance checker and its report remain immutable.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import subprocess
import struct

from movement_cost_results import (digest, known_identity_projection, phases,
                                   raw_difference, read_corpus, runtime_provenance,
                                   variations)

BASE = Path('/home/phulin/c6-ghidra/evidence/linux-movement-cost')
CANDIDATE = 'd4dddd41f8f157a951c5b43720e6a217fef1b6600439b307ee01e300c7ce6718'
HELPER = 'd13acf6c458f1601bd1d16c76d04a2c667d8db4cebbc5ce0190dbd355a231adc'
RUNNER_REVISION = 'c6634f773a82e572cf592d91f81c459870759d12'
RUNNER = '9aa5e146de8451307e7ce2a9f1db23dd3d8d525a09de0452ef6622a512102835'
PRELOAD = 'e4f637840a933dfd81ce59636092f15af988d10963d858eab3028696df82a53a'
SAVE_NAMES = ['before.bin', 'control.bin', 'after.bin', 'after-repeat.bin', 'save.bin', 'repeat.bin']


def require(condition, message):
    if not condition:
        raise ValueError(message)


def load(path):
    return json.loads(path.read_text())


def pin(path, expected):
    require(digest(path) == expected, 'immutable artifact differs: ' + str(path))


def git_hash(revision, path):
    content = subprocess.check_output(['git', 'show', f'{revision}:{path}'],
                                      cwd=Path(__file__).resolve().parents[2])
    return hashlib.sha256(content).hexdigest()


def config_diff(first, second, prefix=''):
    """Retain every changed leaf and both values, including added dictionaries."""
    if isinstance(first, dict) and isinstance(second, dict):
        result = []
        for key in sorted(first.keys() | second.keys()):
            path = prefix + '/' + key
            if key not in first or key not in second:
                result.append(dict(path=path, before=first.get(key), after=second.get(key)))
            else:
                result.extend(config_diff(first[key], second[key], path))
        return result
    return [] if first == second else [dict(path=prefix, before=first, after=second)]


def cache_gate(old, new, review):
    """Construct the entire expected config; unrelated changes cannot disappear."""
    expected = json.loads(json.dumps(old))
    expected['runner_sha256'] = RUNNER
    expected['preload_sha256'] = PRELOAD
    expected['lower_core_movement_cost']['integrated_preload_sha256'] = PRELOAD
    expected['lower_core_movement_cost']['admission_binding_cache'] = {
        'revision': 'movement-scope-immutable-bindings-v1',
        'audit': {'baseline_commit': review['baseline_commit'], 'bindings': 12,
                  'original_mutable_guard_and_call_order_unchanged': True,
                  'other_game_load_behavior_unchanged': True,
                  'invalidation_before_uninitialize': True,
                  'native_execution_performed': False},
        'native_acceptance_pending': True}
    expected['sources'].update(review['sources'])
    require(expected == new, 'cache configuration differs beyond exact reviewed revision')
    return config_diff(old, new)


def rng(summary):
    before = bytes.fromhex(summary['rng_before'])
    after = bytes.fromhex(summary['rng_after'])
    map_after = bytes.fromhex(summary['map_rng_after'])
    require(len(before) == 64 and len(after) == len(map_after) == 32,
            'missing raw two-object Random32 endpoints')
    require(before[:32] == after and before[32:] == map_after and summary['known_rng_equal'],
            'raw observed Random32 endpoints changed')
    def owned(value):
        seed, calls, resets, locks = struct.unpack_from('<4I', value, 8)
        return dict(seed=seed, calls=calls, resets=resets, locks=locks, sync=value[24])
    return dict(owned_game=owned(after), owned_map=owned(map_after),
                game_before_sha256=hashlib.sha256(before[:32]).hexdigest(),
                game_after_sha256=hashlib.sha256(after).hexdigest(),
                map_before_sha256=hashlib.sha256(before[32:]).hexdigest(),
                map_after_sha256=hashlib.sha256(map_after).hexdigest(),
                equal=True, compared_bytes_per_object=32,
                cross_process_vptrs_not_compared=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cache-cost', type=Path, default=BASE / 'native-cache-cost-1')
    parser.add_argument('--cache-control', type=Path, default=BASE / 'native-cache-control-1')
    parser.add_argument('--baseline-cost', type=Path, default=BASE / 'native-cost-xp2-2')
    parser.add_argument('--baseline-control', type=Path, default=BASE / 'native-control-xp2-3')
    parser.add_argument('--private-candidate-output', type=Path, required=True)
    parser.add_argument('--output', type=Path, default=Path('evidence/linux/movement-cost-cache-results.json'))
    args = parser.parse_args()
    accepted_path = Path('evidence/linux/movement-cost-results.json')
    pin(accepted_path, 'e7db3c366a762e30dc761e8c4507f127505745d34a0f6910f1489c4d415d8458')
    accepted = load(accepted_path)
    contract_path = Path('evidence/linux/movement-cost-probe-xp2-contract.json')
    pin(contract_path, '03c12bdf33d23e8a6cb12d79c50cb5d3363951e2180a06e4838220483e80a0fb')
    contract = load(contract_path)
    review_path = Path('evidence/linux/oracle-admission-cache-review.json')
    pin(review_path, '821b3bdbfe41c97586044d778f9b1303ecd3ef3bcf97d5896b3c8e0d64e11b23')
    review = load(review_path)
    require(review['source_lifetime_review_passed'], 'cache review failed')
    for path, expected in review['sources'].items():
        require(git_hash(review['reviewed_commit'], path) == expected,
                'reviewed source revision mismatch: ' + path)
    require(git_hash(RUNNER_REVISION, 'tools/linux/mod_activation_probe.py') == RUNNER,
            'reviewed runner revision mismatch')
    frozen = Path('target/frozen/movement-cost-probe')
    candidate = frozen / CANDIDATE / 'candidate'
    manifest_path = frozen / CANDIDATE / 'source-manifest.json'
    pin(candidate, CANDIDATE)
    pin(frozen / HELPER / 'native_helper', HELPER)
    pin(manifest_path, accepted['source_manifest_sha256'])
    manifest = load(manifest_path)
    require(manifest['candidate_sha256'] == CANDIDATE and
            manifest['source_reference_commit'] == accepted['candidate_source_revision'],
            'candidate manifest revision mismatch')
    for path, expected in manifest['sources_at_reference_commit'].items():
        require(git_hash(manifest['source_reference_commit'], path) == expected,
                'candidate source mismatch: ' + path)
    profiles = {}
    for name, directory, execute in [
            ('baseline_cost', args.baseline_cost, True),
            ('baseline_control', args.baseline_control, False),
            ('cache_cost', args.cache_cost, True),
            ('cache_control', args.cache_control, False)]:
        corpus = directory / 'facts.jsonl'
        header, summary, cases, registries = read_corpus(corpus, execute)
        require(header['game_sha256'] == contract['game_sha256'] and
                header['host_sha256'] == contract['host_sha256'], 'wrong Linux target')
        require(len(cases) == 1536 and summary['formations'] == 256 and summary['truncated'],
                'changed or incomplete full fixture')
        runtime, proof = runtime_provenance(directory / 'runtime.json', header, corpus,
                                            CANDIDATE, contract)
        require(load(directory / 'configuration.json') == runtime['configuration'],
                'standalone configuration archive differs')
        movement = runtime['configuration']['lower_core_movement_cost']
        require(movement['frozen_artifacts']['native_helper']['sha256'] == HELPER and
                movement['candidate_source_manifest']['sha256'] == digest(manifest_path),
                'profile changed frozen helper/manifest')
        profiles[name] = dict(directory=directory, header=header, summary=summary,
                              cases=cases, registries=registries, runtime=runtime, proof=proof)
    for name, expected in [('baseline_cost', accepted['native_corpus_sha256']),
                           ('baseline_control', accepted['control_corpus_sha256'])]:
        pin(profiles[name]['directory'] / 'facts.jsonl', expected)
        pin(profiles[name]['directory'] / 'runtime.json',
            accepted['runtime_provenance']['cost' if name == 'baseline_cost' else 'control']['runtime_sha256'])
    for name, profile_hash, runtime_hash in [
            ('cache_cost', 'c5c3c7db38fbf52209ee2ee1ac5f5acd41a7f39138f55f0389bfaf74223436bf',
             '020ed0db001c28cead8f3beaf04813a8c26796e86d6b284cbb3dd477c80a4e42'),
            ('cache_control', '99b39eafc7bb8776b3a589f5f45aa302115a8c12778dda29792525b47cc18fe2',
             '0a3dd621b5d00faea78a3ddfa5a0f2d17a8b0f8b81ad971bebd9ee049be83320')]:
        require(profiles[name]['runtime']['configuration_sha256'] == profile_hash, 'closed cache profile differs')
        pin(profiles[name]['directory'] / 'runtime.json', runtime_hash)
    pin(args.cache_cost / 'facts.jsonl', '73cfc3e3cd24edc91ef4613ee869cd6dee1f0236a7b6a80cd4597522f714c7b2')
    old_config = profiles['baseline_cost']['runtime']['configuration']
    cost_config = profiles['cache_cost']['runtime']['configuration']
    control_config = profiles['cache_control']['runtime']['configuration']
    changes = cache_gate(old_config, cost_config, review)
    expected_control = json.loads(json.dumps(cost_config))
    expected_control['lower_core_movement_cost']['mode'] = 'control'
    require(expected_control == control_config, 'cache control/cost differ beyond exact mode')
    comparisons = {}
    for first, second, facts_only in [('baseline_cost', 'cache_cost', False),
                                     ('baseline_control', 'cache_control', True),
                                     ('cache_control', 'cache_cost', True)]:
        a, b = profiles[first], profiles[second]
        aa, bb = a['cases'], b['cases']
        if facts_only:
            aa, bb = known_identity_projection(aa), known_identity_projection(bb)
        else:
            # Preserve every scalar output; project only named process address metadata.
            def addresses(value):
                if isinstance(value, list):
                    return [addresses(v) for v in value]
                if isinstance(value, dict):
                    return {k: addresses(v) for k, v in value.items()
                            if not ({'address', 'resolved'} <= value.keys() and k in {'address', 'module_base'})}
                return value
            aa, bb = addresses(aa), addresses(bb)
        require(aa == bb and a['registries'] == b['registries'],
                'ordered cases/registries differ: ' + first + '/' + second)
        comparisons[first + '_to_' + second] = dict(ordered_cases_equal=True,
            facts_only=facts_only, registries_equal=True, cases=len(aa))
    cases = profiles['cache_cost']['cases']
    phase_mismatches = sum(a != b for c in cases for a, b in zip(phases(c), c['phases']))
    require(all(len(c['phases']) == 9 for c in cases) and phase_mismatches == 0,
            'independent nine-phase replay mismatch')
    raw = {}
    for name, profile in profiles.items():
        directory = profile['directory']
        metadata = list(profile['runtime']['movement_cost_saves'].values()) + list(profile['runtime']['game_saves'].values())
        require({v['filename'] for v in metadata} == set(SAVE_NAMES), 'missing six save metadata')
        for value in metadata:
            path = directory / value['filename']
            pin(path, value['sha256'])
            require(path.stat().st_size == value['bytes'], 'save length mismatch')
        # Each of all six streams is compared against accepted cost before, unmasked.
        raw[name] = {filename: raw_difference(args.baseline_cost / 'before.bin', directory / filename)
                     for filename in SAVE_NAMES}
        require(all(v['different_bytes'] == 0 for v in raw[name].values()), 'raw save changed')
    rng_results = {name: rng(p['summary']) for name, p in profiles.items()}
    owned_rng = {name: {key: value[key] for key in ['owned_game', 'owned_map']}
                 for name, value in rng_results.items()}
    require(all(value == owned_rng['baseline_cost'] for value in owned_rng.values()),
            'known owned RNG fields differ across profiles')
    run = subprocess.run([str(candidate.resolve()), str((args.cache_cost / 'facts.jsonl').resolve())],
                         capture_output=True, text=True, timeout=120)
    args.private_candidate_output.parent.mkdir(parents=True, exist_ok=True)
    args.private_candidate_output.write_text(run.stdout)
    require(run.returncode == 0, 'frozen offline candidate failed: ' + run.stderr.strip())
    candidate_summary = json.loads(run.stdout.splitlines()[-1])
    expected_summary = dict(accepted['frozen_rust_candidate'])
    expected_summary.update(input_sha256=digest(args.cache_cost / 'facts.jsonl'),
                            profile_sha256=profiles['cache_cost']['header']['profile_sha256'])
    require(candidate_summary == expected_summary and
            candidate_summary['compared'] == 1536 and candidate_summary['mismatches'] == 0,
            'candidate result differs from accepted baseline')
    report = dict(schema='c6-linux-movement-cost-cache-results-v1',
        accepted_baseline_sha256=digest(accepted_path), offline_checker_sha256=digest(Path(__file__)),
        review_sha256=digest(review_path), reviewed_source_revision=review['reviewed_commit'],
        runner_revision=RUNNER_REVISION, reviewed_sources=review['sources'],
        candidate_sha256=CANDIDATE, helper_sha256=HELPER, source_manifest_sha256=digest(manifest_path),
        contract_sha256=digest(contract_path), exact_configuration_changes=changes,
        cache_control_configuration_changes=config_diff(cost_config, control_config),
        baseline_control_configuration_changes=config_diff(profiles['baseline_control']['runtime']['configuration'], control_config),
        runtime_provenance={name: p['proof'] for name, p in profiles.items()},
        archived_source_manifests={name: p['runtime']['configuration']['sources'] for name, p in profiles.items()},
        corpus_sha256={name: digest(p['directory'] / 'facts.jsonl') for name, p in profiles.items()},
        ordered_comparisons=comparisons, compared_cases=1536, compared_phase_states=13824,
        independent_phase_mismatches=phase_mismatches, frozen_rust_candidate=candidate_summary,
        candidate_exit=run.returncode, raw_snapshot_pairs=raw, raw_rng_endpoints=rng_results,
        known_owned_rng_fields_equal_across_profiles=True,
        rng_unclassified_bytes='vptr and padding retained in raw within-process comparisons; no classification or masking',
        actual_variations=variations(cases), formation_layers=dict(Counter(str(c['formation_layer']) for c in cases)),
        all_genuine_sentinels_retained=True, scan_truncated=True, query_helper_behavior_invariant=True,
        original_code_executed_by_checker=False, performance_measured=False,
        complete_affected_state=False, complete_rng_inventory=False, full_movement_parity=False,
        scope='Reviewed harness cache invariance for resolved movement query outputs only; no action, legality, turn or native engine speed acceptance.',
        save_load_89198_byte_failure='Separate unresolved raw Save-to-Load failure; no unknown bytes masked or excluded.')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(output=str(args.output), cases=1536, phases=13824,
                          query_helper_behavior_invariant=True, raw_pairs=24)))


if __name__ == '__main__':
    main()
