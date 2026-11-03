#!/usr/bin/env python3
"""Read-only handler preflight with an isolated probe linked to frozen301f source."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
from placement_count_reconstruction_results import candidate_gate,canonical,digest,load,require,ROOT
REPORT='8e6726ffb48e4799a77c2bdadc84722978f646f8f1095433ce7d0a51b1efa116'
INPUT='79167949acf5e3482f0782bcc973b4ef5e56119ef1af0d10771f56b6b62950d0'
OUTPUT='e76ba3b6656c496dc0bc477a48f37802aae2d286e0d145c6706f5232a899db95'
def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--report',type=Path,default=ROOT/'evidence/linux/placement-count-native-1.json')
 p.add_argument('--manifest',type=Path,default=ROOT/'evidence/linux/placement-count-candidate-301f9e9.json')
 p.add_argument('--output',type=Path,required=True)
 a=p.parse_args();manifest,binary=candidate_gate(a.manifest);require(digest(a.report)==REPORT,'accepted count report differs')
 accepted=load(a.report);directory=Path(accepted['diagnostics_path']);request=directory/'input.json';result=directory/'candidate.stdout'
 require(digest(request)==INPUT and digest(result)==OUTPUT,'accepted state projection/output differs')
 frozen_source=Path(manifest['source_snapshot_path']).parent/'source'
 for name,h in manifest['sources'].items():require(digest(frozen_source/name)==h,'frozen extracted source differs: '+name)
 parent=ROOT.parent/'c6-ghidra/evidence/linux-skip-turn-handler-preflight';parent.mkdir(parents=True,exist_ok=True)
 work=Path(tempfile.mkdtemp(prefix='probe-',dir=parent));(work/'src').mkdir()
 code=(ROOT/'tools/linux/skip_turn_handler_preflight.rs').read_bytes();(work/'src/main.rs').write_bytes(code)
 # Paths are generated local absolute paths, serialized as TOML basic strings.
 dependencies={'c6-core':{'path':str(frozen_source/'crates/c6-core')},'c6-data':{'path':str(frozen_source/'crates/c6-data')},'serde_json':'1','sha2':'0.10'}
 text='[package]\nname="c6-handler-preflight"\nversion="0.1.0"\nedition="2021"\n[dependencies]\n'
 for name,value in dependencies.items():text+=name+'='+('"'+value+'"' if isinstance(value,str) else '{path='+json.dumps(value['path'])+'}')+'\n'
 (work/'Cargo.toml').write_text(text)
 env=os.environ.copy()
 for k in ['CARGO_TARGET_DIR','CARGO_BUILD_TARGET','RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER']:env.pop(k,None)
 subprocess.run(['cargo','build','--offline','--release'],cwd=work,env=env,check=True)
 probe=work/'target/release/c6-handler-preflight'
 raw=subprocess.check_output([str(probe),str(request),str(result)],env=env);(work/'values.json').write_bytes(raw)
 values=json.loads(raw);rows=values['rows'];unplaced=[r for r in rows if not r['current_plot_present']]
 groups={}
 for name,selected in [('all',rows),('unplaced',unplaced),('human',[r for r in rows if r['is_human']]),('human_reaching_count',[r for r in rows if r['friendly_count'] is not None])]:
  groups[name]=dict(units=len(selected),true=sum(r['result'] for r in selected),false=sum(not r['result'] for r in selected),human=sum(r['is_human'] for r in selected),count_queried=sum(r['friendly_count'] is not None for r in selected),empty_layered_vector=sum(r['layered_occupancy_len']==0 for r in selected),delayed_death=sum(r['delayed_death'] for r in selected),formations_resolved=sum(r['formation_resolved'] is not None for r in selected))
 require(len(rows)==1262 and len(unplaced)==21,'preflight actual fixture coverage differs')
 require(digest(binary)==manifest['candidate_sha256'],'accepted candidate changed during independent preflight')
 report=dict(schema='c6-linux-skip-turn-handler-preflight-v1',checks_passed=True,candidate_source_commit=manifest['source_commit'],candidate_sha256=manifest['candidate_sha256'],candidate_manifest_sha256=digest(a.manifest),source_snapshot_sha256=manifest['source_snapshot_sha256'],
  accepted_count_report_sha256=digest(a.report),state_projection_file_sha256=digest(request),frozen_output_sha256=digest(result),
  preflight_python_sha256=digest(Path(__file__)),preflight_rust_sha256=hashlib.sha256(code).hexdigest(),probe_sha256=digest(probe),rustc_version=subprocess.check_output(['rustc','-Vv'],text=True).strip(),
  groups=groups,unsupported_handlers=0,all_frozen_results_equal=values['all_frozen_results_equal'],all_source_provider_orders_equal=values['all_source_provider_orders_equal'],all_units_unchanged=values['all_units_unchanged'],
  values_sha256=hashlib.sha256(raw).hexdigest(),private_values_path=str(work/'values.json'),
  native_helper_contract_commit='99e810b',native_handler_comparison=False,original_game_execution=False,full_manager_predicate=False,gameplay_action_acceptance=False,complete_state_actions_turns=False,historical_raw_save_load_difference_bytes=89198)
 a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(groups))
if __name__=='__main__':main()
