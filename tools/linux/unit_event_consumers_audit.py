#!/usr/bin/env python3
"""Pinned static Unit event consumer boundary and two-record typed property check."""
import argparse
import hashlib
import json
import re
from pathlib import Path
import struct
import subprocess
import tempfile
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
from skip_turn_state_success import compile_tool

ROOT=Path(__file__).resolve().parents[2]
PRIVATE=ROOT.parent/'c6-ghidra'
GAME=PRIVATE/'evidence/linux-rules/asl-launch-xp2-2/libGameCore_XP2.so'
HOST=ROOT.parent/'c6-assets-linux/Civ6'
DECODER=r'''
use c6_core::unit::{Unit,Limits};
use std::{env,fs};
fn main()->Result<(),Box<dyn std::error::Error>>{
 for p in env::args().skip(1){
  let b=fs::read(p)?;let(u,r)=Unit::decode_with_ranges(&b,Limits::default())?;
  assert_eq!(u.encode(Limits::default())?,b);
  let f=r.iter().find(|f|f.name=="m_kProperties").unwrap();
  assert_eq!(r.len(),73);assert!(u.k_properties.entries.is_empty());
  assert_eq!(&b[f.start..f.end],&[0u8;4]);
  println!("{{\"owner\":{},\"id\":{},\"fields\":{},\"properties_count\":{},\"properties_range\":[{},{}],\"roundtrip\":true}}",u.e_owner,u.id,r.len(),u.k_properties.entries.len(),f.start,f.end);
 }Ok(())
}
'''
GAME_METHODS=[('Unit.DynamicProperties',0x2421a00,8),('Unit.PropertyWrapperCtor',0x2c1ca10,69),
 ('Event.EntryCtor',0x27ffae0,61),('Event.Add',0x27ff2d0,222)]
HOST_METHODS=[('Map.Ctor',0x395f070,124),('Map.Copy',0x39612d0,1012),('Map.Clear',0x39611c0,261),
 ('Map.SetManager',0x395f300,173),('Container.SetManagerBase',0x3968b60,17),('Map.Serialize',0x3961e50,196)]

CONSUMER_METHODS = [('GameCore::Cache::Dispatcher::Receive(GameCore::Events::CacheEventData const&)', 41041472, 170),
 ('GameCore::Cache::Map::Instance::EditUnits()', 41698016, 8),
 ('GameCore::Cache::Unit::EventStream::Manager::Initialize()', 41936848, 76),
 ('GameCore::Cache::Unit::EventStream::Manager::Uninitialize()', 41939456, 12),
 ('GameCore::Cache::Unit::EventStream::Manager::PublishCreate(unsigned int, GameCore::Player::ComponentID '
  'const&, unsigned int)',
  41941216,
  174),
 ('GameCore::Cache::Unit::EventStream::Manager::PublishUpdate(unsigned int, GameCore::Player::ComponentID '
  'const&)',
  41941088,
  114),
 ('GameCore::Cache::Unit::EventStream::Manager::PublishDestroy(unsigned int, GameCore::Player::ComponentID '
  'const&)',
  41941392,
  94),
 ('GameCore::Cache::Unit::EventStream::Manager::RegisterEvents()', 41936928, 2522),
 ('GameCore::Cache::Unit::EventStream::Manager::OnCivicCompleted(unsigned int, GameCore::Events::EventData1 '
  'const*, Data::TypedVariantMap const*)',
  41940896,
  5),
 ('GameCore::Cache::Unit::EventStream::Manager::OnGoodyHutReward(unsigned int, GameCore::Events::EventData1 '
  'const*, Data::TypedVariantMap const*)',
  41940944,
  47),
 ('GameCore::Cache::Unit::EventStream::Manager::UnregisterEvents()', 41939472, 1),
 ('GameCore::Cache::Unit::EventStream::Manager::OnUnitCacheUpdate(GameCore::Events::CacheEventData const&)',
  41940032,
  18),
 ('GameCore::Cache::Unit::EventStream::Manager::OnUnitCreateEvent(unsigned int, GameCore::Events::EventData1 '
  'const*, Data::TypedVariantMap const*)',
  41940064,
  12),
 ('GameCore::Cache::Unit::EventStream::Manager::OnUnitUpdateEvent(unsigned int, GameCore::Events::EventData1 '
  'const*, Data::TypedVariantMap const*)',
  41940016,
  9),
 ('GameCore::Cache::Unit::EventStream::Manager::OnPlayerAgeChanged(unsigned int, '
  'GameCore::Events::EventData1 const*, Data::TypedVariantMap const*)',
  41940912,
  5),
 ('GameCore::Cache::Unit::EventStream::Manager::OnUnitDestroyEvent(unsigned int, '
  'GameCore::Events::EventData1 const*, Data::TypedVariantMap const*)',
  41940080,
  9),
 ('GameCore::Cache::Unit::EventStream::Manager::OnResearchCompleted(unsigned int, '
  'GameCore::Events::EventData1 const*, Data::TypedVariantMap const*)',
  41940880,
  5),
 ('GameCore::Cache::Unit::EventStream::Manager::OnUnitAbilityChanged(unsigned int, '
  'GameCore::Events::EventData1 const*, Data::TypedVariantMap const*)',
  41941040,
  47),
 ('GameCore::Cache::Unit::EventStream::Manager::OnBarbarianSpottedCity(unsigned int, '
  'GameCore::Events::EventData1 const*, Data::TypedVariantMap const*)',
  41940992,
  47),
 ('GameCore::Cache::Unit::EventStream::Manager::OnFormationUpdateEvent(unsigned int, '
  'GameCore::Events::EventData1 const*, Data::TypedVariantMap const*)',
  41940096,
  270),
 ('GameCore::Cache::Unit::EventStream::Manager::OnPlayerDarkAgeChanged(unsigned int, '
  'GameCore::Events::EventData1 const*, Data::TypedVariantMap const*)',
  41940928,
  5),
 ('GameCore::Cache::Unit::EventStream::Manager::OnPlayerUnitsUpdateEvent(unsigned int, '
  'GameCore::Events::EventData1 const*, Data::TypedVariantMap const*)',
  41940368,
  192),
 ('GameCore::Cache::Unit::EventStream::Manager::OnGovernmentPolicyChanged(unsigned int, '
  'GameCore::Events::EventData1 const*, Data::TypedVariantMap const*)',
  41940784,
  45),
 ('GameCore::Cache::Unit::EventStream::Manager::OnGovernmentPolicyObsoleted(unsigned int, '
  'GameCore::Events::EventData1 const*, Data::TypedVariantMap const*)',
  41940832,
  45),
 ('GameCore::Cache::Unit::EventStream::Manager::OnLocalPlayerUnitsUpdateEvent(unsigned int, '
  'GameCore::Events::EventData1 const*, Data::TypedVariantMap const*)',
  41940560,
  224),
 ('GameCore::Cache::Unit::EventStream::Manager::Add(GameCore::Cache::Unit::EventStream::Command, '
  'GameCore::Unit::Instance const*, unsigned int, bool)',
  41939664,
  222),
 ('GameCore::Cache::Unit::EventStream::Manager::Clear()', 41940000, 12),
 ('GameCore::Cache::Unit::EventStream::Manager::Destroy(unsigned int)', 41939984, 16),
 ('GameCore::Cache::Unit::EventStream::Manager::Process(unsigned int)', 41939888, 96),
 ('GameCore::Cache::Unit::EventStream::Manager::Manager()', 41941616, 25),
 ('GameCore::Cache::Unit::Instance::Update(GameCore::Cache::Unit::EventStream::Entry const&, bool)',
  41965248,
  83),
 ('GameCore::Cache::Player::Units::Update(GameCore::Cache::Unit::EventStream::Entry const&, bool)',
  41891072,
  562),
 ('GameCore::Exports::Context::UpdateCache(GameCore::Events::CacheEventData const&)', 46650320, 8),
 ('GameCore::Cache::Unit::EventStream::Manager::GetName() const', 41939488, 8),
 ('GameCore::Cache::Unit::EventStream::Manager::GetStatus(eastl::list<GameCore::Cache::System::EventStream::EntryStatus, '
  'eastl::allocator>&) const',
  41939504,
  149),
 ('GameCore::Cache::Dispatcher::Uninitialize()', 41038032, 277),
 ('GameCore::Cache::Dispatcher::LogStatus()', 41041728, 123),
 ('void '
  'GameCore::Cache::EventStreamPolicy::FreeNode<GameCore::Cache::KeyedBlockContainer<GameCore::Cache::BlockAllocatorEventStreamNodePolicy<GameCore::Cache::Unit::EventStream::Entry>, '
  '4992> '
  '>(GameCore::Cache::KeyedBlockContainer<GameCore::Cache::BlockAllocatorEventStreamNodePolicy<GameCore::Cache::Unit::EventStream::Entry>, '
  '4992>&, unsigned int)',
  41942032,
  77),
 ('GameCore::Cache::KeyedBlockContainer<GameCore::Cache::BlockAllocatorEventStreamNodePolicy<GameCore::Cache::Unit::EventStream::Entry>, '
  '4992>::clear()',
  41942112,
  132),
 ('GameCore::Cache::KeyedBlockContainer<GameCore::Cache::BlockAllocatorEventStreamNodePolicy<GameCore::Cache::Unit::EventStream::Entry>, '
  '4992>::KeyedBlockContainer()',
  41941488,
  92),
 ('GameCore::Cache::KeyedBlockContainer<GameCore::Cache::BlockAllocatorEventStreamNodePolicy<GameCore::Cache::Unit::EventStream::Entry>, '
  '4992>::operator[](unsigned int)',
  41942000,
  25),
 ('GameCore::Cache::Map::UnitContainer::UnitContainer()', 41697504, 5),
 ('GameCore::Cache::Map::Instance::Instance(GameCore::Cache::Game::Instance*)', 41692928, 299),
 ('GameCore::Cache::Game::Instance::EditPlayer(GameCore::PlayerTypes)', 41035600, 9),
 ('GameCore::Cache::Game::Instance::CreateMap()', 41302336, 114),
 ('GameCore::Cache::Player::Instance::EditUnits()', 41822448, 13),
 ('GameCore::Cache::Context::Instance::EditPlayer(GameCore::PlayerTypes)', 41034736, 24),
 ('GameCore::Cache::Context::Instance::EditMap()', 41034704, 20),
 ('GameCore::Cache::Context::Instance::EditGame()', 41034688, 5),
 ('GameCore::Exports::Unit::Unit(GameCore::Cache::Unit::Instance const*)', 46741216, 75),
 ('GameCore::Exports::Unit::Unit(GameCore::Unit::Instance*)', 46741136, 75),
 ('GameCore::Cache::KeyedBlockContainer<GameCore::Cache::BlockAllocatorEventStreamNodePolicy<GameCore::Cache::Unit::EventStream::Entry>, '
  '4992>::max_allocated() const',
  41941712,
  4),
 ('GameCore::Cache::System::EventStream::IComponentManager::CachingEventPlaybackComplete() const',
  41914400,
  417),
 ('GameCore::Cache::System::EventStream::IComponentManager::LogStatus() const', 41913952, 412),
 ('GameCore::Exports::Unit::GetLocation() const', 46741712, 48)]

def sha(data):return hashlib.sha256(data).hexdigest()
def require(ok,why):
 if not ok:raise ValueError(why)

def audit(module,host_path,private_root):
 game,host=Elf(module,GAME_SHA),Elf(host_path,HOST_SHA)
 for slot,target in [(0x18,0x395f300),(0x168,0x39611c0)]:
  require(struct.unpack('<Q',host.read(0x53b3680+16+slot,8))[0]==target,'map virtual differs')
 require(game.read(0x2421a00,8).hex()=='488d87f8010000c3','Unit property source differs')
 game.call(0x27ff313,0x1f16310);game.call(0x27ff327,0x1e73580)
 game.call(0x2c1ca35,0x1f9be20);game.call(0x27ffafd,0x1f9be20)
 for table,slot,target in [(0x3ab7cd0,0xc8,0x27c42e0),(0x3ab4bb8,0x90,0x27223d0),(0x3ab4bb8,0x98,0x27223f0),(0x3ab4bb8,0x88,0x27223c0)]:
  require(game.relocated_pointer(table+16+slot)==target,'actual cache consumer binding differs')
 for call,target in [(0x2c7d3d3,0x1f6b970),(0x27ff44c,0x1e99b80),(0x27ff3f5,0x1f92900),
                     (0x27ff403,0x1f9dd80),(0x28056ea,0x1e4f800),(0x280570e,0x1e73580)]:
  transfer=game.read(call,5)
  require(transfer[0] in (0xe8,0xe9) and call+5+struct.unpack_from('<i',transfer,1)[0]==target,
          'consumer call/tail dispatch differs')
 for slot,target in [(0x10,0x27f8e60),(0x18,0x27ff230),(0x20,0x27f9020)]:
  require(game.relocated_pointer(0x3ab8680+16+slot)==target,'Unit status binding differs')
 require(game.read(0x27ff440,7).hex()=='817f08181ed115','Unit cache stream type gate differs')
 require(game.read(0x27ff320,4).hex()=='31d231c9','property Copy arguments differ')
 require(host.read(0x3961330,8).hex()=='84c00f8426030000','empty source branch differs')
 require(host.read(0x3968b60,17).hex()=='4885f67507488d35ccd8bb0248897720c3','manager pointer tail differs')
 evidence_path=ROOT/'evidence/linux/skip-turn-state-execute-3.json'
 evidence=json.loads(evidence_path.read_text())
 require(evidence['native_quiescence_proven'] and evidence['native_cleanup_proven'],'closed original pair lacks quiescence')
 region=next(r for r in evidence['changed_owner_regions'] if r['owner']=='unit')
 subprocess.run(['cargo','build','-p','c6-core','--lib'],cwd=ROOT,check=True)
 target=Path(json.loads(subprocess.check_output(['cargo','metadata','--format-version=1','--no-deps'],cwd=ROOT))['target_directory'])/'debug'
 decoded=[]
 with tempfile.TemporaryDirectory(prefix='c6-unit-event-properties-') as tmp:
  directory=Path(tmp);binary=compile_tool(DECODER,'properties',directory,target)
  for side in ['before','after']:
   data=(private_root/f'evidence/linux-core-component/native-execute-3/{side}.bin').read_bytes()
   require(sha(data)==evidence['original_core_saves'][side]['sha256'],'closed full stream differs')
   lo,hi=region[side+'_range'];record=data[lo:hi]
   require(sha(record)==region[side+'_sha256'],'typed original Unit record differs')
   path=directory/(side+'.bin');path.write_bytes(record)
   row=json.loads(subprocess.check_output([str(binary),str(path)],text=True))
   require((row['owner'],row['id'])==(evidence['selected']['owner'],evidence['selected']['id']),'actual selected Unit differs')
   row.update(side=side,record_range=[lo,hi],record_sha256=sha(record),property_bytes='00000000')
   decoded.append(row)
 methods=[dict(image=image,name=n,rva=hex(a),bytes=z,sha256=sha(elf.read(a,z)))
          for image,elf,items in [('game',game,GAME_METHODS+CONSUMER_METHODS),('host',host,HOST_METHODS)] for n,a,z in items]
 # Full original text scan, retaining only bounded global-reference facts.
 references=[];current=None
 process=subprocess.Popen(['objdump','-d','-C',str(module)],stdout=subprocess.PIPE,text=True)
 for line in process.stdout:
  header=re.match(r'^([0-9a-f]+) <(.+)>:',line)
  if header:current=dict(rva='0x'+header[1],symbol=header[2])
  if re.search(r'# 0*3c03858\b',line):
   pc=re.match(r'\s*([0-9a-f]+):',line)
   references.append(dict(method=current['rva'],symbol=current['symbol'],instruction_rva='0x'+pc[1]))
 require(process.wait()==0,'target text scan failed')
 require({int(r['method'],16) for r in references}=={0x1fc2e40,0x27ff200,0x27ff230,0x27ff2d0,0x27ff3b0,0x27ff410,0x27ff420},
         'entry global reference ownership differs')
 return dict(schema='c6-linux-unit-event-consumers-v1',checks_passed=True,native_execution=False,
  gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,audit_source_sha256=sha(Path(__file__).read_bytes()),
  state_evidence_sha256=sha(evidence_path.read_bytes()),codec_adapter_sha256=sha(DECODER.encode()),
  codec_library_sha256=sha((target/'libc6_core.rlib').read_bytes()),
  typed_fixture_records=decoded,methods=methods,
  property_copy=dict(source='Actual Unit embedded Data::TypedVariantMap at+1f8; direct getter, no alternate provider',
   destination='Fresh event Entry property map at+200',exclude_map=None,append=False,
   empty_branch='Clear empty destination, source begin==end skips every per-entry variant/custom-array/CreateVariant branch; SetManager on empty destination only stores source manager pointer or default',
   empty_source_mutation=False,empty_rng_reporting_gameplay_callbacks=False,
   manager_copy='Destination pointer+20 only, not reference acquisition or manager callback',
   fixture_before_after_entry_counts=[0,0],per_callback_live_property_map_observed=False,
   nonempty='Custom array-copy and nested variant-manager branches exist; not covered by this empty-property profile'),
  direct_global_references=references,
  output_boundary=dict(classification='Rebuildable Unit presentation snapshot queue and reflected-cache delivery; may omit entries/node IDs/allocator in no-UI profile',
   basis='Exact global readers and concrete downstream bindings, not Cache namespace',
   delivery=['Exports.Context.UpdateCache2c7d3d0','Dispatcher.Receive2723e40','Unit.OnUnitCacheUpdate27ff440','Unit.Process27ff3b0'],
   process='Validate key, resolve copied owner in Cache.Context, update reflected Unit collection, destroy node; entry pointer not returned',
   command2='Copy constants/variants/properties to reflected Unit; if location changes, remove/add in separately-owned cached Map UnitContainer',
   separate_storage='Cache.Game.CreateMap allocates new0x1e0 Map; Cache.Map ctor builds its own UnitContainer+b0; virtual+c8 returns that container',
   status='Entry type/count/capacity/entry size/peak allocation scalar statistics to LogStatus; no authority-facing node pointer',
   key_usage='Transient queue lookup/destruction only; copied reflected Unit IDs come from entry constants, not node keys',
   serialization='No serializer among complete direct global references; entry has transient copy/consume/destroy lifecycle, not authoritative Save ownership',
   authoritative_gameplay_consumer_found=False,arbitrary_interface_dispatch_universally_excluded=False,
   exact_ui_heap_or_node_id_parity_required_for_headless=False,
   source_property_state_must_remain_exact=True,
   omission_scope='Only Unit event entries/keys/allocator and their reflected output, not all reporting, source Unit properties or other cache subscribers',
   preserve='Any authoritative side effects of snapshot queries; empty property copy has none; HoldCycling fallback outside observed branch remains unclosed'),
  gameplay_source_properties_excluded=False,complete_action_acceptance=False,complete_rng_inventory=False,
  historical_raw_save_load_failure_bytes=89198,masked_bytes=0)

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--module',type=Path,default=GAME)
 p.add_argument('--host',type=Path,default=HOST);p.add_argument('--private-root',type=Path,default=PRIVATE)
 p.add_argument('--output',type=Path,required=True);a=p.parse_args();r=audit(a.module,a.host,a.private_root)
 a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(r,indent=2)+'\n')
 print(json.dumps(dict(checks_passed=True,typed_unit_records=len(r['typed_fixture_records']))))
if __name__=='__main__':main()
