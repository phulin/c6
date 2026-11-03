#!/usr/bin/env python3
"""SHA-gated read-only Eras contracts and isolated committed-source wire audit.

Never executes original game code. Owner cuts validate record coverage only.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile
ROOT = Path(__file__).resolve().parents[2]
def sha(data):
    return hashlib.sha256(data).hexdigest()
HOST_SHA = '106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
GAME_SHA = 'a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'


class Elf:
    def __init__(self, path, digest):
        self.raw = path.read_bytes()
        if hashlib.sha256(self.raw).hexdigest() != digest:
            raise ValueError(f'original Linux identity mismatch: {path.name}')
        if self.raw[:6] != b'\x7fELF\x02\x01':
            raise ValueError('requires little-endian ELF64')
        self.loads = []
        start = struct.unpack_from('<Q', self.raw, 32)[0]
        width, count = struct.unpack_from('<HH', self.raw, 54)
        for i in range(count):
            kind, _, offset, address, _, size, _, _ = struct.unpack_from('<IIQQQQQQ', self.raw, start + i * width)
            if kind == 1:
                self.loads.append((address, address + size, offset))

    def read(self, address, size):
        for lo, hi, offset in self.loads:
            if lo <= address and address + size <= hi:
                return self.raw[offset + address - lo:offset + address - lo + size]
        raise ValueError(f'not file-backed: {address:#x}')

    def relocated_pointer(self, address):
        section_start = struct.unpack_from('<Q', self.raw, 40)[0]
        section_width, section_count = struct.unpack_from('<HH', self.raw, 58)
        sections = [struct.unpack_from('<IIQQQQIIQQ', self.raw, section_start + i * section_width)
                    for i in range(section_count)]
        for section in sections:
            if section[1] != 4:  # SHT_RELA
                continue
            symbols = sections[section[6]]
            for pos in range(section[4], section[4] + section[5], section[9]):
                target, info, addend = struct.unpack_from('<QQq', self.raw, pos)
                if target != address:
                    continue
                if info & 0xffffffff == 8:  # R_X86_64_RELATIVE
                    return addend
                if info & 0xffffffff == 1:  # R_X86_64_64
                    symbol = symbols[4] + (info >> 32) * symbols[9]
                    return struct.unpack_from('<Q', self.raw, symbol + 8)[0] + addend
                raise ValueError('unsupported relocation type')
        raise ValueError('pointer relocation missing')

    def call(self, address, target):
        raw = self.read(address, 5)
        if raw[0] != 0xe8 or address + 5 + struct.unpack_from('<i', raw, 1)[0] != target:
            raise ValueError(f'call contract mismatch: {address:#x}')


def relocation_symbols(elf):
    raw=elf.raw
    start=struct.unpack_from('<Q',raw,40)[0]
    width,count=struct.unpack_from('<HH',raw,58)
    sections=[struct.unpack_from('<IIQQQQIIQQ',raw,start+i*width) for i in range(count)]
    result={}
    for section in sections:
        if section[1]!=4: continue
        symbols=sections[section[6]]
        for pos in range(section[4],section[4]+section[5],section[9]):
            target,info,_=struct.unpack_from('<QQq',raw,pos)
            if info & 0xffffffff != 7: continue
            symbol=symbols[4]+(info>>32)*symbols[9]
            result[target]=struct.unpack_from('<Q',raw,symbol+8)[0]
    return result

FIXTURES = [('ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884','trace-benchmark-1.jsonl.gz','528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'),
 ('85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8','trace-captured-1.jsonl.gz','027e6d4470a8755121d3e02221a36ae978fd4fd8e128ca296896144064f2a369')]
METHODS = [(45019568, 15, 'FAutoVariable<GameCore::EraTypes, GameCore::Player::Eras>::load(Serialization::IStream&)', '7ee2369e7a7be2d4f91ca488e5901a36d155b7f648fd0fbb2c0a2cec40313df9'), (47517872, 34, 'int Serialization::HashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Era> >(unsigned int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Era> const&)', 'a53222fe1ac63af8f4e0ba9d013a019338f774f95ec270948f74020b6a345894'), (45018368, 34, 'Serialization::IStream& Serialization::ReadVersioned<FAutoVariable<GameCore::EraTypes, GameCore::Player::Eras>, GameCore::EraTypes>(Serialization::IStream&, FAutoVariable<GameCore::EraTypes, GameCore::Player::Eras>&, int, int&&, GameCore::EraTypes&&)', 'a0184602a9d52b040a68a308bf1da24c04864b7c9d0d3eefe35299ba00f51317'), (47517824, 41, 'void Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Era> >(Serialization::OStream&, unsigned int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Era> const&)', '9a6dce189a54362122658819c2dcb40c5ccbe9afd2c202d92e7cf9d253ce615d'), (45016064, 66, 'Serialization::operator<<(Serialization::OStream&, GameCore::Player::Eras const&)', '93c799a67928cbbce16bae245d7c7cecf66ed21acc870a342da9a86f91e77809'), (47508896, 36, 'Serialization::operator<<(Serialization::OStream&, GameCore::EraTypes const&)', 'e246d7b03b8b87e64a140a859d261d190aed6aacf1f69d421667e19c8910889d'), (45016144, 97, 'Serialization::operator>>(Serialization::IStream&, GameCore::Player::Eras&)', '495267ea19bb024f9029aaff583368ea04ff55f221a730b69729250d6707b461'), (47508944, 58, 'Serialization::operator>>(Serialization::IStream&, GameCore::EraTypes&)', 'a7695171a09cc43289748975d8d653be8e0ddc5594aa8e7c1ce88fffd435c54b'), (45016912, 28, 'GameCore::Player::Eras::Reset()', '58ac0652157cf362ccbfec77d0edc9955333613f9ddc4574c0e6c86e88411d5a'), (45019632, 15, 'FAutoVariable<GameCore::EraTypes, GameCore::Player::Eras>::save(Serialization::OStream&) const', '3988666dc64dd03ce8fa3652a6e5122bea51155e4f698ab6515f24fd71104a32'), (33777968, 8, 'GameCore::Player::Instance::GetEras() const', 'b38de34690aced1000a31e455940c58b29439104413f8a3d08a90027fb739674'), (45018304, 21, 'Serialization::OStream& operator<< <GameCore::EraTypes, GameCore::Player::Eras>(Serialization::OStream&, FAutoVariable<GameCore::EraTypes, GameCore::Player::Eras> const&)', '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'), (45018336, 21, 'Serialization::IStream& operator>><GameCore::EraTypes, GameCore::Player::Eras>(Serialization::IStream&, FAutoVariable<GameCore::EraTypes, GameCore::Player::Eras>&)', '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55')]
BINDINGS = [(45019578, 32442288, 47508944, 'jmp'), (47517880, 32541792, 35512720, 'call'), (47517894, 33046416, 33919552, 'jmp'), (45018382, 32859760, 45018592, 'call'), (45018392, 32816640, 45018336, 'call'), (47517837, 32718704, 47519536, 'call'), (47517854, 31588976, 33341776, 'call'), (45016081, 32898000, 33341744, 'call'), (45016096, 32823776, 45018304, 'call'), (45016114, 32823776, 45018304, 'call'), (47508904, 32555776, 35732336, 'call'), (47508917, 31661440, 47517824, 'call'), (45016162, 31938512, 33341856, 'call'), (45016177, 32816640, 45018336, 'call'), (45016225, 33126384, 45018368, 'call'), (47508963, 31815744, 33341888, 'call'), (47508972, 32555776, 35732336, 'call'), (47508982, 32105440, 47517872, 'call'), (45016933, 32859760, 45018592, 'call'), (45019642, 32053584, 47508896, 'jmp'), (45412609, 31470912, 33777968, 'call'), (45412620, 31939840, 45016064, 'call'), (45412628, 32955232, 33345776, 'call'), (45412639, 31692768, 45689104, 'call')]
DWARF_SHA = '21987f0cb7a073ef99b80eefa2a2ab7805f2a0f0dbe640206067b3a8db1b900d'

HARNESS = r'''pub use c6_core::unit;
#[path="MODULE"] pub mod player_eras;
fn main(){let args:Vec<_>=std::env::args().collect();let input=std::fs::read(&args[1]).unwrap();let exact=unit::Limits{max_bytes:12,max_items:0};let(v,n,items)=player_eras::Eras::decode_prefix_with_items(&input,exact).unwrap();assert_eq!(n,input.len());assert_eq!(items,0);let(out,witems)=v.encode_with_items(exact).unwrap();assert_eq!(items,witems);assert_eq!(input,out);assert_eq!(v,player_eras::Eras::decode(&out,exact).unwrap());for end in 0..12{assert!(player_eras::Eras::decode(&input[..end],exact).is_err());}let short=unit::Limits{max_bytes:11,..exact};assert!(player_eras::Eras::decode(&input,short).is_err());assert!(v.encode(short).is_err());std::fs::write(&args[2],out).unwrap();println!("{} {} {} {}",n,items,v.era_hash,v.game_era_hash);}
'''

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-root', type=Path, default=ROOT.parent/'c6-ghidra')
    p.add_argument('--source-commit', required=True)
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    game = Elf(args.assets/'libGameCore_XP2.so', GAME_SHA)
    Elf(args.assets/'Civ6', HOST_SHA)
    for address, size, name, digest in METHODS:
        if sha(game.read(address, size)) != digest:
            raise ValueError('method mismatch: '+name)
    if struct.unpack('<I', game.read(0x2f5ded8, 4))[0] != 2:
        raise ValueError('current version mismatch')
    relocations = relocation_symbols(game)
    for site, plt, body, kind in BINDINGS:
        code = game.read(site, 5)
        if code[0] != (0xe8 if kind == 'call' else 0xe9) or site+5+struct.unpack_from('<i', code, 1)[0] != plt:
            raise ValueError('direct binding mismatch')
        stub = game.read(plt, 6)
        if stub[:2] != b'\xff\x25' or relocations.get(plt+6+struct.unpack_from('<i', stub, 2)[0]) != body:
            raise ValueError('concrete PLT binding mismatch')
    for offset, body in [(0x20, 0x2aef1b0), (0x38, 0x2aef1f0)]:
        if game.relocated_pointer(0x3ad9ed8+offset) != body:
            raise ValueError('FAutoVariable virtual binding mismatch')
    dwarf = subprocess.check_output(['gdb','-nx','-batch',str(args.assets/'libGameCore_XP2.so'),'-ex',"ptype /o 'GameCore::Player::Eras'"], text=True)
    if sha(dwarf.encode()) != DWARF_SHA:
        raise ValueError('DWARF mismatch')
    pin = subprocess.check_output(['git','rev-parse',args.source_commit+'^{commit}'],cwd=ROOT,text=True).strip()
    reports = []
    with tempfile.TemporaryDirectory(prefix='c6-eras-') as tmp:
        tmp = Path(tmp)
        source = tmp/'source'; source.mkdir()
        archive = subprocess.check_output(['git','archive',pin],cwd=ROOT)
        subprocess.run(['tar','-x','-C',str(source)],input=archive,check=True)
        if (source/'tools/linux/player_eras_audit.py').read_bytes() != Path(__file__).read_bytes():
            raise ValueError('audit driver must be committed in candidate source identity')
        paths = [source/'Cargo.toml',source/'Cargo.lock',source/'crates/c6-core/Cargo.toml',*sorted((source/'crates/c6-core/src').rglob('*.rs')),source/'tools/linux/player_eras_audit.py']
        sources = {str(x.relative_to(source)):sha(x.read_bytes()) for x in paths}
        subprocess.run(['cargo','build','--offline','--release','-p','c6-core'],cwd=source,check=True)
        harness = HARNESS.replace('MODULE',str(source/'crates/c6-core/src/player_eras.rs'))
        (tmp/'candidate.rs').write_text(harness)
        cmd = ['rustc','--edition=2021','-Dwarnings','-O',str(tmp/'candidate.rs'),'--extern','c6_core='+str(source/'target/release/libc6_core.rlib'),'-L','dependency='+str(source/'target/release/deps')]
        subprocess.run(cmd+['-o',str(tmp/'candidate')],check=True)
        subprocess.run(cmd+['--test','-o',str(tmp/'tests')],check=True)
        subprocess.run([str(tmp/'tests')],check=True)
        candidate_sha = sha((tmp/'candidate').read_bytes())
        for identity,trace_name,trace_sha in FIXTURES:
            data = gzip.decompress((args.private_root/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes())
            trace = gzip.decompress((args.private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes())
            if sha(data)!=identity or sha(trace)!=trace_sha:
                raise ValueError('fixture identity mismatch')
            rows = [json.loads(x) for x in trace.splitlines()]
            cuts = sorted([x for x in rows if x.get('event')=='owner' and x.get('save')==1 and x.get('owner')=='GameCore::Player::Eras'],key=lambda x:x['start'])
            if len(cuts)!=43:
                raise ValueError('Eras record count mismatch')
            observations=[];digests=[]
            for cut in cuts:
                payload=data[cut['start']:cut['end']]
                (tmp/'in').write_bytes(payload)
                words=[int(x) for x in subprocess.check_output([str(tmp/'candidate'),str(tmp/'in'),str(tmp/'out')],text=True).split()]
                if (tmp/'out').read_bytes()!=payload or words[:2]!=[12,0]:
                    raise ValueError('typed Eras comparison mismatch')
                observations.append(words);digests.append(sha(payload))
            reports.append(dict(stream_sha256=identity,trace_sha256=trace_sha,records=len(cuts),record_bytes=sum(x['end']-x['start'] for x in cuts),first_observed_range=[cuts[0]['start'],cuts[0]['end']],ordered_record_digests_sha256=sha(json.dumps(digests,separators=(',',':')).encode()),typed_observations_sha256=sha(json.dumps(observations,separators=(',',':')).encode())))
    report=dict(schema='c6-linux-player-eras-v1',checks_passed=True,gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,current_version=2,version_address='0x2f5ded8',methods=[dict(address=hex(a),size=n,name=name,sha256=h) for a,n,name,h in METHODS],concrete_bindings=[dict(site=hex(a),plt=hex(b),body=hex(c),kind=k)for a,b,c,k in BINDINGS],dwarf_sha256=DWARF_SHA,candidate_source_commit=pin,candidate_archive_sha256=sha(archive),candidate_sources=sources,candidate_sha256=candidate_sha,harness_sha256=sha(harness.encode()),audit_source_sha256=sha(Path(__file__).read_bytes()),fixtures=reports,source_only_tests=2,exact_native_record_byte_item_caps_checked=True,one_short_native_record_byte_cap_rejected=True,item_cap='zero fixed-scalar charge; one-short cap inapplicable',native_every_truncation_rejected=True,allocation_policy='aggregate serialized bytes; zero collection items; no dynamic typed storage; fallible wire output reservation',original_execution=False,private_writes=False,complete_current_eras_wire=True,legacy_versions=False,definition_remapping=False,native_read_fixups=False,native_current_read_reset=False,native_current_read_callbacks=False,full_player=False,world_import=False,gameplay_state=False,actions=False,turns=False,next_player_component='Trade',unknown_save_bytes_masked=False,recorded_raw_save_load_difference_bytes=89198,attribution_reference_revision='5af389445118bed2bdebf7a8a42b219c8fba5175',reference_role='discovery_only')
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,fixtures=reports)))

if __name__=='__main__':
    main()
