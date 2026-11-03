#!/usr/bin/env python3
"""Static source gates and retained-source Rust checks for owned TradeGraph paths.

Never executes original code; the actual frame is serializer-attributed input,
not a captured native query answer or a complete live Trade manager.
"""
import argparse
import gzip
import hashlib
import json
import io
import os
from pathlib import Path
import subprocess
import tarfile
from definition_ids_audit import ROOT, TARGET, HOST, elf
from load_archive_postlude_audit import methods
CORE='ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884'
GRAPH='64a196d77b6bc35a72354226dda4111e10f46742cd62d4e335b43c737a186c33'
TRACE='528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae'
METHODS=dict(graph_read=0x2bf8210,graph_get_path=0x2bfbc50,graph_city_path=0x2bfbbe0,
 graph_get_edge=0x2bfc240,manager_existing_path=0x2c08eb0,manager_find_new_path=0x2c06210,
 city_vertex=0x2bf85f0,vertex_hash=0x2bf8540,vertex_equal=0x2bfe340,
 inner_find=0x2bfd5e0,inner_find_base=0x2c03a30,inner_find_equality=0x2c03ae0,
 outer_find=0x2bfd940,outer_find_node=0x2bfe170,
 component_player=0x1fd0e30,component_id=0x1fd1080)
HARNESS=r'''
#[path="trade.rs"] pub mod trade;
#[path="trade_read_order.rs"] pub mod trade_read_order;
#[path="trade_graph_state.rs"] pub mod trade_graph_state;
fn main(){
 let bytes=std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
 let state=trade_graph_state::GraphState::decode(&bytes,trade::Limits::default()).unwrap();
 let mut edges=0;let mut nonempty=0;let mut active=0;let mut active_hits=0;let mut active_nodes=0;
 let mut reverse_hits=0;
 for origin in state.graph().origins(){for d in &origin.destinations{
  edges+=1;
  let actual=state.get_path(origin.vertex,d.vertex);
  if d.edge.most_recent_path.is_empty(){assert!(actual.is_none())}else{
   nonempty+=1;assert_eq!(actual,Some(d.edge.most_recent_path.as_slice()));
  }
  if d.edge.has_active_route{
   active+=1;let route=&d.edge.most_recent_route;
   if let Some(path)=state.city_path(route.origin_city,route.destination_city){
    active_hits+=1;active_nodes+=path.len();
   }
   if let Some(reverse)=state.reverse_city_path(route.destination_city,route.origin_city){
    reverse_hits+=1;
    let expected=state.city_path(route.origin_city,route.destination_city).unwrap();
    assert_eq!(reverse.len(),expected.len());assert!(!reverse.is_empty());
    assert!(reverse.iter().eq(expected.iter().rev().copied()));
   }
  }
 }}
 assert_eq!(edges,1571);assert_eq!(state.graph().origins().len(),179);
 assert_eq!(active,182);assert_eq!(active_hits,182);assert_eq!(active_nodes,6193);
 println!("{{\"origins\":179,\"edges\":{edges},\"nonempty_paths\":{nonempty},\"active_routes\":{active},\"active_route_city_path_hits\":{active_hits},\"active_path_nodes\":{active_nodes},\"reverse_projection_checks\":{reverse_hits}}}");
}
'''
def sha(data):return hashlib.sha256(data).hexdigest()
def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--directory',type=Path,required=True)
 p.add_argument('--output',type=Path,required=True)
 p.add_argument('--source-commit',help='archive and test this exact complete Core source tree too')
 p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
 private=ROOT.parent/'c6-ghidra/evidence'
 p.add_argument('--core',type=Path,default=private/'linux-reporting-provider/native-passive-2/before.bin')
 p.add_argument('--trace',type=Path,default=private/'linux-save-normalization/checkpoint-203-owners/trace-benchmark-1.jsonl.gz')
 a=p.parse_args();a.directory=a.directory.resolve();a.directory.mkdir(parents=True,exist_ok=False)
 source_repo=ROOT;full_archive=None;commit=None
 if a.source_commit:
  commit=subprocess.check_output(['git','rev-parse',a.source_commit+'^{commit}'],cwd=ROOT,text=True).strip()
  full_archive=subprocess.check_output(['git','archive','--format=tar',commit],cwd=ROOT)
  (a.directory/'committed-source.tar').write_bytes(full_archive)
  source_repo=a.directory/'committed-source';source_repo.mkdir()
  with tarfile.open(fileobj=io.BytesIO(full_archive)) as tar:tar.extractall(source_repo,filter='data')
 native,asm,read=methods(a.assets/'libGameCore_XP2.so',TARGET,METHODS,a.directory/'native-source')
 elf(a.assets/'Civ6',HOST)
 gates={0x2c03a6c:'83f8ff',0x2c03a80:'4439f8',0x2c03a94:'e8a76d36ff',
        0x1fd0e30:'0fbf07c3',0x1fd1080:'8b4704c3'}
 for at,expected in gates.items():
  value=bytes.fromhex(expected)
  if read(at,len(value))!=value:raise ValueError('source predicate/field layout differs')
 if '0x188' not in asm['graph_get_path'] or 'Vertex::operator==' not in asm['inner_find_equality']:
  raise ValueError('path/equality binding differs')
 core=a.core.read_bytes();trace=gzip.decompress(a.trace.read_bytes())
 if len(core)!=80657462 or sha(core)!=CORE or sha(trace)!=TRACE:raise ValueError('actual source identity differs')
 frames=[]
 for line in trace.splitlines():
  row=json.loads(line)
  if row.get('event')=='owner' and row.get('save')==1 and row.get('owner')=='GameCore::Trade::Graph':frames.append(row)
 expected=dict(event='owner',save=1,depth=2,call=162111,parent=162110,owner='GameCore::Trade::Graph',start=71027800,end=71497438)
 if frames!=[expected]:raise ValueError('Graph owner attribution differs')
 frame=core[expected['start']:expected['end']]
 if sha(frame)!=GRAPH:raise ValueError('Graph frame differs')
 (a.directory/'graph.bin').write_bytes(frame)
 source=a.directory/'source';source.mkdir()
 inputs=[source_repo/'crates/c6-core/src'/name for name in ['trade.rs','trade_read_order.rs','trade_graph_state.rs']]
 payloads={path.name:path.read_bytes() for path in inputs}
 payloads['harness.rs']=HARNESS.encode();payloads['check.py']=Path(__file__).read_bytes()
 if any(path.read_bytes()!=payloads[path.name] for path in inputs):raise ValueError('source changed during copy')
 for name,data in payloads.items():(source/name).write_bytes(data)
 archive=a.directory/'source.tar'
 with tarfile.open(archive,'w') as tar:
  for name in sorted(payloads):tar.add(source/name,arcname=name)
 binary=a.directory/'check';tests=a.directory/'tests'
 for output,extra in [(binary,[]),(tests,['--test'])]:
  subprocess.run(['rustc','--edition=2021','-Dwarnings',*extra,str(source/'harness.rs'),'-o',str(output)],check=True)
 unit=subprocess.run([str(tests),'trade_graph_state::tests','--nocapture'],capture_output=True,text=True)
 (a.directory/'test.stdout').write_text(unit.stdout);(a.directory/'test.stderr').write_text(unit.stderr)
 if unit.returncode:raise RuntimeError(unit.stdout+unit.stderr)
 run=subprocess.run([str(binary),str(a.directory/'graph.bin')],capture_output=True,text=True)
 if run.returncode:raise RuntimeError(run.stderr)
 result=json.loads(run.stdout)
 for name,data in payloads.items():
  if (source/name).read_bytes()!=data:raise ValueError('retained source changed')
  (source/name).chmod(0o444)
 bridge={}
 if full_archive is not None:
  build=subprocess.run(['cargo','test','--locked','--offline','-p','c6-core','--lib','--no-run','--message-format=json'],
      cwd=source_repo,env=dict(os.environ,CARGO_TARGET_DIR=str(ROOT/'target')),capture_output=True,text=True)
  (a.directory/'core-build.stdout.jsonl').write_text(build.stdout);(a.directory/'core-build.stderr').write_text(build.stderr)
  if build.returncode:raise RuntimeError(build.stderr)
  binaries=[r['executable']for line in build.stdout.splitlines() if (r:=json.loads(line)).get('reason')=='compiler-artifact' and r.get('executable')]
  if len(binaries)!=1:raise ValueError('ambiguous Core test executable')
  core_test=a.directory/'core-tests';core_test.write_bytes(Path(binaries[0]).read_bytes());core_test.chmod(0o555)
  checked=subprocess.run([str(core_test),'trade_graph','--nocapture'],capture_output=True,text=True)
  (a.directory/'core-test.stdout').write_text(checked.stdout);(a.directory/'core-test.stderr').write_text(checked.stderr)
  if checked.returncode:raise RuntimeError(checked.stdout+checked.stderr)
  preimages=0
  with tarfile.open(fileobj=io.BytesIO(full_archive)) as tar:
   for member in tar.getmembers():
    if member.isfile():
     path=source_repo/member.name
     if path.read_bytes()!=tar.extractfile(member).read():raise ValueError('committed source changed: '+member.name)
     path.chmod(0o444);preimages+=1
  bridge=dict(loaded_state_bridge_checked=True,committed_source_archive_sha256=sha(full_archive),
      committed_source_preimages_checked=preimages,core_test_candidate_sha256=sha(core_test.read_bytes()),core_test_output=checked.stdout,
      cargo_lock_sha256=sha((source_repo/'Cargo.lock').read_bytes()))
 revision=commit or subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
 paths=[str(path.relative_to(source_repo)) for path in inputs]
 changes=[] if commit else subprocess.check_output(['git','status','--short','--',*paths],cwd=ROOT,text=True).splitlines()
 driver_status=subprocess.check_output(['git','status','--short','--','tools/linux/trade_graph_state_check.py'],cwd=ROOT,text=True).splitlines()
 result.update(schema='c6-linux-trade-graph-state-v1',gamecore_sha256=TARGET,host_sha256=HOST,
  source_commit=revision,uncommitted_source_paths=changes,sources={name:sha(data)for name,data in payloads.items()},
  complete_source_preimages_retained=True,source_archive_sha256=sha(archive.read_bytes()),candidate_directory=str(a.directory),
  candidate_sha256=sha(binary.read_bytes()),test_candidate_sha256=sha(tests.read_bytes()),test_output=unit.stdout,
  rustc=subprocess.check_output(['rustc','-Vv'],text=True),methods=native,instruction_gates=len(gates),
  source_core_sha256=CORE,source_frame_sha256=GRAPH,source_trace_sha256=TRACE,frame=expected,
  original_execution=False,direct_native_query_comparison=False,
  source_fresh_graph_read_logical_index_authority=True,live_native_index_observation=False,
  loaded_state_bridge_checked=False,driver_source_status=driver_status,native_id_map_iteration_order=False,cold_pathfinder_supported=False,
  full_state_action_turn_parity=False)
 result.update(bridge)
 a.output.write_text(json.dumps(result,sort_keys=True,indent=2)+'\n')
 archive.chmod(0o444)
 print(json.dumps({k:v for k,v in result.items() if k not in ['methods','sources','test_output','core_test_output']},sort_keys=True))
if __name__=='__main__':main()
