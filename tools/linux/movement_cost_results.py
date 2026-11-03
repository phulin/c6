#!/usr/bin/env python3
"""Compare frozen movement query facts and nine native phase outputs, offline only.

Runs only the SHA-gated Rust candidate. Never loads or executes a game module.
Core snapshots are compared byte-for-byte; unknown ranges are never masked.
"""
import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path
import struct
import subprocess

PHASES = ['city', 'terrain', 'feature', 'improvement', 'route', 'river',
          'portal', 'embark', 'disembark']
COST_KEYS = {'is_slowed_by_zoc', 'full_final', 'no_zoc_final', 'initial',
             'phases', 'phased_final', 'values_overload_final', 'aware_values',
             'native_paths_agree'}


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def i32(value):
    return (value + (1 << 31)) % (1 << 32) - (1 << 31)


def integer(value):
    return i32(value << 8)


def f32(value):
    try:
        return struct.unpack('<f', struct.pack('<f', value))[0]
    except OverflowError:
        return math.copysign(math.inf, value)


def float_bits(bits):
    return struct.unpack('<f', struct.pack('<I', bits))[0]


def cvtt(value):
    if not math.isfinite(value) or value < -(1 << 31) or value >= 1 << 31:
        return -(1 << 31)
    return math.trunc(value)


def route_q8(bits):
    value = float_bits(bits)
    floor = f32(math.floor(value)) if math.isfinite(value) else value
    fraction = cvtt(f32(f32(value - floor) * 256.0)) & 255
    return i32((cvtt(floor) << 8) | fraction)


def phases(case):
    """Independent instruction-contract replay; facts remain supplied observations."""
    source, to = case['from'], case['to']
    form, edge, params, budget = (case[k] for k in ['formation', 'edge', 'parameters', 'budget'])
    hard, soft, regular, exit_add = 0, 0x7fffff00, 0, 0
    outputs = []

    def retain():
        outputs.append([hard, soft, regular, exit_add])

    if to['is_city']:
        soft = min(soft, integer(params['city_movement_cost']))
    retain()
    if to['terrain_cost'] is not None:
        regular = i32(regular + integer(1 if form['ignore_terrain'] else to['terrain_cost']))
    retain()
    if to['feature_change'] is not None and not form['ignore_feature']:
        regular = i32(regular + integer(to['feature_change']))
    retain()
    if to['improvement_change'] is not None:
        regular = i32(regular + integer(to['improvement_change']))
    retain()
    first, last = source['route'], to['route']
    if first and last and not first['pillaged'] and not last['pillaged']:
        if not edge['river_crossing'] or first['bridged'] and last['bridged']:
            a, b = first['movement_cost_float_bits'], last['movement_cost_float_bits']
            if a is not None and b is not None:
                selected = b if float_bits(b) > float_bits(a) else a
                soft = min(soft, route_q8(selected))
    retain()
    if edge['river_crossing'] and not form['ignore_rivers']:
        regular = i32(regular + integer(params['movement_river_cost']))
    retain()
    if not edge['adjacent'] and edge['source_portal_has_exit_at_destination']:
        regular = i32(regular + integer(params['movement_portal_cost']))
    retain()
    from_embarked = source['is_water'] and not (source['has_city_attributes'] and source['city_domain_valid'])
    to_embarked = to['is_water'] and not (to['has_city_attributes'] and to['city_domain_valid'])
    for transition in [form['can_embark'] and to_embarked and not source['is_water'],
                       form['can_embark'] and from_embarked and not to['is_water']]:
        if transition:
            free = source['district_complete'] and source['district_free_embark'] is True
            if source['district_complete'] and to['district_free_embark'] is not None:
                free = to['district_free_embark']
            if not free and not form['ignore_shores']:
                regular = i32(regular + integer(params['movement_embark_cost']))
            base = integer(budget['base_moves'])
            if base < budget['remaining']:
                exit_add = i32(exit_add + i32(budget['remaining'] - base))
        retain()
    return outputs


def read_corpus(path, execute):
    if path.stat().st_size > 256 * 1024 * 1024:
        raise ValueError('native corpus exceeds 256 MiB budget')
    rows = [json.loads(line) for line in path.read_text().splitlines()]
    if not rows or rows[0].get('schema') != 'c6-linux-movement-cost-native-v1':
        raise ValueError('missing native corpus header')
    if rows[0].get('execute') is not execute or rows[0].get('options') != 0:
        raise ValueError('unexpected native control/cost profile')
    if rows[-1].get('kind') != 'summary' or rows[-1].get('status') != 0:
        raise ValueError('native corpus incomplete or failed; preserve the prefix')
    cases = [r for r in rows if r.get('kind') == 'case']
    if len(cases) != rows[-1].get('cases') or not cases:
        raise ValueError('empty or inconsistent native corpus')
    registries = {r['name']: r for r in rows if r.get('kind') == 'registry'}
    if set(registries) != {'terrain', 'feature', 'improvement', 'route', 'district'}:
        raise ValueError('incomplete native definition registries')
    if any(r['count'] != len(r['entries']) for r in registries.values()):
        raise ValueError('inconsistent native registry size')
    return rows[0], rows[-1], cases, registries


def known_identity_projection(value):
    """Drop only named process-address metadata, never saved Core bytes."""
    if isinstance(value, list):
        return [known_identity_projection(v) for v in value]
    if isinstance(value, dict):
        if {'address', 'resolved'} <= value.keys():
            return {k: v for k, v in value.items() if k not in {'address', 'module_base'}}
        return {k: known_identity_projection(v) for k, v in value.items() if k not in COST_KEYS}
    return value


def variations(cases):
    counts = Counter()
    for c in cases:
        source, to, form, edge = (c[k] for k in ['from', 'to', 'formation', 'edge'])
        flags = {
            'destination_city': to['is_city'], 'destination_hills': to['is_hills'],
            'destination_feature_definition': to['feature_change'] is not None,
            'source_water': source['is_water'], 'destination_water': to['is_water'],
            'river_crossing': edge['river_crossing'], 'bridge_between': edge['bridge_between'],
            'route_pair': bool(source['route'] and to['route']),
            'pillaged_route_endpoint': any(p['route'] and p['route']['pillaged'] for p in [source, to]),
            'land_to_water': not source['is_water'] and to['is_water'],
            'water_to_land': source['is_water'] and not to['is_water'],
            'ignore_terrain': form['ignore_terrain'], 'ignore_feature': form['ignore_feature'],
            'ignore_rivers': form['ignore_rivers'], 'ignore_shores': form['ignore_shores'],
            'can_embark': form['can_embark'], 'slowed_by_zoc': c['is_slowed_by_zoc'],
            'source_free_embark_district': source['district_free_embark'] is True,
            'destination_free_embark_district': to['district_free_embark'] is True,
            'escort_member': any(m.get('found') and m.get('escort') for m in c['members']),
        }
        for name, present in flags.items():
            counts[name] += int(present)
        remaining, base = c['budget']['remaining'], integer(c['budget']['base_moves'])
        counts['remaining_' + ('below' if remaining < base else 'equal' if remaining == base else 'above') + '_base'] += 1
    return dict(sorted(counts.items()))


def raw_difference(first, second):
    changed, offset = 0, 0
    with first.open('rb') as a, second.open('rb') as b:
        while True:
            x, y = a.read(1024 * 1024), b.read(1024 * 1024)
            if not x and not y:
                break
            if x != y:
                changed += sum(p != q for p, q in zip(x, y)) + abs(len(x) - len(y))
            offset += max(len(x), len(y))
    return dict(first_sha256=digest(first), second_sha256=digest(second),
                compared_span=offset, different_bytes=changed, masked_bytes=0)


def runtime_provenance(path, header, corpus, candidate_hash, contract):
    runtime = json.loads(path.read_text())
    if runtime['returncode'] != 0 or not runtime['framework_cleanup_proven'] or not runtime['client_cleanup_proven']:
        raise ValueError('native runtime did not complete its normal lifecycle')
    if runtime['configuration_sha256'] != header['profile_sha256']:
        raise ValueError('native corpus/runtime configuration identity mismatch')
    if runtime['movement_cost_output']['sha256'] != digest(corpus):
        raise ValueError('native corpus/runtime artifact identity mismatch')
    config = runtime['configuration']
    canonical = json.dumps(config, sort_keys=True, separators=(',', ':')).encode()
    if hashlib.sha256(canonical).hexdigest() != runtime['configuration_sha256']:
        raise ValueError('native canonical configuration hash mismatch')
    if digest(path.parent / 'movement-cost-preload.so') != config['preload_sha256']:
        raise ValueError('native archived integrated preload identity mismatch')
    movement = config['lower_core_movement_cost']
    if movement['frozen_artifacts']['candidate']['sha256'] != candidate_hash:
        raise ValueError('native profile names another candidate')
    if movement['contract'] != contract or not movement['candidate_frozen_before_original_execution']:
        raise ValueError('native profile differs from the frozen probe contract')
    archive = path.parent / 'source'
    for name, expected in config['sources'].items():
        if Path(name).is_absolute() or '..' in Path(name).parts:
            raise ValueError('invalid archived source path')
        if digest(archive / name) != expected:
            raise ValueError('native archived source mismatch: ' + name)
    if digest(archive / 'tools/linux/mod_activation_probe.py') != config['runner_sha256']:
        raise ValueError('native archived runner identity mismatch')
    return runtime, dict(runtime_sha256=digest(path), configuration_sha256=runtime['configuration_sha256'],
        archived_sources_verified=len(config['sources']), archived_runner_sha256=config['runner_sha256'],
        integrated_preload_sha256=config['preload_sha256'], canonical_configuration_verified=True,
        framework_cleanup_proven=True, client_cleanup_proven=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--native', type=Path, required=True)
    parser.add_argument('--control', type=Path, required=True)
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--source-manifest', type=Path, required=True)
    parser.add_argument('--contract', type=Path, default=Path('evidence/linux/movement-cost-probe-contract.json'))
    parser.add_argument('--native-runtime', type=Path, required=True)
    parser.add_argument('--control-runtime', type=Path, required=True)
    parser.add_argument('--prior-timeout-runtime', type=Path, required=True)
    parser.add_argument('--supervisor-timing-change', type=Path, required=True)
    parser.add_argument('--snapshot-pair', nargs=3, action='append', default=[], metavar=('LABEL', 'FIRST', 'SECOND'))
    parser.add_argument('--private-candidate-output', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    contract = json.loads(args.contract.read_text())
    candidate_hash = digest(args.candidate)
    if candidate_hash != contract['frozen_builds']['candidate']['sha256']:
        raise ValueError('frozen candidate identity mismatch; do not rebuild/relabel it')
    manifest = json.loads(args.source_manifest.read_text())
    if manifest.get('candidate_sha256') != candidate_hash:
        raise ValueError('source manifest belongs to another candidate')
    revision = manifest.get('source_reference_commit', '')
    if len(revision) != 40 or any(c not in '0123456789abcdef' for c in revision):
        raise ValueError('missing immutable candidate source revision')
    required_sources = {'Cargo.lock', 'crates/c6-core/src/lib.rs',
                        'crates/c6-core/src/movement.rs',
                        'crates/c6-differential/src/bin/c6-movement-cost-candidate.rs'}
    if not required_sources <= manifest['sources_at_reference_commit'].keys():
        raise ValueError('incomplete immutable candidate source manifest')
    for path, expected in manifest['sources_at_reference_commit'].items():
        content = subprocess.check_output(['git', 'show', f'{revision}:{path}'],
                                          cwd=Path(__file__).resolve().parents[2])
        if hashlib.sha256(content).hexdigest() != expected:
            raise ValueError('frozen source revision/hash differs: ' + path)
    header, summary, cases, registries = read_corpus(args.native, True)
    control_header, control_summary, controls, control_registries = read_corpus(args.control, False)
    for h in [header, control_header]:
        if h.get('game_sha256') != contract['game_sha256'] or h.get('host_sha256') != contract['host_sha256']:
            raise ValueError('native corpus uses another original module profile')
    native_runtime, native_proof = runtime_provenance(args.native_runtime, header, args.native, candidate_hash, contract)
    control_runtime, control_proof = runtime_provenance(args.control_runtime, control_header, args.control, candidate_hash, contract)
    normalized = []
    for runtime in [control_runtime, native_runtime]:
        config = json.loads(json.dumps(runtime['configuration']))
        for key in ['runner_sha256', 'timeout_seconds']:
            config.pop(key)
        config['lower_core_movement_cost'].pop('mode')
        normalized.append(config)
    if normalized[0] != normalized[1]:
        raise ValueError('fresh control differs beyond mode/supervisor metadata')
    prior = json.loads(args.prior_timeout_runtime.read_text())
    timing = json.loads(args.supervisor_timing_change.read_text())
    changed = sorted(k for k in set(prior['configuration']) | set(native_runtime['configuration'])
                     if prior['configuration'].get(k) != native_runtime['configuration'].get(k))
    if (prior['returncode'] != 124 or changed != ['runner_sha256', 'timeout_seconds']
            or prior['configuration']['timeout_seconds'] != 180
            or native_runtime['configuration']['timeout_seconds'] != 900
            or timing['baseline_runtime_sha256'] != digest(args.prior_timeout_runtime)
            or timing['baseline_configuration_sha256'] != prior['configuration_sha256']
            or timing['configuration_sha256'] != header['profile_sha256']
            or timing['full_case_count_from_control'] != len(cases)
            or timing['quiescence_checks_weakened'] or timing['partial_run_accepted']):
        raise ValueError('retry differs beyond the explicitly authorized supervisor budget/runner')
    fact_equal = len(cases) == len(controls) and all(
        known_identity_projection(a) == known_identity_projection(b) for a, b in zip(cases, controls))
    first_mismatch, phase_mismatches = None, 0
    for index, case in enumerate(cases):
        observed = case['phases']
        expected = phases(case)
        if len(observed) != 9:
            raise ValueError('missing native phase snapshots')
        for phase, (a, b) in enumerate(zip(expected, observed)):
            if a != b:
                phase_mismatches += 1
                if first_mismatch is None:
                    first_mismatch = dict(case_index=index, phase=PHASES[phase],
                        owner=case['owner'], formation_id=case['formation_id'],
                        direction=case['direction'], expected=a, observed=b)
    run = subprocess.run([str(args.candidate.resolve()), str(args.native.resolve())],
                         capture_output=True, text=True, timeout=120)
    args.private_candidate_output.parent.mkdir(parents=True, exist_ok=True)
    args.private_candidate_output.write_text(run.stdout)
    if run.returncode not in [0, 1]:
        raise ValueError('frozen candidate rejected corpus: ' + run.stderr.strip())
    results = [json.loads(line) for line in run.stdout.splitlines()]
    candidate_summary = results[-1]
    if candidate_summary.get('kind') != 'comparison_summary' or candidate_summary.get('compared') != len(cases):
        raise ValueError('frozen candidate comparison incomplete')
    report = dict(schema='c6-linux-movement-cost-results-v1',
        native_corpus_sha256=digest(args.native), control_corpus_sha256=digest(args.control),
        candidate_sha256=candidate_hash, source_manifest_sha256=digest(args.source_manifest),
        candidate_source_revision=revision,
        source_contract_sha256=digest(args.contract),
        offline_checker_sha256=digest(Path(__file__)),
        runtime_provenance=dict(cost=native_proof, control=control_proof),
        fresh_control_configuration_comparison='identical except lower_core_movement_cost.mode, runner_sha256, timeout_seconds',
        timing_only_retry=dict(metadata_sha256=digest(args.supervisor_timing_change),
            prior_runtime_sha256=digest(args.prior_timeout_runtime), changed_configuration_keys=changed,
            prior_timeout_seconds=180, timeout_seconds=900, case_count_reduced=False, guards_weakened=False),
        native_profile=header['profile_sha256'], control_profile=control_header['profile_sha256'],
        compared_cases=len(cases), compared_phase_states=len(cases)*9,
        phase_state_mismatches=phase_mismatches, first_phase_mismatch=first_mismatch,
        frozen_rust_candidate=candidate_summary, candidate_exit=run.returncode,
        fresh_control_facts_equal=fact_equal, registries_equal=registries == control_registries,
        known_rng_equal=dict(cost=summary['known_rng_equal'], control=control_summary['known_rng_equal']),
        native_scan_truncated=summary['truncated'], control_scan_truncated=control_summary['truncated'],
        actual_variations=variations(cases),
        owners=sorted({c['owner'] for c in cases}),
        formation_layers=dict(Counter(str(c['formation_layer']) for c in cases)),
        formation_domains=dict(Counter(str(c['formation_domain']) for c in cases)),
        terrain_definition_hashes=sorted({c['to']['definitions'][0]['hash'] for c in cases
                                          if c['to']['definitions'][0]['present']}),
        feature_definition_hashes=sorted({c['to']['definitions'][1]['hash'] for c in cases
                                          if c['to']['definitions'][1]['present']}),
        route_float_bits=sorted({p['route']['movement_cost_float_bits'] for c in cases
                                 for p in [c['from'], c['to']] if p['route']
                                 and p['route']['movement_cost_float_bits'] is not None}),
        raw_snapshot_pairs={label: raw_difference(Path(a), Path(b)) for label, a, b in args.snapshot_pair},
        save_load_89198_byte_failure='separate recorded unresolved failure; no bytes masked or excluded here',
        scope='resolved-query arithmetic and actual native phase outputs; no movement action/legality/turn acceptance',
        complete_affected_state=False, complete_rng_inventory=False, full_movement_parity=False)
    report['scalar_comparison_equal'] = phase_mismatches == 0 and run.returncode == 0
    report['provided_raw_snapshot_pairs_equal'] = (all(r['different_bytes'] == 0
        for r in report['raw_snapshot_pairs'].values()) if report['raw_snapshot_pairs'] else None)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(output=str(args.output), cases=len(cases), phase_mismatches=phase_mismatches,
                          rust_mismatches=candidate_summary['mismatches'], control_facts_equal=fact_equal)))
    if (not report['scalar_comparison_equal'] or not fact_equal or registries != control_registries
            or not summary['known_rng_equal'] or not control_summary['known_rng_equal']
            or report['provided_raw_snapshot_pairs_equal'] is False):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
