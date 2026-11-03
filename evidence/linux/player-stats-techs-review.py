#!/usr/bin/env python3
"""Independent immutable Stats/TrackedValue/Techs replay; static ELF reads only."""
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

ROOT = Path(__file__).resolve().parents[2]
SOURCE = 'f09cba77c388402e7d6de64f5841ddb336a3d410'
def sha(data):
    return hashlib.sha256(data).hexdigest()
def run(args, **kwargs):
    return subprocess.check_output(args, **kwargs)

HARNESS = r'''use c6_core::{player_stats::Stats, player_techs::Techs, tracked_value::TrackedValue, unit::Limits};
fn main() {
 let a: Vec<_> = std::env::args().collect();
 let b = std::fs::read(&a[2]).unwrap();
 let loose = Limits { max_bytes: 64*1024*1024, max_items: 4_000_000 };
 macro_rules! check { ($ty:ty, $shape:expr) => {{
  let (v, n, items) = <$ty>::decode_prefix_with_items(&b, loose).unwrap();
  assert_eq!(n, b.len());
  let exact = Limits { max_bytes: n, max_items: items };
  assert_eq!(v.encode_with_items(exact).unwrap(), (b.clone(), items));
  assert_eq!(<$ty>::decode(&b, exact).unwrap(), v);
  let mut tail = b.clone(); tail.extend([0xab,0xcd]);
  assert_eq!(<$ty>::decode_prefix_with_items(&tail, exact).unwrap(), (v.clone(), n, items));
  assert!(<$ty>::decode(&tail, loose).is_err());
  for k in 0..n { assert!(<$ty>::decode(&b[..k], loose).is_err()); }
  for low in [Limits { max_bytes:n-1, ..exact }, Limits { max_items:items-1, ..exact }] {
   assert!(<$ty>::decode(&b, low).is_err()); assert!(v.encode(low).is_err());
  }
  let mut bad = b.clone(); bad[..4].copy_from_slice(&0xfffffff0u32.to_le_bytes());
  assert!(<$ty>::decode(&bad, loose).is_err());
  println!("{} {} {}", n, items, ($shape)(&v));
 }} }
 match a[1].as_str() {
  "stats" => check!(Stats, |v: &Stats| format!("{} {} {} {} {}",v.num_units_constructed.len(),v.num_projects_advanced.len(),v.wonders_lost_plots.len(),v.cached_yields_q8.len(),v.diplomatic_victory_points.steps.len())),
  "tracked" => check!(TrackedValue, |v: &TrackedValue| format!("{} {} {} {}",v.steps.len(),u8::from(v.include_steps),u8::from(v.min_q8.is_some()),u8::from(v.max_q8.is_some()))),
  "techs" => check!(Techs, |v: &Techs| format!("{} {} {} {}",v.research_queue_hashes.len(),v.has_tech.pairs.len(),v.boost_triggered.pairs.len(),v.research_progress_q8.len())),
  _ => panic!("unsupported mode")
 }
}
'''

def main():
 ap=argparse.ArgumentParser(description=__doc__)
 ap.add_argument('--output',type=Path,required=True)
 args=ap.parse_args()
 assets=ROOT.parent/'c6-assets-linux'
 private=ROOT.parent/'c6-ghidra'
 archive=run(['git','archive',SOURCE],cwd=ROOT)
 with tempfile.TemporaryDirectory(prefix='c6-stats-techs-review-') as temp:
  temp=Path(temp); source=temp/'source'; source.mkdir()
  subprocess.run(['tar','-x','-C',str(source)],input=archive,check=True)
  sys.path.insert(0,str(source/'tools/linux'))
  audit={name:importlib.import_module(name+'_audit') for name in ['player_stats','tracked_value','player_techs']}
  from save_lifecycle_audit import Elf,GAME_SHA,HOST_SHA
  from player_districts_audit import FIXTURES
  game=Elf(assets/'libGameCore_XP2.so',GAME_SHA); Elf(assets/'Civ6',HOST_SHA)
  symbols={}
  for row in run(['nm','-S','-C',str(assets/'libGameCore_XP2.so')],text=True).splitlines():
   parts=row.split(' ',3)
   if len(parts)==4 and parts[2] in ('T','W'): symbols[parts[3]]=(int(parts[0],16),int(parts[1],16))
  static={}
  for name,mod in audit.items():
   for address,size,symbol,digest in mod.METHODS:
    assert symbols[symbol]==(address,size)
    assert sha(game.read(address,size))==digest
   static[name]={'methods':len(mod.METHODS),'method_manifest_sha256':sha(json.dumps(mod.METHODS,separators=(',',':')).encode())}
  for address,version in [(0x2f678cc,27),(0x2f88e98,1),(0x2f685e4,6)]:
   assert struct.unpack('<I',game.read(address,4))[0]==version
  for address,target in [(0x2b4f0ad,0x1eb9310),(0x2b4f0c0,0x1f4cb40),(0x2b4f0d3,0x1f84390),(0x2b4f0e6,0x1f66d50),(0x2b4f0f9,0x1f35f50)]:game.call(address,target)
  for name in ['player_stats','player_techs']:
   dwarf=run(['gdb','-nx','-batch',str(assets/'libGameCore_XP2.so'),'-ex',"ptype /o 'GameCore::Player::"+audit[name].TYPES[0]+"'"],text=True)
   assert sha(dwarf.encode())==audit[name].DWARF_SHA
   static[name]['dwarf_sha256']=sha(dwarf.encode())
  dwarf=run(['gdb','-nx','-batch',str(assets/'libGameCore_XP2.so'),'-ex',"ptype /o 'TrackedValue'",'-ex',"ptype /o 'std::__1::tuple<TrackedValue, Localization::String, unsigned int, int>'"],text=True)
  assert sha(dwarf.encode())=='0cd7cb063b150dfa46db32413b15480fb17a3a2ceb006fc4cd1eca4dfa2a12dd'
  static['tracked_value']['dwarf_sha256']=sha(dwarf.encode())
  for component in ['player_stats','tracked_value','player_techs']:
   subprocess.run(['cargo','test','--offline','-p','c6-core','--lib',component+'::tests'],cwd=source,check=True)
  subprocess.run(['cargo','build','--offline','-p','c6-core'],cwd=source,check=True)
  (temp/'review.rs').write_text(HARNESS)
  candidate=temp/'review'
  subprocess.run(['rustc','--edition=2021','-Dwarnings',str(temp/'review.rs'),'--extern','c6_core='+str(source/'target/debug/libc6_core.rlib'),'-L','dependency='+str(source/'target/debug/deps'),'-o',str(candidate)],check=True)
  fixtures=[]
  for identity,trace_name,trace_sha in FIXTURES:
   data=gzip.decompress((private/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes());assert sha(data)==identity
   trace=gzip.decompress((private/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes());assert sha(trace)==trace_sha
   rows=[json.loads(x) for x in trace.splitlines()]
   report={'stream_sha256':identity,'trace_sha256':trace_sha,'components':{}}
   for mode,owner in [('stats','Stats'),('tracked','Stats'),('techs','Techs')]:
    cuts=sorted([x for x in rows if x.get('event')=='owner' and x.get('save')==1 and x.get('owner')=='GameCore::Player::'+owner],key=lambda x:x['start'])
    assert len(cuts)==43
    observations=[];digests=[]
    for cut in cuts:
     payload=data[cut['start']:cut['end']]
     if mode=='tracked':
      assert struct.unpack_from('<I',payload)[0]==27
      # Target v27 Stats: version + 22 scalar words precede tracked prefix.
      payload=payload[92:]
      # Existing records have an empty v1 leaf. Assert framing independently.
      assert payload[:4]==struct.pack('<I',1) and payload[8:11]==bytes([1,0,0]) and payload[11:15]==bytes(4)
      payload=payload[:15]
     (temp/'record').write_bytes(payload)
     observation=[int(x) for x in run([str(candidate),mode,str(temp/'record')],text=True).split()]
     assert observation[0]==len(payload)
     observations.append(observation);digests.append(sha(payload))
    report['components'][mode]={'records':len(cuts),'bytes':sum(x[0] for x in observations),'shapes':sorted({tuple(x) for x in observations}),'ordered_record_digests_sha256':sha(json.dumps(digests,separators=(',',':')).encode())}
   fixtures.append(report)
  paths=['crates/c6-core/src/'+name+'.rs' for name in ['player_stats','tracked_value','player_techs','player_prerequisites']]+['crates/c6-core/src/map/localization.rs','crates/c6-core/src/lib.rs']
  result={'schema':'c6-linux-player-stats-techs-independent-review-v1','source_commit':SOURCE,'archive_sha256':sha(archive),'candidate_sha256':sha(candidate.read_bytes()),'harness_sha256':sha(HARNESS.encode()),'review_script_sha256':sha(Path(__file__).read_bytes()),'sources':{path:sha((source/path).read_bytes()) for path in paths},'gamecore_sha256':GAME_SHA,'host_sha256':HOST_SHA,'static_contracts':static,'fixtures':fixtures,'shape_columns':{'stats':['bytes','items','unit_pairs','project_pairs','wonders_lost_plots','yield_pairs','tracked_steps'],'tracked':['bytes','items','steps','include_steps','minimum_present','maximum_present'],'techs':['bytes','items','queue_hashes','acquired_pairs','boost_pairs','progress_pairs']},'source_tests':{'stats':4,'tracked':4,'techs':5},'checks_passed':True,'checked_each_record':['exact decode/reencode bytes and item charge','every truncated prefix rejected','strict trailing bytes rejected','prefix tail uncharged under exact record budget','unsupported version rejected','exact bytes/items accepted and one-short rejected'],'native_record_branches_missing':['nonempty TrackedValue steps','include_steps false','optional tracked bounds','recursive localization','depth boundary'],'original_execution':False,'private_writes':False,'acceptance_scope':'current bounded wire codecs only; expanded byte-zero composition requires separate frozen candidate review','native_read_application':False,'runtime_tech_provider':False,'full_43_players':False,'full_gameplay_state':False,'all_rng':False,'actions':False,'turns':False,'raw_save_load_difference_bytes':89198,'unknown_bytes_masked':False}
  args.output.write_text(json.dumps(result,indent=2)+'\n')
  print(json.dumps({'checks_passed':True,'source':SOURCE,'records_per_component':86,'candidate_sha256':result['candidate_sha256']}))
if __name__=='__main__':main()
