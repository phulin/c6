#!/usr/bin/env python3
"""Static stream projection and synthetic-only adapter ABI/negative checks."""
import ast
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import sys
from military_observer_runner import StreamLifetimes,marker_entry_gate,process_identity,dispose_session
ROOT=Path(__file__).resolve().parents[2]
BASELINE='cd4b227'

def sha(data):return hashlib.sha256(data).hexdigest()
def require(value,why):
    if not value:raise ValueError(why)

def project_stream(text):
    text=text.replace('#include "stream_observation.h"\n','',1)
    start=text.index('/* Passive marker profile only.');end=text.index('/* Actual host constructors',start);text=text[:start]+text[end:]
    start=text.index('    int observe=getenv("C6_MILITARY_STREAM_OBSERVER")');end=text.index('    active_input=stream;',start);text=text[:start]+text[end:]
    start=text.index('    int observe=getenv("C6_MILITARY_STREAM_OBSERVER")');end=text.index('    int (*trace_end)',start);text=text[:start]+text[end:]
    lines=[]
    for line in text.splitlines(keepends=True):
        if 'OBSERVE(' in line or 'size_t loaded_offset=input_offset(stream);' in line:continue
        lines.append(line.replace('result,loaded_offset,count);','result,input_offset(stream),count);'))
    text=''.join(lines)
    original=subprocess.check_output(['git','show',BASELINE+':native/linux/context_stream.c'],cwd=ROOT,text=True)
    require(text==original,'original Load/Save source changed beyond passive marker projection')
    return text

def negative_tests():
    count=0
    def row(event,occurrence=1,**changes):return dict(schema=1,event=event,occurrence=occurrence,context=10,stream=20,buffer=30,size=100,path=40,result=0,offset=0,**changes)
    for sequence in ((1,2,3,4),(11,12,13,14,15)):
        state=StreamLifetimes()
        for event in sequence:state.advance(row(event))
        require(state.active is None and state.completed==[1],'complete lifetime rejected');count+=1
    for sequence in ((1,3),(11,13),(1,2,4),(11,12,14),(1,1),(1,2,3,4,1)):
        try:
            state=StreamLifetimes()
            for event in sequence:state.advance(row(event))
        except ValueError:count+=1
        else:raise ValueError('bad sequence accepted')
    state=StreamLifetimes();state.advance(row(1));changed=row(2);changed['context']=999
    try:state.advance(changed)
    except ValueError:count+=1
    else:raise ValueError('owner switch accepted')
    require(marker_entry_gate(bytes.fromhex('f30f1efac3900f0b'),100,104)==4,'CET marker rejected');count+=1
    require(marker_entry_gate(bytes.fromhex('c3900f0b'),100,100)==0,'non-CET marker rejected');count+=1
    for blob,pc in ((bytes.fromhex('f30f1efac3900f0b'),101),(bytes.fromhex('554889e5c3'),100)):
        try:marker_entry_gate(blob,100,pc)
        except ValueError:count+=1
        else:raise ValueError('wrong marker accepted')
    # A real synthetic child moves to a different process group, like a debugger
    # inferior; disposal must still own/terminate its launched session only.
    child_code="import time;time.sleep(30)"
    parent_code="import os,subprocess,sys;p=subprocess.Popen([sys.executable,'-c',"+repr(child_code)+"],preexec_fn=os.setpgrp);print(p.pid,flush=True);p.wait()"
    process=subprocess.Popen([sys.executable,'-c',parent_code],start_new_session=True,stdout=subprocess.PIPE,text=True)
    identity=process_identity(process.pid);child_pid=int(process.stdout.readline().strip());child=process_identity(child_pid)
    require(child and child['session']==process.pid and child['pgrp']!=identity['pgrp'],'synthetic separate process group absent')
    status=dispose_session(process,identity,'synthetic_timeout_test');process.stdout.close()
    require(status['no_executing_processes'] and process.poll() is not None and status['native_cleanup_proven'] is False,'session disposal failed');count+=1
    return count

def stop_api_guard(source):
    tree=ast.parse(source)
    for node in ast.walk(tree):
        if isinstance(node,(ast.FunctionDef,ast.AsyncFunctionDef)) and node.name=='stop':
            for call in ast.walk(node):
                if isinstance(call,ast.Call):
                    name=call.func.attr if isinstance(call.func,ast.Attribute) else call.func.id if isinstance(call.func,ast.Name) else ''
                    require(name not in ('Breakpoint','delete','execute','call','write_memory'),'unsafe stop callback API')
                if isinstance(call,(ast.Assign,ast.AnnAssign,ast.AugAssign)):
                    targets=call.targets if isinstance(call,ast.Assign) else [call.target]
                    require(not any(isinstance(t,ast.Attribute) and t.attr in ('enabled','condition','thread','ignore_count') for t in targets),'breakpoint mutation in stop')


def audit(assets,host):
    from player_wmds_audit import Elf
    from military_initialization_observer import GAME_SHA,HOST_SHA
    source=(ROOT/'native/linux/context_stream.c').read_text();project_stream(source)
    caller=(ROOT/'tools/linux/military_observer_runner.py').read_text();stop_api_guard(caller)
    for bad in ('self.delete()','self.enabled=False','gdb.execute("continue")'):
        try:stop_api_guard('def stop(self):\n    '+bad+'\n')
        except ValueError:pass
        else:raise ValueError('unsafe stop injection accepted')
    contracts=[('host',0x3c5411e,'488b4708c3'),('host',0x3c54124,'488b4710c3'),('host',0x3c54130,'488b4718c3'),('host',0x3c54386,'488b4708c3'),('host',0x3c5438c,'488b4710c3'),('host',0x31d1720,'8a470bc3'),('host',0x31d18a0,'807f04007411488b3d5b782c034885ff7405e9f935000031c0c3'),('game',0x1fcc320,'488b05f12bb401488b00c3'),('game',0x2d7b4d0,'488d87f8000000c3')]
    objects={'host':Elf(Path(host),HOST_SHA),'game':Elf(Path(assets)/'libGameCore_XP2.so',GAME_SHA)}
    for owner,address,hex_bytes in contracts:require(objects[owner].read(address,len(bytes.fromhex(hex_bytes))).hex()==hex_bytes,'native stream/RNG accessor bytes')
    with tempfile.TemporaryDirectory(prefix='c6-military-marker-') as temporary:
        directory=Path(temporary);library=directory/'marker.so';main=directory/'main.c';exe=directory/'main';script=directory/'smoke.gdb';result=directory/'result.json'
        subprocess.run(['cc','-shared','-fPIC','-Wall','-Wextra','-Werror',str(ROOT/'native/linux/context_stream.c'),'-ldl','-o',str(library)],check=True,capture_output=True)
        main.write_text('#include <dlfcn.h>\n#include "'+str(ROOT/'native/linux/stream_observation.h')+'"\nint main(void){void *h=dlopen("'+str(library)+'",RTLD_NOW);if(!h)return 1;void (*f)(const C6StreamObservation *)=dlsym(h,"c6_linux_stream_observation_marker");if(!f)return 2;C6StreamObservation row={1,11,7,10,20,30,100,40,0,0};f(&row);return 0;}\n')
        subprocess.run(['cc',str(main),'-ldl','-o',str(exe)],check=True,capture_output=True)
        script.write_text('set pagination off\nset confirm off\nset debuginfod enabled off\nset breakpoint pending on\nstarti\nhbreak c6_linux_stream_observation_marker\ncontinue\npython\nimport gdb,json,struct,sys\nsys.path.insert(0,'+repr(str(ROOT/'tools/linux'))+')\nfrom military_observer_runner import marker_entry_gate\np=int(gdb.parse_and_eval("$rdi"));row=struct.unpack("<10Q",bytes(gdb.selected_inferior().read_memory(p,80)))\nassert row==(1,11,7,10,20,30,100,40,0,0)\na=int(gdb.parse_and_eval("(void *)&c6_linux_stream_observation_marker"));prefix=marker_entry_gate(bytes(gdb.selected_inferior().read_memory(a,8)),a,int(gdb.parse_and_eval("$pc")))\nopen('+repr(str(result))+',"w").write(json.dumps({"marker_prefix_bytes":prefix,"actual_descriptor_equal":True}))\nend\ncontinue\n')
        subprocess.run(['gdb','-nx','-batch','-x',str(script),'--args',str(exe)],check=True,capture_output=True,timeout=30)
        smoke=json.loads(result.read_text());require(smoke['actual_descriptor_equal'],'toy ABI failed')
    names=['native/linux/context_stream.c','native/linux/stream_observation.h','tools/linux/military_observer_runner.py','tools/linux/military_stream_fixture_audit.py','tools/linux/mod_activation_probe.py']
    return dict(schema='c6-linux-military-stream-fixture-v1',baseline_commit=subprocess.check_output(['git','rev-parse',BASELINE],cwd=ROOT,text=True).strip(),original_stream_source_projection_equal=True,original_getter_call_count_and_order_preserved=True,marker_record_bytes=80,marker_optional_endbr64_bytes=smoke['marker_prefix_bytes'],marker_toy_actual_descriptor_equal=True,toy_gdb_test_is_original_execution=False,guard_test_groups=negative_tests(),stop_mutation_injection_groups=3,hardware_lanes=4,software_game_breakpoints=False,gdb_loader_rendezvous_instrumentation=True,stream_native_accessor_contracts=[dict(owner=o,address=hex(a),bytes=b) for o,a,b in contracts],known_rng_global_rvas={'Game_global':'0x3c051a8','Game_GOT':'0x3b0ef18','sync_offset':'0xf8','Map_random':'0x3c072f0','bytes_each':32},source_sha256={n:sha((ROOT/n).read_bytes()) for n in names},original_execution=False,native_capture=False,full_gameplay=False)

if __name__=='__main__':
    import argparse
    p=argparse.ArgumentParser();p.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux');p.add_argument('--host',type=Path);p.add_argument('--output',type=Path);a=p.parse_args();result=audit(a.assets,a.host or a.assets/'Civ6');raw=json.dumps(result,indent=2)+'\n'
    if a.output:a.output.write_text(raw)
    else:print(raw,end='')
