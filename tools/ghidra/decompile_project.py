#!/usr/bin/env python3
"""Decompile known addresses in an exclusively owned, closed Ghidra project."""
import argparse
import json
from pathlib import Path
import pyghidra

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('addresses', nargs='+')
p.add_argument('--project-dir', type=Path, default=Path('../c6-ghidra/projects'))
p.add_argument('--project-name', default='c6-linux')
p.add_argument('--program', default='libGameCore_XP2.so-614d46')
p.add_argument('--create-functions', action='store_true', help='Disassemble and persist verified exact symbol entries before decompiling')
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()
pyghidra.start()
from ghidra.base.project import GhidraProject
from ghidra.app.decompiler import DecompInterface
from ghidra.util.task import ConsoleTaskMonitor
project = GhidraProject.openProject(str(a.project_dir.resolve()), a.project_name, False)
try:
    program = project.openProgram('/', a.program, False)
    identities=json.loads(Path(__file__).with_name('program_identities.json').read_text())
    expected=identities.get('/'+a.program.lstrip('/'))
    if expected is None or str(program.getExecutableSHA256()).lower()!=expected:
        raise ValueError('Unregistered program or program SHA256 mismatch')
    from ghidra.program.flatapi import FlatProgramAPI
    from address_guard import require_function_entry, require_instruction_start
    api=FlatProgramAPI(program)
    decomp = DecompInterface()
    decomp.openProgram(program)
    result = {'program':a.program, 'sha256':str(program.getExecutableSHA256()), 'image_base':str(program.getImageBase()), 'functions':[]}
    for address in a.addresses:
        addr = program.getAddressFactory().getAddress(address)
        func = program.getFunctionManager().getFunctionAt(addr)
        if a.create_functions and (func is None or program.getListing().getInstructionAt(addr) is None):
            tx=program.startTransaction('Create verified exact target boundary')
            ok=False
            try:
                block=program.getMemory().getBlock(addr)
                if block is None or not block.isExecute():raise ValueError('Requested address is not executable')
                if not len(program.getSymbolTable().getSymbols(addr)):raise ValueError('Boundary lacks imported target symbol')
                api.disassemble(addr)
                ins=program.getListing().getInstructionAt(addr)
                require_instruction_start(address, str(ins.getAddress())+' '+str(ins) if ins else '')
                if func is None:func=api.createFunction(addr,None)
                if func is None:raise ValueError('Could not create exact function')
                ok=True
            finally:program.endTransaction(tx,ok)
        if func is None:
            result['functions'].append({'address':address,'error':'no function at address'})
            continue
        if a.create_functions:
            from ghidra.app.cmd.function import CreateFunctionCmd
            tx=program.startTransaction('Repair targeted imported-symbol function body')
            ok=False
            try:
                api.disassemble(addr)
                CreateFunctionCmd.fixupFunctionBody(program,func,ConsoleTaskMonitor())
                ok=True
            finally:program.endTransaction(tx,ok)
        require_function_entry(address,func.getEntryPoint())
        ins=program.getListing().getInstructionAt(addr)
        require_instruction_start(address, str(ins.getAddress())+' '+str(ins) if ins else '')
        out = decomp.decompileFunction(func, 60, ConsoleTaskMonitor())
        instructions=[]
        it = program.getListing().getInstructions(func.getBody(), True)
        while it.hasNext():
            ins=it.next();instructions.append({'address':str(ins.getAddress()),'instruction':str(ins)})
        result['functions'].append({'address':address,'name':str(func.getName()),'decompiled':out.getDecompiledFunction().getC() if out.decompileCompleted() else None,'error':str(out.getErrorMessage()),'instructions':instructions})
    decomp.dispose()
    if a.create_functions:project.save(program)
    project.close(program)
finally:
    project.close()
a.output.parent.mkdir(parents=True, exist_ok=True)
a.output.write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps({'output':str(a.output),'functions':len(result['functions'])}))
