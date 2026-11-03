#!/usr/bin/env python3
"""Offline independent current-wire review of five exported Linux Player AI components."""
import gzip, hashlib, importlib, json, re, struct, subprocess, sys, tempfile
from collections import Counter
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
PIN='968ab50e37206e6c15d8a06bf20e3e6567b03644'
NAMES=['diplomatic','economic','governors','grand_strategic','homeland']
CLOSED_SOURCES=['f09c9d2cd66d51426265f36bb45537297e851aad','c8c60e47e1b2f3bee4ecec1ecf96e7f87e1ea20a','668d09fe9663517ce9156e46d46688b23c16eb11','668d09fe9663517ce9156e46d46688b23c16eb11','14ececeb27428448f6ba8ec037a0e41456f6f29a']
TYPES=['Diplomatic','Economic','Governors','GrandStrategic','Homeland']
def sha(x):return hashlib.sha256(x).hexdigest()
def run(args,**kw):return subprocess.check_output(args,**kw)
# Frozen reviewed native field contracts are appended below, separate from Rust Wire code.
SCHEMAS={'diplomatic': {'TradeItem': [1,
                              [['game_turn', 'i'],
                               ['other_player', 'i'],
                               ['item_type', 'u'],
                               ['item_sub_type', 'u'],
                               ['item_value', 'u']]],
                'StateVector': [2, [['states', '[DiplomaticState]'], ['is_updated', 'b']]],
                'DiplomaticState': [5,
                                    [['owner', 'i'],
                                     ['opponent', 'i'],
                                     ['state_name', '[c]'],
                                     ['is_updating', 'b'],
                                     ['is_active', 'b'],
                                     ['has_changed_state', 'b'],
                                     ['diplomatic_bonus_q8', 'i'],
                                     ['state_hash', 'u'],
                                     ['allowed_action_hashes', '[u]'],
                                     ['response_action_hashes', '[u]'],
                                     ['desired_action_hashes', '[u]'],
                                     ['transitions', '[DiplomaticTransition]'],
                                     ['triggers', '[TriggeredTransition]']]],
                'DiplomaticTransition': [2,
                                         [['player_diplomacy_player', 'i'],
                                          ['opponent_player', 'i'],
                                          ['current_score_q8', 'i'],
                                          ['transition_index', 'i'],
                                          ['next_state', '[c]'],
                                          ['next_state_index', 'i'],
                                          ['has_required_min', 'b'],
                                          ['required_min', 'i'],
                                          ['has_required_max', 'b'],
                                          ['required_max', 'i'],
                                          ['has_allowed_min', 'b'],
                                          ['allowed_min', 'i'],
                                          ['has_allowed_max', 'b'],
                                          ['allowed_max', 'i'],
                                          ['must_transition', 'b'],
                                          ['transition_score_q8', 'i'],
                                          ['throttle_turns', 'i'],
                                          ['condition_hash', 'u'],
                                          ['transition_hash', 'u'],
                                          ['transition_action_hashes', '[u]'],
                                          ['actions_gained_hashes', '[u]'],
                                          ['actions_lost_hashes', '[u]']]],
                'TriggeredTransition': [1,
                                        [['next_state_name', '[c]'], ['next_state_index', 'i'], ['trigger', 'i']]],
                'ActionEvalVector': [2, [['handlers', '[DiplomaticActionHandler]'], ['is_updated', 'b']]],
                'DiplomaticActionHandler': [5,
                                            [['owner', 'i'],
                                             ['opponent', 'i'],
                                             ['desirability', '?{i}'],
                                             ['cost', '?{i}'],
                                             ['alliance_value_q8', 'i'],
                                             ['war_value_q8', 'i'],
                                             ['war_value_penalties_q8', 'i'],
                                             ['is_initialized', 'b']]],
                'ControlHandle': [1, [['player_id', 'i'], ['id', 'u']]],
                'Diplomatic': [18,
                               [['player_id', 'i'],
                                ['min_force_to_notice_q8', 'i'],
                                ['min_troubling_force_q8', 'i'],
                                ['diplomatic_scores_q8', '[i]'],
                                ['base_diplo_scores_q8', '[i]'],
                                ['visible_scores_q8', '[[i]]'],
                                ['diplomatic_states', 'StateVector'],
                                ['diplo_action_evaluator', 'ActionEvalVector'],
                                ['num_majors_met', 'i'],
                                ['player_threats_q8', '[i]'],
                                ['player_major_threats', '[b]'],
                                ['player_trusts_q8', '[i]'],
                                ['managed_contracts', '[ControlHandle]'],
                                ['best_potential_friend', 'i'],
                                ['partner_action_preference', '[[{i}]]'],
                                ['partner_action_worth_q8', '[[{i}]]'],
                                ['closed_partner_action', '[[{b}]]'],
                                ['requested_items', '[TradeItem]'],
                                ['excluded_items', '[TradeItem]'],
                                ['favor_traded_this_turn', 'i'],
                                ['done_off_turn', 'b']]]},
 'economic': {'ControlHandle': (1, [('player_id', 'i'), ('id', 'u')]),
              'TradePlayer': (3,
                              [('player_id', 'i'),
                               ('control_id', 'u'),
                               ('best_tech_driven_road_hash', 'u'),
                               ('turns_since_railroad_tree', 'i')]),
              'Economic': (4,
                           [('player_id', 'i'),
                            ('current_balance_q8', 'i'),
                            ('city_income_q8', 'i'),
                            ('religious_income_q8', 'i'),
                            ('diplomatic_income_q8', 'i'),
                            ('percentage_bonuses_income_q8', 'i'),
                            ('city_expenses_q8', 'i'),
                            ('turns_with_negative_income', 'i'),
                            ('total_saving_weight', 'i'),
                            ('saving_weights', '[i]'),
                            ('savings_q8', '[i]'),
                            ('yield_priority_boost_q8', '[{i}]'),
                            ('managed_contracts', '[ControlHandle]'),
                            ('trade_manager', 'TradePlayer')])},
 'governors': {'CityId': (None, [('owner', 's'), ('component_type', 's'), ('id', 'u')]),
               'GovernorAnalysis': (4,
                                    [('player_id', 'i'),
                                     ('governor_hash', 'u'),
                                     ('city_ids', '[CityId]'),
                                     ('values_q8', '[i]')]),
               'GovernorPromotionAnalysis': (2,
                                             [('player_id', 'i'),
                                              ('promotion_hash', 'u'),
                                              ('for_governor_hashes', '[u]'),
                                              ('value_q8', 'i')]),
               'Governors': (3,
                             [('player_id', 'i'),
                              ('done_off_turn', 'b'),
                              ('governor_analyses', '[{GovernorAnalysis}]'),
                              ('promotion_analyses', '[{GovernorPromotionAnalysis}]')])},
 'grand_strategic': {'CityPlanner': (2,
                                     [('player_id', 'i'),
                                      ('expansion_score', 'i'),
                                      ('turns_since_city_founding', 'i')]),
                     'CitySettleSiteEvaluation': (1, [('x', 'i'), ('y', 'i'), ('score_q8', 'i')]),
                     'PlotEvaluation': (5,
                                        [('x', 'i'),
                                         ('y', 'i'),
                                         ('my_nearest_city', 'i'),
                                         ('nearest_enemy_city', 'i'),
                                         ('danger', 'i'),
                                         ('cultural_pressure', 'i'),
                                         ('is_coastal_raw', 'b'),
                                         ('has_fresh_water_raw', 'b'),
                                         ('is_valid_raw', 'b'),
                                         ('new_resources_in_inner_ring', 'i'),
                                         ('plot_score_q8', 'i'),
                                         ('inner_ring_yield', '[{i}]'),
                                         ('full_city_yield', '[{i}]'),
                                         ('num_terrains', '[{i}]'),
                                         ('num_resources_inner_ring', '[{i}]'),
                                         ('num_resources', '[{i}]'),
                                         ('num_features', '[{i}]')]),
                     'RegionEvaluator': (1,
                                         [('player_id', 'i'),
                                          ('region_id', 'u'),
                                          ('evaluations', '[CitySettleSiteEvaluation]'),
                                          ('intermediate_data', 'PlotEvaluation'),
                                          ('has_positive_eval_raw', 'b')]),
                     'VictoryStrategy': (4,
                                         [('player_id', 'i'),
                                          ('pursuing_strategies', '[{b}]'),
                                          ('forbidden_strategies', '[{b}]'),
                                          ('must_change_state', '[{b}]'),
                                          ('meets_exclusive_condition', '[{b}]'),
                                          ('next_turn_change_allowed', '[{i}]'),
                                          ('yield_priorities_q8', '[{i}]'),
                                          ('pseudo_yield_priorities_saved_as_yield_hash_q8', '[{i}]')]),
                     'UnitEfficiencyInfo': (4,
                                            [('unit_hash', 'u'),
                                             ('max_damage', 'i'),
                                             ('is_obsolete_raw', 'b'),
                                             ('is_ranged_raw', 'b'),
                                             ('is_naval_raw', 'b'),
                                             ('has_zone_of_control_raw', 'b'),
                                             ('civ_unique_raw', 'b'),
                                             ('exclude_from_efficiency_calculation_raw', 'b'),
                                             ('can_build_raw', 'b'),
                                             ('speed', 'i'),
                                             ('range', 'i'),
                                             ('main_combat_strength', 'i'),
                                             ('defense', 'i'),
                                             ('num_in_use', 'i'),
                                             ('efficiency_against_all_enemies_q8', 'i'),
                                             ('efficiency_against_everyone_q8', 'i'),
                                             ('efficiency_against_player_q8', '[i]'),
                                             ('raw_efficiency_q8', 'i'),
                                             ('maintenance_resource_hash', 'u'),
                                             ('num_resource_needed', 'i')]),
                     'UnitProduction': (1,
                                        [('player_id', 'i'),
                                         ('offense_units', '[UnitEfficiencyInfo]'),
                                         ('has_targets_raw', 'b')]),
                     'GrandStrategic': (12,
                                        [('player_id', 'i'),
                                         ('city_planner', 'CityPlanner'),
                                         ('primary_regions', '[u]'),
                                         ('expansion_regions', '[u]'),
                                         ('tertiary_regions', '[u]'),
                                         ('settle_cities', '[RegionEvaluator]'),
                                         ('done_island_check_raw', 'b'),
                                         ('is_on_island_raw', 'b'),
                                         ('commemorate_era_raw', 'b'),
                                         ('victory_manager', 'VictoryStrategy'),
                                         ('pseudo_yield_priority_boost_q8', '[{i}]'),
                                         ('pseudo_yield_adjustments_q8', '[{i}]'),
                                         ('unit_production', 'UnitProduction'),
                                         ('yield_sensitivity_boost_q8', '[{i}]')])},
 'homeland': {'Move': (2, [('allowed_move_hash', 'u'), ('priority', 'i')]),
              'MoveList': (1, [('base_moves', '[i]'), ('moves', '[Move]')]),
              'Homeland': (1,
                           [('player_id', 'i'),
                            ('control_instance_id', 'u'),
                            ('additional_priority', 'i'),
                            ('prioritized_moves', 'MoveList')])}}

class Projection:
 def __init__(self,data,schema,root):self.data=data;self.schema=schema;self.root=root;self.pos=0;self.items=0;self.tokens=bytearray();self.cuts=set();self.spans={};self.versions={};self.count_sites={};self.bools=[];self.shapes=Counter();self.branches=Counter();self.path=[]
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

def rust_projection(schemas):
 out='#![allow(non_snake_case)]\nuse c6_core::unit::Limits;\n'+''.join('use c6_core::player_ai_'+n+' as '+n+';\n'for n in NAMES)+'fn put(o:&mut Vec<u8>,v:i64){o.extend_from_slice(&v.to_le_bytes());}\n'
 for family,schema in schemas.items():
  def expr(t,v):
   if t in ['i','u','s','c']:return f'put(o,{v} as i64);'
   if t=='b':return f'put(o,{v}'+('.0'if family=='diplomatic'else'')+' as i64);'
   if t.startswith('['):return f'put(o,{v}.len() as i64);for x in &{v}{{'+expr(t[1:-1],'(*x)')+'}'
   if t.startswith('{'):return f'put(o,{v}.hash as i64);'+expr(t[1:-1],v+'.value')
   if t.startswith('?'):return f'if let Some(x)=&{v}{{'+expr('['+t[1:]+']','(*x)')+'}'
   return f'p_{family}_{t}(&{v},o);'
  for name,(version,fields)in schema.items():
   out+=f'fn p_{family}_{name}(v:&{family}::{name},o:&mut Vec<u8>){{'+(f'put(o,{version});'if version is not None else'')+''.join(expr(t,'v.'+f)for f,t in fields)
   if family=='diplomatic'and name=='Diplomatic':out+='let mut bad=v.clone();if let Some(h)=bad.diplo_action_evaluator.handlers.first_mut(){if h.desirability.is_some(){h.owner=-1;}else{h.desirability=Some(vec![]);}assert_eq!(bad.encode(Limits{max_bytes:usize::MAX,max_items:usize::MAX}),Err(diplomatic::Error::ConditionalPayload));}'
   out+='}\n'
 out+=r'''
fn main(){let a:Vec<String>=std::env::args().collect();let input=std::fs::read(&a[1]).unwrap();let mut pos=0;let mut output=Vec::new();let mut records=0;
while pos<input.len(){let mut word=||{let x=u32::from_le_bytes(input[pos..pos+4].try_into().unwrap())as usize;pos+=4;x};let kind=word();let size=word();let items=word();let nc=word();let cuts=(0..nc).map(|_|word()).collect::<Vec<_>>();let nm=word();let mutations=(0..nm).map(|_|(word(),word())).collect::<Vec<_>>();let raw=&input[pos..pos+size];pos+=size;let lim=Limits{max_bytes:size,max_items:items};
macro_rules! check{($type:ty,$error:expr,$projection:expr)=>{{let v=<$type>::decode(raw,lim).unwrap();assert_eq!(v.encode_with_items(lim).unwrap(),(raw.to_vec(),items));$projection(&v,&mut output);
for end in cuts{assert!(<$type>::decode(&raw[..end],lim).is_err());}
for short in [Limits{max_bytes:size-1,..lim},Limits{max_items:items.saturating_sub(1),..lim}]{if short.max_bytes==size&&short.max_items==items{continue;}assert_eq!(<$type>::decode(raw,short),Err($error));assert_eq!(v.encode(short),Err($error));}
let mut tail=raw.to_vec();tail.resize(size+1048576,255);assert!(<$type>::decode(&tail,lim).is_err());assert_eq!(<$type>::decode_prefix_with_items(&tail,lim).unwrap(),(v.clone(),size,items));
let mut cursor=0;let mut remaining=items*2;let joined=raw.repeat(2);for _ in 0..2{let(value,n,charged)=<$type>::decode_prefix_with_items(&joined[cursor..],Limits{max_bytes:size,max_items:remaining}).unwrap();assert_eq!(value,v);assert_eq!((n,charged),(size,items));cursor+=n;remaining-=charged;}assert_eq!((cursor,remaining),(size*2,0));
for(offset,value)in mutations{let mut b=raw.to_vec();b[offset..offset+4].copy_from_slice(&(value as u32).to_le_bytes());assert!(<$type>::decode(&b,Limits{max_bytes:size,max_items:usize::MAX}).is_err());}
}};}
match kind {
'''
 for i,(family,t)in enumerate(zip(NAMES,TYPES)):out+=f'{i}=>check!({family}::{t},{family}::Error::ResourceLimit,p_{family}_{t}),\n'
 return out+'_=>panic!(),}\nrecords+=1;}std::fs::write(&a[2],output).unwrap();println!("{records}");}\n'

def main():
 with tempfile.TemporaryDirectory(prefix='c6-ai-middle-review-')as directory:
  tmp=Path(directory);src=tmp/'source';src.mkdir();archive=run(['git','archive',PIN],cwd=ROOT);subprocess.run(['tar','-x','-C',str(src)],input=archive,check=True);files={str(p.relative_to(src)):sha(p.read_bytes())for p in src.rglob('*')if p.is_file()};sys.path.insert(0,str(src/'tools/linux'))
  for family,closed in zip(NAMES,CLOSED_SOURCES):
   path='crates/c6-core/src/player_ai_'+family+'.rs';assert sha(run(['git','show',closed+':'+path],cwd=ROOT))==files[path]
  modules={n:importlib.import_module('player_ai_'+n+'_audit')for n in NAMES};base=modules['diplomatic'];binary=ROOT.parent/'c6-assets-linux/libGameCore_XP2.so';elf=base.Elf(binary,base.GAME_SHA);relocs=base.relocation_symbols(elf)
  nm=run(['nm','-C','-S','--defined-only',str(binary)],text=True);symbols={};vtables=[]
  for line in nm.splitlines():
   words=line.split()
   if len(words)<4:continue
   address,size=int(words[0],16),int(words[1],16);label=' '.join(words[3:]);symbols.setdefault(address,[]).append((size,label))
   if 'vtable for FAutoVariable<'in label and any(label.endswith('GameCore::AI::'+t+'>')for t in TYPES):vtables.append((address,label))
  static={};allmethods={};virtuals=[]
  for n,m in modules.items():
   for address,size,label,digest in m.METHODS:
    assert any(s==size for s,_ in symbols[address]);assert sha(elf.read(address,size))==digest;allmethods[address]=(size,label,digest)
   for slot,body in getattr(m,'VIRTUALS',[]):assert elf.relocated_pointer(slot)==body
   bindings=getattr(m,'BINDINGS',[])
   for site,target,body,kind in bindings:
    raw=elf.read(site,5);op=raw[0];actual=site+2+struct.unpack_from('<b',raw,1)[0]if op==0xeb else site+5+struct.unpack_from('<i',raw,1)[0];assert actual==target and op in ([0xe8]if kind=='call'else[0xe9,0xeb]);stub=elf.read(target,6);resolved=relocs.get(target+6+struct.unpack_from('<i',stub,2)[0],target)if stub[:2]==b'\xff\x25'else target;assert resolved==body
   dwarf=run(['gdb','-nx','-batch',str(binary)]+sum((['-ex',"ptype /o '"+t+"'"]for t in m.DWARF_TYPES),[]));assert sha(dwarf)==m.DWARF_SHA
   static[n]=dict(whole_methods=len(m.METHODS),method_contract_sha256=sha(json.dumps(m.METHODS).encode()),explicit_bindings=len(bindings),explicit_virtual_slots=len(getattr(m,'VIRTUALS',[])),dwarf_sha256=sha(dwarf))
  for address,label in vtables:
   for off in [0x20,0x38]:
    body=elf.relocated_pointer(address+off);size,name=symbols[body][0];digest=sha(elf.read(body,size));virtuals.append([address+off,body,size,name,digest]);allmethods.setdefault(body,(size,name,digest))
  # Decode instructions over compact merged ranges; only sites in complete reviewed methods enter evidence.
  ranges=[]
  for address,(size,_,_)in sorted(allmethods.items()):
   if ranges and address<=ranges[-1][1]+8192:ranges[-1][1]=max(ranges[-1][1],address+size)
   else:ranges.append([address,address+size])
  edges=[]
  for lo,hi in ranges:
   asm=run(['objdump','-d','-C',f'--start-address={lo}',f'--stop-address={hi}',str(binary)],text=True)
   for line in asm.splitlines():
    match=re.search(r'^\s*([0-9a-f]+):.*?\b(call|jmp)\s+([0-9a-f]+) <',line)
    if not match:continue
    site,target=int(match[1],16),int(match[3],16)
    if not any(a<=site<a+s for a,(s,_,_)in allmethods.items()):continue
    stub=elf.read(target,6);body=relocs.get(target+6+struct.unpack_from('<i',stub,2)[0])if stub[:2]==b'\xff\x25'else target;edges.append([site,target,body])
  corpus=bytearray();expected=bytearray();reports=[];cuts_total=0;hostile_total=0;record_count=0
  def add(kind,raw,p,cuts=None):
   nonlocal record_count,cuts_total,hostile_total
   assert p.pos==len(raw);cuts=sorted(p.cuts)if cuts is None else cuts;mutations=[(off,0x7fffffff)for off in p.versions.values()]+[(off,0xffffffff)for off in p.count_sites.values()];corpus.extend(struct.pack('<IIII',kind,len(raw),p.items,len(cuts))+struct.pack('<'+'I'*len(cuts),*cuts)+struct.pack('<I',len(mutations)))
   for off,v in mutations:corpus.extend(struct.pack('<II',off,v))
   corpus.extend(raw);expected.extend(p.tokens);record_count+=1;cuts_total+=len(cuts);hostile_total+=len(mutations)
   return dict(bytes=len(raw),items=p.items,sha256=sha(raw),projection_sha256=sha(p.tokens),projection_values=len(p.tokens)//8,cuts=len(cuts),cuts_sha256=sha(struct.pack('<'+'I'*len(cuts),*cuts)),hostile_mutations=len(mutations),typed_counts={k:len(v)for k,v in p.spans.items()},branches=dict(p.branches),empty_vector_occurrences=sum(v for k,v in p.shapes.items()if k.endswith(':0')),raw_bool_histogram=dict(Counter(v for _,v in p.bools)))
  for identity,trace_name,trace_sha in base.FIXTURES:
   data=gzip.decompress((ROOT.parent/'c6-ghidra/evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes());trace=gzip.decompress((ROOT.parent/'c6-ghidra/evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes());assert sha(data)==identity and sha(trace)==trace_sha;owners={x['call']:x for x in map(json.loads,trace.splitlines())if x.get('event')=='owner'and x.get('save')==1};rows=[]
   for diplomatic in sorted((x for x in owners.values()if x['owner']=='GameCore::AI::Diplomatic'),key=lambda x:x['start']):
    parent=owners[diplomatic['parent']];assert parent['owner']=='GameCore::Player::Instance';version,player=struct.unpack_from('<Ii',data,parent['start']);assert version==47 and data[parent['start']+77]==1
    siblings=[x for x in owners.values()if x['parent']==diplomatic['parent']];governors=[x for x in siblings if x['owner']=='GameCore::AI::Governors'];military=[x for x in siblings if x['owner']=='GameCore::AI::Military'];assert len(governors)==len(military)==1;gov=governors[0];mil=military[0]
    intervals=[(diplomatic['start'],diplomatic['end']),(diplomatic['end'],gov['start']),(gov['start'],gov['end'])];cursor=gov['end']
    for kind in [3,4]:
     p=Projection(data[cursor:mil['start']],SCHEMAS[NAMES[kind]],TYPES[kind]).parse();intervals.append((cursor,cursor+p.pos));cursor+=p.pos
    assert cursor==mil['start'] and struct.unpack_from('<Ii',data,cursor)==(14,player)
    for kind,(start,end)in enumerate(intervals):
     raw=data[start:end];p=Projection(raw,SCHEMAS[NAMES[kind]],TYPES[kind]).parse();assert p.value['player_id']==player;row=add(kind,raw,p);matched={}
     for t,spans in p.spans.items():
      label='GameCore::AI::'+t;hooks=sorted((x['start']-start,x['end']-start)for x in owners.values()if x['owner']==label and start<=x['start']<end)
      if hooks:assert hooks==sorted(spans);matched[t]=len(hooks)
     row.update(kind=TYPES[kind],player=player,start=start,end=end,boundary='direct_owner'if kind in [0,2]else'contextual_source_grammar',nested_native_extents=matched);rows.append(row)
   assert len(rows)==215;reports.append(dict(identity=identity,trace_sha256=trace_sha,records=rows))
  native_record_count=record_count;native_cuts=cuts_total;native_hostiles=hostile_total;synthetics=[]
  # All vectors nonempty; duplicate hash entries and non-UTF8/NUL strings remain carriers.
  for kind,family in enumerate(NAMES):
   scenarios=([(-1,-1,2),(-1,3,2),(3,-1,2),(3,4,2)]if kind==0 else[(3,4,2)])+[(3,4,0)]
   for owner,opponent,width in scenarios:
    b=bytearray();schema=SCHEMAS[family]
    def atom(t,v):b.extend(struct.pack('<'+{'u':'I','i':'i','b':'B','c':'B','s':'h'}[t],v))
    def make(t,field='',record=''):
     if t in ['u','i','b','c','s']:atom(t,0xffffffff if t=='u'else 197 if t in ['b','c']else -32768 if t=='s'else owner if record=='DiplomaticActionHandler'and field=='owner'else opponent if record=='DiplomaticActionHandler'and field=='opponent'else -2147483648);return
     if t.startswith('['):
      atom('u',width)
      for j in range(width):
       if t=='[c]':atom('c',[0,255][j])
       else:make(t[1:-1])
     elif t.startswith('{'):atom('u',0xffffffff);make(t[1:-1])
     else:
      version,fields=schema[t]
      if version is not None:atom('u',version)
      for name,ft in fields:
       if ft.startswith('?'):
        if owner!=-1 and opponent!=-1:make('['+ft[1:]+']',name,t)
       else:make(ft,name,t)
    make(TYPES[kind]);p=Projection(b,schema,TYPES[kind]).parse();entry=add(kind,b,p,list(range(len(b))));entry.update(kind=TYPES[kind],owner=owner,opponent=opponent,vector_width=width,exhaustive_small_prefixes=True);synthetics.append(entry)
   if kind in [0,2,3]:
    offset=p.bools[0][0]
    for byte in range(256):
     altered=bytearray(b);altered[offset]=byte;q=Projection(altered,schema,TYPES[kind]).parse();add(kind,altered,q,[])
  code=rust_projection(SCHEMAS);(tmp/'candidate.rs').write_text(code);(tmp/'corpus').write_bytes(corpus);log=tmp/'validation.log'
  with log.open('w')as f:
   for family in NAMES:subprocess.run(['cargo','test','--offline','-p','c6-core','player_ai_'+family+'::tests'],cwd=src,stdout=f,stderr=f,check=True)
   subprocess.run(['cargo','clippy','--offline','-p','c6-core','--all-targets','--','-D','warnings'],cwd=src,stdout=f,stderr=f,check=True);subprocess.run(['cargo','build','--offline','--release','-p','c6-core'],cwd=src,stdout=f,stderr=f,check=True)
   subprocess.run(['rustc','--edition=2021','-Dwarnings','-O',str(tmp/'candidate.rs'),'--extern','c6_core='+str(src/'target/release/libc6_core.rlib'),'-L','dependency='+str(src/'target/release/deps'),'-o',str(tmp/'candidate')],stdout=f,stderr=f,check=True)
  assert run([str(tmp/'candidate'),str(tmp/'corpus'),str(tmp/'projection')],text=True).strip()==str(record_count);assert(tmp/'projection').read_bytes()==expected
  for path,digest in files.items():assert sha((src/path).read_bytes())==digest,path
  report=dict(schema='c6-linux-player-ai-middle-focused-review-v1',source_commit=PIN,closed_component_sources=dict(zip(NAMES,CLOSED_SOURCES)),archive_sha256=sha(archive),all_archived_regular_files_verified_unchanged=True,archive_file_count=len(files),actual_exports=True,module_sha256={n:files['crates/c6-core/src/player_ai_'+n+'.rs']for n in NAMES},static=static,supplemental_archive_virtual_contracts=virtuals,total_unique_whole_methods=len(allmethods),direct_instruction_edges=len(edges),resolved_body_edges=sum(x[2]is not None for x in edges),external_or_unresolved_body_edges=sum(x[2]is None for x in edges),edges_sha256=sha(json.dumps(edges).encode()),wire_schemas_sha256=sha(json.dumps(SCHEMAS).encode()),fixtures=reports,synthetics=synthetics,native_records=native_record_count,native_truncations=native_cuts,native_hostile_mutations=native_hostiles,all_tested_records=record_count,all_tested_truncations=cuts_total,all_hostile_mutations=hostile_total,raw_bool_values_per_carrier=256,all_native_byte_prefixes_exhausted=False,typed_projection_bytes=len(expected),typed_projection_sha256=sha(expected),candidate_sha256=sha((tmp/'candidate').read_bytes()),harness_sha256=sha(code.encode()),driver_sha256=sha(Path(__file__).read_bytes()),validation_log_sha256=sha(log.read_bytes()),focused_cargo_tests=True,strict_all_target_clippy=True,exact_one_short_byte_item_caps=True,prefix_1MiB_unparsed_tail=True,shared_item_consumption=True,original_execution=False,native_read_effects=False,planning_only_exclusions=[],complete_player=False,full_state=False,gameplay_rng_complete=False,actions=False,turns=False,recorded_raw_save_load_difference_bytes=89198,unknown_bytes_masked=0)
  (ROOT/'evidence/linux/player-ai-middle-review.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:report[k]for k in ['native_records','native_truncations','native_hostile_mutations','all_tested_records','typed_projection_sha256']}))
if __name__=='__main__':main()
