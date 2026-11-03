#!/usr/bin/env python3
"""Sole-runner debugger launch/disposal for a separately admitted Manager case.
No original execution occurs on import; run requires the exact prepared profile.
"""
import json
import os
from pathlib import Path
import subprocess
import time
from military_observer_runner import process_identity,session_processes,dispose_session,require,sha
ROOT=Path(__file__).resolve().parents[2]

def run(host,env,work,timeout,directory,prepared,plan):
    """Called by the identity-gated runner, after its expected-profile gate."""
    directory=Path(directory);require(directory.is_dir(),'observer directory absent')
    capture_directory=directory/'capture';require(not capture_directory.exists(),'observer capture directory must be fresh')
    prepared_data=json.loads(Path(prepared).read_text());cfg=prepared_data['configuration']
    source_files={str((ROOT/name).resolve()):value for name,value in cfg['sources'].items()};source_files[str((ROOT/'tools/linux/mod_activation_probe.py').resolve())]=cfg['runner_sha256']
    gate=dict(schema='c6-linux-full-manager-caller-runtime-v1',report_path=str(prepared),report_sha256=sha(prepared),configuration_sha256=prepared_data['configuration_sha256'],source_files=source_files,static_plan_path=str(Path(plan).resolve()))
    manifest=json.loads(Path(plan).read_text());require(manifest==cfg['lower_core_full_manager_observer']['static_plan'] and sha(plan)==cfg['lower_core_full_manager_observer']['static_plan_sha256'],'static plan drift after admission');manifest['runtime_gate']=gate
    runtime_plan=directory/'runtime-plan.json';runtime_plan.write_text(json.dumps(manifest,indent=2)+'\n')
    command=directory/'capture.gdb';lines=['set pagination off','set confirm off','set debuginfod enabled off','set breakpoint pending on','set non-stop off','set startup-with-shell off']
    for key,value in env.items():
        if not key.startswith(('C6_','LD_')):continue
        require('\n' not in key+value and '\r' not in key+value,'unsafe environment newline')
        lines.append('set environment '+key+' '+value)
    module=str(ROOT/'tools/linux/full_manager_observer_runner.py')
    lines+=['starti','python',"import sys; sys.path.insert(0,"+repr(str(ROOT/'tools/linux'))+")",'import full_manager_observer_runner', 'full_manager_observer_runner.install('+repr(str(runtime_plan))+','+repr(str(capture_directory))+')','end','c6-full-manager-drain']
    command.write_text('\n'.join(lines)+'\n')
    child_env={k:v for k,v in os.environ.items() if not k.startswith(('C6_','LD_'))}
    transcript=directory/'transcript.txt'
    process=None;identity=None;status={};owned={}
    try:
        with transcript.open('x') as stream:
            process=subprocess.Popen(['gdb','-nx','-batch','-x',str(command),'--args',str(host)],cwd=work,env=child_env,stdout=stream,stderr=subprocess.STDOUT,start_new_session=True)
            identity=process_identity(process.pid);require(identity and identity['session']==process.pid,'debugger session not established')
            (directory/'supervisor.json').write_text(json.dumps(dict(launched=identity,host_executable=str(host)))+'\n')
            deadline=time.monotonic()+timeout
            try:
                while True:
                    current=process_identity(process.pid)
                    if current is not None:
                        require(all(current[k]==identity[k] for k in ('pid','starttime','session','exe')),'debugger session leader changed')
                        for row in session_processes(process.pid):owned[(row['pid'],row['starttime'])]=row
                    remaining=deadline-time.monotonic()
                    if remaining<=0:raise subprocess.TimeoutExpired(process.args,timeout)
                    try:process.wait(timeout=min(1.0,remaining));break
                    except subprocess.TimeoutExpired:pass
            except subprocess.TimeoutExpired:
                status=dispose_session(process,identity,'supervisor_timeout',owned.values())
                (directory/'disposal.json').write_text(json.dumps(status,indent=2)+'\n')
                return 124,'',transcript.read_text(errors='replace')
        status=json.loads((capture_directory/'exit.json').read_text()) if (capture_directory/'exit.json').exists() else {}
        alive=[row for row in session_processes(process.pid) if row['state'] not in ('Z','X')]
        if process.returncode!=0 or status.get('observer_failed',True) or alive:
            disposal=dispose_session(process,identity,'debugger_failure_or_remaining_inferior',owned.values())
            (directory/'disposal.json').write_text(json.dumps(disposal,indent=2)+'\n')
        status.update(gdb_returncode=process.returncode,remaining_session_processes=session_processes(process.pid))
        (directory/'supervisor.json').write_text(json.dumps(dict(launched=identity,terminal=status),indent=2)+'\n')
        code=status.get('inferior_exitcode') if process.returncode==0 and not status.get('observer_failed',True) and not alive else 239
        if type(code) is not int:code=239
        return code,'',transcript.read_text(errors='replace')
    except BaseException:
        if process is not None and identity is not None:
            disposal=dispose_session(process,identity,'caller_exception',owned.values())
            (directory/'disposal.json').write_text(json.dumps(disposal,indent=2)+'\n')
        raise
