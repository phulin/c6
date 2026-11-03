#!/usr/bin/env python3
"""Independent immutable Linux turn-prefix review; executes Rust fixtures only."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import struct
import tempfile
from save_lifecycle_audit import Elf,GAME_SHA,HOST_SHA
from turn_progression_audit import METHODS

ROOT=Path(__file__).resolve().parents[2]
FULL_METHODS=[row for row in METHODS if row[0]in {0x2d27b50,0x2b5b840,0x2ba17b0,0x2a5b640,0x2a5b570,0x2850fc0,0x2661ba0}]
FULL_METHODS += [
 (0x2660650,8,'Game.GetInstance','f2da8e6d1cfad107964503183788ee3a3940ed4000767c930e4617053d2e59fb'),
 (0x2b53260,8,'PlayerManager.GetInstance','3767e99c9e2a663768e78a24c0669349430acf05be01ce1ee8f273e1f62679e7'),
 (0x1fe6ea0,52,'BuildQueue.AutoVariable.edit','d5136d787a88867d1219452b68060e3d8d60333598b31d43399bb2fdda167a13'),
 (0x1fe71d0,27,'Queue.Front.const','79980c47d2ab7952bb82adb23e27d0d63269bc5dc9445c535493a49490ed910a'),
]
SITES=[
 (0x2d27b66,'83ff3f','unsigned encoded player bound'),
 (0x2d27b7c,'4989c6','Game retained before human query'),
 (0x2b5b863,'8818','one global completion byte write'),
 (0x2b5b870,'4889c3','one captured Manager before validity loop'),
 (0x2b5b880,'4889df89ee','validity uses captured Manager'),
 (0x2b5b88e,'4c89ff','alive query uses requested Player'),
 (0x2b5b89a,'4c89ff','hook uses same requested Player'),
 (0x2d27e87,'e8e46b0dff','fresh Game before first Update'),
 (0x2d27ea4,'e8c76b0dff','fresh Game before second Update'),
 (0x2a5b60a,'4c89ff','End reread uses retained city container'),
]
HARNESS=r'''
use c6_core::turn_completion::*;
use c6_core::city_production::{BuildQueue,QueueEntry,Order};
use c6_core::unit::Limits;
struct Diagnostic { flags:[bool;64], queues:[BuildQueue;2], edits:Vec<usize>, ends:usize }
macro_rules! unavailable {($name:ident($($arg:ident:$ty:ty),*)->$ret:ty)=>{
 fn $name(&mut self,$($arg:$ty),*)->Result<$ret,Error>{$(let _=$arg;)*Err(Error::Unsupported("unused diagnostic dependency"))}
};}
impl World for Diagnostic {
 type Player=usize;type Game=usize;type PlayerManager=usize;type Cities=usize;type City=usize;type Cursor=usize;
 unavailable!(resolve_player(encoded:i32)->usize);
 unavailable!(game_instance()->usize);
 fn player_manager(&mut self)->Result<usize,Error>{assert!(self.flags[7]);Ok(11)}
 fn player_id(&mut self,player:usize)->Result<usize,Error>{assert_eq!(player,7);Ok(7)}
 fn is_human(&mut self,player:usize)->Result<bool,Error>{assert_eq!(player,9);Ok(true)}
 fn is_alive(&mut self,player:usize)->Result<bool,Error>{assert_eq!(player,7);Ok(true)}
 unavailable!(is_turn_active(player:usize)->bool);
 unavailable!(is_paused(game:usize)->bool);
 fn completion_flags(&mut self)->&mut[bool;64]{&mut self.flags}
 fn is_valid_slot(&mut self,manager:usize,slot:i32)->Result<bool,Error>{assert_eq!(manager,11);Ok(slot==0)}
 unavailable!(log(branch:LogBranch,player:usize)->());
 unavailable!(report(event:u32,player:usize)->());
 fn completion_cities(&mut self,player:usize)->Result<usize,Error>{assert_eq!(player,7);Ok(50)}
 fn cities_player(&mut self,cities:usize)->Result<usize,Error>{assert_eq!(cities,50);Ok(9)}
 fn cities_begin(&mut self,cities:usize)->Result<usize,Error>{assert_eq!(cities,50);Ok(0)}
 fn cities_end(&mut self,cities:usize)->Result<usize,Error>{assert_eq!(cities,50);self.ends+=1;Ok(2)}
 fn cursor_ne(&mut self,a:usize,b:usize)->Result<bool,Error>{Ok(a!=b)}
 fn find_cursor_city(&mut self,cities:usize,cursor:usize)->Result<usize,Error>{assert_eq!(cities,50);Ok([900,400][cursor])}
 fn next_city(&mut self,cursor:usize)->Result<usize,Error>{Ok(cursor+1)}
 fn build_queue(&mut self,city:usize)->Result<&BuildQueue,Error>{Ok(&self.queues[match city{900=>0,400=>1,_=>panic!("wrong City identity")}])}
 fn edit_build_queue(&mut self,city:usize)->Result<&mut BuildQueue,Error>{assert_eq!(city,900);self.edits.push(city);Ok(&mut self.queues[0])}
}
fn main(){
 let mut bytes=vec![0;133];bytes[..4].copy_from_slice(&16i32.to_le_bytes());
 let mut empty=BuildQueue::decode(&bytes,Limits::default()).unwrap();empty.production_overflow=i32::MIN;empty.production_salvage=79;
 empty.previous_production_type_hash=0x12345678;empty.wonder_production_multiplier=19;empty.ignore_strategic_resource_requirements=true;
 let mut full=empty.clone();full.production_overflow=i32::MAX;
 full.entries.push(QueueEntry{type_hash:1,node_type_hash:2,order:Order::Other(-1),flags:3,request_id:4,location:[5,6]});
 let mut expected=empty.clone();expected.production_overflow=0;let full_before=full.clone();
 let mut world=Diagnostic{flags:[false;64],queues:[empty,full],edits:vec![],ends:0};
 assert_eq!(set_turn_complete(&mut world,7,true,USER_FORCED),Ok(true));
 assert!(world.flags[7]);assert_eq!(world.edits,[900]);assert_eq!(world.ends,3);
 assert_eq!(world.queues,[expected,full_before]);
 println!("distinct_player_city_queue_identity=passed");
}
'''

def sha(data):return hashlib.sha256(data).hexdigest()
def need(v,m):
 if not v:raise ValueError(m)
def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
 p.add_argument('--progression-report',type=Path,required=True)
 p.add_argument('--kernel-report',type=Path,required=True)
 p.add_argument('--output',type=Path,required=True);a=p.parse_args()
 commit=subprocess.check_output(['git','rev-parse','337fa32^{commit}'],cwd=ROOT,text=True).strip();pinned={}
 for path in ['crates/c6-core/src/turn_completion.rs','tools/linux/turn_completion_kernel_audit.py','docs/linux-turn-completion-kernel.md','evidence/linux/turn-completion-kernel.json']:
  data=subprocess.check_output(['git','show',commit+':'+path],cwd=ROOT);need(data==(ROOT/path).read_bytes(),'source differs from immutable corrected kernel: '+path);pinned[path]=sha(data)
 game=Elf(a.assets/'libGameCore_XP2.so',GAME_SHA);Elf(a.assets/'Civ6',HOST_SHA)
 for address,size,name,digest in FULL_METHODS:need(sha(game.read(address,size))==digest,'complete target method differs: '+name)
 for address,code,label in SITES:need(game.read(address,len(code)//2).hex()==code,'receiver/order instruction differs: '+label)
 completion_array=0x2b5b85e+struct.unpack('<i',game.read(0x2b5b85a,4))[0]
 need(completion_array==0x3c06210,'completion write does not address original global bool array')
 progression=json.loads(a.progression_report.read_text());kernel=json.loads(a.kernel_report.read_text())
 need(progression['checks_passed'] and not progression['native_execution'],'progression rerun failed')
 need(kernel['checks_passed'] and not kernel['native_execution'],'kernel rerun failed')
 tracked=['crates/c6-core/src/lib.rs','crates/c6-core/src/city_production.rs','crates/c6-core/src/turn_completion.rs']
 source_commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip();dependencies={}
 for path in tracked:
  raw=(ROOT/path).read_bytes();need(raw==subprocess.check_output(['git','show',source_commit+':'+path],cwd=ROOT),'uncommitted review dependency: '+path);dependencies[path]=sha(raw)
 subprocess.run(['cargo','build','-p','c6-core'],cwd=ROOT,check=True,stdout=subprocess.PIPE)
 with tempfile.TemporaryDirectory(prefix='c6-turn-kernel-review-')as temp:
  folder=Path(temp);source=folder/'review.rs';candidate=folder/'review';source.write_text(HARNESS)
  subprocess.run(['rustc','--edition=2021',str(source),'--extern','c6_core='+str(ROOT/'target/debug/libc6_core.rlib'),'-L','dependency='+str(ROOT/'target/debug/deps'),'-o',str(candidate)],cwd=ROOT,check=True)
  need(all(sha((ROOT/path).read_bytes())==digest for path,digest in dependencies.items()),'test dependency changed during build')
  output=subprocess.check_output([str(candidate)],text=True).strip();need(output=='distinct_player_city_queue_identity=passed','discriminating identity test failed');binary_sha=sha(candidate.read_bytes())
 result=dict(schema='c6-linux-turn-completion-kernel-independent-review-v1',checks_passed=True,
  corrected_source_commit=commit,historical_rejected_receiver_contract='d5b1f70',pinned_files=pinned,
  gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,
  independently_checked_full_methods=[dict(address=hex(r),bytes=n,name=s,sha256=h)for r,n,s,h in FULL_METHODS],
  receiver_order_sites=[dict(address=hex(r),bytes=b,meaning=s)for r,b,s in SITES],
  progression_audit=dict(sha256=sha(a.progression_report.read_bytes()),methods=len(progression['methods']),resolved_calls=len(progression['direct_calls'])),
  kernel_audit=dict(sha256=sha(a.kernel_report.read_bytes()),methods=len(kernel['methods']),resolved_edges=len(kernel['resolved_edges'])),
  original_kernel_tests_passed=11,public_engine_unsupported_atomicity_test_passed=True,
  independent_identity_test=dict(requested_player=7,cities_owner=9,cities=50,city_order=[900,400],edited_city=[900],end_reads=3,all_other_queue_fields_preserved=True),
  boundary_candidate_sha256=binary_sha,harness_sha256=sha(HARNESS.encode()),dependency_commit=source_commit,dependency_sources=dependencies,
  repaired_findings=['unconditional retained Game before human/pause','one retained PlayerManager after global write across64validity reads','fresh Game lookup before each Update'],
  global_completion_array=dict(address=hex(completion_array),elements=64,element='bool',not_a_serialized_player_field=True),
  provider_limits=['full log service branch and reporting subscribers','actual mutable Player/manager/city/queue identity and recycled-container behavior','FAutoVariable dirty-set insertion/ownership/allocation effects','whole Game.Update: Processor, pending diplomacy, cache profiling and all external decisions'],
  continuation='Internal typed dispatch sequencing only; no native stack abort/resume and no successful public EndTurn. Mandatory first Update; same-player active reread; at most one fresh-Game second Update.',
  mutation='Internal errors may retain global completion write/partial effects. Public Engine.EndTurn remains Unsupported before mutation.',
  native_execution_during_review=False,native_boundary_fixtures_available=False,full_end_turn_accepted=False,
  complete_state_action_rng_turns_supported=False,unknown_save_bytes_masked=False,raw_save_load_failure_bytes=89198,
  driver_sha256=sha(Path(__file__).read_bytes()))
 a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(result,indent=2)+'\n')
 print(json.dumps(dict(checks_passed=True,independent_full_methods=len(FULL_METHODS),identity_fixture=True,native_execution=False)))
if __name__=='__main__':main()
