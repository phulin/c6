#!/usr/bin/env python3
"""Source gates and synthetic scheduling/unwind checks; no original execution."""
import argparse
import ast
import contextlib
import hashlib
import io
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest
from unittest import mock
from types import SimpleNamespace
import sys
import full_manager_observer_runner as producer

from full_manager_observer_runner import Schedule,Frames,descriptor,mandatory_bodies,ROOT,REGISTERS
from save_lifecycle_audit import Elf,GAME_SHA,HOST_SHA

SOURCE='tools/linux/full_manager_observer_runner.py'


def frame(module,pc):
    return dict(module=module,relative_pc=pc)


class Tests(unittest.TestCase):
    def ready(self):
        schedule=Schedule()
        for phase in range(1,5):
            schedule.marker(dict(phase=phase))
        schedule.enter(dict(rsp=0x1000),0x2222)
        return schedule

    def test_early_manager_false_is_not_callback_absence(self):
        schedule=self.ready()
        self.assertEqual(schedule.points(0x10000000),[('manager_return',0x2222),('dispatch',0x12ce79e9)])
        schedule.returned(dict(rsp=0x1008,rax=0))
        self.assertFalse(schedule.allowed)
        self.assertFalse(schedule.script_complete)
        schedule.marker(dict(phase=5,predicate_returned=True,allowed=False))

    def test_natural_lua_fork_keeps_two_target_lanes(self):
        schedule=self.ready()
        for event,next_sites in [('dispatch',('ccall_entry',)),('ccall_entry',('acquired',)),
                                 ('acquired',('resolved','protected_return')),
                                 ('resolved',('protected_return',)),('protected_return',('before_release',)),
                                 ('before_release',('released',)),('released',('wrapper_return',)),
                                 ('wrapper_return',('dispatch',))]:
            schedule.script(event,next_sites)
            self.assertLessEqual(len(schedule.points(0x10000000)),2)
        self.assertEqual(schedule.points(0x10000000),[('manager_return',0x2222)])
        schedule.returned(dict(rsp=0x1008,rax=1))
        self.assertTrue(schedule.allowed)

    def test_missing_lua_checkpoint_refuses_manager_return(self):
        schedule=self.ready()
        schedule.script('acquired',('resolved','protected_return'))
        with self.assertRaisesRegex(ValueError,'unfinished Lua'):
            schedule.returned(dict(rsp=0x1008,rax=1))

    def test_missing_actual_query_is_not_a_false_answer(self):
        schedule=Schedule()
        for phase in range(1,5):
            schedule.marker(dict(phase=phase))
        with self.assertRaisesRegex(ValueError,'actual Manager return'):
            schedule.marker(dict(phase=5,predicate_returned=True,allowed=False))

    def test_result_mismatch_or_return_stack_refused(self):
        schedule=self.ready()
        with self.assertRaisesRegex(ValueError,'unmatched Manager'):
            schedule.returned(dict(rsp=0x1000,rax=1))
        schedule.returned(dict(rsp=0x1008,rax=1))
        with self.assertRaisesRegex(ValueError,'result differs'):
            schedule.marker(dict(phase=5,predicate_returned=True,allowed=False))

    def test_unknown_reordered_duplicate_marker_refused(self):
        schedule=Schedule()
        with self.assertRaisesRegex(ValueError,'reordered'):
            schedule.marker(dict(phase=2))
        schedule.marker(dict(phase=1))
        with self.assertRaisesRegex(ValueError,'reordered'):
            schedule.marker(dict(phase=1))

    def test_duplicate_script_or_query_refused(self):
        schedule=self.ready()
        schedule.script('wrapper_return',('dispatch',))
        with self.assertRaisesRegex(ValueError,'outside selected'):
            schedule.script('dispatch',('ccall_entry',))
        with self.assertRaisesRegex(ValueError,'duplicate'):
            schedule.enter(dict(rsp=0x1000),0x2222)

    def test_actual_full_caller_suffix_required(self):
        frames=Frames([frame('/preload',0x123),frame('/host',0x456)],'/core',0x10000000)
        frames.manager_entry([frame('/core',0x2da20f0),frame('/preload',0x123),frame('/host',0x456)])
        with self.assertRaisesRegex(ValueError,'caller/parent'):
            frames.manager_entry([frame('/core',0x2da20f0),frame('/preload',0x123)])
        with self.assertRaisesRegex(ValueError,'caller/parent'):
            frames.manager_entry([frame('/core',0x2da20f0),frame('/host',0x456),frame('/preload',0x123)])

    def test_reentrant_manager_parent_refused(self):
        frames=Frames([frame('/host',0x456)],'/core',0x10000000)
        with self.assertRaisesRegex(ValueError,'reentrant'):
            frames.descendant([frame('/core',0x2da22a0),frame('/core',0x2da22a0),frame('/host',0x456)])
        self.assertEqual(frames.descendant([frame('/host',0x3bf22fa),frame('/core',0x2da22a0),frame('/host',0x456)]),1)

    def test_descriptor_unknown_bytes_preserved(self):
        raw=bytearray(88)
        struct.pack_into('<I',raw,0,4)
        raw[4:8]=b'ABCD';raw[87]=123
        struct.pack_into('<8Q',raw,8,*range(1,9))
        struct.pack_into('<iiI',raw,72,7,9,0x120001)
        raw[84]=1
        row=descriptor(bytes(raw))
        self.assertEqual(bytes.fromhex(row['raw88']),raw)
        self.assertEqual(row['actor_full_id'],0x120001)
        self.assertTrue(row['visible_only'])
        raw[85]=2
        with self.assertRaisesRegex(ValueError,'bool'):
            descriptor(bytes(raw))

    def test_zero_or_short_descriptor_refused(self):
        with self.assertRaisesRegex(ValueError,'complete'):
            descriptor(bytes(87))
        with self.assertRaisesRegex(ValueError,'phase'):
            descriptor(bytes(88))

    def fake_drain(self,unexpected=False,early=False):
        live=[];commands={};events=[];inside=[False];peak=[0];phase=[0]
        sequence=['marker']*4+['manager_entry','dispatch','ccall_entry','acquired','resolved',
                   'protected_return','before_release','released','wrapper_return','manager_return']+['marker']*5
        if early:
            sequence=['marker']*4+['manager_entry','manager_return']+['marker']*5
        inferior=SimpleNamespace(pid=77)
        class Breakpoint:
            def __init__(self,spec,**kw):
                self.spec=spec
                self.kind=kw['type']
                self.asserted=False
                live.append(self);events.append(('new',self.kind))
                peak[0]=max(peak[0],len(live))
            def delete(self):
                if inside[0]:
                    raise AssertionError('breakpoint mutated in stop callback')
                live.remove(self);events.append(('delete',self.kind))
        class Command:
            def __init__(self,name,kind):
                commands[name]=self
        def execute(command):
            events.append(('execute',command))
            if unexpected:
                return
            if not sequence:
                inferior.pid=0
                callbacks[0](SimpleNamespace(exit_code=0))
                return
            name=sequence.pop(0)
            lane=next(point for point in live if point.name==name)
            inside[0]=True
            try:
                lane.stop()
            finally:
                inside[0]=False
        callbacks=[]
        fake=SimpleNamespace(Breakpoint=Breakpoint,Command=Command,BP_HARDWARE_BREAKPOINT=97,
            COMMAND_RUNNING=0,execute=execute,selected_inferior=lambda:inferior,
            selected_thread=lambda:SimpleNamespace(ptid=(77,77,0)),
            newest_frame=lambda:SimpleNamespace(read_register=lambda name:0),
            events=SimpleNamespace(exited=SimpleNamespace(connect=callbacks.append)))
        class FakeAdapter:
            def __init__(self,gdb,plan,directory,emit):
                self.schedule=Schedule();self.base=0x10000000;self.profile='a'*64
                self.observer=SimpleNamespace(failed=False,ordinal=0)
                self.observer.close=lambda:not self.observer.failed
                self.observer.fail=lambda reason:setattr(self.observer,'failed',True)
            def checkpoint(self,name,r,thread):
                if name=='marker':
                    phase[0]+=1
                    self.schedule.marker(dict(phase=phase[0],predicate_returned=phase[0]>=5,
                                              allowed=not early,facade=0 if phase[0]==9 else 1))
                elif name=='manager_entry':
                    self.schedule.enter(dict(rsp=0x1000),0x2222)
                elif name=='manager_return':
                    self.schedule.returned(dict(rsp=0x1008,rax=0 if early else 1))
                else:
                    next_sites=dict(dispatch=('ccall_entry',),ccall_entry=('acquired',),
                        acquired=('resolved','protected_return'),resolved=('protected_return',),
                        protected_return=('before_release',),before_release=('released',),
                        released=('wrapper_return',),wrapper_return=('dispatch',))[name]
                    self.schedule.script(name,next_sites)
            def close(self):
                if self.schedule.phase!=9:
                    raise ValueError('missing terminal marker')
        with tempfile.TemporaryDirectory(prefix='c6-gdb-drain-test-') as temp:
            plan=Path(temp)/'plan.json';plan.write_text('{}')
            directory=Path(temp)/'fresh'
            with mock.patch.dict(sys.modules,{'gdb':fake}),mock.patch.object(producer,'validate_plan',return_value=({},'a'*64)),mock.patch.object(producer,'Adapter',FakeAdapter):
                state=producer.install(plan,directory)['state']
                commands['c6-full-manager-drain'].invoke('',False)
                executes_before=sum(kind=='execute' for kind,_ in events)
                with self.assertRaisesRegex(ValueError,'one-shot'):
                    commands['c6-full-manager-drain'].invoke('',False)
                self.assertEqual(sum(kind=='execute' for kind,_ in events),executes_before)
            terminal=json.loads((directory/'exit.json').read_text())
            self.assertEqual(terminal['observer_failed'],unexpected)
            self.assertEqual(state['failed'],unexpected)
            self.assertLessEqual(peak[0],3)
            self.assertTrue(all(value==97 for kind,value in events if kind=='new'))
            self.assertTrue(all(value=='continue' for kind,value in events if kind=='execute'))
            if unexpected:
                self.assertEqual(inferior.pid,77) # supervisor retains disposal ownership

    def test_outer_drain_hardware_lifecycle_in_fake_gdb(self):
        self.fake_drain()

    def test_early_false_no_Lua_full_drain(self):
        self.fake_drain(early=True)

    def test_unexpected_stop_persists_failure_and_leaves_disposal_to_owner(self):
        self.fake_drain(unexpected=True)

    def plan_fixture(self,temp,mode="source_closed_registry"):
        sources={str((ROOT/name).resolve()):hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in
            [SOURCE,'tools/linux/script_manager_observer.py','tools/linux/script_hks_registry.py',
             'tools/linux/military_observer_runner.py','tools/linux/save_lifecycle_audit.py',
             'tools/linux/script_can_start_boundary_audit.py','tools/linux/script_vm_lifecycle_audit.py',
             'tools/linux/script_hks_registry_audit.py','tools/linux/mod_activation_probe.py']}
        fake_source=Path(temp)/'fake.c';preload=Path(temp)/'fake.so'
        fake_source.write_text("static long observed_mod_main_tid;\n"+
            "\n".join('static void '+name+'(void){}' for name in ('invoke','run_bound','notify'))+
            "\nstatic void mod_probe_main(void){observed_mod_main_tid=1;}\n"+
            "void c6_linux_full_manager_fixture_marker(void){}\nvoid c6_linux_full_manager_fixture_run(void){}\nvoid c6_linux_full_manager_profile_probe(void){}\n")
        subprocess.run(['cc','-shared','-fPIC','-O0',str(fake_source),'-o',str(preload)],check=True)
        raw=preload.read_bytes();_,loads=producer.elf_contract(raw)
        required={producer.MARKER,'c6_linux_full_manager_fixture_run','invoke','run_bound','notify',
                  'c6_linux_full_manager_profile_probe','mod_probe_main','observed_mod_main_tid'}
        contracts=[dict(row,module=str(preload)) for row in producer.symbols(raw) if row['name'] in required]
        # Deduplicate symtab/dynsym entries; these are explicitly synthetic symbols.
        contracts=list({producer.canonical(row):row for row in contracts}.values())
        bodies=[]
        for row in contracts:
            if row['kind']!=2:
                continue
            load=next(entry for entry in loads if entry['address']<=row['relative_address']<entry['address']+entry['file_size'])
            offset=load['offset']+row['relative_address']-load['address']
            bodies.append(dict(name=row['name'],module=str(preload),relative_address=row['relative_address'],
                               size=row['size'],sha256=hashlib.sha256(raw[offset:offset+row['size']]).hexdigest()))
        marker=next(row for row in contracts if row['name']==producer.MARKER)
        main=next(row for row in contracts if row['name']=='mod_probe_main')
        witness=next(row for row in contracts if row['name']=='observed_mod_main_tid')
        static=dict(schema='c6-linux-full-manager-observer-plan-v1',profile_kind='genuine_lower_core_full_Manager',observation_mode=mode,
            host='/host',core='/core',preload=str(preload),modules={'/host':{'sha256':HOST_SHA},'/core':{'sha256':GAME_SHA},str(preload):{'sha256':producer.digest(preload)}},
            caller_suffix_frames=[frame(str(preload),main['relative_address'])],marker_relative_address=marker['relative_address'],
            live_bodies=bodies,compiled_symbols=contracts,
            window_parent_prefixes={} if mode=='raw_unresolved' else {name:[[frame('/core',1)]] for name in producer.SITES},
            marker_parent_frames={str(phase):[[frame(str(preload),marker['relative_address']),frame(str(preload),main['relative_address'])]] for phase in range(1,10)},
            main_entry_witness=dict(module=str(preload),relative_address=witness['relative_address']))
        review=Path(temp)/'review.json'
        packet=dict(schema='c6-linux-full-manager-stack-contract-v1',reviewed_sources=sources,
            no_VM_or_GC_parent_frames_source_closed=mode=='source_closed_registry',observation_mode=mode,**{key:static[key] for key in
            ('caller_suffix_frames','window_parent_prefixes','marker_parent_frames','live_bodies','main_entry_witness','compiled_symbols')})
        review.write_text(json.dumps(packet))
        static['stack_contract_review']=dict(path=str(review),sha256=producer.digest(review))
        path=Path(temp)/'static.json';path.write_text(json.dumps(static))
        cfg=dict(lower_core_full_manager_observer=dict(static_plan=static,static_plan_sha256=producer.digest(path)),
                 sources={str(Path(key).relative_to(ROOT)):value for key,value in sources.items() if not key.endswith('/mod_activation_probe.py')},
                 runner_sha256=sources[str(ROOT/'tools/linux/mod_activation_probe.py')],preload_sha256=producer.digest(preload))
        profile=hashlib.sha256(producer.canonical(cfg)).hexdigest()
        report=Path(temp)/'report.json';report.write_text(json.dumps(dict(configuration=cfg,configuration_sha256=profile,
            schema='c6-linux-native-profile-prepared-v1',native_execution_performed=False,returncode=None,
            execution_paths=dict(preload=str(preload)))))
        return dict(**static,runtime_gate=dict(report_path=str(report),report_sha256=producer.digest(report),
            configuration_sha256=profile,source_files=sources,static_plan_path=str(path)))

    def test_exact_profile_and_source_review_gate(self):
        with tempfile.TemporaryDirectory(prefix='c6-plan-gate-') as temp:
            plan=self.plan_fixture(temp)
            producer.validate_plan(plan)
            plan['runtime_gate']['configuration_sha256']='0'*64
            with self.assertRaisesRegex(ValueError,'configuration identity'):
                producer.validate_plan(plan)

    def test_missing_source_dependency_gate_refused(self):
        with tempfile.TemporaryDirectory(prefix='c6-plan-gate-') as temp:
            plan=self.plan_fixture(temp)
            del plan['runtime_gate']['source_files'][str((ROOT/'tools/linux/script_hks_registry.py').resolve())]
            with self.assertRaisesRegex(ValueError,'source map differs'):
                producer.validate_plan(plan)

    def test_changed_frame_plan_refused_before_attachment(self):
        with tempfile.TemporaryDirectory(prefix='c6-plan-gate-') as temp:
            plan=self.plan_fixture(temp)
            plan['caller_suffix_frames']=[frame('/preload',987)]
            with self.assertRaisesRegex(ValueError,'static full Manager plan'):
                producer.validate_plan(plan)

    def real_mapping_adapter(self):
        adapter=producer.Adapter.__new__(producer.Adapter)
        adapter.bias={'/core':0x1000}
        adapter.module_identity={'/core':dict(device=(1,2),inode=3)}
        adapter.loads={'/core':[dict(address=0,offset=0,file_size=4096,memory_size=4096,flags=5)]}
        row=dict(low=0x1000,high=0x2000,permissions='r-xp',path='/core',offset=0,device=(1,2),inode=3)
        adapter.mappings=lambda:[row]
        adapter.read=lambda address,size:bytes(size)
        adapter.stopped=lambda:None
        return adapter,row

    def test_real_executable_gate_refuses_read_only_body(self):
        adapter,row=self.real_mapping_adapter()
        adapter.executable('/core',0x1000,8)
        row['permissions']='r--p'
        with self.assertRaisesRegex(ValueError,'permissions'):
            adapter.executable('/core',0x1000,8)

    def test_real_body_gate_routes_through_executable_mapping_gate(self):
        adapter,row=self.real_mapping_adapter()
        row['permissions']='r--p'
        adapter.module_endpoint=lambda:None
        adapter.plan=dict(host='/host',core='/core',live_bodies=[])
        contract=[('/core','synthetic body',0,8,hashlib.sha256(bytes(8)).hexdigest())]
        with mock.patch.object(producer,'mandatory_bodies',return_value=contract):
            with self.assertRaisesRegex(ValueError,'permissions'):
                adapter.body_gate()

    def test_real_frame_gate_refuses_wrong_inode_device(self):
        adapter,row=self.real_mapping_adapter()
        row['inode']=99
        adapter.gdb=SimpleNamespace(newest_frame=lambda:SimpleNamespace(pc=lambda:0x1001,name=lambda:'synthetic',older=lambda:None))
        with self.assertRaisesRegex(ValueError,'owner'):
            adapter.frame_rows()
        row['inode']=3;row['device']=(4,5)
        with self.assertRaisesRegex(ValueError,'owner'):
            adapter.frame_rows()

    def test_real_executable_gate_refuses_changed_file_offset(self):
        adapter,row=self.real_mapping_adapter()
        row['offset']=1
        with self.assertRaisesRegex(ValueError,'file offset'):
            adapter.executable('/core',0x1000,8)

    def test_real_endpoint_refuses_changed_module_version(self):
        with tempfile.TemporaryDirectory(prefix='c6-module-endpoint-') as temp:
            path=Path(temp)/'synthetic';path.write_bytes(b'first')
            _,retained=producer.file_version(path)
            adapter=producer.Adapter.__new__(producer.Adapter)
            adapter.module_identity={str(path):retained}
            adapter.mappings=lambda:[dict(path=str(path),inode=retained['inode'],device=retained['device'])]
            adapter.module_endpoint()
            path.write_bytes(b'second')
            with self.assertRaisesRegex(ValueError,'file version/hash'):
                adapter.module_endpoint()

    def test_prepared_executed_or_wrong_schema_refused(self):
        with tempfile.TemporaryDirectory(prefix='c6-plan-prepared-') as temp:
            plan=self.plan_fixture(temp)
            path=Path(plan['runtime_gate']['report_path']);report=json.loads(path.read_text())
            report['native_execution_performed']=True;path.write_text(json.dumps(report))
            plan['runtime_gate']['report_sha256']=producer.digest(path)
            with self.assertRaisesRegex(ValueError,'unexecuted native preparation'):
                producer.validate_plan(plan)
            report['native_execution_performed']=False;report['schema']='wrong';path.write_text(json.dumps(report))
            plan['runtime_gate']['report_sha256']=producer.digest(path)
            with self.assertRaisesRegex(ValueError,'unexecuted native preparation'):
                producer.validate_plan(plan)

    def test_actual_compiled_caller_and_witness_required(self):
        with tempfile.TemporaryDirectory(prefix='c6-plan-symbol-') as temp:
            plan=self.plan_fixture(temp)
            raw=Path(plan['preload']).read_bytes()
            plan['compiled_symbols']=[row for row in plan['compiled_symbols'] if row['name']!='invoke']
            with self.assertRaisesRegex(ValueError,'missing compiled caller'):
                producer.compiled_contracts(plan,raw)

    def test_raw_plan_needs_no_predicted_HKS_prefix_or_GC_zero(self):
        with tempfile.TemporaryDirectory(prefix='c6-plan-raw-') as temp:
            plan=self.plan_fixture(temp,mode='raw_unresolved')
            self.assertEqual(plan['window_parent_prefixes'],{})
            producer.validate_plan(plan)

    def test_raw_known_native_context_protocol_never_decodes_HKS(self):
        memory={};rows=[];base=0x10000000;wrapper=0x9000-152
        def put(address,raw):
            memory.update({address+index:byte for index,byte in enumerate(raw)})
        def q(address,value):put(address,struct.pack('<Q',value))
        def read(address,size):return bytes(memory[address+index] for index in range(size))
        put(0x2000,b'CanStartOperation\0');q(0x3000,0x53a63a8)
        put(0x4000,struct.pack('<QQQBB',0x2000,0x462e7b4,0x5000,0,0))
        put(0x6000,b'\x00');put(0x7000,bytes(range(128)))
        q(wrapper+56,base+0x2da22a0);q(wrapper-72,0x36b40e1);q(wrapper-8,base+0x2ce79ff)
        observer=producer.RawContextTrace(read,rows.append,base)
        def point(name,**r):
            r['pc']=producer.SITES[name]+(base if name in producer.GAME_SITES else 0)
            return observer.observe(name,r,(77,77,0))
        with mock.patch.object(producer,'Observer',side_effect=AssertionError('HKS decoder must remain suppressed')):
            point('dispatch',rsp=wrapper,r12=0x2000,r13=0x3000,r15=0x5000,r14=0x6000)
            point('ccall_entry',rsp=wrapper-72,rdi=0x7000,rsi=0x36b3bc0,rdx=0x4000)
            point('acquired',rsp=wrapper-96,rbx=0x7000,r14=0x4000,r15=0x36b3bc0)
            # An unresolved generic CCall return does not become the selected result.
            self.assertEqual(point('protected_return',rsp=wrapper-192,r14=0x4500,rax=9),('resolved','protected_return'))
            point('resolved',rsp=wrapper-96,rbx=0x7000,r14=0x4000)
            point('protected_return',rsp=wrapper-96,r14=0x4000,rax=0)
            point('before_release',rsp=wrapper-96,r14=0x4000,rbx=1)
            point('released',rsp=wrapper-96,r14=0x4000)
            point('wrapper_return',rsp=wrapper,rax=0)
            self.assertTrue(observer.close())
        encoded=json.dumps(rows)
        self.assertNotIn('active_lua_parent_frames',encoded)
        self.assertNotIn('active_gc_parent_frames',encoded)
        self.assertIn('actual_CCall_state_header_raw',encoded)
        self.assertTrue(all(not row.get('registry_decoded',False) for row in rows))
        self.assertTrue(all(not row.get('callback_absence_proven',False) for row in rows))

    def test_raw_partial_internal_unwind_preserved(self):
        adapter,row=self.real_mapping_adapter()
        unknown=SimpleNamespace(pc=lambda:0x3000,name=lambda:None,
            older=lambda:(_ for _ in ()).throw(ValueError('synthetic missing CFI')))
        adapter.gdb=SimpleNamespace(newest_frame=lambda:SimpleNamespace(pc=lambda:0x1001,name=lambda:'known',older=lambda:unknown))
        frames=adapter.frame_rows(allow_unresolved=True)
        self.assertEqual(frames[1]['classification'],'unresolved_parent')
        self.assertEqual(frames[2]['classification'],'partial_unwind')
        row['inode']=99
        with self.assertRaisesRegex(ValueError,'owner'):
            adapter.frame_rows(allow_unresolved=True)

    def test_raw_own_main_prefix_does_not_predict_startup_tail(self):
        guard=Frames([frame('/preload',0x123),frame('/preload',0x456)],'/core',0x10000000,allow_unresolved_tail=True)
        guard.caller([frame('/preload',0x123),frame('/preload',0x456),frame('/libc-unclassified',None)])
        with self.assertRaisesRegex(ValueError,'caller/parent'):
            guard.caller([frame('/preload',0x123)])
        with self.assertRaisesRegex(ValueError,'caller/parent'):
            guard.caller([frame('/wrong',0x123),frame('/preload',0x456)])

    def test_only_hardware_outer_drain_mutation(self):
        tree=ast.parse((ROOT/SOURCE).read_text())
        stop=next(node for node in ast.walk(tree) if isinstance(node,ast.FunctionDef) and node.name=='stop')
        forbidden={'delete','execute','read_memory','frame_rows','checkpoint'}
        self.assertFalse(any(isinstance(node,ast.Attribute) and node.attr in forbidden for node in ast.walk(stop)))
        calls=[node for node in ast.walk(tree) if isinstance(node,ast.Call) and isinstance(node.func,ast.Attribute) and node.func.attr=='execute']
        self.assertEqual(len(calls),1)
        self.assertIsInstance(calls[0].args[0],ast.Constant)
        self.assertEqual(calls[0].args[0].value,'continue')
        self.assertNotIn('BP_BREAKPOINT',(ROOT/SOURCE).read_text())
        self.assertIn('BP_HARDWARE_BREAKPOINT',(ROOT/SOURCE).read_text())


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    host,core=Elf(args.assets/'Civ6',HOST_SHA),Elf(args.assets/'libGameCore_XP2.so',GAME_SHA)
    rows=mandatory_bodies('host','core')
    for module,name,address,size,sha in rows:
        actual=hashlib.sha256((host if module=='host' else core).read(address,size)).hexdigest()
        if actual!=sha:
            raise ValueError('mandatory original body differs: '+name)
    with tempfile.TemporaryDirectory(prefix='c6-full-manager-observer-abi-') as temp:
        source=Path(temp)/'abi.c'
        source.write_text('#include <stddef.h>\n#include "'+str(ROOT/'native/linux/full_manager_fixture.h')+'"\n'+r'''
#define CHECK(field, offset) _Static_assert(offsetof(C6FullManagerObservation,field)==offset,"marker ABI " #field)
_Static_assert(sizeof(C6FullManagerObservation)==88,"marker ABI full extent");
CHECK(phase,0);CHECK(module,8);CHECK(context,16);CHECK(game,24);CHECK(facade,32);
CHECK(manager,40);CHECK(unit,48);CHECK(operation,56);CHECK(core_base,64);
CHECK(actor_owner,72);CHECK(operation_id,76);CHECK(actor_full_id,80);
CHECK(visible_only,84);CHECK(predicate_returned,85);CHECK(allowed,86);
int main(void){return 0;}
''')
        subprocess.run(['cc','-std=c11','-Wall','-Wextra','-Werror','-fsyntax-only',str(source)],check=True)
    output=io.StringIO()
    result=unittest.TextTestRunner(stream=output).run(unittest.defaultTestLoader.loadTestsFromTestCase(Tests))
    if not result.wasSuccessful():
        raise ValueError(output.getvalue())
    paths=[SOURCE,'tools/linux/full_manager_observer_runner_audit.py',
           'native/linux/full_manager_fixture.h','tools/linux/script_manager_observer.py',
           'tools/linux/military_observer_runner.py','tools/linux/script_hks_registry.py',
           'tools/linux/script_can_start_boundary_audit.py','tools/linux/script_vm_lifecycle_audit.py',
           'tools/linux/script_hks_registry_audit.py','tools/linux/save_lifecycle_audit.py']
    report=dict(schema='c6-linux-full-manager-observer-runner-source-v1',original_execution=False,
                sources={path:hashlib.sha256((ROOT/path).read_bytes()).hexdigest() for path in paths},
                checked_original_bodies=[dict(module=m,name=n,address=hex(a),size=z,sha256=s) for m,n,a,z,s in rows],
                checks=dict(synthetic_cases=result.testsRun,marker_ABI_88_bytes=True,
                            hardware_target_lanes_at_most=2,marker_lanes=1,
                            outer_drain_only_breakpoint_mutation=True),
                launch_profile_or_native_capture_created=False,
                pending=['exact future compiled preload marker/caller/main-entry witness contracts',
                         'source-closed compiled caller/main ownership; internal HKS prefixes optional in raw_unresolved',
                         'distinct guard/loader/profile and independent adapter integration review',
                         'sole runner actual native outcomes and all affected callback/state/RNG comparison'],
                full_manager_native_parity=False,complete_state_actions_rng_turns=False,
                historical_raw_save_load_difference_bytes=89198,unknown_bytes_masked=0)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')


if __name__=='__main__':
    main()
