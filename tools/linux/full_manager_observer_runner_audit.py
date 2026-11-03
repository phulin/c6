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

    def fake_drain(self,unexpected=False):
        live=[];commands={};events=[];inside=[False];peak=[0];phase=[0]
        sequence=['marker']*4+['manager_entry','dispatch','ccall_entry','acquired','resolved',
                   'protected_return','before_release','released','wrapper_return','manager_return']+['marker']*5
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
                                              allowed=True,facade=0 if phase[0]==9 else 1))
                elif name=='manager_entry':
                    self.schedule.enter(dict(rsp=0x1000),0x2222)
                elif name=='manager_return':
                    self.schedule.returned(dict(rsp=0x1008,rax=1))
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

    def test_unexpected_stop_persists_failure_and_leaves_disposal_to_owner(self):
        self.fake_drain(unexpected=True)

    def plan_fixture(self,temp):
        sources={str((ROOT/name).resolve()):hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in
            [SOURCE,'tools/linux/script_manager_observer.py','tools/linux/script_hks_registry.py',
             'tools/linux/military_observer_runner.py','tools/linux/save_lifecycle_audit.py',
             'tools/linux/script_can_start_boundary_audit.py','tools/linux/script_vm_lifecycle_audit.py',
             'tools/linux/script_hks_registry_audit.py']}
        static=dict(schema='c6-linux-full-manager-observer-plan-v1',profile_kind='genuine_lower_core_full_Manager',
            host='/host',core='/core',preload='/preload',modules={'/host':{'sha256':HOST_SHA},'/core':{'sha256':GAME_SHA}},
            caller_suffix_frames=[frame('/preload',123)],marker_relative_address=456,
            live_bodies=[dict(module='/preload',relative_address=456,size=1,sha256='a'*64,name='marker')],
            window_parent_prefixes={name:[[frame('/core',1)]] for name in producer.SITES},
            marker_parent_frames={str(phase):[[frame('/preload',456)]] for phase in range(1,10)},
            main_entry_witness=dict(module='/preload',relative_address=789))
        review=Path(temp)/'review.json'
        packet=dict(schema='c6-linux-full-manager-stack-contract-v1',reviewed_sources=sources,
            no_VM_or_GC_parent_frames_source_closed=True,**{key:static[key] for key in
            ('caller_suffix_frames','window_parent_prefixes','marker_parent_frames','live_bodies','main_entry_witness')})
        review.write_text(json.dumps(packet))
        static['stack_contract_review']=dict(path=str(review),sha256=producer.digest(review))
        path=Path(temp)/'static.json';path.write_text(json.dumps(static))
        cfg=dict(lower_core_full_manager_observer=dict(static_plan=static,static_plan_sha256=producer.digest(path)))
        profile=hashlib.sha256(producer.canonical(cfg)).hexdigest()
        report=Path(temp)/'report.json';report.write_text(json.dumps(dict(configuration=cfg,configuration_sha256=profile)))
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
            with self.assertRaisesRegex(ValueError,'source gate absent'):
                producer.validate_plan(plan)

    def test_changed_frame_plan_refused_before_attachment(self):
        with tempfile.TemporaryDirectory(prefix='c6-plan-gate-') as temp:
            plan=self.plan_fixture(temp)
            plan['caller_suffix_frames']=[frame('/preload',987)]
            with self.assertRaisesRegex(ValueError,'static full Manager plan'):
                producer.validate_plan(plan)

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
                         'source-closed complete caller and natural-window parent-frame prefixes',
                         'distinct guard/loader/profile and independent adapter integration review',
                         'sole runner actual native outcomes and all affected callback/state/RNG comparison'],
                full_manager_native_parity=False,complete_state_actions_rng_turns=False,
                historical_raw_save_load_difference_bytes=89198,unknown_bytes_masked=0)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')


if __name__=='__main__':
    main()
