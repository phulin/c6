#!/usr/bin/env python3
"""Closed-file raw Manager/CCall attribution; no original code or VM decoding."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import struct
from save_lifecycle_audit import Elf, HOST_SHA

ROOT=Path(__file__).resolve().parents[2]
CAPTURE=ROOT.parent/'c6-ghidra/evidence/linux-full-manager-predicate/native-2'
SEAL_SHA='449c875d1f45d01257ddfc59ded6b1690f10e9a2c76132f9171903a599575822'
RUNTIME_SHA='255603fc41c1768a5da113beeb94b14aa28010da837e71bb9540fd7b16028903'
RAW_SHA='f13e5dda8ddbfe2bc4adea6f0f1c07df0eec226e0e6528b1f6c375bd18ecbe27'
EXPECTED=['marker']*4+['manager_entry','dispatch','ccall_entry','acquired','resolved','protected_return',
 'before_release','released','wrapper_return','manager_return']+['marker']*5
HOST_PC=dict(ccall_entry=0x3bf22e0,acquired=0x3bf22fa,resolved=0x36b3c44,
 protected_return=0x3bf2308,before_release=0x3bf2329,released=0x3bf2335)


def require(value,reason):
    if not value:raise ValueError(reason)


def digest(path):
    result=hashlib.sha256()
    with Path(path).open('rb') as stream:
        while block:=stream.read(1024*1024):result.update(block)
    return result.hexdigest()


def one(rows,event):
    found=[row for row in rows if row['event']==event]
    require(len(found)==1,'expected one '+event)
    return found[0]


def parsed_marker(raw):
    require(len(raw)==88,'raw fixture descriptor extent')
    result=dict(zip(('module','context','game','facade','manager','unit','operation','core_base'),struct.unpack_from('<8Q',raw,8)))
    result.update(phase=struct.unpack_from('<I',raw)[0],actor_owner=struct.unpack_from('<i',raw,72)[0],
                  operation_id=struct.unpack_from('<i',raw,76)[0],actor_full_id=struct.unpack_from('<I',raw,80)[0],
                  visible_only=bool(raw[84]),predicate_returned=bool(raw[85]),allowed=bool(raw[86]))
    require(all(value in (0,1) for value in raw[84:87]),'noncanonical raw marker bool')
    return result


def classify(rows,predicate,plan,profile):
    require([row['checkpoint'] for row in rows if row['event']=='checkpoint']==EXPECTED,'ordered native checkpoint sequence')
    require(not any(row['event'] in ('runner_refused','raw_trace_failure','observer_failure','partial_snapshot') for row in rows),
            'refused or partial raw capture')
    markers=[row for row in rows if row['event']=='fixture_marker']
    require([row['phase'] for row in markers]==list(range(1,10)),'ordered original fixture markers')
    for row in markers:
        require(all(row[key]==value for key,value in parsed_marker(bytes.fromhex(row['raw88'])).items()),'raw descriptor fields disagree')
    enter_marker,return_marker=markers[3:5];base=markers[0]['core_base']
    for row in markers:
        require(all(row[key]==markers[0][key] for key in ('module','context','game','core_base','actor_owner','actor_full_id','visible_only')),
                'retained owner/input identity changed')
    require(all(enter_marker[key]==return_marker[key] for key in ('manager','unit','operation','operation_id')),'enter/return address tokens changed')
    require(markers[-1]['facade']==0 and all(row['unit']==row['operation']==0 for row in markers[5:]),'released or post-call tokens not cleared')
    checkpoints=[row for row in rows if row['event']=='checkpoint']
    points={row['checkpoint']:row for row in checkpoints if row['checkpoint']!='marker'}
    thread=one(rows,'actual_main_entry_thread_witness')['os_main_tid']
    require(all(row['thread']==[thread,thread,0] for row in checkpoints),'actual observing thread changed')
    reg={name:row['registers'] for name,row in points.items()}
    require(reg['manager_entry']['pc']==base+0x2da20f0 and reg['dispatch']['pc']==base+0x2ce79e9 and
            reg['wrapper_return']['pc']==base+0x2ce79ff,'original Core PCs differ')
    require(all(reg[name]['pc']==pc for name,pc in HOST_PC.items()),'original host checkpoint PC differs')
    require((reg['manager_entry']['rdi'],reg['manager_entry']['rsi'],reg['manager_entry']['rdx'],
             reg['manager_entry']['rcx'],reg['manager_entry']['r8']&255,reg['manager_entry']['r9'])==
            (enter_marker['manager'],enter_marker['unit'],enter_marker['operation'],0,int(enter_marker['visible_only']),0),
            'actual six-argument Manager ABI mismatch')
    wrapper=reg['dispatch']['rsp'];entry=reg['ccall_entry']['rsp']
    require(wrapper==reg['manager_entry']['rsp']-152 and entry==wrapper-72,'selected Manager/wrapper/CCall stack mismatch')
    require(all(reg[name]['rsp']==entry-24 for name in ('acquired','protected_return','before_release','released')) and
            reg['wrapper_return']['rsp']==wrapper and reg['manager_return']['rsp']==reg['manager_entry']['rsp']+8,
            'actual native return stack mismatch')
    called=one(rows,'raw_actual_CallTestAll_return')
    require(called['wrapper_rsp']==wrapper and called['entry_rsp']==entry and called['thread']==[thread,thread,0],
            'recorded CCall frame mismatch')
    require(called['state']==reg['ccall_entry']['rdi']==reg['acquired']['rbx'] and
            called['context']==reg['ccall_entry']['rdx']==reg['acquired']['r14'] and
            reg['ccall_entry']['rsi']==reg['acquired']['r15']==0x36b3bc0,
            'actual protected state/context/body identity mismatch')
    require(called['args']==reg['dispatch']['r15'] and called['out']==reg['dispatch']['r14'] and
            called['event_pointer']==reg['dispatch']['r12']==base+0x2f0bac6,'native args/out/event identity mismatch')
    contexts=[row for row in rows if row['event']=='raw_actual_CCall_context']
    require([row['checkpoint'] for row in contexts]==['ccall_entry','acquired','before_release'],'raw context checkpoint sequence')
    header_hashes=[]
    for row in contexts:
        raw=bytes.fromhex(row['context_raw26']);header=bytes.fromhex(row['actual_CCall_state_header_raw'])
        require(len(raw)==26 and len(header)==128 and struct.unpack_from('<QQQ',raw)==
                (called['event_pointer'],0x462e7b4,called['args']),'raw CCall context/header mismatch')
        require(row['registry_decoded'] is False and row['initialized_main_VM_or_GC_safe_proven'] is False,'raw state promoted to VM proof')
        header_hashes.append(hashlib.sha256(header).hexdigest())
    require(called['protected_status_raw_u32']==reg['protected_return']['rax']&0xffffffff==0 and
            called['protected_success_raw_u8']==reg['before_release']['rbx']&255==1,'actual protected status/success mismatch')
    require(called['return_bit']==bool(reg['wrapper_return']['rax']&255)==False and called['out_value_raw']=='00',
            'actual CallTestAll return/out mismatch')
    actual=one(rows,'actual_Manager_return')
    require(actual['allowed']==bool(reg['manager_return']['rax']&255)==return_marker['allowed']==True and
            actual['natural_script_call_observed'] is True,'actual Manager AL mismatch')
    unattributed=one(rows,'unattributed_native_checkpoint')
    require(unattributed['checkpoint']=='resolved' and unattributed['registers']==reg['resolved'],
            'resolved point must remain explicitly unattributed')
    require(reg['resolved']['rsp']!=entry-24,'resolved HKS depth unexpectedly treated as CCall frame')
    lock_rows=([one(rows,'before_original_Lua_acquisition'),one(rows,'original_Lua_Enter_returned')]+
        [row for row in rows if row['event']=='raw_original_Lua_release_endpoint'])
    require(len(lock_rows)==4,'lock endpoint count')
    for row in lock_rows:
        raw=bytes.fromhex(row['raw64']);require(len(raw)==64,'raw Lua critical section extent')
        require((row['count'],row['recursion'],row['owner_ASL_ThreadHANDLE_pointer'],row['pthread_mutex_pointer'])==
                (struct.unpack_from('<i',raw,8)[0],struct.unpack_from('<i',raw,12)[0],
                 struct.unpack_from('<Q',raw,16)[0],struct.unpack_from('<Q',raw,24)[0]),'raw lock fields differ')
    require([row['count'] for row in lock_rows]==[-1,0,0,-1] and [row['recursion'] for row in lock_rows]==[0,1,1,0],
            'observed Lua lock count/recursion sequence differs')
    require(all(row['owner_ASL_ThreadHANDLE_pointer']==0 for row in lock_rows) and
            len({row['pthread_mutex_pointer'] for row in lock_rows})==1 and lock_rows[0]['pthread_mutex_pointer']!=0,
            'observed unresolved ASL owner/mutex facts differ')
    require(one(rows,'raw_trace_terminal')['VM_GC_parent_classification']=='unresolved','VM/GC classification promoted')
    terminal=one(rows,'adapter_terminal')
    require(terminal['natural_Lua_calls']==1 and terminal['observation_mode']=='raw_unresolved' and
            terminal['registry_images_captured'] is terminal['callback_effects_accepted'] is terminal['native_parity_accepted'] is False,
            'raw terminal scope promoted')
    require(not any(row.get('callback_absence_proven') is True for row in rows),'callback absence falsely established')
    prefixes=[row for row in rows if row['event']=='actual_protected_parent_prefix']
    require(len(prefixes)==8 and all(row['source_closed'] is False and row['registry_decoding_enabled'] is False for row in prefixes),
            'raw parent-prefix classification promoted')
    header=next(row for row in predicate if row['kind']=='header');summary=next(row for row in predicate if row['kind']=='summary')
    require(header['profile_sha256']==profile and header['actor_owner']==enter_marker['actor_owner'] and
            header['actor_full_id']==enter_marker['actor_full_id'] and header['visible_only']==enter_marker['visible_only'],
            'fixture profile/actor inputs disagree')
    require(summary['predicate_invoked'] is summary['predicate_returned'] is summary['allowed'] is summary['facade_released'] is True,
            'fixture summary differs from actual native returns')
    core=[row for row in predicate if row['kind']=='core_lock_endpoint']
    require([row['phase'] for row in core]==list(range(1,10)) and
            [struct.unpack('<II',bytes.fromhex(row['owner_u32_depth_u32_raw'])) for row in core]==[(1,1)]*8+[(0,0)],
            'separate core ownership endpoint facts differ')
    return dict(checkpoint_count=19,fixture_markers=9,natural_CallTestAll_count=1,
        actor=dict(owner=header['actor_owner'],full_id=header['actor_full_id'],visible_only=header['visible_only'],runtime_operation_id=enter_marker['operation_id']),
        actual_Manager_AL=True,actual_CallTestAll_return_bit=False,actual_out_byte='00',protected_status=0,protected_success_byte=1,
        source_only_argument_order=['current owner','current Unit ID bits as signed i32','current runtime operation ID'],
        pushed_argument_vector_bytes_captured=False,operation_token_stable_at_enter_return=True,
        native_constructor_destructor_breakpoints_observed=False,
        raw_state_header_sha256=header_hashes,unattributed_resolved_checkpoints=1,unresolved_parent_prefixes=8,
        observed_resolved_PC='0x36b3c44',resolved_raw_state_context_registers_match=(
            reg['resolved']['rbx']==called['state'] and reg['resolved']['r14']==called['context']),
        before_release_context_non_nil_flag_raw_u8=bytes.fromhex(contexts[-1]['context_raw26'])[24],
        Lua_lock_counts=[-1,0,0,-1],Lua_lock_recursion=[0,1,1,0],Lua_ASL_owner_pointer_values=[0]*4,
        Lua_mutex_pointer_stable=True,Lua_owner_classification='unresolved',VM_GC_classification='unresolved',
        separate_core_owner_depth_phases_1_to_8=[1,1],separate_core_owner_depth_phase_9=[0,0],
        core_owner_equals_OS_TID_claimed=False,core_owner_equals_Lua_ASL_handle_claimed=False,
        callback_registry_decoded=False,installed_TestAll_closure_identity_captured=False,callback_absence_proven=False,
        callback_effects_accepted=False,complete_RNG_or_VM_state=False,full_manager_native_parity=False)


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--capture',type=Path,default=CAPTURE)
    parser.add_argument('--output',type=Path,required=True);args=parser.parse_args();root=args.capture.resolve()
    require(digest(root/'closed-files.json')==SEAL_SHA,'closed seal identity mismatch')
    seal=json.loads((root/'closed-files.json').read_text());require(len(seal['files'])==129,'sealed file count')
    for name,identity in seal['files'].items():
        path=root/name;require(path.resolve().is_relative_to(root),'sealed relative path escaped')
        require(path.stat().st_size==identity['bytes'] and digest(path)==identity['sha256'],'sealed file changed: '+name)
    require(digest(root/'runtime.json')==RUNTIME_SHA and digest(root/'observer/capture/observations.jsonl')==RAW_SHA,'closed runtime/raw identity')
    runtime=json.loads((root/'runtime.json').read_text());profile=runtime['configuration_sha256']
    canonical=json.dumps(runtime['configuration'],sort_keys=True,separators=(',',':')).encode()
    require(hashlib.sha256(canonical).hexdigest()==profile==seal['configuration_sha256'],'configuration identity')
    require(runtime['returncode']==0,'closed original result not zero')
    for name,expected in runtime['configuration']['sources'].items():
        require(digest(root/'source-preimages'/name)==expected,'retained producer/source preimage mismatch: '+name)
    require(digest(root/'source-preimages/tools/linux/mod_activation_probe.py')==runtime['configuration']['runner_sha256'],'retained runner source mismatch')
    rows=[json.loads(line) for line in (root/'observer/capture/observations.jsonl').read_text().splitlines()]
    predicate=[json.loads(line) for line in (root/'predicate.jsonl').read_text().splitlines()]
    plan=json.loads((root/'observer-plan.json').read_text());facts=classify(rows,predicate,plan,profile)
    host=Elf(Path(plan['host']),HOST_SHA)
    require(hashlib.sha256(host.read(0x36b3bc0,225)).hexdigest()==
        'e4a1ed2a09e18c21f82b023210d4eeb35f4f9574319ec11e2468e5a48f4ea6af','protected body source contract')
    require(host.read(0x36b3c40,4)==bytes.fromhex('85c07453'),'resolved function-test branch source contract')
    exit_row=json.loads((root/'observer/capture/exit.json').read_text());supervisor=json.loads((root/'observer/supervisor.json').read_text())['terminal']
    require(exit_row['inferior_exitcode']==0 and exit_row['observer_failed'] is False and supervisor['gdb_returncode']==0 and
            supervisor['remaining_session_processes']==[],'retained observer/process terminal differs')
    # In-memory minimized challenges never change sealed files or rerun originals.
    mutations=[]
    for kind in ('AL','checkpoint','ASL_owner','registry','context'):
        changed=copy.deepcopy(rows)
        if kind=='AL':one(changed,'actual_Manager_return')['allowed']=False
        elif kind=='checkpoint':next(row for row in changed if row['event']=='checkpoint')['checkpoint']='wrong'
        elif kind=='ASL_owner':one(changed,'original_Lua_Enter_returned')['owner_ASL_ThreadHANDLE_pointer']=1
        elif kind=='registry':one(changed,'adapter_terminal')['registry_images_captured']=True
        else:next(row for row in changed if row['event']=='raw_actual_CCall_context')['context_raw26']='00'*26
        try:classify(changed,predicate,plan,profile)
        except ValueError:mutations.append(kind)
        else:raise ValueError('mutation accepted: '+kind)
    report=dict(schema='c6-linux-full-manager-raw-capture-v1',checks_passed=True,original_execution_by_checker=False,
        capture_path=str(root),seal_sha256=SEAL_SHA,raw_observations_sha256=RAW_SHA,runtime_sha256=RUNTIME_SHA,
        configuration_sha256=profile,sealed_files_verified=129,raw_facts=facts,mutations_refused=mutations,
        producer_source_sha256=digest(root/'source-preimages/tools/linux/full_manager_observer_runner.py'),
        classifier_sha256=digest(Path(__file__)),
        source_guarantees_separate_from_raw=['Pinned protected-body branch makes observed PC0x36b3c44 the post-lua_isfunction success path. The producer still leaves this generic HKS checkpoint unattributed; no closure identity is established.','Pinned source supplies Operation32/alignment8 and native ctor/dtor ordering; markers retain only live address tokens.',
         'Argument push order is source-backed; actual ScriptSystemArgs vector and callback closure/group storage were not copied.',
         'Original PC/source path is observed; raw Lua owner pointer is zero despite recursion one, so owner/GC/VM proof remains unresolved.'],
        limits=['One actual original predicate boundary observation, no Rust/native full Manager comparison or action execution acceptance.',
         'resolved PC is explicitly unattributed; no installed method/closure/registry identity or empty-callback claim.',
         'No heap/registry traversal, collector snapshot, callback effects or complete RNG/VM state.',
         'Whole-profile effects, diagnostic Save comparisons and native lifecycle review belong to separate reports.'],
        complete_state_actions_rng_turns=False,historical_raw_save_load_difference_bytes=89198,unknown_save_bytes_masked=0)
    args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')


if __name__=='__main__':main()
