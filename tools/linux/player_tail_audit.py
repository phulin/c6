#!/usr/bin/env python3
"""SHA-gated static Linux Player tail contracts and isolated committed-source audit.
Never executes original code. Trace cuts are validation only; production takes a prefix.
"""
import argparse,gzip,hashlib,json,struct,subprocess,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
def sha(b): return hashlib.sha256(b).hexdigest()
GAME_SHA='a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa'
HOST_SHA='106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
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


METHODS = [[45388240, 15, 'FAutoVariable<Data::TypedVariantMap, GameCore::Player::Instance>::load(Serialization::IStream&)', '9754975ab7d08c84d97cf131f260f1291a9b848e25ae26f92f217dc8eece2a24'], [45366976, 141, 'bool Serialization::WriteIndexHashArray<GameCore::ImprovementTypes, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement> >(Serialization::OStream&, GameCore::ImprovementTypes const*, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement> const&)', 'f986220645bb337af7824adc6f5a29f5cf8113c4a51ac4a4e7b50b37c0120c27'], [45394032, 141, 'bool Serialization::WriteIndexHashArray<int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Project> >(Serialization::OStream&, int const*, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Project> const&)', '2d49aa4f6ef9309c4440be11617cc97ae4d15848c2a13eceebeff36c55bf76e2'], [45398496, 46, 'Serialization::SerializeFromSequenceContainer<GameCore::Player::ChopFeatureBehavior const, eastl::vector<GameCore::Player::ChopFeatureBehavior, eastl::allocator> const>::SerializeFromSequenceContainer(Serialization::OStream&, eastl::vector<GameCore::Player::ChopFeatureBehavior, eastl::allocator> const&)', 'da0780a7adb1c981743c8b1c09a97516ba190f211d2aef58093607b0cb0bc089'], [46101808, 36, 'Serialization::operator<<(Serialization::OStream&, GameCore::RandomEventTypes const&)', 'b0de9a4f2786dfa2343a398fdf67ec9f5a8fb9900bbf164c01f5c8ae32e99a5b'], [47514352, 36, 'Serialization::operator<<(Serialization::OStream&, GameCore::UnitPromotionClassTypes const&)', '821b0ff788c56d14d16ee196521bb6c3123fad502e2cb692d159c6b7c0c59e32'], [46772288, 33, 'Serialization::operator<<(Serialization::OStream&, GameCore::DiplomaticVisibilityTypes const&)', 'd2d6ffd3c11ef7d8a27751ea476f900ab91ed41f7b5096809a8ce1a1cb45fb63'], [45411680, 65, 'Serialization::operator<<(Serialization::OStream&, GameCore::Player::AdditionalPillaging const&)', 'f236c9c120e1317fb114b3998fdf9b55e115af415d8bf2a37670633f98540193'], [45411520, 76, 'Serialization::operator<<(Serialization::OStream&, GameCore::Player::ChopFeatureBehavior const&)', '635fe7b4de634d845344fb19c42691b6f47ce6ce5865e98e550940d3d7fcc0fc'], [45411840, 1679, 'Serialization::operator<<(Serialization::OStream&, GameCore::Player::Instance const&)', 'cbeacc8a90c5fa8853e90d6971174683990e8bbee2c91c00a9be1bb2023d8593'], [45366912, 56, 'Serialization::OStream& Serialization::operator<< <GameCore::ImprovementTypes, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement>, &GameCore::Definitions::GetImprovementDefinitions>(Serialization::OStream&, GameCore::IndexList<GameCore::ImprovementTypes, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement>, &GameCore::Definitions::GetImprovementDefinitions, void> const&)', '9fb8dd3928d69696298fdd82c6c0212af61375b04c459855f8190c6f32dadf1a'], [45370832, 91, 'Serialization::OStream& Serialization::operator<< <GameCore::RandomEventTypes, Types::LinearAlloc::Default<1u, 0u> >(Serialization::OStream&, Types::Vector<GameCore::RandomEventTypes, Types::LinearAlloc::Default<1u, 0u> > const&)', '6573e802ca5d63421e5f0177569ea4c99ef1d7aa6d99aca11d8f2c737ef9fb6c'], [45384576, 91, 'Serialization::OStream& Serialization::operator<< <GameCore::UnitPromotionClassTypes, Types::LinearAlloc::Default<1u, 0u> >(Serialization::OStream&, Types::Vector<GameCore::UnitPromotionClassTypes, Types::LinearAlloc::Default<1u, 0u> > const&)', 'f2d1b6adb95c0c4a505e16c7617b7800ccabb28fe7606e2b063de29cd610f776'], [45381024, 91, 'Serialization::OStream& Serialization::operator<< <GameCore::DiplomaticVisibilityTypes, Types::LinearAlloc::Default<1u, 0u> >(Serialization::OStream&, Types::Vector<GameCore::DiplomaticVisibilityTypes, Types::LinearAlloc::Default<1u, 0u> > const&)', 'd0f598f53ce8ba02e0eee01d5ce3bb25de52deff4e7fd2f944b0eaeff3b1cd6d'], [45403360, 91, 'Serialization::OStream& Serialization::operator<< <GameCore::Player::AdditionalPillaging>(Serialization::OStream&, eastl::vector<GameCore::Player::AdditionalPillaging, eastl::allocator> const&)', 'e74da715e1594c2fb8112b8d06a7af5b584368577addf72a7a9ddb00579d601f'], [45398320, 91, 'Serialization::OStream& Serialization::operator<< <GameCore::Player::ChopFeatureBehavior>(Serialization::OStream&, eastl::vector<GameCore::Player::ChopFeatureBehavior, eastl::allocator> const&)', 'f25cf45a9e0c408c2db468999617d61928c1d69394b8124b3e111d4ed2196a60'], [45393968, 56, 'Serialization::OStream& Serialization::operator<< <int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Project>, &GameCore::Definitions::GetProjectDefinitions>(Serialization::OStream&, GameCore::IndexList<int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Project>, &GameCore::Definitions::GetProjectDefinitions, void> const&)', '59b80fc579affeab2d24191c8f86b8017c57345efb6163c775ed3a22eb6a8ae4'], [45411760, 65, 'Serialization::operator>>(Serialization::IStream&, GameCore::Player::AdditionalPillaging&)', 'dae65c56623c860f4b8378e5271d129412452db43621609da4ec5813a69a069b'], [45411600, 65, 'Serialization::operator>>(Serialization::IStream&, GameCore::Player::ChopFeatureBehavior&)', '3a0a958993c1ef76def576bebb4979d0c7262475606f59329ac7936d8a82897e'], [45413520, 3518, 'Serialization::operator>>(Serialization::IStream&, GameCore::Player::Instance&)', '6cafd8cbdf97f8acc05be3175d60823c706339e01e84c9b757aa63009ef0f3ed'], [45388352, 15, 'FAutoVariable<Data::TypedVariantMap, GameCore::Player::Instance>::save(Serialization::OStream&) const', '2411cdaa43112bebc6bc78e39e373185461d7086257199f2633499af89094cd3'], [47512240, 33, 'Serialization::operator<<(OStream&, PlunderTypes const&)', '78d4e27dec33d7dd67aceec825a7365e2f868ca99c6a1453bd0b7b9bd3a22355'], [47512432, 33, 'Serialization::operator<<(OStream&, YieldTypes const&)', 'b971caad466bb8cb3baf0b50ec89d1f83d536c9d25f15145cc664077fdd4a5c7'], [47508448, 36, 'Serialization::operator<<(OStream&, DistrictTypes const&)', 'dc40e9bc43e0fc8aec2e195881b9a1c8eab8c366272b24f46990d7fb1231425c'], [47509008, 36, 'Serialization::operator<<(OStream&, ImprovementTypes const&)', '553cc1de61effac943a18e8873ec9df614ca5f5c40d8631d2077791eb977a632']]
DWARF_SHA = '3ec3f0e0f1a7bb552d21a3731c67884e40409d29ca92fd00fb0c945ea91991ad'
FIXTURES = [('ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884','trace-benchmark-1.jsonl.gz','528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'),
 ('85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8','trace-captured-1.jsonl.gz','027e6d4470a8755121d3e02221a36ae978fd4fd8e128ca296896144064f2a369')]
HARNESS=r'''use c6_core::{player_tail::{Tail,Context},unit::Limits};
fn projection(t:&Tail)->Vec<i64>{let mut p=Vec::new();macro_rules! list{($v:expr)=>{{p.push($v.len() as i64);for &x in &$v{p.push(x as i64);}}};}if let Some(a)=&t.alive{list!(a.agendas);list!(a.agenda_visibilities);list!(a.foreign_continent_city_unit_class_hashes);list!(a.district_naval_unit_hashes);list!(a.improvement_hashes);p.push(a.special_allowed_projects.len() as i64);for x in &a.special_allowed_projects{p.extend([x.definition_hash as i64,x.value as i64]);}p.push(a.chop_behaviors.len() as i64);for x in &a.chop_behaviors{p.extend([x.feature_wire as i64,x.ban as i64,x.yield_type as i64,x.amount as i64]);}p.push(a.additional_pillaging.len() as i64);for x in &a.additional_pillaging{p.extend([x.plunder_type as i64,x.improvement_hash as i64,x.district_hash as i64,x.amount as i64]);}list!(a.co2_generation_reductions);}p.push(t.properties.entries.len() as i64);p.push(t.improvement_replacements.len() as i64);for x in &t.improvement_replacements{p.extend([x.definition_hash as i64,x.value as i64]);}list!(t.random_event_hashes);list!(t.terrain_work_impassable);list!(t.random_event_no_unit_damage);list!(t.random_event_modified_damage_opposing_player);p}
fn main(){let a:Vec<_>=std::env::args().collect();let b=std::fs::read(&a[1]).unwrap();let c=Context{player_version:47,was_ever_alive:true};let(v,n,items)=Tail::decode_prefix_with_items(&b,c,Limits{max_bytes:b.len(),max_items:b.len()}).unwrap();assert_eq!(n,b.len());let lim=Limits{max_bytes:n,max_items:items};let(out,wi)=v.encode_with_items(c,lim).unwrap();assert_eq!(out,b);assert_eq!(wi,items);assert_eq!(v,Tail::decode(&b,c,lim).unwrap());for end in 0..n{assert!(Tail::decode(&b[..end],c,lim).is_err());}for l in [Limits{max_bytes:n-1,..lim},Limits{max_items:items-1,..lim}]{assert!(Tail::decode(&b,c,l).is_err());assert!(v.encode(c,l).is_err());}std::fs::write(&a[2],out).unwrap();println!("{} {} {:?}",n,items,projection(&v));}'''

def project(data):
    """Independent exact saved field projection for observed empty properties."""
    pos=0;items=0;out=[];counts=[]
    def read(fmt):
        nonlocal pos
        value=struct.unpack_from('<'+fmt,data,pos);pos+=struct.calcsize('<'+fmt)
        return value
    def vector(fmt):
        nonlocal items
        count=read('I')[0];items+=count;out.append(count);counts.append(count)
        for _ in range(count):out.extend(read(fmt))
    for fmt in ['I','i','I','I','I','Ii','i?ii','iIIi','i']:vector(fmt)
    properties=read('I')[0]
    if properties:raise ValueError('independent fixture projection only covers observed empty properties')
    out.append(properties)
    for fmt in ['II','I','i','i','i']:vector(fmt)
    if pos!=len(data):raise ValueError('independent grammar does not reach Player end')
    return [int(x)for x in out],items,counts

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra')
    p.add_argument('--source-commit',required=True);p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();game=Elf(a.assets/'libGameCore_XP2.so',GAME_SHA);host=Elf(a.assets/'Civ6',HOST_SHA)
    for address,size,name,digest in METHODS:
        if sha(game.read(address,size))!=digest:raise ValueError('method mismatch: '+name)
    dwarf=''.join(subprocess.check_output(['gdb','-nx','-batch',str(a.assets/'libGameCore_XP2.so'),'-ex',"ptype /o '"+name+"'"],text=True)for name in ['GameCore::Player::Instance','GameCore::Player::ChopFeatureBehavior','GameCore::Player::AdditionalPillaging'])
    if sha(dwarf.encode())!=DWARF_SHA:raise ValueError('DWARF mismatch')
    # Data object vptr points two entries past the ELF vtable symbol.
    virtuals=[]
    for offset,body in [(0x1f8,0x3961e50),(0x1f0,0x3971330)]:
        slot=0x53b3680+16+offset
        value=struct.unpack('<Q',host.read(slot,8))[0]
        if value!=body:raise ValueError('properties virtual binding mismatch')
        virtuals.append([hex(slot),hex(body)])
    pin=subprocess.check_output(['git','rev-parse',a.source_commit+'^{commit}'],cwd=ROOT,text=True).strip();reports=[]
    with tempfile.TemporaryDirectory(prefix='c6-tail-') as tmp:
        tmp=Path(tmp);source=tmp/'source';source.mkdir();archive=subprocess.check_output(['git','archive',pin],cwd=ROOT)
        subprocess.run(['tar','-x','-C',str(source)],input=archive,check=True)
        if (source/'tools/linux/player_tail_audit.py').read_bytes()!=Path(__file__).read_bytes():raise ValueError('driver must be committed with candidate')
        sources={str(x.relative_to(source)):sha(x.read_bytes())for x in [source/'Cargo.toml',source/'Cargo.lock',source/'crates/c6-core/Cargo.toml',*sorted((source/'crates/c6-core/src').rglob('*.rs')),source/'tools/linux/player_tail_audit.py']}
        subprocess.run(['cargo','clippy','--offline','-p','c6-core','--all-targets','--','-D','warnings'],cwd=source,check=True)
        subprocess.run(['cargo','test','--offline','-p','c6-core','player_tail::tests'],cwd=source,check=True)
        subprocess.run(['cargo','build','--offline','--release','-p','c6-core'],cwd=source,check=True)
        (tmp/'candidate.rs').write_text(HARNESS)
        subprocess.run(['rustc','--edition=2021','-Dwarnings','-O',str(tmp/'candidate.rs'),'--extern','c6_core='+str(source/'target/release/libc6_core.rlib'),'-L','dependency='+str(source/'target/release/deps'),'-o',str(tmp/'candidate')],check=True)
        for identity,trace_name,trace_sha in FIXTURES:
            data=gzip.decompress((a.private_root/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes())
            trace=gzip.decompress((a.private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes())
            if sha(data)!=identity or sha(trace)!=trace_sha:raise ValueError('fixture identity mismatch')
            owners={x['call']:x for x in map(json.loads,trace.splitlines())if x.get('event')=='owner' and x.get('save')==1}
            cuts=sorted([x for x in owners.values()if x['owner']=='GameCore::AI::BehaviorTree::Manager'],key=lambda x:x['start'])
            if len(cuts)!=43:raise ValueError('actual tail count mismatch')
            observations=[];digests=[];ranges=[];count_profiles=[]
            for cut in cuts:
                parent=owners[cut['parent']]
                if parent['owner']!='GameCore::Player::Instance':raise ValueError('wrong owner')
                start,end=cut['end'],parent['end'];payload=data[start:end];projection,items,counts=project(payload)
                (tmp/'in').write_bytes(payload)
                words=subprocess.check_output([str(tmp/'candidate'),str(tmp/'in'),str(tmp/'out')],text=True).strip().split(' ',2)
                n,actual_items=int(words[0]),int(words[1]);actual_projection=json.loads(words[2])
                if n!=len(payload) or items!=actual_items or projection!=actual_projection or (tmp/'out').read_bytes()!=payload:raise ValueError('typed projection/roundtrip mismatch')
                observations.append([n,items]);digests.append(sha(payload));ranges.append([start,end]);count_profiles.append(counts)
            reports.append(dict(stream_sha256=identity,trace_sha256=trace_sha,records=43,record_bytes=sum(x[0]for x in observations),record_items=sum(x[1]for x in observations),byte_item_caps=sorted(set(tuple(x)for x in observations)),vector_count_profiles=sorted(set(tuple(x)for x in count_profiles)),first_observed_range=ranges[0],ordered_record_digests_sha256=sha(json.dumps(digests,separators=(',',':')).encode())))
        candidate_sha=sha((tmp/'candidate').read_bytes())
    report=dict(schema='c6-linux-player-tail-v1',checks_passed=True,gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,current_parent_version=47,methods=[dict(address=hex(x),size=n,name=name,sha256=h)for x,n,name,h in METHODS],properties_virtual_bindings=virtuals,dwarf_sha256=DWARF_SHA,candidate_source_commit=pin,candidate_archive_sha256=sha(archive),candidate_sources=sources,candidate_sha256=candidate_sha,harness_sha256=sha(HARNESS.encode()),audit_source_sha256=sha(Path(__file__).read_bytes()),fixtures=reports,source_only_tests=4,production_start='actual caller supplied prefix; explicit Player47/wasEverAlive context',validation_start='actual BehaviorTree.Manager.end with its enclosing Player.end; 43 per stream, no guessed offset or signature',actual_false_branch_records=0,empty_properties_projection_only=True,all_actual_truncations_rejected=True,exact_byte_item_caps_checked=True,one_short_caps_rejected=True,original_execution=False,private_writes=False,tail_owned_allocation='two-pass byte/item/storage preflight, checked count/minimum bytes before try_reserve_exact; output preflight and exact fallible reserve',nested_variant_allocation='existing bounded TypedVariantMap decoder/encoder with cumulative remaining bytes/items and depth64; inherited Vec push/to_vec/collect may allocate infallibly',unsupported_forms='Player versions other than47; unsupported inherited typed variant branches/modifiers fail closed',native_read_effects=False,full_player=False,world_import=False,gameplay_state=False,actions=False,turns=False,unknown_save_bytes_masked=False,recorded_raw_save_load_difference_bytes=89198)
    a.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(checks_passed=True,fixtures=reports)))
if __name__=='__main__':main()
