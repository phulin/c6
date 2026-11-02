#!/usr/bin/env python3
"""Import one fingerprinted binary without eager analysis; close MCP first."""
import argparse,hashlib,json
from pathlib import Path
import pyghidra
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('binary',type=Path);p.add_argument('--sha256',required=True)
p.add_argument('--project-dir',type=Path,default=Path('../c6-ghidra/projects'))
p.add_argument('--create-project',action='store_true',help='Create a new private project if absent')
p.add_argument('--image-base',help='Explicit analysis image base, e.g. 0 for ELF ET_DYN module offsets')
p.add_argument('--project-name',default='c6-linux');p.add_argument('--output',type=Path,required=True)
a=p.parse_args();data=a.binary.read_bytes();actual=hashlib.sha256(data).hexdigest()
if actual!=a.sha256:raise SystemExit('Binary SHA256 mismatch')
name=a.binary.name+'-'+hashlib.sha1(data).hexdigest()[:6]
pyghidra.start()
from ghidra.base.project import GhidraProject
from java.io import File
project_path=a.project_dir.resolve()
if not (project_path / (a.project_name+'.gpr')).exists():
 if not a.create_project:raise SystemExit('Project missing; pass --create-project to create it')
 project_path.mkdir(parents=True,exist_ok=True)
 project=GhidraProject.createProject(str(project_path),a.project_name,False)
else:project=GhidraProject.openProject(str(project_path),a.project_name,False)
try:
 if project.getRootFolder().getFile(name) is not None:
  program=project.openProgram('/',name,False);created=False
 else:
  program=project.importProgram(File(str(a.binary.resolve())))
  if program is None:raise RuntimeError('Ghidra binary import failed')
  program.setName(name);project.saveAs(program,'/',name,True);created=True
 if str(program.getExecutableSHA256()).lower()!=actual:raise ValueError('Stored program SHA256 mismatch')
 if a.image_base is not None:
  tx=program.startTransaction('Set explicit target analysis image base');ok=False
  try:
   program.setImageBase(program.getAddressFactory().getAddress(a.image_base),True);ok=True
  finally:program.endTransaction(tx,ok)
 result={'program':'/'+name,'source':str(a.binary.resolve()),'size':len(data),'sha256':actual,'image_base':str(program.getImageBase()),'created':created,'analysis':'targeted import; no full autoanalysis requested','functions':program.getFunctionManager().getFunctionCount()}
 project.save(program);project.close(program)
finally:project.close()
a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
