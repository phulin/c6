#!/usr/bin/env python3
"""SHA-gated read-only Governors contracts and isolated committed-source wire audit.

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
METHODS = [[35019072, 34, 'Serialization::IStream& Serialization::ReadVersioned<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0>, char const (&) [1]>(Serialization::IStream&, String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0>&, int, int&&, char const (&) [1])', 'f6169cab97413d50c95797565444d5cd98efec9176f180d529234e49820ffe7c'], [35019024, 34, 'Serialization::IStream& Serialization::ReadVersioned<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0>, char const (&) [5]>(Serialization::IStream&, String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0>&, int, int&&, char const (&) [5])', 'ecee758465e74945f179622d7e8b2d9492687dc6c80b2e48d665a54208792d16'], [44943888, 34, 'Serialization::IStream& Serialization::ReadVersioned<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0>, char const (&) [8]>(Serialization::IStream&, String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0>&, int, int&&, char const (&) [8])', '0fc402543610cdf35e5a1a282cf6a48b5aeee6becfef77d787f62f54ea494bfc'], [43913728, 34, 'Serialization::IStream& Serialization::ReadVersioned<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0>, String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0>&>(Serialization::IStream&, String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0>&, int, int&&, String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0>&)', 'd680251f57e656c4d0229dbedca40ef6c4ce9b415fe305b1c6f221159052cfc7'], [34759840, 34, 'Serialization::IStream& Serialization::ReadVersioned<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0>, String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> >(Serialization::IStream&, String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0>&, int, int&&, String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0>&&)', '682fda9a4ef5cd7925c1181b29cfe8892a4342d53edf47fe96c56e16e3f9f124'], [44269824, 46, 'Serialization::SerializeFromSequenceContainer<GameCore::Governor::GrievenceScoreModifier const, eastl::vector<GameCore::Governor::GrievenceScoreModifier, eastl::allocator> const>::SerializeFromSequenceContainer(Serialization::OStream&, eastl::vector<GameCore::Governor::GrievenceScoreModifier, eastl::allocator> const&)', '833dfc618f4adca76788ff91e62891bcda8ff07a0fc4f058bd014bf6eea9f90e'], [44269872, 8, 'Serialization::SerializeFromSequenceContainer<GameCore::Governor::GrievenceScoreModifier const, eastl::vector<GameCore::Governor::GrievenceScoreModifier, eastl::allocator> const>::operator()(GameCore::Governor::GrievenceScoreModifier const&)', '1263e37fb626c8a7e5c8b67659887e1cf2f246bd0f864e00fbcb3c2159648b3d'], [46099632, 36, 'Serialization::operator<<(Serialization::OStream&, GameCore::GovernorTypes const&)', '85e3d6cd7993fdbb0d639e73a63d1e92f64869f67decb46b65d73b3cb86ef7d6'], [46771472, 95, 'Serialization::operator<<(Serialization::OStream&, GameCore::Player::CityID const&)', 'a0462572e6b0309d6de60c35fd14f41c3cb054c915751ab1103d279b6f0d981c'], [45126240, 123, 'Serialization::operator<<(Serialization::OStream&, GameCore::Player::Governors const&)', '6bd26410d3012175689612ad8cb47c498d3d4a92c5d7970381cc9986f0cc822c'], [44264112, 59, 'Serialization::operator<<(Serialization::OStream&, GameCore::Governor::GrievenceScoreModifier const&)', 'd21da44f2664627f796fb59d349d7eaa66d6c072f7316d7c022ce425612fd92c'], [44262448, 554, 'Serialization::operator<<(Serialization::OStream&, GameCore::Governor::Instance const&)', 'f6497ac1155d6af26a3444bf7993634ec1a0cd93115f23aa2619ed47d23316a6'], [45141296, 374, 'Serialization::OStream& Serialization::operator<< <GameCore::Governor::Instance, 8u, 16u>(Serialization::OStream&, RecycledIDContainer<GameCore::Governor::Instance, 8u, 16u> const&)', '6f0af8eab748c6ca97e7a05015cc02f5e59a73e1be4388b2acdcc8947c3aa600'], [33510448, 81, 'Serialization::OStream& Serialization::operator<< <Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0>(Serialization::OStream&, String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const&)', '045813e437c3874ae9899508e422a1126cbd16d136f2af8ce3eff6603cb99cca'], [46099680, 58, 'Serialization::operator>>(Serialization::IStream&, GameCore::GovernorTypes&)', '9f00443af04e45479cf917be9818146c4f5cdc72c6b2cea94b3c1091fe257e30'], [46771568, 76, 'Serialization::operator>>(Serialization::IStream&, GameCore::Player::CityID&)', '95c0f2a3adabb22a1ccc987faac9ec466d51c1c76898180eef6caa3a2d279e07'], [45126368, 291, 'Serialization::operator>>(Serialization::IStream&, GameCore::Player::Governors&)', 'd6c99c51dc17a8f372f1ac4f3bdf039d6becde2ca9a5b3e55aa40e612007c7d7'], [44264176, 57, 'Serialization::operator>>(Serialization::IStream&, GameCore::Governor::GrievenceScoreModifier&)', '58881bf301df5a8bec8e9150e1bb26a9b91f88b8aa1cda2f9efdc6c975cc4eff'], [44263008, 1099, 'Serialization::operator>>(Serialization::IStream&, GameCore::Governor::Instance&)', '43693f2472056dbe1fcfd0cc1952153da0a7af22b132c56cce8e97c0891f5bd9'], [45141824, 386, 'Serialization::IStream& Serialization::operator>><GameCore::Governor::Instance, 8u, 16u>(Serialization::IStream&, RecycledIDContainer<GameCore::Governor::Instance, 8u, 16u>&)', 'bc2018fa8e26c9138c9578f17e6091cc53d308ed3612badc6d1383ccfd46cd32'], [33508640, 298, 'Serialization::IStream& Serialization::operator>><Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0>(Serialization::IStream&, String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0>&)', '7bef00c45e9a3eccd9989b48d1c8dbfe0b5ee20adb2ce7c5922995b76d6172af'], [44269744, 71, 'Serialization::SerializeFromSequenceContainer<GameCore::Governor::GrievenceScoreModifier const, eastl::vector<GameCore::Governor::GrievenceScoreModifier, eastl::allocator> const> eastl::for_each<GameCore::Governor::GrievenceScoreModifier const*, Serialization::SerializeFromSequenceContainer<GameCore::Governor::GrievenceScoreModifier const, eastl::vector<GameCore::Governor::GrievenceScoreModifier, eastl::allocator> const> >(GameCore::Governor::GrievenceScoreModifier const*, GameCore::Governor::GrievenceScoreModifier const*, Serialization::SerializeFromSequenceContainer<GameCore::Governor::GrievenceScoreModifier const, eastl::vector<GameCore::Governor::GrievenceScoreModifier, eastl::allocator> const>)', 'b682c129682d9181937d8e908deda12e3f2231202a62ef8b02694e2f57f83865']]
BINDINGS = [[35019086, 31939168, 0, 'call'], [35019096, 32309680, 33508640, 'call'], [35019038, 31939168, 0, 'call'], [35019048, 32309680, 33508640, 'call'], [44943902, 31939168, 0, 'call'], [44943912, 32309680, 33508640, 'call'], [43913742, 32277312, 0, 'call'], [43913752, 32309680, 33508640, 'call'], [34759854, 32975664, 0, 'call'], [34759864, 32309680, 33508640, 'call'], [44269842, 31854992, 44269888, 'call'], [44269859, 31588976, 33341776, 'call'], [44269875, 32502224, 44264112, 'jmp'], [46099640, 32859488, 35732816, 'call'], [46099653, 32424816, 34505104, 'call'], [46771485, 32732160, 33450864, 'call'], [46771503, 32522480, 34884432, 'call'], [46771511, 32847680, 35023648, 'call'], [46771526, 32522480, 34884432, 'call'], [46771534, 32984848, 33362048, 'call'], [46771551, 31588976, 33341776, 'call'], [45126257, 32898000, 33341744, 'call'], [45126272, 32749728, 45141264, 'call'], [45126284, 31802896, 45141296, 'call'], [45126299, 32749728, 45141264, 'call'], [45126314, 32963072, 45141680, 'call'], [45126329, 32629696, 45141712, 'call'], [45126347, 32749728, 45141264, 'call'], [44264129, 32898000, 33341744, 'call'], [44264140, 32898000, 33341744, 'call'], [44264155, 32898000, 33341744, 'call'], [44262474, 32898000, 33341744, 'call'], [44262485, 32898000, 33341744, 'call'], [44262497, 33247344, 46099632, 'call'], [44262509, 32498736, 33510448, 'call'], [44262521, 32190240, 46771472, 'call'], [44262533, 32035680, 46771104, 'call'], [44262545, 32898000, 33341744, 'call'], [44262557, 32898000, 33341744, 'call'], [44262569, 32898000, 33341744, 'call'], [44262581, 32898000, 33341744, 'call'], [44262593, 32898000, 33341744, 'call'], [44262605, 32898000, 33341744, 'call'], [44262617, 31462048, 33431440, 'call'], [44262632, 32898000, 33341744, 'call'], [44262647, 32898000, 33341744, 'call'], [44262662, 32898000, 33341744, 'call'], [44262677, 32898000, 33341744, 'call'], [44262692, 32898000, 33341744, 'call'], [44262707, 32898000, 33341744, 'call'], [44262722, 32898000, 33341744, 'call'], [44262737, 31462048, 33431440, 'call'], [44262752, 32898000, 33341744, 'call'], [44262767, 32898000, 33341744, 'call'], [44262782, 32898000, 33341744, 'call'], [44262794, 31812016, 44267776, 'call'], [44262808, 31588976, 33341776, 'call'], [44262816, 32251632, 44267792, 'call'], [44262828, 32396496, 44267824, 'call'], [44262846, 32767168, 44267856, 'call'], [44262867, 32635744, 44267872, 'call'], [44262881, 32190240, 46771472, 'call'], [44262893, 32898000, 33341744, 'call'], [44262901, 31821744, 44267888, 'call'], [44262912, 32767168, 44267856, 'call'], [44262925, 32315536, 33567344, 'call'], [44262933, 31750416, 35732880, 'call'], [44262947, 32396256, 44267920, 'call'], [44262962, 31462048, 33431440, 'call'], [44262980, 31501184, 44268064, 'call'], [45141323, 31588976, 33341776, 'call'], [45141344, 31588976, 33341776, 'call'], [45141352, 32080640, 45145296, 'call'], [45141369, 31588976, 33341776, 'call'], [45141377, 32529328, 43023312, 'call'], [45141394, 31588976, 33341776, 'call'], [45141415, 32102800, 45145312, 'call'], [45141473, 32898000, 33341744, 'call'], [45141484, 31613472, 44262448, 'call'], [45141495, 31712272, 45145328, 'call'], [45141519, 31588976, 33341776, 'call'], [45141527, 31561584, 45145424, 'call'], [45141544, 31588976, 33341776, 'call'], [45141556, 31588976, 33341776, 'call'], [45141568, 31588976, 33341776, 'call'], [45141584, 32747344, 45145456, 'call'], [45141622, 31588976, 33341776, 'call'], [45141634, 32637648, 45145472, 'call'], [33510466, 31667600, 0, 'call'], [33510483, 31588976, 33341776, 'call'], [33510498, 33115616, 0, 'call'], [46099699, 31815744, 33341888, 'call'], [46099708, 32859488, 35732816, 'call'], [46099718, 32246144, 34512640, 'call'], [46771583, 32697920, 34883984, 'call'], [46771594, 32697920, 34883984, 'call'], [46771607, 31815744, 33341888, 'call'], [46771628, 31669824, 46772480, 'call'], [45126386, 31938512, 33341856, 'call'], [45126406, 31938512, 33341856, 'call'], [45126414, 32069488, 45141744, 'call'], [45126426, 31949760, 45141824, 'call'], [45126434, 32069488, 45141744, 'call'], [45126451, 33253152, 45142224, 'call'], [45126463, 31949760, 45141824, 'call'], [45126508, 32742192, 45142256, 'call'], [45126550, 33119536, 45142304, 'call'], [45126595, 33122320, 45142352, 'call'], [45126643, 32742192, 45142256, 'call'], [44264191, 31938512, 33341856, 'call'], [44264202, 31938512, 33341856, 'call'], [44264217, 31938512, 33341856, 'call'], [44263033, 31938512, 33341856, 'call'], [44263044, 31938512, 33341856, 'call'], [44263056, 32309904, 46099680, 'call'], [44263068, 32309680, 33508640, 'call'], [44263080, 32512544, 46771568, 'call'], [44263092, 33204544, 46771152, 'call'], [44263104, 31938512, 33341856, 'call'], [44263116, 31938512, 33341856, 'call'], [44263158, 33253312, 33563776, 'call'], [44263200, 33253312, 33563776, 'call'], [44263242, 33253312, 33563776, 'call'], [44263284, 33253312, 33563776, 'call'], [44263323, 31828752, 33447552, 'call'], [44263368, 33253312, 33563776, 'call'], [44263413, 33253312, 33563776, 'call'], [44263458, 33253312, 33563776, 'call'], [44263503, 33253312, 33563776, 'call'], [44263548, 33253312, 33563776, 'call'], [44263593, 33253312, 33563776, 'call'], [44263638, 33253312, 33563776, 'call'], [44263680, 31828752, 33447552, 'call'], [44263725, 33253312, 33563776, 'call'], [44263770, 33253312, 33563776, 'call'], [44263815, 33253312, 33563776, 'call'], [44263840, 31938512, 33341856, 'call'], [44263891, 31866560, 33564704, 'call'], [44263902, 32512544, 46771568, 'call'], [44263913, 31938512, 33341856, 'call'], [44263929, 31542416, 44268160, 'call'], [44263955, 32137760, 33566672, 'call'], [44263963, 31750416, 35732880, 'call'], [44264004, 33172144, 44268368, 'call'], [44264046, 31828752, 33447552, 'call'], [44264084, 31717504, 44268496, 'call'], [45141855, 31815744, 33341888, 'call'], [45141879, 31815744, 33341888, 'call'], [45141900, 31815744, 33341888, 'call'], [45141908, 32080640, 45145296, 'call'], [45141921, 32874208, 45145632, 'call'], [45141932, 32953232, 45145728, 'call'], [45141945, 31815744, 33341888, 'call'], [45141990, 31938512, 33341856, 'call'], [45142002, 32908528, 45145824, 'call'], [45142013, 32152352, 44263008, 'call'], [45142048, 31815744, 33341888, 'call'], [45142075, 31815744, 33341888, 'call'], [45142087, 31815744, 33341888, 'call'], [45142099, 31815744, 33341888, 'call'], [45142134, 31815744, 33341888, 'call'], [45142149, 32141488, 45146384, 'call'], [45142182, 32141488, 45146384, 'call'], [33508667, 31815744, 33341888, 'call'], [33508737, 31471216, 33348384, 'call'], [33508775, 31949952, 0, 'call'], [33508789, 32474528, 33509408, 'call'], [33508800, 31703344, 33509472, 'call'], [33508824, 31471216, 33348384, 'call'], [33508835, 31793744, 33509584, 'call'], [33508870, 31793744, 33509584, 'call'], [33508884, 31949952, 0, 'call'], [33508893, 32157808, 33509600, 'call'], [33508925, 32157808, 33509600, 'call'], [33508933, 32992576, 0, 'call'], [44269782, 32630816, 44269872, 'call'], [45412753, 33156432, 45126240, 'call']]
VIRTUALS = []
DWARF_SHA = '709a2d896aa3be48ae8da50c3f793012527d40bed4326b29d477e4ffe5d1bb2c'

HARNESS = r'''pub use c6_core::unit;
#[path="MODULE"] pub mod player_governors;
fn main(){let a:Vec<_>=std::env::args().collect();let b=std::fs::read(&a[1]).unwrap();let generous=unit::Limits{max_bytes:b.len(),max_items:b.len()};let(v,n,items)=player_governors::Governors::decode_prefix_with_items(&b,generous).unwrap();assert!(n<=b.len());let record=&b[..n];let exact=unit::Limits{max_bytes:n,max_items:items};let(out,wi)=v.encode_with_items(exact).unwrap();assert_eq!(wi,items);assert_eq!(out,record);assert_eq!(v,player_governors::Governors::decode(record,exact).unwrap());for end in 0..n{assert!(player_governors::Governors::decode(&b[..end],exact).is_err());}let mut caps=vec![unit::Limits{max_bytes:n-1,..exact}];if items>0{caps.push(unit::Limits{max_items:items-1,..exact});}for lim in caps{assert!(player_governors::Governors::decode(record,lim).is_err());assert!(v.encode(lim).is_err());}std::fs::write(&a[2],out).unwrap();println!("{} {}",n,items);}
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
    for address,version in [(0x2f61330,6),(0x2f551cc,16),(0x2f551d0,1),(0x2f61360,65537)]:
        if struct.unpack('<I',game.read(address,4))[0]!=version:raise ValueError('version mismatch')
    for address,body in VIRTUALS:
        if game.relocated_pointer(address)!=body:raise ValueError("virtual save/load binding mismatch")
    relocations=relocation_symbols(game)
    for site,plt,body,kind in BINDINGS:
        code=game.read(site,5)
        if code[0]!=(0xe8 if kind=='call' else 0xe9) or site+5+struct.unpack_from('<i',code,1)[0]!=plt:raise ValueError('direct binding mismatch')
        stub=game.read(plt,6)
        if stub[:2]!=b'\xff\x25' or relocations.get(plt+6+struct.unpack_from('<i',stub,2)[0])!=body:raise ValueError('PLT binding mismatch')
    dwarf=''.join(subprocess.check_output(['gdb','-nx','-batch',str(args.assets/'libGameCore_XP2.so'),'-ex',"ptype /o '"+name+"'"],text=True)for name in ['GameCore::Player::Governors','GameCore::Governor::Instance','GameCore::Governor::GrievenceScoreModifier'])
    if sha(dwarf.encode())!=DWARF_SHA:raise ValueError('DWARF mismatch')
    pin=subprocess.check_output(['git','rev-parse',args.source_commit+'^{commit}'],cwd=ROOT,text=True).strip()
    reports=[]
    with tempfile.TemporaryDirectory(prefix='c6-governors-') as tmp:
        tmp=Path(tmp);source=tmp/'source';source.mkdir()
        archive=subprocess.check_output(['git','archive',pin],cwd=ROOT)
        subprocess.run(['tar','-x','-C',str(source)],input=archive,check=True)
        if (source/'tools/linux/player_governors_audit.py').read_bytes()!=Path(__file__).read_bytes():raise ValueError('driver must be committed with candidate')
        paths=[source/'Cargo.toml',source/'Cargo.lock',source/'crates/c6-core/Cargo.toml',*sorted((source/'crates/c6-core/src').rglob('*.rs')),source/'tools/linux/player_governors_audit.py']
        sources={str(x.relative_to(source)):sha(x.read_bytes())for x in paths}
        subprocess.run(['cargo','clippy','--offline','-p','c6-core','--all-targets','--','-D','warnings'],cwd=source,check=True)
        subprocess.run(['cargo','build','--offline','--release','-p','c6-core'],cwd=source,check=True)
        harness=HARNESS.replace('MODULE',str(source/'crates/c6-core/src/player_governors.rs'))
        (tmp/'candidate.rs').write_text(harness)
        cmd=['rustc','--edition=2021','-Dwarnings','-O',str(tmp/'candidate.rs'),'--extern','c6_core='+str(source/'target/release/libc6_core.rlib'),'-L','dependency='+str(source/'target/release/deps')]
        subprocess.run(cmd+['-o',str(tmp/'candidate')],check=True)
        manifest = '[package]\nname="c6-player-governors-audit"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[dependencies]\nc6-core={path=' + json.dumps(str(source/'crates/c6-core')) + '}\n[[bin]]\nname="candidate"\npath="candidate.rs"\n'
        (tmp/'Cargo.toml').write_text(manifest)
        subprocess.run(['cargo','test','--offline','--manifest-path',str(tmp/'Cargo.toml')],check=True)
        subprocess.run(['cargo','clippy','--offline','--manifest-path',str(tmp/'Cargo.toml'),'--all-targets','--','-D','warnings'],check=True)
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
                expected=words[0]
                if (tmp/'out').read_bytes()!=payload[:expected]:raise ValueError('typed comparison mismatch')
                observations.append(words);digests.append(sha(payload[:expected]));ranges.append([start,start+expected])
            reports.append(dict(stream_sha256=identity,trace_sha256=trace_sha,records=len(cuts),record_bytes=sum(x[0]for x in observations),record_items=sum(x[1]for x in observations),first_observed_range=ranges[0],byte_item_caps=sorted(set(tuple(x)for x in observations)),ordered_record_digests_sha256=sha(json.dumps(digests,separators=(',',':')).encode())))
    report=dict(schema='c6-linux-player-governors-v1',checks_passed=True,gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,current_version=6,instance_version=16,grievance_version=1,recycled_version=65537,methods=[dict(address=hex(a),size=n,name=name,sha256=h)for a,n,name,h in METHODS],concrete_bindings=[dict(site=hex(a),plt=hex(b),body=hex(c),kind=k)for a,b,c,k in BINDINGS],dwarf_sha256=DWARF_SHA,candidate_source_commit=pin,candidate_archive_sha256=sha(archive),candidate_sources=sources,candidate_sha256=candidate_sha,harness_sha256=sha(harness.encode()),audit_source_sha256=sha(Path(__file__).read_bytes()),fixtures=reports,source_only_tests=3,exact_native_record_byte_item_caps_checked=True,one_short_native_record_byte_caps_rejected=True,one_short_nonempty_native_record_item_caps_rejected=True,native_every_truncation_rejected=True,validation_cuts='No Governors owner cuts. Source-closed WMD and Improvements counts locate prefix from Quests.end; enclosing Player.end is contextual bound. Comparison uses original full records. No production offsets/traces.',allocation_policy='Aggregate list entries and encoding-0 bytes count as items; full no-allocation scan checks every nested extent and cumulative typed storage <= isize::MAX before fallible reservations. Encoder measures complete wire/items/storage before output allocation. Capacity is preserved metadata, never an allocation request.',original_execution=False,private_writes=False,complete_current_governors_wire=True,legacy_versions=False,definition_remapping=False,native_read_fixups=False,native_read_effects='Recycled reader may destroy/reserve and AddAt, reconstructs free links; Governor reader writes definition-remapped types/promotions and inserts city-map entries. These effects are not executed by wire codec.',full_player=False,world_import=False,gameplay_state=False,actions=False,turns=False,next_player_component='inspect Player serializer successor of Governors',unknown_save_bytes_masked=False,recorded_raw_save_load_difference_bytes=89198,attribution_reference_revision='5af389445118bed2bdebf7a8a42b219c8fba5175',reference_role='discovery_only')
    args.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(checks_passed=True,fixtures=reports)))

if __name__=='__main__':main()
