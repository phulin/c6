#!/usr/bin/env python3
"""Independent complete typed projection and boundary review; never runs game code."""
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
PIN = '69b5aea78cd0af35b4b89f9e2c069aed7d32ae81'
sys.path.insert(0, str(ROOT/'tools/linux'))
from player_wmds_audit import Elf, GAME_SHA, HOST_SHA, FIXTURES, relocation_symbols

def sha(b): return hashlib.sha256(b).hexdigest()

class Projection:
    def __init__(self, raw):
        self.raw, self.p, self.words, self.items = raw, 0, [], 0
        self.sites, self.children, self.cuts = [], [], {0, len(raw)-1}
    def get(self, fmt, projection=True):
        start = self.p
        values = struct.unpack_from('<'+fmt, self.raw, start)
        self.p += struct.calcsize('<'+fmt)
        self.cuts.update((start, min(start+1, self.p), self.p-1))
        if projection: self.words.extend(values)
        return values[0] if len(values)==1 else values
    def version(self, n):
        self.sites.append((self.p, 'version'))
        assert self.get('I', False)==n
    def vec(self, fn):
        self.sites.append((self.p, 'count'))
        n = self.get('I'); assert n <= len(self.raw)
        self.items += n
        for _ in range(n): fn()
    def city(self): self.get('hhI')
    def handle(self): self.version(1); self.get('iI')
    def q8(self): self.get('Ii')
    def espionage_city(self):
        self.version(3); self.city()
        old = self.p; self.vec(lambda:self.get('i'))
        assert struct.unpack_from('<I',self.raw,old)[0]==11
        self.get('i')
    def espionage(self):
        start=self.p; self.version(4); self.get('ii')
        self.vec(self.espionage_city); self.get('B'); self.vec(self.city)
        for _ in range(3): self.vec(lambda:self.get('i'))
        self.children.append(('GameCore::AI::Espionage',start,self.p))
    def belief(self): self.get('I'); self.version(1); self.get('iIiBi')
    def religious_city(self):
        self.version(4); self.city(); self.get('iiiiiiiB'); self.vec(lambda:self.get('I'))
    def religious(self):
        start=self.p; self.version(6); flag=self.get('B')
        if flag:
            self.get('iIB'); self.vec(self.belief); self.vec(self.handle); self.handle()
            self.get('B'); self.vec(self.religious_city); self.vec(self.q8)
        self.children.append(('GameCore::AI::Religious',start,self.p))
    def parse(self):
        self.version(4); self.get('I'); self.version(1); self.get('I')
        self.version(1); self.get('I'); self.version(1); self.vec(self.espionage)
        self.version(2); self.vec(self.religious); self.get('i')
        self.vec(lambda:self.get('i')); self.vec(self.q8); self.vec(self.q8)
        assert self.p==len(self.raw)
        return struct.pack('<'+'q'*len(self.words),*self.words)

HARNESS = r'''
use c6_core::{game_global_ai::*,unit::Limits};
fn put(o:&mut Vec<i64>,x:impl Into<i64>){o.push(x.into());}
macro_rules! p {($o:expr,$($v:expr),*)=>{$(put($o,$v);)*};}
fn city(o:&mut Vec<i64>,v:&CityId){p!(o,v.owner,v.component_type,v.id);}
fn handle(o:&mut Vec<i64>,v:&ControlHandle){p!(o,v.player_ordinal,v.id);}
fn yields(o:&mut Vec<i64>,v:&[HashQ8]){o.push(v.len() as i64);for x in v {p!(o,x.definition_hash,x.raw_q8);}}
fn ints(o:&mut Vec<i64>,v:&[i32]){o.push(v.len() as i64);for x in v {put(o,*x);}}
fn projection(v:&GlobalAi)->Vec<u8>{let o=&mut Vec::new();
p!(o,v.value_f8,v.mayhem.native_f32_bits,v.control_factory.next_id);
o.push(v.espionage.entries.len() as i64);for e in &v.espionage.entries {
p!(o,e.player_ordinal,e.control_priority);o.push(e.city_analyses.len() as i64);
for c in &e.city_analyses {city(o,&c.city);ints(o,&c.act_values_raw_q8);put(o,c.value_34);}
put(o,e.flag_2a0);o.push(e.cities.len() as i64);for c in &e.cities{city(o,c);}
ints(o,&e.values_158);ints(o,&e.values_180);ints(o,&e.values_1a8);}
o.push(v.religious.entries.len() as i64);for r in &v.religious.entries {put(o,r.initialized_raw);
if let Some(b)=&r.body {p!(o,b.player_ordinal,b.religion_hash,b.flag_118);o.push(b.beliefs.len() as i64);
for x in &b.beliefs {let a=&x.analysis;p!(o,x.definition_hash,a.player_ordinal,a.belief_hash,a.belief_class_ordinal,a.flag_10,a.value_raw_q8);}
o.push(b.handles.len() as i64);for h in &b.handles {handle(o,h);}handle(o,&b.handle);put(o,b.flag_130);
o.push(b.cities.len() as i64);for c in &b.cities {city(o,&c.city);p!(o,c.position.x,c.position.y,c.position_10.x,c.position_10.y,c.value_18_raw_q8,c.value_24_raw_q8,c.value_1c_raw_q8,c.flag_20);o.push(c.values_28.len() as i64);for x in &c.values_28 {put(o,*x);}}
yields(o,&b.yields);}}
put(o,v.value_128_raw_q8);ints(o,&v.values_140);yields(o,&v.yields_a8);yields(o,&v.yields_d0);
o.iter().flat_map(|x|x.to_le_bytes()).collect()}
fn word(b:&[u8],p:&mut usize)->usize {let v=u32::from_le_bytes(b[*p..*p+4].try_into().unwrap());*p+=4;v as usize}
fn main(){let a:Vec<_>=std::env::args().collect();let all=std::fs::read(&a[1]).unwrap();let mut pos=0;
while pos<all.len(){let size=word(&all,&mut pos);let items=word(&all,&mut pos);let pn=word(&all,&mut pos);let cn=word(&all,&mut pos);let sn=word(&all,&mut pos);
let cuts:Vec<_>=(0..cn).map(|_|word(&all,&mut pos)).collect();let sites:Vec<_>=(0..sn).map(|_|word(&all,&mut pos)).collect();
let expected=&all[pos..pos+pn];pos+=pn;let raw=&all[pos..pos+size];pos+=size;let lim=Limits{max_bytes:size,max_items:items};
let(v,n,i)=GlobalAi::decode_prefix_with_items(raw,lim).unwrap();assert_eq!((n,i),(size,items));assert_eq!(projection(&v),expected);assert_eq!(v.encode_with_items(lim).unwrap(),(raw.to_vec(),items));
for c in cuts {assert!(GlobalAi::decode(&raw[..c],lim).is_err(),"cut {c}");}
for s in sites {for val in [0u32,u32::MAX] {if raw[s..s+4]==val.to_le_bytes(){continue;}let mut b=raw.to_vec();b[s..s+4].copy_from_slice(&val.to_le_bytes());assert!(GlobalAi::decode(&b,lim).is_err(),"site {s} value {val}");}}
for short in [Limits{max_bytes:size-1,..lim},Limits{max_items:items-1,..lim}] {assert!(GlobalAi::decode(raw,short).is_err());assert!(v.encode(short).is_err());}
let mut suffix=raw.to_vec();suffix.extend([0xaa,0xbb]);assert_eq!(GlobalAi::decode(&suffix,lim),Err(Error::Trailing));assert_eq!(GlobalAi::decode_prefix_with_items(&suffix,lim).unwrap(),(v,size,items));
}}
'''

def main():
    elf=Elf(ROOT.parent/'c6-assets-linux/libGameCore_XP2.so',GAME_SHA)
    host=Elf(ROOT.parent/'c6-assets-linux/Civ6',HOST_SHA)
    assert struct.unpack_from('<H',host.raw,16)[0]==2
    assert (ROOT/'tools/linux/game_global_ai_audit.py').read_bytes()==subprocess.check_output(['git','show',PIN+':tools/linux/game_global_ai_audit.py'],cwd=ROOT)
    spec=importlib.util.spec_from_file_location('audit',ROOT/'tools/linux/game_global_ai_audit.py')
    audit=importlib.util.module_from_spec(spec);spec.loader.exec_module(audit)
    for address,size,_,digest in audit.METHODS: assert sha(elf.read(address,size))==digest
    relocs=relocation_symbols(elf)
    bindings=[]
    for site in [0x2d7a866,0x2d7a879,0x2d7a889,0x2d7ab6f,0x2d7ab9a,0x2d7abaa]:
        code=elf.read(site,5);assert code[0]==0xe8
        plt=site+5+struct.unpack_from('<i',code,1)[0];stub=elf.read(plt,6);assert stub[:2]==b'\xff\x25'
        target=relocs[plt+6+struct.unpack_from('<i',stub,2)[0]]
        bindings.append(dict(site=hex(site),plt=hex(plt),target=hex(target)))
    assert bindings[1]['target']=='0x20f41b0' and bindings[4]['target']=='0x20f42b0'
    reports=[]; corpus=bytearray()
    for identity,name,digest in FIXTURES:
        private=ROOT.parent/'c6-ghidra/evidence'
        data=gzip.decompress((private/'linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes())
        trace=gzip.decompress((private/'linux-save-normalization/checkpoint-203-owners'/name).read_bytes())
        assert sha(data)==identity and sha(trace)==digest
        owners=[r for r in map(json.loads,trace.splitlines()) if r.get('event')=='owner' and r.get('save')==1]
        row,=[r for r in owners if r['owner']=='GameCore::AI::GlobalAi']
        assert (row['start'],row['end'])==(70566134,70934595)
        assert any(r['owner']=='GameCore::Player::Manager' and r['end']==row['start'] for r in owners)
        assert any(r['owner']=='GameCore::Barbarian::Manager' and r['start']==row['end'] for r in owners)
        raw=data[row['start']:row['end']];parser=Projection(raw);projection=parser.parse()
        children=[(r['owner'],r['start']-row['start'],r['end']-row['start']) for r in owners if r['parent']==row['call']]
        assert parser.children==children
        # Every scalar start, interior and end, plus every structural site. Current-version
        # framing/count words changed to zero/MAX must fail at this fixed full extent.
        cuts=sorted(c for c in parser.cuts if c<len(raw));sites=[s for s,_ in parser.sites]
        corpus.extend(struct.pack('<IIIII',len(raw),parser.items,len(projection),len(cuts),len(sites)))
        corpus.extend(struct.pack('<'+'I'*(len(cuts)+len(sites)),*(cuts+sites)))
        corpus.extend(projection);corpus.extend(raw)
        reports.append(dict(core_sha256=identity,trace_sha256=digest,wire_sha256=sha(raw),start=row['start'],end=row['end'],bytes=len(raw),items=parser.items,typed_projection_words=len(projection)//8,typed_projection_sha256=sha(projection),scalar_structural_interior_cuts=len(cuts),hostile_sites=len(sites),direct_children=len(children)))
    archive=subprocess.check_output(['git','archive',PIN],cwd=ROOT)
    with tempfile.TemporaryDirectory(prefix='c6-global-ai-review-') as d:
        tmp=Path(d);source=tmp/'source';source.mkdir()
        subprocess.run(['tar','-x','-C',str(source)],input=archive,check=True)
        inputs={str(p.relative_to(source)):sha(p.read_bytes()) for p in source.rglob('*') if p.is_file()}
        assert 'pub mod game_global_ai;' in (source/'crates/c6-core/src/lib.rs').read_text()
        subprocess.run(['cargo','test','--offline','-p','c6-core','game_global_ai::tests'],cwd=source,check=True)
        subprocess.run(['cargo','clippy','--offline','-p','c6-core','--all-targets','--','-D','warnings'],cwd=source,check=True)
        subprocess.run(['cargo','build','--offline','--release','-p','c6-core'],cwd=source,check=True)
        (tmp/'review.rs').write_text(HARNESS);(tmp/'corpus').write_bytes(corpus)
        subprocess.run(['rustc','--edition=2021','-Dwarnings','-O',str(tmp/'review.rs'),'--extern','c6_core='+str(source/'target/release/libc6_core.rlib'),'-L','dependency='+str(source/'target/release/deps'),'-o',str(tmp/'review')],check=True)
        subprocess.run([str(tmp/'review'),str(tmp/'corpus')],check=True)
        assert all(sha((source/p).read_bytes())==h for p,h in inputs.items())
        exe=sha((tmp/'review').read_bytes())
    report=dict(schema='c6-linux-game-global-ai-independent-review-v1',checks_passed=True,source_commit=PIN,archive_sha256=sha(archive),module_sha256=inputs['crates/c6-core/src/game_global_ai.rs'],review_driver_sha256=sha(Path(__file__).read_bytes()),review_harness_sha256=sha(HARNESS.encode()),review_executable_sha256=exe,gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,parent_bindings=bindings,fixtures=reports,all_archive_files_unchanged=True,archive_files_checked=len(inputs),actual_export_used=True,source_tests=3,strict_core_clippy=True,full_typed_projection_match=True,byte_roundtrip=True,all_transfer_boundary_and_interior_truncations=True,all_changed_version_and_count_sites_zero_and_max_rejected=True,shared_item_and_byte_caps=True,strict_and_prefix_suffix_behavior=True,original_execution=False,native_read_side_effects=False,gameplay_state=False,gameplay_rng=False,actions=False,turns=False,ai_planning_exclusion=False,unknown_save_bytes_masked=False,recorded_raw_save_load_difference_bytes=89198)
    (ROOT/'evidence/linux/game-global-ai-review.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,fixtures=reports,archive_files_checked=len(inputs)),indent=2))
if __name__=='__main__':main()
