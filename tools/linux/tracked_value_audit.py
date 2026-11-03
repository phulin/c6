#!/usr/bin/env python3
"""Read-only pinned Linux contracts and offline TrackedValue prefix comparisons."""
import argparse
import gzip
import json
from pathlib import Path
import struct
import subprocess
import tempfile
from core_stream_index import sha
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
from player_districts_audit import FIXTURES
ROOT=Path(__file__).resolve().parents[2]
METHODS=[(47516672, 1060, 'TrackedValue::load(Serialization::IStream&)', '0cc1c7269bca5e7cf8986fbd2d9821bf6a8d3ac8b434a6ad37c7aa6bc005e452'), (47506368, 789, 'Serialization::operator<<(Serialization::OStream&, Localization::String const&)', 'd6d2f9afa7e0b9f602dd3aba62aa91cb68ffd1b7e3cf023046dc5cbc43722b86'), (47507168, 1043, 'Serialization::operator>>(Serialization::IStream&, Localization::String&)', 'a176bccc9ae5f037cb45a132196f79fe8e391ca694c1c37c70cbc1392683fca0'), (47516256, 412, 'TrackedValue::save(Serialization::OStream&) const', 'ed48f46c7af4611926bbad40e8268b685767123d097bd1d165e6b26d97f6e0ec'), (47506304, 20, 'operator<<(Serialization::OStream&, TrackedValue const&)', 'beab724856cd02c4f8e51c14062e44344bfb750ba4c6303d536d11d2aeb54bbd'), (47506336, 20, 'operator>>(Serialization::IStream&, TrackedValue&)', '23037a467400e88d4fce3b5bc3527f61550670209a0a0016e1390837ad19ea45'), (47519088, 34, 'Serialization::IStream& Serialization::ReadVersioned<TrackedValue, TrackedValue>(Serialization::IStream&, TrackedValue&, int, int&&, TrackedValue&&)', '636c642133d1ae0577e93589746182a50ffd7730a954d4896d4bde3ff7b2276a'), (47519040, 34, 'Serialization::IStream& Serialization::ReadVersioned<Localization::String, Localization::String>(Serialization::IStream&, Localization::String&, int, int&&, Localization::String&&)', 'af8621e70499dfc7af9ea87333413fc77020e42a3a9aa92123709bcb0dfa8ba3'), (33447552, 28, 'Serialization::IStream& Serialization::ReadVersioned<bool, bool>(Serialization::IStream&, bool&, int, int&&, bool&&)', '57e8e97c7be2741a1dc218f74049b9cf7a99191cffd41785ce1aa9dd4c6518e5'), (33563776, 28, 'Serialization::IStream& Serialization::ReadVersioned<int, int>(Serialization::IStream&, int&, int, int&&, int&&)', '23f9d1493d61ddb5d8df9b32f1d08543a6eabe7d8b75c1ff9463c7772bc52442'), (40820336, 28, 'Serialization::IStream& Serialization::ReadVersioned<unsigned int, int>(Serialization::IStream&, unsigned int&, int, int&&, int&&)', 'b7174d80583402d829fcee8f5c294367ef494d6f5d149395862e1d96f6f8cd72'), (33431440, 14, 'Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)', '441a140d71c552a6ced859e0e9efa0994c519cad451b975cbb5fc13e8025cd3f'), (33341744, 14, 'Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)', '2a3fc110a86f71122b45f03f192ddbc4499241d298e49b843ea750bd49d5e1cf'), (33341776, 14, 'Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)', '9731ca1d2959dc0f406a684267a29d425c9bf7ea2f127425123508fbc2ed2d3b'), (46772192, 42, 'Serialization::operator<<(Serialization::OStream&, FixedPointT<8u> const&)', 'bbd0553ff77e4be5a896850082327f68662e870164f4771d58a0e5c73c7d5704')]
HARNESS=r'''pub use c6_core::{map,unit};
#[path="MODULE"] pub mod tracked_value;
fn main(){let a:Vec<_>=std::env::args().collect();let input=std::fs::read(&a[1]).unwrap();let(v,n,items)=tracked_value::TrackedValue::decode_prefix_with_items(&input,unit::Limits::default()).unwrap();let exact=unit::Limits{max_bytes:n,max_items:items};let(out,witems)=v.encode_with_items(exact).unwrap();assert_eq!(items,witems);assert_eq!(v,tracked_value::TrackedValue::decode(&out,exact).unwrap());for short in [unit::Limits{max_bytes:n-1,..exact},unit::Limits{max_items:items-1,..exact}]{assert!(v.encode(short).is_err());assert!(tracked_value::TrackedValue::decode(&input[..n],short).is_err());}std::fs::write(&a[2],out).unwrap();println!("{} {} {} {} {} {}",n,items,v.steps.len(),u8::from(v.include_steps),u8::from(v.min_q8.is_some()),u8::from(v.max_q8.is_some()));}
'''
def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
 p.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra')
 p.add_argument('--source-ref',default='HEAD',help='Immutable committed implementation source; no shared-tree source recovery')
 p.add_argument('--output',type=Path,required=True)
 a=p.parse_args()
 game=Elf(a.assets/'libGameCore_XP2.so',GAME_SHA);Elf(a.assets/'Civ6',HOST_SHA)
 symbols={}
 for line in subprocess.check_output(['nm','-S','-C',str(a.assets/'libGameCore_XP2.so')],text=True).splitlines():
  parts=line.split(' ',3)
  if len(parts)==4:
   address,size,kind,name=parts
   if kind in ('T','W'):symbols[name]=(int(address,16),int(size,16))
 for address,size,name,digest in METHODS:
  if symbols.get(name)!=(address,size):raise ValueError('concrete symbol identity mismatch: '+name)
  if sha(game.read(address,size))!=digest:raise ValueError('method mismatch: '+name)
 if struct.unpack('<I',game.read(0x2f88e98,4))[0]!=1:raise ValueError('current version mismatch')
 calls=[(0x2d50b60,0x1f5fbd0),(0x2d50b75,0x1e20270),(0x2d50b8c,0x1f857a0),(0x2d50baa,0x1fb1bb0),(0x2d50e44,0x1fb67c0),(0x2d50e6e,0x1f7a070),(0x2d50e95,0x1e822d0),(0x2d50eed,0x1f238b0)]
 for address,target in calls:game.call(address,target)
 dwarf=subprocess.check_output(['gdb','-nx','-batch',str(a.assets/'libGameCore_XP2.so'),'-ex',"ptype /o 'TrackedValue'",'-ex',"ptype /o 'std::__1::tuple<TrackedValue, Localization::String, unsigned int, int>'"],text=True)
 if sha(dwarf.encode())!='0cd7cb063b150dfa46db32413b15480fb17a3a2ceb006fc4cd1eca4dfa2a12dd':raise ValueError('matching DWARF layout changed')
 commit=subprocess.check_output(['git','rev-parse',a.source_ref+'^{commit}'],cwd=ROOT,text=True).strip()
 reports=[]
 with tempfile.TemporaryDirectory(prefix='c6-tracked-source-')as tmp:
  source=Path(tmp)
  pack=subprocess.check_output(['git','archive',commit],cwd=ROOT)
  subprocess.run(['tar','-x','-C',str(source)],input=pack,check=True)
  paths=[source/'Cargo.toml',source/'Cargo.lock',source/'crates/c6-core/Cargo.toml',*sorted((source/'crates/c6-core/src').rglob('*.rs'))]
  sources={str(x.relative_to(source)):sha(x.read_bytes())for x in paths}
  subprocess.run(['cargo','build','--offline','-p','c6-core'],cwd=source,check=True)
  harness=HARNESS.replace('MODULE',str(source/'crates/c6-core/src/tracked_value.rs'))
  (source/'candidate.rs').write_text(harness)
  cmd=['rustc','--edition=2021','-Dwarnings',str(source/'candidate.rs'),'--extern','c6_core='+str(source/'target/debug/libc6_core.rlib'),'-L','dependency='+str(source/'target/debug/deps')]
  subprocess.run(cmd+['-o',str(source/'candidate')],check=True)
  subprocess.run(cmd+['--test','-o',str(source/'tests')],check=True)
  subprocess.run([str(source/'tests')],check=True)
  candidate_sha=sha((source/'candidate').read_bytes())
  for identity,trace_name,trace_sha in FIXTURES:
   data=gzip.decompress((a.private_root/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes())
   if sha(data)!=identity:raise ValueError('Core identity mismatch')
   trace=gzip.decompress((a.private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes())
   if sha(trace)!=trace_sha:raise ValueError('owner trace identity mismatch')
   rows=[json.loads(x)for x in trace.splitlines()]
   cuts=sorted([x for x in rows if x.get('event')=='owner'and x.get('save')==1 and x.get('owner')=='GameCore::Player::Stats'],key=lambda x:x['start'])
   if not cuts:raise ValueError('no enclosing Stats validation cuts')
   observations=[];digests=[]
   for cut in cuts:
    # Validation only: the target Stats v27 writer has 22 scalar i32 fields
    # after its version before the diplomatic-victory TrackedValue prefix.
    if struct.unpack_from('<I',data,cut['start'])[0]!=27:raise ValueError('Stats version mismatch')
    payload=data[cut['start']+92:cut['end']]
    (source/'in').write_bytes(payload)
    words=[int(x)for x in subprocess.check_output([str(source/'candidate'),str(source/'in'),str(source/'out')],text=True).split()]
    out=(source/'out').read_bytes()
    if out!=payload[:words[0]]:raise ValueError('saved TrackedValue prefix differs')
    observations.append(words);digests.append(sha(out))
   reports.append(dict(stream_sha256=identity,trace_sha256=trace_sha,records=len(cuts),typed_observations=observations,ordered_record_digests_sha256=sha(json.dumps(digests,separators=(',',':')).encode())))
 report=dict(schema='c6-linux-tracked-value-v1',checks_passed=True,gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,current_version=1,version_address='0x2f88e98',methods=[dict(address=hex(x),size=n,name=name,sha256=h)for x,n,name,h in METHODS],direct_calls=[dict(address=hex(x),target=hex(y))for x,y in calls],dwarf_sha256=sha(dwarf.encode()),source_commit=commit,isolated_committed_source=True,candidate_sources=sources,candidate_sha256=candidate_sha,harness_sha256=sha(harness.encode()),audit_source_sha256=sha(Path(__file__).read_bytes()),fixtures=reports,source_only_tests=4,source_only_branches=['nonempty ordered and duplicate-id steps','include_steps=false with serialized steps','optional min/max and extreme raw Q8 values','recursive localization and all current argument tags','depth64 and over-depth rejection','every-prefix truncation and strict malformed framing'],native_record_coverage='enclosing Stats v27 hash-gated owner cuts; no direct TrackedValue cuts',original_execution=False,private_writes=False,legacy_versions=False,attribution_reference_revision='5af389445118bed2bdebf7a8a42b219c8fba5175',reference_role='discovery_only',complete_current_wire=True,native_read_application=False,constructors=False,callbacks=False,gameplay_rng=False,planning_only_classification=False,full_player=False,world_import=False,gameplay_state=False,actions=False,turns=False,unknown_save_bytes_masked=False,recorded_raw_save_load_difference_bytes=89198)
 a.output.write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(dict(checks_passed=True,methods=len(METHODS),records=sum(x['records']for x in reports))))
if __name__=='__main__':main()
