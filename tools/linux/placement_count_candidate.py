#!/usr/bin/env python3
"""Freeze an offline placement candidate from an immutable complete git archive.

Never builds from the shared working tree and never executes original game code.
Output directories/manifests are create-only; previous candidates stay immutable.
"""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
ROOT=Path(__file__).resolve().parents[2]
BIN='c6-linux-placement-count-candidate'
def sha(data):return hashlib.sha256(data).hexdigest()
def output(command,cwd=ROOT):return subprocess.check_output(command,cwd=cwd,text=True).strip()
def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--source-commit',required=True)
 p.add_argument('--directory',type=Path,required=True)
 p.add_argument('--output',type=Path,required=True)
 a=p.parse_args()
 if a.output.exists():raise ValueError('output manifest already exists; choose a new immutable candidate')
 commit=output(['git','rev-parse',a.source_commit+'^{commit}'])
 archive=subprocess.check_output(['git','archive','--format=tar',commit],cwd=ROOT)
 directory=a.directory.resolve();directory.mkdir(parents=True,exist_ok=False)
 source=directory/'source';source.mkdir()
 archive_path=directory/'source.tar';archive_path.write_bytes(archive)
 with tarfile.open(fileobj=io.BytesIO(archive)) as t:t.extractall(source,filter='data')
 required=source/f'crates/c6-differential/src/bin/{BIN}.rs'
 if not required.is_file():raise ValueError('committed source lacks placement candidate')
 env=os.environ.copy()
 for key in ['CARGO_TARGET_DIR','CARGO_BUILD_TARGET','RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','CARGO_BUILD_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER']:
  env.pop(key,None)
 env['CARGO_INCREMENTAL']='0';env['CARGO_NET_OFFLINE']='true'
 command=['cargo','build','--locked','--offline','--release','-p','c6-differential','--bin',BIN]
 subprocess.run(command,cwd=source,env=env,check=True)
 candidate=source/'target/release'/BIN
 paths=output(['git','ls-tree','-r','--name-only',commit]).splitlines()
 paths=[p for p in paths if p.endswith('.rs') or p.endswith('Cargo.toml') or p=='Cargo.lock' or p in ['tools/linux/placement_count_candidate.py','tools/linux/placement_count_input_contract.py']]
 sources={p:sha((source/p).read_bytes()) for p in paths}
 rustc=shutil.which('rustc');cargo=shutil.which('cargo')
 sysroot=Path(output(['rustc','--print','sysroot']))
 report=dict(schema='c6-linux-placement-count-candidate-v1',source_commit=commit,
  source_snapshot_path=str(archive_path),source_snapshot_bytes=len(archive),source_snapshot_sha256=sha(archive),
  sources=sources,candidate_path=str(candidate),candidate_bytes=candidate.stat().st_size,candidate_sha256=sha(candidate.read_bytes()),
  rustc_version=output(['rustc','-Vv']),cargo_version=output(['cargo','-Vv']),
  rustc_toolchain_sha256=sha((sysroot/'bin/rustc').read_bytes()),cargo_toolchain_sha256=sha((sysroot/'bin/cargo').read_bytes()),
  rustc_launcher_sha256=sha(Path(rustc).read_bytes()),cargo_launcher_sha256=sha(Path(cargo).read_bytes()),
  build_command=command,build_environment={'CARGO_INCREMENTAL':'0','CARGO_NET_OFFLINE':'true','inherited_rustflags_and_wrappers_removed':True},
  input_schema='c6-linux-placement-inputs-v1',output_schema='c6-linux-placement-count-results-v1',
  expected_outputs_used_as_inputs=False,native_execution=False,native_comparison=False,
  native_handler_comparison=False,complete_state_actions_turns=False,
  historical_shared_build_23539a3_candidate_reproducible_from_isolated_source=False,
  prior_profile_reports_rewritten=False)
 a.output.parent.mkdir(parents=True,exist_ok=True)
 with a.output.open('x') as f:json.dump(report,f,indent=2);f.write('\n')
 with (directory/'manifest.json').open('x') as f:json.dump(report,f,indent=2);f.write('\n')
 print(json.dumps({'source_commit':commit,'candidate_sha256':report['candidate_sha256'],'manifest':str(a.output)}))
if __name__=='__main__':main()
