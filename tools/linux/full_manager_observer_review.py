#!/usr/bin/env python3
"""Narrow independent hardware-adapter regressions; no native game execution."""
import json
from pathlib import Path
import struct
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock

import full_manager_observer_runner as producer
import full_manager_observer_runner_audit as fixtures


class ReviewTests(unittest.TestCase):
    def test_consistent_report_with_wrong_preload_identity_refuses(self):
        with tempfile.TemporaryDirectory(prefix='c6-manager-review-') as temp:
            plan=fixtures.Tests().plan_fixture(temp,mode='raw_unresolved')
            path=Path(plan['runtime_gate']['report_path'])
            report=json.loads(path.read_text())
            report['configuration']['preload_sha256']='0'*64
            identity=__import__('hashlib').sha256(producer.canonical(report['configuration'])).hexdigest()
            report['configuration_sha256']=identity
            path.write_text(json.dumps(report))
            plan['runtime_gate'].update(configuration_sha256=identity,report_sha256=producer.digest(path))
            with self.assertRaisesRegex(ValueError,'compiled preload differs'):
                producer.validate_plan(plan)

    def test_missing_plan_persists_failure_without_lane_or_continue(self):
        def forbidden(*args,**kwargs):
            self.fail('missing plan reached hardware installation or continuation')
        fake=SimpleNamespace(Breakpoint=forbidden,execute=forbidden)
        with tempfile.TemporaryDirectory(prefix='c6-manager-review-') as temp:
            directory=Path(temp)/'fresh'
            with mock.patch.dict(sys.modules,{'gdb':fake}):
                with self.assertRaises(FileNotFoundError):
                    producer.install(Path(temp)/'absent-plan.json',directory)
                before={path.name:path.read_bytes() for path in directory.iterdir()}
                with self.assertRaises(FileExistsError):
                    producer.install(Path(temp)/'absent-plan.json',directory)
                self.assertEqual(before,{path.name:path.read_bytes() for path in directory.iterdir()})
            self.assertTrue(json.loads((directory/'exit.json').read_text())['observer_failed'])
            row=json.loads((directory/'observations.jsonl').read_text())
            self.assertEqual(row['event'],'installation_refused')
            self.assertFalse(row['callback_absence_proven'])

    def test_writable_original_code_mapping_refuses(self):
        adapter,row=fixtures.Tests().real_mapping_adapter()
        row['permissions']='rwxp'
        with self.assertRaisesRegex(ValueError,'permissions'):
            adapter.executable('/core',0x1000,8)

    def test_raw_protected_error_skips_lookup_and_never_reads_registry(self):
        memory={};events=[];wrapper=0x9000-152;base=0x10000000
        def put(address,raw):memory.update({address+i:byte for i,byte in enumerate(raw)})
        def q(address,value):put(address,struct.pack('<Q',value))
        q(wrapper+56,base+0x2da22a0);q(wrapper-72,0x36b40e1);q(wrapper-8,base+0x2ce79ff)
        put(0x2000,b'CanStartOperation\0');q(0x3000,0x53a63a8)
        put(0x4000,struct.pack('<QQQBB',0x2000,0x462e7b4,0x5000,0,0))
        put(0x6000,b'\0');put(0x7000,bytes(128))
        allowed={(wrapper+56,8),(wrapper-72,8),(wrapper-8,8),(0x2000,18),
                 (0x3000,8),(0x4000,26),(0x6000,1),(0x7000,128)}
        def read(address,size):
            self.assertIn((address,size),allowed,'raw mode attempted unrelated heap traversal')
            return bytes(memory[address+i] for i in range(size))
        trace=producer.RawContextTrace(read,events.append,base)
        def point(name,**r):
            r['pc']=producer.SITES[name]+(base if name in producer.GAME_SITES else 0)
            return trace.observe(name,r,(77,77,0))
        point('dispatch',rsp=wrapper,r12=0x2000,r13=0x3000,r15=0x5000,r14=0x6000)
        point('ccall_entry',rsp=wrapper-72,rdi=0x7000,rsi=0x36b3bc0,rdx=0x4000)
        point('acquired',rsp=wrapper-96,rbx=0x7000,r14=0x4000,r15=0x36b3bc0)
        self.assertEqual(point('protected_return',rsp=wrapper-192,r14=0x4500,rax=7),('resolved','protected_return'))
        point('protected_return',rsp=wrapper-96,r14=0x4000,rax=0xffffffff)
        point('before_release',rsp=wrapper-96,r14=0x4000,rbx=0)
        point('released',rsp=wrapper-96,r14=0x4000)
        point('wrapper_return',rsp=wrapper,rax=0)
        self.assertTrue(trace.close())
        returned=next(row for row in events if row['event']=='raw_actual_CallTestAll_return')
        self.assertEqual(returned['protected_status_raw_u32'],0xffffffff)
        self.assertFalse(returned['return_bit'])
        self.assertFalse(any(row.get('registry_decoded') or row.get('callback_absence_proven') for row in events))
        self.assertNotIn('active_gc_parent_frames',json.dumps(events))
        self.assertNotIn('active_lua_parent_frames',json.dumps(events))


if __name__=='__main__':
    unittest.main()
