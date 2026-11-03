"""Bounded synthetic natural-window images; never execute the original engine."""
import hashlib
import json
from pathlib import Path
import struct
import tempfile
import unittest
from unittest import mock
from types import SimpleNamespace
import script_state_snapshot as producer
from script_hks_registry import Image,SnapshotError,load_image
from script_manager_observer import CaptureImage,SITES
from full_manager_observer_runner import Adapter


def spec():
    return dict(schema=producer.SCHEMA,phases=list(producer.PHASES),byte_budget=65536,
        max_items=1024,source_sha256=hashlib.sha256(Path(producer.__file__).read_bytes()).hexdigest())


def actual():
    call=dict(context=0x2000,state=0x3000,args=0x4000,event_pointer=0x6000,entry_rsp=0x10000)
    regs=dict(pc=SITES['resolved'],rsp=0xf000,r14=call['context'],rbx=call['state'],r15=0x36b3bc0)
    frames=[dict(pc=0x3bf2308,native_registers=dict(rsp=call['entry_rsp']-24,
        r14=call['context'],rbx=call['state'],r15=0x36b3bc0))]
    return call,regs,frames


class Tests(unittest.TestCase):
    def test_late_argument_layout_and_signed_int_raw_bytes(self):
        raw=bytearray(0x1b8);struct.pack_into('<QQ',raw,0,0x53a6300,0xabcd)
        struct.pack_into('<Q',raw,0x10,0xdeadbeef) # conventional-vector bait
        struct.pack_into('<Q',raw,0x1a0,0x2000);struct.pack_into('<I',raw,0x1a8,3);struct.pack_into('<I',raw,0x1b0,2)
        cells=bytearray(48);struct.pack_into('<I',cells,0,2);struct.pack_into('<i',cells,8,-12)
        struct.pack_into('<I',cells,24,99)
        result=producer.arguments(Image([(0x1000,raw),(0x2000,cells)]),0x1000)
        self.assertEqual(result['cells'][0]['signed_i32'],-12)
        self.assertEqual(result['unsupported_variant_cells'],[1])
        self.assertEqual(result['cells'][1]['raw'],cells[24:].hex())

    def test_arguments_extent_overflow_refused_before_reader(self):
        raw=bytearray(0x1b8);struct.pack_into('<Q',raw,0,0x53a6300)
        struct.pack_into('<Q',raw,0x1a0,(1<<64)-16)
        struct.pack_into('<I',raw,0x1a8,2);struct.pack_into('<I',raw,0x1b0,2)
        reader=mock.Mock(side_effect=lambda p,n:bytes(raw) if p==0x1000 else self.fail('overflow reached native reader'))
        with self.assertRaisesRegex(ValueError,'extent'):
            producer.arguments(CaptureImage(reader),0x1000)
        self.assertEqual(reader.call_count,1)

    def test_wrong_concrete_argument_binding_unsupported(self):
        with self.assertRaisesRegex(SnapshotError,'override'):
            producer.arguments(Image([(0x1000,bytes(0x1b8))]),0x1000)

    def test_selected_resolved_actual_ancestor_not_predicted_HKS_prefix(self):
        call,r,frames=actual();observer=producer.NaturalSnapshot.__new__(producer.NaturalSnapshot)
        frames.insert(0,dict(pc=0x1234,classification='unresolved_parent'))
        self.assertTrue(observer.selected('resolved',r,frames,call))
        frames[-1]['native_registers']['rsp']+=8
        self.assertFalse(observer.selected('resolved',r,frames,call))

    def test_missing_or_wrong_selected_ancestor_never_reads_heap(self):
        call,r,frames=actual();rows=[];reader=mock.Mock()
        with tempfile.TemporaryDirectory() as root:
            observer=producer.NaturalSnapshot(reader,rows.append,0,'a'*64,Path(root)/'fresh',spec())
            observer.capture('resolved',r,[],call)
        reader.assert_not_called();self.assertEqual(rows[0]['event'],'script_snapshot_unattributed')
        self.assertFalse(rows[0]['callback_absence_proven'])

    def test_wrong_current_context_or_PC_is_not_selected(self):
        call,r,frames=actual();observer=producer.NaturalSnapshot.__new__(producer.NaturalSnapshot)
        r['r14']+=8;self.assertFalse(observer.selected('resolved',r,frames,call))
        r['r14']=call['context'];r['pc']+=1
        self.assertFalse(observer.selected('resolved',r,frames,call))

    def test_acquired_requires_actual_state_and_original_body(self):
        call,r,frames=actual();observer=producer.NaturalSnapshot.__new__(producer.NaturalSnapshot)
        r.update(pc=SITES['acquired'],rsp=call['entry_rsp']-24)
        self.assertTrue(observer.selected('acquired',r,frames,call))
        r['rbx']+=8;self.assertFalse(observer.selected('acquired',r,frames,call))

    def test_partial_capture_keeps_independent_mutex_and_offline_image(self):
        call,r,frames=actual();rows=[]
        lock=bytearray(64);struct.pack_into('<i',lock,12,1);struct.pack_into('<Q',lock,24,0x5000)
        image=Image([(0x6ec9c70,lock),(0x5000,b'x'*40),
            (call['context'],struct.pack('<QQQ',call['event_pointer'],0x462e7b4,call['args'])+b'\0\0')]) # ASL handle stays zero
        with tempfile.TemporaryDirectory() as root:
            observer=producer.NaturalSnapshot(image.read,rows.append,0,'a'*64,Path(root)/'fresh',spec())
            observer.capture('resolved',r,frames,call)
            result=rows[0];self.assertTrue(result['partial'])
            self.assertEqual(result['decoder_result']['sections']['Lua_lock']['pointed_pthread_mutex_raw'],(b'x'*40).hex())
            self.assertFalse(result['registry_absence_inferred'])
            manifest=next((Path(root)/'fresh').glob('*/image.json'))
            _,copied=load_image(manifest)
            self.assertEqual(copied.read(0x5000,40),b'x'*40)
            self.assertEqual(result['GC_classification'],'unresolved')

    def test_actual_resolved_builtin_captures_ordered_callbacks_and_weak_mode(self):
        from test_script_hks_registry import Fixture,value,number
        f=Fixture();callback=f.alloc(0x30);f.put(callback+0x10,struct.pack('<Q',0x123456))
        array=f.table(array=[value(10,callback)])
        meta=f.table(entries=[(f.string(b'__mode'),f.string(b'v'))]);f.put(array+0x10,struct.pack('<Q',meta))
        groups=f.table(array=[value(5,array)])
        closure=f.alloc(0x40);f.put(closure+0x10,struct.pack('<Q',0x36b3540))
        f.put(closure+0x20,struct.pack('<h',1));f.put(closure+0x30,value(5,groups).raw)
        state=f.alloc(0x80);stack=f.alloc(16);f.put(stack,value(10,closure).raw)
        f.put(state+0x48,struct.pack('<QQ',stack+16,stack))
        result=producer.decode(f.image(),state,0xdead,0,resolved=True)
        actual=result['sections']['actual_resolved_function']
        self.assertEqual(actual['closure']['function'],'0x36b3540')
        self.assertEqual(actual['callback_arrays'][0]['callbacks'][0]['closure']['function'],'0x123456')
        self.assertEqual(actual['callback_arrays'][0]['table']['raw_weak_mode_bytes_hex'],'76')
        self.assertFalse(result['semantic_snapshot_admitted'])
        self.assertEqual(result['GC_classification'],'unresolved')

    def test_offline_decoder_uses_retained_actual_state_and_arguments(self):
        call,r,frames=actual();rows=[]
        with tempfile.TemporaryDirectory() as root:
            root=Path(root)
            context=struct.pack('<QQQ',call['event_pointer'],0x462e7b4,call['args'])+b'\0\0'
            observer=producer.NaturalSnapshot(lambda p,n:context if p==call['context'] else bytes(n),rows.append,0,'a'*64,root/'fresh',spec())
            observer.capture('resolved',r,frames,call)
            manifest=next((root/'fresh').glob('*/image.json'));output=root/'decoded.json'
            with mock.patch('sys.argv',['snapshot','--image',str(manifest),'--output',str(output)]):
                producer.main()
            decoded=json.loads(output.read_text())
            self.assertTrue(decoded['offline_decode'])
            self.assertFalse(decoded['result']['semantic_snapshot_admitted'])
            self.assertEqual(json.loads(manifest.read_text())['actual_CCall_arguments'],hex(call['args']))

    def test_fresh_private_directory_refuses_overwrite(self):
        with tempfile.TemporaryDirectory() as root:
            directory=Path(root)/'existing';directory.mkdir()
            with self.assertRaisesRegex(SnapshotError,'fresh private'):
                producer.NaturalSnapshot(mock.Mock(),lambda row:None,0,'a'*64,directory,spec())

    def test_budget_failure_retains_failure_image(self):
        call,r,frames=actual();rows=[]
        context=struct.pack('<QQQ',call['event_pointer'],0x462e7b4,call['args'])+b'\0\0'
        args=bytearray(0x1b8);struct.pack_into('<Q',args,0,0x53a6300)
        struct.pack_into('<Q',args,0x1a0,0x9000);struct.pack_into('<I',args,0x1a8,100);struct.pack_into('<I',args,0x1b0,100)
        def reader(p,n):
            return context if p==call['context'] else bytes(args) if p==call['args'] else bytes(n)
        with tempfile.TemporaryDirectory() as root:
            options=spec();options['byte_budget']=1024
            observer=producer.NaturalSnapshot(reader,rows.append,0,'a'*64,Path(root)/'fresh',options)
            observer.capture('resolved',r,frames,call)
            self.assertTrue(rows[0]['partial'])
            self.assertIn('budget exhausted',rows[0]['decoder_result']['errors']['arguments'])
            self.assertTrue(list((Path(root)/'fresh').glob('*/image.json')))

    def test_changed_fresh_context_tuple_prevents_pointee_reads(self):
        call,r,frames=actual();rows=[];reader=mock.Mock(return_value=bytes(26))
        with tempfile.TemporaryDirectory() as root:
            observer=producer.NaturalSnapshot(reader,rows.append,0,'a'*64,Path(root)/'fresh',spec())
            observer.capture('resolved',r,frames,call)
            reader.assert_called_once_with(call['context'],26)
            self.assertIn('tuple changed',rows[0]['decoder_failure'])
            self.assertFalse(rows[0]['registry_absence_inferred'])

    def test_optin_frame_registers_do_not_change_legacy_projection(self):
        adapter=Adapter.__new__(Adapter);adapter.stopped=lambda:None
        adapter.bias={};adapter.mappings=lambda:[];adapter.plan={}
        frame=SimpleNamespace(pc=lambda:0x777,name=lambda:'unknown',older=lambda:None,
            read_register=lambda name:dict(rsp=1,r14=2,r15=3,rbx=4)[name])
        adapter.gdb=SimpleNamespace(newest_frame=lambda:frame)
        old=adapter.frame_rows(allow_unresolved=True)
        adapter.plan={'script_snapshot':spec()};new=adapter.frame_rows(allow_unresolved=True)
        self.assertEqual(new[0].pop('native_registers'),dict(rsp=1,r14=2,r15=3,rbx=4))
        self.assertEqual(old,new)

    def test_invalid_or_changed_snapshot_spec_refused(self):
        options=spec();options['source_sha256']='0'*64
        with self.assertRaisesRegex(SnapshotError,'source identity'):producer.validate_spec(options)
        options=spec();options['phases']=['forced_lookup']
        with self.assertRaisesRegex(SnapshotError,'phases'):producer.validate_spec(options)


if __name__=='__main__':unittest.main()
