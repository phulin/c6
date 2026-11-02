#!/usr/bin/env python3
"""Decompile known addresses in an exclusively owned, closed Ghidra project."""
import argparse
import json
from pathlib import Path
import pyghidra

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('addresses', nargs='+')
p.add_argument('--project-dir', type=Path, default=Path('../c6-ghidra/projects'))
p.add_argument('--project-name', default='c6-xp2')
p.add_argument('--program', default='GameCore_XP2_FinalRelease.dll-15c1c1')
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()
pyghidra.start()
from ghidra.base.project import GhidraProject
from ghidra.app.decompiler import DecompInterface
from ghidra.util.task import ConsoleTaskMonitor
project = GhidraProject.openProject(str(a.project_dir.resolve()), a.project_name, False)
try:
    program = project.openProgram('/', a.program, False)
    decomp = DecompInterface()
    decomp.openProgram(program)
    result = {'program':a.program, 'sha256':str(program.getExecutableSHA256()), 'image_base':str(program.getImageBase()), 'functions':[]}
    for address in a.addresses:
        addr = program.getAddressFactory().getAddress(address)
        func = program.getFunctionManager().getFunctionAt(addr)
        if func is None:
            result['functions'].append({'address':address,'error':'no function at address'})
            continue
        out = decomp.decompileFunction(func, 60, ConsoleTaskMonitor())
        instructions=[]
        it = program.getListing().getInstructions(func.getBody(), True)
        while it.hasNext():
            ins=it.next();instructions.append({'address':str(ins.getAddress()),'instruction':str(ins)})
        result['functions'].append({'address':address,'name':str(func.getName()),'decompiled':out.getDecompiledFunction().getC() if out.decompileCompleted() else None,'error':str(out.getErrorMessage()),'instructions':instructions})
    decomp.dispose()
    project.close(program)
finally:
    project.close()
a.output.parent.mkdir(parents=True, exist_ok=True)
a.output.write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps({'output':str(a.output),'functions':len(result['functions'])}))
