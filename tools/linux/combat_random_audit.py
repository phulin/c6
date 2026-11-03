#!/usr/bin/env python3
"""Exact Linux random-damage prefix/archive audit; no original execution."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

from combat_kernel_audit import ROOT, HOST, XP2, audit as average_audit
from rng_inventory_audit import read_at

CORE_METHODS = [
    ('game_draw', 0x1fe5240, 0x27), ('wrapper_edit', 0x1fe7230, 0x34),
    ('random_draw', 0x2d48d60, 0xf7), ('random_seed', 0x2d48e60, 4),
    ('logging_getter', 0x1fe31d0, 11), ('turn_getter', 0x1fe4810, 7),
    ('tree_emplace', 0x1fcd400, 0x61), ('tree_find', 0x1fcd470, 0x88),
]
HOST_METHODS = [
    ('archive_constructor', 0x3c45760, 0x15b), ('archive_register', 0x3c459e8, 0x27),
    ('archive_save', 0x3c45a64, 0x36), ('archive_clear_delta', 0x3c45d18, 0x85),
    ('tree_emplace', 0x3c4782e, 0x96),
]
INSTRUCTIONS = [
    ('saved_signed_base',0x2bd898f,'458b7e20'),
    ('average_flag',0x2bd8993,'41807e2c00'),
    ('nonaverage_branch',0x2bd8998,'7418'),
    ('native_bound_u16',0x2bd89c7,'0fb7f0'),
    ('draw_call',0x2bd89d4,'e8f70a3eff'),
    ('draw_zero_extension',0x2bd89d9,'0fb7e8'),
    ('zero_defender_result',0x2bd89fe,'418b4608'),
    ('defender_branch',0x2bd89fc,'7409'),
    ('zero_attacker_result',0x2bd8a25,'418b461c'),
    ('attacker_branch',0x2bd8a23,'7409'),
    ('wrapping_base_add',0x2bd8a2e,'4401fd'),
    ('single_signed_float_conversion',0x2bd8a44,'f30f2ac5'),
    ('owner_pointer',0x1fe7238,'488b7b08'),
    ('changed_tree',0x1fe7246,'4883c738'),
    ('contained_random',0x1fe7257,'4883c310'),
]
RNG_BINDINGS = [
    ('get_game',0x1f78450,0x3bdf110,0x1fcc320,False),
    ('game_draw',0x1fb94d0,0x3b1c648,0x1fe5240,False),
    ('wrapper_edit',0x1f22910,0x3bb4370,0x1fe7230,False),
    ('random_draw',0x1e7a2f0,0x3b60060,0x2d48d60,False),
    ('host_tree_emplace',0x1eb3dc0,0x3b7cdc8,0x3c4782e,True),
    ('logging_getter',0x1f2a370,0x3bb80a0,0x1fe31d0,False),
    ('turn_getter',0x1fb9188,0x3b17068,0x1fe4810,False),
    ('random_seed',0x1f50480,0x3bcb128,0x2d48e60,False),
]


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream,'sha256').hexdigest()


def method_audit(module, methods, private):
    symbols = {}
    for line in subprocess.check_output(['nm','-D','-S','--defined-only',str(module)],text=True).splitlines():
        words=line.split()
        if len(words)==4:
            symbols[int(words[0],16)]=(int(words[1],16),words[3])
    result=[]
    for name,address,size in methods:
        actual,symbol=symbols[address]
        if size!=actual:
            raise ValueError(f'original {name} symbol size differs: {actual:x}')
        code=read_at(module,address,size)
        raw=subprocess.check_output(['objdump','-d','-C',f'--start-address={address}',f'--stop-address={address+size}',str(module)])
        (private/(module.name+'-'+name+'.asm')).write_bytes(raw)
        result.append(dict(name=name,address=hex(address),size=size,symbol=symbol,sha256=hashlib.sha256(code).hexdigest()))
    return result


def audit(module, host, private):
    if digest(module)!=XP2 or digest(host)!=HOST:
        raise ValueError('unsupported original Linux runtime identity')
    private.mkdir(parents=True,exist_ok=True)
    average=average_audit(module,private/'average-dependencies',host)
    core=method_audit(module,CORE_METHODS,private)
    native_host=method_audit(host,HOST_METHODS,private)
    symbols={}
    for path in [module,host]:
        for line in subprocess.check_output(['nm','-D','--defined-only',str(path)],text=True).splitlines():
            words=line.split()
            if len(words)==3:symbols[(path,int(words[0],16))]=words[2]
    relocations=subprocess.check_output(['readelf','-rW',str(module)],text=True)
    bindings=[]
    for name,plt,slot,target,is_host in RNG_BINDINGS:
        code=read_at(module,plt,6)
        if code[:2]!=b'\xff\x25' or plt+6+int.from_bytes(code[2:],'little',signed=True)!=slot:
            raise ValueError('original RNG PLT/GOT contract differs: '+name)
        symbol=symbols[(host if is_host else module,target)]
        if not re.search(r'^0*'+f'{slot:x}'+r'\s+\S+\s+R_X86_64_(?:JUMP_SLOT|GLOB_DAT)\s+\S+\s+'+re.escape(symbol)+r' \+ 0$',relocations,re.M):
            raise ValueError('original RNG relocation symbol differs: '+name)
        bindings.append(dict(name=name,plt=hex(plt),slot=hex(slot),target=hex(target),host_absolute_va=is_host,symbol=symbol))
    source=(ROOT/'native/linux/combat_random.c').read_text().split('dependencies[]={',1)[1].split('};',1)[0]
    expected=[(hex(slot),hex(target),str(int(is_host))) for _,_,slot,target,is_host in RNG_BINDINGS]
    if re.findall(r'\{(0x[0-9a-f]+),(0x[0-9a-f]+),([01])\}',source)!=expected:
        raise ValueError('native random-path runtime gates differ from audited bindings')
    instructions=[]
    for name,address,expected in INSTRUCTIONS:
        if read_at(module,address,len(bytes.fromhex(expected)))!=bytes.fromhex(expected):
            raise ValueError('original prefix instruction differs: '+name)
        instructions.append(dict(name=name,address=hex(address),bytes=len(bytes.fromhex(expected))))
    commands=['gdb','-q','-nx','-batch',str(module)]
    for name in ['std::__1::__tree_node<FAutoVariableBase*, void*>','std::__1::__tree_node_base<void*>',
                 'std::__1::__tree_end_node<std::__1::__tree_node_base<void*>*>',
                 'FAutoArchiveClassContainerBase<GameCore::Game::Instance>']:
        commands+=['-ex',f"ptype /o '{name}'"]
    dwarf=subprocess.check_output(commands,text=True,stderr=subprocess.STDOUT)
    (private/'tree-layout.dwarf').write_text(dwarf)
    for offset,name in [(32,'__value_'),(8,'__right_'),(16,'__parent_'),(24,'__is_black_'),(0,'__left_'),(144,'m_classInstance')]:
        if not re.search(r'/\*\s+'+str(offset)+r'\s+\|\s+[18]\s+\*/[^\n]*\b'+name+r';',dwarf):
            raise ValueError('matching native tree/container DWARF differs: '+name)
    return dict(schema='c6-linux-xp2-random-damage-audit-v1',module_sha256=XP2,host_sha256=HOST,
                average_contract=average,core_methods=core,host_methods=native_host,instruction_contracts=instructions,
                additional_runtime_bindings=bindings,
                matching_tree_dwarf_sha256=hashlib.sha256(dwarf.encode()).hexdigest(),
                archive=dict(owner_from_wrapper=8,game_wrapper=232,game_random=248,game_archive=32,
                    registered_begin=8,registered_end=16,registered_capacity=24,changed_begin=56,
                    changed_root=64,changed_size=72,tree_node_left=0,tree_node_right=8,
                    tree_node_parent=16,tree_node_key=32,tree_node_bytes=40,class_instance=144),
                transformation='Original nonaverage CFG zero-extends the u16 draw, tests zero strengths, adds draw+saved base with wrapping i32 ADD, then performs one signed i32-to-f32 conversion. Rust keeps the draw/edit prefix and supplies that identical wrapped base to unchanged average arithmetic with zero extra; subsequent Q8/FP operations and early-return ordering are retained.',
                invalid_arithmetic='Zero Q8 divisor may trap only after edit/draw. Rust reports an explicit post-draw native-fault outcome with the prefix retained; initial native corpus excludes those trap inputs before any execution.',
                random_internal_order='Random::Get increments calls, computes new seed/sample in registers, invokes logging, then stores the new seed. Conditional comparison is at normal call returns; failed/reentrant logging and allocator failures are not modeled as atomic unsupported operations.',
                native_execution=False,complete_rng_inventory=False,full_combat_parity=False,
                limits=['Conditional named Game sync stream only; caller-provided logical keys require validated native pointer-order mapping',
                        'Native tree allocation/links/colors and logging/service state are not a Rust live-layout implementation',
                        'Full raw Save differences must be reported without masking; named-stream agreement is not full-state or branch-restoration parity'])


def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--module',type=Path,default=ROOT.parent/'c6-assets-linux/libGameCore_XP2.so')
    ap.add_argument('--host',type=Path,default=ROOT.parent/'c6-assets-linux/Civ6')
    ap.add_argument('--private-dir',type=Path,default=ROOT.parent/'c6-ghidra/evidence/linux-combat-random')
    ap.add_argument('--output',type=Path,required=True)
    args=ap.parse_args();result=audit(args.module,args.host,args.private_dir)
    args.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(dict(output=str(args.output),native_execution=False,methods=len(result['core_methods'])+len(result['host_methods']))))


if __name__=='__main__':
    main()
