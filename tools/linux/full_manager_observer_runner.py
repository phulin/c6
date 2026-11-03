#!/usr/bin/env python3
"""Pure-hardware GDB installer for one genuine full Manager predicate fixture.

No launch, inferior function call, software game breakpoint, field write, Lua
lookup/lock/GC operation or policy execution is provided. The sole runner owns
launch/profile/lifecycle and timeout disposal. All evidence written here is private.
"""
import hashlib
import json
import os
from pathlib import Path
import struct

from save_lifecycle_audit import GAME_SHA, HOST_SHA
from script_manager_observer import Observer, SITES, GAME_SITES, SCRIPT_METHODS, LIFECYCLE_METHODS
from script_hks_registry_audit import METHODS as HKS_METHODS
from script_can_start_boundary_audit import BINDINGS, STRINGS
from military_observer_runner import marker_entry_gate

ROOT=Path(__file__).resolve().parents[2]
MARKER='c6_linux_full_manager_fixture_marker'
REGISTERS=('pc','rsp','rax','rbx','rbp','rcx','rdx','rsi','rdi','r8','r9','r12','r13','r14','r15')


def mandatory_bodies(host,core):
    rows=[(host,name,address,size,sha) for name,address,size,sha in SCRIPT_METHODS+HKS_METHODS]
    rows += [(host if module=='host' else core,name,address,size,sha)
             for module,name,address,size,sha in LIFECYCLE_METHODS]
    rows += [(core,'Manager.CanStart',0x2da20f0,579,
              '573164c5e12ad82eeb6761d2492634ae376ae6de63526a3b94cf0608d8550927'),
             (core,'Utility.CallTestAll',0x2ce79b0,116,
              '47ad40fd9efa7ddcbe88b6bc4de3eff8c174262c6cc031ce76d78e63094c431e'),
             (host,'hksi_lua_isfunction.complete',0x343db90,0xc9,
              'fbb28ac253e00a7248456b725de9c2982a6cbd6129ed11eb80796c1551f8a19d')]
    return list(dict.fromkeys(rows))


def require(test,reason):
    if not test:
        raise ValueError(reason)


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def canonical(value):
    return json.dumps(value,sort_keys=True,separators=(',',':')).encode()


def descriptor(raw):
    require(len(raw)==88,'complete fixture descriptor required')
    phase=struct.unpack_from('<I',raw)[0]
    require(1<=phase<=9,'unknown fixture marker phase')
    names=('module','context','game','facade','manager','unit','operation','core_base')
    result=dict(zip(names,struct.unpack_from('<8Q',raw,8)))
    result.update(phase=phase,actor_owner=struct.unpack_from('<i',raw,72)[0],
                  operation_id=struct.unpack_from('<i',raw,76)[0],
                  actor_full_id=struct.unpack_from('<I',raw,80)[0])
    require(all(value in (0,1) for value in raw[84:87]),'noncanonical fixture bool')
    result.update(zip(('visible_only','predicate_returned','allowed'),map(bool,raw[84:87])))
    result['raw88']=raw.hex()
    return result


def same_frame(actual,expected):
    return actual['module']==expected['module'] and actual['relative_pc']==expected['relative_pc']


class Frames:
    """Exact reviewed caller suffix, compared against actual complete unwinds."""
    def __init__(self,suffix,core,base,allow_unresolved_tail=False):
        require(suffix and len(suffix)<=120,'missing/excessive reviewed caller frame suffix')
        self.suffix,self.core,self.base=suffix,core,base
        self.allow_unresolved_tail=allow_unresolved_tail

    def caller(self,frames):
        require((len(frames)>=len(self.suffix) if self.allow_unresolved_tail else len(frames)==len(self.suffix)) and
                all(same_frame(a,e) for a,e in zip(frames,self.suffix)),
                'actual caller/parent frames differ from reviewed main-entry suffix')

    def manager_entry(self,frames):
        require(frames and frames[0]['module']==self.core and frames[0]['relative_pc']==0x2da20f0,
                'not original full Manager entry frame')
        self.caller(frames[1:])

    def descendant(self,frames):
        indices=[i for i,row in enumerate(frames) if row['module']==self.core and row['relative_pc']==0x2da22a0]
        require(len(indices)==1,'missing/reentrant original Manager script continuation')
        self.caller(frames[indices[0]+1:])
        return indices[0]


class Schedule:
    """Marker plus <=2 original target lanes; mutations happen only in drain."""
    def __init__(self):
        self.phase=0
        self.entry_rsp=self.return_pc=None
        self.manager_active=False
        self.manager_returned=False
        self.script_complete=False
        self.script_sites=()
        self.allowed=None

    def marker(self,row):
        require(row['phase']==self.phase+1,'missing/reordered/extra fixture marker')
        if row['phase']==4:
            require(not self.manager_active and not self.manager_returned,'duplicate predicate entry')
        if row['phase']==5:
            require(self.manager_returned and row['predicate_returned'] and row['allowed']==self.allowed,
                    'fixture result differs from actual Manager return')
        if row['phase']==9:
            require(self.manager_returned and not self.manager_active and not row['facade'],
                    'incomplete original Manager/facade lifetime')
        self.phase=row['phase']

    def points(self,base):
        if self.phase!=4:
            return []
        if self.entry_rsp is None:
            return [('manager_entry',base+0x2da20f0)]
        if not self.manager_active:
            return []
        if self.script_sites:
            result=[(name,SITES[name]+(base if name in GAME_SITES else 0)) for name in self.script_sites]
        else:
            result=[('manager_return',self.return_pc)]
            if not self.script_complete:
                result.append(('dispatch',base+SITES['dispatch']))
        require(len(result)<=2,'original hardware lane budget exceeded')
        return result

    def enter(self,r,return_pc):
        require(self.phase==4 and self.entry_rsp is None,'unexpected/duplicate full Manager entry')
        self.entry_rsp,self.return_pc=r['rsp'],return_pc
        self.manager_active=True

    def script(self,event,next_sites):
        require(self.manager_active and not self.script_complete,'script outside selected Manager lifetime')
        if event=='wrapper_return':
            self.script_complete=True
            self.script_sites=()
        else:
            self.script_sites=tuple(next_sites)

    def returned(self,r):
        require(self.manager_active and r['rsp']==self.entry_rsp+8 and not self.script_sites,
                'unmatched Manager return or unfinished Lua/lock interval')
        self.manager_active=False
        self.manager_returned=True
        self.allowed=bool(r['rax']&255)


class RawContextTrace:
    """Known native return/context protocol only; no HKS/registry decoder."""
    def __init__(self,read,emit,base):
        self.read,self.emit,self.base=read,emit,base
        self.expected=('dispatch',);self.call=None;self.ordinal=0;self.failed=False
    def number(self,address):
        return struct.unpack('<Q',self.read(address,8))[0]
    def fail(self,reason):
        self.failed=True
        self.emit(dict(event='raw_trace_failure',reason=str(reason),pending_sites=self.expected,
                       partial_call=self.call,callback_absence_proven=False))
    def close(self):
        if self.call is not None:
            self.fail('terminal incomplete known native CCall/lock interval')
        self.emit(dict(event='raw_trace_terminal',protocol_complete=not self.failed,
                       registry_decoded=False,VM_GC_parent_classification='unresolved',
                       callback_absence_proven=False))
        return not self.failed
    def observe(self,name,r,thread,provenance=None):
        require(name in self.expected,'unmatched raw native checkpoint')
        require(r['pc']==SITES[name]+(self.base if name in GAME_SITES else 0),'wrong raw native PC')
        if name=='dispatch':
            require(self.number(r['rsp']+56)==self.base+0x2da22a0,'wrong actual Manager continuation')
            require(self.read(r['r12'],18)==b'CanStartOperation\0' and self.number(r['r13'])==0x53a63a8,
                    'wrong concrete ScriptSystem/event identity')
            self.ordinal+=1
            self.call=dict(thread=thread,wrapper_rsp=r['rsp'],args=r['r15'],out=r['r14'],event_pointer=r['r12'])
            self.expected=('ccall_entry',)
        else:
            call=self.call
            require(thread==call['thread'],'different selected native caller thread')
            # A generic checkpoint can have unresolved selected-frame attribution.
            # Preserve it without assuming nesting or the selected CCall.
            if name not in ('ccall_entry','wrapper_return') and 'entry_rsp' in call and (
                    r['rsp']!=call['entry_rsp']-24 or r['r14']!=call['context']):
                self.emit(dict(event='unattributed_native_checkpoint',checkpoint=name,
                               registers=r,registry_decoded=False,callback_absence_proven=False))
                return self.expected
            if name=='ccall_entry':
                require(r['rsp']==call['wrapper_rsp']-72 and self.number(r['rsp'])==0x36b40e1 and
                        self.number(r['rsp']+64)==self.base+0x2ce79ff and r['rsi']==0x36b3bc0,
                        'wrong concrete CCall frame/return/body')
                context=self.read(r['rdx'],26)
                require(struct.unpack_from('<QQQ',context)==(call['event_pointer'],0x462e7b4,call['args']),
                        'wrong actual named-event context')
                require(r['rdi']>0,'unsafe null actual CCall state')
                call.update(entry_rsp=r['rsp'],state=r['rdi'],context=r['rdx'])
                self.expected=('acquired',)
            elif name=='acquired':
                require(r['rbx']==call['state'] and r['r15']==0x36b3bc0,'wrong retained CCall state/body')
                self.expected=('resolved','protected_return')
            elif name=='resolved':
                require(r['rbx']==call['state'],'wrong actual resolved-call state')
                self.expected=('protected_return',)
            elif name=='protected_return':
                call['protected_status_raw_u32']=r['rax']&0xffffffff
                self.expected=('before_release',)
            elif name=='before_release':
                call['protected_success_raw_u8']=r['rbx']&255
                self.expected=('released',)
            elif name=='released':
                self.expected=('wrapper_return',)
            elif name=='wrapper_return':
                require(r['rsp']==call['wrapper_rsp'],'wrong native wrapper return stack')
                self.emit(dict(event='raw_actual_CallTestAll_return',return_bit=bool(r['rax']&255),
                               out_value_raw=self.read(call['out'],1).hex(),**call,
                               registry_decoded=False,callback_absence_proven=False))
                self.call=None;self.expected=('dispatch',)
            if self.call is not None and name in ('ccall_entry','acquired','resolved','before_release'):
                self.emit(dict(event='raw_actual_CCall_context',checkpoint=name,
                    context_raw26=self.read(call['context'],26).hex(),
                    actual_CCall_state_header_raw=self.read(call['state'],0x80).hex(),
                    initialized_main_VM_or_GC_safe_proven=False,registry_decoded=False,
                    VM_GC_parent_classification='unresolved',callback_absence_proven=False))
        return self.expected


def runtime_module_contracts(configuration,prepared_preload):
    raw=configuration['lower_core_full_manager_observer'].get('runtime_module_paths')
    require(isinstance(raw,dict) and set(raw)=={'host','preload','core','libc'},
            'exact launch/env-derived four-role runtime_module_paths required')
    paths={}
    for role,path in raw.items():
        require(isinstance(path,str) and Path(path).is_absolute() and str(Path(path).resolve())==path,
                'canonical actual runtime path required: '+role)
        paths[role]=path
    configured=configuration.get('original_filesystem_profile',{}).get('launch_host')
    require(isinstance(configured,str) and paths['host']==str(Path(configured).resolve()),
            'runtime host differs from configured launch executable')
    require(paths['preload']==str(Path(prepared_preload).resolve()),'runtime preload differs from actual prepared LD_PRELOAD file')
    libc=configuration['runtime_dependencies']['libc.so.6']
    require(paths['libc']==str(Path(libc['path']).resolve()),'runtime libc differs from LD_LIBRARY_PATH dependency resolution')
    require(configuration['assets']['Civ6']==HOST_SHA and configuration['assets']['libGameCore_XP2.so']==GAME_SHA,
            'runtime original host/Core content identity differs')
    expected={'host':HOST_SHA,'core':GAME_SHA,'preload':configuration['preload_sha256'],'libc':libc['sha256']}
    identities={}
    for role,path in paths.items():
        _,actual=file_version(path)
        require(actual['sha256']==expected[role],'actual runtime file bytes differ: '+role)
        identities[path]=dict(sha256=actual['sha256'],device=list(actual['device']),inode=actual['inode'])
    return paths,identities


def validate_plan(plan):
    require(plan.get('schema')=='c6-linux-full-manager-observer-plan-v1','wrong full Manager plan schema')
    require(plan.get('profile_kind')=='genuine_lower_core_full_Manager','wrong observer profile scope')
    if 'script_snapshot' in plan:
        from script_state_snapshot import validate_spec
        validate_spec(plan['script_snapshot'])
        from script_state_snapshot import ARGUMENT_BODIES
        require(all(any(row['module']==plan['host'] and row['relative_address']==address and
                        row['size']==size and row['sha256']==sha for row in plan['live_bodies'])
                    for _,address,size,sha in ARGUMENT_BODIES),
                'natural snapshot argument layout source body gate absent')
        require(plan.get('observation_mode')=='raw_unresolved','natural snapshot requires explicit unresolved GC mode')
    mode=plan.get('observation_mode')
    require(mode in ('raw_unresolved','source_closed_registry'),'explicit observation mode required')
    gate=plan['runtime_gate']
    require(digest(gate['report_path'])==gate['report_sha256'],'runtime report changed')
    prepared=json.loads(Path(gate['report_path']).read_text())
    require(prepared.get('schema')=='c6-linux-native-profile-prepared-v1' and
            prepared.get('native_execution_performed') is False and 'returncode' in prepared and
            prepared['returncode'] is None,'report is not unexecuted native preparation')
    cfg=prepared['configuration']
    runtime_paths,runtime_identities=runtime_module_contracts(cfg,prepared['execution_paths']['preload'])
    profile=hashlib.sha256(canonical(cfg)).hexdigest()
    require(profile==prepared['configuration_sha256']==gate['configuration_sha256'],'configuration identity')
    require(plan['host']==runtime_paths['host'] and plan['core']==runtime_paths['core'] and
            plan['preload']==runtime_paths['preload'] and plan.get('libc')==runtime_paths['libc'] and
            all(plan['modules'].get(path)==identity
            for path,identity in runtime_identities.items()),
            'plan paths/dev/inodes differ from actual launch/env/cwd module contracts')
    configured_host=cfg.get('original_filesystem_profile',{}).get('launch_host')
    require(isinstance(configured_host,str) and Path(configured_host).is_absolute() and
            plan['host']==str(Path(configured_host).resolve()),
            'plan host differs from actual configured launch executable path')
    static={key:value for key,value in plan.items() if key!='runtime_gate'}
    bound=cfg['lower_core_full_manager_observer']
    require(static==bound['static_plan'] and digest(gate['static_plan_path'])==bound['static_plan_sha256'] and
            json.loads(Path(gate['static_plan_path']).read_text())==static,
            'static full Manager plan differs from reviewed configuration')
    normalized={str((ROOT/path).resolve()):value for path,value in cfg['sources'].items()}
    normalized[str((ROOT/'tools/linux/mod_activation_probe.py').resolve())]=cfg['runner_sha256']
    require(gate['source_files']==normalized,'source map differs from prepared configuration/runner')
    require(gate['source_files'],'missing frozen source closure')
    if 'script_snapshot' in plan:
        require(str(ROOT/'tools/linux/script_state_snapshot.py') in gate['source_files'],
                'natural snapshot collector missing from frozen source closure')
    require(plan['preload'] in plan['modules'] and
            plan['modules'][plan['preload']]['sha256']==cfg['preload_sha256'] and
            str(Path(prepared['execution_paths']['preload']).resolve())==plan['preload'],
            'compiled preload differs from prepared configuration')
    preload_raw,preload_identity=file_version(plan['preload'])
    require(preload_identity['sha256']==cfg['preload_sha256'],'actual prepared preload file differs')
    compiled_contracts(plan,preload_raw)
    for path,expected in gate['source_files'].items():
        require(digest(path)==expected,'changed runner/guard/observer source: '+path)
    for path in (Path(__file__).resolve(),ROOT/'tools/linux/script_manager_observer.py',
                 ROOT/'tools/linux/script_hks_registry.py',ROOT/'tools/linux/military_observer_runner.py',
                 ROOT/'tools/linux/save_lifecycle_audit.py',ROOT/'tools/linux/script_can_start_boundary_audit.py',
                 ROOT/'tools/linux/script_vm_lifecycle_audit.py',ROOT/'tools/linux/script_hks_registry_audit.py'):
        require(str(path) in gate['source_files'],'adapter/decoder producer source gate absent')
    require(plan['modules'][plan['host']]['sha256']==HOST_SHA and plan['modules'][plan['core']]['sha256']==GAME_SHA,
            'wrong original module identities')
    require(plan['caller_suffix_frames'] and plan['live_bodies'],'missing source-reviewed stack/code contracts')
    if mode=='source_closed_registry':
        require(all(plan['window_parent_prefixes'].get(name) for name in SITES),
                'missing source-closed natural-window parent prefix')
    require(all(plan['marker_parent_frames'].get(str(phase)) for phase in range(1,10)),
            'missing source-closed fixture marker parent frames')
    review=plan['stack_contract_review']
    require(digest(review['path'])==review['sha256'],'compiled caller stack review changed')
    packet=json.loads(Path(review['path']).read_text())
    require(packet['schema']=='c6-linux-full-manager-stack-contract-v1' and
            packet['observation_mode']==mode and
            packet['caller_suffix_frames']==plan['caller_suffix_frames'] and
            packet['window_parent_prefixes']==plan['window_parent_prefixes'] and
            packet['marker_parent_frames']==plan['marker_parent_frames'] and
            packet['live_bodies']==plan['live_bodies'] and
            packet['compiled_symbols']==plan['compiled_symbols'] and
            packet['main_entry_witness']==plan['main_entry_witness'] and
            packet['reviewed_sources']==gate['source_files'] and
            (mode=='raw_unresolved' or packet['no_VM_or_GC_parent_frames_source_closed'] is True),
            'missing source-closed main-entry/caller stack contract')
    require(any(row['module']==plan['preload'] and row['relative_address']==plan['marker_relative_address']
                and row['size']>=1 for row in plan['live_bodies']),'compiled marker body contract absent')
    return prepared,profile


def file_version(path):
    with open(path,'rb') as stream:
        before=os.fstat(stream.fileno());raw=stream.read();after=os.fstat(stream.fileno())
    fields=lambda stat:(stat.st_dev,stat.st_ino,stat.st_size,stat.st_mtime_ns,stat.st_ctime_ns)
    require(fields(before)==fields(after),'module file changed during hashing')
    return raw,dict(version=fields(after),device=(os.major(after.st_dev),os.minor(after.st_dev)),
                    inode=after.st_ino,sha256=hashlib.sha256(raw).hexdigest())


def elf_contract(raw):
    require(raw[:6]==b'\x7fELF\x02\x01','wrong module ELF ABI')
    kind=struct.unpack_from('<H',raw,16)[0]
    require(kind in (2,3),'unsupported mapped ELF type')
    start=struct.unpack_from('<Q',raw,32)[0]
    width,count=struct.unpack_from('<HH',raw,54)
    loads=[]
    for index in range(count):
        tag,flags,offset,address,_,file_size,memory_size,_=struct.unpack_from('<IIQQQQQQ',raw,start+index*width)
        if tag==1:
            loads.append(dict(address=address,offset=offset,file_size=file_size,memory_size=memory_size,flags=flags))
    return kind,loads


def symbols(raw):
    start=struct.unpack_from('<Q',raw,40)[0]
    width,count=struct.unpack_from('<HH',raw,58)
    sections=[struct.unpack_from('<IIQQQQIIQQ',raw,start+index*width) for index in range(count)]
    result=[]
    for section in sections:
        if section[1] not in (2,11):
            continue
        strings=sections[section[6]]
        table=raw[strings[4]:strings[4]+strings[5]]
        for position in range(section[4],section[4]+section[5],section[9]):
            name,info,_,index,address,size=struct.unpack_from('<IBBHQQ',raw,position)
            if not index or not name:
                continue
            text=table[name:table.index(b'\0',name)].decode()
            result.append(dict(name=text,relative_address=address,size=size,kind=info&15))
    return result


def compiled_contracts(plan,raw):
    kind,loads=elf_contract(raw)
    require(kind==3,'fixture preload must be ET_DYN')
    actual=symbols(raw)
    contracts=plan['compiled_symbols']
    required={MARKER,'c6_linux_full_manager_fixture_run','invoke','run_bound','notify',
              'c6_linux_full_manager_profile_probe','mod_probe_main','observed_mod_main_tid'}
    require(required<={row['name'] for row in contracts},'missing compiled caller/entry/witness symbol contract')
    for row in contracts:
        require(row['module']==plan['preload'],'compiled fixture contract not owned by preload')
        symbol=dict(name=row['name'],relative_address=row['relative_address'],size=row['size'],kind=row['kind'])
        require(symbol in actual and row['size']>0,'compiled exact ELF symbol/extent differs: '+row['name'])
        if row['kind']==2:
            load=next((entry for entry in loads if entry['flags']&1 and entry['address']<=row['relative_address'] and
                       row['relative_address']+row['size']<=entry['address']+entry['file_size']),None)
            require(load is not None,'compiled caller function not file-backed executable')
            offset=load['offset']+row['relative_address']-load['address']
            sha=hashlib.sha256(raw[offset:offset+row['size']]).hexdigest()
            require(any(body['module']==row['module'] and body['relative_address']==row['relative_address'] and
                        body['size']==row['size'] and body['sha256']==sha for body in plan['live_bodies']),
                    'complete compiled caller body contract absent: '+row['name'])
        else:
            require(row['name']=='observed_mod_main_tid' and row['kind']==1 and row['size']==8,
                    'unsupported compiled non-function witness')
            witness=plan['main_entry_witness']
            require(row['relative_address']==witness['relative_address'] and witness['module']==row['module'],
                    'main-entry witness differs from actual compiled object')
            require(any(entry['flags']&2 and entry['address']<=row['relative_address'] and
                        row['relative_address']+8<=entry['address']+entry['memory_size'] for entry in loads),
                    'main-entry witness is not preload writable storage')
    if plan.get('observation_mode')=='raw_unresolved':
        main=next(row for row in contracts if row['name']=='mod_probe_main')
        def ends_at_own_main(chain):
            return bool(chain and chain[-1]['module']==plan['preload'] and main['relative_address']<=
                        chain[-1]['relative_pc']<main['relative_address']+main['size'])
        require(ends_at_own_main(plan['caller_suffix_frames']),'raw caller prefix must reach actual compiled mod_probe_main')
        require(all(ends_at_own_main(chain) for chains in plan['marker_parent_frames'].values() for chain in chains),
                'raw marker prefix must reach actual compiled mod_probe_main')
    marker=next(row for row in contracts if row['name']==MARKER)
    require(marker['relative_address']==plan['marker_relative_address'],'compiled actual marker address differs')


class Adapter:
    def __init__(self,gdb,plan,directory,emit):
        self.gdb,self.plan,self.directory,self.emit=gdb,plan,Path(directory),emit
        self.prepared,self.profile=validate_plan(plan)
        self.raw_mode=plan['observation_mode']=='raw_unresolved'
        require(not gdb.parameter('non-stop'),'GDB all-stop mode required')
        self.pid=gdb.selected_inferior().pid
        require(self.pid>0,'no original inferior')
        require(str(Path('/proc/%d/exe'%self.pid).resolve())==plan['host'],'wrong original host executable')
        self.bias={};self.module_identity={};self.loads={}
        for path,identity in plan['modules'].items():
            require(str(Path(path).resolve())==path,'module path must be canonical')
            raw,retained=file_version(path)
            require(retained['sha256']==identity['sha256'],'module file identity changed')
            require(identity.get('inode')==retained['inode'] and tuple(identity.get('device',()))==retained['device'],
                    'declared prelaunch module device/inode changed')
            self.module_identity[path]=retained
            kind,loads=elf_contract(raw);self.loads[path]=loads
            rows=[row for row in self.mappings() if row['path']==path]
            require(rows and all(row['inode']==retained['inode'] and row['device']==retained['device'] for row in rows),
                    'mapped module/file inode or device differs')
            zero=[row['low'] for row in rows if row['offset']==0]
            require(len(zero)==1,'ambiguous/missing ELF first mapping')
            self.bias[path]=0 if kind==2 else zero[0]
            require(kind!=2 or path!=plan['host'] or zero[0]==0x400000,'host ET_EXEC base')
        self.base=self.bias[plan['core']]
        self.frames=Frames(plan['caller_suffix_frames'],plan['core'],self.base,allow_unresolved_tail=self.raw_mode)
        self.schedule=Schedule()
        self.marker_thread=None
        self.marker_row=None
        self.pre_lock=None
        self.body_gate()
        self.observer=(RawContextTrace(self.read,emit,self.base) if self.raw_mode else
                       Observer(self.read,emit,self.base,self.profile,self.directory/'images'))
        self.script_snapshot=None
        if 'script_snapshot' in plan:
            from script_state_snapshot import NaturalSnapshot
            self.script_snapshot=NaturalSnapshot(self.read,emit,self.base,self.profile,
                self.directory/'script-state',plan['script_snapshot'])

    def mappings(self):
        result=[]
        for line in Path('/proc/%d/maps'%self.pid).read_text().splitlines():
            parts=line.split(maxsplit=5)
            lo,hi=(int(item,16) for item in parts[0].split('-'))
            result.append(dict(low=lo,high=hi,permissions=parts[1],offset=int(parts[2],16),
                               device=tuple(int(v,16) for v in parts[3].split(':')),inode=int(parts[4]),
                               path=parts[5] if len(parts)>5 else ''))
        return result

    def stopped(self):
        require(self.gdb.selected_inferior().pid==self.pid,'inferior identity changed')
        threads=self.gdb.selected_inferior().threads()
        require(threads and all(thread.is_stopped() for thread in threads),'all inferior threads must be stopped')

    def read(self,address,size):
        require(type(address) is int and 0<address<1<<64 and type(size) is int and
                0<=size<=65536*40 and address+size<=1<<64,'invalid bounded read')
        self.stopped()
        if not size:
            return b''
        end=address+size
        cursor=address
        for row in self.mappings():
            if row['low']<=cursor<row['high'] and 'r' in row['permissions']:
                cursor=min(end,row['high'])
            if cursor==end:
                return bytes(self.gdb.selected_inferior().read_memory(address,size))
        raise ValueError('unmapped/unreadable stopped memory extent')

    def number(self,address):
        return struct.unpack('<Q',self.read(address,8))[0]

    def module_endpoint(self):
        for path,retained in self.module_identity.items():
            _,current=file_version(path)
            require(current==retained,'retained module file version/hash changed: '+path)
            rows=[row for row in self.mappings() if row['path']==path]
            require(rows and all(row['inode']==retained['inode'] and row['device']==retained['device'] for row in rows),
                    'current module mapping identity changed: '+path)

    def executable(self,path,address,size):
        require(path in self.bias and size>0,'unknown/empty executable contract')
        end=address+size;cursor=address;retained=self.module_identity[path]
        for row in self.mappings():
            if not row['low']<=cursor<row['high']:
                continue
            require(row['path']==path and row['device']==retained['device'] and
                    row['inode']==retained['inode'] and 'r' in row['permissions'] and
                    'x' in row['permissions'] and 'w' not in row['permissions'],
                    'current executable mapping owner/permissions changed')
            stop=min(end,row['high'])
            relative=cursor-self.bias[path]
            load=next((entry for entry in self.loads[path] if entry['flags']&1 and
                       entry['address']<=relative and stop-self.bias[path]<=entry['address']+entry['file_size']),None)
            require(load is not None and row['offset']+cursor-row['low']==
                    load['offset']+relative-load['address'],'current executable mapping file offset changed')
            cursor=stop
            if cursor==end:
                return self.read(address,size)
        raise ValueError('unmapped executable body/frame extent')

    def body_gate(self):
        self.module_endpoint()
        for module,name,address,size,sha in mandatory_bodies(self.plan['host'],self.plan['core']):
            require(hashlib.sha256(self.executable(module,self.bias[module]+address,size)).hexdigest()==sha,
                    'mandatory original/decoder ABI live body changed: '+name)
        for vptr,slot,target,name in BINDINGS:
            require(self.number(vptr+slot)==target,'concrete original host slot changed: '+name)
        for address,raw in STRINGS:
            require(self.read(address,len(raw))==raw,'original named-event string changed')
        require(self.read(self.base+0x2f0bac6,18)==b'CanStartOperation\0','original Manager event changed')
        for row in self.plan['live_bodies']:
            require(row['module'] in self.bias,'ungated original/code module')
            address=self.bias[row['module']]+row['relative_address']
            require(hashlib.sha256(self.executable(row['module'],address,row['size'])).hexdigest()==row['sha256'],
                    'live method/marker/trampoline body changed: '+row['name'])

    def frame_rows(self,allow_unresolved=False):
        self.stopped()
        frame=self.gdb.newest_frame()
        result=[]
        while frame is not None:
            try:
                require(len(result)<128,'unwind exceeds bounded frame depth')
                pc=int(frame.pc());name=frame.name()
            except Exception as error:
                if not allow_unresolved:
                    raise
                result.append(dict(pc=None,module=None,relative_pc=None,classification='partial_unwind',reason=str(error)))
                break
            matches=[row for row in self.mappings() if row['low']<=pc<row['high'] and 'x' in row['permissions']]
            if len(matches)==1 and matches[0]['path'] in self.bias:
                path=matches[0]['path']
                self.executable(path,pc,1) # known-owner mapping failures remain fatal
                result.append(dict(pc=pc,module=path,relative_pc=pc-self.bias[path],name=name))
            else:
                require(allow_unresolved,'unknown/unpinned executable parent frame')
                result.append(dict(pc=pc,module=matches[0]['path'] if len(matches)==1 else None,
                                   relative_pc=None,name=name,classification='unresolved_parent'))
            if getattr(self,'plan',{}).get('script_snapshot'):
                registers={}
                for register in ('rsp','r14','r15','rbx'):
                    try:registers[register]=int(frame.read_register(register))
                    except Exception:pass # unavailable unwind register remains unresolved
                result[-1]['native_registers']=registers
            try:
                frame=frame.older()
            except Exception as error:
                if not allow_unresolved:
                    raise
                result.append(dict(pc=None,module=None,relative_pc=None,classification='partial_unwind',reason=str(error)))
                break
        require(result,'empty actual unwind')
        return result

    def lua_lock(self):
        raw=self.read(0x6ec9c70,64)
        return dict(raw64=raw.hex(),count=struct.unpack_from('<i',raw,8)[0],
                    recursion=struct.unpack_from('<i',raw,12)[0],
                    owner_ASL_ThreadHANDLE_pointer=struct.unpack_from('<Q',raw,16)[0],
                    pthread_mutex_pointer=struct.unpack_from('<Q',raw,24)[0])

    def checkpoint(self,name,r,thread):
        self.stopped()
        target_module=(self.plan['preload'] if name in ('marker','manager_return') else
                       self.plan['core'] if name in GAME_SITES or name=='manager_entry' else self.plan['host'])
        self.executable(target_module,r['pc'],1)
        frames=self.frame_rows(allow_unresolved=self.raw_mode)
        require(self.marker_thread is None or thread==self.marker_thread,'different thread than original caller marker')
        self.emit(dict(event='checkpoint',checkpoint=name,registers=r,thread=list(thread),frames=frames))
        if name=='marker':
            address=self.bias[self.plan['preload']]+self.plan['marker_relative_address']
            marker_entry_gate(self.read(address,8),address,r['pc'])
            row=descriptor(self.read(r['rdi'],88))
            marker_frames=self.plan['marker_parent_frames'].get(str(row['phase']),[])
            require(marker_frames and any((len(frames)>=len(expected) if self.raw_mode else len(expected)==len(frames)) and
                    all(same_frame(actual,wanted) for actual,wanted in zip(frames,expected))
                    for expected in marker_frames),
                    'actual marker/caller/main-entry frames differ from source-closed phase')
            require(row['core_base']==self.base,'fixture actual core base differs')
            if self.marker_row is None:
                witness=self.plan['main_entry_witness']
                require(witness['module']==self.plan['preload'],'main-entry witness not owned by reviewed preload')
                main_tid=self.number(self.bias[witness['module']]+witness['relative_address'])
                require(main_tid>0 and thread[0]==self.pid and thread[1]==main_tid==self.pid,
                        'actual observing thread differs from source-witnessed original main entry')
                self.emit(dict(event='actual_main_entry_thread_witness',os_main_tid=main_tid,
                               witness_module=witness['module'],witness_relative_address=witness['relative_address'],
                               native_core_owner_inferred=False,Lua_ThreadHANDLE_inferred=False))
                require(row['phase']==1,'first marker is not acquired original facade')
                self.marker_thread=thread
            else:
                require(all(row[key]==self.marker_row[key] for key in
                            ('module','context','game','core_base','actor_owner','actor_full_id','visible_only')),
                        'fixture owner/actor/profile inputs changed')
            self.schedule.marker(row)
            self.marker_row=row
            self.emit(dict(event='fixture_marker',**row))
            if row['phase']==9:
                self.body_gate()
            return
        require(self.marker_row is not None,'original point before owned caller marker')
        if name=='manager_entry':
            self.frames.manager_entry(frames)
            row=self.marker_row
            require((r['rdi'],r['rsi'],r['rdx'],r['rcx'],r['r8']&255,r['r9'])==
                    (row['manager'],row['unit'],row['operation'],0,int(row['visible_only']),0),
                    'actual full Manager ABI/actor/Operation differs from live enter marker')
            return_pc=self.number(r['rsp'])
            expected=self.plan['caller_suffix_frames'][0]
            require(return_pc==self.bias[expected['module']]+expected['relative_pc'],
                    'actual full Manager caller return differs')
            self.schedule.enter(r,return_pc)
        elif name=='manager_return':
            self.frames.caller(frames)
            self.schedule.returned(r)
            self.emit(dict(event='actual_Manager_return',allowed=self.schedule.allowed,
                           natural_script_call_observed=self.schedule.script_complete,
                           callback_absence_proven=False))
        else:
            require(self.schedule.manager_active,'Lua point outside genuine selected Manager invocation')
            index=None
            try:
                index=self.frames.descendant(frames)
            except ValueError:
                if not self.raw_mode:
                    raise
            prefixes=self.plan['window_parent_prefixes'].get(name,[])
            classified=bool(index is not None and prefixes and any(len(prefix)==index+1 and
                    all(same_frame(actual,expected) for actual,expected in zip(frames,prefix))
                    for prefix in prefixes))
            if not self.raw_mode:
                require(classified,'actual protected/VM/GC parent prefix differs from source-closed window')
            self.emit(dict(event='actual_protected_parent_prefix',checkpoint=name,
                           frames=frames[:index+1] if index is not None else frames,
                           source_closed=classified,VM_GC_classification='source_closed' if classified else 'unresolved',
                           registry_decoding_enabled=not self.raw_mode))
            if name=='dispatch':
                # Pinned Manager prologue (48+40), call (8), wrapper (56).
                # This identifies the selected outer Manager even when an
                # internal HKS unwind is incomplete or a nested query exists.
                require(r['rsp']==self.schedule.entry_rsp-152,'dispatch is not selected outer Manager stack')
                if not self.raw_mode:
                    require(index==1,'unexpected wrapper/Manager parent chain')
            if name=='ccall_entry':
                if not self.raw_mode:
                    require(index==3 and frames[1]['pc']==0x36b40e1 and
                            frames[2]['pc']==self.base+0x2ce79ff,'concrete CCall direct parents differ')
                self.pre_lock=self.lua_lock()
                if not self.raw_mode:
                    require(self.pre_lock['recursion']==0 and self.pre_lock['owner_ASL_ThreadHANDLE_pointer']==0 and
                            self.pre_lock['pthread_mutex_pointer']!=0,'unwitnessed parent Lua ownership')
                self.emit(dict(event='before_original_Lua_acquisition',**self.pre_lock))
            if name=='acquired':
                lock=self.lua_lock()
                if not self.raw_mode:
                    require(self.pre_lock is not None and lock['recursion']==1 and
                            lock['owner_ASL_ThreadHANDLE_pointer']!=0 and
                            lock['pthread_mutex_pointer']==self.pre_lock['pthread_mutex_pointer'],
                            'original Lua acquisition/mutex provenance differs')
                self.emit(dict(event='original_Lua_Enter_returned' if self.raw_mode else 'original_Lua_acquired',
                               **lock,owner_is_OS_or_core_thread_ID=False,
                               ownership_transition_classification='unresolved' if self.raw_mode else 'source_closed_baseline'))
            provenance=None
            if not self.raw_mode:
                provenance=dict(configuration_sha256=self.profile,scope='genuine_lower_core_full_Manager',
                                all_threads_stopped=True,active_lua_parent_frames=0,active_gc_parent_frames=0,
                                actual_complete_parent_frames=frames,
                                source_review_sha256=self.plan['stack_contract_review']['sha256'])
            if self.raw_mode and name in ('before_release','released'):
                self.emit(dict(event='raw_original_Lua_release_endpoint',checkpoint=name,**self.lua_lock(),
                               ownership_transition_classification='unresolved',registry_decoded=False))
            if self.script_snapshot is not None:
                self.script_snapshot.capture(name,r,frames,self.observer.call)
            next_sites=self.observer.observe(name,r,thread,provenance)
            self.schedule.script(name,next_sites)

    def close(self):
        require(self.schedule.phase==9 and self.schedule.manager_returned,
                'incomplete original fixture/Manager/observer lifetime')
        require(self.observer.close(),'producer observer terminal refusal')
        self.emit(dict(event='adapter_terminal',natural_Lua_calls=self.observer.ordinal,
                       registry_images_captured=not self.raw_mode and self.observer.ordinal>0,
                       observation_mode=self.plan['observation_mode'],callback_effects_accepted=False,
                       callback_absence_proven=False,native_parity_accepted=False))


def install(plan_path,directory):
    """Register c6-full-manager-drain in an existing separately launched GDB.

    Caller supplies its timeout/process disposal supervisor; this code neither
    launches a process nor attempts native teardown when observation fails.
    """
    import gdb
    directory=Path(directory).resolve()
    require(not directory.is_relative_to(ROOT),'private observer directory required')
    directory.mkdir(parents=True,exist_ok=False)
    output=(directory/'observations.jsonl').open('x',buffering=1)
    def emit(row):
        output.write(json.dumps(row,sort_keys=True)+'\n')
        output.flush()
    def installation_failed(reason):
        try:
            emit(dict(event='installation_refused',reason=str(reason),callback_absence_proven=False))
            with (directory/'exit.json').open('x') as stream:
                stream.write(json.dumps(dict(inferior_exitcode=None,observer_failed=True,
                                             original_cleanup_proven=False))+'\n')
        finally:
            output.close()
    try:
        plan=json.loads(Path(plan_path).read_text())
        validate_plan(plan)
    except Exception as error:
        installation_failed(error)
        raise
    state=dict(adapter=None,pending=None,lanes=[],failed=False,exitcode=None,drain_started=False)
    def registers():
        frame=gdb.newest_frame()
        return {name:int(frame.read_register(name))&((1<<64)-1) for name in REGISTERS}
    class Lane(gdb.Breakpoint):
        def __init__(self,name,address=None):
            self.name=name
            spec=MARKER if address is None else '*'+hex(address)
            super().__init__(spec,type=gdb.BP_HARDWARE_BREAKPOINT,internal=False)
        def stop(self):
            try:
                require(state['pending'] is None,'multiple stop callbacks without outer drain')
                state['pending']=(self.name,registers(),gdb.selected_thread().ptid)
            except Exception as error:
                state['pending']=('stop_error',{'reason':str(error)},None)
            return True
    try:
        marker=Lane('marker')
    except Exception as error:
        installation_failed(error)
        raise
    def rebuild():
        for lane in state['lanes']:
            lane.delete()
        state['lanes']=[]
        adapter=state['adapter']
        for name,address in adapter.schedule.points(adapter.base):
            state['lanes'].append(Lane(name,address))
        require(len(state['lanes'])+1<=3,'hardware breakpoint budget exceeded')
    class Drain(gdb.Command):
        def __init__(self):
            super().__init__('c6-full-manager-drain',gdb.COMMAND_RUNNING)
        def invoke(self,argument,from_tty):
            require(not state['drain_started'] and not state['failed'],'full Manager drain is one-shot; continuation refused')
            state['drain_started']=True
            try:
                require(not argument,'unexpected drain arguments')
                while gdb.selected_inferior().pid:
                    gdb.execute('continue')
                    if not gdb.selected_inferior().pid:
                        break
                    pending=state['pending'];state['pending']=None
                    require(pending is not None,'unexpected signal or unrelated breakpoint')
                    name,r,thread=pending
                    require(name!='stop_error','cannot capture original registers: '+str(r))
                    if state['adapter'] is None:
                        require(name=='marker','original point before first fixture marker')
                        state['adapter']=Adapter(gdb,plan,directory,emit)
                        emit(dict(event='installed',configuration_sha256=state['adapter'].profile,
                                  hardware_only=True,software_game_breakpoints=False,
                                  original_calls_by_observer=0,loader_rendezvous_instrumentation_separate=True))
                    state['adapter'].checkpoint(name,r,thread)
                    rebuild()
                require(state['adapter'] is not None,'no genuine fixture marker reached')
                state['adapter'].close()
            except Exception as error:
                state['failed']=True
                emit(dict(event='runner_refused',reason=str(error),callback_absence_proven=False))
                if state['adapter'] is not None:
                    state['adapter'].observer.fail('adapter: '+str(error))
                    state['adapter'].observer.close()
                # Sole runner supervisor owns exact-PID/session disposal. Do
                # not continue originals, call cleanup, detach, or kill an
                # unverified process from this adapter's failure handler.
            finally:
                try:
                    with (directory/'exit.json').open('x') as stream:
                        stream.write(json.dumps(dict(inferior_exitcode=state['exitcode'],
                            observer_failed=state['failed'],original_cleanup_proven=False))+'\n')
                    output.flush()
                finally:
                    output.close()
    def exited(event):
        state['exitcode']=getattr(event,'exit_code',None)
    gdb.events.exited.connect(exited)
    Drain()
    return dict(marker=marker,state=state)
