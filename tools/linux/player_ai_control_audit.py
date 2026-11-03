#!/usr/bin/env python3
"""SHA-gated current Linux Control.Manager wire audit; no game execution.

Owner cuts are comparison evidence only, never a production decoding strategy.
"""
import argparse
from collections import Counter
import gzip
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile
from player_wmds_audit import Elf, GAME_SHA, FIXTURES

ROOT = Path(__file__).resolve().parents[2]
def sha(data):
    return hashlib.sha256(data).hexdigest()

# Complete target method bodies and virtual binding identities.
METHODS = [[47512336, 33, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::Control::ControlPriority const&)', '777ee93239f8610f836cd9f3c9c65848ebaa5659917e6f6e8684fd194634207f'], [33653264, 24, 'Serialization::operator<<(Serialization::OStream&, GameCore::AI::Control::Contract const&)', '33ddce5b8a48788d688ee481223f439049d2a476b78b82158871c52b4d311cde'], [46772000, 41, 'Serialization::operator<<(Serialization::OStream&, GameCore::PlotCoord const&)', '09befdb0e6703290977db65b34b4a9d237076caa3199171f77138d55361c355d'], [33692016, 374, 'Serialization::OStream& Serialization::operator<< <GameCore::AI::Control::Instance, 8u, 16u>(Serialization::OStream&, RecycledIDContainer<GameCore::AI::Control::Instance, 8u, 16u> const&)', '16fd902428341cbb384c3650869cd07688ad7e42f42de1536cfcbafdea4a4a15'], [33653296, 24, 'Serialization::operator>>(Serialization::IStream&, GameCore::AI::Control::Contract&)', '7ed76f8f837bcce64085db9b6c4a0f9e53a6fdf961fed0ff92a06a1d5ae1cb43'], [33691424, 386, 'Serialization::IStream& Serialization::operator>><GameCore::AI::Control::Instance, 8u, 16u>(Serialization::IStream&, RecycledIDContainer<GameCore::AI::Control::Instance, 8u, 16u>&)', 'd0fe095725806eec89690f1391c04a365da1ac7e44c444034a8bb8a73fa0886a'], [33604000, 183, 'GameCore::AI::Control::UnitRequestContract::Read(Serialization::IStream&)', '52e9ac711dcc29b1afb276657691b093cac33f5e6f837dbc1d000872adc2f960'], [33607360, 51, 'GameCore::AI::Control::PurchasePlotContract::Read(Serialization::IStream&)', 'f5b8568acd3d63d6eebdd88ff56ab90617a807cbf4d0645dafa86d3f50be021f'], [33605728, 234, 'GameCore::AI::Control::TechResearchContract::Read(Serialization::IStream&)', 'e0e5e01d8a882d4d404fd42b691f5e5aba21d0743a0e01afde9755e1bba14ee0'], [33606800, 116, 'GameCore::AI::Control::CivicResearchContract::Read(Serialization::IStream&)', '8632df6aea847c807cbfb4fae5052c9238dd53ab893d4aa768c35e7f46c1288f'], [33608352, 142, 'GameCore::AI::Control::DiplomaticActionContract::Read(Serialization::IStream&)', '4061056947186224ebdbbc154e58aea6b24389172c4fc5c0cf00cc0a864aa64b'], [33671696, 491, 'GameCore::AI::Control::Factory::CreateRawContract(GameCore::AI::Control::ContractType)', '7d00c0a7d1f4647b1d75c610b85fd027b512d10c28a58d2a66f6e580ff5b9206'], [33673408, 401, 'GameCore::AI::Control::Manager::Read(Serialization::IStream&)', 'd593b24d73bcc457218164da8b971daf2fc46b4345724c1eb158abb2272bab41'], [33655776, 222, 'GameCore::AI::Control::Contract::Read(Serialization::IStream&)', 'b6f075bc5b70a5e3599e87251cf0f1b39462cd44aedb78609694aab251ce6234'], [33672816, 108, 'GameCore::AI::Control::Instance::Read(Serialization::IStream&)', '6d99f7d0a6d10f37752c7e1c0aa6bac570672aaf436a1a52425ab80dfc5d8d6c'], [33604192, 169, 'GameCore::AI::Control::UnitRequestContract::Write(Serialization::OStream&) const', 'dd09e2ae0ebea314e3ff52585e3413fd550fc60058c699e6f16f4c24d149f56a'], [33607424, 59, 'GameCore::AI::Control::PurchasePlotContract::Write(Serialization::OStream&) const', '0c973a804b5d77b9ff1def725364ac89f3267ff63b762d84dcf12878997c2898'], [33605968, 114, 'GameCore::AI::Control::TechResearchContract::Write(Serialization::OStream&) const', '39dcba54d4fdca7cbbd6e0c4341778f3c2763e61cdfc19a7b606b0c8dc8ffe4e'], [33606928, 99, 'GameCore::AI::Control::CivicResearchContract::Write(Serialization::OStream&) const', '755e3eef818d6f2c124966901903a3039fd53781ecb6db6b6ef3b43e41901149'], [33608496, 89, 'GameCore::AI::Control::DiplomaticActionContract::Write(Serialization::OStream&) const', '55c7ea9b3dcdd8f1443410a1f71bde8788b6ff52e214ed2ea223c98b8cba37d2'], [33673088, 279, 'GameCore::AI::Control::Manager::Write(Serialization::OStream&) const', '2ec55fb0cae521cfd78e3a52ad0621c3a5056de61942c9534d7ad0891e16c139'], [33656000, 159, 'GameCore::AI::Control::Contract::Write(Serialization::OStream&) const', '617a5f626be3018aa0c88442327f08e33f49805c60a07bd46df5927c61292397'], [33672704, 74, 'GameCore::AI::Control::Instance::Write(Serialization::OStream&) const', '5a6bb0d94184c0cc150174341bac3e0a2fd0da3b65ed4d3fa05c5b7985bd2e84']]
VIRTUALS = [[60837440, 33604000, 33604192, 'UnitRequestContract'], [60837656, 33605728, 33605968, 'TechResearchContract'], [60837872, 33606800, 33606928, 'CivicResearchContract'], [60838088, 33607360, 33607424, 'PurchasePlotContract'], [60838304, 33608352, 33608496, 'DiplomaticActionContract']]

HARNESS = r'''pub use c6_core::unit;
use c6_core::player_ai_control::{ControlManager as Manager, Error};
fn main() {
    let input = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let mut pos = 0;
    let mut records = 0;
    while pos < input.len() {
        let size = u32::from_le_bytes(input[pos..pos+4].try_into().unwrap()) as usize;
        let items = u32::from_le_bytes(input[pos+4..pos+8].try_into().unwrap()) as usize;
        pos += 8;
        let raw = &input[pos..pos+size]; pos += size;
        let limits = unit::Limits { max_bytes: size, max_items: items };
        let value = Manager::decode(raw, limits).unwrap();
        assert_eq!(value.encode(limits).unwrap(), raw);
        assert_eq!(value.encode_with_items(limits).unwrap(), (raw.to_vec(), items));
        assert_eq!(Manager::decode_prefix(raw, limits).unwrap(), (value.clone(), size));
        for end in 0..size { assert!(Manager::decode(&raw[..end], limits).is_err()); }
        let short = unit::Limits { max_bytes: size-1, ..limits };
        assert_eq!(Manager::decode(raw, short), Err(Error::ResourceLimit));
        assert_eq!(value.encode(short), Err(Error::ResourceLimit));
        if items > 0 {
            let short = unit::Limits { max_items: items-1, ..limits };
            assert_eq!(Manager::decode(raw, short), Err(Error::ResourceLimit));
            assert_eq!(value.encode(short), Err(Error::ResourceLimit));
        }
        let mut suffix = raw.to_vec(); suffix.push(77);
        assert_eq!(Manager::decode(&suffix, limits), Err(Error::Trailing));
        assert_eq!(Manager::decode_prefix_with_items(&suffix, limits).unwrap(), (value, size, items));
        records += 1;
    }
    println!("{records}");
}
'''


def current_record(raw):
    """Independent typed field walk, without nested owner offsets or byte searches."""
    pos, items = 0, 0
    reconstructed = bytearray()
    def take(fmt):
        nonlocal pos
        size = struct.calcsize('<'+fmt)
        values = struct.unpack_from('<'+fmt, raw, pos)
        reconstructed.extend(struct.pack('<'+fmt, *values))
        pos += size
        return values[0] if len(values) == 1 else values
    def version(expected):
        if take('I') != expected:
            raise ValueError('unsupported version')
    def count(width):
        nonlocal items
        n = take('I')
        if n > (len(raw)-pos)//width:
            raise ValueError('count exceeds remaining record')
        items += n
        return n
    version(2)
    player, priority = take('ii')
    version(0x10001)
    if take('I') != 16:
        raise ValueError('block size mismatch')
    capacity = take('I')
    active = count(20)
    for _ in range(active):
        take('i')
        version(2)
        take('Iii')
    current_id_bits = take('I')
    free = count(4)
    take('II')
    for _ in range(free):
        take('I')
    flags = take('BB')
    if any(x > 1 for x in flags):
        raise ValueError('noncanonical bool')
    contracts = count(4)
    kinds = Counter()
    for _ in range(contracts):
        key = take('i')
        kinds[str(key)] += 1
        if key == -1:
            continue
        if key not in range(5):
            raise ValueError('unknown unframed contract')
        version(3)
        take('iiiiiIIi')
        version(2 if key in (1, 4) else 1)
        if key == 0:
            take('IIIiiI')
            if take('B') > 1:
                raise ValueError('noncanonical contract bool')
        elif key == 1:
            take('III')
        elif key in (2, 4):
            take('II')
        else:
            take('ii')
    if pos != len(raw) or reconstructed != raw:
        raise ValueError('wire extent/reconstruction differs')
    return dict(player=player, priority=priority, bytes=pos, items=items,
                capacity=capacity, active=active, free=free,
                current_id_bits=current_id_bits, contracts=contracts, contract_keys=dict(kinds))


def candidate(pin, records):
    pin = subprocess.check_output(['git', 'rev-parse', pin+'^{commit}'], cwd=ROOT, text=True).strip()
    archive = subprocess.check_output(['git', 'archive', pin], cwd=ROOT)
    with tempfile.TemporaryDirectory(prefix='c6-control-wire-') as directory:
        tmp = Path(directory)
        source = tmp/'source'
        source.mkdir()
        subprocess.run(['tar', '-x', '-C', str(source)], input=archive, check=True)
        driver = source/'tools/linux/player_ai_control_audit.py'
        if driver.read_bytes() != Path(__file__).read_bytes():
            raise ValueError('actual audit driver must be committed with candidate')
        module = source/'crates/c6-core/src/player_ai_control.rs'
        lib = source/'crates/c6-core/src/lib.rs'
        if 'pub mod player_ai_control;' not in lib.read_text():
            raise ValueError('candidate archive must already export player_ai_control')
        source_paths = ['Cargo.toml', 'Cargo.lock', 'crates/c6-core/Cargo.toml',
                        'tools/linux/player_ai_control_audit.py', 'tools/linux/player_wmds_audit.py']
        source_paths += [str(path.relative_to(source)) for path in
                         sorted((source/'crates/c6-core/src').rglob('*.rs'))]
        source_hashes = {}
        for path in source_paths:
            archived = (source/path).read_bytes()
            committed = subprocess.check_output(['git', 'show', pin+':'+path], cwd=ROOT)
            if archived != committed:
                raise ValueError('archived source differs from Git pin: '+path)
            source_hashes[path] = sha(archived)
        subprocess.run(['cargo', 'test', '--offline', '-p', 'c6-core', 'player_ai_control::tests'], cwd=source, check=True)
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
        if n != 86:
            raise ValueError('candidate record count differs')
        for path, digest in source_hashes.items():
            if sha((source/path).read_bytes()) != digest:
                raise ValueError('candidate build mutated archived source: '+path)
        return dict(source_commit=pin, archive_sha256=sha(archive),
                    module_sha256=sha(module.read_bytes()), driver_sha256=sha(driver.read_bytes()),
                    harness_sha256=sha(harness.encode()), executable_sha256=sha((tmp/'candidate').read_bytes()),
                    records=n, source_tests=2, strict_clippy=True,
                    isolated_export_added=False, source_files_sha256=source_hashes,
                    every_source_hash_verified_against_git_pin=True, actual_exported_crate_tests=True,
                    candidate_uses_exported_crate=True, archive_source_mutations=False,
                    exact_byte_and_shared_item_caps=True, every_truncation_rejected=True,
                    short_byte_and_item_caps_rejected=True, strict_trailing_rejected=True,
                    exact_prefix_consumption=True)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-root', type=Path, default=ROOT.parent/'c6-ghidra')
    p.add_argument('--source-commit')
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    elf = Elf(args.assets/'libGameCore_XP2.so', GAME_SHA)
    for address, size, name, digest in METHODS:
        if sha(elf.read(address, size)) != digest:
            raise ValueError('target method mismatch: '+name)
    for address, reader, writer, name in VIRTUALS:
        if elf.relocated_pointer(address+16+0xb8) != reader or elf.relocated_pointer(address+16+0xc0) != writer:
            raise ValueError('contract vtable mismatch: '+name)
    if elf.read(0x2dff1f0, 4) != struct.pack('<I', 0x10001):
        raise ValueError('recycled version differs')
    jump = [0x2dfef74+x for x in struct.unpack('<5i', elf.read(0x2dfef74, 20))]
    if jump != [0x201ca37, 0x201cb19, 0x201ca71, 0x201caab, 0x201cae2]:
        raise ValueError('factory switch differs')
    fixtures, corpus = [], bytearray()
    for identity, trace_name, trace_digest in FIXTURES:
        data = gzip.decompress((args.private_root/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes())
        trace = gzip.decompress((args.private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes())
        if sha(data) != identity or sha(trace) != trace_digest:
            raise ValueError('fixture identity differs')
        owners = {r['call']: r for r in map(json.loads, trace.splitlines()) if r.get('event') == 'owner' and r.get('save') == 1}
        players = {r['call']: r for r in owners.values() if r['owner'] == 'GameCore::Player::Instance'}
        control_rows = [r for r in owners.values() if r['owner'] == 'GameCore::AI::Control::Manager']
        expected = {k for k, r in players.items() if data[r['start']+77] == 1}
        if {r['parent'] for r in control_rows} != expected or len(control_rows) != len(expected):
            raise ValueError('Control presence differs from saved was-ever-alive branch')
        records = []
        for row in sorted(control_rows, key=lambda x: x['start']):
            raw = data[row['start']:row['end']]
            result = current_record(raw)
            parent = players[row['parent']]
            version, player = struct.unpack_from('<Ii', data, parent['start'])
            if version != 47 or result['player'] != player:
                raise ValueError('parent identity differs')
            corpus.extend(struct.pack('<II', len(raw), result['items']))
            corpus.extend(raw)
            result['wire_sha256'] = sha(raw)
            records.append(result)
        fixtures.append(dict(core_sha256=identity, trace_sha256=trace_digest, players=len(players),
                             ever_alive=len(expected), skipped=len(players)-len(expected), records=records))
    report = dict(schema='c6-linux-player-ai-control-exported-v2', checks_passed=True, gamecore_sha256=GAME_SHA,
                  methods=[dict(address=hex(a), size=n, name=s, sha256=h) for a,n,s,h in METHODS],
                  virtuals=[dict(vtable=hex(a), reader=hex(r), writer=hex(w), name=n) for a,r,w,n in VIRTUALS],
                  factory_keys={'0':'UnitRequest', '1':'TechResearch', '2':'CivicResearch', '3':'PurchasePlot', '4':'DiplomaticAction', '-1':'null'},
                  contract_bytes_including_key={'0':69, '1':56, '2':52, '3':52, '4':52, '-1':4},
                  fixtures=fixtures, candidate=candidate(args.source_commit, corpus) if args.source_commit else None,
                  original_execution=False, private_writes=False, complete_current_control_wire=True,
                  native_read_side_effects=False, gameplay_state=False, full_player=False, world_import=False,
                  actions=False, turns=False, unknown_save_bytes_masked=False,
                  recorded_raw_save_load_difference_bytes=89198, next_component='InfluenceMap v2',
                  reference_revision='5af389445118bed2bdebf7a8a42b219c8fba5175')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2)+'\n')

if __name__ == '__main__':
    main()
