#!/usr/bin/env python3
"""Focused Improvements/Governors review from one committed archive; offline only."""
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
SOURCE='356109b8b8e4c79a9e76a618cf108fe082dc7568'
SPECS={'improvements':{'type':'Improvements','version':1,'dwarf':['GameCore::Player::Improvements']},'governors':{'type':'Governors','version':6,'dwarf':['GameCore::Player::Governors','GameCore::Governor::Instance','GameCore::Governor::GrievenceScoreModifier']}}
def sha(b):return hashlib.sha256(b).hexdigest()
def run(a,**kw):return subprocess.check_output(a,**kw)

class Parser:
 def __init__(self,data):self.data=data;self.pos=0;self.items=0;self.tokens=[];self.counts=[];self.bools=[];self.versions=[];self.instances=[]
 def word(self,signed=False):
  v=struct.unpack_from('<i'if signed else '<I',self.data,self.pos)[0];self.pos+=4;self.tokens.append(v);return v
 def half(self):
  v=struct.unpack_from('<h',self.data,self.pos)[0];self.pos+=2;self.tokens.append(v);return v
 def version(self,n):self.versions.append(self.pos);assert self.word()==n
 def boolean(self):
  self.bools.append(self.pos);v=self.data[self.pos];assert v in [0,1];self.pos+=1;self.tokens.append(v)
 def count(self,signed=False):
  self.counts.append(self.pos);n=self.word();assert not signed or n<=0x7fffffff;self.items+=n;return n
 def city(self):self.half();self.half();self.word()
 def governor(self):
  start=self.pos;self.version(16);self.word(True);self.word()
  for _ in range(self.count()):v=self.data[self.pos];self.pos+=1;self.tokens.append(v)
  self.city()
  for _ in range(7):self.word(True)
  self.boolean()
  for _ in range(7):self.word(True)
  self.boolean()
  for _ in range(3):self.word(True)
  for _ in range(self.count(True)):self.city();self.word(True)
  for _ in range(self.count()):self.word();self.boolean()
  self.boolean()
  for _ in range(self.count()):self.version(1);self.word(True);self.word(True)
  self.instances.append([start,self.pos])
 def parse(self,name):
  if name=='improvements':
   self.version(1)
   for _ in range(self.count()):self.word(True)
   self.word()
  else:
   self.version(6);self.word(True);self.version(65537);assert self.word()==16;cap=self.word();n=self.count();assert n<=cap
   for _ in range(n):self.word(True);self.governor()
   self.word();n=self.count();self.word();self.word()
   for _ in range(n):self.word()
   self.word(True);self.boolean();self.word();self.word(True)
  return self

def specimen(name):
 b=bytearray()
 def word(v):b.extend(struct.pack('<I',v&0xffffffff))
 def city():b.extend(struct.pack('<hhI',-32768,32767,0xffffffff))
 if name=='improvements':
  word(1);word(4)
  for x in [-2147483648,0,2147483647,-2147483648]:word(x)
  word(0xffffffff)
 else:
  for x in [6,-2147483648,65537,16,12,2]:word(x)
  for slot in [-7,-7]:
   for x in [slot,16,-2147483648,0xffffffff,4]:word(x)
   b.extend([0,255,0xc3,0xa9]);city()
   for x in range(7):word(-257*(x+1))
   b.append(1)
   for x in range(7):word(257*(x+1))
   b.append(0)
   for x in range(3):word(-514*(x+1))
   word(2)
   for _ in range(2):city();word(-2147483648)
   word(2)
   for value in [1,0]:word(0xffffffff);b.append(value)
   b.append(1);word(2)
   for _ in range(2):word(1);word(-2147483648);word(2147483647)
  for x in [0xffffffff,3,0x80000000,12,9,9,0xffffffff,-2147483648]:word(x)
  b.append(1);word(0x80000000);word(2147483647)
 return bytes(b)

HARNESS=r'''use c6_core::{unit::Limits,player_MODULE::TYPE};
fn main(){let a:Vec<_>=std::env::args().collect();let b=std::fs::read(&a[1]).unwrap();
 if a.len()>3{assert!(TYPE::decode(&b,Limits{max_bytes:usize::MAX,max_items:usize::MAX}).is_err());return;}
 let loose=Limits{max_bytes:64*1024*1024,max_items:4_000_000};let(v,n,items)=TYPE::decode_prefix_with_items(&b,loose).unwrap();
 let record=&b[..n];let exact=Limits{max_bytes:n,max_items:items};assert_eq!(TYPE::decode(record,exact).unwrap(),v);
 let(out,charged)=v.encode_with_items(exact).unwrap();assert_eq!(out,record);assert_eq!(charged,items);
 for k in 0..n{assert!(TYPE::decode(&b[..k],exact).is_err());}
 let short=Limits{max_bytes:n-1,..exact};assert!(TYPE::decode(record,short).is_err());assert!(v.encode(short).is_err());
 if items>0{let short=Limits{max_items:items-1,..exact};assert!(TYPE::decode(record,short).is_err());assert!(v.encode(short).is_err());}
 let mut tail=record.to_vec();tail.extend_from_slice(&vec![0xa5;1024*1024]);
 assert_eq!(TYPE::decode_prefix_with_items(&tail,exact).unwrap(),(v.clone(),n,items));assert_eq!(TYPE::decode_prefix(&tail,exact).unwrap(),(v.clone(),n));assert!(TYPE::decode(&tail,loose).is_err());
 let mut joined=record.repeat(2);joined.extend_from_slice(&[0xa5;17]);let mut remaining=items*2;let mut cursor=0;
 for _ in 0..2{let(_,used,charged)=TYPE::decode_prefix_with_items(&joined[cursor..],Limits{max_bytes:n,max_items:remaining}).unwrap();assert_eq!((used,charged),(n,items));cursor+=used;remaining-=charged;}
 assert_eq!((cursor,remaining),(n*2,0));if items>0{let(_,used,charged)=TYPE::decode_prefix_with_items(&joined,Limits{max_bytes:n,max_items:items*2-1}).unwrap();assert_eq!((used,charged),(n,items));assert!(TYPE::decode_prefix_with_items(&joined[n..],Limits{max_bytes:n,max_items:items-1}).is_err());}
 let mut t=Vec::<i64>::new();PROJECTION
 std::fs::write(&a[2],out).unwrap();println!("{} {}",n,items);println!("{:?}",t);
}
'''
GOVERNOR_PROJECTION=r'''
t.extend([6,v.governor_points as i64,65537,16,v.capacity as i64,v.entries.len() as i64]);
for entry in &v.entries {
 let g=&entry.governor;t.extend([entry.index as i64,16,g.id as i64,g.definition_hash as i64,g.name.len() as i64]);
 t.extend(g.name.iter().map(|x|*x as i64));
 t.extend([g.assigned_city.owner as i64,g.assigned_city.component_type as i64,g.assigned_city.id as i64]);
 t.extend([g.owning_player as i64,g.turns_on_site as i64,g.cultural_pressure as i64,g.domestic_identity_pressure as i64,g.foreign_identity_pressure as i64,g.domestic_identity_pressure_once_per_city as i64,g.foreign_identity_pressure_once_per_city as i64,i64::from(g.established)]);
 t.extend([g.neutralized_turns as i64,g.neutralized_indefinitely as i64,g.identity_per_title as i64,g.alliance_points as i64,g.ignore_identity_pressure as i64,g.feature_no_improvement_appeal_modifier as i64,g.prevent_structural_damage as i64,i64::from(g.assign_to_major)]);
 t.extend([g.resource_power_provided_change as i64,g.faith_purchase_districts as i64,g.extra_heal as i64,g.established_cities.len() as i64]);
 for x in &g.established_cities{t.extend([x.city.owner as i64,x.city.component_type as i64,x.city.id as i64,x.value as i64]);}
 t.push(g.promotions.len() as i64);for x in &g.promotions{t.extend([x.definition_hash as i64,i64::from(x.earned)]);}
 t.extend([i64::from(g.govern_city_state),g.grievances.len() as i64]);for x in &g.grievances{t.extend([1,x.score as i64,x.turns as i64]);}
}
t.extend([v.current_id_bits as i64,v.free_links.len() as i64,v.free_head as i64,v.free_tail as i64]);t.extend(v.free_links.iter().map(|x|*x as i64));
t.extend([v.neutralized_indefinitely as i64,i64::from(v.title_considered),v.favored_governor_hash as i64,v.diplomatic_favor as i64]);
'''

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--output',type=Path,required=True);args=ap.parse_args()
 assets=ROOT.parent/'c6-assets-linux';private=ROOT.parent/'c6-ghidra';packed=run(['git','archive',SOURCE],cwd=ROOT)
 with tempfile.TemporaryDirectory(prefix='c6-improvements-governors-review-')as temp:
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
  for address,version in [(0x2f61dc0,1),(0x2f61330,6),(0x2f551cc,16)]:assert struct.unpack('<I',game.read(address,4))[0]==version
  subprocess.run(['cargo','clippy','--offline','-p','c6-core','--all-targets','--','-D','warnings'],cwd=source,check=True)
  subprocess.run(['cargo','build','--offline','--release','-p','c6-core'],cwd=source,check=True)
  for name,spec in SPECS.items():
   projection='t.extend([1,v.plot_indices.len() as i64]);t.extend(v.plot_indices.iter().map(|x|*x as i64));t.push(v.improvement_array_size as i64);'if name=='improvements'else GOVERNOR_PROJECTION
   text=HARNESS.replace('MODULE',name).replace('TYPE',spec['type']).replace('PROJECTION',projection);(temp/(name+'.rs')).write_text(text)
   subprocess.run(['rustc','--edition=2021','-Dwarnings','-O',str(temp/(name+'.rs')),'--extern','c6_core='+str(source/'target/release/libc6_core.rlib'),'-L','dependency='+str(source/'target/release/deps'),'-o',str(temp/name)],check=True)
   candidates[name]={'harness_sha256':sha(text.encode()),'candidate_sha256':sha((temp/name).read_bytes())}
  def compare(name,data):
   parsed=Parser(data).parse(name);(temp/'in').write_bytes(data);lines=run([str(temp/name),str(temp/'in'),str(temp/'out')],text=True).splitlines()
   assert [int(x)for x in lines[0].split()]==[parsed.pos,parsed.items]
   assert (temp/'out').read_bytes()==data[:parsed.pos]and json.loads(lines[1])==parsed.tokens
   return parsed
  for name in SPECS:
   data=specimen(name);p=compare(name,data);assert p.pos==len(data);negative=0
   for offset in p.counts+p.versions:
    bad=bytearray(data);struct.pack_into('<I',bad,offset,0xffffffff);(temp/'in').write_bytes(bad);subprocess.run([str(temp/name),str(temp/'in'),str(temp/'out'),'reject'],check=True);negative+=1
   for offset in p.bools:
    bad=bytearray(data);bad[offset]=2;(temp/'in').write_bytes(bad);subprocess.run([str(temp/name),str(temp/'in'),str(temp/'out'),'reject'],check=True);negative+=1
   if name=='governors':
    bad=bytearray(data);struct.pack_into('<I',bad,12,8);(temp/'in').write_bytes(bad);subprocess.run([str(temp/name),str(temp/'in'),str(temp/'out'),'reject'],check=True);negative+=1
    wide=bytearray(data);struct.pack_into('<I',wide,16,0xffffffff);compare(name,wide)
   synthetics[name]={'bytes':p.pos,'items':p.items,'hostile_mutations_with_huge_limits':negative,'raw_negative_duplicate_slot_transport':name=='governors'}
  fixtures=[]
  for identity,trace_name,trace_sha in FIXTURES:
   data=gzip.decompress((private/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes());assert sha(data)==identity
   trace=gzip.decompress((private/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes());assert sha(trace)==trace_sha
   rows=[x for x in map(json.loads,trace.splitlines())if x.get('event')=='owner'and x.get('save')==1];bycall={x['call']:x for x in rows};cuts=sorted([x for x in rows if x.get('owner')=='GameCore::Player::Quests'],key=lambda x:x['start']);assert len(cuts)==43
   assert not any(x.get('owner')in['GameCore::Player::Governors','GameCore::Player::Improvements']for x in rows)
   observations={n:[]for n in SPECS};digests={n:[]for n in SPECS};nonempty=0;instances=0;nested_digests=[]
   for cut in cuts:
    parent=bycall[cut['parent']];assert parent['owner']=='GameCore::Player::Instance';context=data[cut['end']:parent['end']]
    version,count=struct.unpack_from('<II',context);assert version==1;improvement_offset=8+8*count
    version,count=struct.unpack_from('<II',context,improvement_offset);assert version==1;improvement_size=12+4*count
    improvement=compare('improvements',context[improvement_offset:]);assert improvement.pos==improvement_size
    observations['improvements'].append([improvement.pos,improvement.items]);digests['improvements'].append(sha(context[improvement_offset:improvement_offset+improvement.pos]))
    start=cut['end']+improvement_offset+improvement_size;governors=compare('governors',data[start:parent['end']]);end=start+governors.pos
    next_rows=[x for x in rows if x.get('owner')=='GameCore::Player::CulturalIdentity'and x.get('parent')==parent['call']];assert len(next_rows)==1 and end==next_rows[0]['start']
    children=sorted([x for x in rows if x.get('owner')=='GameCore::Governor::Instance'and x.get('parent')==parent['call']],key=lambda x:x['start'])
    assert [[start+a,start+b]for a,b in governors.instances]==[[x['start'],x['end']]for x in children]
    nonempty+=bool(children);instances+=len(children);nested_digests.extend(sha(data[x['start']:x['end']])for x in children)
    observations['governors'].append([governors.pos,governors.items]);digests['governors'].append(sha(data[start:end]))
   fixtures.append({'stream_sha256':identity,'trace_sha256':trace_sha,'components':{n:{'records':43,'bytes':sum(x[0]for x in observations[n]),'items':sum(x[1]for x in observations[n]),'byte_item_shapes':sorted({tuple(x)for x in observations[n]}),'ordered_record_digests_sha256':sha(json.dumps(digests[n],separators=(',',':')).encode())}for n in SPECS},'governors_nonempty_envelopes':nonempty,'governor_instances':instances,'all_nested_instance_bounds_match_native_hooks':True,'governors_end_matches_next_native_cultural_identity_entry':True,'ordered_nested_instance_digests_sha256':sha(json.dumps(nested_digests,separators=(',',':')).encode())})
  files=['Cargo.toml','Cargo.lock','crates/c6-core/Cargo.toml','crates/c6-core/src/lib.rs']+['crates/c6-core/src/player_'+n+'.rs'for n in SPECS]+['tools/linux/player_'+n+'_audit.py'for n in SPECS]
  result={'schema':'c6-linux-improvements-governors-independent-review-v1','checks_passed':True,'source_commit':SOURCE,'archive_sha256':sha(packed),'review_script_sha256':sha(Path(__file__).read_bytes()),'source_sha256':{p:sha((source/p).read_bytes())for p in files},'candidates':candidates,'static_contracts':contracts,'gamecore_sha256':GAME_SHA,'host_sha256':HOST_SHA,'source_tests':tests,'actual_strict_cargo_clippy':'cargo clippy --offline -p c6-core --all-targets -- -D warnings','synthetics':synthetics,'fixtures':fixtures,'independent_projection':'Python grammar/item interpreter compared with every Rust typed scalar/hash/bool/name byte/CityId halfword/free-word token; nested Governor hooks and next component entry verified independently','boundary_qualification':'No outer Improvements or Governors hooks. Quests-end and Player-end provide context; source-closed WMD/Improvements extents locate starts. Governors end matches next CulturalIdentity hook; all Governor.Instance bounds match their own native hooks. No production offsets/traces.','slot_policy':'Raw signed slot words/order/duplicates preserved; native Write canonical indices and native Read low16 AddAt normalization/reserve/overwrite are not reproduced. Active count<=capacity is a codec consistency policy, not native Read acceptance equivalence.','native_read_effects_reproduced':False,'original_execution':False,'private_writes':False,'full_43_players':False,'full_gameplay_state':False,'all_gameplay_rng':False,'actions':False,'turns':False,'raw_save_load_difference_bytes':89198,'unknown_bytes_masked':False,'whole_prefix_acceptance':'separate frozen producer/composition review'}
  args.output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'checks_passed':True,'source':SOURCE,'records_per_component':86,'governor_instances':sum(x['governor_instances']for x in fixtures),'nonempty_envelopes':sum(x['governors_nonempty_envelopes']for x in fixtures)}))
if __name__=='__main__':main()
