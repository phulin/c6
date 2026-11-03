#!/usr/bin/env python3
"""Offline own-preload symbols and caller fragments; never loads native code.

Produces a draft fragment, not an admitted executable runtime plan. All addresses
are extracted from the exact supplied ELF, with O0 call-role checks that reject
ambiguous compiler output. Regenerate after every final CLI link.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import tempfile
import subprocess

from full_manager_observer_runner import symbols,elf_contract,compiled_contracts,MARKER
from save_lifecycle_audit import Elf

ROOT=Path(__file__).resolve().parents[2]
REQUIRED=(MARKER,'c6_linux_full_manager_fixture_run','invoke','run_bound','notify',
          'c6_linux_full_manager_profile_probe','mod_probe_main','observed_mod_main_tid')
CHAIN=('prepare','c6_linux_action_prepare_full_manager_fixture','c6_linux_game_load','__libc_start_main')


def require(test,reason):
    if not test:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def instructions(text):
    rows=[]
    for line in text.splitlines():
        match=re.match(r'^\s*([0-9a-f]+):\s+((?:[0-9a-f]{2}\s+)+)\s*(.*)$',line)
        if match:
            raw=bytes.fromhex(match[2]);address=int(match[1],16)
            rows.append(dict(address=address,size=len(raw),raw=raw.hex(),
                             asm=re.sub(r'\s+',' ',match[3].strip()),return_pc=address+len(raw)))
    return rows


def operands(asm):
    text=asm.split('#',1)[0].strip()
    parts=text.split(' ',1)
    if len(parts)<2:return parts[0],[]
    values=[];depth=0;start=0
    for index,char in enumerate(parts[1]):
        if char=='(':depth+=1
        elif char==')':depth-=1
        elif char==',' and depth==0:values.append(parts[1][start:index].strip());start=index+1
    values.append(parts[1][start:].strip())
    return parts[0],values


def register(value):
    aliases={'rax':('rax','eax','ax','al','ah'),'rbx':('rbx','ebx','bx','bl','bh'),
             'rcx':('rcx','ecx','cx','cl','ch'),'rdx':('rdx','edx','dx','dl','dh'),
             'rsi':('rsi','esi','si','sil'),'rdi':('rdi','edi','di','dil'),
             'rbp':('rbp','ebp','bp','bpl'),'rsp':('rsp','esp','sp','spl')}
    for index in range(8,16):aliases['r'+str(index)]=tuple('r'+str(index)+suffix for suffix in ('','d','w','b'))
    value=value.removeprefix('%')
    return next((name for name,items in aliases.items() if value in items),None)


def writes(row,reg):
    mnemonic,args=operands(row['asm'])
    if mnemonic.startswith('call'):
        return reg in ('rax','rcx','rdx','rsi','rdi','r8','r9','r10','r11')
    if mnemonic.startswith(('cmp','test','push','ret','nop','endbr64','j')):
        return False
    return bool(args and register(args[-1])==reg)


def unchanged_slot(code,start,end,slot):
    for row in code[start+1:end]:
        mnemonic,args=operands(row['asm'])
        if args and args[-1]==slot+'(%rbp)' and not mnemonic.startswith(('cmp','test','push')):
            raise ValueError('identified pointer slot overwritten')
        if mnemonic.startswith('lea') and args and args[0]==slot+'(%rbp)':
            raise ValueError('identified pointer slot address escaped')


def reload_before_call(code,index):
    call=re.fullmatch(r'call \*%(\w+)',code[index]['asm'])
    if not call:return None
    reg=register(call[1]);require(reg is not None,'unsupported indirect-call register')
    previous=next((j for j in range(index-1,-1,-1) if writes(code[j],reg)),None)
    if previous is None:return None
    match=re.fullmatch(r'mov (-0x[0-9a-f]+)\(%rbp\),%'+re.escape(call[1]),code[previous]['asm'])
    return (previous,match[1]) if match else None


class OwnElf:
    def __init__(self,path,expected):
        self.path=Path(path).resolve();self.raw=self.path.read_bytes()
        require(sha(self.raw)==expected,'compiled preload identity mismatch')
        self.kind,self.loads=elf_contract(self.raw)
        require(self.kind==3,'compiled preload requires ET_DYN')
        table=symbols(self.raw);self.table={}
        for name in REQUIRED+CHAIN:
            candidates={tuple(sorted(row.items())) for row in table if row['name']==name}
            require(len(candidates)==1,'missing/ambiguous compiled own symbol: '+name)
            self.table[name]=dict(next(iter(candidates)))
        with tempfile.TemporaryDirectory(prefix='c6-preload-immutable-') as temporary:
            snapshot=Path(temporary)/'preload.so';snapshot.write_bytes(self.raw);snapshot.chmod(0o400)
            self.image=Elf(snapshot,expected)
            self.text=subprocess.check_output(['objdump','-d','-w',str(snapshot)],text=True).replace(str(snapshot),str(self.path))
        self.code={}
        for name,row in self.table.items():
            if row['kind']!=2:
                continue
            start=row['relative_address'];end=start+row['size']
            self.code[name]=[insn for insn in instructions(self.text) if start<=insn['address']<end]
            require(self.code[name] and self.code[name][-1]['return_pc']==end,'incomplete compiled own body: '+name)

    def direct(self,name,target):
        matches=[row for row in self.code[name] if row['asm'].startswith('call ') and
                 re.search(r'<'+re.escape(target)+r'(?:@plt)?>$',row['asm'])]
        require(matches,'missing own call edge: '+name+' -> '+target)
        return matches

    def one_direct(self,name,target):
        rows=self.direct(name,target)
        require(len(rows)==1,'ambiguous own call edge: '+name+' -> '+target)
        return rows[0]

    def cstring(self,address):
        data=bytearray()
        for offset in range(512):
            value=self.image.read(address+offset,1)
            if value==b'\0':
                return data.decode('ascii')
            data.extend(value)
        raise ValueError('unterminated own ELF string')

    def selected_pointer_call(self,name,target):
        """O0 dlsym result -> RBP slot -> exact indirect call register.

        This never assumes ordinal order among indirect calls. The selector
        string, following dlsym, result store and later reload are all retained.
        """
        code=self.code[name];selectors=[]
        for index,row in enumerate(code):
            if row['asm'].startswith('lea ') and '#' in row['asm']:
                address=re.search(r'# ([0-9a-f]+)',row['asm'])
                if address:
                    try:
                        if self.cstring(int(address[1],16))==target:
                            selectors.append(index)
                    except (ValueError,UnicodeDecodeError):
                        pass
        require(len(selectors)==1,'missing/ambiguous own dlsym selector: '+target)
        selected=selectors[0]
        dlsym=next((i for i in range(selected+1,len(code)) if re.search(r'<dlsym@plt>$',code[i]['asm'])),None)
        require(dlsym is not None,'own selector never reaches dlsym')
        # Different ternary branches may reach the same call; pointer identity
        # is closed by its actual result store, never by choosing call ordinal.
        store=next((row for row in code[dlsym+1:dlsym+5] if re.fullmatch(r'mov %rax,(-0x[0-9a-f]+)\(%rbp\)',row['asm'])),None)
        require(store is not None,'dlsym result has no source-supported O0 slot')
        slot=re.fullmatch(r'mov %rax,(-0x[0-9a-f]+)\(%rbp\)',store['asm'])[1]
        candidates=[]
        store_index=code.index(store)
        for index,row in enumerate(code):
            reload=reload_before_call(code,index)
            if reload and reload[1]==slot and reload[0]>store_index:
                unchanged_slot(code,store_index,reload[0],slot)
                candidates.append(row)
        require(candidates,'dlsym result slot never reaches an indirect call: '+target)
        return candidates,dict(selector=code[selected],dlsym=code[dlsym],result_store=store,result_slot=slot)


def phase_calls(elf):
    result={}
    for name,phases in [('invoke',(4,5)),('run_bound',(1,None,8,9))]:
        calls=elf.direct(name,'notify')
        require(len(calls)==len(phases),'unexpected own notify call topology')
        for call,phase in zip(calls,phases):
            index=elf.code[name].index(call)
            constants=[row for row in elf.code[name][max(0,index-12):index] if re.fullmatch(r'mov \$0x[0-9a-f]+,%ecx',row['asm'])]
            if phase is not None:
                require(constants and constants[-1]['asm']==f'mov $0x{phase:x},%ecx','actual notify phase constant differs')
                result[phase]=(name,call)
            else:
                require(not constants,'loop phase unexpectedly constant')
                dynamic={int(match[1],16) for row in elf.code[name][max(0,index-40):index]
                         if (match:=re.fullmatch(r'mov \$0x([0-9a-f]+),%eax',row['asm']))}
                require(dynamic=={2,3,6,7},'actual compiled observation loop phase set differs')
                for value in (2,3,6,7):result[value]=(name,call)
    return result


def extract(path,expected):
    elf=OwnElf(path,expected);module=str(elf.path)
    contracts=[dict(elf.table[name],module=module) for name in REQUIRED+CHAIN]
    bodies=[]
    for row in contracts:
        if row['kind']==2:
            raw=elf.image.read(row['relative_address'],row['size'])
            bodies.append(dict(name=row['name'],module=module,relative_address=row['relative_address'],
                               size=row['size'],sha256=sha(raw)))
    marker=elf.table[MARKER];marker_raw=elf.image.read(marker['relative_address'],marker['size'])
    prefix=4 if marker_raw.startswith(bytes.fromhex('f30f1efa')) else 0
    require(marker_raw[prefix:prefix+1]==b'\xc3','compiled own marker has pre-return prologue')
    witness=elf.table['observed_mod_main_tid']
    fragment=dict(preload=module,compiled_symbols=contracts,live_bodies=bodies,
                  marker_relative_address=marker['relative_address'],
                  main_entry_witness=dict(module=module,relative_address=witness['relative_address']))
    compiled_contracts(fragment,elf.raw)
    manager=[]
    code=elf.code['invoke']
    for index,row in enumerate(code):
        if row['asm']!='call *%r10':continue
        field=next((j for j in range(index-1,-1,-1) if writes(code[j],'r10')),None)
        if field is None or code[field]['asm']!='mov 0x38(%rax),%r10':continue
        binding_reload=next((j for j in range(field-1,-1,-1) if writes(code[j],'rax')),None)
        require(binding_reload is not None,'Bindings source register missing')
        match=re.fullmatch(r'mov (-0x[0-9a-f]+)\(%rbp\),%rax',code[binding_reload]['asm'])
        require(match is not None,'Bindings pointer source unsupported')
        store=next((j for j,item in enumerate(code[:binding_reload]) if item['asm']=='mov %rdi,'+match[1]+'(%rbp)'),None)
        require(store is not None,'Bindings source is not original first argument')
        unchanged_slot(code,store,binding_reload,match[1])
        manager.append(dict(row,Bindings_argument_store=code[store],Bindings_reload=code[binding_reload],field_load=code[field]))
    require(len(manager)==1,'missing/ambiguous actual Bindings.can_start indirect call')
    invoke_to_run=elf.one_direct('run_bound','invoke')
    run_to_entry=elf.one_direct('c6_linux_full_manager_fixture_run','run_bound')
    entry_to_profile=elf.one_direct('c6_linux_full_manager_profile_probe','c6_linux_full_manager_fixture_run')
    profile_calls,profile_proof=elf.selected_pointer_call('prepare','c6_linux_full_manager_profile_probe')
    require(len(profile_calls)==1,'ambiguous actual profile-probe call')
    prepare_to_export=elf.one_direct('c6_linux_action_prepare_full_manager_fixture','prepare')
    game_calls,game_proof=elf.selected_pointer_call('c6_linux_game_load','c6_linux_action_prepare_full_manager_fixture')
    require(len(game_calls)==1,'ambiguous actual full-Manager lifecycle call')
    main_calls,main_proof=elf.selected_pointer_call('mod_probe_main','c6_linux_game_load')
    require(len(main_calls)==1,'ambiguous actual game-load call')
    def frame(call):return dict(module=module,relative_pc=call['return_pc'])
    outer=[frame(row) for row in (run_to_entry,entry_to_profile,profile_calls[0],prepare_to_export,game_calls[0],main_calls[0])]
    own_suffix=[frame(manager[0]),frame(invoke_to_run)]+outer
    notify_return=frame(elf.one_direct('notify',MARKER))
    marker_frame=dict(module=module,relative_pc=marker['relative_address'])
    parents={}
    phases=phase_calls(elf)
    for phase,(owner,call) in phases.items():
        middle=[frame(call),frame(invoke_to_run)] if owner=='invoke' else [frame(call)]
        parents[str(phase)]=[[marker_frame,notify_return]+middle+outer]
        if prefix:
            parents[str(phase)].append([dict(marker_frame,relative_pc=marker['relative_address']+prefix),notify_return]+middle+outer)
    fragment.update(schema='c6-linux-full-manager-compiled-plan-fragment-v1',observation_mode='raw_unresolved',
                    preload_sha256=expected,original_execution=False,profile_admitted=False,
                    caller_suffix_preload_prefix=own_suffix,marker_parent_preload_prefixes=parents,
                    caller_suffix_frames=own_suffix,marker_parent_frames=parents,
                    window_parent_prefixes={},internal_HKS_GC_classification='unresolved',
                    marker_contract=dict(raw=marker_raw.hex(),sha256=sha(marker_raw),CET_prefix_bytes=prefix,
                                         accepted_stop_relative_addresses=[marker['relative_address'],marker['relative_address']+prefix]),
                    indirect_call_proofs=dict(Manager=dict(call=manager[0],Bindings_field_offset=56),
                        profile_probe=profile_proof,full_manager_lifecycle=game_proof,game_load=main_proof),
                    complete_runtime_plan=False,
                    remaining=['final CLI-prepared profile/source identity and actual final linked preload',
                               'actual prepared module identities; optional startup frames may remain unresolved in raw mode',
                               'stack review packet and private fresh capture paths; no HKS prediction needed for raw mode'])
    return fragment,elf


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    identity=parser.add_mutually_exclusive_group(required=True)
    identity.add_argument('--build-manifest',type=Path)
    identity.add_argument('--prepared-report',type=Path)
    parser.add_argument('--output-directory',type=Path,required=True)
    parser.add_argument('--source-root',type=Path,default=ROOT)
    args=parser.parse_args()
    manifest_path=args.build_manifest or args.prepared_report
    manifest_raw=manifest_path.read_bytes();manifest=json.loads(manifest_raw)
    extractor_raw=Path(__file__).read_bytes()
    if args.build_manifest:
        require(manifest['schema']=='c6-linux-full-manager-offline-draft-build-v1' and manifest['native_execution'] is False,
                'wrong draft build manifest')
        path=Path(manifest['preload_path']);expected=manifest['preload_sha256'];sources=manifest['source_sha256']
    else:
        require(manifest['schema']=='c6-linux-native-profile-prepared-v1' and manifest['native_execution_performed'] is False and
                manifest['returncode'] is None,'expected unexecuted final prepared profile')
        cfg=manifest['configuration'];path=Path(manifest['execution_paths']['preload']);expected=cfg['preload_sha256'];sources=cfg['sources']
    source_bytes={}
    for name,expected_source in sources.items():
        source=Path(name) if Path(name).is_absolute() else args.source_root/name
        raw=source.read_bytes();require(sha(raw)==expected_source,'declared source preimage changed: '+name)
        relative=str(source.resolve().relative_to(args.source_root.resolve()))
        source_bytes[relative]=raw
    fragment,elf=extract(path,expected)
    out=args.output_directory.resolve();out.mkdir(parents=True,exist_ok=False)
    (out/'preload.so').write_bytes(elf.raw)
    require(sha((out/'preload.so').read_bytes())==expected,'retained compiled preload identity differs')
    (out/'build-or-prepared.json').write_bytes(manifest_raw)
    (out/'extractor.py').write_bytes(extractor_raw)
    for relative,raw in source_bytes.items():
        target=out/'source'/relative;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(raw)
    (out/'owned-preload-disassembly.txt').write_text(elf.text)
    fragment.update(input_manifest_sha256=sha(manifest_raw),retained_preload_sha256=sha((out/'preload.so').read_bytes()),
                    retained_source_sha256={name:sha(raw) for name,raw in source_bytes.items()},
                    extractor_sha256=sha(extractor_raw),
                    declared_header_preimages=[name for name in source_bytes if name.endswith('.h')],
                    provenance_limit='Only build/prepared-declared source preimages are attributed; draft manifests may omit headers.')
    (out/'fragment.json').write_text(json.dumps(fragment,indent=2,sort_keys=True)+'\n')
    print(json.dumps(dict(preload_sha256=expected,fragment=str(out/'fragment.json'),complete_runtime_plan=False,
                         compiled_symbols=len(fragment['compiled_symbols']),original_execution=False)))


if __name__=='__main__':
    main()
