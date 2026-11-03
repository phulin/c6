#!/usr/bin/env python3
"""Independent private-fixture review against an explicit committed source tree."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import tarfile
from save_lifecycle_audit import Elf, GAME_SHA
ROOT=Path(__file__).resolve().parents[2]
def sha(b):return hashlib.sha256(b).hexdigest()
def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--source-commit',required=True)
 p.add_argument('--directory',type=Path,required=True)
 p.add_argument('--output',type=Path,required=True)
 p.add_argument('--module',type=Path,default=ROOT.parent/'c6-assets-linux/libGameCore_XP2.so')
 a=p.parse_args();a.directory=a.directory.resolve();a.directory.mkdir(parents=True,exist_ok=False)
 commit=subprocess.check_output(['git','rev-parse',a.source_commit+'^{commit}'],cwd=ROOT,text=True).strip()
 archive=subprocess.check_output(['git','archive','--format=tar',commit],cwd=ROOT)
 (a.directory/'source.tar').write_bytes(archive);source=a.directory/'source';source.mkdir()
 driver=Path(__file__).read_bytes();(a.directory/'review-driver.py').write_bytes(driver)
 with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
  tar.extractall(source,filter='data')
 contract=source/'evidence/linux/diplomacy-manager-source.json';facts=json.loads(contract.read_text())
 elf=Elf(a.module,GAME_SHA)
 for method in facts['methods']:
  if sha(elf.read(int(method['rva'],16),method['bytes']))!=method['sha256']:raise ValueError('native source method differs')
 for row in facts['instruction_checks']:
  expected=bytes.fromhex(row['bytes'])
  if elf.read(int(row['rva'],16),len(expected))!=expected:raise ValueError('native checked instruction differs')
 template=(ROOT/'tools/linux/diplomacy_manager_independent_review.rs').read_bytes()
 (a.directory/'review-template.rs').write_bytes(template)
 fixture=template
 allocation_api=b'pub allocation_slots:' in (source/'crates/c6-core/src/diplomacy_manager.rs').read_bytes()
 if allocation_api:
  fixture=fixture.replace(b'/* REVIEW_ALLOCATION_FIELD */',b'allocation_slots:{let mut slots=vec![None;65];slots[0]=Some(PlayerHandle{owner:7});slots[1]=Some(PlayerHandle{owner:1});slots},')
  fixture=fixture.replace(b'/* REVIEW_OWNER64_ALLOCATION */',b'snap.allocation_slots[0]=Some(PlayerHandle{owner:64});')
  extra=r"""
    #[test]
    fn malformed_allocation_inventory_is_atomic() {
        for kind in 0..4 {
            let mut s=carrier();let w=bind(&mut s,2,2);let mut snap=snapshot(w);
            match kind {
                0=>{snap.allocation_slots.pop();},
                1=>{snap.allocation_slots.swap(0,1);},
                2=>{snap.allocation_slots[1]=snap.allocation_slots[0];},
                _=>{snap.allocation_slots[64]=Some(PlayerHandle{owner:63});},
            }
            assert!(s.install_diplomacy_manager(snap).is_err());
            assert!(s.diplomacy_manager().is_err());assert_eq!(s.revision(),0);
        }
    }
    #[test]
    fn allocation_slot64_is_excluded_even_for_ever_alive_owner7() {
        let mut s=carrier();let w=witness(&s);let mut c=controls(&s);
        c.player_slots=[false;65];c.player_mapping=[-1;65];c.players.clear();
        for (owner,index) in [(7,64),(1,1)] {
            c.player_slots[owner as usize]=true;c.player_mapping[owner as usize]=index;
            c.players.push(PlayerControls{owner,slot_status:2,team:owner,is_alive:false,turn_active:false});
        }
        s.bind_observed_snapshot_controls(w.clone(),c).unwrap();
        let mut snap=snapshot(w);snap.allocation_slots=vec![None;65];
        snap.allocation_slots[1]=Some(PlayerHandle{owner:1});
        snap.allocation_slots[64]=Some(PlayerHandle{owner:7});
        snap.ever_alive=vec![PlayerHandle{owner:1}];
        s.install_diplomacy_manager(snap).unwrap();
        assert_eq!(s.diplomacy_manager().unwrap().ever_alive(),[PlayerHandle{owner:1}]);
    }
  """
  fixture=fixture.replace(b'/* REVIEW_ALLOCATION_TESTS */',extra.encode())
 (a.directory/'review-fixture.rs').write_bytes(fixture)
 target=source/'crates/c6-core/src/loaded_state.rs';before=target.read_bytes()
 anchor=b'    #[test]\n    fn explicit_image_sharing_never_copies_bytes()'
 if before.count(anchor)!=1:raise ValueError('private fixture insertion point changed')
 target.write_bytes(before.replace(anchor,fixture+b'\n'+anchor,1))
 env=dict(os.environ,CARGO_TARGET_DIR=str(ROOT/'target'))
 command=['cargo','test','--locked','--offline','-p','c6-core','--lib','--no-run','--message-format=json']
 build=subprocess.run(command,cwd=source,env=env,capture_output=True,text=True)
 (a.directory/'build.stderr').write_text(build.stderr);(a.directory/'build.stdout.jsonl').write_text(build.stdout)
 if build.returncode:raise RuntimeError(build.stderr)
 binaries=[r['executable'] for line in build.stdout.splitlines() if (r:=json.loads(line)).get('reason')=='compiler-artifact' and r.get('executable')]
 if len(binaries)!=1:raise ValueError('ambiguous test executable')
 binary=Path(binaries[0]);retained=a.directory/'review-tests';retained.write_bytes(binary.read_bytes());retained.chmod(0o555)
 run=subprocess.run([str(retained),'independent_diplomacy_review','--nocapture'],capture_output=True,text=True)
 (a.directory/'test.stdout').write_text(run.stdout);(a.directory/'test.stderr').write_text(run.stderr)
 # Verify all untouched preimages and the one explicit private fixture patch.
 checked=0
 with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
  for member in tar.getmembers():
   if member.isfile():
    expected=tar.extractfile(member).read()
    if member.name=='crates/c6-core/src/loaded_state.rs':expected=before.replace(anchor,fixture+b'\n'+anchor,1)
    path=source/member.name
    if path.read_bytes()!=expected:raise ValueError('candidate source changed: '+member.name)
    path.chmod(0o444);checked+=1
 report=dict(schema='c6-linux-diplomacy-manager-independent-review-v1',source_commit=commit,
  source_archive_sha256=sha(archive),source_preimages_checked=checked,
  source_contract_sha256=sha(contract.read_bytes()),target_methods_verified=len(facts['methods']),
  target_instructions_verified=len(facts['instruction_checks']),target_sha256=GAME_SHA,
  private_fixture_sha256=sha(fixture),review_template_sha256=sha(template),allocation_inventory_api=allocation_api,
  production_loaded_state_sha256=sha(before),
  patched_private_loaded_state_sha256=sha(target.read_bytes()),candidate_sha256=sha(retained.read_bytes()),
  candidate_directory=str(a.directory),review_driver_sha256=sha(driver),
  rustc_version=subprocess.check_output(['rustc','-Vv'],text=True),cargo_lock_sha256=sha((source/'Cargo.lock').read_bytes()),source_retained=True,only_private_test_fixture_patch=True,
  test_returncode=run.returncode,test_stdout=run.stdout,test_stderr=run.stderr,
  original_execution=False,actual_current_snapshot_admitted=False,full_state_action_turn_parity=False,
  configuration_mutation_test_scope='private read-timing model only; production setter unavailable; such Statement effects remain unsupported',
  unsupported_errors_scope='partial diagnostic kernel state; not accepted action or resumable continuation')
 a.output.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
 print(json.dumps({k:v for k,v in report.items() if k not in ['test_stdout','test_stderr']},sort_keys=True))
 print(run.stdout);print(run.stderr)
if __name__=='__main__':main()
