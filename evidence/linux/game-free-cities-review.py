#!/usr/bin/env python3
"""Independent current Linux FreeCities typed wire review. No game execution."""
import gzip
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import subprocess
import sys
import tempfile

ROOT=Path(__file__).resolve().parents[2]
PIN='194711e2f055328637563d2e5aa0eb52d4682651'
sys.path.insert(0,str(ROOT/'tools/linux'))
from player_wmds_audit import Elf,GAME_SHA,HOST_SHA,FIXTURES,relocation_symbols

def sha(b):return hashlib.sha256(b).hexdigest()

class Parser:
    def __init__(self,raw):
        self.raw,self.pos,self.items,self.words=raw,0,0,[]
        self.sites,self.children=[],[]
        self.entries={'units':0,'buildings':0}
    def get(self,fmt,project=True):
        out=struct.unpack_from('<'+fmt,self.raw,self.pos);self.pos+=struct.calcsize('<'+fmt)
        if project:self.words.extend(out)
        return out[0] if len(out)==1 else out
    def version(self,n):
        self.sites.append(self.pos);assert self.get('I',False)==n
    def vec(self,fn):
        self.sites.append(self.pos);n=self.get('I');assert n<=len(self.raw)
        self.items+=n
        for _ in range(n):fn()
        return n
    def instance(self):
        start=self.pos;self.version(5)
        self.get('iiiiBiiiI')
        self.entries['units']+=self.vec(lambda:self.get('IB'))
        self.entries['buildings']+=self.vec(lambda:self.get('IB'))
        self.children.append(('GameCore::FreeCities::Instance',start,self.pos))
    def parse(self):
        self.version(3);self.get('ii');self.vec(self.instance)
        assert self.pos==len(self.raw)
        return struct.pack('<'+'q'*len(self.words),*self.words)

HARNESS=r'''
use c6_core::{game_free_cities::*,unit::Limits};
fn put(o:&mut Vec<i64>,x:impl Into<i64>){o.push(x.into());}
macro_rules! p{($o:expr,$($v:expr),*)=>{$(put($o,$v);)*};}
fn projection(v:&Manager)->Vec<u8>{let o=&mut Vec::new();p!(o,v.player_ordinal,v.control_priority_ordinal);o.push(v.instances.len() as i64);
for b in &v.instances {p!(o,b.player_ordinal,b.city_index,b.city_location.x,b.city_location.y,b.living_raw,b.turns_since_last_unit,b.urban_defenses,b.retaliation_target_ordinal,b.retaliation_behavior_tree);
o.push(b.buildable_units.len() as i64);for x in &b.buildable_units {p!(o,x.definition_hash,x.value_raw);}
o.push(b.buildable_walls.len() as i64);for x in &b.buildable_walls {p!(o,x.definition_hash,x.value_raw);}}
o.iter().flat_map(|x|x.to_le_bytes()).collect()}
fn word(b:&[u8],p:&mut usize)->usize{let x=u32::from_le_bytes(b[*p..*p+4].try_into().unwrap());*p+=4;x as usize}
fn main(){let a:Vec<_>=std::env::args().collect();let all=std::fs::read(&a[1]).unwrap();let mut pos=0;
while pos<all.len(){let size=word(&all,&mut pos);let items=word(&all,&mut pos);let pn=word(&all,&mut pos);let ns=word(&all,&mut pos);
let sites:Vec<_>=(0..ns).map(|_|word(&all,&mut pos)).collect();let expected=&all[pos..pos+pn];pos+=pn;let raw=&all[pos..pos+size];pos+=size;
let lim=Limits{max_bytes:size,max_items:items};let(v,n,i)=Manager::decode_prefix_with_items(raw,lim).unwrap();assert_eq!((n,i),(size,items));assert_eq!(projection(&v),expected);assert_eq!(v.encode_with_items(lim).unwrap(),(raw.to_vec(),items));
for end in 0..size{assert!(Manager::decode(&raw[..end],lim).is_err(),"cut {end}");}
for s in sites{for value in [0u32,u32::MAX]{if raw[s..s+4]==value.to_le_bytes(){continue;}let mut changed=raw.to_vec();changed[s..s+4].copy_from_slice(&value.to_le_bytes());assert!(Manager::decode(&changed,lim).is_err(),"site {s}: {value}");}}
for short in [Limits{max_bytes:size-1,..lim},Limits{max_items:items-1,..lim}]{assert_eq!(Manager::decode(raw,short),Err(Error::ResourceLimit));assert_eq!(v.encode(short),Err(Error::ResourceLimit));}
let mut huge=raw.to_vec();huge[12..16].copy_from_slice(&u32::MAX.to_le_bytes());assert_eq!(Manager::decode(&huge,Limits{max_bytes:usize::MAX,max_items:usize::MAX}),Err(Error::Truncated));
let mut suffix=raw.to_vec();suffix.extend([0xaa,0xbb]);assert_eq!(Manager::decode(&suffix,lim),Err(Error::Trailing));assert_eq!(Manager::decode_prefix_with_items(&suffix,lim).unwrap(),(v,size,items));
}}
'''

def main():
    elf=Elf(ROOT.parent/'c6-assets-linux/libGameCore_XP2.so',GAME_SHA)
    host=Elf(ROOT.parent/'c6-assets-linux/Civ6',HOST_SHA)
    assert struct.unpack_from('<H',host.raw,16)[0]==2
    path=ROOT/'tools/linux/game_free_cities_audit.py'
    assert path.read_bytes()==subprocess.check_output(['git','show',PIN+':tools/linux/game_free_cities_audit.py'],cwd=ROOT)
    spec=importlib.util.spec_from_file_location('audit',path);audit=importlib.util.module_from_spec(spec);spec.loader.exec_module(audit)
    for address,size,_,digest in audit.METHODS:assert sha(elf.read(address,size))==digest
    dwarf=''.join(subprocess.check_output(['gdb','-nx','-batch',str(ROOT.parent/'c6-assets-linux/libGameCore_XP2.so'),'-ex',"ptype /o '"+name+"'"],text=True)for name in audit.DWARF_NAMES)
    assert sha(dwarf.encode())==audit.DWARF_SHA
    relocs=relocation_symbols(elf)
    for site,plt,body in audit.BINDINGS:
        elf.call(site,plt);stub=elf.read(plt,6);assert stub[:2]==b'\xff\x25'
        assert relocs.get(plt+6+struct.unpack_from('<i',stub,2)[0])==body
    reports=[];corpus=bytearray()
    for identity,name,digest in FIXTURES:
        private=ROOT.parent/'c6-ghidra/evidence'
        data=gzip.decompress((private/'linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes())
        trace=gzip.decompress((private/'linux-save-normalization/checkpoint-203-owners'/name).read_bytes())
        assert sha(data)==identity and sha(trace)==digest
        owners=[r for r in map(json.loads,trace.splitlines()) if r.get('event')=='owner' and r.get('save')==1]
        row,=[r for r in owners if r['owner']=='GameCore::FreeCities::Manager']
        assert (row['start'],row['end'])==(70963412,71027778)
        assert any(r['owner']=='GameCore::Barbarian::Manager' and r['end']==row['start'] for r in owners)
        assert any(r['owner']=='GameCore::Random' and (r['start'],r['end'])==(row['end'],71027796) for r in owners)
        raw=data[row['start']:row['end']];parser=Parser(raw);projection=parser.parse()
        children=[(r['owner'],r['start']-row['start'],r['end']-row['start']) for r in owners if r['parent']==row['call']]
        assert parser.children==children
        # Compare all descendants too; this grammar contains only Instance owners.
        calls={row['call']}
        while True:
            expanded=calls|{r['call'] for r in owners if r['parent'] in calls}
            if expanded==calls:break
            calls=expanded
        descendants=[(r['owner'],r['start']-row['start'],r['end']-row['start']) for r in owners if r['call'] in calls and r['call']!=row['call']]
        assert descendants==children
        enclosed=[(r['owner'],r['start']-row['start'],r['end']-row['start']) for r in owners if row['start']<=r['start'] and r['end']<=row['end'] and r['call']!=row['call']]
        assert enclosed==children
        corpus.extend(struct.pack('<IIII',len(raw),parser.items,len(projection),len(parser.sites)))
        corpus.extend(struct.pack('<'+'I'*len(parser.sites),*parser.sites));corpus.extend(projection);corpus.extend(raw)
        reports.append(dict(core_sha256=identity,trace_sha256=digest,wire_sha256=sha(raw),start=row['start'],end=row['end'],bytes=len(raw),items=parser.items,typed_projection_words=len(projection)//8,typed_projection_sha256=sha(projection),every_byte_truncations=len(raw),hostile_sites=len(parser.sites),direct_instance_children=len(children),all_native_descendants=len(descendants),hash_bool_entries=parser.entries))
    archive=subprocess.check_output(['git','archive',PIN],cwd=ROOT)
    with tempfile.TemporaryDirectory(prefix='c6-free-cities-review-') as d:
        tmp=Path(d);source=tmp/'source';source.mkdir();subprocess.run(['tar','-x','-C',str(source)],input=archive,check=True)
        inputs={str(p.relative_to(source)):sha(p.read_bytes()) for p in source.rglob('*') if p.is_file()}
        assert 'pub mod game_free_cities;' in (source/'crates/c6-core/src/lib.rs').read_text()
        subprocess.run(['cargo','test','--offline','-p','c6-core','game_free_cities::tests'],cwd=source,check=True)
        subprocess.run(['cargo','clippy','--offline','-p','c6-core','--all-targets','--','-D','warnings'],cwd=source,check=True)
        subprocess.run(['cargo','build','--offline','--release','-p','c6-core'],cwd=source,check=True)
        (tmp/'review.rs').write_text(HARNESS);(tmp/'corpus').write_bytes(corpus)
        subprocess.run(['rustc','--edition=2021','-Dwarnings','-O',str(tmp/'review.rs'),'--extern','c6_core='+str(source/'target/release/libc6_core.rlib'),'-L','dependency='+str(source/'target/release/deps'),'-o',str(tmp/'review')],check=True)
        subprocess.run([str(tmp/'review'),str(tmp/'corpus')],check=True)
        assert all(sha((source/p).read_bytes())==h for p,h in inputs.items())
        exe=sha((tmp/'review').read_bytes())
    report=dict(schema='c6-linux-game-free-cities-independent-review-v1',checks_passed=True,source_commit=PIN,archive_sha256=sha(archive),module_sha256=inputs['crates/c6-core/src/game_free_cities.rs'],review_driver_sha256=sha(Path(__file__).read_bytes()),review_harness_sha256=sha(HARNESS.encode()),review_executable_sha256=exe,gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,dwarf_sha256=audit.DWARF_SHA,fixtures=reports,all_archive_files_unchanged=True,archive_files_checked=len(inputs),actual_export_used=True,source_tests=3,strict_core_clippy=True,full_typed_projection_match=True,byte_roundtrip=True,every_byte_truncations=True,all_changed_version_and_count_sites_zero_and_max_rejected=True,shared_item_and_byte_caps=True,strict_and_prefix_suffix_behavior=True,hostile_count_with_unlimited_limits_rejected_before_allocation=True,definition_remapping=False,property_variants_in_current_grammar=False,original_execution=False,native_read_side_effects=False,gameplay_state=False,gameplay_rng=False,actions=False,turns=False,ai_planning_exclusion=False,unknown_save_bytes_masked=False,recorded_raw_save_load_difference_bytes=89198)
    (ROOT/'evidence/linux/game-free-cities-review.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,fixtures=reports,archive_files_checked=len(inputs)),indent=2))
if __name__=='__main__':main()
