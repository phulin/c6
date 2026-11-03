#!/usr/bin/env python3
"""Independent current Barbarian observed-wire review. No original execution."""
import gzip
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
PIN = 'd401fb587522087e8a2c55f60ecb30378ed3905f'
sys.path.insert(0,str(ROOT/'tools/linux'))
from player_wmds_audit import Elf, GAME_SHA, HOST_SHA, FIXTURES, relocation_symbols

def sha(b): return hashlib.sha256(b).hexdigest()

class Parser:
    def __init__(self, raw):
        self.raw,self.pos,self.items,self.words=raw,0,0,[]
        self.sites,self.children,self.properties=[],[],[]
        self.empty_only={'properties':0,'camp_locations':0,'clan_player_stats':0,'claims':0}
    def get(self, fmt, project=True):
        out=struct.unpack_from('<'+fmt,self.raw,self.pos)
        self.pos+=struct.calcsize('<'+fmt)
        if project:self.words.extend(out)
        return out[0] if len(out)==1 else out
    def version(self,n):
        self.sites.append(self.pos);assert self.get('I',False)==n
    def vec(self,fn):
        self.sites.append(self.pos);n=self.get('I');assert n<=len(self.raw)
        self.items+=n
        for _ in range(n):fn()
        return n
    def component(self):self.get('hhI')
    def string(self):self.vec(lambda:self.get('B'))
    def stats(self):
        self.version(15);self.get('iiiii');self.vec(lambda:self.get('I'));self.get('iiiii')
    def instance(self):
        start=self.pos;self.version(15);self.get('iIiiiB')
        # Rampage target, six fields20..34, three80..88, four required spawns.
        self.get('i'*14);self.vec(self.string);self.get('III')
        self.vec(self.component);self.vec(self.component);self.get('ii')
        self.properties.append(self.pos);self.sites.append(self.pos)
        # The original fixtures contain only empty Count-header maps. Keep that
        # qualification explicit: no opaque map bytes or invented field values.
        assert self.get('I')==0
        self.empty_only['properties']+=1
        self.vec(lambda:self.get('I'));self.get('ii');self.vec(lambda:self.get('i'))
        self.empty_only['clan_player_stats']+=self.vec(self.stats)
        self.vec(lambda:self.get('ii'))
        self.children.append(('GameCore::Barbarian::Instance',start,self.pos))
    def camp(self):self.version(1);self.get('i')
    def claim(self):self.version(3);self.get('Iii')
    def parse(self):
        self.version(5);self.get('iIIIiIB');self.vec(lambda:self.get('i'));self.vec(self.instance)
        self.empty_only['camp_locations']=self.vec(self.camp)
        self.version(3);self.empty_only['claims']=self.vec(self.claim);self.get('i')
        assert self.pos==len(self.raw)
        return struct.pack('<'+'q'*len(self.words),*self.words)

HARNESS=r'''
use c6_core::{game_barbarian::*,unit::Limits};
use c6_core::unit::variants::{KeyedHeader,VariantError,TypedEntry,TypedVariant,ArrayVariant,VariantPayload};
fn put(o:&mut Vec<i64>,v:impl Into<i64>){o.push(v.into());}
macro_rules! p{($o:expr,$($v:expr),*)=>{$(put($o,$v);)*};}
fn ints(o:&mut Vec<i64>,v:&[i32]){o.push(v.len() as i64);for x in v {put(o,*x);}}
fn uints(o:&mut Vec<i64>,v:&[u32]){o.push(v.len() as i64);for x in v {put(o,*x);}}
fn components(o:&mut Vec<i64>,v:&[ComponentId]){o.push(v.len() as i64);for c in v {p!(o,c.owner,c.component_type,c.id);}}
fn projection(v:&Manager)->Vec<u8>{let o=&mut Vec::new();
p!(o,v.player_ordinal,v.value_c8,v.value_e0,v.value_f8,v.control_priority_ordinal,v.improvement_hash,v.flag_168_raw);ints(o,&v.values_1a8);
o.push(v.instances.len() as i64);for b in &v.instances {
p!(o,b.player_ordinal,b.tribe_name_hash,b.tribe_index,b.camp_location.x,b.camp_location.y,b.living_raw,b.rampage_target_ordinal,
b.boldness,b.max_num_scouts,b.cur_num_scouts,b.turns_since_scout_death,b.max_standard_military_units,b.num_standard_military_units,
b.turns_since_last_warrior_spawn,b.percent_ranged_units,b.turns_between_warrior_spawn,b.required_melee_spawn,b.required_ranged_spawn,b.required_siege_spawn,b.required_support_spawn);
o.push(b.needed_unit_tags.len() as i64);for s in &b.needed_unit_tags {o.push(s.bytes.len() as i64);for x in &s.bytes {put(o,*x);}}
p!(o,b.scouting_behavior_tree,b.raiding_behavior_tree,b.city_attack_behavior_tree);components(o,&b.scouted_cities);components(o,&b.raided_cities);p!(o,b.raiding_score,b.turns_in_enemy_territory);
assert_eq!(b.properties.header,KeyedHeader::Count);assert!(b.properties.entries.is_empty());o.push(0);
uints(o,&b.clan_tech_hashes);p!(o,b.tech_steal_turns_cooldown,b.conversion_points);ints(o,&b.player_bribe_turns_remaining);
o.push(b.clan_player_stats.len() as i64);for s in &b.clan_player_stats {p!(o,s.raid_count,s.hire_count,s.bribe_count,s.incite_count,s.ransom_count);uints(o,&s.unit_hashes_hired);p!(o,s.last_turn_raided,s.last_turn_hired,s.last_turn_bribed,s.last_turn_incited,s.last_turn_ransomed);}
o.push(b.incite_targets.len() as i64);for t in &b.incite_targets {p!(o,t.player_ordinal,t.weight);}}
o.push(v.camp_locations.len() as i64);for c in &v.camp_locations {put(o,c.plot_index);}
o.push(v.clans.unique_unit_claims.len() as i64);for c in &v.clans.unique_unit_claims {p!(o,c.unit_hash,c.value_4,c.value_8);}put(o,v.clans.value_100);
o.iter().flat_map(|x|x.to_le_bytes()).collect()}
fn word(b:&[u8],p:&mut usize)->usize {let v=u32::from_le_bytes(b[*p..*p+4].try_into().unwrap());*p+=4;v as usize}
fn property(raw:&[u8],s:usize,ty:u16,modifier:u16,flags:u32)->Vec<u8>{let mut replacement=Vec::new();replacement.extend(1u32.to_le_bytes());replacement.extend(7u32.to_le_bytes());replacement.extend(ty.to_le_bytes());replacement.extend(modifier.to_le_bytes());replacement.extend(flags.to_le_bytes());replacement.extend(0u32.to_le_bytes());replacement.extend(0u32.to_le_bytes());let mut changed=raw.to_vec();changed.splice(s..s+4,replacement);changed}
fn main(){let a:Vec<_>=std::env::args().collect();let all=std::fs::read(&a[1]).unwrap();let mut pos=0;
while pos<all.len(){let size=word(&all,&mut pos);let items=word(&all,&mut pos);let pn=word(&all,&mut pos);let ns=word(&all,&mut pos);let np=word(&all,&mut pos);
let sites:Vec<_>=(0..ns).map(|_|word(&all,&mut pos)).collect();let properties:Vec<_>=(0..np).map(|_|word(&all,&mut pos)).collect();let expected=&all[pos..pos+pn];pos+=pn;let raw=&all[pos..pos+size];pos+=size;
let lim=Limits{max_bytes:size,max_items:items};let(v,n,i)=Manager::decode_prefix_with_items(raw,lim).unwrap();assert_eq!((n,i),(size,items));assert_eq!(projection(&v),expected);assert_eq!(v.encode_with_items(lim).unwrap(),(raw.to_vec(),items));
for end in 0..size {assert!(Manager::decode(&raw[..end],lim).is_err(),"cut {end}");}
for s in sites {for value in [0u32,u32::MAX] {if raw[s..s+4]==value.to_le_bytes(){continue;}let mut b=raw.to_vec();b[s..s+4].copy_from_slice(&value.to_le_bytes());assert!(Manager::decode(&b,lim).is_err(),"site {s}: {value}");}}
for short in [Limits{max_bytes:size-1,..lim},Limits{max_items:items-1,..lim}] {assert!(Manager::decode(raw,short).is_err());assert!(v.encode(short).is_err());}
let generous=Limits{max_bytes:size+1024,max_items:items+1024};
for (ty,modifier,flags,error) in [(6,0,0,VariantError::UnsupportedType(6)),(2,1,0,VariantError::UnsupportedModifiers(1)),(2,0,0x20000000,VariantError::InvalidMetadata)] {
let mut unsupported=v.clone();unsupported.instances[0].properties.entries.push(TypedEntry{key:7,name:None,value:TypedVariant{type_bits:ty,modifier_bits:modifier,value:ArrayVariant{count_flags:flags,element_size:0,payload:VariantPayload::Inline32(0)}}});assert_eq!(unsupported.encode(generous),Err(Error::Variant(error)));}
let mut suffix=raw.to_vec();suffix.extend([0xaa,0xbb]);assert_eq!(Manager::decode(&suffix,lim),Err(Error::Trailing));assert_eq!(Manager::decode_prefix_with_items(&suffix,lim).unwrap(),(v,size,items));
for s in properties {
for (ty,modifier,flags,error) in [(6,0,0,VariantError::UnsupportedType(6)),(2,1,0,VariantError::UnsupportedModifiers(1)),(2,0,0x20000000,VariantError::InvalidMetadata)] {
assert_eq!(Manager::decode(&property(raw,s,ty,modifier,flags),generous),Err(Error::Variant(error)));}}
let mut huge=raw.to_vec();huge[29..33].copy_from_slice(&u32::MAX.to_le_bytes());assert_eq!(Manager::decode(&huge,Limits{max_bytes:usize::MAX,max_items:usize::MAX}),Err(Error::Truncated));
}}
'''

def main():
    elf=Elf(ROOT.parent/'c6-assets-linux/libGameCore_XP2.so',GAME_SHA)
    host=Elf(ROOT.parent/'c6-assets-linux/Civ6',HOST_SHA)
    assert struct.unpack_from('<H',host.raw,16)[0]==2
    path=ROOT/'tools/linux/game_barbarian_audit.py'
    assert path.read_bytes()==subprocess.check_output(['git','show',PIN+':tools/linux/game_barbarian_audit.py'],cwd=ROOT)
    spec=importlib.util.spec_from_file_location('audit',path);audit=importlib.util.module_from_spec(spec);spec.loader.exec_module(audit)
    for address,size,_,digest in audit.METHODS:assert sha(elf.read(address,size))==digest
    for method in audit.HOST_METHODS.values():assert sha(host.read(int(method['address'],16),method['size_bound']))==method['sha256']
    assert struct.unpack('<I',elf.read(0x2f19340,4))[0]==15
    assert struct.unpack('<I',elf.read(0x2eec340,4))[0]==3
    relocs=relocation_symbols(elf)
    for site,plt,body in audit.BINDINGS:
        elf.call(site,plt);stub=elf.read(plt,6);assert stub[:2]==b'\xff\x25'
        assert relocs[plt+6+struct.unpack_from('<i',stub,2)[0]]==body
    reports=[];corpus=bytearray()
    for identity,name,digest in FIXTURES:
        private=ROOT.parent/'c6-ghidra/evidence'
        data=gzip.decompress((private/'linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes())
        trace=gzip.decompress((private/'linux-save-normalization/checkpoint-203-owners'/name).read_bytes())
        assert sha(data)==identity and sha(trace)==digest
        owners=[r for r in map(json.loads,trace.splitlines()) if r.get('event')=='owner' and r.get('save')==1]
        row,=[r for r in owners if r['owner']=='GameCore::Barbarian::Manager']
        assert (row['start'],row['end'])==(70934595,70963412)
        assert any(r['owner']=='GameCore::AI::GlobalAi' and r['end']==row['start'] for r in owners)
        assert any(r['owner']=='GameCore::FreeCities::Manager' and (r['start'],r['end'])==(row['end'],71027778) for r in owners)
        raw=data[row['start']:row['end']];parser=Parser(raw);projection=parser.parse()
        children=[(r['owner'],r['start']-row['start'],r['end']-row['start']) for r in owners if r['parent']==row['call']]
        assert parser.children==children
        sites=parser.sites;properties=parser.properties
        corpus.extend(struct.pack('<IIIII',len(raw),parser.items,len(projection),len(sites),len(properties)))
        corpus.extend(struct.pack('<'+'I'*(len(sites)+len(properties)),*(sites+properties)));corpus.extend(projection);corpus.extend(raw)
        reports.append(dict(core_sha256=identity,trace_sha256=digest,wire_sha256=sha(raw),start=row['start'],end=row['end'],bytes=len(raw),items=parser.items,typed_projection_words=len(projection)//8,typed_projection_sha256=sha(projection),every_byte_truncations=len(raw),hostile_sites=len(sites),direct_instance_children=len(children),property_qualified_rejections=3*len(properties),native_empty_branches=parser.empty_only))
    archive=subprocess.check_output(['git','archive',PIN],cwd=ROOT)
    with tempfile.TemporaryDirectory(prefix='c6-barbarian-review-') as d:
        tmp=Path(d);source=tmp/'source';source.mkdir();subprocess.run(['tar','-x','-C',str(source)],input=archive,check=True)
        inputs={str(p.relative_to(source)):sha(p.read_bytes()) for p in source.rglob('*') if p.is_file()}
        assert 'pub mod game_barbarian;' in (source/'crates/c6-core/src/lib.rs').read_text()
        subprocess.run(['cargo','test','--offline','-p','c6-core','game_barbarian::tests'],cwd=source,check=True)
        subprocess.run(['cargo','clippy','--offline','-p','c6-core','--all-targets','--','-D','warnings'],cwd=source,check=True)
        subprocess.run(['cargo','build','--offline','--release','-p','c6-core'],cwd=source,check=True)
        (tmp/'review.rs').write_text(HARNESS);(tmp/'corpus').write_bytes(corpus)
        subprocess.run(['rustc','--edition=2021','-Dwarnings','-O',str(tmp/'review.rs'),'--extern','c6_core='+str(source/'target/release/libc6_core.rlib'),'-L','dependency='+str(source/'target/release/deps'),'-o',str(tmp/'review')],check=True)
        subprocess.run([str(tmp/'review'),str(tmp/'corpus')],check=True)
        assert all(sha((source/p).read_bytes())==h for p,h in inputs.items())
        exe=sha((tmp/'review').read_bytes())
    report=dict(schema='c6-linux-game-barbarian-independent-review-v1',checks_passed=True,source_commit=PIN,archive_sha256=sha(archive),module_sha256=inputs['crates/c6-core/src/game_barbarian.rs'],review_driver_sha256=sha(Path(__file__).read_bytes()),review_harness_sha256=sha(HARNESS.encode()),review_executable_sha256=exe,gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,fixtures=reports,all_archive_files_unchanged=True,archive_files_checked=len(inputs),actual_export_used=True,source_tests=3,strict_core_clippy=True,full_observed_typed_projection_match=True,byte_roundtrip=True,every_byte_truncations=True,all_changed_version_and_count_sites_zero_and_max_rejected=True,shared_item_and_byte_caps=True,strict_and_prefix_suffix_behavior=True,qualified_property_variants_rejected=True,hostile_count_with_unlimited_limits_rejected_before_allocation=True,arbitrary_typed_property_coverage=False,original_execution=False,native_read_side_effects=False,gameplay_state=False,gameplay_rng=False,actions=False,turns=False,ai_planning_exclusion=False,unknown_save_bytes_masked=False,recorded_raw_save_load_difference_bytes=89198)
    (ROOT/'evidence/linux/game-barbarian-review.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,fixtures=reports,archive_files_checked=len(inputs)),indent=2))
if __name__=='__main__':main()
