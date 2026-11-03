#!/usr/bin/env python3
"""SHA-gated read-only CulturalIdentity contracts and isolated committed-source wire audit.

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
METHODS = [[44479664, 246, 'Serialization::operator<<(Serialization::OStream&, GameCore::Player::CulturalIdentity const&)', '8849206344d4f3951e6b134e765c6c7bb96769fccb6124d35a0d58fccdb1ae6b'], [33341744, 14, 'Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)', '2a3fc110a86f71122b45f03f192ddbc4499241d298e49b843ea750bd49d5e1cf'], [33348288, 14, 'Serialization::OStream::WriteObject(int const&)', '0baf2c125cf20f28f26a56bc061ccbb70dadd2a825949e44932986634bbea10c'], [44483536, 21, 'Serialization::OStream& operator<< <int, GameCore::Player::CulturalIdentity>(Serialization::OStream&, FAutoVariable<int, GameCore::Player::CulturalIdentity> const&)', '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'], [44483568, 21, 'Serialization::OStream& operator<< <FixedPointT<8u>, GameCore::Player::CulturalIdentity>(Serialization::OStream&, FAutoVariable<FixedPointT<8u>, GameCore::Player::CulturalIdentity> const&)', '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'], [44479920, 645, 'Serialization::operator>>(Serialization::IStream&, GameCore::Player::CulturalIdentity&)', '65df38fd94743fd4ea16036ac54b227d73e3803e18d3ec277892fad926e0c9cd'], [33341856, 14, 'Serialization::IStream& Serialization::IStream::operator>><int>(int&)', 'c6cf9d3175d5134bd8d7b3d396184fd7b9abd3f7ca364cbdc38a1794ce742775'], [33348304, 75, 'Serialization::IStream::ReadObject(int&)', '09ddfd4d815a2204789075821ff06a3e8d881dc2bae4791ff3f746206deb3230'], [33348384, 25, 'Serialization::IStream::ReadRequest::ReadRequest(unsigned long, unsigned long)', '43f291f25ae0e59e98bf6b96e5341ed0345d1627c75c056bac01ada2cfd2f796'], [33348416, 18, 'fastdelegate::FastDelegate1<Platform::MemoryBuffer&, void>::FastDelegate1()', '4e8b03911ca2343c3a991ee6ca0e016459940dd311827a466245aaa71c1b1f1e'], [33348448, 5, 'fastdelegate::detail::ClosurePtr<void (fastdelegate::detail::GenericClass::*)(Platform::MemoryBuffer&), void (*)(Platform::MemoryBuffer&), void (*)(Platform::MemoryBuffer&)>::ClosurePtr()', 'c95a5b9c255a854a133eed1caf8eec628aa4a808998bc832507859fe333b1022'], [33348480, 15, 'fastdelegate::DelegateMemento::DelegateMemento()', '8d239b62e5d2e8dd82c2a8028f2cccf6b4a732111f049ae62935198342dfa72b'], [33348464, 5, 'fastdelegate::FastDelegate1<Platform::MemoryBuffer&, void>::clear()', 'b5710e855cde244bc34ae23da361977603b7c3777904a93dd5a56c0c3be86398'], [33348496, 15, 'fastdelegate::DelegateMemento::clear()', '8d239b62e5d2e8dd82c2a8028f2cccf6b4a732111f049ae62935198342dfa72b'], [44483600, 21, 'Serialization::IStream& operator>><int, GameCore::Player::CulturalIdentity>(Serialization::IStream&, FAutoVariable<int, GameCore::Player::CulturalIdentity>&)', '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55'], [42331456, 36, 'Serialization::IStream& Serialization::ReadVersioned<FixedPointT<8u>, int>(Serialization::IStream&, FixedPointT<8u>&, int, int&&, int&&)', '3523338d40d65af7b912247d0f6c3c40fa502332c8ed14e65366fe93765463ff'], [33348624, 9, 'FixedPointT<8u>::operator=(int)', '289ee59e91d4f68ddbe5f570b5002092702cd7aae484d25f21ca0576db55b96f'], [46772240, 43, 'Serialization::operator>>(Serialization::IStream&, FixedPointT<8u>&)', '5d47fcdfb3868524b3d8da8a2fab95de165678efa05f23c615963da12ddf010d'], [35393568, 3, 'FixedPointT<8u>::SetRaw(int)', 'e57f6082d61fbfc2f1375ebe719921bae81e81ff55cb3522867874eaaf77ff16'], [44483696, 34, 'Serialization::IStream& Serialization::ReadVersioned<FAutoVariable<int, GameCore::Player::CulturalIdentity>, int>(Serialization::IStream&, FAutoVariable<int, GameCore::Player::CulturalIdentity>&, int, int&&, int&&)', 'df0e59ce900587e4dad74df344ea0febd39b0938c0f09ae2796766c172a19a6a'], [37995664, 14, 'FAutoVariable<int, GameCore::Player::CulturalIdentity>::operator=(int&&)', 'ec07399b816920455fad9d4183a6cafa0f7b7d161e8c4e9f452a79ac8ecd4f0d'], [37995680, 5, 'void MoveAssignment<int, FAutoVariable<int, GameCore::Player::CulturalIdentity> >(FAutoVariable<int, GameCore::Player::CulturalIdentity>&, int&&)', '44f8f707d1973441cdb7254124b08bacae30a0eddfc413af5937e0fdb696bfe9'], [37995696, 52, 'void MoveAssignmentDo<int, FAutoVariable<int, GameCore::Player::CulturalIdentity> >(FAutoVariable<int, GameCore::Player::CulturalIdentity>&, int&&, std::__1::integral_constant<bool, true>)', 'fd1bb024f55adb291472f532d58531686b11c8c52628dacf2950d19327ed92df'], [33346560, 97, 'std::__1::pair<std::__1::__tree_iterator<FAutoVariableBase*, std::__1::__tree_node<FAutoVariableBase*, void*>*, long>, bool> std::__1::__tree<FAutoVariableBase*, std::__1::less<FAutoVariableBase*>, std::__1::allocator<FAutoVariableBase*> >::__emplace_unique_key_args<FAutoVariableBase*, FAutoVariableBase*>(FAutoVariableBase* const&, FAutoVariableBase*&&)', '6dcaf248881c7d6fab9cc61bae2a628baaf053a6dd817909ad792108fe9c5d1f'], [33346672, 136, 'std::__1::__tree_node_base<void*>*& std::__1::__tree<FAutoVariableBase*, std::__1::less<FAutoVariableBase*>, std::__1::allocator<FAutoVariableBase*> >::__find_equal<FAutoVariableBase*>(std::__1::__tree_end_node<std::__1::__tree_node_base<void*>*>*&, FAutoVariableBase* const&)', '57c12053a21da4341aa319f58c0afa1ab8d5a20831c00d7fd14c2793710832d0'], [33346944, 5, 'std::__1::__tree<FAutoVariableBase*, std::__1::less<FAutoVariableBase*>, std::__1::allocator<FAutoVariableBase*> >::__root_ptr() const', '337222f0640328de908dacc52ced92ff844fe6d4f480a84a6e4aa98bb839dc80'], [33346816, 55, 'std::__1::unique_ptr<std::__1::__tree_node<FAutoVariableBase*, void*>, std::__1::__tree_node_destructor<std::__1::allocator<std::__1::__tree_node<FAutoVariableBase*, void*> > > > std::__1::__tree<FAutoVariableBase*, std::__1::less<FAutoVariableBase*>, std::__1::allocator<FAutoVariableBase*> >::__construct_node<FAutoVariableBase*>(FAutoVariableBase*&&)', 'a2d6fdcfaaa075bd82058413535768ea27a25e7b657dc34db26929862a9f4c6b'], [33346880, 54, 'std::__1::__tree<FAutoVariableBase*, std::__1::less<FAutoVariableBase*>, std::__1::allocator<FAutoVariableBase*> >::__insert_node_at(std::__1::__tree_end_node<std::__1::__tree_node_base<void*>*>*, std::__1::__tree_node_base<void*>*&, std::__1::__tree_node_base<void*>*)', '28c39727a266ced60899033f320970c2ee3396f1489cf8602422244c149bf713'], [33346960, 192, 'void std::__1::__tree_balance_after_insert<std::__1::__tree_node_base<void*>*>(std::__1::__tree_node_base<void*>*, std::__1::__tree_node_base<void*>*)', '0ee87eaa10edf2b4ae811b74ff57bb997a26c1c1c0cbab83d2c6f1c28d46df31'], [33347216, 56, 'void std::__1::__tree_right_rotate<std::__1::__tree_node_base<void*>*>(std::__1::__tree_node_base<void*>*)', '3bc4eb432d8d4c537275b48fe4d30b21abb24c155165f7334344347427fbd768'], [33347152, 56, 'void std::__1::__tree_left_rotate<std::__1::__tree_node_base<void*>*>(std::__1::__tree_node_base<void*>*)', 'c47f0f3ac77cac87b3facadc65ee1f6b07ad6742272950c109bb2a7b1b390d51'], [44483744, 72, 'Serialization::IStream& Serialization::ReadVersioned<FAutoVariable<FixedPointT<8u>, GameCore::Player::CulturalIdentity>, int>(Serialization::IStream&, FAutoVariable<FixedPointT<8u>, GameCore::Player::CulturalIdentity>&, int, int&&, int&&)', 'd2832fa63ecf70aa17e4381d74d4f366d5883df12e06605e139744415ee95f51'], [33343664, 5, 'FixedPointT<8u>::FixedPointT(int)', 'a5050360463a3fe04639a9ad47d842d6a2c680138f54cfbbbe95eaf24f73e912'], [37997568, 14, 'FAutoVariable<FixedPointT<8u>, GameCore::Player::CulturalIdentity>::operator=(FixedPointT<8u>&&)', '7ddd723d9d18175887127b8763f84bec76e1f29d8ebe171ca81707baec3896cd'], [37997584, 5, 'void MoveAssignment<FixedPointT<8u>, FAutoVariable<FixedPointT<8u>, GameCore::Player::CulturalIdentity> >(FAutoVariable<FixedPointT<8u>, GameCore::Player::CulturalIdentity>&, FixedPointT<8u>&&)', '7cced8dc16d3ce892d20e1cf8e1557f65bc5a4d49164d594053649e491ca3b9e'], [37997600, 61, 'void MoveAssignmentDo<FixedPointT<8u>, FAutoVariable<FixedPointT<8u>, GameCore::Player::CulturalIdentity> >(FAutoVariable<FixedPointT<8u>, GameCore::Player::CulturalIdentity>&, FixedPointT<8u>&&, std::__1::integral_constant<bool, true>)', '4ed03fe950b4b4bff505f495edcd3429b17ef26143c59795ced2c7844bcbe047'], [33348608, 8, 'FixedPointT<8u>::operator=(FixedPointT<8u> const&)', '775143daff273e7ffd54aaf1fdd8ee35101783a6286e5c6a3d9b76fed694472a'], [44484208, 21, 'Serialization::IStream& operator>><FixedPointT<8u>, GameCore::Player::CulturalIdentity>(Serialization::IStream&, FAutoVariable<FixedPointT<8u>, GameCore::Player::CulturalIdentity>&)', '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55'], [44484880, 15, 'FAutoVariable<int, GameCore::Player::CulturalIdentity>::load(Serialization::IStream&)', 'c043d64ec33301559f00cdb203da412384090ebac9aabce9599d1925d388d71a'], [44484944, 15, 'FAutoVariable<int, GameCore::Player::CulturalIdentity>::save(Serialization::OStream&) const', '871e3489d9e5333819ef78d143b0ae00d168f5f34c3d07d37ada0c65eb2e2523'], [44487440, 15, 'FAutoVariable<FixedPointT<8u>, GameCore::Player::CulturalIdentity>::load(Serialization::IStream&)', 'e83a57b98da2353302a03f3818dfed9d9c3b1d997e197b4103566b1b74e875df'], [44487520, 15, 'FAutoVariable<FixedPointT<8u>, GameCore::Player::CulturalIdentity>::save(Serialization::OStream&) const', '8e6f693d04b55f94388d4da19d5153bbd57737aadac6911439612ffa193e621c'], [46772192, 42, 'Serialization::operator<<(Serialization::OStream&, FixedPointT<8u> const&)', 'bbd0553ff77e4be5a896850082327f68662e870164f4771d58a0e5c73c7d5704'], [46772496, 3, 'FixedPointT<8u>::GetRaw() const', 'cadd8f26469e7198cb603ea5ba1b165c25958dd0f2df803f0ade24798f5915ac']]
BINDINGS = [[44479681, 32898000, 33341744, 'call'], [33341748, 32120032, 33348288, 'call'], [44479696, 33191424, 44483536, 'call'], [44479711, 32345280, 44483568, 'call'], [44479726, 33191424, 44483536, 'call'], [44479741, 33191424, 44483536, 'call'], [44479756, 33191424, 44483536, 'call'], [44479771, 33191424, 44483536, 'call'], [44479786, 33191424, 44483536, 'call'], [44479801, 33191424, 44483536, 'call'], [44479816, 33191424, 44483536, 'call'], [44479831, 32345280, 44483568, 'call'], [44479846, 32345280, 44483568, 'call'], [44479861, 32345280, 44483568, 'call'], [44479876, 32345280, 44483568, 'call'], [44479894, 33191424, 44483536, 'call'], [44479938, 31938512, 33341856, 'call'], [33341860, 31737840, 33348304, 'call'], [33348335, 31471216, 33348384, 'call'], [33348404, 31796656, 33348416, 'jmp'], [33348420, 32773984, 33348448, 'call'], [33348448, 32218944, 33348480, 'jmp'], [33348429, 32763152, 33348464, 'jmp'], [33348464, 33209600, 33348496, 'jmp'], [44479953, 32407360, 44483600, 'call'], [44479965, 32446944, 44483632, 'call'], [44480006, 32910240, 42331456, 'call'], [42331472, 32747424, 33348624, 'call'], [42331482, 32963584, 46772240, 'call'], [46772255, 31938512, 33341856, 'call'], [46772267, 32345808, 35393568, 'call'], [44480051, 33074448, 44483696, 'call'], [44483710, 31546320, 37995664, 'call'], [37995668, 33033504, 37995680, 'call'], [37995680, 32273536, 37995696, 'jmp'], [37995729, 32193984, 33346560, 'call'], [33346584, 33165680, 33346672, 'call'], [33346694, 32309264, 33346944, 'call'], [33346615, 33142080, 33346816, 'call'], [33346839, 31925616, 0, 'call'], [33346635, 32744096, 33346880, 'call'], [33346922, 32850272, 33346960, 'call'], [33347080, 33105184, 33347216, 'call'], [33347105, 31473456, 33347152, 'jmp'], [33347122, 31473456, 33347152, 'call'], [33347147, 33105184, 33347216, 'jmp'], [44483720, 32407360, 44483600, 'call'], [44480096, 33074448, 44483696, 'call'], [44480141, 33074448, 44483696, 'call'], [44480186, 33074448, 44483696, 'call'], [44480231, 33074448, 44483696, 'call'], [44480276, 33074448, 44483696, 'call'], [44480321, 33074448, 44483696, 'call'], [44480366, 32832832, 44483744, 'call'], [44483774, 32860128, 33343664, 'call'], [33343664, 32747424, 33348624, 'jmp'], [44483785, 31643008, 37997568, 'call'], [37997572, 32698464, 37997584, 'call'], [37997584, 32230864, 37997600, 'jmp'], [37997633, 32193984, 33346560, 'call'], [37997648, 31649728, 33348608, 'call'], [44483798, 32120432, 44484208, 'call'], [44480411, 32832832, 44483744, 'call'], [44480456, 32832832, 44483744, 'call'], [44480501, 32832832, 44483744, 'call'], [44480549, 33074448, 44483696, 'call'], [44484890, 31938512, 33341856, 'jmp'], [44484954, 32898000, 33341744, 'jmp'], [44487450, 32963584, 46772240, 'jmp'], [44487530, 33099232, 46772192, 'jmp'], [46772203, 32683728, 46772496, 'call'], [46772220, 32898000, 33341744, 'call'], [45412772, 32400368, 44479664, 'call']]
VIRTUALS = [(0x3ad68c8,0x2a6c910),(0x3ad68e0,0x2a6c950),(0x3ad6988,0x2a6d310),(0x3ad69a0,0x2a6d360)]
DWARF_SHA = '34539c9c992b88776ebbeb6e78a262508f7413b2c4452440cfc873a07d3273ce'

HARNESS = r'''pub use c6_core::unit;
#[path="GOVERNORS"] pub mod player_governors;
#[path="MODULE"] pub mod player_cultural_identity;
fn main(){let a:Vec<_>=std::env::args().collect();let b=std::fs::read(&a[1]).unwrap();let generous=unit::Limits{max_bytes:b.len(),max_items:b.len()};let (_, skip)=player_governors::Governors::decode_prefix(&b,generous).unwrap();let b=&b[skip..];let(v,n,items)=player_cultural_identity::CulturalIdentity::decode_prefix_with_items(&b,generous).unwrap();assert!(n<=b.len());let record=&b[..n];let exact=unit::Limits{max_bytes:n,max_items:items};let(out,wi)=v.encode_with_items(exact).unwrap();assert_eq!(wi,items);assert_eq!(out,record);assert_eq!(v,player_cultural_identity::CulturalIdentity::decode(record,exact).unwrap());for end in 0..n{assert!(player_cultural_identity::CulturalIdentity::decode(&b[..end],exact).is_err());}let mut caps=vec![unit::Limits{max_bytes:n-1,..exact}];if items>0{caps.push(unit::Limits{max_items:items-1,..exact});}for lim in caps{assert!(player_cultural_identity::CulturalIdentity::decode(record,lim).is_err());assert!(v.encode(lim).is_err());}std::fs::write(&a[2],out).unwrap();println!("{} {} {}",n,items,skip);}
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
    for address,version in [(0x2f579f8,10)]:
        if struct.unpack('<I',game.read(address,4))[0]!=version:raise ValueError('version mismatch')
    for address,body in VIRTUALS:
        if game.relocated_pointer(address)!=body:raise ValueError("virtual save/load binding mismatch")
    relocations=relocation_symbols(game)
    for site,plt,body,kind in BINDINGS:
        code=game.read(site,5)
        if code[0]!=(0xe8 if kind=='call' else 0xe9) or site+5+struct.unpack_from('<i',code,1)[0]!=plt:raise ValueError('direct binding mismatch')
        stub=game.read(plt,6)
        if stub[:2]!=b'\xff\x25' or relocations.get(plt+6+struct.unpack_from('<i',stub,2)[0])!=body:raise ValueError('PLT binding mismatch')
    dwarf=''.join(subprocess.check_output(['gdb','-nx','-batch',str(args.assets/'libGameCore_XP2.so'),'-ex',"ptype /o '"+name+"'"],text=True)for name in ['GameCore::Player::CulturalIdentity'])
    if sha(dwarf.encode())!=DWARF_SHA:raise ValueError('DWARF mismatch')
    pin=subprocess.check_output(['git','rev-parse',args.source_commit+'^{commit}'],cwd=ROOT,text=True).strip()
    reports=[]
    with tempfile.TemporaryDirectory(prefix='c6-cultural-identity-') as tmp:
        tmp=Path(tmp);source=tmp/'source';source.mkdir()
        archive=subprocess.check_output(['git','archive',pin],cwd=ROOT)
        subprocess.run(['tar','-x','-C',str(source)],input=archive,check=True)
        if (source/'tools/linux/player_cultural_identity_audit.py').read_bytes()!=Path(__file__).read_bytes():raise ValueError('driver must be committed with candidate')
        paths=[source/'Cargo.toml',source/'Cargo.lock',source/'crates/c6-core/Cargo.toml',*sorted((source/'crates/c6-core/src').rglob('*.rs')),source/'tools/linux/player_cultural_identity_audit.py']
        sources={str(x.relative_to(source)):sha(x.read_bytes())for x in paths}
        subprocess.run(['cargo','clippy','--offline','-p','c6-core','--all-targets','--','-D','warnings'],cwd=source,check=True)
        subprocess.run(['cargo','build','--offline','--release','-p','c6-core'],cwd=source,check=True)
        harness=HARNESS.replace('MODULE',str(source/'crates/c6-core/src/player_cultural_identity.rs')).replace('GOVERNORS',str(source/'crates/c6-core/src/player_governors.rs'))
        (tmp/'candidate.rs').write_text(harness)
        cmd=['rustc','--edition=2021','-Dwarnings','-O',str(tmp/'candidate.rs'),'--extern','c6_core='+str(source/'target/release/libc6_core.rlib'),'-L','dependency='+str(source/'target/release/deps')]
        subprocess.run(cmd+['-o',str(tmp/'candidate')],check=True)
        subprocess.run(['cargo','test','--offline','-p','c6-core','player_cultural_identity::tests'],cwd=source,check=True)
        subprocess.run(cmd+['--test','-o',str(tmp/'tests')],check=True)
        subprocess.run([str(tmp/'tests')],check=True)
        candidate_sha=sha((tmp/'candidate').read_bytes())
        for identity,trace_name,trace_sha in FIXTURES:
            data=gzip.decompress((args.private_root/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes())
            trace=gzip.decompress((args.private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes())
            if sha(data)!=identity or sha(trace)!=trace_sha:raise ValueError('fixture identity mismatch')
            cuts=sorted([x for x in map(json.loads,trace.splitlines())if x.get('event')=='owner' and x.get('save')==1 and x.get('owner')=='GameCore::Player::Quests'],key=lambda x:x['start'])
            if len(cuts)!=43:raise ValueError('record count mismatch')
            owners={x['call']:x for x in map(json.loads,trace.splitlines()) if x.get('event')=='owner' and x.get('save')==1};observations=[];digests=[];ranges=[]
            for cut in cuts:
                parent=owners[cut['parent']];wmd_version,wmd_count=struct.unpack_from('<II',data,cut['end']);start=cut['end']+8+8*wmd_count
                if wmd_version!=1 or start+12>parent['end']:raise ValueError('invalid preceding WMD extent')
                improvement_version,improvement_count=struct.unpack_from('<II',data,start)
                if improvement_version!=1:raise ValueError('preceding Improvements version')
                start+=12+4*improvement_count
                payload=data[start:parent['end']];(tmp/'in').write_bytes(payload)
                if parent['owner']!='GameCore::Player::Instance':raise ValueError('wrong Player owner')
                words=[int(x)for x in subprocess.check_output([str(tmp/'candidate'),str(tmp/'in'),str(tmp/'out')],text=True).split()]
                start+=words[2];payload=payload[words[2]:];expected=words[0]
                if (tmp/'out').read_bytes()!=payload[:expected]:raise ValueError('typed comparison mismatch')
                observations.append(words);digests.append(sha(payload[:expected]));ranges.append([start,start+expected])
            reports.append(dict(stream_sha256=identity,trace_sha256=trace_sha,records=len(cuts),record_bytes=sum(x[0]for x in observations),record_items=sum(x[1]for x in observations),first_observed_range=ranges[0],byte_item_caps=sorted(set(tuple(x[:2])for x in observations)),ordered_record_digests_sha256=sha(json.dumps(digests,separators=(',',':')).encode())))
    report=dict(schema='c6-linux-player-cultural-identity-v1',checks_passed=True,gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,current_version=10,wire_bytes=60,items=0,methods=[dict(address=hex(a),size=n,name=name,sha256=h)for a,n,name,h in METHODS],concrete_bindings=BINDINGS,virtual_bindings=VIRTUALS,dwarf_sha256=DWARF_SHA,candidate_source_commit=pin,candidate_archive_sha256=sha(archive),candidate_sources=sources,candidate_sha256=candidate_sha,harness_sha256=sha(harness.encode()),audit_source_sha256=sha(Path(__file__).read_bytes()),fixtures=reports,source_only_tests=3,exact_native_record_byte_item_caps_checked=True,one_short_native_record_byte_caps_rejected=True,native_every_truncation_rejected=True,validation_cuts='No direct CulturalIdentity owner cuts. WMD, Improvements and complete Governors source grammars locate prefix from Quests.end; enclosing Player.end is contextual bound. No production trace offsets.',allocation_policy='Fixed 60-byte grammar, zero dynamic items. Decode allocates nothing; encode preflights byte cap then fallibly reserves exactly 60 bytes.',original_execution=False,private_writes=False,complete_current_cultural_identity_wire=True,legacy_versions=False,native_read_fixups=False,native_read_effects='Read invokes archive edit/virtual load wrappers; current version reads all members, Q8 via SetRaw. Codec does not edit native archives or initialize/reset parent.',full_player=False,world_import=False,gameplay_state=False,actions=False,turns=False,next_player_component='Heroes',unknown_save_bytes_masked=False,recorded_raw_save_load_difference_bytes=89198,attribution_reference_revision='5af389445118bed2bdebf7a8a42b219c8fba5175',reference_role='discovery_only')
    args.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(checks_passed=True,fixtures=reports)))

if __name__=='__main__':main()
