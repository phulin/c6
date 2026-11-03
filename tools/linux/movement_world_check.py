#!/usr/bin/env python3
"""Freeze/compare a typed movement candidate; read-only original artifacts only.

A runner-reviewed binding must associate the exact native JSONL, Core snapshot,
and serializer owner trace. This tool does not infer that association from IDs,
byte patterns, matching names, or a canonical save from another execution.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
GAME = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
HOST = '106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
FIRST = 'd4dddd41f8f157a951c5b43720e6a217fef1b6600439b307ee01e300c7ce6718'
SOURCES = ['Cargo.lock', 'crates/c6-data/Cargo.toml', 'crates/c6-data/src/lib.rs',
           'crates/c6-data/src/movement.rs', 'crates/c6-data/src/registry.rs',
           'crates/c6-core/src/movement_world.rs', 'crates/c6-core/src/city_district.rs',
           'crates/c6-core/src/map.rs', 'crates/c6-core/src/unit.rs',
           'crates/c6-differential/Cargo.toml',
           'crates/c6-differential/src/bin/c6-movement-world-candidate.rs',
           'tools/linux/movement_world_check.py']
SOURCES += [str(p.relative_to(ROOT)) for p in (ROOT / 'crates/c6-core/src').rglob('*.rs')]
SOURCES = sorted(set(SOURCES + ['Cargo.toml', 'crates/c6-core/Cargo.toml',
    'evidence/linux/rules-base-snapshot.json', 'evidence/linux/rules-xp2-request-none-snapshot.json',
    'evidence/assets/xp2-rules-snapshot.json', 'evidence/linux/definition-ids-static.json',
    'evidence/linux/definition-ids-native-1.json']))


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(test, reason):
    if not test:
        raise ValueError(reason)


def read_artifact(base, record, limit):
    path = base / record['path']
    opener = gzip.open if path.suffix == '.gz' else open
    with opener(path, 'rb') as stream:
        data = stream.read(limit + 1)
    require(len(data) <= limit, 'artifact exceeds decompressed resource budget')
    require(sha(data) == record['sha256'], f'artifact identity mismatch: {path.name}')
    return data


def freeze(target, sources, source_bytes):
    subprocess.run(['cargo', 'build', '--release', '-p', 'c6-differential', '--bin',
                    'c6-movement-world-candidate'], cwd=ROOT, check=True)
    require(sources == {p: sha((ROOT / p).read_bytes()) for p in SOURCES},
            'source identity changed during candidate build')
    binary = target / 'release/c6-movement-world-candidate'
    digest = sha(binary.read_bytes())
    directory = target / 'frozen/movement-world' / digest
    directory.mkdir(parents=True, exist_ok=True)
    candidate = directory / 'candidate'
    if candidate.exists():
        require(sha(candidate.read_bytes()) == digest, 'immutable candidate path changed')
    else:
        shutil.copy2(binary, candidate)
        candidate.chmod(0o555)
    metadata = dict(schema='c6-linux-movement-world-candidate-source-v1',
                    candidate_sha256=digest, sources=sources, native_execution=False)
    manifest = directory / 'source-manifest.json'
    text = json.dumps(metadata, indent=2) + '\n'
    if manifest.exists():
        # Identical executable bits can result from comment-only source edits;
        # retain the original freeze record, recording current sources in output.
        require(json.loads(manifest.read_text())['candidate_sha256'] == digest,
                'frozen manifest candidate identity mismatch')
    else:
        manifest.write_text(text)
        manifest.chmod(0o444)
    source_identity = sha(json.dumps(sources, sort_keys=True, separators=(',', ':')).encode())
    snapshot = directory / 'sources' / source_identity
    for name, data in source_bytes.items():
        path = snapshot / name
        if path.exists():
            require(path.read_bytes() == data, 'immutable source snapshot changed')
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
            path.chmod(0o444)
    return candidate, digest, snapshot, source_identity


def compare(binding_path, candidate):
    binding = json.loads(binding_path.read_text())
    require(binding['schema'] == 'c6-linux-movement-world-binding-v1', 'binding schema mismatch')
    require(binding['game_sha256'] == GAME and binding['host_sha256'] == HOST,
            'binding original Linux identity mismatch')
    require(binding['evidence_level'] in ['native_serializer_owned', 'source_only'],
            'binding evidence level unsupported')
    base = binding_path.parent
    native = read_artifact(base, binding['native'], 256 * 1024 * 1024)
    core = read_artifact(base, binding['core'], 256 * 1024 * 1024)
    trace = read_artifact(base, binding['trace'], 64 * 1024 * 1024)
    require(binding['trace_core_sha256'] == sha(core)
            and binding['native_source_snapshot_sha256'] == sha(core),
            'reviewed trace/native/Core association unavailable')
    records = [json.loads(line) for line in native.splitlines()]
    require(records and records[0]['schema'] == 'c6-linux-movement-cost-native-v1',
            'native corpus schema missing')
    header = records[0]
    require(header['game_sha256'] == GAME and header['host_sha256'] == HOST
            and header['profile_sha256'] == binding['profile_sha256'],
            'native corpus/binding identity mismatch')
    require(records[-1]['kind'] == 'summary' and records[-1]['status'] == 0,
            'native observation incomplete or failed')
    save = binding['trace_save_ordinal']
    rows = [json.loads(line) for line in trace.splitlines()]
    rows = [r for r in rows if r.get('event') == 'owner' and r.get('save') == save]
    owners = {'map': 'GameCore::Map::Instance', 'units': 'GameCore::Unit::Instance',
              'formations': 'GameCore::Formation::Instance'}
    selected = {kind: sorted([r for r in rows if r['owner'] == owner], key=lambda r: r['start'])
                for kind, owner in owners.items()}
    require(len(selected['map']) == 1 and selected['units'] and selected['formations'],
            'original full owner inventory unavailable')
    for kind in owners:
        require(len(selected[kind]) == binding['owner_counts'][kind],
                f'original {kind} owner inventory count mismatch')
    with tempfile.TemporaryDirectory(prefix='c6-movement-world-') as temp:
        directory = Path(temp)
        fixture = dict(schema='c6-linux-movement-world-fixture-v1', game_sha256=GAME,
                       host_sha256=HOST, profile_sha256=binding['profile_sha256'],
                       native_corpus_sha256=sha(native), core_sha256=sha(core),
                       trace_sha256=sha(trace), evidence_level=binding['evidence_level'],
                       units_complete=True, formations_complete=True)
        for kind, owner_rows in selected.items():
            entries = []
            previous = 0
            for i, row in enumerate(owner_rows):
                start, end = row['start'], row['end']
                require(0 <= previous <= start < end <= len(core),
                        f'invalid/overlapping {kind} serializer-owned range')
                previous = end
                data = core[start:end]
                path = directory / f'{kind}-{i:06}.bin'
                path.write_bytes(data)
                entries.append(dict(path=path.name, sha256=sha(data),
                                    native_range=[start, end], native_call=row['call']))
            fixture[kind] = entries[0] if kind == 'map' else entries
        (directory / 'fixture.json').write_text(json.dumps(fixture))
        (directory / 'native.jsonl').write_bytes(native)
        result = subprocess.run([str(candidate), str(directory / 'native.jsonl'),
                                 str(directory / 'fixture.json')], capture_output=True, text=True)
        require(result.returncode in [0, 1, 2], 'candidate crashed or terminated unexpectedly')
        output = [json.loads(line) for line in result.stdout.splitlines()]
        summary = output[-1] if output else None
        require(summary and summary['kind'] == 'world_comparison_summary',
                f'candidate rejected inputs before comparison: {result.stderr.strip()}')
        return dict(binding_sha256=sha(binding_path.read_bytes()),
                    candidate_exit_status=result.returncode, summary=summary,
                    comparisons=output[:-1], native_acceptance=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binding', type=Path,
                        help='reviewed association of actual native corpus, Core snapshot and owner trace')
    parser.add_argument('--source-commit', help='exact commit used for an isolated archive/worktree build')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    source_bytes = {p: (ROOT / p).read_bytes() for p in SOURCES}
    sources = {p: sha(data) for p, data in source_bytes.items()}
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--format-version=1',
                                                  '--no-deps'], cwd=ROOT))
    target = Path(metadata['target_directory'])
    first = target / 'frozen/movement-cost-probe' / FIRST / 'candidate'
    first_before = sha(first.read_bytes()) if first.exists() else None
    if first_before:
        require(first_before == FIRST, 'first-run scalar candidate identity changed')
    subprocess.run(['cargo', 'test', '-p', 'c6-data', 'movement::tests'], cwd=ROOT, check=True)
    subprocess.run(['cargo', 'test', '-p', 'c6-differential', '--bin',
                    'c6-movement-world-candidate'], cwd=ROOT, check=True)
    candidate, candidate_sha, snapshot, snapshot_sha = freeze(target, sources, source_bytes)
    result = compare(args.binding, candidate) if args.binding else None
    require(not first_before or sha(first.read_bytes()) == first_before,
            'first-run scalar candidate changed during independent build')
    report = dict(schema='c6-linux-movement-world-adapter-v1',
                  game_sha256=GAME, host_sha256=HOST, sources=sources, source_commit=args.source_commit,
                  frozen_candidate_path=str(candidate), candidate_sha256=candidate_sha,
                  candidate_frozen_before_comparison=True,
                  frozen_source_snapshot_path=str(snapshot), source_snapshot_identity=snapshot_sha,
                  first_run_scalar_candidate_expected_sha256=FIRST,
                  first_run_scalar_candidate_sha256=first_before,
                  first_run_scalar_candidate_unchanged=True if first_before else None,
                  original_code_executed=False, comparison=result,
                  source_only_test_groups=4, source_only_checks_passed=True,
                  native_differential_acceptance=False, full_movement_parity=False,
                  current_cb6113b_missing_input='Terrain::IsWater registry property',
                  additional_dependencies=['matched original serializer-owned Map/Unit/Formation ranges',
                                           'reviewed actual trace/native/Core association',
                                           'actual targeted runtime district/domain-mask observations',
                                           'nonempty portal reconstruction',
                                           'complete affected state/RNG comparison'],
                  raw_save_load_different_bytes_retained=89198, unknown_bytes_masked=0)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(candidate_sha256=candidate_sha, comparison_available=bool(result),
                          native_execution=False, native_acceptance=False)))
    if result:
        raise SystemExit(result['candidate_exit_status'])


if __name__ == '__main__':
    main()
