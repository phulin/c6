#!/usr/bin/env python3
"""Independent narrow snapshot refusals; synthetic memory only."""
import copy
from pathlib import Path
import struct
import tempfile
import unittest
from unittest import mock

import full_manager_observer_runner as adapter
import script_state_snapshot as producer
import test_script_state_snapshot as fixtures
from script_hks_registry import SnapshotError, load_image


class ReviewTests(unittest.TestCase):
    def test_each_selected_ancestor_register_and_current_identity_required(self):
        call, regs, frames = fixtures.actual()
        for target, names in [('parent', ['rsp', 'r14', 'r15', 'rbx']), ('current', ['pc', 'r14', 'rbx'])]:
            for name in names:
                changed_regs, changed_frames = copy.deepcopy(regs), copy.deepcopy(frames)
                values = changed_frames[0]['native_registers'] if target == 'parent' else changed_regs
                values[name] += 1
                reader, events = mock.Mock(), []
                with tempfile.TemporaryDirectory() as directory:
                    snapshot = producer.NaturalSnapshot(reader, events.append, 0, 'a' * 64,
                        Path(directory) / 'fresh', fixtures.spec())
                    snapshot.capture('resolved', changed_regs, changed_frames, call)
                reader.assert_not_called()
                self.assertEqual(events[0]['event'], 'script_snapshot_unattributed')

    def test_changed_tuple_preserves_exact_failure_bytes_without_pointees(self):
        call, regs, frames = fixtures.actual()
        raw = struct.pack('<QQQBB', call['event_pointer'], 0x462e7b4, call['args'] + 8, 0, 0)
        reader, events = mock.Mock(return_value=raw), []
        with tempfile.TemporaryDirectory() as directory:
            snapshot = producer.NaturalSnapshot(reader, events.append, 0, 'a' * 64,
                Path(directory) / 'fresh', fixtures.spec())
            snapshot.capture('resolved', regs, frames, call)
            manifest = next((Path(directory) / 'fresh').glob('*/image.json'))
            _, image = load_image(manifest)
            self.assertEqual(image.read(call['context'], 26), raw)
            self.assertTrue(events[0]['partial'])
            self.assertIsNone(events[0]['decoder_result'])
            self.assertFalse(events[0]['registry_absence_inferred'])
        reader.assert_called_once_with(call['context'], 26)

    def test_each_argument_body_is_mandatory_before_any_runtime_read(self):
        bodies = [dict(module='/host', relative_address=address, size=size, sha256=digest)
                  for _, address, size, digest in producer.ARGUMENT_BODIES]
        for index in range(len(bodies)):
            plan = dict(schema='c6-linux-full-manager-observer-plan-v1',
                profile_kind='genuine_lower_core_full_Manager', host='/host',
                observation_mode='raw_unresolved', script_snapshot=fixtures.spec(),
                live_bodies=bodies[:index] + bodies[index + 1:])
            with self.assertRaisesRegex(ValueError, 'argument layout source body gate absent'):
                adapter.validate_plan(plan)

    def test_partial_capture_occurrences_never_overwrite_prior_image(self):
        call, regs, frames = fixtures.actual()
        context = struct.pack('<QQQBB', call['event_pointer'], 0x462e7b4, call['args'], 0, 0)
        def reader(address, size):
            if address == call['context'] and size == 26:
                return context
            raise SnapshotError('uncaptured independent section')
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'fresh'; events = []
            snapshot = producer.NaturalSnapshot(reader, events.append, 0, 'a' * 64, root, fixtures.spec())
            snapshot.capture('resolved', regs, frames, call)
            before = {str(path.relative_to(root)): path.read_bytes() for path in root.rglob('*') if path.is_file()}
            snapshot.capture('resolved', regs, frames, call)
            self.assertEqual(len(list(root.glob('*/image.json'))), 2)
            for name, raw in before.items():
                self.assertEqual((root / name).read_bytes(), raw)
            self.assertTrue(all(row['partial'] and row['decoder_result']['errors'] for row in events))


if __name__ == '__main__':
    unittest.main()
