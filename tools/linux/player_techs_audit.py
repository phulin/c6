#!/usr/bin/env python3
"""Pinned read-only Techs contracts and offline saved-record comparisons; no original execution."""
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
SOURCE_PIN="5057bac"
METHODS = [(45673360,
  67,
  'FAutoVariable<GameCore::Utilities::Queue<GameCore::TechTypes, 4>, '
  'GameCore::Player::Techs>::dummyLoadDelta(Serialization::IStream&)',
  '2b0a1944b140865d9b2cb3c6057b79457cf01e37be0f280a4a638a7c1ba89001'),
 (45673328,
  15,
  'FAutoVariable<GameCore::Utilities::Queue<GameCore::TechTypes, 4>, '
  'GameCore::Player::Techs>::load(Serialization::IStream&)',
  '2a25fb352e651e85a85ab07e925fbc78c5376ca7777eb104c038936e86ea8616'),
 (45673344,
  9,
  'FAutoVariable<GameCore::Utilities::Queue<GameCore::TechTypes, 4>, '
  'GameCore::Player::Techs>::loadDelta(Serialization::IStream&)',
  '167c0fdc50d578e181c5dddff2172f571505e7e00cb3cf946d8dc5f0b4ead6fa'),
 (33645696,
  34,
  'int '
  'Serialization::HashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> '
  '>(unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> const&)',
  'bbba3422fa0bd6194a1c30c88ee7b8e77f8f7f0733c020e55d5c967202a1dd21'),
 (33646304,
  66,
  'unsigned int '
  'Serialization::IndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> '
  '>(unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> const&)',
  'ff8dfe200251d2bcc9ff280ba3b4dcc5a7c69ec8ca304022f0971b19b81e6907'),
 (34505664,
  36,
  'int '
  'Serialization::ReadHashToIndex<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> '
  '>(Serialization::IStream&, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> const&)',
  'f845c681d586b1747a7ef50ff6e12d05359e9200b67d2b8f0a060216e7c0aa90'),
 (33612288,
  41,
  'void '
  'Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> '
  '>(Serialization::OStream&, unsigned int, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> const&)',
  '175d86dbe583455f87afaeec42db117ecf32444a8129b3fbd0f1e2306ca31abc'),
 (45669984,
  157,
  'bool Serialization::ReadIndexHashArray<FixedPointT<8u>, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> '
  '>(Serialization::IStream&, FixedPointT<8u>*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> const&)',
  '6cc21bd5aee5a51c276450691b384a9e359d688a69ba01d4caef3e0c9e3a285c'),
 (35192160,
  149,
  'bool Serialization::ReadIndexHashArray<bool, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> '
  '>(Serialization::IStream&, bool*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> const&)',
  '40b036252c1824179cc68526958d444b6d22db6d9791ec360d73dfacc11e3083'),
 (45669712,
  141,
  'bool Serialization::WriteIndexHashArray<FixedPointT<8u>, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> '
  '>(Serialization::OStream&, FixedPointT<8u> const*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> const&)',
  'a7b88d3c271637165513b272f7dc875930f9a5beba54df77bdedd83fbd961326'),
 (35192400,
  138,
  'bool Serialization::WriteIndexHashArray<bool, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> '
  '>(Serialization::OStream&, bool const*, '
  'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Technology> const&)',
  'ff63bebb8b64f6198886ad14a9b928b9e9201b097a4a00784c99e9f8335d6249'),
 (46772192,
  42,
  'Serialization::operator<<(Serialization::OStream&, FixedPointT<8u> const&)',
  'bbd0553ff77e4be5a896850082327f68662e870164f4771d58a0e5c73c7d5704'),
 (45653904,
  242,
  'Serialization::operator<<(Serialization::OStream&, GameCore::Player::Techs const&)',
  'a1eee5b85bf91e347826bbf1fc76bdaa2c7f54e983e55d9bde95e9b36055e8d6'),
 (47509232,
  36,
  'Serialization::operator<<(Serialization::OStream&, GameCore::TechTypes const&)',
  'e8b21f56f96030386a4c9aac4582d7fce63120854b0a642cf15392ac5210cdad'),
 (45676160,
  105,
  'Serialization::OStream& Serialization::operator<< <GameCore::TechTypes>(Serialization::OStream&, '
  'GameCore::Utilities::Queue<GameCore::TechTypes, 4> const&)',
  '2dd86784c6563fd6019e4f3ebef2beda5df8e6765f432da6037bebb0168b8c58'),
 (46772240,
  43,
  'Serialization::operator>>(Serialization::IStream&, FixedPointT<8u>&)',
  '5d47fcdfb3868524b3d8da8a2fab95de165678efa05f23c615963da12ddf010d'),
 (45654160,
  494,
  'Serialization::operator>>(Serialization::IStream&, GameCore::Player::Techs&)',
  'eadb0d0289bb1cdefd1870b3bccac5a6022119147595fdd84327e634156b6553'),
 (47509280,
  58,
  'Serialization::operator>>(Serialization::IStream&, GameCore::TechTypes&)',
  '047050d264acc31d6a4477873412f54329d63a12b971850fa8ca7602c1cfa31c'),
 (45676048,
  108,
  'Serialization::IStream& Serialization::operator>><GameCore::TechTypes>(Serialization::IStream&, '
  'GameCore::Utilities::Queue<GameCore::TechTypes, 4>&)',
  'a27a57c1df6d34ec421b4f58135f77b4bfa3f0ae886f433bd73677c5e840cbba'),
 (45673440,
  15,
  'FAutoVariable<GameCore::Utilities::Queue<GameCore::TechTypes, 4>, '
  'GameCore::Player::Techs>::save(Serialization::OStream&) const',
  '4ec8ebead703d66071bc76f0235c8df8e98a5b527acde0698e7ead5ed0090f0b'),
 (45673600,
  1815,
  'FAutoVariable<GameCore::Utilities::Queue<GameCore::TechTypes, 4>, '
  'GameCore::Player::Techs>::compare(Serialization::IStream&, '
  'Types::Vector<Types::pair<AutoVariableString, AutoVariableString>, Types::LinearAlloc::Default<1u, '
  '0u> > const&) const',
  '79638e031e71d4624484bcff28d82cbf7839e1934ab29bc8b42d4b17eef03daf'),
 (45673456,
  9,
  'FAutoVariable<GameCore::Utilities::Queue<GameCore::TechTypes, 4>, '
  'GameCore::Player::Techs>::saveDelta(Serialization::OStream&) const',
  'd36ea3021ffa47b529ed32deccaac19d2e06c32d8eaaf12802f2a9502f727aaa'),
 (45669648,
  21,
  'Serialization::OStream& operator<< <FixedPointT<8u>, '
  'GameCore::Player::Techs>(Serialization::OStream&, FAutoVariable<FixedPointT<8u>, '
  'GameCore::Player::Techs> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (45669616,
  21,
  'Serialization::OStream& operator<< <GameCore::Utilities::Queue<GameCore::TechTypes, 4>, '
  'GameCore::Player::Techs>(Serialization::OStream&, '
  'FAutoVariable<GameCore::Utilities::Queue<GameCore::TechTypes, 4>, GameCore::Player::Techs> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (45669680,
  21,
  'Serialization::OStream& operator<< <int, GameCore::Player::Techs>(Serialization::OStream&, '
  'FAutoVariable<int, GameCore::Player::Techs> const&)',
  '449f3cf4add733712c9d8730e004d66cf06f33459d9190f6e6242aad6c2f52f0'),
 (45669888,
  21,
  'Serialization::IStream& operator>><FixedPointT<8u>, '
  'GameCore::Player::Techs>(Serialization::IStream&, FAutoVariable<FixedPointT<8u>, '
  'GameCore::Player::Techs>&)',
  '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55'),
 (45669856,
  21,
  'Serialization::IStream& operator>><GameCore::Utilities::Queue<GameCore::TechTypes, 4>, '
  'GameCore::Player::Techs>(Serialization::IStream&, '
  'FAutoVariable<GameCore::Utilities::Queue<GameCore::TechTypes, 4>, GameCore::Player::Techs>&)',
  '4e8b90f2c2ff7b59319d6ac09e5fb89c5e13953ca3ac4f9faf4342de51a46c55')]
TYPES=['Techs']
DWARF_SHA='ce973bd273feb8e10f2dbaaceb8c3818d3dfcf58f1e9fa352a231d8eea3db15d'
HARNESS=r'''pub use c6_core::{unit,player_prerequisites};
#[path="MODULE"] pub mod player_techs;
fn main(){let args:Vec<_>=std::env::args().collect();let input=std::fs::read(&args[1]).unwrap();let limits=unit::Limits{max_bytes:64*1024*1024,max_items:4_000_000};let(v,n,items)=player_techs::Techs::decode_prefix_with_items(&input,limits).unwrap();assert_eq!(n,input.len());for k in 0..input.len(){assert!(player_techs::Techs::decode(&input[..k],limits).is_err());}let exact=unit::Limits{max_bytes:n,max_items:items};assert_eq!(player_techs::Techs::decode(&input,exact).unwrap(),v);assert_eq!(v.encode_with_items(exact).unwrap().1,items);for low in [unit::Limits{max_bytes:n-1,..exact},unit::Limits{max_items:items-1,..exact}]{assert!(player_techs::Techs::decode(&input,low).is_err());assert!(v.encode(low).is_err());}let(out,witems)=v.encode_with_items(limits).unwrap();assert_eq!(items,witems);std::fs::write(&args[2],out).unwrap();println!("{} {} {} {} {} {}",n,items,v.research_queue_hashes.len(),v.has_tech.pairs.len(),v.boost_triggered.pairs.len(),v.research_progress_q8.len());}
'''
def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
 p.add_argument('--private-root',type=Path,default=ROOT.parent/'c6-ghidra')
 p.add_argument('--output',type=Path,required=True)
 a=p.parse_args()
 game=Elf(a.assets/'libGameCore_XP2.so',GAME_SHA)
 Elf(a.assets/'Civ6',HOST_SHA)
 for address,size,name,digest in METHODS:
  if sha(game.read(address,size))!=digest:raise ValueError('method mismatch: '+name)
 if struct.unpack('<I',game.read(0x2f685e4,4))[0]!=6:raise ValueError('current version mismatch')
 calls=[(0x2b4f0ad,0x1eb9310),(0x2b4f0c0,0x1f4cb40),(0x2b4f0d3,0x1f84390),(0x2b4f0e6,0x1f66d50),(0x2b4f0f9,0x1f35f50)]
 for address,target in calls:game.call(address,target)
 command=['gdb','-nx','-batch',str(a.assets/'libGameCore_XP2.so')]
 for name in TYPES:command+=['-ex',"ptype /o 'GameCore::Player::"+name+"'"]
 dwarf=subprocess.check_output(command,text=True)
 if sha(dwarf.encode())!=DWARF_SHA:raise ValueError('matching DWARF changed')
 pin=subprocess.check_output(['git','rev-parse',SOURCE_PIN+'^{commit}'],cwd=ROOT,text=True).strip()
 reports=[]
 with tempfile.TemporaryDirectory(prefix='c6-techs-')as tmp:
  tmp=Path(tmp); source=tmp/'source';source.mkdir()
  archive=subprocess.check_output(['git','archive',pin],cwd=ROOT)
  subprocess.run(['tar','-x','-C',str(source)],input=archive,check=True)
  paths=[source/'Cargo.toml',source/'Cargo.lock',source/'crates/c6-core/Cargo.toml',*sorted((source/'crates/c6-core/src').rglob('*.rs'))]
  sources={str(x.relative_to(source)):sha(x.read_bytes())for x in paths}
  subprocess.run(['cargo','build','--offline','-p','c6-core'],cwd=source,check=True)
  harness=HARNESS.replace('MODULE',str(source/'crates/c6-core/src/player_techs.rs'));(tmp/'candidate.rs').write_text(harness)
  cmd=['rustc','--edition=2021','-Dwarnings',str(tmp/'candidate.rs'),'--extern','c6_core='+str(source/'target/debug/libc6_core.rlib'),'-L','dependency='+str(source/'target/debug/deps')]
  subprocess.run(cmd+['-o',str(tmp/'candidate')],check=True)
  subprocess.run(cmd+['--test','-o',str(tmp/'tests')],check=True)
  subprocess.run([str(tmp/'tests')],check=True)
  candidate_sha=sha((tmp/'candidate').read_bytes())
  for identity,trace_name,trace_sha in FIXTURES:
   data=gzip.decompress((a.private_root/'evidence/linux-game-load/canonical'/(identity+'.bin.gz')).read_bytes())
   if sha(data)!=identity:raise ValueError('Core identity mismatch')
   trace=gzip.decompress((a.private_root/'evidence/linux-save-normalization/checkpoint-203-owners'/trace_name).read_bytes())
   if sha(trace)!=trace_sha:raise ValueError('owner trace identity mismatch')
   rows=[json.loads(x)for x in trace.splitlines()]
   cuts=sorted([x for x in rows if x.get('event')=='owner'and x.get('save')==1 and x.get('owner')=='GameCore::Player::Techs'],key=lambda x:x['start'])
   if not cuts:raise ValueError('no Techs validation cuts')
   observations=[];digests=[];total=0
   for cut in cuts:
    payload=data[cut['start']:cut['end']];(tmp/'in').write_bytes(payload)
    result=subprocess.check_output([str(tmp/'candidate'),str(tmp/'in'),str(tmp/'out')],text=True)
    if (tmp/'out').read_bytes()!=payload:raise ValueError('saved record differs')
    words=[int(x)for x in result.split()]
    if words[0]!=len(payload):raise ValueError('typed prefix mismatch')
    total+=len(payload);observations.append(words);digests.append(sha(payload))
   reports.append(dict(stream_sha256=identity,trace_sha256=trace_sha,records=len(cuts),record_bytes=total,first_observed_range=[cuts[0]['start'],cuts[0]['end']],ordered_record_digests_sha256=sha(json.dumps(digests,separators=(',',':')).encode()),typed_observations_sha256=sha(json.dumps(observations,separators=(',',':')).encode())))
 report=dict(schema='c6-linux-player-techs-v1',checks_passed=True,gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,current_version=6,version_address='0x2f685e4',methods=[dict(address=hex(x),size=n,name=name,sha256=h)for x,n,name,h in METHODS],direct_calls=[dict(address=hex(x),target=hex(y))for x,y in calls],dwarf_sha256=sha(dwarf.encode()),candidate_source_commit=pin,candidate_archive_sha256=sha(archive),candidate_sources=sources,candidate_sha256=candidate_sha,harness_sha256=sha(harness.encode()),audit_source_sha256=sha(Path(__file__).read_bytes()),fixtures=reports,source_only_tests=5,exact_native_record_byte_item_caps_checked=True,one_short_native_record_caps_rejected=True,every_native_record_prefix_rejected=True,allocation_policy='checked per-vector storage; aggregate consumed wire bytes and items',original_execution=False,private_writes=False,complete_current_techs_wire=True,definition_remapping=False,native_read_fixups=False,full_player=False,world_import=False,gameplay_state=False,actions=False,turns=False,next_player_component='Treasury',unknown_save_bytes_masked=False,recorded_raw_save_load_difference_bytes=89198)
 a.output.write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(dict(checks_passed=True,methods=len(METHODS),fixtures=reports)))
if __name__=='__main__':main()
