#!/usr/bin/env python3
"""Exact Linux operation registry ABI audit. Static inspection never runs the game."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
from unit_wire_contracts_operations import symbols, relocations, PREFIX

ROOT = Path(__file__).resolve().parents[2]
METHODS = {
    'manager_get': 0x2da1260, 'manager_initialize': 0x2da13b0,
    'manager_register': 0x2da14c0, 'manager_uninitialize': 0x2da1450,
    'handler_at_const': 0x2da1650, 'handler_for_hash_const': 0x2da16f0,
    'id_for_hash': 0x2da1720, 'handler_hash': 0x2d95650,
    'handler_id': 0x2d95610, 'handler_count': 0x2da2f30,
    'definition_get': 0x2214890, 'definition_count': 0x20e9bd0,
    'definition_find': 0x20b3d20, 'definition_index': 0x20b42c0,
    'definition_hash': 0x20b4900, 'initialize_components': 0x2d77d80,
}

def sha(data):
    return hashlib.sha256(data).hexdigest()


def contract(assets, private_dir=None):
    path = assets / 'libGameCore_XP2.so'
    game = Elf(path, GAME_SHA)
    Elf(assets / 'Civ6', HOST_SHA)
    sy = symbols(path)
    inverse = {}
    for name, (address, size) in sy.items():
        inverse.setdefault(address, []).append((name, size))
    methods = {}
    raw = ''
    for label, address in METHODS.items():
        sizes = {size for _, size in inverse[address]}
        if len(sizes) != 1:
            raise ValueError('ambiguous original function size')
        size = sizes.pop()
        methods[label] = dict(address=hex(address), bytes=size, sha256=sha(game.read(address,size)))
        raw += subprocess.check_output(['objdump','-d','-C',f'--start-address={address}',f'--stop-address={address+size}',str(path)],text=True)
    checks = [(0x2da2f30,'488b4708482b0748c1e803c3'),
              (0x2d95610,'8b4708c3'), (0x2d95650,'8b470cc3'),
              (0x20b42c0,'8b4714c3'), (0x20b4900,'8b4718c3'),
              (0x2da1554,'488918'), (0x2da1575,'8928896b08')]
    for address, code in checks:
        if game.read(address,len(bytes.fromhex(code))) != bytes.fromhex(code):
            raise ValueError(f'original contract changed: {address:#x}')
    rel = relocations(game)
    handlers = []
    for name,(table,size) in sorted(sy.items()):
        if not name.startswith('vtable for '+PREFIX) or size < 0xd8:
            continue
        cls = name.removeprefix('vtable for '+PREFIX)
        if cls == 'Manager':
            continue
        hash_name=PREFIX+cls+'::GetOperationHash()'
        if hash_name not in sy:
            continue
        a,n=sy[hash_name]
        code=game.read(a,n)
        if n == 6 and code[0]==0xb8 and code[-1]==0xc3:
            h=int.from_bytes(code[1:5],'little')
        elif code==bytes.fromhex('31c0c3'):
            h=0
        else:
            raise ValueError('nonconstant handler hash')
        handlers.append(dict(class_name=cls,hash=h,vtable_rva=table+16,
            make_parameters_rva=rel[table+16+0x80],write_rva=rel[table+16+0xb0],read_rva=rel[table+16+0xb8]))
    a,n=sy['GameCore::XP2::Game::Instance::InitializeComponents()']
    init=subprocess.check_output(['objdump','-d','-C',f'--start-address={a}',f'--stop-address={a+n}',str(path)],text=True)
    registrations=re.findall(r'<'+re.escape(PREFIX)+r'(.+?)::Register\(\)@plt>',init)
    by_class={x['class_name']:x for x in handlers}
    selected={}
    for cls in registrations:
        selected[by_class[cls]['hash']]=by_class[cls]
    dwarf=subprocess.check_output(['gdb','-q','-nx','-batch',str(path),'-ex',"ptype /o 'GameCore::Unit::Operations::Manager'",'-ex',"ptype /o 'GameCore::Unit::Operations::BaseHandler'"],text=True)
    for required in ['24      |      32','56      |      48','total size (bytes):  104','8      |       4','12      |       4']:
        if required not in dwarf:
            raise ValueError('DWARF layout mismatch')
    private=dict(handlers=handlers,registration_order=registrations,selected_by_hash=list(selected.values()))
    result=dict(schema='c6-linux-operation-registry-v1',gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,
        methods=methods,manager_layout=dict(bytes=104,handlers_offset=24,hash_map_offset=56),
        handler_layout=dict(bytes=16,id_offset=8,hash_offset=12),
        concrete_tables=len(handlers),registration_calls=len(registrations),selected_hashes=len(selected),
        static_inventory_sha256=sha(json.dumps(private,sort_keys=True,separators=(',',':')).encode()),
        dwarf_sha256=sha(dwarf.encode()),native_query=None,full_state_action_turn_parity=False)
    if private_dir:
        private_dir.mkdir(parents=True,exist_ok=True)
        (private_dir/'manager-methods.asm').write_text(raw)
        (private_dir/'manager-handler.dwarf.txt').write_text(dwarf)
        (private_dir/'static-inventory.json').write_text(json.dumps(private,indent=2)+'\n')
    return result,private


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    p.add_argument('--private-dir',type=Path)
    p.add_argument('--output',type=Path,required=True)
    args=p.parse_args()
    result,_=contract(args.assets,args.private_dir)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:result[k] for k in ['concrete_tables','registration_calls','selected_hashes']}))

if __name__=='__main__':
    main()
