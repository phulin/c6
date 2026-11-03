#!/usr/bin/env python3
"""Independent current wire review; Military fixture acceptance is conditional on explicit shape only."""
import gzip, hashlib, importlib, json, re, struct, subprocess, sys, tempfile
from collections import Counter
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
PIN='3b84280a0b458fc26c2d6f6e3d1a807f7483d733'
NAMES=['military','research','tactical']
TYPES=['Military','Research','Tactical']
CLOSED={'military':'3b84280a0b458fc26c2d6f6e3d1a807f7483d733','military_operations':'3b84280a0b458fc26c2d6f6e3d1a807f7483d733','research':'67e8ca369e5d4907d48ae63165d65168e321096d','tactical':'3b84280a0b458fc26c2d6f6e3d1a807f7483d733','culture':'f746129c92df3102a7d7b9078ffc9cbd88ce8332','homeland':'14ececeb27428448f6ba8ec037a0e41456f6f29a'}
def sha(x):return hashlib.sha256(x).hexdigest()
def run(a,**kw):return subprocess.check_output(a,**kw)
# Reviewed current native contracts, independent of Rust Wire implementations.
SCHEMAS={'military': {'ComponentId': (None, [('owner', 's'), ('component_type', 's'), ('id', 'u')]),
              'HashValue': (None, [('hash', 'u'), ('value', 'i')]),
              'DefenseAssignment': (1, [('unit', 'u'), ('location', 'Coord')]),
              'MilitaryDefense': (1,
                                  [('player_id', 'i'),
                                   ('defense_type', 'i'),
                                   ('subject', 'u'),
                                   ('desired_units', 'i'),
                                   ('current_units', 'i'),
                                   ('assignments', '[DefenseAssignment]')]),
              'OperationEntry': (None, [('index', 'i'), ('operation', 'Operation')]),
              'Military': (14,
                           [('player_id', 'i'),
                            ('regional_combat_strength', 'i'),
                            ('regional_enemy_strength', 'i'),
                            ('regional_other_strength', 'i'),
                            ('on_war_footing', 'b'),
                            ('turns_cities_taken', '[i]'),
                            ('desired_explorers', 'i'),
                            ('desired_naval_explorers', 'i'),
                            ('current_explorers', 'i'),
                            ('current_naval_explorers', 'i'),
                            ('allow_unit_construction', 'b'),
                            ('most_desired_technology_hash', 'u'),
                            ('prepare_for_war_with', '[b]'),
                            ('player_rivals', '[b]'),
                            ('operation_types', 'OperationTypes'),
                            ('active_operations', 'RecycledOperations'),
                            ('regional_military_strength', 'Fixed64'),
                            ('modified_unit_preferences', '[HashValue]'),
                            ('pseudo_yield_preferences', '[HashValue]'),
                            ('civ_defense', '[MilitaryDefense]'),
                            ('levy_military_cost', '[i]'),
                            ('levy_military_value', '[i]')]),
              'OperationTypes': (None,
                                 [('allowed', '[HashValue]'),
                                  ('per_war', '[HashValue]'),
                                  ('current', '[HashValue]')]),
              'TeamRequirement': (2,
                                  [('unit_tag_hash', 'i'),
                                   ('unit_requirement_hash', 'i'),
                                   ('min_units', 'i'),
                                   ('max_units', 'i'),
                                   ('min_percentage_bits', 'u'),
                                   ('max_percentage_bits', 'u'),
                                   ('reconsider_until_ready', 'b')]),
              'TeamDefinition': (2,
                                 [('min_units', 'i'),
                                  ('max_units', 'i'),
                                  ('strength_advantage_bits', 'u'),
                                  ('ongoing_strength_advantage_bits', 'u'),
                                  ('safe_rally_point', 'b'),
                                  ('condition_hash', 'u'),
                                  ('requirements', '[TeamRequirement]')]),
              'OperationDef': (7,
                               [('op_name', 'String'),
                                ('name_hash', 'i'),
                                ('target_type', 'i'),
                                ('target_parameter', 'i'),
                                ('update_team_points', 'b'),
                                ('op_type', 'i'),
                                ('behavior_tree_name', 'String'),
                                ('priority', 'i'),
                                ('teams', '[TeamDefinition]'),
                                ('max_dist_in_region', 'i'),
                                ('max_dist_in_area', 'i'),
                                ('max_dist_in_world', 'i'),
                                ('max_target_strength', 'i'),
                                ('max_target_defense', 'i'),
                                ('min_odds_of_success_bits', 'u'),
                                ('self_start', 'b'),
                                ('must_be_at_war', 'b'),
                                ('must_have_nukes', 'b'),
                                ('must_have_min_units', 'i'),
                                ('enemy_type', 'String'),
                                ('target_lua_script', 'String'),
                                ('active_emergency', 'String')]),
              'OperationTarget': (1,
                                  [('enemy', 'i'),
                                   ('target_type', 'i'),
                                   ('location', 'Coord'),
                                   ('rally', 'Coord'),
                                   ('target_id', 'ComponentId')]),
              'OperationTeam': (3,
                                [('is_at_minimums', 'b'),
                                 ('is_ready', 'b'),
                                 ('is_full_strength', 'b'),
                                 ('needed_unit_hash', 'u'),
                                 ('team_members', '[u]'),
                                 ('new_team_members', '[u]'),
                                 ('member_storage', '[[u]]'),
                                 ('member_storage_location', '[Coord]'),
                                 ('endangered_members', '[u]'),
                                 ('has_enemies_near', 'b'),
                                 ('op_positions', 'Fixed6'),
                                 ('local_enemy_strength', 'i'),
                                 ('local_team_strength', 'i')]),
              'Operation': (6,
                            [('player_id', 'i'),
                             ('op_definition', 'OperationDef'),
                             ('teams', '[OperationTeam]'),
                             ('target', 'OperationTarget'),
                             ('rally_point', 'Coord'),
                             ('blocked_turns', 'i'),
                             ('tree_control_id', 'u'),
                             ('tree_manager_id', 'u'),
                             ('backup_target', 'OperationTarget'),
                             ('force_target_switch', 'b'),
                             ('op_strength', 'i'),
                             ('target_strength', 'i'),
                             ('current_combat_strength', 'i'),
                             ('force_ready', 'b'),
                             ('war_ready', 'b'),
                             ('is_emergency', 'b'),
                             ('control_id', 'u')])},
 'research': {'ControlHandle': (1, [('player_id', 'i'), ('id', 'u')]),
              'ActiveBoost': (None, [('behavior_tree_control_id', 'u'), ('boost_uniqueness_hash', 'i')]),
              'BoostManager': (1,
                               [('player_id', 'i'),
                                ('allowed_active_boosts', 'u'),
                                ('active_boosts', '[ActiveBoost]')]),
              'TechTreeAnalysis': (3,
                                   [('tech_hash', 'u'),
                                    ('next_tech_hash', 'u'),
                                    ('player_id', 'i'),
                                    ('turns_to_research', 'i'),
                                    ('value_q8', 'i'),
                                    ('full_value_q8', 'i'),
                                    ('has_tech', 'b'),
                                    ('is_repeatable', 'b'),
                                    ('has_boost', 'b'),
                                    ('is_leaf', 'b')]),
              'Research': (4,
                           [('player_id', 'i'),
                            ('research_target_hash', 'u'),
                            ('turns_to_research_unit', '[{i}]'),
                            ('favored_techs', '[{b}]'),
                            ('tree_analysis', '[{TechTreeAnalysis}]'),
                            ('done_off_turn', 'b'),
                            ('managed_contracts', '[ControlHandle]'),
                            ('boost_manager', 'BoostManager'),
                            ('turns_remaining_on_current_research', 'i')])},
 'tactical': {'Move': (2, [('allowed_move_hash', 'u'), ('priority', 'i')]),
              'MoveList': (1, [('base_moves', '[i]'), ('moves', '[Move]')]),
              'Tactical': (2,
                           [('player_id', 'i'),
                            ('control_instance_id', 'u'),
                            ('prioritized_moves', 'MoveList'),
                            ('zombie_type', 'i')])}}

class Projection:
 def __init__(self,data,schema,root,present=True):self.present=present;self.data=data;self.schema=schema;self.root=root;self.pos=0;self.items=0;self.tokens=bytearray();self.cuts=set();self.spans={};self.versions={};self.count_sites={};self.bools=[];self.shapes=Counter();self.branches=Counter();self.path=[]
 def atom(self,t,track=True):
  fmt={'i':'i','u':'I','b':'B','c':'B','s':'h'}[t];start=self.pos;size=struct.calcsize('<'+fmt);v=struct.unpack_from('<'+fmt,self.data,start)[0];self.pos+=size;self.tokens.extend(struct.pack('<q',v))
  if track:self.cuts.update([start,start+1,self.pos])
  if t=='b':self.bools.append((start,v))
  return v
 def read(self,t,track=True):
  if t in ['i','u','b','c','s']:return self.atom(t,track)
  if t.startswith('['):
   self.count_sites.setdefault('/'.join(self.path),self.pos);n=self.atom('u',track);assert n<=len(self.data)-self.pos;self.items+=n;self.shapes['/'.join(self.path)+':'+str(n)]+=1
   result=[self.read(t[1:-1],track and j in [0,n-1])for j in range(n)]
   if track:self.cuts.add(self.pos)
   return result
  if t.startswith('{'):return dict(hash=self.atom('u',track),value=self.read(t[1:-1],track))
  if t=='Coord':return [self.atom('i',track),self.atom('i',track)]
  if t=='String':return self.read('[c]',track)
  if t in ['Fixed6','Fixed64']:
   expected=6 if t=='Fixed6'else 64;self.count_sites.setdefault(t,self.pos);assert self.atom('u',track)==expected;self.items+=expected
   return [self.read('Coord'if t=='Fixed6'else'i',track)for _ in range(expected)]
  if t=='RecycledOperations':
   self.versions.setdefault(t,self.pos);assert self.atom('u',track)==65537;self.count_sites.setdefault('block',self.pos);assert self.atom('u',track)==16
   capacity=self.atom('u',track);self.path.append(t+'.entries');entries=self.read('[OperationEntry]',track);self.path.pop();current=self.atom('u',track);self.count_sites.setdefault('free',self.pos);n=self.atom('u',track);self.items+=n;first=self.atom('u',track);last=self.atom('u',track);free=[self.atom('u',track)for _ in range(n)]
   return dict(capacity=capacity,entries=entries,current_id_bits=current,first_free=first,last_free=last,free_indices=free)
  if t=='OperationTypes'and not self.present:return None
  start=self.pos;v,fields=self.schema[t]
  if v is not None:self.versions.setdefault(t,self.pos);assert self.atom('u',track)==v
  result={}
  for name,kind in fields:
   self.path.append(t+'.'+name)
   if kind.startswith('?'):
    present=result['owner']!=-1 and result['opponent']!=-1;self.branches['handler_payload_'+str(present)]+=1
    result[name]=self.read('['+kind[1:]+']',track)if present else None
   else:result[name]=self.read(kind,track)
   self.path.pop()
  self.spans.setdefault(t,[]).append((start,self.pos));return result
 def parse(self):self.value=self.read(self.root);self.cuts={x for x in self.cuts if x<self.pos};return self

def rust_projection():
 out='#![allow(non_snake_case)]\nuse c6_core::{unit::Limits,player_ai_military as military,player_ai_research as research,player_ai_tactical as tactical};\nfn put(o:&mut Vec<u8>,v:i64){o.extend_from_slice(&v.to_le_bytes());}\n'
 for family,schema in SCHEMAS.items():
  def expr(t,v):
   if t in ['i','u','s','c']:return f'put(o,{v} as i64);'
   if t=='b':return f'put(o,{v}.0 as i64);'
   if t=='Coord':return f'for x in &{v}{{put(o,*x as i64);}}'
   if t=='String':return f'put(o,{v}.0.len() as i64);for x in &{v}.0{{put(o,*x as i64);}}'
   if t in ['Fixed6','Fixed64']:
    return f'put(o,{6 if t=="Fixed6"else 64});for x in &{v}.0{{'+(f'for y in x{{put(o,*y as i64);}}'if t=='Fixed6'else'put(o,*x as i64);')+'}'
   if t.startswith('['):return f'put(o,{v}.len() as i64);for x in &{v}{{'+expr(t[1:-1],'(*x)')+'}'
   if t.startswith('{'):return f'put(o,{v}.hash as i64);'+expr(t[1:-1],v+'.value')
   if t=='OperationTypes':return f'if let Some(x)=&{v}{{p_military_OperationTypes(x,o);}}'
   return f'p_{family}_{t}(&{v},o);'
  for name,(version,fields)in schema.items():
   path=f'{family}::{name}'
   if family=='research'and name in ['ActiveBoost','BoostManager']:path='c6_core::player_ai_culture::'+name
   if family=='tactical'and name in ['Move','MoveList']:path='c6_core::player_ai_homeland::'+name
   out+=f'fn p_{family}_{name}(v:&{path},o:&mut Vec<u8>){{'+(f'put(o,{version});'if version is not None else'')+''.join(expr(t,'v.'+f)for f,t in fields)+'}\n'
 out+='fn p_military_RecycledOperations(v:&military::RecycledOperations,o:&mut Vec<u8>){put(o,65537);put(o,16);put(o,v.capacity as i64);put(o,v.entries.len() as i64);for x in &v.entries{p_military_OperationEntry(x,o);}put(o,v.current_id_bits as i64);put(o,v.free_indices.len() as i64);put(o,v.first_free as i64);put(o,v.last_free as i64);for x in &v.free_indices{put(o,*x as i64);}}\n'
 for i,(family,t)in enumerate(zip(NAMES,TYPES)):
  out+=f'fn d{i}(b:&[u8],l:Limits,c:bool)->Result<{family}::{t},{family}::Error>{{'+(f'{family}::{t}::decode_in_context(b,l,military::Context{{operation_type_arrays_present:c}})'if i==0 else f'let _=c;{family}::{t}::decode(b,l)')+'}\n'
  out+=f'fn p{i}(b:&[u8],l:Limits,c:bool)->Result<({family}::{t},usize,usize),{family}::Error>{{'+(f'{family}::{t}::decode_prefix_with_items_in_context(b,l,military::Context{{operation_type_arrays_present:c}})'if i==0 else f'let _=c;{family}::{t}::decode_prefix_with_items(b,l)')+'}\n'
 out+=r'''
fn main(){let a:Vec<String>=std::env::args().collect();let input=std::fs::read(&a[1]).unwrap();let mut at=0;let mut output=Vec::new();let mut records=0;
while at<input.len(){let mut word=||{let x=u32::from_le_bytes(input[at..at+4].try_into().unwrap())as usize;at+=4;x};let kind=word();let present=word()!=0;let size=word();let items=word();let nc=word();let cuts=(0..nc).map(|_|word()).collect::<Vec<_>>();let nm=word();let mutations=(0..nm).map(|_|(word(),word())).collect::<Vec<_>>();let raw=&input[at..at+size];at+=size;let lim=Limits{max_bytes:size,max_items:items};
macro_rules! check{($type:ty,$error:expr,$decode:expr,$prefix:expr,$projection:expr)=>{{let v=$decode(raw,lim,present).unwrap();assert_eq!(v.encode_with_items(lim).unwrap(),(raw.to_vec(),items));$projection(&v,&mut output);
for end in cuts{assert!($decode(&raw[..end],lim,present).is_err());}
for short in [Limits{max_bytes:size-1,..lim},Limits{max_items:items.saturating_sub(1),..lim}]{if short.max_bytes==size&&short.max_items==items{continue;}assert_eq!($decode(raw,short,present),Err($error));assert_eq!(v.encode(short),Err($error));}
let mut tail=raw.to_vec();tail.resize(size+1048576,197);assert!($decode(&tail,lim,present).is_err());assert_eq!($prefix(&tail,lim,present).unwrap(),(v.clone(),size,items));
let joined=raw.repeat(2);let mut cursor=0;let mut remaining=items*2;for _ in 0..2{let(value,n,charged)=$prefix(&joined[cursor..],Limits{max_bytes:size,max_items:remaining},present).unwrap();assert_eq!(value,v);assert_eq!((n,charged),(size,items));cursor+=n;remaining-=charged;}assert_eq!((cursor,remaining),(size*2,0));
for(offset,value)in mutations{let mut b=raw.to_vec();b[offset..offset+4].copy_from_slice(&(value as u32).to_le_bytes());assert!($decode(&b,Limits{max_bytes:size,max_items:usize::MAX},present).is_err());}
}};}
match kind{0=>{assert_eq!(military::Military::decode(raw,lim),Err(military::Error::MissingContext));check!(military::Military,military::Error::ResourceLimit,d0,p0,p_military_Military);},1=>check!(research::Research,research::Error::ResourceLimit,d1,p1,p_research_Research),2=>check!(tactical::Tactical,tactical::Error::ResourceLimit,d2,p2,p_tactical_Tactical),_=>panic!()};records+=1;}
std::fs::write(&a[2],output).unwrap();println!("{records}");}
'''
 return out

def main():
 with tempfile.TemporaryDirectory(prefix='c6-military-research-review-')as directory:
  tmp=Path(directory);src=tmp/'source';src.mkdir();archive=run(['git','archive',PIN],cwd=ROOT);subprocess.run(['tar','-x','-C',str(src)],input=archive,check=True);files={str(p.relative_to(src)):sha(p.read_bytes())for p in src.rglob('*')if p.is_file()}
  for family,closed in CLOSED.items():
   path='crates/c6-core/src/player_ai_'+family+'.rs';assert sha(run(['git','show',closed+':'+path],cwd=ROOT))==files[path]
  sys.path.insert(0,str(src/'tools/linux'));modules={n:importlib.import_module('player_ai_'+n+'_audit')for n in NAMES};base=modules['research'];binary=ROOT.parent/'c6-assets-linux/libGameCore_XP2.so';elf=base.Elf(binary,base.GAME_SHA);relocs=base.relocation_symbols(elf);ops=json.loads((src/'evidence/linux/military-operations-contract.json').read_text())
  for name,fields in ops['schemas'].items():assert json.dumps(fields)==json.dumps(SCHEMAS['military'][name])
  nm=run(['nm','-C','-S','--defined-only',str(binary)],text=True);symbols={};vtables=[]
  for line in nm.splitlines():
   words=line.split()
   if len(words)<4:continue
   address,size=int(words[0],16),int(words[1],16);label=' '.join(words[3:]);symbols.setdefault(address,[]).append((size,label))
   if 'vtable for FAutoVariable<'in label and any(label.endswith('GameCore::AI::'+t+'>')for t in TYPES):vtables.append((address,label))
  static={};methods={};virtuals=[]
  groups=[(n,m.METHODS,m.BINDINGS,getattr(m,'VIRTUALS',[]),m.DWARF_TYPES,m.DWARF_SHA)for n,m in modules.items()]+[('operations',ops['methods'],ops['bindings'],[],ops['dwarf_types'],ops['dwarf_sha256'])]
  for name,contract,bindings,slots,types,dwarf_hash in groups:
   for address,size,label,digest in contract:assert any(n==size for n,_ in symbols[address])and sha(elf.read(address,size))==digest;methods[address]=(size,label,digest)
   for slot,body in slots:assert elf.relocated_pointer(slot)==body
   for binding in bindings:
    if name=='military':slot,body,_,_=binding;assert elf.relocated_pointer(slot)==body;continue
    site,target,body,kind=binding;raw=elf.read(site,5);actual=site+2+struct.unpack_from('<b',raw,1)[0]if raw[0]==0xeb else site+5+struct.unpack_from('<i',raw,1)[0];assert raw[0]in([0xe8]if kind=='call'else[0xe9,0xeb])and actual==target;stub=elf.read(target,6);resolved=relocs.get(target+6+struct.unpack_from('<i',stub,2)[0],0)if stub[:2]==b'\xff\x25'else target;assert resolved==body
   dwarf=run(['gdb','-nx','-batch',str(binary)]+sum((['-ex',"ptype /o '"+t+"'"]for t in types),[]));assert sha(dwarf)==dwarf_hash;static[name]=dict(whole_methods=len(contract),manifest_sha256=sha(json.dumps(contract).encode()),bindings=len(bindings),virtuals=len(slots),dwarf_sha256=sha(dwarf))
  assert elf.relocated_pointer(60940128)==34946080
  for address,label in vtables:
   for offset in [0x20,0x38]:
    body=elf.relocated_pointer(address+offset);size,name=symbols[body][0];assert size>0;digest=sha(elf.read(body,size));virtuals.append([address+offset,body,size,name,digest]);methods.setdefault(body,(size,name,digest))
  ranges=[]
  for address,(size,_,_)in sorted(methods.items()):
   if ranges and address<=ranges[-1][1]+8192:ranges[-1][1]=max(ranges[-1][1],address+size)
   else:ranges.append([address,address+size])
  edges=[]
  for lo,hi in ranges:
   asm=run(['objdump','-d','-C',f'--start-address={lo}',f'--stop-address={hi}',str(binary)],text=True)
   for line in asm.splitlines():
    m=re.search(r'^\s*([0-9a-f]+):.*?\b(call|jmp)\s+([0-9a-f]+) <',line)
    if not m:continue
    site,target=int(m[1],16),int(m[3],16)
    if not any(a<=site<a+s for a,(s,_,_)in methods.items()):continue
    stub=elf.read(target,6);body=relocs.get(target+6+struct.unpack_from('<i',stub,2)[0])if stub[:2]==b'\xff\x25'else target;edges.append([site,target,body])
  corpus=bytearray();expected=bytearray();reports=[];records=0;cut_total=0;hostile_total=0
  def add(kind,raw,p,cuts=None):
   nonlocal records,cut_total,hostile_total
   assert p.pos==len(raw);cuts=sorted(p.cuts)if cuts is None else cuts;mutations=[(off,0x7fffffff)for off in p.versions.values()]+[(off,0xffffffff)for off in p.count_sites.values()];corpus.extend(struct.pack('<IIIII',kind,int(p.present),len(raw),p.items,len(cuts))+struct.pack('<'+'I'*len(cuts),*cuts)+struct.pack('<I',len(mutations)))
   for off,value in mutations:corpus.extend(struct.pack('<II',off,value))
   corpus.extend(raw);expected.extend(p.tokens);records+=1;cut_total+=len(cuts);hostile_total+=len(mutations)
   return dict(bytes=len(raw),items=p.items,sha256=sha(raw),projection_sha256=sha(p.tokens),projection_values=len(p.tokens)//8,cuts=len(cuts),cuts_sha256=sha(struct.pack('<'+'I'*len(cuts),*cuts)),hostile_mutations=len(mutations),typed_counts={k:len(v)for k,v in p.spans.items()},raw_bool_histogram=dict(Counter(v for _,v in p.bools)))
  for identity,trace_name,trace_sha in base.FIXTURES:
   data=gzip.decompress((ROOT.parent/'c6-ghidra/evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes());trace=gzip.decompress((ROOT.parent/'c6-ghidra/evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes());assert sha(data)==identity and sha(trace)==trace_sha;owners={x['call']:x for x in map(json.loads,trace.splitlines())if x.get('event')=='owner'and x.get('save')==1};rows=[]
   for military in sorted((x for x in owners.values()if x['owner']=='GameCore::AI::Military'),key=lambda x:x['start']):
    parent=owners[military['parent']];assert parent['owner']=='GameCore::Player::Instance';version,player=struct.unpack_from('<Ii',data,parent['start']);assert version==47 and data[parent['start']+77]==1
    target=[x for x in owners.values()if x['parent']==military['parent']and x['owner']=='GameCore::AI::Targets'];assert len(target)==1;target=target[0];intervals=[(military['start'],military['end'])];cursor=military['end']
    for kind in [1,2]:
     p=Projection(data[cursor:target['start']],SCHEMAS[NAMES[kind]],TYPES[kind]).parse();intervals.append((cursor,cursor+p.pos));cursor+=p.pos
    assert cursor==target['start']and struct.unpack_from('<Ii',data,cursor)==(4,player)
    for kind,(start,end)in enumerate(intervals):
     raw=data[start:end];p=Projection(raw,SCHEMAS[NAMES[kind]],TYPES[kind],True).parse();assert p.value['player_id']==player;row=add(kind,raw,p);matched={}
     for t,spans in p.spans.items():
      label=('GameCore::AI::Operation::'if t in ['Operation','OperationDef','OperationTeam']else'GameCore::AI::')+t;hooks=sorted((x['start']-start,x['end']-start)for x in owners.values()if x['owner']==label and start<=x['start']<end)
      if hooks:assert hooks==sorted(spans);matched[t]=len(hooks)
     if kind==0:
      for t in ['Operation','OperationDef','OperationTeam']:assert matched.get(t,0)==len(p.spans.get(t,[]))
     row.update(kind=TYPES[kind],player=player,start=start,end=end,boundary='direct_owner'if kind==0 else'contextual_source_grammar',nested_native_extents=matched,military_shape='explicit_true_structural_only'if kind==0 else None,initialization_context_proven=False);rows.append(row)
   assert len(rows)==129;reports.append(dict(identity=identity,trace_sha256=trace_sha,military_context_borrowed_from_other_lifecycle=False,records=rows))
  native_records=records;native_cuts=cut_total;native_hostiles=hostile_total;synthetics=[]
  for kind,family in enumerate(NAMES):
   for present,width in ([(True,1),(False,1),(True,0),(False,0)]if kind==0 else[(True,1),(True,0)]):
    b=bytearray();schema=SCHEMAS[family]
    def atom(t,value):b.extend(struct.pack('<'+{'i':'i','u':'I','b':'B','c':'B','s':'h'}[t],value))
    def make(t,field=''):
     if t in ['i','u','b','c','s']:
      value={'u':0xffffffff,'i':-2147483648,'b':197,'c':255,'s':-32768}[t]
      if field.endswith('_bits'):value={'min_percentage_bits':0x7fc01234,'max_percentage_bits':0xff800000,'strength_advantage_bits':0x80000000,'ongoing_strength_advantage_bits':0x3f800001,'min_odds_of_success_bits':0x7fa01234}[field]
      atom(t,value);return
     if t=='Coord':atom('i',-2147483648);atom('i',2147483647);return
     if t=='String':atom('u',2);atom('c',0);atom('c',255);return
     if t in ['Fixed6','Fixed64']:
      n=6 if t=='Fixed6'else 64;atom('u',n)
      for _ in range(n):make('Coord'if t=='Fixed6'else'i')
      return
     if t=='RecycledOperations':
      atom('u',65537);atom('u',16);atom('u',0xffffffff);make('[OperationEntry]');atom('u',0xffffffff);atom('u',2);atom('u',7);atom('u',0xffffffff);atom('u',0xffffffff);atom('u',7);return
     if t=='OperationTypes'and not present:return
     if t.startswith('['):
      atom('u',width)
      for _ in range(width):make(t[1:-1])
     elif t.startswith('{'):atom('u',0xffffffff);make(t[1:-1])
     else:
      version,fields=schema[t]
      if version is not None:atom('u',version)
      for f,ft in fields:make(ft,f)
    make(TYPES[kind]);p=Projection(b,schema,TYPES[kind],present).parse();row=add(kind,b,p,list(range(len(b))));row.update(kind=TYPES[kind],operation_type_arrays_present=present,vector_width=width,exhaustive_small_prefixes=True,valid_native_initialized_context_claim=False);synthetics.append(row)
   if kind in [0,1]:
    offset=p.bools[0][0]
    for byte in range(256):
     raw=bytearray(b);raw[offset]=byte;q=Projection(raw,schema,TYPES[kind],present).parse();add(kind,raw,q,[])
  code=rust_projection();(tmp/'candidate.rs').write_text(code);(tmp/'corpus').write_bytes(corpus);log=tmp/'validation.log'
  with log.open('w')as f:
   for family in NAMES:subprocess.run(['cargo','test','--offline','-p','c6-core','player_ai_'+family],cwd=src,stdout=f,stderr=f,check=True)
   subprocess.run(['cargo','test','--offline','-p','c6-core','player_ai_culture::tests'],cwd=src,stdout=f,stderr=f,check=True);subprocess.run(['cargo','clippy','--offline','-p','c6-core','--all-targets','--','-D','warnings'],cwd=src,stdout=f,stderr=f,check=True);subprocess.run(['cargo','build','--offline','--release','-p','c6-core'],cwd=src,stdout=f,stderr=f,check=True)
   subprocess.run(['rustc','--edition=2021','-Dwarnings','-O',str(tmp/'candidate.rs'),'--extern','c6_core='+str(src/'target/release/libc6_core.rlib'),'-L','dependency='+str(src/'target/release/deps'),'-o',str(tmp/'candidate')],stdout=f,stderr=f,check=True)
  assert run([str(tmp/'candidate'),str(tmp/'corpus'),str(tmp/'projection')],text=True).strip()==str(records);assert(tmp/'projection').read_bytes()==expected
  for path,digest in files.items():assert sha((src/path).read_bytes())==digest,path
  report=dict(schema='c6-linux-player-ai-military-research-focused-review-v1',target_sha256=base.GAME_SHA,source_commit=PIN,archive_sha256=sha(archive),closed_component_sources=CLOSED,all_archived_files_unchanged=True,archive_files=len(files),actual_exports=True,module_sha256={n:files['crates/c6-core/src/player_ai_'+n+'.rs']for n in CLOSED},static=static,archive_virtual_contracts=virtuals,unique_whole_methods=len(methods),direct_edges=len(edges),resolved_body_edges=sum(e[2]is not None for e in edges),unresolved_import_bodies=sum(e[2]is None for e in edges),edges_sha256=sha(json.dumps(edges).encode()),fixtures=reports,synthetics=synthetics,native_records=native_records,native_truncations=native_cuts,native_hostile_mutations=native_hostiles,total_records=records,total_truncations=cut_total,total_hostile_mutations=hostile_total,raw_bool_values_per_carrier=256,exhaustive_native_prefixes=False,typed_projection_sha256=sha(expected),typed_projection_bytes=len(expected),harness_sha256=sha(code.encode()),candidate_sha256=sha((tmp/'candidate').read_bytes()),driver_sha256=sha(Path(__file__).read_bytes()),validation_log_sha256=sha(log.read_bytes()),module_tests=True,strict_all_target_clippy=True,exact_one_short_byte_items=True,shared_items=True,prefix_1MiB_tail=True,military_schema_acceptance='conditional_on_explicit_known_shape_only',native_initialization_context_proven=False,context_borrowed_between_lifecycles=False,boost_may_allocate_bounded_inner_output_during_parent_measure=True,native_execution=False,native_read_effects=False,planning_only_exclusions=[],complete_player=False,full_state=False,gameplay_rng_complete=False,actions=False,turns=False,unmasked_raw_save_load_difference_bytes=89198)
  (ROOT/'evidence/linux/player-ai-military-research-review.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:report[k]for k in ['native_records','native_truncations','native_hostile_mutations','total_records','typed_projection_sha256']}))
if __name__=='__main__':main()
