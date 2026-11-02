#!/usr/bin/env python3
"""Validate actual original controller/cache readiness and native teardown control."""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from save_lifecycle_audit import GAME_SHA, HOST_SHA


def digest(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime-report',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    run=json.loads(args.runtime_report.read_text())
    identity=hashlib.sha256(json.dumps(run['configuration'],sort_keys=True,separators=(',',':')).encode()).hexdigest()
    if identity!=run['configuration_sha256']:raise ValueError('configuration identity mismatch')
    assets=json.dumps(run['configuration']['assets'])
    if GAME_SHA not in assets or HOST_SHA not in assets:raise ValueError('original module gates missing')
    if run['returncode'] or not run['framework_cleanup_proven'] or not run['client_cleanup_proven']:raise ValueError('native lifecycle or cleanup failed')
    rows=[json.loads(x.split(' ',1)[1])for x in run['stderr'].splitlines()if x.startswith('C6_LINUX_ACTION_EXECUTION {')]
    if len(rows)!=2:raise ValueError('requires exactly one host/cache observation')
    host,cache=rows
    if host['stage']!='host_delivery_observation' or cache['stage']!='cache_dispatch_observation':raise ValueError('observation order mismatch')
    for key in ('controller_present','controller_initialized','controller_context_matches','host_context_matches','network_gamecore_active'):
        if host[key]!=1:raise ValueError('native host prerequisite failed: '+key)
    for key in ('live_active','live_human','cache_present','cached_active','cached_human','request_cache_predicate_passes'):
        if cache[key]!=1:raise ValueError('native cache prerequisite failed: '+key)
    if len({cache[x]for x in ('local_player','live_player','cached_player')})!=1:raise ValueError('actual player identities differ')
    marker='C6_LINUX_ACTION_LIFECYCLE uninitialize_return initialized 0 context_bound 0 executing 0 cleanup 1 action_executed 0'
    new_marker='C6_LINUX_ACTION_LIFECYCLE uninitialize_return initialized 0 context_bound 0 executing 0 cleanup 1 dispatch_mode 0 requested_execution 0'
    if marker not in run['stderr'] and new_marker not in run['stderr']:raise ValueError('original Uninitialize cleanup missing')
    trace_path=Path(run['execution_paths']['action_trace_directory'])/'events.jsonl'
    trace=[json.loads(x)for x in trace_path.read_text().splitlines()]
    if not trace[0]['UnitOperation_Execute_virtual_interposed']:raise ValueError('native action virtual binding missing')
    bindings=[x for x in trace if x['event']=='original_PLT_binding']
    if len(bindings)not in (11,12,13) or not all(x['interposed'] for x in bindings):raise ValueError('original trace/guard PLT bindings differ')
    end=trace[-1]
    if end['event']!='end' or any(end[k] for k in ('queued','received','unit_executed','reports')):raise ValueError('initialization control unexpectedly executed messages/reports')
    saves={}
    for key in ('save','repeat'):
        path=Path(run['execution_paths']['game_outputs'][key]);value=dict(bytes=path.stat().st_size,sha256=digest(path))
        if any(run['game_saves'][key][k]!=v for k,v in value.items()):raise ValueError('actual original Save identity differs')
        saves[key]=value
    if saves['save']!=saves['repeat']:raise ValueError('serialized Core changed in initialization control')
    entries=[];module_symbols={};module_hashes={}
    for line in run['stderr'].splitlines():
        if 'C6_LINUX_ACTION_TRACE unarmed_guarded_entry' in line or 'C6_LINUX_ACTION_TRACE guarded_entry' in line:
            raise ValueError('unresolved guarded entry observed before or during native control')
        if not line.startswith('C6_LINUX_ACTION_TRACE unarmed_entry '):continue
        fields=line.split()
        if len(fields)!=10:raise ValueError('malformed successful unarmed entry')
        symbol=fields[3];caller=Path(fields[5]).resolve();original=Path(fields[7]).resolve();rva=int(fields[9],16)
        if caller!=original:raise ValueError('unarmed delegate DSO differs from caller DSO')
        if original not in module_hashes:
            module_hashes[original]=digest(original)
            if module_hashes[original]!=run['configuration']['assets'].get(original.name):
                raise ValueError('unarmed original DSO content gate differs')
            symbols={}
            for row in subprocess.check_output(['nm','-D','-S',str(original)],text=True).splitlines():
                columns=row.split()
                if len(columns)==4:symbols[columns[3]]=int(columns[0],16)
            module_symbols[original]=symbols
        if module_symbols[original].get(symbol)!=rva:raise ValueError('unarmed delegate symbol/RVA differs')
        entries.append(dict(symbol=symbol,module=original.name,module_sha256=module_hashes[original],rva=hex(rva)))
    unarmed_complete=bool(entries) and len(bindings)>=12
    report=dict(schema='c6-linux-action-lifecycle-control-v1',runtime_report_sha256=digest(args.runtime_report),configuration_sha256=identity,
                host_sha256=HOST_SHA,gamecore_sha256=GAME_SHA,host_observation=host,cache_observation=cache,
                trace_sha256=digest(trace_path),trace_plt_bindings=bindings,trace_end=end,original_core_saves=saves,
                serialized_core_unchanged=True,original_uninitialize_before_unload=True,full_cleanup=True,
                action_executed=False,unarmed_entry_log_complete=unarmed_complete,successful_unarmed_entries=entries,complete_policy_boundary_coverage=False,gameplay_parity=False,
                limits='Actual original controller/cache initialization and teardown only. Selected guards verified in the armed interval. Successful unarmed entries, when present, are checked against original DSO hashes and actual symbol RVAs; no complete transient-state or RNG comparison is claimed here.')
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(native_prerequisites_pass=True,serialized_core_unchanged=True,action_executed=False)))


if __name__=='__main__':main()
