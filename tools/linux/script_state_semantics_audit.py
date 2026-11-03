#!/usr/bin/env python3
"""Pinned Linux scripting semantics from original source bytes + closed native2.
No original execution, Lua calls, locks, heap traversal or Rust replay backend.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import struct
import unittest
from save_lifecycle_audit import Elf,HOST_SHA,GAME_SHA
from script_vm_lifecycle_audit import METHODS as LIFECYCLE
from script_can_start_boundary_audit import METHODS as SCRIPT
from full_manager_raw_capture_audit import SEAL_SHA,RAW_SHA,RUNTIME_SHA

ROOT=Path(__file__).resolve().parents[2]
CAPTURE=ROOT.parent/'c6-ghidra/evidence/linux-full-manager-predicate/native-2'
EXTRA=[('vm_call_internal',0x3bc2945,0x564),('runProtected',0x3bd4a89,0x7f),
 ('hksi_lua_pcall',0x3b98d03,0x1c7),('ArgsCount',0x3245b40,9),('ArgsGetInt',0x3245d10,0x51),
 ('ArgsVectorCtor',0x3246670,0x42),('ArgsVectorSize',0x3246750,7),('ArgsVectorAt',0x3246810,0x1a),
 ('VariantSetInt',0x31d5a10,15),('ArrayVariantSetInt',0x31d5a20,11),('TypedVariantGetValue',0x31a2d10,5)]
# New complete spans are gated by the whole target module and preserved per-span
# below; no cross-version or Windows equality is inferred.


def sha(raw):return hashlib.sha256(raw).hexdigest()
def require(value,message):
    if not value:raise ValueError(message)


def closed_facts(rows):
    contexts=[row for row in rows if row['event']=='raw_actual_CCall_context']
    require([row['checkpoint'] for row in contexts]==['ccall_entry','acquired','before_release'],'closed raw phases')
    headers=[bytes.fromhex(row['actual_CCall_state_header_raw']) for row in contexts]
    context=[bytes.fromhex(row['context_raw26']) for row in contexts]
    call=next(row for row in rows if row['event']=='raw_actual_CallTestAll_return')
    resolved=next(row for row in rows if row['event']=='checkpoint' and row['checkpoint']=='resolved')
    regs=resolved['registers'];entry=call['entry_rsp']
    require(regs['pc']==0x36b3c44 and regs['rbx']==call['state'] and regs['r14']==call['context'],'actual resolved context/state identity')
    ancestors=[row for row in resolved['frames'] if row['pc']==0x3bf2308]
    require(len(ancestors)==1,'selected original protected CCall ancestor')
    require(all(struct.unpack_from('<QQQ',raw)==(call['event_pointer'],0x462e7b4,call['args']) for raw in context),
            'actual CCall event/method/args context continuity')
    require(call['protected_status_raw_u32']==0 and call['protected_success_raw_u8']==1 and
            call['return_bit'] is False and call['out_value_raw']=='00','actual protected/wrapper result')
    require(context[0][24]==context[1][24]==context[2][24]==0 and context[2][25]==0,'actual non-nil flag/truthiness byte')
    # Never parse initial byte25 as bool: it is uninitialized storage (99 here).
    return dict(actual_callable_success_path_PC=hex(regs['pc']),actual_state_context_registers_match=True,
        actual_selected_CCall_ancestor_PC='0x3bf2308',raw_producer_label='unattributed_native_checkpoint (preserved)',
        narrow_source_attribution='Same retained CCall context/state + actual protected ancestor and pinned body establish the selected callable path; no generic HKS-stack prediction.',
        non_nil_flag_before=0,non_nil_flag_after=0,initial_uninitialized_truthiness_raw_u8=context[0][25],
        final_truthiness_raw_u8=context[2][25],protected_status=0,protected_success=1,
        source_backed_result='The actually called function produced the requested nil result; this is not a false Lua result or proof of zero callbacks.',
        actual_Method_TestAll_builtin_identity_known=False,callback_registry_known=False,
        main_global_HksObject_raw_sha256=sha(headers[1][0x70:0x80]),
        captured_outer_top_equals_base=all(struct.unpack_from('<Q',raw,0x48)[0]==struct.unpack_from('<Q',raw,0x50)[0] for raw in headers),
        resolved_state_header_captured=False,resolved_top_TValue_captured=False,
        actual_args_vector_bytes_captured=False,pointed_pthread_mutex_bytes_captured=False,
        raw_header_pointee_facts='State+0x10 global-state pointer and +0x70 table pointer exist in copied bytes; their pointees were not copied.',
        exact_missing=['Fresh resolved-state header and top[-1] HksObject, actual closure/function/environment/upvalues',
          'Actual raw global/event/method tables and shared callback-group/array storage with weak/order facts',
          'Live ScriptSystemArgs object and active 24-byte TypedVariant cells',
          '40-byte pointed pthread mutex storage; original return-status ownership proof remains separate',
          'Collector prefix/control bytes and callback Lua Method/prototype/effects where present; preserve unknowns'])


class Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        raw=(CAPTURE/'observer/capture/observations.jsonl').read_bytes()
        require(sha(raw)==RAW_SHA,'closed test preimage identity')
        cls.rows=[json.loads(line) for line in raw.splitlines()]
    def test_wrong_selected_context_refused(self):
        rows=copy.deepcopy(self.rows)
        next(row for row in rows if row['event']=='checkpoint' and row['checkpoint']=='resolved')['registers']['r14']+=8
        with self.assertRaisesRegex(ValueError,'context/state'):
            closed_facts(rows)
    def test_false_Lua_result_cannot_be_replayed_as_nil(self):
        rows=copy.deepcopy(self.rows)
        final=next(row for row in rows if row['event']=='raw_actual_CCall_context' and row['checkpoint']=='before_release')
        raw=bytearray.fromhex(final['context_raw26']);raw[24]=1;final['context_raw26']=raw.hex()
        with self.assertRaisesRegex(ValueError,'non-nil flag'):
            closed_facts(rows)
    def test_missing_protected_ancestor_refused(self):
        rows=copy.deepcopy(self.rows)
        row=next(row for row in rows if row['event']=='checkpoint' and row['checkpoint']=='resolved')
        row['frames']=[frame for frame in row['frames'] if frame['pc']!=0x3bf2308]
        with self.assertRaisesRegex(ValueError,'protected CCall ancestor'):
            closed_facts(rows)


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    parser.add_argument('--capture',type=Path,default=CAPTURE);parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();host=Elf(args.assets/'Civ6',HOST_SHA);Elf(args.assets/'libGameCore_XP2.so',GAME_SHA)
    checked=[]
    for module,name,address,size,expected in LIFECYCLE:
        if module=='host' and name in ('InitializeCriticalSection','EnterCriticalSection','LeaveCriticalSection','ASL::GetCurrentThreadHANDLE()','GetCurrentThreadId'):
            require(sha(host.read(address,size))==expected,'existing lock source span differs')
            checked.append(dict(name=name,address=hex(address),size=size,sha256=expected))
    for name,address,size,expected in SCRIPT:
        if name in ('GameEventLibrary.protected_named_event_body','LuaSupport::GameEventLibrary::CallTestAll(lua_State*, char const*, GameCore::Interfaces::IScriptSystemArgs1*, bool&)'):
            require(sha(host.read(address,size))==expected,'existing script source span differs')
            checked.append(dict(name=name,address=hex(address),size=size,sha256=expected))
    for name,address,size in EXTRA:
        checked.append(dict(name=name,address=hex(address),size=size,sha256=sha(host.read(address,size))))
    # Exact observed call returns and critical field stores.
    require(host.read(0x3bc2aa6,2)==bytes.fromhex('ffd3'),'observed C callback indirect call source')
    require(host.read(0x2ce0c32,5)==bytes.fromhex('bf28000000'),'allocated mutex storage size source')
    require(host.read(0x2ce0d4a,4)==bytes.fromhex('49894610'),'ASL pointer assignment source')
    require(host.read(0x2cdf95a,2)==bytes.fromhex('31db'),'absent registry handle returns null source')
    require(host.read(0x36b3c40,4)==bytes.fromhex('85c07453'),'post-isfunction branch source')
    root=args.capture
    require(sha((root/'closed-files.json').read_bytes())==SEAL_SHA and
            sha((root/'runtime.json').read_bytes())==RUNTIME_SHA and
            sha((root/'observer/capture/observations.jsonl').read_bytes())==RAW_SHA,'closed native2 identity')
    rows=[json.loads(line) for line in (root/'observer/capture/observations.jsonl').read_text().splitlines()]
    facts=closed_facts(rows)
    result=unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(Tests));require(result.wasSuccessful(),'semantic boundary tests')
    report=dict(schema='c6-linux-script-state-semantics-v1',source_checks_passed=True,original_execution=False,
        host_sha256=HOST_SHA,gamecore_sha256=GAME_SHA,checked_source_spans=checked,closed_native2=facts,
        Lua_lock=dict(address='0x6ec9c70',raw_object_size=64,count_offset=8,recursion_offset=12,
          ASL_ThreadHANDLE_pointer_offset=16,allocated_pthread_mutex_pointer_offset=24,pointed_mutex_size=40,
          mutex_storage_inline=False,ASL_handle_null_when='TLS numeric-ID registry lookup absent (or stored handle null); not a mutex-state test',
          GetCurrentThreadHANDLE_inspection_call_forbidden=True,
          Enter_semantics='Increment bookkeeping; original try/block pthread call; increment recursion; on 0→1 store actual registry lookup pointer, including null.',
          blocking_pthread_status_tested_by_original=False,zero_handle_missing_lock_inference=False,
          ASL_registration_prerequisite_for_passive_copy=False,OSTID_conversion_claimed=False),
        passive_extension=dict(natural_PC='0x36b3c44 before original PushScriptSystemArgs',
          selected_context='Actual state/r14 context plus retained selected CCall ancestor rsp/regs; no full HKS prefix prediction.',
          no_new_native_calls_or_locks=True,GC_classification='unresolved',
          collector_bytes='Global-state+0x58 prefix, retained unclassified; no idle inference.',
          args_layout=dict(object_size=440,data_pointer_offset=416,capacity_u32_offset=424,size_u32_offset=432,
                           cell_size=24,type_u16_offset=0,signed_int_type=2,signed_int_payload_offset=8),
          registry_decoder='Existing immutable Image/Table/closure + actual resolved top TValue reader; unsupported layouts preserve partial images.'),
        Rust_provider_added=False,constant_or_replayed_script_backend_added=False,callback_behavior_supported=False,
        complete_state_actions_rng_turns=False,historical_raw_save_load_difference_bytes=89198,unknown_bytes_masked=0,
        source_sha256=sha(Path(__file__).read_bytes()))
    args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')


if __name__=='__main__':main()
