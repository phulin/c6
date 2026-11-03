#!/usr/bin/env python3
"""Probe original Read-input first-City grammar using retained parser instrumentation.

Only the retained test copy gains a private-Reader stop. Live core source and
production support remain unchanged. No offset is transferred from SavedOutput.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import uuid

ROOT = Path(__file__).resolve().parents[2]
RAW_SHA = 'a66736f0423e9a59b11c77b1879b471d936fe3887342df5008940f763dbcc5aa'
INSTRUMENT = '''
// Retained test-only first-Cities stop; not production API or complete Read.
pub fn diagnostic_first_city_prefix(bytes: &[u8], limits: Limits)
    -> Result<(u32, i32, Vec<(i32, u32, bool)>, usize), Error> {
    let mut r = Reader { bytes, pos: 0, items: 0, limits };
    let _religion = r.religion()?;
    r.version(1)?;
    let player_count = r.u32()?;
    if player_count == 0 || player_count as usize > r.remaining().max_items {
        return Err(Error::ResourceLimit);
    }
    r.charge(0, 1)?;
    let requested_owner = r.i32()?;
    let header = r.header()?;
    if requested_owner != header.owner {
        return Err(Error::OwnerMismatch { requested: requested_owner, record: header.owner });
    }
    r.version(1)?;
    let _formations = r.recycled(Reader::formation)?;
    let _goody_hut = r.component(pc::GoodyHut::decode_prefix_with_items, Error::Component)?;
    let _bonuses = r.component(pc::Bonuses::decode_prefix_with_items, Error::Component)?;
    let _congress = r.component(pc::Congress::decode_prefix_with_items, Error::Component)?;
    let cities = r.cities()?;
    let flags = cities.records.entries.iter().map(|entry| {
        let city = &entry.value;
        (city.header.owner, city.header.id, city.citizens.force_update_plot_appeal)
    }).collect();
    Ok((player_count, requested_owner, flags, r.pos))
}
'''


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--raw-input', type=Path, default=ROOT.parent / 'c6-ghidra/evidence/saves/xp2-gamecore-stream.bin')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if sha(args.raw_input) != RAW_SHA:
        raise ValueError('requires exact original a667 Read-input stream')
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    retained = ROOT / 'target/linux/yield-monitor-raw-city-prefix-candidates' / uuid.uuid4().hex
    retained.mkdir(parents=True)
    original = retained / 'committed-source.tar'
    subprocess.run(['git', 'archive', '--format=tar', '--output', str(original), revision,
                    'Cargo.toml', 'Cargo.lock', 'crates',
                    'tools/linux/yield_monitor_raw_city_prefix_check.rs',
                    'tools/linux/yield_monitor_raw_city_prefix_check.py'], cwd=ROOT, check=True)
    source = retained / 'source'
    source.mkdir()
    with tarfile.open(original) as contents:
        contents.extractall(source, filter='data')
    core = source / 'crates/c6-core/src/core_player.rs'
    committed_core_sha = sha(core)
    core.write_bytes(core.read_bytes() + INSTRUMENT.encode())
    (retained / 'test-instrumentation.rs').write_text(INSTRUMENT)
    preimages = {str(path.relative_to(source)): sha(path)
                 for path in sorted(source.rglob('*')) if path.is_file()}
    (retained / 'source-manifest.json').write_text(json.dumps(preimages, indent=2, sort_keys=True) + '\n')
    with tarfile.open(retained / 'compiled-source.tar', 'w') as archive:
        archive.add(source, arcname='source')
    package = retained / 'check'
    package.mkdir()
    cargo = package / 'Cargo.toml'
    cargo.write_text('[workspace]\n[package]\nname="yield-monitor-raw-city-prefix-check"\nversion="0.0.0"\nedition="2021"\n'
                     '[dependencies]\nserde_json="1"\nc6-core={path=' + json.dumps(str(source / 'crates/c6-core')) + '}\n'
                     '[[bin]]\nname="check"\npath=' + json.dumps(str(source / 'tools/linux/yield_monitor_raw_city_prefix_check.rs')) + '\n')
    shutil.copyfile(source / 'Cargo.lock', package / 'Cargo.lock')
    target = ROOT / 'target/linux/yield-monitor-raw-city-prefix-build'
    run = subprocess.run(['cargo', 'run', '--offline', '--manifest-path', str(cargo),
                          '--target-dir', str(target), '--', str(args.raw_input)], text=True, capture_output=True)
    (retained / 'build-stderr.txt').write_text(run.stderr)
    if run.returncode:
        raise RuntimeError(run.stderr)
    result = json.loads(run.stdout)
    candidate = retained / 'candidate'
    shutil.copyfile(target / 'debug/check', candidate)
    result.update(raw_input_sha256=RAW_SHA, source_revision=revision,
                  test_only_parser_instrumentation=True,
                  committed_core_player_sha256=committed_core_sha,
                  compiled_core_player_sha256=sha(core),
                  instrumentation_sha256=sha(retained / 'test-instrumentation.rs'),
                  retained_source=str(source.relative_to(ROOT)),
                  committed_source_archive_sha256=sha(original),
                  compiled_source_archive_sha256=sha(retained / 'compiled-source.tar'),
                  source_sha256=preimages, candidate_sha256=sha(candidate),
                  generated_manifest_sha256=sha(cargo), generated_lock_sha256=sha(package / 'Cargo.lock'),
                  production_raw_input_support_extended=False)
    (retained / 'result.json').write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps({key: value for key, value in result.items() if key not in ['source_sha256', 'city_identities']}))


if __name__ == '__main__':
    main()
