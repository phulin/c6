#!/usr/bin/env python3
"""Strict offline domain capture comparison. Never runs original game code.

Only genuine registries/membership and serializer-owned bytes enter the frozen
Rust candidate. All plot_expected values remain comparison targets. Absent
owned attributes retain explicit unavailable fallback getter/mask results.
"""
import argparse
import collections
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

from core_stream_index import index_core

ROOT = Path(__file__).resolve().parents[2]
GAME = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
HOST = '106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
PROFILE = 'b8fcda058c1ff086e51ab9911cc81d00b58919cde3cb43ab6aca8ecd4d92c2e3'
CORE = 'ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884'
TRACE = '528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'
TRACE_GZIP = '5dd8fb4dd50a9445f32d34d3cbadf9f0ceacba0670fe57fd50e544fe57e47093'


def require(test, reason):
    if not test:
        raise ValueError(reason)


def digest(path):
    result = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            result.update(chunk)
    return result.hexdigest()


def object_pairs(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, 'duplicate JSON key: ' + key)
        result[key] = value
    return result


def parse(text):
    return json.loads(text, object_pairs_hook=object_pairs)


def load(path):
    return parse(path.read_text())


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':')).encode()


def candidate_gate(manifest_path):
    manifest = load(manifest_path)
    require(manifest['schema'] == 'c6-linux-domain-reconstruction-candidate-v1'
            and manifest['expected_outputs_used_as_inputs'] is False
            and manifest['native_execution'] is False and manifest['native_comparison'] is False,
            'candidate preparation boundary differs')
    candidate, archive = Path(manifest['candidate_path']), Path(manifest['source_snapshot_path'])
    require(candidate.stat().st_size == manifest['candidate_bytes']
            and digest(candidate) == manifest['candidate_sha256'], 'frozen candidate differs')
    require(archive.stat().st_size == manifest['source_snapshot_bytes']
            and digest(archive) == manifest['source_snapshot_sha256'], 'source archive differs')
    archive_bytes = subprocess.check_output(['git', 'archive', manifest['source_commit']], cwd=ROOT)
    require(hashlib.sha256(archive_bytes).hexdigest() == manifest['source_snapshot_sha256'],
            'source archive is not the exact committed tree')
    for name, expected in manifest['sources'].items():
        source = subprocess.check_output(['git', 'show', manifest['source_commit'] + ':' + name], cwd=ROOT)
        require(hashlib.sha256(source).hexdigest() == expected, 'committed source differs: ' + name)
    return manifest, candidate


def core_controls(capture, runtime):
    paths = [('before', 'before.bin'), ('control', 'control.bin'),
             ('after', 'after.bin'), ('repeat', 'after-repeat.bin')]
    records = []
    for role, filename in paths:
        path = capture / filename
        actual = dict(filename=filename, bytes=path.stat().st_size, sha256=digest(path))
        require(actual == runtime['domain_metadata_saves'][role], 'native internal Core artifact differs: ' + role)
        records.append(dict(role='internal_' + role, **actual))
    for role, filename in [('save', 'save.bin'), ('repeat', 'repeat.bin')]:
        path = capture / filename
        actual = dict(filename=filename, bytes=path.stat().st_size, sha256=digest(path))
        require(actual == runtime['game_saves'][role], 'native outer Core artifact differs: ' + role)
        records.append(dict(role='outer_' + role, **actual))
    baseline = (capture / 'before.bin').read_bytes()
    for record in records:
        if record['sha256'] == records[0]['sha256'] and record['bytes'] == len(baseline):
            record['raw_differences_from_before'] = 0
        else:
            data = (capture / record['filename']).read_bytes()
            record['raw_differences_from_before'] = (sum(a != b for a, b in zip(baseline, data))
                                                     + abs(len(baseline) - len(data)))
    return baseline, records


def make_request(data, native, map_path):
    """No plot_expected object or derived getter answer enters this request."""
    trace_path = ROOT.parent / 'c6-ghidra/evidence/linux-save-normalization/checkpoint-203-owners/trace-benchmark-1.jsonl.gz'
    require(digest(trace_path) == TRACE_GZIP, 'original compressed owner-trace artifact differs')
    with gzip.open(trace_path, 'rb') as stream:
        trace_bytes = stream.read()
    require(hashlib.sha256(trace_bytes).hexdigest() == TRACE, 'original serializer owner trace differs')
    owners = [parse(line) for line in trace_bytes.decode().splitlines()]
    owners = [row for row in owners if row.get('event') == 'owner' and row.get('save') == 1]
    city = [row for row in owners if row['owner'] == 'GameCore::City::Instance']
    district = [row for row in owners if row['owner'] == 'GameCore::District::Instance']
    require(len(city) == 245 and len(district) == 1284, 'original City/District ownership cardinality differs')
    for group, minimum in [(city, 20), (district, 45)]:
        prior = 0
        for row in sorted(group, key=lambda row: row['start']):
            require(0 <= row['start'] < row['end'] <= len(data)
                    and row['end'] - row['start'] >= minimum and row['start'] >= prior,
                    'invalid/overlapping original owner span')
            prior = row['end']
    section = next(section for section in index_core(data)['sections'] if section['name'] == 'map')
    map_path.write_bytes(data[section['start']:section['end']])
    registries = [row for row in native if row['kind'] == 'registry']
    require(len(registries) == 3 and {row['name'] for row in registries} == {'terrain', 'district', 'building'},
            'complete original definition packets unavailable')
    terrain = next(row for row in registries if row['name'] == 'terrain')
    districts = next(row for row in registries if row['name'] == 'district')
    players = [row for row in native if row['kind'] == 'player']
    require(len(players) == 65 and [row['query'] for row in players] == list(range(65)),
            'complete actual player mapping unavailable')
    return dict(map_path=str(map_path),
                district_hashes=[entry['hash'] for entry in districts['entries']],
                terrain_hashes=[entry['hash'] for entry in terrain['entries']],
                terrain_is_water=[entry['is_water'] for entry in terrain['entries']],
                terrain_movement_cost=[entry['movement_cost'] for entry in terrain['entries']],
                district_prefixes=[list(data[row['start']:row['start'] + 45]) for row in district],
                require_domain_metadata=True,
                domain_metadata=dict(registries=[row for row in registries if row['name'] != 'terrain'],
                                     terrain=terrain, players=players,
                                     city_records=[list(data[row['start']:row['end']]) for row in city]))


def compare(calculated, expected, width, height):
    count = width * height
    require(len(expected) == count and len(calculated['plots']) == count,
            'all plot rows must be retained')
    reconstruction = calculated['domain_reconstruction']
    require(reconstruction is not None and len(reconstruction['masks']) == count
            and len(reconstruction['wonder_complete']) == count, 'full reconstruction output unavailable')
    groups = {name: collections.Counter() for name in ['has_city_attributes', 'coastal_min_one',
                                                       'owned_wonder_complete', 'owned_raw_mask',
                                                       'owned_domain_queries']}
    fallback_masks, fallback_wonders = collections.Counter(), collections.Counter()
    rows = []
    for index, (plot, target) in enumerate(zip(calculated['plots'], expected)):
        coord = (index % width, index // width)
        require((target['x'], target['y']) == coord == (plot['x'], plot['y']),
                'original plot order/coordinate association differs')
        require(type(target['has_city_attributes']) is bool
                and type(target['coastal_min_one']) is bool and type(target['wonder_complete']) is bool,
                'incomplete original getter observations')
        raw = target['valid_domains_mask']
        require(type(raw) is int and -32768 <= raw <= 32767, 'original raw mask is not signed i16')
        require(len(target['valid_domains']) == 3
                and all(type(value) is bool for value in target['valid_domains'])
                and target['valid_domains'] == [bool(raw & (1 << domain)) for domain in range(3)],
                'original mask/getter observations are inconsistent')
        mask, wonder = reconstruction['masks'][index], reconstruction['wonder_complete'][index]
        row = dict(index=index, native_expected=target, calculated=plot,
                   calculated_owned_mask=mask, calculated_owned_wonder=wonder, unavailable=[])
        checks = dict(has_city_attributes=plot['has_attributes'] == target['has_city_attributes'],
                      coastal_min_one=plot['coastal_min_one'] == target['coastal_min_one'])
        if plot['has_attributes']:
            require(type(mask) is int and -32768 <= mask <= 32767 and type(wonder) is bool,
                    'owned attribute reconstruction missing; do not drop plot')
            checks.update(owned_wonder_complete=wonder == target['wonder_complete'], owned_raw_mask=mask == raw,
                          owned_domain_queries=[bool(mask & (1 << domain)) for domain in range(3)] == target['valid_domains'])
        else:
            require(mask is None and wonder is None, 'absent owned attributes became invented fallback state')
            fallback_masks[raw] += 1
            fallback_wonders[target['wonder_complete']] += 1
            row['unavailable'] = ['reserved-entry raw mask/domain getters', 'reserved-entry Wonder getter']
        for name, equal in checks.items():
            groups[name]['equal' if equal else 'mismatched'] += 1
        row['checks'] = checks
        rows.append(row)
    return rows, {name: dict(equal=counts['equal'], mismatched=counts['mismatched'])
                  for name, counts in groups.items()}, dict(fallback_masks), dict(fallback_wonders)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--capture', type=Path, default=ROOT.parent / 'c6-ghidra/evidence/linux-domain-metadata/native-2')
    parser.add_argument('--manifest', type=Path, default=ROOT / 'evidence/linux/movement-domain-reconstruction-candidate.json')
    parser.add_argument('--profile-sha256', default=PROFILE)
    parser.add_argument('--diagnostics', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    manifest, candidate = candidate_gate(args.manifest)
    capture = args.capture.resolve()
    runtime_path, prepared_path, native_path = capture / 'runtime.json', capture / 'prepared.json', capture / 'metadata.jsonl'
    runtime, prepared = load(runtime_path), load(prepared_path)
    configuration = runtime['configuration']
    require(hashlib.sha256(canonical(configuration)).hexdigest() == runtime['configuration_sha256']
            == prepared['configuration_sha256'] == args.profile_sha256
            and configuration == prepared['configuration'], 'actual full native configuration differs from admitted preparation')
    require(configuration['assets']['libGameCore_XP2.so'] == GAME
            and configuration['assets']['Civ6'] == HOST, 'original module/host identity differs')
    profile = configuration['lower_core_domain_metadata']
    require(profile['reconstruction_manifest'] == manifest
            and profile['reconstruction_manifest_sha256'] == digest(args.manifest)
            and profile['expected_outputs_used_as_inputs'] is False, 'native profile did not bind this candidate')
    require(runtime['returncode'] == 0 and runtime['fixture_success'] is True
            and runtime['framework_boundary_returned'] is True
            and runtime['framework_cleanup_proven'] is True and runtime['client_cleanup_proven'] is True,
            'native execution did not close with full fixture/cleanup proof')
    actual_native = dict(filename='metadata.jsonl', bytes=native_path.stat().st_size, sha256=digest(native_path))
    require(actual_native == runtime['domain_metadata_output'], 'original metadata bytes differ from closed report')
    native = [parse(line) for line in native_path.read_text().splitlines()]
    require(native and native[0]['kind'] == 'header' and native[-1]['kind'] == 'summary'
            and all(row['kind'] in {'header', 'registry', 'player', 'plot_expected', 'summary'} for row in native)
            and sum(row['kind'] == 'header' for row in native) == sum(row['kind'] == 'summary' for row in native) == 1,
            'native stream incomplete or has unexpected records')
    header, summary = native[0], native[-1]
    require(header['schema'] == 'c6-linux-domain-metadata-native-v1'
            and header['game_sha256'] == GAME and header['host_sha256'] == HOST
            and header['profile_sha256'] == args.profile_sha256 and header['player_slots'] == 65
            and header['expected_outputs_are_inputs'] is False, 'native metadata header differs')
    count = header['map_width'] * header['map_height']
    require(summary['status'] == 0 and summary['plots'] == count and summary['known_rng_equal'] is True
            and summary['rng_before'] == summary['sync_rng_after'] + summary['map_rng_after'],
            'native query summary/count/known-RNG controls failed')
    data, controls = core_controls(capture, runtime)
    report = dict(schema='c6-linux-domain-reconstruction-comparison-v1', game_sha256=GAME, host_sha256=HOST,
                  profile_sha256=args.profile_sha256, candidate_source_commit=manifest['source_commit'],
                  candidate_sha256=manifest['candidate_sha256'], manifest_sha256=digest(args.manifest),
                  runtime_sha256=digest(runtime_path), prepared_sha256=digest(prepared_path),
                  native_metadata_sha256=digest(native_path), core_controls=controls,
                  owner_trace_sha256=TRACE, owner_trace_gzip_sha256=TRACE_GZIP,
                  checker_sha256=digest(Path(__file__)),
                  expected_outputs_used_as_inputs=False, original_game_execution_by_checker=False,
                  complete_gameplay_state_actions_turns=False, unknown_bytes_masked=0,
                  historical_raw_save_load_difference_preserved=89198)
    if not all(record['sha256'] == CORE and record['bytes'] == 80657462 for record in controls):
        report.update(status='unavailable', reason='new raw Core streams differ from exact accepted ff31 owner-trace source; no spans guessed', comparisons=None)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2) + '\n')
        raise SystemExit(report['reason'])
    expected = [row for row in native if row['kind'] == 'plot_expected']
    with tempfile.TemporaryDirectory(prefix='c6-domain-reconstruction-') as temporary:
        temporary = Path(temporary)
        request = make_request(data, native, temporary / 'map.bin')
        logical_request = dict(request, map_path='sha256:' + digest(temporary / 'map.bin'))
        report['candidate_input_projection_sha256'] = hashlib.sha256(canonical(logical_request)).hexdigest()
        report['genuine_inputs'] = dict(terrain_definitions=len(request['domain_metadata']['terrain']['entries']),
            district_definitions=len(request['domain_metadata']['registries'][0]['entries']),
            building_definitions=len(request['domain_metadata']['registries'][1]['entries']),
            player_slots=len(request['domain_metadata']['players']),
            resolved_players=sum(player['resolved'] for player in request['domain_metadata']['players']),
            city_records=len(request['domain_metadata']['city_records']), district_records=len(request['district_prefixes']),
            membership_from_original_containers=True)
        request_path = temporary / 'input.json'
        request_path.write_text(json.dumps(request))
        completed = subprocess.run([str(candidate), str(request_path)], capture_output=True, text=True)
        require(completed.returncode == 0, 'frozen Rust reconstruction unavailable: ' + completed.stderr[:2000])
        calculated = parse(completed.stdout)
    rows, groups, fallback_masks, fallback_wonders = compare(calculated, expected, header['map_width'], header['map_height'])
    diagnostics = args.diagnostics or ROOT / 'target/linux/domain-metadata-comparisons' / digest(native_path) / (digest(args.manifest) + '.json')
    diagnostic_bytes = (json.dumps(dict(rows=rows, complete_candidate_output=calculated), indent=2) + '\n').encode()
    diagnostics.parent.mkdir(parents=True, exist_ok=True)
    if diagnostics.exists():
        require(diagnostics.read_bytes() == diagnostic_bytes, 'existing diagnostics differ; preserve both')
    else:
        diagnostics.write_bytes(diagnostic_bytes)
    failed = [row for row in rows if not all(row['checks'].values())]
    report.update(status='compared', plots_accounted=len(rows), comparisons=groups,
                  fallback_raw_masks=dict(unavailable=sum(fallback_masks.values()), observed_signed_values=fallback_masks),
                  fallback_wonder=dict(unavailable=sum(fallback_wonders.values()), observed_values=fallback_wonders),
                  diagnostics_path=str(diagnostics), diagnostics_sha256=digest(diagnostics),
                  first_mismatches=failed[:5], all_owned_attribute_queries_equal=not failed,
                  owned_signed_mask_histogram=dict(collections.Counter(mask for mask in calculated['domain_reconstruction']['masks'] if mask is not None)),
                  owned_wonder_true=sum(wonder is True for wonder in calculated['domain_reconstruction']['wonder_complete']),
                  full_all_plot_raw_mask_parity=False)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(plots=len(rows), comparisons=groups, fallback_unavailable=sum(fallback_masks.values()))))
    if failed:
        raise SystemExit('typed domain reconstruction differs; preserve all failing rows')


if __name__ == '__main__':
    main()
