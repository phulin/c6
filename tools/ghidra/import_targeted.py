#!/usr/bin/env python3
"""Import one fingerprinted binary without eager analysis; close MCP first."""
import argparse,hashlib,json
from pathlib import Path
import pyghidra
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('binary',type=Path);p.add_argument('--sha256',required=True)
p.add_argument('--project-dir',type=Path,default=Path('../c6-ghidra/projects'))
p.add_argument('--project-name',default='c6-xp2');p.add_argument('--output',type=Path,required=True)
a=p.parse_args();data=a.binary.read_bytes();actual=hashlib.sha256(data).hexdigest()
if actual!=a.sha256:raise SystemExit('Binary SHA256 mismatch')
name=a.binary.name+'-'+hashlib.sha1(data).hexdigest()[:6]
pyghidra.start()
from ghidra.base.project import GhidraProject
from java.io import File
project=GhidraProject.openProject(str(a.project_dir.resolve()),a.project_name,False)
try:
 if project.getRootFolder().getFile(name) is not None:
  program=project.openProgram('/',name,False);created=False
 else:
  program=project.importProgram(File(str(a.binary.resolve())))
  if program is None:raise RuntimeError('Ghidra binary import failed')
  program.setName(name);project.saveAs(program,'/',name,True);created=True
 result={'program':'/'+name,'source':str(a.binary.resolve()),'size':len(data),'sha256':actual,'image_base':str(program.getImageBase()),'created':created,'analysis':'targeted import; no full autoanalysis requested','functions':program.getFunctionManager().getFunctionCount()}
 project.save(program);project.close(program)
finally:project.close()
a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
