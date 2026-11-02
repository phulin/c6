#!/usr/bin/env python3
"""Apply an exact-identity JSON annotation plan to an exclusively owned project."""
import argparse,hashlib,json
from pathlib import Path
import pyghidra
p=argparse.ArgumentParser(description=__doc__);p.add_argument('plan',type=Path);p.add_argument('--project-dir',type=Path,default=Path('../c6-ghidra/projects'));p.add_argument('--project-name',default='c6-linux');p.add_argument('--output',type=Path,required=True);a=p.parse_args();plan=json.loads(a.plan.read_text())
for path,sha in plan['sources'].items():
 if hashlib.sha256(Path(path).read_bytes()).hexdigest()!=sha:raise SystemExit('Evidence source changed: '+path)
pyghidra.start()
from ghidra.base.project import GhidraProject
from ghidra.program.flatapi import FlatProgramAPI
from ghidra.program.model.listing import CodeUnit
from ghidra.program.model.symbol import SourceType
p=GhidraProject.openProject(str(a.project_dir.resolve()),a.project_name,False);report={'plan_sha256':hashlib.sha256(a.plan.read_bytes()).hexdigest(),'programs':[]}
try:
 for entry in plan['programs']:
  g=p.openProgram('/',entry['program'],False)
  if str(g.getExecutableSHA256()).lower()!=entry['sha256']:raise ValueError('Program SHA mismatch')
  api=FlatProgramAPI(g);listing=g.getListing();tx=g.startTransaction(plan['title']);ok=False;rows=[]
  try:
   for note in entry['annotations']:
    addr=g.getImageBase().add(int(note['rva'],16));previous=listing.getComment(CodeUnit.PRE_COMMENT,addr)
    text=plan['title']+'\nTarget SHA256 '+entry['sha256']+'\n'+note['text']+'\n'+plan['scope']+'\nEvidence plan SHA256 '+report['plan_sha256']
    if previous is None or text not in str(previous):listing.setComment(addr,CodeUnit.PRE_COMMENT,(str(previous)+'\n\n' if previous else '')+text)
    f=g.getFunctionManager().getFunctionAt(addr);created=False
    if note.get('function',True) and f is None:
     api.disassemble(addr);f=api.createFunction(addr,note['name']);created=f is not None
    oldname=str(f.getName()) if f else None;oldsource=f.getSymbol().getSource() if f else None
    if g.getSymbolTable().getSymbol(note['name'],addr,g.getGlobalNamespace()) is None:g.getSymbolTable().createLabel(addr,note['name'],g.getGlobalNamespace(),SourceType.USER_DEFINED)
    if f and oldsource!=SourceType.DEFAULT and str(f.getName())!=oldname:f.setName(oldname,oldsource)
    rows.append({'rva':note['rva'],'name':note['name'],'created_function':created,'comment_sha256':hashlib.sha256(str(listing.getComment(CodeUnit.PRE_COMMENT,addr)).encode()).hexdigest(),'previous_comment_preserved':previous is None or str(listing.getComment(CodeUnit.PRE_COMMENT,addr)).startswith(str(previous))})
   ok=True
  finally:g.endTransaction(tx,ok)
  p.save(g);p.close(g);report['programs'].append({'program':entry['program'],'sha256':entry['sha256'],'annotations':rows})
finally:p.close()
a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'programs':len(report['programs']),'annotations':sum(len(e['annotations']) for e in report['programs'])}))
