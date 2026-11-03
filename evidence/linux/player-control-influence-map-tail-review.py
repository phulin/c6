#!/usr/bin/env python3
"""Focused exported Control/InfluenceMap/PlayerTail current-wire review; no original execution."""
import argparse
from collections import Counter
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
SOURCE='63c16f1dd162c518435cfbe11c0bef5facea14a4'
NAMES=['player_ai_control','player_ai_influence_map','player_tail']
CONTROL_DWARF='ca1e18547066c5adf2129b6250bb37fe60288cc703f1562020bf5b3e6d23f15f'
MAP_DWARF='8bdc4ec52e454e1eba72f248bce6cb72dbbb3074f0d46fce2bd296cba1ee533d'
def sha(b):return hashlib.sha256(b).hexdigest()
def run(a,**kw):return subprocess.check_output(a,**kw)

class Parser:
 def __init__(self,data):self.data=data;self.pos=0;self.items=0;self.tokens=[];self.counts=[];self.versions=[];self.bools=[];self.keys=[];self.kinds=Counter();self.property_modifiers=[];self.shape=[]
 def get(self,fmt):
  v=struct.unpack_from('<'+fmt,self.data,self.pos)[0];self.pos+=struct.calcsize('<'+fmt);self.tokens.append(v);return v
 def version(self,v):self.versions.append(self.pos);assert self.get('I')==v
 def count(self):self.counts.append(self.pos);n=self.get('I');self.items+=n;self.shape.append(n);return n
 def boolean(self):self.bools.append(self.pos);v=self.get('B');assert v in [0,1]
 def vector(self,fmt):
  for _ in range(self.count()):
   for t in fmt:self.boolean()if t=='?'else self.get(t)
 def control(self):
  self.version(2);self.get('i');self.get('i');self.version(65537);assert self.get('I')==16;self.get('I')
  for _ in range(self.count()):self.get('i');self.version(2);self.get('I');self.get('i');self.get('i')
  self.get('I');n=self.count();self.get('I');self.get('I')
  for _ in range(n):self.get('I')
  self.boolean();self.boolean()
  for _ in range(self.count()):
   self.keys.append(self.pos);k=self.get('i');self.kinds[str(k)]+=1
   if k==-1:continue
   assert 0<=k<=4;self.version(3)
   for t in 'iiiiiIIi':self.get(t)
   self.version(2 if k in [1,4]else 1)
   for t in ['IIIiiI?','III','II','ii','iI'][k]:self.boolean()if t=='?'else self.get(t)
  return self
 def influence_map(self):self.version(2);self.vector('h');self.vector('h');return self
 def properties(self):
  start=self.pos;token_start=len(self.tokens);first=self.get('I')
  n=self.count()if first&0x80000000 else first
  if first&0x80000000==0:self.counts.append(start);self.items+=n
  for _ in range(n):
   self.get('I')
   if first&1 and first&0x80000000:
    length=self.get('H');self.items+=length
    for _ in range(length):self.get('B')
   typ=self.get('H');self.property_modifiers.append(self.pos);modifier=self.get('H');assert typ==5 and modifier==0
   flags=self.get('I');size=self.get('I');assert flags&0xff000000==0x21000000 and size==1
   length=flags&0xffffff;self.items+=length
   for _ in range(length):self.get('B')
  del self.tokens[token_start:];self.tokens.extend([self.pos-start,*self.data[start:self.pos]])
 def tail(self,alive):
  if alive:
   for fmt in ['I','i','I','I','I','Ii','i?ii','iIIi','i']:self.vector(fmt)
  self.properties()
  for fmt in ['II','I','i','i','i']:self.vector(fmt)
  return self

def specimen(name,alive=True):
 b=bytearray()
 def put(fmt,*x):b.extend(struct.pack('<'+fmt,*x))
 if name=='player_ai_control':
  put('IiiIIII',2,-1,-257,65537,16,16,1);put('iIIii',-7,2,0xffffffff,-2147483648,2147483647)
  put('IIIIIIBB',0xffffffff,2,0x80000000,0xffffffff,7,7,0,1);put('I',6)
  for k in range(5):
   put('iIiiiiiIIiI',k,3,-2147483648,-1,2147483647,-1,0,0xffffffff,0x80000001,-257,2 if k in[1,4]else 1)
   formats=['IIIiiIB','III','II','ii','iI'];values=[(0xffffffff,0x80000001,0xffffffff,-2147483648,2147483647,0xffffffff,1),(0xffffffff,0x80000001,0xffffffff),(0xffffffff,0xffffffff),(-2147483648,2147483647),(-1,0xffffffff)];put(formats[k],*values[k])
  put('i',-1)
 elif name=='player_ai_influence_map':put('IIhhhhIhh',2,4,-32768,-1,0,32767,2,8,-8)
 else:
  if alive:
   for fmt in ['I','i','I','I','I','Ii','iBii','iIIi','i']:
    put('I',3)
    for i in range(3):
     vals=[]
     for t in fmt:vals.append((0xffffffff if i<2 else 0x80000001)if t=='I'else i%2 if t=='B'else[-2147483648,2147483647,-257][i])
     put(fmt,*vals)
  b.extend(bytes([1,0,0,128,1,0,0,0,7,0,0,0,2,0,110,109,5,0,0,0,3,0,0,33,1,0,0,0,120,121,0]))
  for fmt in ['II','I','i','i','i']:
   put('I',3)
   for i in range(3):put(fmt,*[(0xffffffff if i<2 else 0x80000001)if t=='I'else[-2147483648,2147483647,-257][i]for t in fmt])
 return bytes(b)

HARNESS=r'''use c6_core::{unit::Limits,MODULE::TYPE};
EXTRA_IMPORT
fn main(){let a:Vec<_>=std::env::args().collect();let b=std::fs::read(&a[1]).unwrap();CONTEXT
 let decode=|b:&[u8],l:Limits|TYPE::decode(b,CTXl);
 let prefix=|b:&[u8],l:Limits|TYPE::decode_prefix_with_items(b,CTXl);
 let encode=|v:&TYPE,l:Limits|v.encode_with_items(CTXl);
 if a.len()>4{assert!(decode(&b,Limits{max_bytes:usize::MAX,max_items:usize::MAX}).is_err());return;}
 let loose=Limits{max_bytes:64*1024*1024,max_items:4_000_000};let(v,n,items)=prefix(&b,loose).unwrap();assert_eq!(n,b.len());
 let exact=Limits{max_bytes:n,max_items:items};assert_eq!(decode(&b,exact).unwrap(),v);let(out,charged)=encode(&v,exact).unwrap();assert_eq!(out,b);assert_eq!(charged,items);
 for k in 0..n{assert!(decode(&b[..k],exact).is_err());}
 let short=Limits{max_bytes:n-1,..exact};assert!(decode(&b,short).is_err());assert!(encode(&v,short).is_err());
 if items>0{let short=Limits{max_items:items-1,..exact};assert!(decode(&b,short).is_err());assert!(encode(&v,short).is_err());}
 let mut tail=b.clone();tail.extend_from_slice(&vec![0xa5;1024*1024]);assert_eq!(prefix(&tail,exact).unwrap(),(v.clone(),n,items));assert!(decode(&tail,loose).is_err());
 let mut joined=b.repeat(2);joined.extend_from_slice(&[0xa5;17]);let mut remain=items*2;let mut cursor=0;
 for _ in 0..2{let(_,used,charged)=prefix(&joined[cursor..],Limits{max_bytes:n,max_items:remain}).unwrap();assert_eq!((used,charged),(n,items));cursor+=used;remain-=charged;}assert_eq!((cursor,remain),(2*n,0));
 if items>0{let(_,used,charged)=prefix(&joined,Limits{max_bytes:n,max_items:items*2-1}).unwrap();assert_eq!((used,charged),(n,items));assert!(prefix(&joined[n..],Limits{max_bytes:n,max_items:items-1}).is_err());}
 let mut t=Vec::<i64>::new();PROJECTION
 std::fs::write(&a[2],out).unwrap();println!("{} {}",n,items);println!("{:?}",t);
}
'''
CONTROL_PROJ=r'''
t.extend([2,v.player_id as i64,v.priority as i64,65537,16,v.controls.capacity as i64,v.controls.instances.len() as i64]);
for x in &v.controls.instances{t.extend([x.index as i64,2,x.id as i64,x.priority as i64,x.recruit_priority as i64]);}
t.extend([v.controls.current_id_bits as i64,v.controls.free_indices.len() as i64,v.controls.first_free as i64,v.controls.last_free as i64]);t.extend(v.controls.free_indices.iter().map(|x|*x as i64));
t.extend([i64::from(v.has_out_of_turn_processing),i64::from(v.running_as_human),v.contracts.len() as i64]);
for c in &v.contracts{match c{None=>t.push(-1),Some(c)=>{
 use c6_core::player_ai_control::ContractData as D;
 let key=match c.data{D::UnitRequest{..}=>0,D::TechResearch{..}=>1,D::CivicResearch{..}=>2,D::PurchasePlot{..}=>3,D::DiplomaticAction{..}=>4};
 let b=&c.base;t.extend([key,3,b.status as i64,b.player_id as i64,b.priority as i64,b.time_limit as i64,b.time_estimate as i64,b.id as i64,b.owner as i64,b.extra_present_value as i64,if key==1||key==4{2}else{1}]);
 match c.data{D::UnitRequest{unit_hash,building_hash,district_hash,location,required_city,keep_control}=>t.extend([unit_hash as i64,building_hash as i64,district_hash as i64,location[0]as i64,location[1]as i64,required_city as i64,i64::from(keep_control)]),D::TechResearch{technology_hashes,best_controller}=>t.extend([technology_hashes[0]as i64,technology_hashes[1]as i64,best_controller as i64]),D::CivicResearch{civic_hashes}=>t.extend([civic_hashes[0]as i64,civic_hashes[1]as i64]),D::PurchasePlot{location}=>t.extend([location[0]as i64,location[1]as i64]),D::DiplomaticAction{player_id,action_hash}=>t.extend([player_id as i64,action_hash as i64])}
}}}
'''
MAP_PROJ='t.extend([2,v.influence.len()as i64]);t.extend(v.influence.iter().map(|x|*x as i64));t.push(v.religious_influence.len()as i64);t.extend(v.religious_influence.iter().map(|x|*x as i64));'
TAIL_PROJ=r'''
macro_rules! list{($v:expr)=>{{t.push($v.len()as i64);t.extend($v.iter().map(|x|*x as i64));}};}
if let Some(a)=&v.alive{list!(a.agendas);list!(a.agenda_visibilities);list!(a.foreign_continent_city_unit_class_hashes);list!(a.district_naval_unit_hashes);list!(a.improvement_hashes);
t.push(a.special_allowed_projects.len()as i64);for x in &a.special_allowed_projects{t.extend([x.definition_hash as i64,x.value as i64]);}
t.push(a.chop_behaviors.len()as i64);for x in &a.chop_behaviors{t.extend([x.feature_wire as i64,i64::from(x.ban),x.yield_type as i64,x.amount as i64]);}
t.push(a.additional_pillaging.len()as i64);for x in &a.additional_pillaging{t.extend([x.plunder_type as i64,x.improvement_hash as i64,x.district_hash as i64,x.amount as i64]);}list!(a.co2_generation_reductions);}
let (prop,_)=v.properties.encode(c6_core::unit::variants::Limits::default()).unwrap();t.push(prop.len()as i64);t.extend(prop.iter().map(|x|*x as i64));
t.push(v.improvement_replacements.len()as i64);for x in &v.improvement_replacements{t.extend([x.definition_hash as i64,x.value as i64]);}list!(v.random_event_hashes);list!(v.terrain_work_impassable);list!(v.random_event_no_unit_damage);list!(v.random_event_modified_damage_opposing_player);
for player_version in [0,46,48,u32::MAX]{assert!(TYPE::decode(&b,Context{player_version,..c},loose).is_err());}
assert!(v.encode(Context{was_ever_alive:!c.was_ever_alive,..c},loose).is_err());
'''

def make_harness(name):
 typ={'player_ai_control':'ControlManager','player_ai_influence_map':'InfluenceMap','player_tail':'Tail'}[name];tail=name=='player_tail'
 projection={'player_ai_control':CONTROL_PROJ,'player_ai_influence_map':MAP_PROJ,'player_tail':TAIL_PROJ}[name]
 return HARNESS.replace('EXTRA_IMPORT','use c6_core::player_tail::Context;'if tail else'').replace('CONTEXT','let c=Context{player_version:47,was_ever_alive:a[3]!="false"};'if tail else'').replace('CTX','c,'if tail else'').replace('PROJECTION',projection).replace('MODULE',name).replace('TYPE',typ)

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--output',type=Path,required=True);args=ap.parse_args();assets=ROOT.parent/'c6-assets-linux';private=ROOT.parent/'c6-ghidra';packed=run(['git','archive',SOURCE],cwd=ROOT)
 with tempfile.TemporaryDirectory(prefix='c6-control-map-tail-review-')as temp:
  temp=Path(temp);source=temp/'source';source.mkdir();subprocess.run(['tar','-x','-C',str(source)],input=packed,check=True);sys.path.insert(0,str(source/'tools/linux'))
  from save_lifecycle_audit import Elf,GAME_SHA,HOST_SHA
  from player_districts_audit import FIXTURES
  from player_religion_audit import relocation_symbols
  game=Elf(assets/'libGameCore_XP2.so',GAME_SHA);host=Elf(assets/'Civ6',HOST_SHA);relocs=relocation_symbols(game);symbols=set()
  for row in run(['nm','-S','-C',str(assets/'libGameCore_XP2.so')],text=True).splitlines():
   p=row.split(' ',3)
   if len(p)==4 and p[2]in('T','W'):symbols.add((int(p[0],16),int(p[1],16)))
  contracts={};tests={};candidates={};synthetics={};audits={n:importlib.import_module(n+'_audit')for n in NAMES}
  for name,audit in audits.items():
   complete=0;windows=[]
   for row in audit.METHODS:
    address,size=row[:2];digest=row[-1];assert sha(game.read(address,size))==digest
    if(address,size)in symbols:complete+=1
    else:windows.append([hex(address),size])
   contracts[name]={'pinned_ranges':len(audit.METHODS),'complete_symbol_extents':complete,'additional_pinned_windows':windows,'method_manifest_sha256':sha(json.dumps(audit.METHODS,separators=(',',':')).encode())}
  control=audits['player_ai_control']
  for address,reader,writer,_ in control.VIRTUALS:assert game.relocated_pointer(address+16+0xb8)==reader and game.relocated_pointer(address+16+0xc0)==writer
  assert game.read(0x2dff1f0,4)==struct.pack('<I',65537)
  assert [0x2dfef74+x for x in struct.unpack('<5i',game.read(0x2dfef74,20))]==[0x201ca37,0x201cb19,0x201ca71,0x201caab,0x201cae2]
  cmd=['gdb','-nx','-batch',str(assets/'libGameCore_XP2.so')]
  for n in ['Manager','Instance','Contract','UnitRequestContract','TechResearchContract','CivicResearchContract','PurchasePlotContract','DiplomaticActionContract']:cmd+=['-ex',"ptype /o 'GameCore::AI::Control::"+n+"'"]
  dwarf=run(cmd,text=True);assert sha(dwarf.encode())==CONTROL_DWARF
  contracts['player_ai_control'].update({'concrete_contract_virtual_pairs':5,'factory_keys':[0,1,2,3,4],'dwarf_sha256':CONTROL_DWARF})
  influence=audits['player_ai_influence_map']
  for site,plt,body,kind in influence.BINDINGS:
   code=game.read(site,5);assert code[0]==(0xe8 if kind=='call'else 0xe9)and site+5+struct.unpack_from('<i',code,1)[0]==plt
   stub=game.read(plt,6);assert stub[:2]==b'\xff\x25'and relocs.get(plt+6+struct.unpack_from('<i',stub,2)[0])==body
  for slot,body in [(0x3a1d8a8,0x2143e10),(0x3a1d8c0,0x2143e80)]:assert game.relocated_pointer(slot)==body
  dwarf=run(['gdb','-nx','-batch',str(assets/'libGameCore_XP2.so'),'-ex',"ptype /o 'GameCore::AI::InfluenceMap'"],text=True);assert sha(dwarf.encode())==MAP_DWARF
  contracts['player_ai_influence_map'].update({'bindings':len(influence.BINDINGS),'virtuals':2,'dwarf_sha256':MAP_DWARF})
  tail=audits['player_tail'];dwarf=''.join(run(['gdb','-nx','-batch',str(assets/'libGameCore_XP2.so'),'-ex',"ptype /o '"+n+"'"],text=True)for n in ['GameCore::Player::Instance','GameCore::Player::ChopFeatureBehavior','GameCore::Player::AdditionalPillaging']);assert sha(dwarf.encode())==tail.DWARF_SHA
  for offset,body in [(0x1f8,0x3961e50),(0x1f0,0x3971330)]:assert struct.unpack('<Q',host.read(0x53b3680+16+offset,8))[0]==body
  branch=game.read(0x2b4f1bc,14);j=branch.find(b'\x0f\x84');assert j>=0 and 0x2b4f1bc+j+6+struct.unpack_from('<i',branch,j+2)[0]==0x2b4f413
  contracts['player_tail'].update({'properties_virtuals':2,'dwarf_sha256':tail.DWARF_SHA,'false_branch_target':'0x2b4f413','true_tail_entry':'0x2b4f315'})
  paths=['Cargo.toml','Cargo.lock','crates/c6-core/Cargo.toml']+[str(p.relative_to(source))for p in sorted((source/'crates/c6-core/src').rglob('*.rs'))]+['tools/linux/'+n+'_audit.py'for n in NAMES]
  source_hashes={p:sha((source/p).read_bytes())for p in paths}
  for p,digest in source_hashes.items():assert sha(run(['git','show',SOURCE+':'+p],cwd=ROOT))==digest
  lib=(source/'crates/c6-core/src/lib.rs').read_text();assert all('pub mod '+n+';'in lib for n in NAMES)
  for prefix in NAMES+['unit::variants']:
   result=run(['cargo','test','--offline','-p','c6-core','--lib',prefix+'::tests'],cwd=source,text=True);tests[prefix]=next(x for x in result.splitlines()if x.startswith('test result:'))
  subprocess.run(['cargo','clippy','--offline','-p','c6-core','--all-targets','--','-D','warnings'],cwd=source,check=True);subprocess.run(['cargo','build','--offline','--release','-p','c6-core'],cwd=source,check=True)
  for name in NAMES:
   text=make_harness(name);(temp/(name+'.rs')).write_text(text);subprocess.run(['rustc','--edition=2021','-Dwarnings','-O',str(temp/(name+'.rs')),'--extern','c6_core='+str(source/'target/release/libc6_core.rlib'),'-L','dependency='+str(source/'target/release/deps'),'-o',str(temp/name)],check=True);candidates[name]={'harness_sha256':sha(text.encode()),'candidate_sha256':sha((temp/name).read_bytes())}
  def compare(name,data,alive=True):
   p=Parser(data);p.control()if name=='player_ai_control'else p.influence_map()if name=='player_ai_influence_map'else p.tail(alive);assert p.pos==len(data)
   (temp/'in').write_bytes(data);lines=run([str(temp/name),str(temp/'in'),str(temp/'out'),'true'if alive else'false'],text=True).splitlines();assert [int(x)for x in lines[0].split()]==[p.pos,p.items]
   assert (temp/'out').read_bytes()==data and json.loads(lines[1])==p.tokens;return p
  for name in NAMES:
   for alive in([True,False]if name=='player_tail'else[True]):
    data=specimen(name,alive);p=compare(name,data,alive);negative=0
    mutations=[(x,0xffffffff)for x in p.counts+p.versions]+[(x,k)for x in p.keys for k in [0xfffffffe,5,0x7fffffff]]
    for offset,value in mutations:
     bad=bytearray(data);struct.pack_into('<I',bad,offset,value);(temp/'in').write_bytes(bad);subprocess.run([str(temp/name),str(temp/'in'),str(temp/'out'),'true'if alive else'false','reject'],check=True);negative+=1
    for offset in p.bools:
     bad=bytearray(data);bad[offset]=2;(temp/'in').write_bytes(bad);subprocess.run([str(temp/name),str(temp/'in'),str(temp/'out'),'true'if alive else'false','reject'],check=True);negative+=1
    for offset in p.property_modifiers:
     bad=bytearray(data);struct.pack_into('<H',bad,offset,1);(temp/'in').write_bytes(bad);subprocess.run([str(temp/name),str(temp/'in'),str(temp/'out'),'true'if alive else'false','reject'],check=True);negative+=1
    synthetics[name+('_true'if alive else'_false')]={'bytes':p.pos,'items':p.items,'hostile_mutations_with_huge_limits':negative,'typed_named_char8_properties':name=='player_tail'}
  fixtures=[]
  for identity,trace_name,trace_sha in FIXTURES:
   data=gzip.decompress((private/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes());assert sha(data)==identity
   trace=gzip.decompress((private/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes());assert sha(trace)==trace_sha
   rows=[x for x in map(json.loads,trace.splitlines())if x.get('event')=='owner'and x.get('save')==1];owners={x['call']:x for x in rows};controls=sorted([x for x in rows if x.get('owner')=='GameCore::AI::Control::Manager'],key=lambda x:x['start']);assert len(controls)==43 and not any('InfluenceMap'in x['owner']for x in rows)
   observations={n:[]for n in NAMES};digests={n:[]for n in NAMES};shapes={n:set()for n in NAMES};kinds=Counter();flags=Counter()
   for control in controls:
    parent=owners[control['parent']];assert parent['owner']=='GameCore::Player::Instance';assert struct.unpack_from('<I',data,parent['start'])[0]==47
    header_flags=data[parent['start']+76:parent['start']+94];assert header_flags[1]==1;flags[str(header_flags[1])]+=1
    planning=[x for x in rows if x.get('parent')==parent['call']and x.get('owner')=='GameCore::AI::Planning::Manager'];behavior=[x for x in rows if x.get('parent')==parent['call']and x.get('owner')=='GameCore::AI::BehaviorTree::Manager'];assert len(planning)==len(behavior)==1
    payloads={'player_ai_control':data[control['start']:control['end']],'player_ai_influence_map':data[control['end']:planning[0]['start']],'player_tail':data[behavior[0]['end']:parent['end']]}
    for name,payload in payloads.items():
     p=compare(name,payload);observations[name].append([p.pos,p.items]);digests[name].append(sha(payload));shapes[name].add(tuple(p.shape))
     if name=='player_ai_control':kinds.update(p.kinds)
   fixtures.append({'stream_sha256':identity,'trace_sha256':trace_sha,'records_per_component':43,'saved_header_flags_1':dict(flags),'control_contract_kinds':dict(kinds),'components':{n:{'bytes':sum(x[0]for x in observations[n]),'items':sum(x[1]for x in observations[n]),'byte_item_shapes':sorted({tuple(x)for x in observations[n]}),'dynamic_collection_shapes':sorted(shapes[n]),'ordered_record_digests_sha256':sha(json.dumps(digests[n],separators=(',',':')).encode())}for n in NAMES}})
  for p,digest in source_hashes.items():assert sha((source/p).read_bytes())==digest
  result={'schema':'c6-linux-control-influence-map-tail-independent-review-v1','checks_passed':True,'source_commit':SOURCE,'archive_sha256':sha(packed),'review_script_sha256':sha(Path(__file__).read_bytes()),'source_sha256':source_hashes,'all_archived_sources_match_git_pin':True,'archive_source_mutations':False,'exports_added_after_extraction':False,'candidates':candidates,'static_contracts':contracts,'gamecore_sha256':GAME_SHA,'host_sha256':HOST_SHA,'source_tests':tests,'actual_strict_cargo_clippy':'cargo clippy --offline -p c6-core --all-targets -- -D warnings','synthetics':synthetics,'fixtures':fixtures,'independent_projection':'complete Control/InfluenceMap/Tail typed scalar/hash/raw enum/coordinate/count projections; known empty/named-Char8 properties separately parsed for item credit and compared via typed map encoding','boundary_qualification':{'control':'actual Control.Manager owner cuts','influence_map':'no InfluenceMap owner hook; same-parent Control.end to Planning.start','tail':'BehaviorTree.Manager.end to its enclosing Player.end; true-context actual records, no standalone Tail hook'},'properties_allocation':'Tail-owned vectors preflight and reserve fallibly. TypedVariantMap can allocate infallibly during both scan/measure and collection; byte/item/depth64 bounds remain shared, no complete-allocation-free or total heap-byte promise.','source_only_branches':['Control null contract slots and empty Manager','InfluenceMap nonempty religious vector and empty-map fallback','Tail false-to-common-properties path, nonempty properties/chop/pillaging and unobserved vector forms'],'false_tail_branch_is_source_only':True,'preliminary_control_altered_archive_unaccepted':True,'no_ai_planning_only_exclusions':True,'native_read_side_effects_reproduced':False,'original_execution':False,'private_writes':False,'full_43_players':False,'full_gameplay_state':False,'all_gameplay_rng':False,'actions':False,'turns':False,'raw_save_load_difference_bytes':89198,'unknown_bytes_masked':False,'whole_prefix_acceptance':'separate frozen producer/composition review'}
  args.output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'checks_passed':True,'source':SOURCE,'records_per_component':86,'components':NAMES}))
if __name__=='__main__':main()
