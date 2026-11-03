#!/usr/bin/env python3
"""Pinned Linux Game19/22 header audit; optional committed-source raw diagnostic.

Never executes original game code. No search, opaque skip or transferred offset.
"""
import argparse
import json
from pathlib import Path
import struct
import subprocess
import tarfile
import uuid
from definition_ids_audit import ROOT, TARGET, elf, sha

RAW_SHA = 'a66736f0423e9a59b11c77b1879b471d936fe3887342df5008940f763dbcc5aa'
BODY_SHA = '917d430f7b9c105932f394d62b2b75c83f698437ba89e7ce0ece979dad395873'
HEADER_SHA = '25d570856e4b1e07b8fa86aa055c2b198f5b132987d0135802f08a7be46978f7'
CALLS = {
    0x2d7aab7: 0x1f24f10,  # Reset (runtime side effects not implemented here)
    0x2d7aac4: 0x1e757d0,  # Game version
    0x2d7aad0: 0x1e3e370,  # Rules manager, then virtual Read +0x38
    0x2d7aae4: 0x1ee6ef0,  # Processor Read
    0x2d7aaf3: 0x1e40590,  # current turn
    0x2d7ab02: 0x1e40590,  # max turns
    0x2d7ab0f: 0x1faa940,  # local player
    0x2d7ab23: 0x1eeb340,  # InitializeLocalPlayer (not a wire field)
    0x2d7ab32: 0x1e4d6b0,  # properties
    0x2d7ab3a: 0x1fb0c90,  # EditMap
    0x2d7ab45: 0x1f105b0,  # Map Read
}
PROBE = r'''
use c6_core::{core_stream::{CorePrefix, ThroughFirstPlayer}, map, unit};
fn main() {
 let bytes = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
 match CorePrefix::decode(&bytes, map::Limits::default()) {
  Ok(c) => {
   println!("prefix_ok {} {} {} {}", c.context_version, c.game_version, c.map_start, c.map_end);
   match ThroughFirstPlayer::decode(&bytes, map::Limits::default(), unit::Limits::default()) {
    Ok(p) => println!("continuation_ok {}", p.consumed),
    Err(e) => println!("continuation_error {:?}", e),
   }
  },
  Err(e) => println!("prefix_error {:?}", e),
 }
}
'''


def raw_probe(path):
    data = path.read_bytes()
    if sha(data) != RAW_SHA or len(data) != 76556938:
        raise ValueError('requires exact retained a667 raw Read input')
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    retained = ROOT / 'target/linux/game-header-versions' / uuid.uuid4().hex
    retained.mkdir(parents=True)
    archive = retained / 'committed-source.tar'
    subprocess.run(['git', 'archive', '--format=tar', '--output', str(archive), revision,
                    'Cargo.toml', 'Cargo.lock', 'crates',
                    'tools/linux/game_header_versions_audit.py'], cwd=ROOT, check=True)
    source = retained / 'source'
    source.mkdir()
    with tarfile.open(archive) as contents:
        contents.extractall(source, filter='data')
    package = retained / 'probe'
    package.mkdir()
    (package / 'main.rs').write_text(PROBE)
    manifest = '[workspace]\n[package]\nname="game-header-versions-probe"\nversion="0.0.0"\nedition="2021"\n[dependencies]\nc6-core={path=' + json.dumps(str(source / 'crates/c6-core')) + '}\n[[bin]]\nname="check"\npath="main.rs"\n'
    (package / 'Cargo.toml').write_text(manifest)
    run = subprocess.run(['cargo', 'run', '--offline', '--manifest-path', str(package / 'Cargo.toml'),
                          '--', str(path.resolve())], cwd=ROOT, text=True, capture_output=True)
    (retained / 'build-stderr.txt').write_text(run.stderr)
    if run.returncode:
        raise RuntimeError(run.stderr)
    binary = package / 'target/debug/check'
    return dict(raw_input_sha256=RAW_SHA, raw_input_bytes=len(data),
                source_revision=revision, source_archive_sha256=sha(archive.read_bytes()),
                retained_source_archive=str(archive.relative_to(ROOT)),
                candidate_sha256=sha(binary.read_bytes()),
                generated_manifest_sha256=sha(manifest.encode()),
                generated_lock_sha256=sha((package / 'Cargo.lock').read_bytes()),
                probe_source_sha256=sha(PROBE.encode()), result_lines=run.stdout.splitlines(),
                original_code_executed=False, complete_raw_load=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--raw-input', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    read = elf(args.assets / 'libGameCore_XP2.so', TARGET)
    body = read(0x2d7aaa0, 2540)
    header = read(0x2d7aaa0, 0xaa)
    if sha(body) != BODY_SHA or sha(header) != HEADER_SHA:
        raise ValueError('Game.Read method/header identity differs')
    if read(0x2d7aac9, 7) != bytes.fromhex('837c240c0e7c11'):
        raise ValueError('Rules >=14 signed branch differs')
    if read(0x2d7aade, 3) != bytes.fromhex('ff5138'):
        raise ValueError('Rules virtual Read differs')
    for site, target in CALLS.items():
        call = read(site, 5)
        if call[0] != 0xe8 or site + 5 + struct.unpack('<i', call[1:])[0] != target:
            raise ValueError(f'header call differs at {site:x}')
    report = dict(schema='c6-linux-game-header-versions-v1', gamecore_sha256=TARGET,
                  method=dict(address='0x2d7aaa0', bytes=2540, sha256=BODY_SHA),
                  header=dict(start='0x2d7aaa0', end='0x2d7ab4a', sha256=HEADER_SHA),
                  supported_game_versions=[19, 22], rules_gate=dict(address='0x2d7aac9', minimum=14),
                  direct_call_targets={hex(k): hex(v) for k, v in CALLS.items()},
                  prefix_wire_delta='none through Map entry', synthesized_fields=[],
                  native_runtime_side_effects_reconstructed=False,
                  legacy_nested_schemas_supported=False, complete_gameplay_state=False)
    if args.raw_input:
        report['raw_diagnostic'] = raw_probe(args.raw_input)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
    print(json.dumps(report, sort_keys=True))


if __name__ == '__main__':
    main()
