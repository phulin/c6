#!/usr/bin/env python3
"""Focused post-Trade wire review: one committed archive, static gates, offline records."""
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
SOURCE='8a99f1fba96a5f189545751c323a4a6ce2f35e11'
def fields(names,kind):return [(n,kind)for n in names.split()]
SPECS={
 'great_people_points':{'type':'GreatPeoplePoints','owner':'GreatPeoplePoints','version':13,'version_address':0x2f617c4,'dwarf':['GameCore::Player::GreatPeoplePoints','GameCore::Player::GreatPersonEarnedBoost','GameCore::Player::YieldChangePerEarnedGreatPersonClassOnResourceType'],'schema':fields('player','i32')+fields('points_total points_earned_this_turn bonus_points_per_turn','table')+fields('free_great_person_points points_refund_percent','i32')+fields('no_patronage_faith no_patronage_gold','bool')+fields('patronage_discount_percent bonus_points_per_turn_percent yield_modifier_per_earned_great_person','table')+fields('yield_change_per_earned_great_person_class','changes')+fields('great_person_boost_list','boosts')+fields('great_person_class_no_points','u32s')},
 'influence':{'type':'Influence','owner':'Influence','version':22,'version_address':0x2f61f44,'dwarf':['GameCore::Player::Influence'],'schema':fields('player points_earned base_points_per_turn','i32')+fields('not_earning_points','bool')+fields('points_threshold tokens_per_threshold tokens_to_give duplicate_first_tokens tokens_on_first_to_meet tokens_on_trade_route_started','i32')+fields('tokens_received virtual_tokens_received','ints')+fields('disallow_tokens_from','bools')+fields('yield_per_given_token','table')+fields('suzerain suzerain_changed_count suzerain_unique_bonus_disabled_count levy_turn_counter','i32')+fields('yield_modifier_per_tributary yield_per_tributary','table')+fields('open_borders_count levy_discount_percent_by_player copy_luxuries_count copy_strategics_count','ints')+fields('duplicate_tokens_when_rival_government duplicate_tokens_when_trade_route_to duplicate_tokens_when_same_religion suzerain_visibility_counter levy_discount_percent','i32')+fields('tokens_received_modifier max_tokens_received','ints')},
 'quests':{'type':'Quests','owner':'Quests','version':1,'version_address':0x2f665b0,'dwarf':['GameCore::Player::Quests'],'schema':fields('player','i32')},
 'wmds':{'type':'Wmds','owner':'WMDs','version':1,'version_address':0x2f6b014,'dwarf':['GameCore::Player::WMDs'],'schema':fields('weapons','weapons')},
}
def sha(b):return hashlib.sha256(b).hexdigest()
def run(a,**kw):return subprocess.check_output(a,**kw)

def parse(data,spec):
 pos=0;items=0;tokens=[];counts=[];bools=[];versions=[];shape=[]
 def word(signed=False):
  nonlocal pos
  v=struct.unpack_from('<i'if signed else '<I',data,pos)[0];pos+=4;tokens.append(v);return v
 def boolean():
  nonlocal pos
  bools.append(pos);v=data[pos];assert v in [0,1];pos+=1;tokens.append(v)
 def version():
  versions.append(pos);assert word()==spec['version']
 version()
 for name,kind in spec['schema']:
  if kind=='i32':word(True)
  elif kind=='bool':boolean()
  else:
   counts.append(pos);n=word();items+=n;shape.append([name,n])
   for _ in range(n):
    if kind in ['table','weapons']:word();word(True)
    elif kind=='ints':word(True)
    elif kind=='u32s':word()
    elif kind=='bools':boolean()
    elif kind=='changes':version();word();word();word();word(True)
    elif kind=='boosts':version();word();boolean();boolean()
    else:raise ValueError(kind)
 return pos,items,tokens,counts,bools,versions,shape

def synthetic(spec):
 out=bytearray();serial=0
 def word(v):out.extend(struct.pack('<I',v&0xffffffff))
 word(spec['version'])
 for _,kind in spec['schema']:
  serial+=1
  if kind=='i32':word(-257*serial)
  elif kind=='bool':out.append(serial%2)
  else:
   word(3)
   for i in range(3):
    if kind in ['table','weapons']:word([0xffffffff,0x80000001,0x80000001][i]);word([-2147483648,2147483647,-257][i])
    elif kind=='ints':word([-2147483648,2147483647,-1][i])
    elif kind=='u32s':word([0xffffffff,0,0xffffffff][i])
    elif kind=='bools':out.append(i%2)
    elif kind=='changes':word(spec['version']);word(0x80000001);word(0xffffffff);word(0xffffffff);word([-2147483648,2147483647,-257][i])
    elif kind=='boosts':word(spec['version']);word(0xffffffff);out.extend([i%2,(i+1)%2])
 return bytes(out)

HARNESS=r'''use c6_core::unit::Limits;
use c6_core::player_MODULE::TYPE;
fn main(){let a:Vec<_>=std::env::args().collect();let b=std::fs::read(&a[1]).unwrap();
 if a.len()>3{assert!(TYPE::decode(&b,Limits{max_bytes:usize::MAX,max_items:usize::MAX}).is_err());return;}
 let loose=Limits{max_bytes:64*1024*1024,max_items:4_000_000};
 let(v,n,items)=TYPE::decode_prefix_with_items(&b,loose).unwrap();
 let record=&b[..n];let exact=Limits{max_bytes:n,max_items:items};
 assert_eq!(TYPE::decode(record,exact).unwrap(),v);
 let(out,charged)=v.encode_with_items(exact).unwrap();assert_eq!(out,record);assert_eq!(charged,items);
 for k in 0..n{assert!(TYPE::decode(&b[..k],exact).is_err());}
 let short=Limits{max_bytes:n-1,..exact};assert!(TYPE::decode(record,short).is_err());assert!(v.encode(short).is_err());
 if items>0{let short=Limits{max_items:items-1,..exact};assert!(TYPE::decode(record,short).is_err());assert!(v.encode(short).is_err());}
 let mut tail=record.to_vec();tail.extend_from_slice(&vec![0xa5;1024*1024]);
 assert_eq!(TYPE::decode_prefix_with_items(&tail,exact).unwrap(),(v.clone(),n,items));
 assert_eq!(TYPE::decode_prefix(&tail,exact).unwrap(),(v.clone(),n));assert!(TYPE::decode(&tail,loose).is_err());
 let mut joined=record.repeat(2);joined.extend_from_slice(&[0xa5;17]);let mut remaining=items*2;let mut cursor=0;
 for _ in 0..2{let(_,used,charged)=TYPE::decode_prefix_with_items(&joined[cursor..],Limits{max_bytes:n,max_items:remaining}).unwrap();assert_eq!((used,charged),(n,items));cursor+=used;remaining-=charged;}
 assert_eq!((cursor,remaining),(n*2,0));
 if items>0{let(_,used,charged)=TYPE::decode_prefix_with_items(&joined,Limits{max_bytes:n,max_items:items*2-1}).unwrap();assert_eq!((used,charged),(n,items));assert!(TYPE::decode_prefix_with_items(&joined[n..],Limits{max_bytes:n,max_items:items-1}).is_err());}
 let mut t=vec![VERSIONi64];PROJECTION
 std::fs::write(&a[2],out).unwrap();println!("{} {}",n,items);println!("{:?}",t);
}
'''
def harness(name,spec):
 lines=[]
 for field,kind in spec['schema']:
  f='v.'+field
  if kind=='i32':lines.append('t.push('+f+' as i64);')
  elif kind=='bool':lines.append('t.push(i64::from('+f+'));')
  else:
   lines.append('t.push('+f+'.len() as i64);')
   if kind=='table':body='t.push(x.0 as i64);t.push(x.1 as i64);'
   elif kind=='weapons':body='t.push(x.definition_hash as i64);t.push(x.count as i64);'
   elif kind in ['ints','u32s']:body='t.push(*x as i64);'
   elif kind=='bools':body='t.push(i64::from(*x));'
   elif kind=='changes':body='t.push(13);t.push(x.yield_hash as i64);t.push(x.great_person_class_hash as i64);t.push(x.resource_hash as i64);t.push(x.amount as i64);'
   elif kind=='boosts':body='t.push(13);t.push(x.great_person_class_hash as i64);t.push(i64::from(x.tech_boost));t.push(i64::from(x.other_players));'
   lines.append('for x in &'+f+'{'+body+'}')
 return HARNESS.replace('MODULE',name).replace('TYPE',spec['type']).replace('VERSION',str(spec['version'])).replace('PROJECTION','\n'.join(lines))

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--output',type=Path,required=True);args=ap.parse_args()
 assets=ROOT.parent/'c6-assets-linux';private=ROOT.parent/'c6-ghidra';packed=run(['git','archive',SOURCE],cwd=ROOT)
 with tempfile.TemporaryDirectory(prefix='c6-post-trade-review-')as temp:
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
   assert struct.unpack('<I',game.read(spec['version_address'],4))[0]==spec['version']
   dwarf=''.join(run(['gdb','-nx','-batch',str(assets/'libGameCore_XP2.so'),'-ex',"ptype /o '"+n+"'"],text=True)for n in spec['dwarf']);assert sha(dwarf.encode())==audit.DWARF_SHA
   contracts[name]={'methods':len(audit.METHODS),'bindings':len(audit.BINDINGS),'virtuals':len(audit.VIRTUALS),'method_manifest_sha256':sha(json.dumps(audit.METHODS,separators=(',',':')).encode()),'dwarf_sha256':audit.DWARF_SHA}
   result=run(['cargo','test','--offline','-p','c6-core','--lib','player_'+name+'::tests'],cwd=source,text=True);tests[name]=next(x for x in result.splitlines()if x.startswith('test result:'))
  subprocess.run(['cargo','build','--offline','--release','-p','c6-core'],cwd=source,check=True)
  for name,spec in SPECS.items():
   text=harness(name,spec);(temp/(name+'.rs')).write_text(text)
   subprocess.run(['rustc','--edition=2021','-Dwarnings','-O',str(temp/(name+'.rs')),'--extern','c6_core='+str(source/'target/release/libc6_core.rlib'),'-L','dependency='+str(source/'target/release/deps'),'-o',str(temp/name)],check=True)
   candidates[name]={'harness_sha256':sha(text.encode()),'candidate_sha256':sha((temp/name).read_bytes())}
  def compare(name,data):
   n,items,tokens,counts,bools,versions,shape=parse(data,SPECS[name]);(temp/'in').write_bytes(data)
   lines=run([str(temp/name),str(temp/'in'),str(temp/'out')],text=True).splitlines();assert [int(x)for x in lines[0].split()]==[n,items]
   assert (temp/'out').read_bytes()==data[:n]and json.loads(lines[1])==tokens
   return n,items,counts,bools,versions,shape
  for name,spec in SPECS.items():
   data=synthetic(spec);n,items,counts,bools,versions,_=compare(name,data);assert n==len(data);negative=0
   for offset in counts+versions:
    bad=bytearray(data);struct.pack_into('<I',bad,offset,0xffffffff);(temp/'in').write_bytes(bad)
    subprocess.run([str(temp/name),str(temp/'in'),str(temp/'out'),'reject'],check=True);negative+=1
   for offset in bools:
    bad=bytearray(data);bad[offset]=2;(temp/'in').write_bytes(bad)
    subprocess.run([str(temp/name),str(temp/'in'),str(temp/'out'),'reject'],check=True);negative+=1
   synthetics[name]={'bytes':n,'items':items,'hostile_mutations_with_huge_limits':negative}
  fixtures=[]
  for identity,trace_name,trace_sha in FIXTURES:
   data=gzip.decompress((private/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes());assert sha(data)==identity
   trace=gzip.decompress((private/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes());assert sha(trace)==trace_sha
   rows=[x for x in map(json.loads,trace.splitlines())if x.get('event')=='owner'and x.get('save')==1];bycall={x['call']:x for x in rows};report={'stream_sha256':identity,'trace_sha256':trace_sha,'components':{}}
   assert not any(x.get('owner')=='GameCore::Player::WMDs'for x in rows)
   for name,spec in SPECS.items():
    owner='GameCore::Player::'+('Quests'if name=='wmds'else spec['owner']);cuts=sorted([x for x in rows if x.get('owner')==owner],key=lambda x:x['start']);assert len(cuts)==43
    observations=[];digests=[];first_shape=None
    for cut in cuts:
     if name=='wmds':
      parent=bycall[cut['parent']];assert parent['owner']=='GameCore::Player::Instance';start=cut['end'];payload=data[start:parent['end']]
      version,count=struct.unpack_from('<II',payload);expected=8+8*count;assert version==1 and expected<=len(payload)
     else:start=cut['start'];payload=data[start:cut['end']];expected=len(payload)
     n,items,_,_,_,shape=compare(name,payload);assert n==expected
     if first_shape is None:first_shape=shape
     observations.append([n,items]);digests.append(sha(payload[:n]))
    report['components'][name]={'records':len(cuts),'bytes':sum(x[0]for x in observations),'items':sum(x[1]for x in observations),'byte_item_shapes':sorted({tuple(x)for x in observations}),'first_field_shapes':first_shape,'ordered_record_digests_sha256':sha(json.dumps(digests,separators=(',',':')).encode()),'boundary_evidence':'Quests-end to enclosing Player-end contextual suffix; independent unsigned-count extent; no WMD owner hook'if name=='wmds'else'actual component serializer owner cuts'}
   fixtures.append(report)
  files=['Cargo.toml','Cargo.lock','crates/c6-core/Cargo.toml','crates/c6-core/src/lib.rs']+['crates/c6-core/src/player_'+n+'.rs'for n in SPECS]+['tools/linux/player_'+n+'_audit.py'for n in SPECS]
  result={'schema':'c6-linux-gpp-influence-quests-wmds-independent-review-v1','checks_passed':True,'source_commit':SOURCE,'archive_sha256':sha(packed),'review_script_sha256':sha(Path(__file__).read_bytes()),'source_sha256':{p:sha((source/p).read_bytes())for p in files},'candidates':candidates,'static_contracts':contracts,'gamecore_sha256':GAME_SHA,'host_sha256':HOST_SHA,'source_tests':tests,'synthetics':synthetics,'fixtures':fixtures,'independent_projection':'Python wire grammar and dynamic-item algebra versus every Rust typed scalar/count/hash/Q8/bool token; nested GPP versions checked explicitly','each_record_checks':['every truncated prefix','exact and one-short byte budgets','exact and one-short nonzero item budgets; zero-item Quests explicitly valid','one MiB unparsed tail accepted at exact prefix caps and rejected by strict decode','two concatenated records with exact and one-short shared items'],'native_branch_gaps':['GPP nonempty yield-change, earned-boost and no-points vectors are source-only','Influence nonempty levy-discount-percent-by-player vector is source-only','WMD consumed boundary is count-derived inside contextual native suffix, not an independently hooked WMD end'],'native_read_initialize_reset_remapping_reproduced':False,'callbacks':False,'original_execution':False,'private_writes':False,'full_43_players':False,'full_gameplay_state':False,'all_gameplay_rng':False,'actions':False,'turns':False,'raw_save_load_difference_bytes':89198,'unknown_bytes_masked':False,'whole_prefix_acceptance':'separate frozen producer/composition review'}
  args.output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'checks_passed':True,'source':SOURCE,'records_per_component':86,'components':list(SPECS)}))
if __name__=='__main__':main()
