#!/usr/bin/env python3
"""Pinned read-only ChooseTech source and frozen offline Rust boundary checks."""
import argparse
import hashlib
import json
import io
import posixpath
import tarfile
from pathlib import Path
import re
import shutil
import subprocess
import uuid
from save_lifecycle_audit import Elf, GAME_SHA
ROOT=Path(__file__).resolve().parents[2]
SOURCE='crates/c6-data/src/loaded_actions/choose_tech.rs'
CHECK='tools/linux/choose_tech_notification_check.rs'
METHODS=[('Notification.GetPlayer',0x2d2e1e0,15),('Notification.GetPlayerID',0x2cd0560,9),('Player.GetTechs',0x1fcd110,8),('Player.GetCities',0x1fccc20,8),('Player.Cities.GetCount',0x1fcd080,12),('Player.IsHuman',0x2b31530,32),('ChooseTech.ctor',0x2d2a2d0,30),('ChooseTech.Initialize',0x2d2a2f0,40),
 ('ChooseTech.Expired',0x2d2a320,121),('ChooseTech.Create',0x2d2a3b0,31),
 ('ChooseTech.Register',0x2d2a3d0,34),('Instance.IsNeeded',0x2d2e290,28),
 ('Instance.Initialize',0x2d2d7e0,1312),('Instance.ctor',0x2d2d6a0,215),
 ('Instance.CreateWithPool',0x2d2e5b0,122),('Manager.Create',0x2d2f700,194),
 ('Manager.Register',0x2d2f7d0,65),('Manager.Initialize',0x2d2ee00,236),
 ('XP2.Manager.RegisterNotifications',0x2d7bc80,197),('PlayerTechs.GetResearchingTech',0x2b8aa50,42),
 ('PlayerTechs.MoreToResearch',0x2b8b010,103),('MoreToResearch.perTech',0x2b8dc20,69),('PlayerTechs.CanResearch',0x2b8bfb0,200),
 ('PlayerTechs.IsTechRevealed',0x2b8c080,234),('PlayerTechs.HasTech',0x2b8ac60,79),
 ('PlayerTechs.HasBoostBeenTriggered',0x2b8c170,79),('GameTechs.HasTech',0x2a2d160,24),('GameTechs.begin',0x2189530,12),('GameTechs.end',0x2189540,12),('TechNode.GetType',0x276a270,3),('TechNode.IsActive',0x2a2e460,4),('TechNode.GetPrereqs',0x276a2a0,5),
 ('GameTechs.GetTechNode',0x2a2d180,132),('GameTechs.GetTechPrerequisites',0x2a2d3e0,39),
 ('Technology.SQL',0x22198d0,8),('Technology.Builder',0x2219660,595),
 ('TechnologyXP2.SQL',0x2219bd0,8),('TechnologyXP2.Builder',0x2219b00,180),
 ('NotificationID.ctor',0x2ab70f0,17),('ComponentID.ctor',0x1fe21f0,14),
 ('ComponentID.Reset',0x2003840,13),('ComponentID.SetPlayer',0x277c7f0,4),
 ('SeverityFromHash',0x2d2d640,87),('AutoNotify',0x2d2f4b0,236)]

def sha(raw):return hashlib.sha256(raw).hexdigest()
def digest(path):return sha(path.read_bytes())
def require(value,reason):
 if not value:raise ValueError(reason)

def freeze_and_check():
 # Preserve complete project source preimages, including uncommitted concurrent
 # WIP, instead of relabeling a hash list as a reproducible committed build.
 revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
 archive_bytes=subprocess.check_output(['git','archive','--format=tar',revision,'Cargo.toml','Cargo.lock','crates','tools/linux/loaded_actions_review_load.rs','tools/linux/save_lifecycle_audit.py'],cwd=ROOT)
 with tarfile.open(fileobj=io.BytesIO(archive_bytes),mode='r:') as archive:
  payload={member.name:archive.extractfile(member).read() for member in archive.getmembers() if member.isfile()}
 for name,data in list(payload.items()):
  if not name.endswith('.rs'):continue
  for include in re.findall(r'include_(?:str|bytes)!\(\s*"([^"]+)"',data.decode()):
   extra=posixpath.normpath(str(Path(name).parent/include))
   if extra in payload:continue
   old=subprocess.run(['git','show',revision+':'+extra],cwd=ROOT,capture_output=True)
   if old.returncode==0:payload[extra]=old.stdout
 require(payload[SOURCE]==(ROOT/SOURCE).read_bytes(),'owned production source differs from selected committed tree')
 differences=[]
 for name in [CHECK,'tools/linux/choose_tech_notification_audit.py']:
  data=(ROOT/name).read_bytes();old=subprocess.run(['git','show',revision+':'+name],cwd=ROOT,capture_output=True)
  if old.returncode or old.stdout!=data:differences.append(dict(path=name,committed_sha256=sha(old.stdout) if old.returncode==0 else None,retained_sha256=sha(data)))
  payload[name]=data
 folder=ROOT/'target/linux/choose-tech-notification-candidates'/uuid.uuid4().hex;snapshot=folder/'source'
 for name,data in payload.items():
  path=snapshot/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(data);path.chmod(0o444)
 identities={name:sha(data) for name,data in payload.items()}
 (folder/'source-manifest.json').write_text(json.dumps(identities,indent=2,sort_keys=True)+'\n')
 package=folder/'check';package.mkdir();manifest=package/'Cargo.toml'
 manifest.write_text('[workspace]\n[package]\nname="choose-tech-notification-check"\nversion="0.0.0"\nedition="2021"\n[dependencies]\nserde_json="1"\nc6-core={path='+json.dumps(str(snapshot/'crates/c6-core'))+'}\nc6-data={path='+json.dumps(str(snapshot/'crates/c6-data'))+'}\n[[bin]]\nname="check"\npath='+json.dumps(str(snapshot/CHECK))+'\n')
 shutil.copyfile(snapshot/'Cargo.lock',package/'Cargo.lock');target=Path('/tmp/c6-choose-tech-notification-target')
 (folder/'source-origin.json').write_text(json.dumps(dict(source_revision=revision,differences=differences,selection='committed project tree plus exact owned checker/audit preimages; unrelated WIP excluded'),indent=2,sort_keys=True)+'\n')
 run=subprocess.run(['cargo','run','--release','--offline','--manifest-path',str(manifest),'--target-dir',str(target),'--',str(ROOT)],cwd=ROOT,capture_output=True,text=True)
 (folder/'build-stderr.txt').write_text(run.stderr)
 require(run.returncode==0,'frozen checker failed: '+run.stderr)
 result=json.loads(run.stdout);candidate=folder/'candidate';shutil.copyfile(target/'release/check',candidate);candidate.chmod(0o555)
 dependency_artifacts={str(p.relative_to(target)):digest(p) for p in (target/'release/deps').iterdir() if p.is_file() and p.suffix in ('.rlib','.rmeta','.so')}
 result.update(source_revision=revision,source_differences_from_revision=differences,source_sha256=identities,
  retained_source_snapshot=str(snapshot.relative_to(ROOT)),retained_candidate=str(candidate.relative_to(ROOT)),candidate_sha256=digest(candidate),
  generated_manifest=str(manifest.relative_to(ROOT)),generated_manifest_sha256=digest(manifest),dependency_lock_sha256=digest(package/'Cargo.lock'),
  rustc_identity=subprocess.check_output(['rustc','-Vv'],text=True),cargo_identity=subprocess.check_output(['cargo','-V'],text=True),dependency_artifact_sha256=dependency_artifacts,
  source_preimages_complete=True,source_selection='committed project tree plus exact owned checker/audit preimages; unrelated WIP excluded',worktree_matches_retained_sources=all((ROOT/name).is_file() and (ROOT/name).read_bytes()==data for name,data in payload.items()))
 require(all(digest(snapshot/name)==expected for name,expected in identities.items()),'retained source preimage changed')
 archive=folder/'source-preimages.tar.gz'
 archive_payload=dict(payload)
 archive_payload['_fixture/Cargo.toml']=manifest.read_bytes();archive_payload['_fixture/Cargo.lock']=(package/'Cargo.lock').read_bytes()
 archive_payload['_fixture/source-manifest.json']=(folder/'source-manifest.json').read_bytes()
 archive_payload['_fixture/source-differences.json']=json.dumps(differences,sort_keys=True,indent=2).encode()+b'\n'
 with tarfile.open(archive,'w:gz') as out:
  for name,data in sorted(archive_payload.items()):
   item=tarfile.TarInfo(name);item.size=len(data);item.mode=0o444;item.mtime=0;out.addfile(item,io.BytesIO(data))
 archive.chmod(0o444)
 result.update(retained_source_archive=str(archive.relative_to(ROOT)),source_archive_sha256=digest(archive),dependency_artifacts_scope='retained target inventory; dependency versions/checksums pinned by retained Cargo.lock')
 (folder/'result.json').write_text(json.dumps(result,indent=2,sort_keys=True)+'\n')
 return result

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);args=p.parse_args()
 module=ROOT.parent/'c6-assets-linux/libGameCore_XP2.so';game=Elf(module,GAME_SHA)
 symbols={}
 for line in subprocess.check_output(['nm','-S','--defined-only',str(module)],text=True).splitlines():
  fields=line.split()
  if len(fields)==4:
   try:symbols.setdefault(int(fields[0],16),[]).append((int(fields[1],16),fields[3]))
   except ValueError:pass
 methods=[]
 for name,address,size in METHODS:
  require(any(n==size for n,_ in symbols.get(address,[])),'complete method extent differs: '+name)
  text=subprocess.check_output(['objdump','-d','--start-address='+hex(address),'--stop-address='+hex(address+size),str(module)],text=True)
  dependencies=[dict(call_rva=hex(int(a,16)),target=hex(int(t,16)),symbol=s) for a,t,s in re.findall(r'^\s*([0-9a-f]+):[^\n]*\b(?:call|jmp)\s+([0-9a-f]+)\s+<([^>]+)>',text,re.M)]
  methods.append(dict(name=name,rva=hex(address),bytes=size,sha256=sha(game.read(address,size)),dependencies=dependencies))
 require(game.relocated_pointer(0x3afb650+16+0x58)==0x2d7bc80,'XP2 manager registration virtual binding')
 require(game.relocated_pointer(0x3afb650+16+0xd8)==0x2d2f7d0,'actual inherited creator-map Register binding')
 relocation_text=subprocess.check_output(['readelf','-Wr',str(module)],text=True)
 creator_relocation=next((line.strip() for line in relocation_text.splitlines() if re.match(r'^0*3b178b8\s',line)),None)
 require(creator_relocation is not None and 'R_X86_64_GLOB_DAT' in creator_relocation and '0000000002d2a3b0' in creator_relocation and 'ChooseTech6Create' in creator_relocation,'ChooseTech Register static GOT relocation identity')
 require(game.relocated_pointer(0x3af5050+16+0x78)==0x2d2a2f0,'ChooseTech initializer virtual binding')
 require(game.relocated_pointer(0x3af5050+16+0x38)==0x2d2e290,'ChooseTech IsNeeded vtable')
 require(game.relocated_pointer(0x3af5050+16+0x28)==0x2d2a320,'ChooseTech expiry vtable')
 require(game.read(0x2d2a2f9,9).hex()=='66c783990100000001','subtype flag writes')
 require(game.read(0x2d2a302,10).hex()=='c7838c01000000000000','subtype expiry delta zero')
 require(game.read(0x2d2a30c,10).hex()=='c78330010000f91bf1ee','subtype blocking value')
 require(game.read(0x2d2a3c4,3).hex()=='4531c9','ChooseTech drops incoming parameter container')
 sql={}
 for label,address in [('technologies',0x2e42001),('xp2',0x2e421c9)]:sql[label]=game.read(address,1024).split(b'\0')[0].decode()
 require(sql['technologies']=='SELECT rowid, AdvisorType, BarbarianFree, Cost, Critical, Description, EmbarkAll, EmbarkUnitType, EraType, Name, Repeatable, TechnologyType, UITreeRow FROM Technologies ORDER BY rowid','technology native order')
 require(sql['xp2']=='SELECT rowid, HiddenUntilPrereqComplete, RandomPrereqs, TechnologyType FROM Technologies_XP2 ORDER BY rowid','XP2 native order')
 result=freeze_and_check()
 report=dict(schema='c6-linux-choose-tech-notification-source-v1',checks_passed=True,gamecore_sha256=GAME_SHA,original_execution=False,
  methods=methods,definition_sql=sql,creator_static_relocation=creator_relocation,fixture=result,
  creation_scope='source-proven auto=true and NULL parameters; explicit logical allocation/reuse and creator authority; before InitializeText/Add',
  pool_scope='LIFO object selection/logical identity is caller-owned; physical addresses and allocator capacity are not required; reused logical ID/component type retained',
  constructor_effects=dict(default_blocking='0xeef11bf9',expires_delta=0,flag_199=False,flag_19a=True,locks=0,group_default='0xb6fa1284'),
  registration='XP2 Manager RegisterNotifications calls ChooseTech.Register; writes creator under hash0x592df4c8 via Manager.Register. This is source proof, not a live creator-map capture.',
  open=['actual creator mapping at invocation and logical object allocation/reuse authority','complete initialized mutable GameTechs ordered types/active nodes/prerequisite graph when reached',
        'nonempty reused parameter destruction; arbitrary non-auto/fallback creation not implemented','InitializeText/Add/reporting and complete notification Update remain separate',
        'complete affected gameplay state/RNG/order and native action/turn validation'],
  native_factory_or_predicate_compared=False,complete_state_actions_rng_turns=False)
 args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')
 print(json.dumps(dict(checks_passed=True,methods=len(methods),benchmark=result['benchmark'],original_execution=False)))
if __name__=='__main__':main()
