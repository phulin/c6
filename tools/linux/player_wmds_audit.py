#!/usr/bin/env python3
"""SHA-gated read-only WMDs contracts and isolated committed-source wire audit.

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
METHODS = [[45901072, 149, 'bool Serialization::ReadIndexHashArray<int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::WMD> >(Serialization::IStream&, int*, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::WMD> const&)', '9b7c9a0e9231598cdde29b8db31c1c6e3d0aaff80c1eeb877c996afadf4de576'], [45900928, 141, 'bool Serialization::WriteIndexHashArray<int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::WMD> >(Serialization::OStream&, int const*, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::WMD> const&)', '330d8787d32741bf0e98b0e9e21f7a6c02ca282d8ae30fa3695a19dfaf92e6ac'], [33348528, 75, 'Serialization::IStream::ReadObject(unsigned int&)', 'b5c97ed9421b7f58e0ea8a7e2989ec1c05dd9c16a79416af11d59bd45e57dc3e'], [33341888, 14, 'Serialization::IStream& Serialization::IStream::operator>><unsigned int>(unsigned int&)', '5edd23b31cc38a2cc74b61c4b33889da2f35c69f9ce10ee24f2899eb7e91af01'], [33348512, 14, 'Serialization::OStream::WriteObject(unsigned int const&)', '0baf2c125cf20f28f26a56bc061ccbb70dadd2a825949e44932986634bbea10c'], [33341776, 14, 'Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)', '9731ca1d2959dc0f406a684267a29d425c9bf7ea2f127425123508fbc2ed2d3b'], [45899696, 67, 'Serialization::operator<<(Serialization::OStream&, GameCore::Player::WMDs const&)', '323b93c853d92faff2acad398ebe724cdee9ed8389c07693841e9ac87c9f60c8'], [45899776, 73, 'Serialization::operator>>(Serialization::IStream&, GameCore::Player::WMDs&)', '04c7df0b3c0c32e3f1bcc6d8177fc6cae3b4d7f803ab8ef5b15fce3ec5dc2289'], [33341744, 14, 'Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)', '2a3fc110a86f71122b45f03f192ddbc4499241d298e49b843ea750bd49d5e1cf'], [33341856, 14, 'Serialization::IStream& Serialization::IStream::operator>><int>(int&)', 'c6cf9d3175d5134bd8d7b3d396184fd7b9abd3f7ca364cbdc38a1794ce742775'], [33348288, 14, 'Serialization::OStream::WriteObject(int const&)', '0baf2c125cf20f28f26a56bc061ccbb70dadd2a825949e44932986634bbea10c'], [33348304, 75, 'Serialization::IStream::ReadObject(int&)', '09ddfd4d815a2204789075821ff06a3e8d881dc2bae4791ff3f746206deb3230'], [33348384, 25, 'Serialization::IStream::ReadRequest::ReadRequest(unsigned long, unsigned long)', '43f291f25ae0e59e98bf6b96e5341ed0345d1627c75c056bac01ada2cfd2f796']]
BINDINGS = [[45901100, 31815744, 33341888, 'call'], [45901142, 31815744, 33341888, 'call'], [45901154, 31715440, 45901808, 'call'], [45901190, 31938512, 33341856, 'call'], [45900953, 31771648, 43368112, 'call'], [45900970, 31588976, 33341776, 'call'], [45900997, 31816480, 34469152, 'call'], [45901005, 32568416, 36806288, 'call'], [45901020, 31588976, 33341776, 'call'], [45901031, 32898000, 33341744, 'call'], [33348559, 31471216, 33348384, 'call'], [33341892, 33039872, 33348528, 'call'], [33341780, 32631936, 33348512, 'call'], [45899713, 32898000, 33341744, 'call'], [45899725, 33113120, 33567184, 'call'], [45899733, 32911120, 35735904, 'call'], [45899747, 32559360, 45900928, 'call'], [45899791, 31938512, 33341856, 'call'], [45899803, 32762928, 45901232, 'call'], [45899811, 32377088, 33566496, 'call'], [45899819, 32911120, 35735904, 'call'], [45899833, 32834800, 45901072, 'call'], [33341748, 32120032, 33348288, 'call'], [33341860, 31737840, 33348304, 'call'], [33348335, 31471216, 33348384, 'call'], [33348404, 31796656, 33348416, 'jmp'], [45412715, 32312528, 45899696, 'call']]
VIRTUALS = []
DWARF_SHA = '19cce2aa08df00719939eddd9c4f066b412cebf77ff10f90a1fa7af119987637'

HARNESS = r'''pub use c6_core::unit;
#[path="MODULE"] pub mod player_wmds;
fn main(){let a:Vec<_>=std::env::args().collect();let b=std::fs::read(&a[1]).unwrap();let generous=unit::Limits{max_bytes:b.len(),max_items:b.len()};let(v,n,items)=player_wmds::Wmds::decode_prefix_with_items(&b,generous).unwrap();assert_eq!(n,b.len());let exact=unit::Limits{max_bytes:n,max_items:items};let(out,wi)=v.encode_with_items(exact).unwrap();assert_eq!(wi,items);assert_eq!(out,b);assert_eq!(v,player_wmds::Wmds::decode(&b,exact).unwrap());for end in 0..n{assert!(player_wmds::Wmds::decode(&b[..end],exact).is_err());}for lim in [unit::Limits{max_bytes:n-1,..exact},unit::Limits{max_items:items-1,..exact}]{assert!(player_wmds::Wmds::decode(&b,lim).is_err());assert!(v.encode(lim).is_err());}std::fs::write(&a[2],out).unwrap();println!("{} {}",n,items);}
'''

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra')
    p.add_argument('--source-commit',required=True)
    p.add_argument('--output',type=Path,required=True)
    args=p.parse_args()
    game=Elf(args.assets/'libGameCore_XP2.so',GAME_SHA)
    Elf(args.assets/'Civ6',HOST_SHA)
    for a,n,name,digest in METHODS:
        if sha(game.read(a,n))!=digest:raise ValueError('method mismatch: '+name)
    if struct.unpack('<I',game.read(0x2f6b014,4))[0]!=1:raise ValueError('version mismatch')
    for address,body in VIRTUALS:
        if game.relocated_pointer(address)!=body:raise ValueError("virtual save/load binding mismatch")
    relocations=relocation_symbols(game)
    for site,plt,body,kind in BINDINGS:
        code=game.read(site,5)
        if code[0]!=(0xe8 if kind=='call' else 0xe9) or site+5+struct.unpack_from('<i',code,1)[0]!=plt:raise ValueError('direct binding mismatch')
        stub=game.read(plt,6)
        if stub[:2]!=b'\xff\x25' or relocations.get(plt+6+struct.unpack_from('<i',stub,2)[0])!=body:raise ValueError('PLT binding mismatch')
    dwarf=''.join(subprocess.check_output(['gdb','-nx','-batch',str(args.assets/'libGameCore_XP2.so'),'-ex',"ptype /o '"+name+"'"],text=True)for name in ['GameCore::Player::WMDs'])
    if sha(dwarf.encode())!=DWARF_SHA:raise ValueError('DWARF mismatch')
    pin=subprocess.check_output(['git','rev-parse',args.source_commit+'^{commit}'],cwd=ROOT,text=True).strip()
    reports=[]
    with tempfile.TemporaryDirectory(prefix='c6-wmds-') as tmp:
        tmp=Path(tmp);source=tmp/'source';source.mkdir()
        archive=subprocess.check_output(['git','archive',pin],cwd=ROOT)
        subprocess.run(['tar','-x','-C',str(source)],input=archive,check=True)
        if (source/'tools/linux/player_wmds_audit.py').read_bytes()!=Path(__file__).read_bytes():raise ValueError('driver must be committed with candidate')
        paths=[source/'Cargo.toml',source/'Cargo.lock',source/'crates/c6-core/Cargo.toml',*sorted((source/'crates/c6-core/src').rglob('*.rs')),source/'tools/linux/player_wmds_audit.py']
        sources={str(x.relative_to(source)):sha(x.read_bytes())for x in paths}
        subprocess.run(['cargo','build','--offline','--release','-p','c6-core'],cwd=source,check=True)
        harness=HARNESS.replace('MODULE',str(source/'crates/c6-core/src/player_wmds.rs'))
        (tmp/'candidate.rs').write_text(harness)
        cmd=['rustc','--edition=2021','-Dwarnings','-O',str(tmp/'candidate.rs'),'--extern','c6_core='+str(source/'target/release/libc6_core.rlib'),'-L','dependency='+str(source/'target/release/deps')]
        subprocess.run(cmd+['-o',str(tmp/'candidate')],check=True)
        subprocess.run(cmd+['--test','-o',str(tmp/'tests')],check=True)
        subprocess.run([str(tmp/'tests')],check=True)
        candidate_sha=sha((tmp/'candidate').read_bytes())
        for identity,trace_name,trace_sha in FIXTURES:
            data=gzip.decompress((args.private_root/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes())
            trace=gzip.decompress((args.private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes())
            if sha(data)!=identity or sha(trace)!=trace_sha:raise ValueError('fixture identity mismatch')
            cuts=sorted([x for x in map(json.loads,trace.splitlines())if x.get('event')=='owner' and x.get('save')==1 and x.get('owner')=='GameCore::Player::WMDs'],key=lambda x:x['start'])
            if len(cuts)!=43:raise ValueError('record count mismatch')
            observations=[];digests=[]
            for cut in cuts:
                payload=data[cut['start']:cut['end']];(tmp/'in').write_bytes(payload)
                words=[int(x)for x in subprocess.check_output([str(tmp/'candidate'),str(tmp/'in'),str(tmp/'out')],text=True).split()]
                if (tmp/'out').read_bytes()!=payload or words[0]!=len(payload):raise ValueError('typed comparison mismatch')
                observations.append(words);digests.append(sha(payload))
            reports.append(dict(stream_sha256=identity,trace_sha256=trace_sha,records=len(cuts),record_bytes=sum(x[0]for x in observations),record_items=sum(x[1]for x in observations),first_observed_range=[cuts[0]['start'],cuts[0]['end']],byte_item_caps=sorted(set(tuple(x)for x in observations)),ordered_record_digests_sha256=sha(json.dumps(digests,separators=(',',':')).encode())))
    report=dict(schema='c6-linux-player-wmds-v1',checks_passed=True,gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,current_version=1,version_address='0x2f6b014',methods=[dict(address=hex(a),size=n,name=name,sha256=h)for a,n,name,h in METHODS],concrete_bindings=[dict(site=hex(a),plt=hex(b),body=hex(c),kind=k)for a,b,c,k in BINDINGS],virtual_bindings=[dict(address=hex(a),body=hex(b))for a,b in VIRTUALS],dwarf_sha256=DWARF_SHA,candidate_source_commit=pin,candidate_archive_sha256=sha(archive),candidate_sources=sources,candidate_sha256=candidate_sha,harness_sha256=sha(harness.encode()),audit_source_sha256=sha(Path(__file__).read_bytes()),fixtures=reports,source_only_tests=2,exact_native_record_byte_item_caps_checked=True,one_short_native_record_byte_caps_rejected=True,one_short_native_record_item_caps_rejected=True,native_every_truncation_rejected=True,validation_cuts='WMDs owner cuts only; production codec has no offsets or search inputs',allocation_policy='one item per ordered hash/count pair; unsigned count and full wire extent checked before fallible typed reservation; encode preflights exact bytes/items before output reservation',original_execution=False,private_writes=False,complete_current_wmds_wire=True,legacy_versions=False,definition_remapping=False,native_read_fixups=False,native_current_read_calls_initialize=False,native_current_read_calls_reset=False,native_current_read_reset_reproduced=False,native_current_read_callbacks_reproduced=False,full_player=False,world_import=False,gameplay_state=False,actions=False,turns=False,next_player_component='Improvements',unknown_save_bytes_masked=False,recorded_raw_save_load_difference_bytes=89198,attribution_reference_revision='5af389445118bed2bdebf7a8a42b219c8fba5175',reference_role='discovery_only')
    args.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(checks_passed=True,fixtures=reports)))

if __name__=='__main__':main()
