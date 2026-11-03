#!/usr/bin/env python3
"""Source-only concrete manager-world replay against genuine typed inputs."""
import argparse
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import re
import sqlite3
import subprocess
import tarfile
import tempfile
from save_lifecycle_audit import Elf,GAME_SHA,HOST_SHA
from player_header_control_audit import METHODS as HEADER_METHODS
from unit_placement_audit import METHODS as PLACEMENT_METHODS
from core_stream_index import Reader,index_core
ROOT=Path(__file__).resolve().parents[2]
SOURCE_COMMIT='7e6421d'
REGISTRY_SHA='d8a43667523e9385bfa9ac35594027609b26d7755551c0bab46d226917899901'
INPUT_SHA='79167949acf5e3482f0782bcc973b4ef5e56119ef1af0d10771f56b6b62950d0'
CORE_SHA='ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884'
METHODS = [('IndexList.Read', 46398144, 67, '469860856d8494b9b1b61725ffddbf58184c9bceae2bbd2827770870f9e5205b'),
 ('IndexHash.Read', 46398240, 149, '610cf73f5268b4f5a066c3c69567be2ed1c28e991458aeb7f128c4a3b3f3ea9c'),
 ('IndexList.Reset', 46398224, 5, '22d3c6f46ccb861f9cb58deb039521cf102ea09aa55fd7abe060e0af78f1ea6f'),
 ('IndexList.DoReset', 46398416, 51, 'a73bf58b8f4686456fb53d6a61112e44df6993edfa62e737f332cbcb6e1891e3'),
 ('IndexList.Get', 46398704, 4, 'f84e915a39a42129d4d8c8f1c771b4a320ba7b1ab0c17754721be4ff7b835a21'),
 ('BoolVector.Size', 33586992, 4, '52a506bbea43a2d8d40e5f64b63939b2c437dfb6aae1f52216a775f464b2e537'),
 ('BoolVector.ConstData',
  34013152,
  5,
  'e54001fc66cad199ad6d854687800685c06eab4392fc86a83b8e136df1008f1e'),
 ('HashToIndex', 34512752, 34, 'b1857e1ade1de689270eacaa15b82957c4a6eaccbf045d053fe6771b1017cad5'),
 ('Operation.FindIndex',
  46318112,
  27,
  '0987f96defd19d195435710a642c1e5fcee85058aee80d49076836b0190e8f2d'),
 ('Operation.IsDisabled',
  46307296,
  73,
  'de0f4942cf7c5e7e3e4fb4cd9ea1973ef50dc1b903add1ff673c871556839751'),
 ('Operation.IsValidIndex',
  46318160,
  23,
  'cfae545fe0031fd04ae072a8b013c6d5ba4739a1269f753a10ecc2b12655707f'),
 ('Operation.ConstAt', 46319424, 55, 'e08593a7c1125bb9289d36e04c49770b7015123437fc7e131ea4c5661bfa2258'),
 ('Definitions.Count', 34511808, 5, '10b6b99e5adb80723195bcee32ed25b405c7afc2575bdd0d413f2c463cf8de2d'),
 ('Definitions.Size', 34511824, 12, '8375e78d364001210bb02186b4a5388e8e09554d7905f614c75f29f2692f7228'),
 ('Phase.Initialize', 44386048, 181, '23621eded7d30d7320575912b08ea3affa64b3112114a84eb7a5a24b24de7147'),
 ('Phase.Comparator', 44390400, 36, 'a0e446f71325e9a8ac038ce669e34600daa0f096c7267797d03c819f3e6346de'),
 ('Phase.GetCurrent', 44388912, 35, '234514ebec562e9eaf779528d82ee52914ca963c8b8559bc4ef6469203e72e5e'),
 ('Phase.Tactical', 44388144, 37, '794ae6ccbdb27984a0baa907c011cab9858884f434482532c7d853791350169b'),
 ('Phase.GetActive', 44391328, 5, 'b811adb2d88317c4120dc7c33dd574a1678d7f98965dd45cb6cf6745cafe2e73'),
 ('Phase.GetType', 44391664, 4, '8e1eb1e8ee76f9d9fd9091b8e14772902a0c006899df4ef599bf389b61b64d35'),
 ('Phase.GetOrder', 44393152, 4, 'de9f36dc140db6f626d63850679aba7ae4d76e6d68a564a1b411de985292fe18'),
 ('Phase.RowId', 36845472, 4, '52a506bbea43a2d8d40e5f64b63939b2c437dfb6aae1f52216a775f464b2e537'),
 ('Segment.RowId', 36845584, 4, '52a506bbea43a2d8d40e5f64b63939b2c437dfb6aae1f52216a775f464b2e537'),
 ('Segment.Tactical', 44391360, 8, '06059c117d2525723825777083e470ee9220181f3044fd23f4b764be0ac6b038'),
 ('Phase.VectorBegin', 44391264, 4, 'afae45c906d9e5d93a0612a2360b14d31fd758c9acf5c8f8787b5095927afc23'),
 ('Phase.VectorEnd', 44391280, 5, '8e3eaf4f4b235f364076d47e3b5b95d9e60a71cf19dd62030d0fa20e80858d2e'),
 ('Phase.Builder', 35760160, 389, '2b2569629afa4f05ff751ed87d8ecad015dd04b507d5b74411b7137efd2b5459'),
 ('Phase.BuilderSql', 35760576, 8, '5f2e3ec97837279b0c7973f1f18441d2f3b86c3e92cbec55b193decd25f61077'),
 ('Segment.Builder', 35760848, 424, '902e20a599d3e8e9028532794282e4d39b8f61c6640343434814d0e6b48110ce'),
 ('Segment.BuilderSql', 35761296, 8, 'a0b58ff8f68287a35f3f6451f8cb7f0fe7fa57a36387bc8a414a8e5698cfc853'),
 ('Phase.ActiveResolver',
  36122416,
  245,
  '90ae5e43365bd3c5cbacc8999ad6328f5952e30774ceae17fbe2d8df0d5d3e5f'),
 ('Phase.ActiveSql', 36845456, 8, '7d6529ffbfc79f249d248423d4ed687d8129683b03581b554d5f2820654f4d2c'),
 ('Phase.AssignActive', 36845568, 5, '6b052b6073ac8eb0730958849dd25b6e080178cd182ea1f3a164f8884608afdc'),
 ('Phase.SetActive', 36845600, 5, '84653c07fca1d0feeb30649cdb5b6bff351959cf87f7542200eb23c319e3c779'),
 ('Segment.SetTactical',
  36123088,
  15,
  '7effd3148f19cc3c93c537209406a75149c58f9eccd97b32c4684f974ff9ecb6'),
 ('Player.TurnActive', 33689648, 7, 'f44ea052578d46f191ac4e546446f30a3022dc5085b44ee6b9378719eabb8c5b'),
 ('Player.GetId', 33341760, 7, 'e997d06cf0d3f9b3bb89701ec15a7b1b096198fdc4b0dc87fc0ca29c37f6dadc'),
 ('Global.GetPlayer', 43039024, 22, '4f70fef1af78b8dea71c792f71f54727722e3e9f4509f9e764e536223739daaf'),
 ('Game.GetPlayer', 40245984, 15, '07eb0641e390978eef06ae34398421fbd03a7269d9a3beb028451ee230f40061'),
 ('Game.Paused', 42055456, 7, 'bf1c7941b3b62b57c2112fc90a57e28fbe0b8e7ab2b783e84a7c6731d061630f'),
 ('Player.ProcessingAutomation',
  45466560,
  7,
  '750b0f58d460bdbe162c9ce1512bdd6a141a3527ce526bc67ef708695777aafb'),
 ('Script.GetSystem', 35067488, 11, '0d5bf15629235063deb60052f71390c42fd80e98a458e0775078bcd7bb77e07e'),
 ('Config.TurnPhase', 42879088, 51, '2ae899c84fc261748e99f2f6198cb0b9c6528623b1268aa8912cd7bfc8e36fc7'),
 ('Config.Network', 42879152, 81, 'a785856ac0f0e8391e79e5cafeb5ee3b889d22211129933e6657a59fb4c4a57e')]
CALLS = [(46398157, 32726064),
 (46398165, 33186928),
 (46398173, 32484192),
 (46398181, 32242656),
 (46398195, 32565216),
 (46398268, 31815744),
 (46398310, 31815744),
 (46398322, 31979824),
 (46398358, 32766320),
 (46398428, 32242656),
 (46398436, 31981120),
 (46398456, 31654288),
 (34512760, 31879312),
 (46318113, 31879312),
 (46307304, 32242656),
 (46307314, 32235840),
 (46307333, 31520592),
 (46307347, 33205680),
 (46318163, 32227360),
 (46318171, 31909040),
 (46319439, 31909040),
 (44386059, 31685296),
 (44386074, 32647472),
 (44386079, 31653856),
 (44386090, 31468576),
 (44386101, 31893776),
 (44386131, 31538528),
 (44386141, 32555312),
 (44386163, 32644160),
 (44386187, 33205360),
 (44386198, 31641328),
 (44386209, 44389376),
 (44390409, 31553280),
 (44390419, 31553280),
 (44388931, 32373280),
 (44388145, 32663424),
 (44388158, 32693360),
 (35760177, 31925616),
 (35760225, 33061984),
 (35760249, 32662448),
 (35760277, 32816064),
 (35760323, 32765792),
 (35760361, 31891808),
 (35760372, 33049152),
 (35760429, 32433760),
 (35760456, 32855360),
 (35760484, 33137872),
 (35760512, 32033040),
 (35760536, 31956832),
 (35760544, 32992576),
 (35760860, 31925616),
 (35760906, 31876416),
 (35760930, 32064992),
 (35760958, 32278176),
 (35760986, 31642192),
 (35761014, 31863552),
 (35761071, 32987600),
 (35761128, 32986736),
 (35761155, 31656912),
 (35761182, 32818576),
 (35761209, 31815232),
 (35761237, 32831456),
 (35761259, 31956832),
 (35761267, 32992576),
 (36122441, 32416416),
 (36122475, 32268880),
 (36122486, 32573936),
 (36122522, 31481920),
 (36122572, 32582256),
 (36122588, 32107872),
 (36122648, 32178544),
 (36122656, 32992576),
 (43039039, 32123472),
 (40245992, 32078528),
 (42879092, 32813936),
 (42879178, 32340400)]

DRIVER = r'''
use c6_data::unit_operation_manager::*;
use c6_data::{Rules,RulesProfile};
use c6_data::registry::linux_type_hash;
use c6_core::unit::{Unit,Limits,PlotCoord};
use c6_core::world::{World,Player};
use c6_core::core_player::PlayerHeader;
use serde_json::{Value,json};
use sha2::{Digest,Sha256};
use std::{fs,path::{Path,Component}};
fn sha(b:&[u8])->String {format!("{:x}",Sha256::digest(b))}
fn bytes(root:&Path,v:&Value)->Vec<u8> {
 let p=Path::new(v["path"].as_str().unwrap());assert!(p.components().all(|x|matches!(x,Component::Normal(_))));
 let b=fs::read(root.join(p)).unwrap();assert_eq!(b.len() as u64,v["bytes"].as_u64().unwrap());assert_eq!(sha(&b),v["sha256"].as_str().unwrap());b
}
fn main() {
 let a:Vec<_>=std::env::args().collect();let input:Value=serde_json::from_slice(&fs::read(&a[1]).unwrap()).unwrap();let root=Path::new(&a[1]).parent().unwrap();
 let registry:Value=serde_json::from_slice(&fs::read(&a[2]).unwrap()).unwrap();
 let entries:Vec<_>=registry.as_array().unwrap().iter().map(|v|OperationDefinition {collection_index:v["index"].as_u64().unwrap() as u32,hash:v["hash"].as_u64().unwrap() as u32}).collect();
 let definitions=OperationDefinitions::from_native_order(&entries).unwrap();
 let mut rows=vec![];let mut players=vec![];
 for p in input["players"].as_array().unwrap() {
  let owner=p["owner"].as_i64().unwrap() as i32;
  let mut player=Player::new(owner,None,None).unwrap();player.slot_status=Some(p["slot_status"].as_i64().unwrap() as i32);players.push(player);
  for e in p["units"]["entries"].as_array().unwrap() {
   let wire=bytes(root,&e["wire"]);let unit=Unit::decode(&wire,Limits::default()).unwrap();assert_eq!(unit.e_owner,owner);assert_eq!(unit.id as u64,e["id"].as_u64().unwrap());
   let restored=definitions.restore_unit_disabled(&unit).unwrap();
   let values:Vec<_>=(0..definitions.count()).map(|i|restored.is_disabled(RuntimeOperationId(i as i32))).collect();assert_eq!(values,restored.values());
   assert_eq!(unit.encode(Limits::default()).unwrap(),wire);
   rows.push(json!({"owner":owner,"id":unit.id,"pairs":unit.k_operations_disabled.iter().map(|v|json!([v.definition_hash,v.value])).collect::<Vec<_>>(),"values":values}));
  }
 }
 let map=c6_core::map::Map::decode(&bytes(root,&input["map"]),c6_core::map::Limits::default()).unwrap();
 let mut world=World::new(map,players).unwrap();world.player_slot_present=Some(std::array::from_fn(|i|input["player_slot_present"][i].as_bool().unwrap()));world.player_mapping=Some(std::array::from_fn(|i|input["player_mapping"][i].as_i64().unwrap() as i32));
 let h:Value=serde_json::from_slice(&fs::read(&a[4]).unwrap()).unwrap();let header=PlayerHeader {owner:h["owner"].as_i64().unwrap() as i32,starting_plot:PlotCoord {x:h["starting_plot"][0].as_i64().unwrap() as i32,y:h["starting_plot"][1].as_i64().unwrap() as i32},ordered_scalars:std::array::from_fn(|i|h["scalars"][i].as_i64().unwrap() as i32),ordered_flags:std::array::from_fn(|i|h["flags"][i].as_bool().unwrap()),handicap_hash:h["tail"][0].as_u64().unwrap() as u32,district_hash:h["tail"][1].as_u64().unwrap() as u32,project_hash:h["tail"][2].as_u64().unwrap() as u32,final_scalar:h["tail"][3].as_u64().unwrap() as u32 as i32};
 let player_rows:Vec<_>=world.players().iter().map(|p|{let p=WorldPlayer::resolve(&world,p.owner(),if p.owner()==header.owner {Some(&header)}else{None}).unwrap();json!({"id":p.id(),"human":p.is_human().unwrap(),"turn_active":p.is_turn_active().ok()})}).collect();
 let source=RulesProfile::LinuxXp2ActivatedDiagnostic.source_identity().unwrap();let rules=Rules::load_profile(RulesProfile::LinuxXp2ActivatedDiagnostic,&a[3],&source).unwrap();let mut phases=vec![];
 for name in ["TURNPHASE_SINGLEPLAYER","TURNPHASE_SIMULTANEOUS","TURNPHASE_DYNAMIC","TURNPHASE_TWOPHASE"] {let defs=PhaseDefinitions::from_rules(&rules,linux_type_hash(name.as_bytes())).unwrap();for (i,p) in defs.slots().iter().enumerate(){let p=p.unwrap();phases.push(json!({"type":name,"slot":i,"rowid":p.row_id.0,"order":p.phase_order,"segment_rowid":p.active_segment.map(|s|s.row_id.0),"flags":p.active_segment.map(|s|s.flags),"allow":defs.allow_tactical_commands(i as i32,header.owner).unwrap()}));}}
 println!("{}",json!({"rows":rows,"players":player_rows,"phases":phases,"definition_count":definitions.count(),"all_units_unchanged":true}));
}
'''

def sha(b): return hashlib.sha256(b).hexdigest()
def require(v,m):
 if not v: raise ValueError(m)
def load(p): return json.loads(Path(p).read_text())
def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
 p.add_argument('--rules',type=Path,default=ROOT.parent/'c6-ghidra/evidence/linux-rules/mod-activation-xp2-constructor-final-1')
 p.add_argument('--registry',type=Path,default=ROOT.parent/'c6-ghidra/evidence/linux-operation-registry/native-1.json')
 p.add_argument('--input',type=Path,default=ROOT.parent/'c6-ghidra/evidence/linux-placement-count-reconstruction/native-1-oyqkhma0/input.json')
 p.add_argument('--output',type=Path,required=True)
 p.add_argument('--work-dir',type=Path)
 a=p.parse_args();elf=Elf(a.assets/'libGameCore_XP2.so',GAME_SHA);Elf(a.assets/'Civ6',HOST_SHA)
 methods=METHODS+[(name,addr,size,h)for addr,size,name,h in HEADER_METHODS]
 methods += [(n,r,s,h)for n,r,s,h in PLACEMENT_METHODS if n in ['Unit.Read','Unit.PostLoad','UnitType.Get','UnitTypes.Read','UnitTypes.HashToIndex']]
 for name,addr,size,h in methods:require(sha(elf.read(addr,size))==h,'method changed: '+name)
 for addr,target in CALLS:elf.call(addr,target)
 for addr,code in [(0x2c3fbd8,'c6431800'),(0x2c3fbe9,'c644240f00'),(0x2b4ef42,'498db680020000'),(0x2b4f88d,'498db680020000'),(0x2021030,'8a8790020000c3'),(0x2a55bc0,'8a47502402d0e8c3')]:require(elf.read(addr,len(code)//2).hex()==code,'state boundary changed')
 dwarf=subprocess.check_output(['gdb','-nx','-batch',str(a.assets/'libGameCore_XP2.so'),'-ex',"ptype 'GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitOperation>'",'-ex',"ptype /o 'eastl::VectorBase<GameCore::Definition::TurnPhase const*, eastl::allocator>'",'-ex',"ptype /o 'GameCore::IndexList<bool, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitOperation>, &GameCore::Definitions::GetUnitOperationDefinitions, void>'",'-ex',"ptype /o 'GameCore::Player::Instance'"],text=True)
 for part in ['uint32_t size() const','uint32_t GetCount() const','int32_t FindIndex(uint32_t) const','mpCapacity','s_Purgatory','total size (bytes):   32']:require(part in dwarf,'DWARF type/layout changed')
 require(re.search(r'/\*\s+640\s+\|\s+24\s+\*/.*?\}\s+m_bIsTurnActive;',dwarf,re.S),'turn-active wrapper identity changed')
 require(re.search(r'/\*\s+656\s+\|\s+1\s+\*/\s+ObjectType m_value;',dwarf),'turn-active value identity changed')
 require(sha(a.input.read_bytes())==INPUT_SHA,'genuine input identity differs')
 require(sha(a.registry.read_bytes())==REGISTRY_SHA,'genuine registry identity differs')
 native_registry=load(a.registry);mapping=native_registry['mapping'];require(mapping['count']==mapping['handler_slots']==63,'registry extent differs')
 registry=[dict(index=e['id'],hash=e['hash'])for e in mapping['entries']]
 require(all(e['index']==i for i,e in enumerate(registry)),'definition order differs')
 work=(a.work_dir or Path(tempfile.mkdtemp(prefix='replay-',dir=ROOT.parent/'c6-ghidra/evidence/linux-unit-operation-manager-world'))).resolve();source=work/'source';fresh=not source.exists();source.mkdir(exist_ok=True)
 archive=subprocess.check_output(['git','archive',SOURCE_COMMIT],cwd=ROOT)
 with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
  if fresh:tar.extractall(source,filter='data')
  expected={m.name for m in tar.getmembers() if m.isfile() or m.issym()}
  require({str(p.relative_to(source))for p in source.rglob('*')if p.is_file() or p.is_symlink()}==expected,'frozen source file inventory changed')
  for member in tar.getmembers():
   if member.isfile():require(sha((source/member.name).read_bytes())==sha(tar.extractfile(member).read()),'frozen source tree changed: '+member.name)
 source_files=['crates/c6-data/src/unit_operation_manager.rs','crates/c6-data/src/lib.rs']
 source_hashes={name:sha((source/name).read_bytes())for name in source_files}
 for name,h in source_hashes.items():require(sha((ROOT/name).read_bytes())==h,'owned source changed since freeze: '+name)
 (work/'src').mkdir(exist_ok=True);(work/'src/main.rs').write_text(DRIVER)
 cargo='[package]\nname="c6-manager-world-replay"\nversion="0.1.0"\nedition="2021"\n[dependencies]\n'
 for name,value in [('c6-core','{path='+json.dumps(str(source/'crates/c6-core'))+'}'),('c6-data','{path='+json.dumps(str(source/'crates/c6-data'))+'}'),('serde_json','"1"'),('sha2','"0.10"')]:cargo+=name+'='+value+'\n'
 (work/'Cargo.toml').write_text(cargo);(work/'registry.json').write_text(json.dumps(registry))
 core=gzip.decompress((ROOT.parent/'c6-ghidra/evidence/linux-game-load/canonical'/(CORE_SHA+'.bin.gz')).read_bytes());require(sha(core)==CORE_SHA,'canonical core differs')
 section=next(s for s in index_core(core,first_formations=True)['sections']if s['name']=='first_player_prefix')
 r=Reader(core[section['start']:section['end']]);r.version(47);header=dict(owner=r.take('i'),starting_plot=[r.take('i'),r.take('i')],scalars=[r.take('i')for _ in range(15)],flags=[r.boolean()for _ in range(6)],tail=[r.u32()for _ in range(4)])
 require(r.pos==section['bytes'],'header boundary differs');(work/'header.json').write_text(json.dumps(header))
 env=os.environ.copy()
 for k in ['CARGO_TARGET_DIR','CARGO_BUILD_TARGET','RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER']:env.pop(k,None)
 subprocess.run(['cargo','build','--offline','--release'],cwd=work,env=env,check=True)
 binary=work/'target/release/c6-manager-world-replay';raw=subprocess.check_output([str(binary),str(a.input),str(work/'registry.json'),str(a.rules),str(work/'header.json')],env=env)
 (work/'reconstruction.json').write_bytes(raw);result=json.loads(raw);require(len(result['rows'])==1262 and result['all_units_unchanged'],'typed Unit coverage/mutation differs')
 hashes=[e['hash']for e in registry]
 def find(key):
  if key<len(hashes):return key
  return next((i for i in range(len(hashes)-1,-1,-1)if hashes[i]==key),None)
 saved_pairs=unknown=disabled=0
 for row in result['rows']:
  values=[False]*len(hashes)
  for key,value in row['pairs']:
   saved_pairs+=1;index=find(key)
   if index is None:unknown+=1
   else:values[index]=value
  require(values==row['values'],'independent disabled remapping differs');disabled+=sum(values)
 connection=sqlite3.connect('file:'+str(a.rules/'gameplay.sqlite')+'?mode=ro',uri=True)
 expected=[]
 for name in ['TURNPHASE_SINGLEPLAYER','TURNPHASE_SIMULTANEOUS','TURNPHASE_DYNAMIC','TURNPHASE_TWOPHASE']:
  records=connection.execute('SELECT P.rowid,P.PhaseOrder,S.rowid,S.AllowStrategicCommands,S.AllowTacticalCommands,S.AllowTurnUnready FROM TurnPhases P LEFT JOIN TurnSegments S ON S.rowid=(SELECT T.rowid FROM TurnSegments T WHERE T.TurnSegmentType=P.ActiveSegmentType ORDER BY T.rowid ASC LIMIT 1) WHERE P.TurnPhaseType=? ORDER BY P.PhaseOrder',(name,)).fetchall()
  for i,(rowid,order,sid,strategic,tactical,unready) in enumerate(records):expected.append(dict(type=name,slot=i,rowid=rowid,order=order,segment_rowid=sid,flags=None if sid is None else strategic|(tactical<<1)|(unready<<2),allow=True if sid is None else tactical!=0))
 require(expected==result['phases'],'independent rules phase projection differs');require(len(expected)==14,'phase coverage differs')
 inputs=load(a.input);players=result['players'];require(len(players)==43,'player coverage differs')
 for value,original in zip(players,inputs['players']):require(value['id']==original['owner'] and value['human']==(original['slot_status']==3),'world Player adapter differs')
 require(next(v for v in players if v['id']==header['owner'])['turn_active']==header['flags'][2],'saved flag2 bridge differs')
 report=dict(schema='c6-linux-unit-operation-manager-world-v1',checks_passed=True,gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,original_execution=False,
  methods=[dict(name=n,address=hex(r),size=s,sha256=h)for n,r,s,h in methods],checked_call_edges=len(CALLS),dwarf_sha256=sha(dwarf.encode()),
  source_commit=SOURCE_COMMIT,source_sha256=source_hashes,audit_sha256=sha(Path(__file__).read_bytes()),driver_sha256=sha(DRIVER.encode()),replay_binary_sha256=sha(binary.read_bytes()),
  rules_files={n:sha((a.rules/n).read_bytes())for n in ['configuration.sqlite','gameplay.sqlite']},registry_input_sha256=sha(a.registry.read_bytes()),placement_input_sha256=INPUT_SHA,canonical_header_core_sha256=CORE_SHA,
  units=1262,definition_slots=63,saved_disabled_pairs=saved_pairs,unknown_saved_keys=unknown,resulting_true_slots=disabled,all_units_unchanged=True,
  phase_slots_reconstructed=len(expected),phase_configuration_types=4,phase_tactical_false=sum(not v['allow']for v in expected),actual_configured_phase_captured=False,
  players=43,human_players=sum(v['human']for v in players),saved_turn_active_headers=1,
  independent_source_model_equal=True,native_live_disabled_compared=False,native_tactical_compared=False,composite_inputs_are_full_manager_profile=False,
  unresolved=['actual configured/current runtime phase and ordered runtime vector under same context','current live disabled vector after mutations','Game pause rawbyte and processingAutomation rawenum','all Player live-turnactive/header crosschecks','actual script registration/callback/args/engine-lock effects','current handler interruption and full requested-handler effects'],
  private_reconstruction_path=str(work/'reconstruction.json'),private_reconstruction_sha256=sha(raw),full_manager_native_acceptance=False,complete_state_actions_rng_turns=False,raw_save_load_difference_bytes=89198)
 a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps({k:report[k]for k in ['checks_passed','units','definition_slots','saved_disabled_pairs','resulting_true_slots','phase_slots_reconstructed','phase_tactical_false','players','human_players']}))
if __name__=='__main__':main()
