#!/usr/bin/env python3
"""Offline serialized District projection vs accepted Linux control3 observations.

Never executes original code. Native masks/type/completion are expected answers,
never inputs to the production projection. Masks remain explicitly unavailable.
"""
import argparse
import collections
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/linux'))
from core_stream_index import index_core


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--source-commit', required=True,
                        help='exact commit archived and independently built for this candidate')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    source_commit = subprocess.check_output(
        ['git', 'rev-parse', args.source_commit + '^{commit}'], cwd=ROOT, text=True).strip()
    source_paths = [ROOT / 'crates/c6-data/src/city_movement.rs',
                    ROOT / 'crates/c6-data/examples/city_movement_inputs.rs',
                    Path(__file__).resolve()]
    for path in source_paths:
        committed = subprocess.check_output(
            ['git', 'show', source_commit + ':' + str(path.relative_to(ROOT))], cwd=ROOT)
        if committed != path.read_bytes():
            raise ValueError('checker/adapter differs from declared committed source: ' + str(path))
    binding_path = ROOT / 'evidence/linux/movement-world-control-3-binding.json'
    binding = json.loads(binding_path.read_text())
    with tempfile.TemporaryDirectory(prefix='c6-district-inputs-') as temporary:
        temporary = Path(temporary)
        # Reuse all existing immutable source, configuration, closed-control,
        # Core/RNG and original serializer ownership identity gates.
        subprocess.run([sys.executable, str(ROOT / 'tools/linux/movement_world_results.py'),
                        '--output', str(temporary / 'accepted.json')], check=True,
                       stdout=subprocess.DEVNULL)
        base = binding_path.parent
        native_path = base / binding['native']['path']
        core_path = base / binding['core']['path']
        trace_path = base / binding['trace']['path']
        native = [json.loads(line) for line in native_path.read_text().splitlines()]
        data = core_path.read_bytes()
        with gzip.open(trace_path, 'rt') as stream:
            owners = [json.loads(line) for line in stream]
        owners = [row for row in owners if row.get('event') == 'owner'
                  and row.get('save') == binding['trace_save_ordinal']]
        districts = [row for row in owners if row['owner'] == 'GameCore::District::Instance']
        assert len(districts) == 1284
        section = next(s for s in index_core(data)['sections'] if s['name'] == 'map')
        map_path = temporary / 'map.bin'
        map_path.write_bytes(data[section['start']:section['end']])
        registry = next(row for row in native if row['kind'] == 'registry'
                        and row['name'] == 'district')
        assert registry['count'] == len(registry['entries']) == 28
        assert all(entry['index'] == i and entry['query'] == i
                   for i, entry in enumerate(registry['entries']))
        terrain = next(row for row in native if row['kind'] == 'registry'
                       and row['name'] == 'terrain')
        assert terrain['count'] == len(terrain['entries']) == 17
        assert all(entry['index'] == i and entry['query'] == i
                   and type(entry['is_water']) is bool
                   for i, entry in enumerate(terrain['entries']))
        request = dict(map_path=str(map_path),
                       district_hashes=[entry['hash'] for entry in registry['entries']],
                       terrain_hashes=[entry['hash'] for entry in terrain['entries']],
                       terrain_is_water=[entry['is_water'] for entry in terrain['entries']],
                       terrain_movement_cost=[entry['integer_cost'] for entry in terrain['entries']],
                       district_prefixes=[list(data[row['start']:row['start'] + 45])
                                          for row in districts])
        request_path = temporary / 'input.json'
        request_path.write_text(json.dumps(request))
        completed = subprocess.run([str(args.candidate.resolve()), str(request_path)],
                                   check=True, capture_output=True, text=True)
        calculated_output = json.loads(completed.stdout)
        calculated = calculated_output['records']
        plots = {(p['x'], p['y']): p for p in calculated_output['plots']}
    records = {(r['owner'], r['id'] & 0xffff): r for r in calculated}
    assert len(records) == 1284
    counts = collections.Counter()
    failures = []
    unique = set()
    for case in native:
        if case['kind'] != 'case':
            continue
        for side in ('from', 'to'):
            expected = case[side]
            counts['endpoints'] += 1
            plot = plots[(expected['x'], expected['y'])]
            if plot['is_water'] != expected['is_water']:
                raise ValueError('derived Map/Terrain water differs from native endpoint')
            identity, owner = plot['district'], plot['owner']
            request_equal = (identity == expected['requested_district_id']
                             and owner == expected['district_owner']
                             and plot['has_attributes'] == expected['has_city_attributes'])
            if identity == 0xffffffff or owner == -1:
                # Native generic ComponentID invalidity precedes player lookup.
                actual = None
                counts['sentinel_misses'] += 1
            else:
                # Positive original lookup is a validity witness for THIS query.
                # No completeness or validity inferred from serialized owners.
                assert expected['district_resolved']
                actual = records.get((owner, identity & 0xffff))
                counts['positive_native_validity_witnesses'] += 1
            equal = request_equal and (actual is not None) == expected['district_resolved']
            if actual:
                native_type = expected['definitions'][4]
                equal &= actual['id'] == expected['actual_district_id']
                equal &= actual['type_index'] == native_type['index']
                equal &= actual['complete'] == expected['district_complete']
                equal &= (actual['x'], actual['y']) == (expected['x'], expected['y'])
                unique.add((actual['owner'], actual['id']))
            counts['equal' if equal else 'mismatched'] += 1
            if not equal:
                failures.append(dict(formation=case['formation_id'], side=side,
                                     requested=identity, calculated=actual))
    report = dict(schema='c6-linux-movement-domain-inputs-v1',
                  game_sha256=binding['game_sha256'], host_sha256=binding['host_sha256'],
                  profile_sha256=binding['profile_sha256'],
                  core_sha256=binding['core']['sha256'],
                  trace_sha256=binding['trace']['sha256'],
                  native_sha256=binding['native']['sha256'],
                  candidate_sha256=sha(args.candidate),
                  source_commit=source_commit,
                  sources={str(p.relative_to(ROOT)): sha(p) for p in source_paths},
                  district_records=len(records), unique_compared_districts=len(unique),
                  comparison=dict(counts), failures=failures[:10],
                  calculated_inputs=['District v15 owner-bounded serialized prefix',
                                     'typed Map actual District plot owner',
                                     'actual native District registry order/hash mapping'],
                  expected_only=['runtime district presence, ID, type, completion, location'],
                  native_validity_scope='positive native lookup proves only queried player valid; sentinel misses bypass player lookup',
                  domain_masks_compared=0,
                  derived_queries=dict(water_native_endpoints_equal=counts['endpoints'],
                      coastal_min_one_calculated_plots=len(plots),
                      coastal_min_one_true_plots=sum(p['coastal_min_one'] for p in plots.values()),
                      original_directional_neighbor_lists_calculated=len(plots),
                      native_coastal_or_neighbor_comparison=False,
                      wonder_completion_source_closed=True,
                      native_wonder_completion_comparison=False),
                  domain_masks_unavailable='No actual District Canal/InternalOnly and Building order/CanalWonder/Bridge export or complete player/container membership; source-derived coastal/neighbors/Wonder code awaits independent mask comparison',
                  native_domain_mask_required_endpoints=257,
                  full_gameplay_state_actions_turns=False,
                  raw_save_load_difference_preserved=89198)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    if failures:
        raise SystemExit('serialized District comparison failed')
    print(json.dumps(dict(counts)))


if __name__ == '__main__':
    main()
