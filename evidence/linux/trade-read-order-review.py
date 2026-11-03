#!/usr/bin/env python3
"""Independent source-order review and immutable pre-answer Trade prediction checks."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import struct
import subprocess
import sys
import tarfile
import tempfile
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools/linux'))
from trade_read_order_audit import Elf,MODULE_METHODS,HOST_METHODS,TARGET,HOST
HARNESS=r'''
use c6_core::trade::{self,ComponentId,Destination,Edge,Graph,Origin,Route,Vertex,YieldValue,Limits};
use c6_core::trade_read_order::{fresh_read_wire_order,vertex_hash,Error};
fn vertex(kind:i16,data1:i32,data2:i32)->Vertex{Vertex{kind,data1,data2}}
fn destination(id:i32,from:Vertex,to:Vertex)->Destination{
 let component=ComponentId{owner:-1,component_type:38,id:u32::MAX};
 Destination{vertex:to,edge:Edge{id,most_recent_route:Route{id:-7,start_turn:i32::MIN,trader_unit:component,resource_type_hash:99,resource_quantity:-3,origin_city:component,destination_city:component,origin_yields:vec![YieldValue{type_hash:3,raw:i32::MIN},YieldValue{type_hash:5,raw:i32::MAX}],destination_yields:vec![]},has_active_route:true,routes_started:-2,routes_completed:i32::MAX,most_recent_path:vec![i32::MIN,-1,0,i32::MAX],path_traversal_index:-9,path_traversal_going_forward:false,from_vertex:from,to_vertex:to}}
}
fn inner(vertices:Vec<Vertex>)->Graph{let from=vertex(2,3,4);Graph::new(-17,vec![Origin{vertex:from,destinations:vertices.into_iter().enumerate().map(|(i,to)|destination(i as i32,from,to)).collect()}]).unwrap()}
fn ids(graph:&Graph)->Vec<i32>{graph.origins()[0].destinations.iter().map(|d|d.edge.id).collect()}
fn check_inner(values:&[i32],expected:&[i32]){
 let graph=inner(values.iter().map(|&v|vertex(0,0,v)).collect());let before=graph.encode(Limits::default()).unwrap();
 let result=fresh_read_wire_order(&graph).unwrap();assert_eq!(ids(&result),expected);assert!(result.logical_eq(&graph));assert_eq!(result.total_edges_created(),-17);assert_eq!(graph.encode(Limits::default()).unwrap(),before);
}
fn main(){
 for (v,h) in [(vertex(0,0,0),0x9be17165),(vertex(0,0,-1),0x6cae0a5d),(vertex(i16::MIN,i32::MIN,i32::MAX),0x586444c1),(vertex(1,3,5),0x937cd88a),(vertex(-1,-1,i32::MIN),0x4b3ee881)]{assert_eq!(vertex_hash(v),h);}
 // Distinct keys sharing home0 retain the earlier resident on equal distance.
 check_inner(&[1,4],&[0,1]);
 // Incoming home0 displaces a home1 resident after the first probe.
 check_inner(&[1,2,4],&[0,2,1]);
 // Home3 wraps, displaces home0, and still uses capacity4 for the third insert.
 check_inner(&[0,1,7],&[2,1,0]);
 // Before insert4, ascending old slots [2,1,0] reinsert into capacity8.
 check_inner(&[0,1,7,4],&[3,1,2,0]);
 for (count,expected) in [(12,vec![11,0,1,2,3,4,5,6,7,8,9,10]),(13,vec![11,0,1,2,3,4,5,6,7,8,9,10,12])]{
  let graph=inner((0..count).map(|i|vertex(0,i,-1)).collect());let result=fresh_read_wire_order(&graph).unwrap();assert_eq!(ids(&result),expected);assert!(result.logical_eq(&graph));
 }
 for count in [53,54,110]{
  let graph=Graph::new(i32::MAX,(0..count).map(|i|Origin{vertex:vertex(0,i,-1),destinations:vec![]}).collect()).unwrap();
  let result=fresh_read_wire_order(&graph).unwrap();let actual:Vec<i32>=result.origins().iter().map(|o|o.vertex.data1).collect();
  let expected:Vec<i32>=match count{53=>(0..53).rev().collect(),54=>std::iter::once(53).chain(0..53).collect(),110=>std::iter::once(109).chain((0..53).rev()).chain(53..109).collect(),_=>unreachable!()};
  assert_eq!(actual,expected);assert_eq!(result.total_edges_created(),i32::MAX);assert!(result.logical_eq(&graph));
 }
 let sentinel=vertex(0,3,1621429197);assert_eq!(vertex_hash(sentinel),u32::MAX);
 let graph=inner(vec![sentinel]);let before=graph.encode(Limits::default()).unwrap();assert_eq!(fresh_read_wire_order(&graph).unwrap_err(),Error::UnsupportedSentinelHash(sentinel));assert_eq!(graph.encode(Limits::default()).unwrap(),before);
 let origin_only=Graph::new(-1,vec![Origin{vertex:sentinel,destinations:vec![]}]).unwrap();assert!(fresh_read_wire_order(&origin_only).unwrap().wire_state_eq(&origin_only));
 let v=vertex(0,0,0);assert!(matches!(Graph::new(0,vec![Origin{vertex:v,destinations:vec![]},Origin{vertex:v,destinations:vec![]}]),Err(trade::Error::DuplicateOrigin)));
 println!("independent_discriminating_case_groups=8");
}

'''

def sha(b):return hashlib.sha256(b).hexdigest()
def need(v,m):
 if not v:raise ValueError(m)
def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--frozen-manifest',type=Path,required=True)
 p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
 p.add_argument('--output',type=Path,required=True);a=p.parse_args()
 manifest_raw=a.frozen_manifest.read_bytes();need(sha(manifest_raw)=='2957dce152ff0f4c284ead3f04b930c8fd298cd0ef10c20a7c86d16740d3ddf5','pre-answer manifest changed')
 frozen=json.loads(manifest_raw);work=a.frozen_manifest.parent
 need(frozen['native_c_answer_available_at_freeze']is False,'prediction was not pre-answer')
 commit=subprocess.check_output(['git','rev-parse','8ba7938^{commit}'],cwd=ROOT,text=True).strip();pinned={}
 for path in ['crates/c6-core/src/trade_read_order.rs','crates/c6-core/examples/trade_read_order.rs']:
  raw=subprocess.check_output(['git','show',commit+':'+path],cwd=ROOT)
  need(raw==(work/'source'/path).read_bytes(),'frozen algorithm differs from immutable source: '+path);pinned[path]=sha(raw)
 archive=(work/'source.tar').read_bytes();need(sha(archive)==frozen['source_archive_sha256'],'frozen archive changed')
 with tarfile.open(fileobj=io.BytesIO(archive))as tar:
  for member in tar.getmembers():
   if member.isfile():need((work/'source'/member.name).read_bytes()==tar.extractfile(member).read(),'archived source changed: '+member.name)
 for path,digest in frozen['source_files'].items():need(sha((work/'source'/path).read_bytes())==digest,'frozen source dependency changed')
 need(sha((work/'candidate').read_bytes())==frozen['candidate_sha256'],'frozen prediction executable changed')
 need(sha((work/'candidate.rs').read_bytes())==frozen['candidate_source_sha256'],'frozen prediction program changed')
 for row in frozen['inputs']:
  data=(work/(row['label']+'.graph')).read_bytes();need(sha(data)==row['graph_sha256']and len(data)==row['graph_bytes'],'frozen full input owner changed')
 for row in frozen['results']:
  data=(work/row['output']).read_bytes();need(sha(data)==row['sha256']and len(data)==row['bytes'],'frozen full output owner changed')
 a_graph=(work/'a.graph').read_bytes();b_graph=(work/'b.graph').read_bytes();c_graph=(work/'b-predicted-c.graph').read_bytes()
 need((work/'a-reconstructed.graph').read_bytes()==b_graph,'full A-to-B byte comparison differs')
 need(len(a_graph)==len(b_graph)==len(c_graph)==469638,'complete Graph extent differs')
 need(sum(x!=y for x,y in zip(c_graph,b_graph))==65510,'pre-answer B-to-C byte prediction differs')
 module=Elf(a.assets/'libGameCore_XP2.so',TARGET);host=Elf(a.assets/'Civ6',HOST)
 need(struct.unpack_from('<H',host.blob,16)[0]==2,'host is not ET_EXEC absolute VA')
 methods=module.methods(MODULE_METHODS);host_methods=host.methods(HOST_METHODS)
 extras=module.methods({'calculate_load_factor':0x2bfed30,'vertex_equal':0x2bfe340})
 imports=subprocess.check_output(['readelf','-rW',str(a.assets/'libGameCore_XP2.so')],text=True)
 exports=subprocess.check_output(['nm','-D','-S',str(a.assets/'Civ6')],text=True)
 bindings=[]
 for got,plt,site,name,va,size in [(0x3b26278,0x1e06720,0x2c03914,'_ZNK5eastl19prime_rehash_policy18GetNextBucketCountEj',0x3974b36,110),(0x3b2a7c8,0x1e0f1c0,0x2bfe0a2,'_ZNK5eastl19prime_rehash_policy17GetRehashRequiredEjjj',0x3974c2a,233)]:
  rows=[line for line in imports.splitlines()if name in line];need(len(rows)==1 and int(rows[0].split()[0],16)==got and 'R_X86_64_JUMP_SLOT'in rows[0],'static module import identity differs')
  rows=[line for line in exports.splitlines()if line.endswith(name)];need(len(rows)==1 and int(rows[0].split()[0],16)==va and int(rows[0].split()[1],16)==size,'original absolute host export differs')
  raw=module.read(plt,6);need(raw[:2]==b'\xff\x25'and plt+6+struct.unpack_from('<i',raw,2)[0]==got,'original PLT slot differs')
  raw=module.read(site,5);need(raw[0]==0xe8 and site+5+struct.unpack_from('<i',raw,1)[0]==plt,'original call/import edge differs')
  bindings.append(dict(symbol=name,module_got_rva=hex(got),module_plt_rva=hex(plt),module_call_rva=hex(site),host_absolute_va=hex(va),host_bytes=size,runtime_binding_observed=False))
 primes=list(struct.unpack('<256I',host.read(0x46433c0,1024)));buckets=[next(n for n in primes if n>=51)]
 while buckets[-1]<1000000:buckets.append(next(n for n in primes if n>=2*buckets[-1]))
 need(buckets==[53,109,227,467,953,2029,4349,8783,17749,35933,72817,147793,299951,608903,1236397],'source-derived prime sequence differs')
 with tempfile.TemporaryDirectory(prefix='c6-trade-order-review-cases-')as tmp:
  directory=Path(tmp);source=directory/'cases.rs';candidate=directory/'cases';source.write_text(HARNESS)
  subprocess.run(['rustc','--edition=2021','-O',str(source),'--extern','c6_core='+str(work/'target/release/libc6_core.rlib'),'-L','dependency='+str(work/'target/release/deps'),'-o',str(candidate)],check=True)
  output=subprocess.check_output([str(candidate)],text=True).strip();need(output=='independent_discriminating_case_groups=8','independent boundary checks incomplete');case_binary=sha(candidate.read_bytes())
  # Execute only the already frozen no-answer program, with no expected Graph input.
  for label in ['a','b']:
   output=directory/(label+'.graph');subprocess.run([str(work/'candidate'),str(work/(label+'.graph')),str(output)],check=True,stdout=subprocess.PIPE)
   expected=work/('a-reconstructed.graph'if label=='a'else'b-predicted-c.graph');need(output.read_bytes()==expected.read_bytes(),'frozen executable no longer reproduces its frozen output')
 report=dict(schema='c6-linux-trade-read-order-independent-review-v1',checks_passed=True,algorithm_commit=commit,algorithm_files=pinned,
  production_export_commit=frozen['source_commit'],whole_source_archive_sha256=frozen['source_archive_sha256'],
  module_sha256=TARGET,host_sha256=HOST,host_va_convention='ET_EXEC absolute VAs; no base added',module_methods=methods,host_methods=host_methods,additional_dependencies=extras,static_import_export_bindings=bindings,
  source_bucket_sequence=buckets,host_prime_table_sha256=sha(host.read(0x46433c0,1024)),
  original_module_tests_passed=2,independent_case_groups=8,case_source_sha256=sha(HARNESS.encode()),case_binary_sha256=case_binary,
  complete_owner_bytes=469638,origins=179,edges=1571,full_a_to_b_bytes_equal=True,payloads_ids_paths_yields_unchanged=True,sorting_or_masking_used=False,
  pre_answer_freeze=dict(manifest_sha256=sha(manifest_raw),frozen_at_utc=frozen['frozen_at_utc'],candidate_sha256=frozen['candidate_sha256'],candidate_source_sha256=frozen['candidate_source_sha256'],inputs=frozen['inputs'],predicted_c_graph_sha256=sha(c_graph),predicted_c_graph_diff_from_b_bytes=65510,native_c_answer_available_at_freeze=False),
  c_prediction_status='Unaccepted pre-answer full Graph-owner prediction; no complete Core C prediction. Any later C comparison requires separate evidence and cannot rewrite this candidate.',
  fresh_pre_read_native_container_state_observed=False,live_host_import_bindings_observed=False,native_allocator_archive_id_map_modeled=False,
  duplicate_contract='Typed codec rejects duplicate/inconsistent keys; native duplicate reuse/replacement is identified but outside this API domain.',
  scope='Fresh default container serialized order only, current Graph version, valid typed unique keys; no existing-container or general save-loader/gameplay acceptance.',
  original_execution_during_review=False,complete_state_action_rng_turns_supported=False,
  historical_raw_core_difference_bytes=89198,historical_raw_graph_difference_bytes=89156,remaining_other_owner_difference_bytes=42,unknown_save_bytes_masked=False,
  driver_sha256=sha(Path(__file__).read_bytes()))
 a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(dict(checks_passed=True,full_graph_bytes=469638,independent_case_groups=8,prediction_still_unaccepted=True,original_execution=False)))
if __name__=='__main__':main()
