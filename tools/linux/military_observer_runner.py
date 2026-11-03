#!/usr/bin/env python3
"""Caller-owned hardware-only GDB drain and real stream lifetime attribution.

This adapter starts only the separately reviewed ordinary Load/Save fixture.
No inferior function call, SQL query, object field write or software game
breakpoint is used. GDB's normal loader rendezvous instrumentation is separate.
"""
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import signal
import time

ROOT=Path(__file__).resolve().parents[2]
MARKER='c6_linux_stream_observation_marker'
FIELDS=('schema','event','occurrence','context','stream','buffer','size','path','result','offset')

def require(test,reason):
    if not test:raise ValueError(reason)

def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def canonical(value):return json.dumps(value,sort_keys=True,separators=(',',':')).encode()

def marker_entry_gate(blob,address,pc):
    prefix=bytes.fromhex('f30f1efa') if blob.startswith(bytes.fromhex('f30f1efa')) else b''
    require(blob[len(prefix):len(prefix)+1]==b'\xc3','marker is not return-only')
    require(pc in (address,address+len(prefix)),'marker PC/prologue mismatch')
    return len(prefix)

class StreamLifetimes:
    """Pure exact sequencing/identity guard used by the actual GDB adapter."""
    def __init__(self):self.active=None;self.seen=set();self.completed=[]
    def advance(self,row):
        event=row['event'];require(row['schema']==1 and type(row['occurrence']) is int and row['occurrence']>0,'marker schema/occurrence')
        for key in ('context','stream','buffer','path'):require(type(row[key]) is int and row[key]>0,'marker pointer')
        if event in (1,11):
            require(self.active is None and row['occurrence'] not in self.seen,'overlapping/reused lifetime')
            self.seen.add(row['occurrence']);self.active=dict(row,next_event=event+1)
        else:
            previous=self.active;require(previous is not None and event==previous['next_event'],'missing/out-of-order marker')
            require(all(row[key]==previous[key] for key in ('schema','occurrence','context','stream','buffer','path')),'marker owner identity')
            if event in (4,15):self.completed.append(row['occurrence']);self.active=None
            else:self.active=dict(row,next_event=event+1)

def process_identity(pid):
    try:
        raw=Path('/proc/'+str(pid)+'/stat').read_text();fields=raw[raw.rfind(')')+2:].split()
        return dict(pid=pid,state=fields[0],ppid=int(fields[1]),pgrp=int(fields[2]),session=int(fields[3]),starttime=int(fields[19]),exe=str(Path('/proc/'+str(pid)+'/exe').resolve()))
    except (OSError,ValueError,IndexError):return None

def session_processes(session):
    rows=[]
    for entry in Path('/proc').iterdir():
        if entry.name.isdigit():
            row=process_identity(int(entry.name))
            if row and row['session']==session:rows.append(row)
    return rows

def dispose_session(process,identity,reason):
    """Only the fresh session launched by this caller; PID/starttime checked."""
    require(identity and identity['pid']==process.pid and identity['session']==process.pid,'unproven debugger session ownership')
    before=session_processes(process.pid)
    for row in [r for r in before if r['pid']!=process.pid]:
        current=process_identity(row['pid'])
        if current and current['starttime']==row['starttime'] and current['session']==process.pid and current['exe']==row['exe']:
            try:os.kill(row['pid'],signal.SIGKILL)
            except ProcessLookupError:pass
    try:process.wait(timeout=0.25)
    except subprocess.TimeoutExpired:
        current=process_identity(process.pid)
        require(current and current['starttime']==identity['starttime'] and current['session']==process.pid and current['exe']==identity['exe'],'debugger ownership changed during disposal')
        process.kill();process.wait(timeout=5)
    remaining=[]
    for _ in range(100):
        remaining=session_processes(process.pid)
        if not any(r['state'] not in ('Z','X') for r in remaining):break
        time.sleep(0.01)
    return dict(reason=reason,launched_session=identity,before=before,remaining=remaining,no_executing_processes=not any(r['state'] not in ('Z','X') for r in remaining),native_cleanup_proven=False)

def run(host,env,work,timeout,directory,prepared,plan):
    """Called by the identity-gated runner, after its expected-profile gate."""
    directory=Path(directory);require(directory.is_dir(),'observer directory absent')
    prepared_data=json.loads(Path(prepared).read_text());cfg=prepared_data['configuration']
    source_files={str(ROOT/name):value for name,value in cfg['sources'].items()};source_files[str(ROOT/'tools/linux/mod_activation_probe.py')]=cfg['runner_sha256']
    gate=dict(schema='c6-linux-military-caller-runtime-v1',report_path=str(prepared),report_sha256=sha(prepared),configuration_sha256=prepared_data['configuration_sha256'],source_files=source_files)
    manifest=json.loads(Path(plan).read_text());manifest['runtime_gate']=gate
    runtime_plan=directory/'runtime-plan.json';runtime_plan.write_text(json.dumps(manifest,indent=2)+'\n')
    command=directory/'capture.gdb';lines=['set pagination off','set confirm off','set debuginfod enabled off','set breakpoint pending on','set non-stop off','set startup-with-shell off']
    for key,value in env.items():
        if not key.startswith(('C6_','LD_')):continue
        require('\n' not in key+value and '\r' not in key+value,'unsafe environment newline')
        lines.append('set environment '+key+' '+value)
    module=str(ROOT/'tools/linux/military_observer_runner.py')
    lines+=['starti','python',"import sys; sys.path.insert(0,"+repr(str(ROOT/'tools/linux'))+")",'import military_observer_runner', 'military_observer_runner.install('+repr(str(runtime_plan))+','+repr(str(directory))+')','end','c6-military-drain']
    command.write_text('\n'.join(lines)+'\n')
    child_env={k:v for k,v in os.environ.items() if not k.startswith(('C6_','LD_'))}
    transcript=directory/'transcript.txt'
    process=None;identity=None;status={}
    try:
        with transcript.open('x') as stream:
            process=subprocess.Popen(['gdb','-nx','-batch','-x',str(command),'--args',str(host)],cwd=work,env=child_env,stdout=stream,stderr=subprocess.STDOUT,start_new_session=True)
            identity=process_identity(process.pid);require(identity and identity['session']==process.pid,'debugger session not established')
            (directory/'supervisor.json').write_text(json.dumps(dict(launched=identity,host_executable=str(host)))+'\n')
            try:process.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                status=dispose_session(process,identity,'supervisor_timeout')
                (directory/'disposal.json').write_text(json.dumps(status,indent=2)+'\n')
                return 124,'',transcript.read_text(errors='replace')
        status=json.loads((directory/'exit.json').read_text()) if (directory/'exit.json').exists() else {}
        alive=[row for row in session_processes(process.pid) if row['state'] not in ('Z','X')]
        if process.returncode!=0 or status.get('observer_failed',True) or alive:
            disposal=dispose_session(process,identity,'debugger_failure_or_remaining_inferior')
            (directory/'disposal.json').write_text(json.dumps(disposal,indent=2)+'\n')
        status.update(gdb_returncode=process.returncode,remaining_session_processes=session_processes(process.pid))
        (directory/'supervisor.json').write_text(json.dumps(dict(launched=identity,terminal=status),indent=2)+'\n')
        code=status.get('inferior_exitcode') if process.returncode==0 and not status.get('observer_failed',True) and not alive else 239
        if type(code) is not int:code=239
        return code,'',transcript.read_text(errors='replace')
    except BaseException:
        if process is not None and identity is not None:
            disposal=dispose_session(process,identity,'caller_exception')
            (directory/'disposal.json').write_text(json.dumps(disposal,indent=2)+'\n')
        raise



def install(plan_path,directory):
    """GDB-only entry; stop callbacks queue work, outer drain mutates HW lanes."""
    import gdb
    from military_initialization_observer import GdbAdapter,HardwareSchedule,SITES,signed32
    directory=Path(directory);plan=json.loads(Path(plan_path).read_text());prepared=json.loads(Path(plan['runtime_gate']['report_path']).read_text());cfg=prepared['configuration'];profile=prepared['configuration_sha256']
    require(hashlib.sha256(canonical(cfg)).hexdigest()==profile,'configuration digest')
    require(sha(__file__)==plan['runtime_gate']['source_files'][str(Path(__file__).resolve())],'runner source gate')
    output=(directory/'observations.jsonl').open('x',buffering=1)
    lifetimes=StreamLifetimes()
    state=dict(pending=None,failed=False,adapter=None,active=None,retired=set(),frames=0,lanes=[],writes=[],rng=None,exitcode=None,markers=[],destroy_pending=None,ready_counts={1:0,11:0})
    def emit(row):output.write(json.dumps(row,sort_keys=True)+'\n')
    def memory(pointer,size):
        require(type(pointer) is int and pointer>0 and 0<=size<=2*1024*1024,'bounded read')
        return bytes(gdb.selected_inferior().read_memory(pointer,size)) if size else b''
    def num(pointer):return int.from_bytes(memory(pointer,8),'little')
    def text(pointer):
        result=bytearray()
        for i in range(4096):
            value=memory(pointer+i,1)
            if value==b'\0':return result.decode()
            result.extend(value)
        raise ValueError('unterminated path')
    def payload_hash(pointer,size):
        require(0<size<=128*1024*1024,'payload extent')
        result=hashlib.sha256()
        for offset in range(0,size,1024*1024):result.update(memory(pointer+offset,min(size-offset,1024*1024)))
        return result.hexdigest()
    def regs():return {name:int(gdb.parse_and_eval('$'+name))&((1<<64)-1) for name in ('pc','rsp','rax','rbx','rbp','rcx','rdx','rsi','rdi','r8','r14')}
    def offset(stream):return num(stream+0x10)
    def rng():
        base=state['adapter'].observer.bias
        # Exact original Globals.GetGame and const GetSyncRandom field bodies.
        require(memory(base+0x1fcc320,11)==bytes.fromhex('488b05f12bb401488b00c3'),'Game getter body')
        require(num(base+0x3b0ef18)==base+0x3c051a8,'Game global GOT binding')
        require(memory(base+0x2d7b4d0,8)==bytes.fromhex('488d87f8000000c3'),'sync RNG body')
        game=num(base+0x3c051a8);require(game>0,'missing actual Game')
        return dict(game_pointer=game,sync_pointer=game+0xf8,map_pointer=base+0x3c072f0,raw64=(memory(game+0xf8,32)+memory(base+0x3c072f0,32)).hex())
    def passive_owner_guard(thread):
        if 'marker_thread' not in state:state['marker_thread']=thread
        require(thread==state['marker_thread'],'unexpected concurrent stream/checkpoint thread')
        controller=num(0x6499100);worker=num(0x6499108)
        require(controller>0,'missing actual controller')
        initialized=memory(controller+0xb,1)[0];enabled=memory(controller+4,1)[0]
        emit(dict(event='passive_owner_guard',controller_pointer=controller,worker_pointer=worker,controller_initialized_raw=initialized,controller_worker_enabled_raw=enabled,thread=list(thread)))
        require(worker==0 and initialized==0,'ordinary fixture worker/controller admission unknown')
    def fresh_frame():state['frames']+=1;return 'frame-'+str(state['frames'])
    def base_binding(kind,stream,military):
        active=state['active'];require(active and active['stream']==stream and active['stage'] in (1,11),'inactive/mismatched stream')
        require((kind=='read')==(active['stage']==1),'Read/Write stream kind')
        require(num(stream+8)==active['buffer'] and offset(stream)<=active['size'],'native stream buffer/offset')
        require(kind!='read' or num(stream+0x18)==active['size'],'native input size')
        state['frames']+=1;occurrence=str(active['occurrence'])+'/military-'+str(state['frames'])
        return dict(kind=kind,lifecycle_id='context-'+hex(active['context']),owner_lifetime_id='military-'+hex(military)+'/stream-'+str(active['occurrence'])+'/scope-'+str(state['frames']),owner_lifetime_kind='selected_original_Read_or_Write_frame_only',military_pointer=military,player_pointer_at_entry=num(military+0x98) if kind=='write' else None,prior_Read_owner_is_unverified=(kind=='read'),configuration_sha256=profile,runtime_profile_sha256=profile,runner_source_sha256=state['adapter'].runner_source_sha256,rules_inputs_sha256=cfg['original_filesystem_profile']['full_content_input_gate_sha256'],rules_database_sha256=None,live_database_snapshot_unavailable=True,dlc_sha256=hashlib.sha256(canonical(cfg['dlc_descriptors'])).hexdigest(),mods_sha256=hashlib.sha256(canonical({'full_content_input_gate_sha256':cfg['original_filesystem_profile']['full_content_input_gate_sha256'],'saved_configuration_sha256':cfg['saved_configuration_sha256']})).hexdigest(),mods_identity_scope='content and saved-configuration inputs; not a serialized active database',input_save_sha256=cfg['lower_core_military_initialization_observer']['input_save_artifact']['sha256'],input_save_kind='whole_file_supplied_to_saved_configuration_loader',input_core_sha256=cfg['lower_core_stream']['sha256'],input_core_kind='actual_MemoryIStream_payload',save_to_core_extraction_binding_proven=False,occurrence_id=occurrence,stream_pointer=stream,stream_occurrence=active['occurrence'],output_id=active['path'] if kind=='write' else None,core_start=offset(stream))
    def observe_target(name,r,thread):
        observer=state['adapter'].observer;schedule=state['schedule']
        if name in ('read_entry','write_entry','initialize_entry','count_entry'):frame=fresh_frame()
        else:frame=observer.current()['frame_id']
        binding=base_binding(name.split('_')[0],r['rdi'],r['rsi']) if name in ('read_entry','write_entry') else None
        returning=observer.current() if name in ('read_return','write_return') else None
        emit(dict(event='checkpoint',checkpoint=name,registers=r,thread=list(thread),frame_id=frame,actual_entry_return_pc=num(r['rsp']) if name in ('read_entry','write_entry','initialize_entry','count_entry') else None,stream_offset=offset(returning['stream']) if returning else binding['core_start'] if binding else None))
        if name=='initialize_entry':emit(dict(event='owner_installation_witness',read_frame=observer.current('read')['frame_id'],initialize_frame=frame,military_pointer=r['rdi'],actual_Player_argument=r['rsi'],stored_Player_pointer=num(r['rdi']+0x98)))
        if observer.observe(name,r,thread,binding,frame):raise ValueError('observer refused '+name)
        if returning:
            start=returning['binding']['core_start'];end=offset(returning['stream']);require(start<end<=state['active']['size'],'actual Read/Write stream range')
            emit(dict(event='original_stream_range',kind=returning['kind'],source_kind='incoming_raw_Core_Load' if returning['kind']=='read' else 'actual_raw_Core_Save',binding=returning['binding'],frame_id=frame,core_range=[start,end]))
        value=None;return_pc=num(r['rsp']) if name.endswith('_entry') else None
        if name=='personality_return':value=bool(r['rax']&255)
        elif name=='leader_return':value=signed32(r['rax'])
        elif name=='query_entry':value=bool(num(r['rdi']))
        elif name=='write_guard':value=bool(r['rax']&0xffffffff)
        if name=='write_return':
            # Capture the exact range before its original stream is destroyed.
            binding=observer.writes[next(reversed(observer.writes))]
            state['writes'].append(dict(binding=binding,end=offset(binding['stream_pointer'])))
        schedule.advance(name,return_pc=return_pc,value=value)
        if name=='initialize_return':
            root=observer.current('read');require(not observer.observe('read_initialized',r,thread,frame_id=root['frame_id']),'Read initialized refusal');schedule.advance('read_initialized')
    def marker(r):
        observed=dict(zip(FIELDS,struct.unpack('<10Q',memory(r['rdi'],80))));require(observed['schema']==1,'marker schema')
        event=observed['event'];observed['path']=text(observed['path'])
        require(state['schedule'].phase=='idle' and not state['adapter'].observer.calls,'stream marker inside unfinished original frame')
        if event in (1,11):
            require(state['active'] is None and state['destroy_pending'] is None and observed['occurrence'] not in state['retired'],'overlapping/reused stream lifetime')
            state['ready_counts'][event]+=1
            require(state['ready_counts'][event]<=(1 if event==1 else 2),'extra unadmitted Load/Save occurrence')
            state['active']={**observed,'stage':event,'next_event':2 if event==1 else 12};require(num(observed['stream']+8)==observed['buffer'] and offset(observed['stream'])==0,'actual constructed stream')
            if event==1:
                require(observed['size']==cfg['lower_core_stream']['bytes'] and payload_hash(observed['buffer'],observed['size'])==cfg['lower_core_stream']['sha256'],'actual input payload differs')
            else:
                require(observed['path'] in prepared['execution_paths']['military_observer']['game_outputs'].values(),'unadmitted output path')
                snapshot=rng();require(state['rng'] is not None and snapshot==state['rng'],'known RNG changed before Save')
        else:
            active=state['active'];require(active and observed['occurrence']==active['occurrence'] and observed['stream']==active['stream'] and observed['context']==active['context'] and observed['path']==active['path'],'stream lifetime mismatch')
            require(event==active['next_event'],'missing/out-of-order stream lifetime event')
            if event==2:
                require(active['stage']==1 and observed['result']==0 and observed['offset']==observed['size'],'incomplete original Load')
                active['next_event']=3;state['rng']=rng();emit(dict(event='known_rng_after_load',**state['rng']))
            elif event==12:
                require(active['stage']==11 and observed['result']==0 and offset(observed['stream'])==observed['size'],'incomplete Save')
                require(rng()==state['rng'],'known RNG changed during Save');active['output_sha256']=payload_hash(observed['buffer'],observed['size']);active['output_bytes']=observed['size'];active['next_event']=13
            elif event==13:
                require(observed['result']==0 and active.get('output_sha256')==sha(observed['path']),'actual saved output differs from native buffer')
                for item in state['writes']:
                    b=item['binding']
                    if b['stream_occurrence']==active['occurrence']:
                        require(b['core_start']<item['end']<=active['output_bytes'],'writer output range')
                        require(not state['adapter'].observer.bind_output(b['occurrence_id'],b['output_id'],active['output_sha256'],active['output_sha256'],[b['core_start'],item['end']]),'saved output binding refused')
                active['next_event']=14
            elif event in (3,14):
                require((event==3 and active['stage']==1) or (event==14 and active['stage']==11 and 'output_sha256' in active),'wrong retirement stage')
                state['retired'].add(active['occurrence']);state['destroy_pending']={**observed,'next_event':event+1};state['active']=None
            elif event in (4,15):raise ValueError('destroyed marker reached active-lifetime branch')
            else:raise ValueError('unknown marker event')
        emit(dict(event='stream_marker',**{('marker_event' if k=='event' else k):v for k,v in observed.items()}))
    class Lane(gdb.Breakpoint):
        def __init__(self,name,address=None):
            self.name=name
            spec=MARKER if name=='marker' else '*'+hex(address)
            super().__init__(spec,type=gdb.BP_HARDWARE_BREAKPOINT,internal=False)
        def stop(self):
            try:state['pending']=(self.name,regs(),gdb.selected_thread().ptid)
            except Exception as error:state['pending']=('stop_error',{'reason':str(error)},None)
            return True
    state['marker']=Lane('marker')
    def rebuild():
        for lane in state['lanes']:lane.delete()
        state['lanes']=[]
        if state['active'] is None:return
        observer=state['adapter'].observer
        for point in state['schedule'].points:
            if type(point) is int:
                matches=[c for c in observer.calls if c['return_pc']==point];require(len(matches)==1,'ambiguous actual return')
                name=matches[0]['kind']+'_return';address=point
            else:
                name=point;address=observer.bias+(SITES['read_initialized'] if point=='initialize_return' else SITES[point])
            state['lanes'].append(Lane(name,address))
        require(len(state['lanes'])<=3,'hardware target lane budget')
    class Drain(gdb.Command):
        def __init__(self):super().__init__('c6-military-drain',gdb.COMMAND_RUNNING)
        def invoke(self,argument,from_tty):
            require(not argument,'unexpected drain arguments')
            try:
                while gdb.selected_inferior().pid:
                    gdb.execute('continue')
                    if not gdb.selected_inferior().pid:break
                    pending=state['pending'];state['pending']=None;require(pending is not None,'unexpected signal/breakpoint stop')
                    name,r,thread=pending
                    require(name!='stop_error','register/frame capture failed: '+str(r))
                    if state['adapter'] is None:
                        require(name=='marker','target observed before mapped gate');state['adapter']=GdbAdapter(plan_path,emit);state['schedule']=HardwareSchedule()
                        inferior_identity=process_identity(gdb.selected_inferior().pid);require(inferior_identity and inferior_identity['exe']==str(Path(plan['host']).resolve()),'actual inferior identity');(directory/'inferior.json').write_text(json.dumps(inferior_identity)+'\n')
                        address=int(gdb.parse_and_eval('(void *)&'+MARKER));marker_prefix=marker_entry_gate(memory(address,8),address,r['pc'])
                        emit(dict(event='installed',profile_sha256=profile,hardware_target_lanes=3,provenance_lanes=1,software_game_breakpoints=False,loader_rendezvous_instrumentation=True,marker_symbol_address=address,marker_stop_pc=r['pc'],marker_optional_endbr64_bytes=marker_prefix))
                    passive_owner_guard(thread)
                    if name=='marker':
                        address=int(gdb.parse_and_eval('(void *)&'+MARKER));marker_entry_gate(memory(address,8),address,r['pc'])
                        emit(dict(event='marker_checkpoint',registers=r,thread=list(thread),descriptor_raw80=memory(r['rdi'],80).hex()))
                        lifetimes.advance(dict(zip(FIELDS,struct.unpack('<10Q',memory(r['rdi'],80)))))
                        event=num(r['rdi']+8)
                        if event in (4,15):
                            observed=dict(zip(FIELDS,struct.unpack('<10Q',memory(r['rdi'],80))));pending_destroy=state['destroy_pending'];require(state['active'] is None and pending_destroy is not None and observed['event']==pending_destroy['next_event'] and all(observed[k]==pending_destroy[k] for k in FIELDS if k not in ('event','path')),'destruction without exact retired lifetime');require(text(observed['path'])==pending_destroy['path'],'destruction path mismatch');state['destroy_pending']=None;emit(dict(event='stream_destroyed',marker=observed))
                            if len(lifetimes.completed)==3:
                                from ai_military_initialization_audit import METHODS
                                from military_initialization_observer import WIRE_METHODS
                                for method,size,label,expected in [*METHODS,*WIRE_METHODS]:require(hashlib.sha256(state['adapter'].read(state['adapter'].observer.bias+method,size)).hexdigest()==expected,'final live method gate: '+label)
                                emit(dict(event='final_live_original_method_fingerprints_equal',method_count=len(METHODS)+len(WIRE_METHODS)))
                        else:marker(r)
                    else:observe_target(name,r,thread)
                    rebuild()
                require(lifetimes.active is None and len(lifetimes.completed)==3,'incomplete physical stream lifetimes')
                require(state['adapter'] is not None and state['active'] is None and state['destroy_pending'] is None and state['ready_counts']=={1:1,11:2},'missing/incomplete stream capture')
                state['adapter'].observer.close();require(not state['adapter'].observer.failed,'observer closing refusal')
            except Exception as error:
                state['failed']=True;emit(dict(event='runner_refused',reason=str(error),capture_complete=False))
                if gdb.selected_inferior().pid:gdb.execute('kill')
            finally:
                (directory/'exit.json').write_text(json.dumps(dict(inferior_exitcode=state['exitcode'],observer_failed=state['failed']))+'\n');output.flush()
    def exited(event):state['exitcode']=getattr(event,'exit_code',None)
    gdb.events.exited.connect(exited);Drain()
