#!/usr/bin/env python3
"""Independent typed/bounded review of an unchanged exported Git archive; no engine execution."""
import gzip, hashlib, importlib, json, re, struct, subprocess, sys, tempfile
from collections import Counter
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
PIN='50b849ac115ee82840b0ff55d25875e4a2755446'
def sha(x):return hashlib.sha256(x).hexdigest()
def run(a,**kw):return subprocess.check_output(a,**kw)

# Reviewed native field contracts, separate from the Rust Wire implementation.
CITY_SCHEMA={'id': (0, ['h', 'h', 'I']),
 'hash': (0, ['I', 'i']),
 'handle': (1, ['i', 'I']),
 'reservation': (1, ['I', 'i', 'I']),
 'connection': (1, ['id', 'B', 'i']),
 'trade': (3, ['id', 'i', 'i', 'v:connection', 'v:id']),
 'efficiency': (4,
                ['I',
                 'i',
                 'B',
                 'B',
                 'B',
                 'B',
                 'B',
                 'B',
                 'B',
                 'i',
                 'i',
                 'i',
                 'i',
                 'i',
                 'i',
                 'i',
                 'v:i',
                 'i',
                 'I',
                 'i']),
 'plot': (1,
          ['i',
           'i',
           'i',
           'B',
           'id',
           'I',
           'I',
           'i',
           'i',
           'n6',
           'B',
           'B',
           'B',
           'B',
           'B',
           'B',
           'B',
           'B',
           'B',
           'B',
           'B',
           'B']),
 'layer': (0, ['v:hash', 'v:hash', 'v:hash']),
 'build': (17,
           ['id',
            'I',
            'i',
            'I',
            'v:hash',
            'v:hash',
            'v:hash',
            'I',
            'i',
            'B',
            'i',
            'i',
            'v:reservation',
            'v:I',
            'handle',
            'v:handle',
            'B',
            'i',
            'i',
            'i',
            'i',
            'v:i',
            'v:i',
            'v:i',
            'v:efficiency',
            'trade',
            'v:i',
            'v:plot',
            'layers',
            'v:hash',
            'v:hash',
            'v:hash',
            'i',
            'i',
            'v:hash'])}
CULTURE_SCHEMA={'ControlHandle': (1, [('player_id', 'i'), ('id', 'u')]),
 'ActiveBoost': (None, [('behavior_tree_control_id', 'u'), ('boost_uniqueness_hash', 'i')]),
 'BoostManager': (1,
                  [('player_id', 'i'), ('allowed_active_boosts', 'u'), ('active_boosts', '[ActiveBoost]')]),
 'GreatWorks': (3,
                [('player_id', 'i'),
                 ('great_work_values_q8', '[i]'),
                 ('working_great_work_values_q8', '[i]')]),
 'CivicAnalysis': (2,
                   [('player_id', 'i'),
                    ('civic_hash', 'u'),
                    ('next_civic_needed_hash', 'u'),
                    ('is_owned', 'b'),
                    ('is_favored', 'b'),
                    ('has_boost', 'b'),
                    ('is_repeatable', 'b'),
                    ('solo_value_q8', 'i'),
                    ('full_value_q8', 'i'),
                    ('turns_to_unlock', 'i')]),
 'PolicyAnalysis': (4,
                    [('player_id', 'i'),
                     ('policy_hash', 'u'),
                     ('slot_type_hash', 'u'),
                     ('dark_age_policy', 'b'),
                     ('golden_age_policy', 'b'),
                     ('min_era_chronology', 'i'),
                     ('max_era_chronology', 'i'),
                     ('value_q8', 'i')]),
 'ResolutionAnalysis': (2,
                        [('player_id', 'i'),
                         ('resolution_hash', 'u'),
                         ('value_q8', 'i'),
                         ('option_one_value_q8', 'i'),
                         ('option_two_value_q8', 'i'),
                         ('best_option', 'i')]),
 'PlotCoord': (None, [('x', 'i'), ('y', 'i')]),
 'RockBandAnalysis': (1,
                      [('player_id', 'i'),
                       ('base_value_q8', 'i'),
                       ('location', 'PlotCoord'),
                       ('district_hash', 'u'),
                       ('improvement_hash', 'u'),
                       ('national_park', 'b')]),
 'CultureAi': (15,
               [('player_id', 'i'),
                ('managed_contracts', '[ControlHandle]'),
                ('boost_manager', 'BoostManager'),
                ('done_off_turn', 'b'),
                ('congress_pseudo_yield_preferences', '[{i}]'),
                ('pseudo_yield_preferences', '[{i}]'),
                ('best_slot_policy_values', '[{i}]'),
                ('civic_analyzers', '[{CivicAnalysis}]'),
                ('policy_analyzers', '[{PolicyAnalysis}]'),
                ('resolution_analyzers', '[{ResolutionAnalysis}]'),
                ('govt_values', '[{i}]'),
                ('rockband_analyzers', '[RockBandAnalysis]'),
                ('great_work_manager', 'GreatWorks'),
                ('national_park_spots', '[i]'),
                ('num_naturalists', 'i'),
                ('must_change_policies', 'b')])}

CULTURE_VIRTUALS=[[60850840,
  33832352,
  15,
  'FAutoVariable<Types::Vector<FixedPointT<8u>, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::AI::CultureAi>::load(Serialization::IStream&)',
  '02b716eab0298e1ed510971fa33099092cf4ec354bbc860c3043f50fb9f68964'],
 [60850864,
  33832464,
  15,
  'FAutoVariable<Types::Vector<FixedPointT<8u>, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::AI::CultureAi>::save(Serialization::OStream&) const',
  '1dbf4ad9cbe289cd74dc502ffb658df75199788fbde0a1e18d19352500f6daee'],
 [60851608,
  33844992,
  15,
  'FAutoVariable<Types::Vector<GameCore::AI::CivicAnalysis, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::AI::CultureAi>::load(Serialization::IStream&)',
  '0a58420061113aa2fe20d42bd5d8554c2b1eddfefda99508d9e728a634f18af0'],
 [60851632,
  33845104,
  15,
  'FAutoVariable<Types::Vector<GameCore::AI::CivicAnalysis, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::AI::CultureAi>::save(Serialization::OStream&) const',
  '1058452243e7ea8bf1c283e223d97ebefcbd77bfb809a28c711417d159bdfdea'],
 [60851800,
  33849760,
  15,
  'FAutoVariable<Types::Vector<GameCore::AI::PolicyAnalysis, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::AI::CultureAi>::load(Serialization::IStream&)',
  'a6335567274cf117f5696ff4ae5f0411624bba2aa4427513c8ef9478f81e111b'],
 [60851824,
  33849872,
  15,
  'FAutoVariable<Types::Vector<GameCore::AI::PolicyAnalysis, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::AI::CultureAi>::save(Serialization::OStream&) const',
  'cee12d36eb20b51e14452fdaa1c484ecfda4cfa601b923b30e9174103d550cf1'],
 [60852184,
  33858944,
  15,
  'FAutoVariable<Types::Vector<GameCore::AI::RockBandAnalysis, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::AI::CultureAi>::load(Serialization::IStream&)',
  'b71386ff195050ea57842a5dde1bdedd10c35c2b67a854ac5c795614f163d8b1'],
 [60852208,
  33859056,
  15,
  'FAutoVariable<Types::Vector<GameCore::AI::RockBandAnalysis, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::AI::CultureAi>::save(Serialization::OStream&) const',
  '6072ecb4a63d2eadfee0ce93bd55f85b94fd22fa071a42f07dce05109caa9cdf'],
 [60851992,
  33854368,
  15,
  'FAutoVariable<Types::Vector<GameCore::AI::ResolutionAnalysis, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::AI::CultureAi>::load(Serialization::IStream&)',
  '391be02798e61745856848609c25eede04409e5bfefbf33600b74fcbca1a836e'],
 [60852016,
  33854480,
  15,
  'FAutoVariable<Types::Vector<GameCore::AI::ResolutionAnalysis, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::AI::CultureAi>::save(Serialization::OStream&) const',
  'e65296283db1a22644446ef6a4f8c2c45b99a435763597979992e7c7a4b9df63'],
 [60850456,
  33826064,
  15,
  'FAutoVariable<Types::Vector<GameCore::AI::Control::Handle, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::AI::CultureAi>::load(Serialization::IStream&)',
  'cc549e836fa1c11da4c945af6ce39568e29ded64aaa0a684eb1b106cb2da8111'],
 [60850480,
  33826176,
  15,
  'FAutoVariable<Types::Vector<GameCore::AI::Control::Handle, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::AI::CultureAi>::save(Serialization::OStream&) const',
  '09be5d1533d2c2971239992c21c627d916e8bfe9ffbf371c51386df15e22ab08'],
 [60850072,
  33820032,
  15,
  'FAutoVariable<Types::Vector<bool, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::AI::CultureAi>::load(Serialization::IStream&)',
  '2c5be2437396ee2dcd0f70b8a7181fd811d96941e39c23b5259b0187150a05b6'],
 [60850096,
  33820144,
  15,
  'FAutoVariable<Types::Vector<bool, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::AI::CultureAi>::save(Serialization::OStream&) const',
  '8855312995afe4ba45628350138aa8ed41a6b105088af1ed2f18ec18c688044f'],
 [60850264,
  33823296,
  15,
  'FAutoVariable<Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::AI::CultureAi>::load(Serialization::IStream&)',
  'e0fae0311341533481dc303dafcc062af81f11874fac5957b26aaa5ebcd109dd'],
 [60850288,
  33823408,
  15,
  'FAutoVariable<Types::Vector<int, Types::LinearAlloc::Default<1u, 0u> >, '
  'GameCore::AI::CultureAi>::save(Serialization::OStream&) const',
  '7394765c578515a70c1fb2894e671b8de70c7b44c8df93dea1203ba25f5888bb'],
 [60851032,
  33835120,
  15,
  'FAutoVariable<eastl::vector<int, eastl::allocator>, '
  'GameCore::AI::CultureAi>::load(Serialization::IStream&)',
  'ebf676e39941859bbe7e3ea6a7c26df06a4e4f34f91e5555cc315674fdadac8b'],
 [60851056,
  33835232,
  15,
  'FAutoVariable<eastl::vector<int, eastl::allocator>, '
  'GameCore::AI::CultureAi>::save(Serialization::OStream&) const',
  '781b5ef3734e590e9ace124729b6174ff2a9fa72e28c2849de6c46524b5e8e28'],
 [60850648,
  33828832,
  15,
  'FAutoVariable<GameCore::AI::BoostManager, GameCore::AI::CultureAi>::load(Serialization::IStream&)',
  'b3719a6d30c2f9c6ffcd5315b032a5c8aec94a66d868b2ea540c2d9cbda90119'],
 [60850672,
  33828944,
  15,
  'FAutoVariable<GameCore::AI::BoostManager, GameCore::AI::CultureAi>::save(Serialization::OStream&) const',
  '1661fe110811fda80cb9e5ee5de9f1af1ed322e0f7b2cd9dcfcc5b7882e4bde6'],
 [60851416,
  33841664,
  15,
  'FAutoVariable<GameCore::AI::CultureAi_GreatWorks, GameCore::AI::CultureAi>::load(Serialization::IStream&)',
  'f575a33a921edd5c66456166449f634aa863d285dae45af1e29ffba65bbecfc4'],
 [60851440,
  33841776,
  15,
  'FAutoVariable<GameCore::AI::CultureAi_GreatWorks, GameCore::AI::CultureAi>::save(Serialization::OStream&) '
  'const',
  'e72001d029aaa062d6d66ca49b5f8de8e751d4f35d7ada94c5c022023dee0889'],
 [60849880,
  33817472,
  15,
  'FAutoVariable<GameCore::AI::Control::ControlPriority, '
  'GameCore::AI::CultureAi>::load(Serialization::IStream&)',
  'c61e0056fa7d74626db90c1283facf483ce9e7f52ea9b6d8e3f323d5f5265d0f'],
 [60849904,
  33817536,
  15,
  'FAutoVariable<GameCore::AI::Control::ControlPriority, '
  'GameCore::AI::CultureAi>::save(Serialization::OStream&) const',
  '2c1cd17530b63ab9d8aebafe723b0ff0d08ded97da110b0553b83c13c1572ddb'],
 [60849688,
  33814928,
  15,
  'FAutoVariable<bool, GameCore::AI::CultureAi>::load(Serialization::IStream&)',
  'aed123c9be470ac88c446264b8a2203499306e5f6a21384e6ef174d435575db5'],
 [60849712,
  33814992,
  15,
  'FAutoVariable<bool, GameCore::AI::CultureAi>::save(Serialization::OStream&) const',
  'd1078e7429bb635eeabdee54afcc64ccc47bab9f7f7f0588c323bbff3b02faf7'],
 [60851224,
  33839104,
  15,
  'FAutoVariable<int, GameCore::AI::CultureAi>::load(Serialization::IStream&)',
  '8d2663e65ba4f62504f02c1c13b017fbecc3b8fa035eb8a20030a3f5705ab5df'],
 [60851248,
  33839168,
  15,
  'FAutoVariable<int, GameCore::AI::CultureAi>::save(Serialization::OStream&) const',
  'abc1879c49fcca74dad3586cce89d4ab157cd23bb1bc1a9ff6ca8c26ec65fa26']]

class CityProjection:
 def __init__(self,raw,schema):self.raw=raw;self.schema=schema;self.pos=0;self.items=0;self.tokens=bytearray();self.cuts=set();self.spans={};self.versions=[];self.counts=[];self.bools=[];self.context=[];self.bool_fields=Counter()
 def atom(self,t,track=True):
  start=self.pos;n=struct.calcsize('<'+t);v=struct.unpack_from('<'+t,self.raw,start)[0];self.pos+=n;self.tokens.extend(struct.pack('<q',v))
  if track:self.cuts.update([start,start+1,self.pos])
  return v
 def version(self,n,track=True):self.versions.append(self.pos);assert self.atom('I',track)==n
 def seq(self,t,n,track=True):
  assert 0<=n<=len(self.raw)-self.pos;self.items+=n
  if track:self.cuts.add(self.pos)
  for j in range(n):self.walk(t,track and j in [0,n-1])
  if track:self.cuts.add(self.pos)
 def walk(self,t,track=True):
  if len(t)==1:
   start=self.pos;v=self.atom(t,track)
   if t=='B':
    self.bools.append((start,v))
    if v not in [0,1]:self.bool_fields['/'.join(self.context)]+=1
  elif t.startswith('v:')or t=='layers' or t=='n6':
   self.counts.append((self.pos,t));n=self.atom('i'if t=='layers'else'I',track)
   if t=='n6':assert n==6
   self.seq('layer'if t=='layers'else'h'if t=='n6'else t[2:],n,track)
  else:
   start=self.pos;version,fields=self.schema[t]
   if version:self.version(version,track)
   for index,f in enumerate(fields):
    self.context.append(f'{t}[{index}]');self.walk(f,track);self.context.pop()
   self.spans.setdefault(t,[]).append((start,self.pos))
 def parse(self):
  self.version(2);self.atom('i');self.version(65537);assert self.atom('I')==16;self.atom('I');self.counts.append((self.pos,'live'));n=self.atom('I');self.items+=n
  for j in range(n):self.atom('i');self.walk('build',j in [0,n-1])
  self.atom('I');self.counts.append((self.pos,'free'));n=self.atom('I');self.atom('I');self.atom('I');self.seq('I',n)
  self.cuts={c for c in self.cuts if c<self.pos};return self

class CultureProjection(CityProjection):
 def parse(self):self.walk('CultureAi');self.cuts={c for c in self.cuts if c<self.pos};return self
 def walk(self,t,track=True):
  if t in ['i','u','b']:
   start=self.pos;v=self.atom({'i':'i','u':'I','b':'B'}[t],track)
   if t=='b':self.bools.append((start,v))
  elif t.startswith('['):
   self.counts.append((self.pos,t));self.seq(t[1:-1],self.atom('I',track),track)
  elif t.startswith('{'):self.atom('I',track);self.walk(t[1:-1],track)
  else:
   start=self.pos;v,fields=self.schema[t]
   if v is not None:self.version(v,track)
   for _,f in fields:self.walk(f,track)
   self.spans.setdefault(t,[]).append((start,self.pos))

BASE=r'''use c6_core::{unit::Limits,player_ai_city_manager as city,player_ai_culture as culture};
fn put(o:&mut Vec<u8>,v:i64){o.extend_from_slice(&v.to_le_bytes());}
trait Project{fn project(&self,o:&mut Vec<u8>);}
macro_rules! scalar{($($t:ty),*)=>{$(impl Project for $t{fn project(&self,o:&mut Vec<u8>){put(o,*self as i64);}})*}}
scalar!(u32,i32,i16);
impl<T:Project> Project for Vec<T>{fn project(&self,o:&mut Vec<u8>){put(o,self.len() as i64);for x in self{x.project(o);}}}
impl<T:Project,const N:usize> Project for [T;N]{fn project(&self,o:&mut Vec<u8>){for x in self{x.project(o);}}}
impl Project for city::BoolByte{fn project(&self,o:&mut Vec<u8>){put(o,self.0 as i64);}}
impl Project for city::Neighbors{fn project(&self,o:&mut Vec<u8>){put(o,6);self.0.project(o);}}
impl Project for city::EvaluationLayers{fn project(&self,o:&mut Vec<u8>){self.0.project(o);}}
impl Project for city::RecycledCityBuilds{fn project(&self,o:&mut Vec<u8>){put(o,65537);put(o,16);self.capacity.project(o);self.entries.project(o);self.current_id_bits.project(o);put(o,self.free_indices.len() as i64);self.first_free.project(o);self.last_free.project(o);for x in &self.free_indices{x.project(o);}}}
'''
def harness(city_source,schemas):
 out=BASE
 # Accessors are generated mechanically; Python's separate native grammar supplies expected values/order.
 for name,version,body in re.findall(r'record!\(\s*(\w+),\s*(\w+),\s*\{(.*?)\}\s*\);',city_source,re.S):
  fields=re.findall(r'(\w+)\s*:',body);v={'CITY_BUILD_VERSION':17,'VERSION':2}.get(version,int(version)if version.isdigit()else None);assert v is not None
  out+=f'impl Project for city::{name}{{fn project(&self,o:&mut Vec<u8>){{'+(f'put(o,{v});'if v else'')+''.join(f'self.{f}.project(o);'for f in fields)+'}}\n'
 def expr(t,v):
  if t in ['i','u']:return f'put(o,{v} as i64);'
  if t=='b':return f'put(o,{v}.0 as i64);'
  if t.startswith('['):return f'put(o,{v}.len() as i64);for x in &{v}{{'+expr(t[1:-1],'(*x)')+'}'
  if t.startswith('{'):return f'put(o,{v}.hash as i64);'+expr(t[1:-1],f'{v}.value')
  return f'p_{t}(&{v},o);'
 for name,(v,fields)in schemas.items():
  out+=f'fn p_{name}(v:&culture::{name},o:&mut Vec<u8>){{'+(f'put(o,{v});'if v is not None else'')+''.join(expr(t,'v.'+f)for f,t in fields)+'}\n'
 out+=r'''
fn main(){let a:Vec<String>=std::env::args().collect();let input=std::fs::read(&a[1]).unwrap();let mut pos=0;let mut result=Vec::new();let mut cases=0;
while pos<input.len(){let mut word=||{let x=u32::from_le_bytes(input[pos..pos+4].try_into().unwrap())as usize;pos+=4;x};let kind=word();let size=word();let items=word();let nc=word();let cuts=(0..nc).map(|_|word()).collect::<Vec<_>>();let nm=word();let mutations=(0..nm).map(|_|(word(),word())).collect::<Vec<_>>();let raw=&input[pos..pos+size];pos+=size;let lim=Limits{max_bytes:size,max_items:items};
macro_rules! check{($type:ty,$error:expr,$project:expr)=>{{let value=<$type>::decode(raw,lim).unwrap();assert_eq!(value.encode_with_items(lim).unwrap(),(raw.to_vec(),items));$project(&value,&mut result);
for end in cuts{assert!(<$type>::decode(&raw[..end],lim).is_err());}
for short in [Limits{max_bytes:size-1,..lim},Limits{max_items:items.saturating_sub(1),..lim}]{if short.max_bytes==size&&short.max_items==items{continue;}assert_eq!(<$type>::decode(raw,short),Err($error));assert_eq!(value.encode(short),Err($error));}
let mut tail=raw.to_vec();tail.resize(size+1048576,197);assert!(<$type>::decode(&tail,lim).is_err());assert_eq!(<$type>::decode_prefix_with_items(&tail,lim).unwrap(),(value,size,items));
for(offset,word)in mutations{let mut b=raw.to_vec();b[offset..offset+4].copy_from_slice(&(word as u32).to_le_bytes());assert!(<$type>::decode(&b,Limits{max_bytes:b.len(),max_items:usize::MAX}).is_err());}
}};}
if kind%2==0{check!(city::CityManager,city::Error::ResourceLimit,|v:&city::CityManager,o:&mut Vec<u8>|v.project(o));}else{check!(culture::CultureAi,culture::Error::ResourceLimit,p_CultureAi);}cases+=1;}
std::fs::write(&a[2],result).unwrap();println!("{cases}");}
'''
 return '#![allow(non_snake_case)]\n'+out

def main():
 with tempfile.TemporaryDirectory(prefix='c6-city-culture-review-')as d:
  tmp=Path(d);src=tmp/'source';src.mkdir();archive=run(['git','archive',PIN],cwd=ROOT);subprocess.run(['tar','-x','-C',str(src)],input=archive,check=True)
  files={str(p.relative_to(src)):sha(p.read_bytes())for p in src.rglob('*')if p.is_file()};sys.path.insert(0,str(src/'tools/linux'))
  city=importlib.import_module('player_ai_city_manager_audit');culture=importlib.import_module('player_ai_culture_audit');elf=city.Elf(ROOT.parent/'c6-assets-linux/libGameCore_XP2.so',city.GAME_SHA);relocs=city.relocation_symbols(elf)
  symbols={}
  for line in run(['nm','-S','--defined-only',str(ROOT.parent/'c6-assets-linux/libGameCore_XP2.so')],text=True).splitlines():
   words=line.split()
   if len(words)>=4:symbols.setdefault(int(words[0],16),set()).add(int(words[1],16))
  static={};edge_total=0
  for name,module in [('city',city),('culture',culture)]:
   edges=[]
   methods=list(module.METHODS)
   if name=='culture':methods += [[body,size,label,digest]for _,body,size,label,digest in CULTURE_VIRTUALS if body not in {x[0]for x in module.METHODS}]
   for address,size,label,digest in methods:
    assert size in symbols[address]
    assert sha(elf.read(address,size))==digest
    asm=run(['objdump','-d','-C',f'--start-address={address}',f'--stop-address={address+size}',str(ROOT.parent/'c6-assets-linux/libGameCore_XP2.so')],text=True)
    for line in asm.splitlines():
     m=re.search(r'^\s*([0-9a-f]+):.*?\b(call|jmp)\s+([0-9a-f]+) <',line)
     if not m:continue
     site=int(m[1],16);target=int(m[3],16)
     if address<=target<address+size:continue
     stub=elf.read(target,6);body=relocs.get(target+6+struct.unpack_from('<i',stub,2)[0])if stub[:2]==b'\xff\x25'else target
     edges.append([site,target,body])
   static[name]=dict(whole_methods=len(methods),method_contract_sha256=sha(json.dumps(module.METHODS).encode()),direct_edges=len(edges),resolved_body_edges=sum(e[2]is not None for e in edges),external_or_unresolved_body_edges=sum(e[2]is None for e in edges),edges_sha256=sha(json.dumps(edges).encode()));edge_total+=len(edges)
  for slot,body,_ in city.VIRTUALS:assert elf.relocated_pointer(slot)==body
  static['city']['virtual_bindings']=len(city.VIRTUALS)
  for slot,body,size,label,digest in CULTURE_VIRTUALS:assert elf.relocated_pointer(slot)==body and sha(elf.read(body,size))==digest
  static['culture']['virtual_bindings']=len(CULTURE_VIRTUALS)
  static['culture']['virtual_contract']=CULTURE_VIRTUALS
  types=['GameCore::AI::CityManager','GameCore::AI::CityBuild','GameCore::AI::CityPlotInfo','GameCore::AI::CityPlotEvaluation','GameCore::AI::UnitEfficiencyInfo','GameCore::AI::Trade::City','GameCore::AI::Trade::CityTradeConnection']
  for name,types_,expected in [('city',types,json.loads(run(['git','show','004afd7:evidence/linux/player-ai-city-manager.json'],cwd=ROOT))['dwarf_sha256']),('culture',culture.DWARF_TYPES,culture.DWARF_SHA)]:
   dwarf=run(['gdb','-nx','-batch',str(ROOT.parent/'c6-assets-linux/libGameCore_XP2.so')]+sum((['-ex',"ptype /o '"+t+"'"]for t in types_),[]));assert sha(dwarf)==expected;static[name]['dwarf_sha256']=sha(dwarf)
  # Prove LEA field address directly at both native serializer call sites. All enclosing methods gated above.
  assert b'\xba\x01\x00\x00\x00' in elf.read(0x1fe7900,14)
  assert b'\xb9\x01\x00\x00\x00' in elf.read(33456544,75)
  bool_proofs=[]
  for offset,w,r in [(0x51,0x1fd2dc9,0x1fd2ef9),(0x6c,0x1fd2d5f,0x1fd2e93)]:
   entries=[]
   for site in [w,r]:
    window=elf.read(site-12,17);assert bytes([0x48,0x8d,0x73,offset])in window or bytes([0x48,0x8d,0x70,offset])in window or bytes([0x48,0x8d,0x75,offset])in window
    assert elf.read(site,1)==b'\xe8';target=site+5+struct.unpack('<i',elf.read(site+1,4))[0];stub=elf.read(target,6);body=relocs[target+6+struct.unpack_from('<i',stub,2)[0]]if stub[:2]==b'\xff\x25'else target
    assert body==(33431440 if site==w else 33431552)
    entries.append(dict(site=hex(site),body=hex(body),window_sha256=sha(window)))
   bool_proofs.append(dict(object_offset=hex(offset),calls=entries,width=1,logical_truth_semantics=False))
  corpus=bytearray();expected=bytearray();fixtures=[];totalcuts=0;totalmutations=0
  for identity,trace_name,trace_sha in city.FIXTURES:
   data=gzip.decompress((ROOT.parent/'c6-ghidra/evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes());trace=gzip.decompress((ROOT.parent/'c6-ghidra/evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes());assert sha(data)==identity and sha(trace)==trace_sha
   owners={x['call']:x for x in map(json.loads,trace.splitlines())if x.get('event')=='owner'and x.get('save')==1};rows=[]
   for owner in sorted(owners.values(),key=lambda x:x['start']):
    if owner['owner']!='GameCore::AI::CultureAi':continue
    parent=owners[owner['parent']];planning=[x for x in owners.values()if x['parent']==owner['parent']and x['owner']=='GameCore::AI::Planning::Manager'];assert len(planning)==1
    start=planning[0]['end'];cp=CityProjection(data[start:owner['start']],CITY_SCHEMA).parse();assert start+cp.pos==owner['start'];up=CultureProjection(data[owner['start']:owner['end']],CULTURE_SCHEMA).parse();assert up.pos==owner['end']-owner['start']
    version,player=struct.unpack_from('<Ii',data,parent['start']);assert version==47
    assert struct.unpack_from('<i',data,start+4)[0]==player and struct.unpack_from('<i',data,owner['start']+4)[0]==player
    for kind,p,beg,end in [(0,cp,start,owner['start']),(1,up,owner['start'],owner['end'])]:
     raw=data[beg:end];nested={}
     pairs=[('efficiency','UnitEfficiencyInfo')]if kind==0 else [(t,t)for t in ['BoostManager','CivicAnalysis','PolicyAnalysis','ResolutionAnalysis','RockBandAnalysis']]
     for key,label in pairs:
      hooks=sorted((x['start']-beg,x['end']-beg)for x in owners.values()if x['owner']=='GameCore::AI::'+label and beg<=x['start']<end);assert hooks==sorted(p.spans.get(key,[]));nested[label]=len(hooks)
     cuts=sorted(p.cuts);mutations=[];version_seen=set()
     for x in p.versions:
      version=struct.unpack_from('<I',raw,x)[0]
      if version not in version_seen:mutations.append((x,0x7fffffff));version_seen.add(version)
     # All record types' first version and each count-kind's first site; avoids quadratic repeated vectors.
     seen=set()
     for off,t in p.counts:
      if t in seen:continue
      seen.add(t);mutations.append((off,0xffffffff))
     corpus.extend(struct.pack('<IIII',kind,len(raw),p.items,len(cuts))+struct.pack('<'+'I'*len(cuts),*cuts)+struct.pack('<I',len(mutations)))
     for off,v in mutations:corpus.extend(struct.pack('<II',off,v))
     corpus.extend(raw);expected.extend(p.tokens);totalcuts+=len(cuts);totalmutations+=len(mutations)
     noncanonical=Counter(v for _,v in p.bools if v not in [0,1]);rows.append(dict(kind=['CityManager','CultureAi'][kind],player=player,start=beg,end=end,bytes=len(raw),items=p.items,sha256=sha(raw),projection_sha256=sha(p.tokens),projection_values=len(p.tokens)//8,cuts=len(cuts),cuts_sha256=sha(struct.pack('<'+'I'*len(cuts),*cuts)),mutations=len(mutations),nested_native_extents=nested,typed_record_counts={k:len(v)for k,v in p.spans.items()},noncanonical_bool_bytes=dict(noncanonical),noncanonical_bool_fields=dict(p.bool_fields)))
   assert len(rows)==86;fixtures.append(dict(identity=identity,trace_sha256=trace_sha,records=rows))
  # Independent nonempty carriers exercise native-empty alternatives, extrema and every raw bool byte.
  synthetic=[]
  for kind,module in [(0,city),(1,culture)]:
   base=bytearray()
   def atom(t,v):base.extend(struct.pack('<'+t,v))
   def make(t):
    if kind==0:
     if len(t)==1:atom(t,197 if t=='B'else -32768 if t=='h'else -2147483648 if t=='i'else 0x80000001)
     elif t.startswith('v:')or t in ['layers','n6']:
      atom('i'if t=='layers'else'I',6 if t=='n6'else 1)
      for _ in range(6 if t=='n6'else 1):make('h'if t=='n6'else'layer'if t=='layers'else t[2:])
     else:
      version,fields=CITY_SCHEMA[t]
      if version:atom('I',version)
      for f in fields:make(f)
    else:
     if t in ['i','u','b']:atom({'i':'i','u':'I','b':'B'}[t],197 if t=='b'else -2147483648 if t=='i'else 0x80000001)
     elif t.startswith('['):atom('I',1);make(t[1:-1])
     elif t.startswith('{'):atom('I',0xffffffff);make(t[1:-1])
     else:
      version,fields=CULTURE_SCHEMA[t]
      if version is not None:atom('I',version)
      for _,f in fields:make(f)
   if kind==0:
    base.extend(struct.pack('<IiIIIIi',2,-1,65537,16,16,1,-7));make('build');base.extend(struct.pack('<IIIIII',0xffffffff,2,0xffffffff,7,7,0xffffffff))
   else:make('CultureAi')
   parse=lambda raw:(CityProjection(raw,CITY_SCHEMA)if kind==0 else CultureProjection(raw,CULTURE_SCHEMA)).parse()
   original=parse(base);assert original.pos==len(base);bool_offset=original.bools[0][0]
   for byte in range(256):
    raw=bytearray(base);raw[bool_offset]=byte;p=parse(raw);cuts=list(range(len(raw)))if byte==197 else[]
    corpus.extend(struct.pack('<IIII',kind+2,len(raw),p.items,len(cuts))+struct.pack('<'+'I'*len(cuts),*cuts)+struct.pack('<I',0)+raw);expected.extend(p.tokens)
   synthetic.append(dict(kind=['CityManager','CultureAi'][kind],bytes=len(base),items=original.items,raw_bool_values=256,exhaustive_small_proper_prefixes=len(base),all_nested_vectors_nonempty=True))
  code=harness((src/'crates/c6-core/src/player_ai_city_manager.rs').read_text(),CULTURE_SCHEMA);(tmp/'harness.rs').write_text(code);(tmp/'corpus').write_bytes(corpus)
  log=tmp/'validation.log'
  with log.open('w')as f:
   for filter_ in ['player_ai_city_manager::tests','player_ai_culture::tests']:subprocess.run(['cargo','test','--offline','-p','c6-core',filter_],cwd=src,stdout=f,stderr=f,check=True)
   subprocess.run(['cargo','clippy','--offline','-p','c6-core','--all-targets','--','-D','warnings'],cwd=src,stdout=f,stderr=f,check=True)
   subprocess.run(['cargo','build','--offline','--release','-p','c6-core'],cwd=src,stdout=f,stderr=f,check=True)
   subprocess.run(['rustc','--edition=2021','-Dwarnings','-O',str(tmp/'harness.rs'),'--extern','c6_core='+str(src/'target/release/libc6_core.rlib'),'-L','dependency='+str(src/'target/release/deps'),'-o',str(tmp/'candidate')],stdout=f,stderr=f,check=True)
  assert run([str(tmp/'candidate'),str(tmp/'corpus'),str(tmp/'projection')],text=True).strip()=='684';actual=(tmp/'projection').read_bytes();assert actual==expected
  for path,digest in files.items():assert sha((src/path).read_bytes())==digest,path
  report=dict(schema='c6-linux-player-ai-city-manager-culture-focused-review-v1',source_commit=PIN,archive_sha256=sha(archive),archive_files=len(files),all_archived_files_verified_unchanged=True,actual_exports=True,modules={n:files['crates/c6-core/src/player_ai_'+n+'.rs']for n in ['city_manager','culture']},static=static,bool_proof=bool_proofs,fixtures=fixtures,complete_typed_projection_bytes=len(expected),typed_projection_sha256=sha(expected),harness_sha256=sha(code.encode()),candidate_sha256=sha((tmp/'candidate').read_bytes()),driver_sha256=sha(Path(__file__).read_bytes()),validation_log_sha256=sha(log.read_bytes()),actual_records=172,synthetic=synthetic,actual_truncation_cases=totalcuts,actual_hostile_mutations=totalmutations,exhaustive_actual_prefixes=False,source_tests=5,exhaustive_small_source_prefixes=True,strict_clippy=True,byte_item_exact_one_short=True,prefix_1MiB_tail=True,original_execution=False,native_read_effects=False,gameplay_truth_semantics=False,planning_only_exclusions=[],complete_player=False,full_state=False,actions=False,turns=False,gameplay_rng_complete=False,recorded_raw_save_load_difference_bytes=89198,unknown_bytes_masked=0)
  (ROOT/'evidence/linux/player-ai-city-manager-culture-review.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:report[k]for k in ['actual_records','actual_truncation_cases','actual_hostile_mutations','typed_projection_sha256']}))
if __name__=='__main__':main()
