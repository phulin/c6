#!/usr/bin/env python3
"""Independent CulturalIdentity/Heroes current-wire review using actual native owner cuts."""
import argparse
import gzip
import hashlib
import importlib
import json
from pathlib import Path
import struct
import subprocess
import sys
import tempfile

ROOT=Path(__file__).resolve().parents[2]
SOURCE='59e30ecfb4c7722a4226a3d24128541af2f1b2fb'
CULTURAL_FIELDS='extra_capital_pressure_radius identity_per_turn_for_domestic_trade_route_origin_raw martial_law_modifier post_combat_foreign_loyalty post_combat_local_loyalty additional_golden_age_post_combat_loyalty post_pillage_foreign_loyalty post_pillage_local_loyalty get_skip_free_city_step foreign_identity_per_turn_from_nearby_great_works_raw domestic_identity_per_turn_from_nearby_great_works_raw foreign_identity_per_turn_from_nearby_wonders_raw domestic_identity_per_turn_from_nearby_wonders_raw always_loyal_coastal_home_continent_count'.split()
SPECS={'cultural_identity':{'type':'CulturalIdentity','owner':'CulturalIdentity','version':10,'dwarf':['GameCore::Player::CulturalIdentity']},'heroes':{'type':'Heroes','owner':'Heroes','version':6,'dwarf':['GameCore::Player::Heroes','GameCore::Player::HeroDiscoveryWeightData','GameCore::Player::HeroClaimData']}}
def sha(b):return hashlib.sha256(b).hexdigest()
def run(a,**kw):return subprocess.check_output(a,**kw)

class Parser:
 def __init__(self,data):self.data=data;self.pos=0;self.items=0;self.tokens=[];self.counts=[];self.bools=[];self.versions=[];self.shape=[]
 def word(self,signed=False):
  v=struct.unpack_from('<i'if signed else '<I',self.data,self.pos)[0];self.pos+=4;self.tokens.append(v);return v
 def version(self,n):self.versions.append(self.pos);assert self.word()==n
 def count(self):self.counts.append(self.pos);n=self.word();self.items+=n;self.shape.append(n);return n
 def boolean(self):
  self.bools.append(self.pos);v=self.data[self.pos];assert v in [0,1];self.pos+=1;self.tokens.append(v)
 def parse(self,name):
  if name=='cultural_identity':
   self.version(10)
   for _ in range(14):self.word(True)
  else:
   self.version(6)
   for _ in range(self.count()):self.word();self.boolean()
   self.word(True)
   for _ in range(self.count()):self.word(True)
   for _ in range(self.count()):self.word(True)
   for _ in range(self.count()):self.version(6);self.word();self.word(True)
   for _ in range(self.count()):self.version(6);self.word();self.word();self.word(True);self.word()
  return self

def specimen(name):
 b=bytearray()
 def word(v):b.extend(struct.pack('<I',v&0xffffffff))
 if name=='cultural_identity':
  word(10)
  for v in [-2147483648,2147483647,-1,0,1,255,256,-256,257,-257,0x12345678,-123456789,42,-42]:word(v)
 else:
  word(6);word(3)
  for i in range(3):word(0xffffffff if i<2 else 0x80000001);b.append(i%2)
  word(-2147483648)
  for _ in range(2):
   word(3)
   for v in [-2147483648,2147483647,-257]:word(v)
  word(3)
  for i in range(3):word(6);word(0xffffffff);word([-2147483648,2147483647,-1][i])
  word(3)
  for i in range(3):word(6);word(0xffffffff);word(0x80000001);word([-2147483648,2147483647,-257][i]);word(0xffffffff)
 return bytes(b)

HARNESS=r'''use c6_core::{unit::Limits,player_MODULE::TYPE};
fn main(){let a:Vec<_>=std::env::args().collect();let b=std::fs::read(&a[1]).unwrap();
 if a.len()>3{assert!(TYPE::decode(&b,Limits{max_bytes:usize::MAX,max_items:usize::MAX}).is_err());return;}
 let loose=Limits{max_bytes:64*1024*1024,max_items:4_000_000};let(v,n,items)=TYPE::decode_prefix_with_items(&b,loose).unwrap();assert_eq!(n,b.len());
 let exact=Limits{max_bytes:n,max_items:items};assert_eq!(TYPE::decode(&b,exact).unwrap(),v);
 let(out,charged)=v.encode_with_items(exact).unwrap();assert_eq!(out,b);assert_eq!(charged,items);
 for k in 0..n{assert!(TYPE::decode(&b[..k],exact).is_err());let cap=Limits{max_bytes:k,..exact};assert!(TYPE::decode(&b,cap).is_err());assert!(v.encode(cap).is_err());}
 if items>0{let short=Limits{max_items:items-1,..exact};assert!(TYPE::decode(&b,short).is_err());assert!(v.encode(short).is_err());}
 let mut tail=b.clone();tail.extend_from_slice(&vec![0xa5;1024*1024]);
 assert_eq!(TYPE::decode_prefix_with_items(&tail,exact).unwrap(),(v.clone(),n,items));assert_eq!(TYPE::decode_prefix(&tail,exact).unwrap(),(v.clone(),n));assert!(TYPE::decode(&tail,loose).is_err());
 let mut joined=b.repeat(2);joined.extend_from_slice(&[0xa5;17]);let mut remaining=items*2;let mut cursor=0;
 for _ in 0..2{let(_,used,charged)=TYPE::decode_prefix_with_items(&joined[cursor..],Limits{max_bytes:n,max_items:remaining}).unwrap();assert_eq!((used,charged),(n,items));cursor+=used;remaining-=charged;}
 assert_eq!((cursor,remaining),(n*2,0));if items>0{let(_,used,charged)=TYPE::decode_prefix_with_items(&joined,Limits{max_bytes:n,max_items:items*2-1}).unwrap();assert_eq!((used,charged),(n,items));assert!(TYPE::decode_prefix_with_items(&joined[n..],Limits{max_bytes:n,max_items:items-1}).is_err());}
 let mut t=Vec::<i64>::new();PROJECTION
 std::fs::write(&a[2],out).unwrap();println!("{} {}",n,items);println!("{:?}",t);
}
'''
HERO_PROJECTION=r'''
t.extend([6,v.discovered.len() as i64]);for d in &v.discovered{t.extend([d.hero_class_hash as i64,i64::from(d.discovered)]);}
t.extend([v.goody_hut_discovery_chance as i64,v.player_influence_chances.len() as i64]);t.extend(v.player_influence_chances.iter().map(|x|*x as i64));
t.push(v.player_suzerain_count.len() as i64);t.extend(v.player_suzerain_count.iter().map(|x|*x as i64));
t.push(v.discovery_order.len() as i64);for d in &v.discovery_order{t.extend([6,d.hero_class_hash as i64,d.weight as i64]);}
t.push(v.claims.len() as i64);for c in &v.claims{t.extend([6,c.hero_class_hash as i64,c.claim_era_hash as i64,c.claim_turn as i64,c.last_appearance_era_hash as i64]);}
'''

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--output',type=Path,required=True);args=ap.parse_args()
 assets=ROOT.parent/'c6-assets-linux';private=ROOT.parent/'c6-ghidra';packed=run(['git','archive',SOURCE],cwd=ROOT)
 with tempfile.TemporaryDirectory(prefix='c6-cultural-heroes-review-')as temp:
  temp=Path(temp);source=temp/'source';source.mkdir();subprocess.run(['tar','-x','-C',str(source)],input=packed,check=True);sys.path.insert(0,str(source/'tools/linux'))
  from save_lifecycle_audit import Elf,GAME_SHA,HOST_SHA
  from player_districts_audit import FIXTURES
  from player_religion_audit import relocation_symbols
  game=Elf(assets/'libGameCore_XP2.so',GAME_SHA);Elf(assets/'Civ6',HOST_SHA);relocs=relocation_symbols(game);symbols={}
  for row in run(['nm','-S','-C',str(assets/'libGameCore_XP2.so')],text=True).splitlines():
   p=row.split(' ',3)
   if len(p)==4 and p[2]in('T','W'):symbols[p[3]]=(int(p[0],16),int(p[1],16))
  contracts={};tests={};candidates={};synthetics={}
  for name,spec in SPECS.items():
   audit=importlib.import_module('player_'+name+'_audit')
   for address,size,symbol,digest in audit.METHODS:assert symbols[symbol]==(address,size)and sha(game.read(address,size))==digest
   for address,body in audit.VIRTUALS:assert game.relocated_pointer(address)==body
   for site,plt,body,kind in audit.BINDINGS:
    code=game.read(site,5);assert code[0]==(0xe8 if kind=='call'else 0xe9)and site+5+struct.unpack_from('<i',code,1)[0]==plt
    stub=game.read(plt,6);assert stub[:2]==b'\xff\x25'and relocs.get(plt+6+struct.unpack_from('<i',stub,2)[0])==body
   dwarf=''.join(run(['gdb','-nx','-batch',str(assets/'libGameCore_XP2.so'),'-ex',"ptype /o '"+n+"'"],text=True)for n in spec['dwarf']);assert sha(dwarf.encode())==audit.DWARF_SHA
   contracts[name]={'methods':len(audit.METHODS),'bindings':len(audit.BINDINGS),'virtuals':len(audit.VIRTUALS),'method_manifest_sha256':sha(json.dumps(audit.METHODS,separators=(',',':')).encode()),'dwarf_sha256':audit.DWARF_SHA}
   result=run(['cargo','test','--offline','-p','c6-core','--lib','player_'+name+'::tests'],cwd=source,text=True);tests[name]=next(x for x in result.splitlines()if x.startswith('test result:'))
  for address,version in [(0x2f579f8,10),(0x2f023ac,6)]:assert struct.unpack('<I',game.read(address,4))[0]==version
  game.call(0x2b4f1a4,0x1ee63f0);game.call(0x2b4f1b7,0x1eadda0)
  subprocess.run(['cargo','clippy','--offline','-p','c6-core','--all-targets','--','-D','warnings'],cwd=source,check=True)
  subprocess.run(['cargo','build','--offline','--release','-p','c6-core'],cwd=source,check=True)
  for name,spec in SPECS.items():
   projection='t.push(10);'+''.join('t.push(v.'+f+' as i64);'for f in CULTURAL_FIELDS)if name=='cultural_identity'else HERO_PROJECTION
   text=HARNESS.replace('MODULE',name).replace('TYPE',spec['type']).replace('PROJECTION',projection);(temp/(name+'.rs')).write_text(text)
   subprocess.run(['rustc','--edition=2021','-Dwarnings','-O',str(temp/(name+'.rs')),'--extern','c6_core='+str(source/'target/release/libc6_core.rlib'),'-L','dependency='+str(source/'target/release/deps'),'-o',str(temp/name)],check=True)
   candidates[name]={'harness_sha256':sha(text.encode()),'candidate_sha256':sha((temp/name).read_bytes())}
  def compare(name,data):
   parsed=Parser(data).parse(name);assert parsed.pos==len(data);(temp/'in').write_bytes(data);lines=run([str(temp/name),str(temp/'in'),str(temp/'out')],text=True).splitlines()
   assert [int(x)for x in lines[0].split()]==[parsed.pos,parsed.items]
   assert (temp/'out').read_bytes()==data and json.loads(lines[1])==parsed.tokens
   return parsed
  for name in SPECS:
   data=specimen(name);p=compare(name,data);negative=0
   for offset in p.counts+p.versions:
    bad=bytearray(data);struct.pack_into('<I',bad,offset,0xffffffff);(temp/'in').write_bytes(bad);subprocess.run([str(temp/name),str(temp/'in'),str(temp/'out'),'reject'],check=True);negative+=1
   for offset in p.bools:
    bad=bytearray(data);bad[offset]=2;(temp/'in').write_bytes(bad);subprocess.run([str(temp/name),str(temp/'in'),str(temp/'out'),'reject'],check=True);negative+=1
   synthetics[name]={'bytes':p.pos,'items':p.items,'hostile_mutations_with_huge_limits':negative}
  fixtures=[]
  for identity,trace_name,trace_sha in FIXTURES:
   data=gzip.decompress((private/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes());assert sha(data)==identity
   trace=gzip.decompress((private/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes());assert sha(trace)==trace_sha
   rows=[x for x in map(json.loads,trace.splitlines())if x.get('event')=='owner'and x.get('save')==1];bycall={x['call']:x for x in rows}
   cuts={name:sorted([x for x in rows if x.get('owner')=='GameCore::Player::'+spec['owner']],key=lambda x:x['start'])for name,spec in SPECS.items()}
   assert all(len(x)==43 for x in cuts.values());byparent={name:{x['parent']:x for x in cuts[name]}for name in SPECS};assert all(len(x)==43 for x in byparent.values())
   assert set(byparent['cultural_identity'])==set(byparent['heroes'])
   for parent,cultural in byparent['cultural_identity'].items():
    hero=byparent['heroes'][parent];assert bycall[parent]['owner']=='GameCore::Player::Instance';assert cultural['end']==hero['start']
   report={'stream_sha256':identity,'trace_sha256':trace_sha,'components':{},'matching_player_parents':43,'native_cultural_end_equals_native_heroes_entry':True}
   for name in SPECS:
    observations=[];digests=[];shapes=[]
    for cut in cuts[name]:
     payload=data[cut['start']:cut['end']];p=compare(name,payload);observations.append([p.pos,p.items]);digests.append(sha(payload));shapes.append(p.shape)
    report['components'][name]={'records':len(cuts[name]),'bytes':sum(x[0]for x in observations),'items':sum(x[1]for x in observations),'byte_item_shapes':sorted({tuple(x)for x in observations}),'dynamic_collection_shapes':sorted({tuple(x)for x in shapes}),'ordered_record_digests_sha256':sha(json.dumps(digests,separators=(',',':')).encode()),'boundary_evidence':'actual retained component serializer owner hooks, matched to Player parent'}
   fixtures.append(report)
  files=['Cargo.toml','Cargo.lock','crates/c6-core/Cargo.toml','crates/c6-core/src/lib.rs']+['crates/c6-core/src/player_'+n+'.rs'for n in SPECS]+['tools/linux/player_'+n+'_audit.py'for n in SPECS]
  result={'schema':'c6-linux-cultural-identity-heroes-independent-review-v1','checks_passed':True,'source_commit':SOURCE,'archive_sha256':sha(packed),'review_script_sha256':sha(Path(__file__).read_bytes()),'source_sha256':{p:sha((source/p).read_bytes())for p in files},'candidates':candidates,'static_contracts':contracts,'gamecore_sha256':GAME_SHA,'host_sha256':HOST_SHA,'source_tests':tests,'actual_strict_cargo_clippy':'cargo clippy --offline -p c6-core --all-targets -- -D warnings','synthetics':synthetics,'fixtures':fixtures,'independent_projection':'Python grammar and item interpreter versus every Rust typed signed/raw Q8 word, hash, bool, vector count, discovery weight and claim field; direct owner extents must match','source_only_branches':['Heroes nonempty discovered hash/bool table','Heroes nonempty nested discovery-order and claim records; nested versions','CulturalIdentity signed extremes and nonzero raw Q8 carriers beyond corpus occupancy'],'historical_reports_preserved':True,'additive_owner_qualifications':['evidence/linux/player-cultural-identity-owner-hook-qualification.json at f521fd4','evidence/linux/player-heroes-owner-hook-qualification.json at 9b45933'],'parent_branch_scope':'Heroes is unconditional. PlayerWasEverAlive and following arrays/AI blocks belong to separate parent composition proof and are not tested or accepted by this standalone review.','native_read_remapping_initialization_effects_reproduced':False,'original_execution':False,'private_writes':False,'full_43_players':False,'full_gameplay_state':False,'all_gameplay_rng':False,'actions':False,'turns':False,'raw_save_load_difference_bytes':89198,'unknown_bytes_masked':False,'whole_prefix_acceptance':'separate frozen producer/composition review'}
  args.output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'checks_passed':True,'source':SOURCE,'actual_owner_records_per_component':86,'components':list(SPECS)}))
if __name__=='__main__':main()
