#!/usr/bin/env python3
"""Independent synthetic regressions for passive script observation; no originals."""
import struct
import tempfile
from pathlib import Path
import unittest

from script_manager_observer import CaptureImage, Observer, inspect_resolved_function
from script_hks_registry import Image, SnapshotError


class ReviewTests(unittest.TestCase):
    def test_uncached_resolved_function_replays_from_copied_image(self):
        memory = {}
        def put(address, raw):
            memory.update({address + i: byte for i, byte in enumerate(raw)})
        put(0x1000, bytes(0x80))
        put(0x1048, struct.pack('<QQ', 0x2010, 0x2000))
        put(0x2000, struct.pack('<QQ', 10, 0x3000))
        closure = bytearray(0x30)
        struct.pack_into('<QQ', closure, 0x10, 0x12345678, 0x4000)
        put(0x3000, closure)
        image = CaptureImage(lambda p, n: bytes(memory[p+i] for i in range(n)))
        actual = inspect_resolved_function(image, 0x1000)
        self.assertEqual(actual['closure']['function'], '0x12345678')
        self.assertEqual(inspect_resolved_function(Image(image.merged()), 0x1000), actual)
        memory.clear()
        self.assertEqual(inspect_resolved_function(Image(image.merged()), 0x1000), actual)

    def test_prior_private_image_cannot_be_overwritten(self):
        with tempfile.TemporaryDirectory(prefix='c6-script-review-') as temp:
            directory = Path(temp)/'image'
            image = CaptureImage(lambda p, n: b'x'*n)
            image.read(0x1000, 4)
            image.write_private(directory, {'phase': 'synthetic'})
            before = {p.name: p.read_bytes() for p in directory.iterdir()}
            with self.assertRaises(FileExistsError):
                image.write_private(directory, {'phase': 'replacement'})
            self.assertEqual({p.name: p.read_bytes() for p in directory.iterdir()}, before)
            with self.assertRaisesRegex(SnapshotError, 'fresh'):
                Observer(lambda p, n: bytes(n), lambda row: None, 0, 'a'*64, directory)

    def test_u64_and_budget_refuse_without_reader(self):
        def forbidden(p, n):
            self.fail('invalid read reached inferior adapter')
        image = CaptureImage(forbidden, budget=4)
        for address, size in [(0, 1), (1 << 64, 0), ((1 << 64)-1, 2), (1, 5), (1, -1)]:
            with self.assertRaises(SnapshotError):
                image.read(address, size)

    def test_failed_protocol_preserves_failure_and_stays_unavailable(self):
        with tempfile.TemporaryDirectory(prefix='c6-script-review-') as temp:
            rows = []
            observer = Observer(lambda p, n: self.fail('unexpected read'), rows.append,
                                0, 'a'*64, Path(temp)/'new')
            with self.assertRaises(SnapshotError):
                observer.observe('acquired', {}, 1)
            self.assertFalse(rows[-1]['callback_absence_proven'])
            with self.assertRaises(SnapshotError):
                observer.observe('dispatch', {}, 1)
            self.assertFalse(observer.close())
            self.assertFalse(rows[-1]['protocol_complete'])

    def test_partial_capture_preserves_copied_bytes_and_failure(self):
        with tempfile.TemporaryDirectory(prefix='c6-script-review-') as temp:
            rows = []
            observer = Observer(lambda p, n: b'z'*n, rows.append,
                                0, 'a'*64, Path(temp)/'new')
            def unsupported(image, phase):
                image.read(0x1000, 4)
                raise SnapshotError('unsupported synthetic graph')
            observer._snapshot = unsupported
            with self.assertRaisesRegex(SnapshotError, 'unsupported'):
                observer.snapshot('synthetic')
            self.assertEqual(rows[-1]['event'], 'partial_snapshot')
            self.assertFalse(rows[-1]['callback_absence_proven'])
            partial = Path(temp)/'new/call-0000-synthetic-partial'
            self.assertEqual((partial/'segment-0000.bin').read_bytes(), b'zzzz')
            self.assertTrue(__import__('json').loads((partial/'image.json').read_text())['partial'])

    def test_wrong_ccall_stack_and_event_pointer_refuse(self):
        for wrong_stack in (True, False):
            with self.subTest(wrong_stack=wrong_stack), tempfile.TemporaryDirectory(prefix='c6-script-review-') as temp:
                memory = {0x1000-72: struct.pack('<Q', 0x36b40e1),
                          0x1000-8: struct.pack('<Q', 0x2ce79ff),
                          0x4000: struct.pack('<QQQBB', 0x9999, 0x462e7b4, 0x5000, 0, 0)}
                observer = Observer(lambda p, n: memory[p][:n], lambda row: None,
                                    0, 'a'*64, Path(temp)/'new')
                observer.expected = ('ccall_entry',)
                observer.call = dict(thread=7, wrapper_rsp=0x1000, event_pointer=0x2000, args=0x5000)
                with self.assertRaisesRegex(SnapshotError, 'stack frame|TestAll context'):
                    observer.observe('ccall_entry', dict(pc=observer.address('ccall_entry'),
                        rsp=0x1000-72+(8 if wrong_stack else 0), rsi=0x36b3bc0,
                        rdi=0x7000, rdx=0x4000), 7)


if __name__ == '__main__':
    unittest.main()
