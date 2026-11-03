#!/usr/bin/env python3
"""SHA-gated complete current Game sync Random wire audit; no original execution.

Native cuts are comparison extents only. Reuses Linux-native-validated RNG codec.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile
from player_wmds_audit import Elf, GAME_SHA, FIXTURES, relocation_symbols
ROOT = Path(__file__).resolve().parents[2]
def sha(data): return hashlib.sha256(data).hexdigest()
METHODS = [[33456544, 75, 'Serialization::IStream::ReadObject(bool&)', '1b35f5e559410da6d1adcde91f94b8b0ee9a16f84d871535a2d18c3ae544487b'], [47483568, 164, 'GameCore::Random::Read(Serialization::IStream&)', 'f52dd24eb9c66c1efe8a5c2f5e877f4f041bbbef5ea5485bf26e8d7561f76dff'], [47483392, 20, 'Serialization::operator<<(Serialization::OStream&, GameCore::Random const&)', '02a0a632a63777a780ef4a70e733f8a8b20e12872840e22cc565048c88d078c6'], [47483424, 105, 'GameCore::Random::Write(Serialization::OStream&) const', 'efacb133bcd6a82530012164e75ac795ff5b0e7c2baf9a63695ea672dea33539'], [33348512, 14, 'Serialization::OStream::WriteObject(unsigned int const&)', '0baf2c125cf20f28f26a56bc061ccbb70dadd2a825949e44932986634bbea10c'], [47483536, 20, 'Serialization::operator>>(Serialization::IStream&, GameCore::Random&)', 'c244375841148ffffffdbe02089217238675fcaa06e9b353d9a5b8cfae1fecdf'], [33348528, 75, 'Serialization::IStream::ReadObject(unsigned int&)', 'b5c97ed9421b7f58e0ea8a7e2989ec1c05dd9c16a79416af11d59bd45e57dc3e'], [33454336, 14, 'Serialization::OStream::WriteObject(bool const&)', '0b101fff740deefceb76f32a685baf980dce55ee6f804dd16993e783e33aebba'], [47690960, 8, 'GameCore::Game::Instance::GetSyncRandom() const', 'f0c112fce3f22ba3110c9b92191ebdc9137f2f7c83a3906235e7ad9806bd2b4b'], [40248416, 12, 'GameCore::Game::Instance::EditSyncRandom()', '7cd63912e413a46606b7e3335de3d364622c11c6d78db119b19e2e7e348778e6'], [47687822, 51, 'Game parent current writer FreeCities/Random/Trade boundary', 'fa183b45d4cd6ab12fe14c5be77e34d4bfb5b5bc1aba7987d5095d3d03e6e6bb'], [47688927, 35, 'Game parent reader Random/Trade boundary', '903fd480466252191f3b31fed06d7a87e2a3204f9b1847e636fe8409e682fffa']]
BINDINGS = [[33456575, 31471216, 33348384], [47483583, 31683216, 47483952], [47483596, 31815744, 33341888], [47483608, 31815744, 33341888], [47483620, 31815744, 33341888], [47483632, 31815744, 33341888], [47483644, 32766320, 33431552], [47483657, 32766320, 33431552], [47483677, 33138160, 33511168], [47483688, 32334656, 33513968], [47483698, 33261496, None], [47483719, 33261496, None], [47483727, 32992576, 0], [47483402, 32349104, 47483424], [47483450, 31588976, 33341776], [47483462, 31588976, 33341776], [47483474, 31588976, 33341776], [47483486, 31588976, 33341776], [47483498, 31462048, 33431440], [47483516, 31462048, 33431440], [47483546, 31822288, 47483568], [33348559, 31471216, 33348384], [47687822, 31852224, 43310416], [47687833, 32536144, 43309200], [47687841, 33092064, 47690960], [47687852, 32285520, 47483392], [47687857, 32038448, 46158288], [47687868, 32041664, 46158000], [47688930, 33168544, 40248416], [47688941, 32968864, 47483536], [47688946, 33171520, 46158320], [47688957, 32869936, 46158080]]
HARNESS = 'use c6_core::{game_random_wire::{Random, Error}, unit::Limits};\nfn main() {\n let input=std::fs::read(std::env::args().nth(1).unwrap()).unwrap();\n let limits=Limits{max_bytes:18,max_items:0};let mut records=0;\n for raw in input.chunks_exact(18) {\n  let value=Random::decode(raw,limits).unwrap();\n  let field=|n|u32::from_le_bytes(raw[n..n+4].try_into().unwrap());\n  assert_eq!((value.record.seed,value.record.calls,value.record.resets,value.record.synchronous_byte),(field(4),field(8),field(12),raw[16]));\n  assert_eq!(value.encode(limits).unwrap(),raw);\n  assert_eq!(value.encode_with_items(limits).unwrap(),(raw.to_vec(),0));\n  for n in 0..18 {assert_eq!(Random::decode(&raw[..n],limits),Err(Error::Truncated));}\n  let short=Limits{max_bytes:17,..limits};\n  assert_eq!(Random::decode(raw,short),Err(Error::ResourceLimit));\n  assert_eq!(value.encode(short),Err(Error::ResourceLimit));\n  for actual in [0u32,2,u32::MAX] {let mut bad=raw.to_vec();bad[..4].copy_from_slice(&actual.to_le_bytes());assert_eq!(Random::decode(&bad,limits),Err(Error::Version{expected:1,actual}));}\n  for flag in 1..=255 {let mut bad=raw.to_vec();bad[17]=flag;assert_eq!(Random::decode(&bad,limits),Err(Error::UnsupportedLegacy{flag}));}\n  let mut suffix=raw.to_vec();suffix.push(99);\n  assert_eq!(Random::decode(&suffix,limits),Err(Error::Trailing));\n  assert_eq!(Random::decode_prefix_with_items(&suffix,limits).unwrap(),(value,18,0));records+=1;\n }\n assert_eq!(input.len(),records*18);println!("{records}");\n}\n'
def candidate(pin, records):
    pin = subprocess.check_output(['git', 'rev-parse', pin+'^{commit}'], cwd=ROOT, text=True).strip()
    archive = subprocess.check_output(['git', 'archive', pin], cwd=ROOT)
    with tempfile.TemporaryDirectory(prefix='c6-game-random-wire-') as directory:
        tmp = Path(directory)
        source = tmp/'source'
        source.mkdir()
        subprocess.run(['tar', '-x', '-C', str(source)], input=archive, check=True)
        driver = source/'tools/linux/game_random_wire_audit.py'
        if driver.read_bytes() != Path(__file__).read_bytes():
            raise ValueError('actual audit driver must be committed with candidate')
        module = source/'crates/c6-core/src/game_random_wire.rs'
        lib = source/'crates/c6-core/src/lib.rs'
        if 'pub mod game_random_wire;' not in lib.read_text():
            raise ValueError('candidate archive must already export game_random_wire')
        source_paths = ['Cargo.toml', 'Cargo.lock', 'crates/c6-core/Cargo.toml',
                        'tools/linux/game_random_wire_audit.py', 'tools/linux/player_wmds_audit.py']
        source_paths += [str(path.relative_to(source)) for path in
                         sorted((source/'crates/c6-core').rglob('*.rs'))]
        source_hashes = {}
        for path in source_paths:
            archived = (source/path).read_bytes()
            committed = subprocess.check_output(['git', 'show', pin+':'+path], cwd=ROOT)
            if archived != committed:
                raise ValueError('archived source differs from Git pin: '+path)
            source_hashes[path] = sha(archived)
        subprocess.run(['cargo', 'test', '--offline', '-p', 'c6-core', 'game_random_wire::tests'], cwd=source, check=True)
        subprocess.run(['cargo', 'clippy', '--offline', '-p', 'c6-core', '--all-targets', '--', '-D', 'warnings'], cwd=source, check=True)
        subprocess.run(['cargo', 'build', '--offline', '--release', '-p', 'c6-core'], cwd=source, check=True)
        harness = HARNESS
        (tmp/'candidate.rs').write_text(harness)
        command = ['rustc', '--edition=2021', '-Dwarnings', '-O', str(tmp/'candidate.rs'),
                   '--extern', 'c6_core='+str(source/'target/release/libc6_core.rlib'),
                   '-L', 'dependency='+str(source/'target/release/deps')]
        subprocess.run(command+['-o', str(tmp/'candidate')], check=True)
        (tmp/'records').write_bytes(records)
        n = int(subprocess.check_output([str(tmp/'candidate'), str(tmp/'records')], text=True).strip())
        if n != 2:
            raise ValueError('candidate record count differs')
        for path, digest in source_hashes.items():
            if sha((source/path).read_bytes()) != digest or sha(subprocess.check_output(['git', 'show', pin+':'+path], cwd=ROOT)) != digest:
                raise ValueError('candidate build mutated archived source: '+path)
        return dict(source_commit=pin, archive_sha256=sha(archive),
                    module_sha256=sha(module.read_bytes()), driver_sha256=sha(driver.read_bytes()),
                    harness_sha256=sha(harness.encode()), executable_sha256=sha((tmp/'candidate').read_bytes()),
                    records=n, source_tests=3, strict_clippy=True, full_typed_projection_compared=True,
                    isolated_export_added=False, source_files_sha256=source_hashes,
                    every_source_hash_verified_against_git_pin=True, actual_exported_crate_tests=True,
                    candidate_uses_exported_crate=True, archive_source_mutations=False,
                    exact_byte_cap_and_zero_item_credit=True, bounded_native_versions_and_legacy_flags=True, bounded_native_truncations_rejected=True,
                    short_byte_caps_rejected=True, strict_trailing_rejected=True,
                    exact_prefix_consumption=True)

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra')
    p.add_argument('--source-commit',required=True)
    p.add_argument('--output',type=Path,required=True)
    args=p.parse_args()
    elf=Elf(args.assets/'libGameCore_XP2.so',GAME_SHA)
    relocations=relocation_symbols(elf)
    for site,plt,body in BINDINGS:
        elf.call(site,plt);stub=elf.read(plt,6)
        if stub[:2]!=b'\xff\x25' or relocations.get(plt+6+struct.unpack_from('<i',stub,2)[0])!=body:raise ValueError('call relocation differs')
    for a,n,name,digest in METHODS:
        if sha(elf.read(a,n))!=digest:raise ValueError('method identity differs: '+name)
    corpus=bytearray();fixtures=[]
    for identity,trace_name,trace_digest in FIXTURES:
        data=gzip.decompress((args.private_root/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes())
        trace=gzip.decompress((args.private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes())
        if sha(data)!=identity or sha(trace)!=trace_digest:raise ValueError('fixture identity differs')
        owners=[r for r in map(json.loads,trace.splitlines()) if r.get('event')=='owner' and r.get('save')==1]
        rows=[r for r in owners if r['owner']=='GameCore::Random' and r['start']==71027778]
        if len(rows)!=1:raise ValueError('Game sync Random cardinality differs')
        row=rows[0];raw=data[row['start']:row['end']]
        if row['end']!=71027796 or len(raw)!=18:raise ValueError('Random extent differs')
        version,seed,calls,resets,synchronous,legacy=struct.unpack('<IIIIBB',raw)
        if version!=1 or legacy!=0:raise ValueError('unsupported native writer record')
        rebuilt=struct.pack('<IIIIBB',version,seed,calls,resets,synchronous,legacy)
        if rebuilt!=raw:raise ValueError('typed reconstruction differs')
        nested=[r for r in owners if r['call']!=row['call'] and row['start']<=r['start'] and r['end']<=row['end']]
        if nested:raise ValueError('unverified nested Random cuts')
        parent=[r for r in owners if r['call']==row['parent']]
        if len(parent)!=1 or parent[0]['owner']!='GameCore::Game::Instance':raise ValueError('Game parent differs')
        previous=[r for r in owners if r['end']==row['start'] and r['owner']=='GameCore::FreeCities::Manager' and r['parent']==row['parent']]
        following=[r for r in owners if r['start']==row['end'] and r['owner']=='GameCore::Trade::Manager' and r['parent']==row['parent']]
        if len(previous)!=1 or len(following)!=1:raise ValueError('adjacent Game child differs')
        corpus.extend(raw)
        fixtures.append(dict(core_sha256=identity,core_bytes=len(data),trace_sha256=trace_digest,wire_sha256=sha(raw),start=row['start'],end=row['end'],bytes=len(raw),version=version,seed=seed,calls=calls,resets=resets,synchronous_byte=synchronous,legacy_flag=legacy,items=0,parent=parent[0]['owner'],all_nested_native_cuts_compared=True,next_owner={k:following[0][k] for k in ('owner','start','end')}))
    report=dict(schema='c6-linux-game-random-wire-v1',checks_passed=True,gamecore_sha256=GAME_SHA,
        methods=[dict(address=hex(a),size=n,name=s,sha256=h) for a,n,s,h in METHODS],outer_bindings=[dict(site=hex(a),plt=hex(b),body=hex(c)) for a,b,c in BINDINGS],fixtures=fixtures,candidate=candidate(args.source_commit,corpus),
        reused_codec='rng::native_serialization::SerializedRandomRecord',prior_native_parity='evidence/linux/random-parity.json',prior_native_parity_sha256=sha((ROOT/'evidence/linux/random-parity.json').read_bytes()),complete_observed_game_sync_random_records=True,complete_current_normal_random_wire=True,complete_live_random_object=False,complete_gameplay_rng_inventory=False,original_execution=False,private_writes=False,native_read_side_effects=False,gameplay_state=False,world_import=False,actions=False,turns=False,unknown_save_bytes_masked=False,recorded_raw_save_load_difference_bytes=89198,next_component='GameCore::Trade::Manager')
    args.output.write_text(json.dumps(report,indent=2)+'\n')
if __name__=='__main__':main()
