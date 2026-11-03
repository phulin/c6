#!/usr/bin/env python3
"""Independent narrow compiled-plan checks; never loads native code."""
import copy
import hashlib
import json
from pathlib import Path
import unittest

import full_manager_observer_plan as extractor
import full_manager_observer_runner as adapter


ROOT = Path(__file__).resolve().parents[2]
RETAINED = ROOT / 'target/linux/full-manager-draft/plan-fragment-v4'


class ReviewTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.fragment = json.loads((RETAINED / 'fragment.json').read_bytes())

    def test_every_required_own_frame_is_contiguous_and_complete(self):
        prefix = self.fragment['caller_suffix_frames']
        guard = adapter.Frames(prefix, '/core', 0, allow_unresolved_tail=True)
        guard.caller(prefix + [dict(module=None, relative_pc=None, classification='partial_unwind')])
        for index in range(len(prefix)):
            for altered in (prefix[:index], prefix[:index] + prefix[index + 1:]):
                with self.subTest(index=index, mutation='missing'), self.assertRaises(ValueError):
                    guard.caller(altered)
            altered = copy.deepcopy(prefix)
            altered[index] = dict(module=None, relative_pc=None)
            with self.subTest(index=index, mutation='unknown'), self.assertRaises(ValueError):
                guard.caller(altered)
            altered = copy.deepcopy(prefix)
            altered[index]['relative_pc'] += 1
            with self.subTest(index=index, mutation='changed'), self.assertRaises(ValueError):
                guard.caller(altered)

    def test_descendant_requires_unique_manager_and_full_own_prefix(self):
        prefix = self.fragment['caller_suffix_frames']
        guard = adapter.Frames(prefix, '/core', 0, allow_unresolved_tail=True)
        continuation = dict(module='/core', relative_pc=0x2da22a0)
        guard.descendant([dict(module=None, relative_pc=None), continuation] + prefix)
        for frames in ([continuation] + prefix[:-1], [continuation, continuation] + prefix):
            with self.assertRaises(ValueError):
                guard.descendant(frames)
        strict = adapter.Frames(prefix, '/core', 0)
        with self.assertRaises(ValueError):
            strict.caller(prefix + [dict(module=None, relative_pc=None)])

    def test_retained_binary_sources_manifest_and_extractor_are_bound(self):
        fragment = self.fragment
        self.assertEqual(extractor.sha((RETAINED / 'preload.so').read_bytes()), fragment['preload_sha256'])
        self.assertEqual(fragment['retained_preload_sha256'], fragment['preload_sha256'])
        self.assertEqual(extractor.sha((RETAINED / 'build-or-prepared.json').read_bytes()), fragment['input_manifest_sha256'])
        self.assertEqual(extractor.sha((RETAINED / 'extractor.py').read_bytes()), fragment['extractor_sha256'])
        self.assertEqual(extractor.sha(Path(extractor.__file__).read_bytes()), fragment['extractor_sha256'])
        self.assertEqual(len(fragment['retained_source_sha256']), 21)
        self.assertEqual(fragment['declared_header_preimages'], [])
        for name, expected in fragment['retained_source_sha256'].items():
            self.assertEqual(extractor.sha((RETAINED / 'source' / name).read_bytes()), expected)
        self.assertFalse(fragment['complete_runtime_plan'])
        self.assertFalse(fragment['profile_admitted'])

    def test_actual_own_return_sites_and_all_marker_prefixes(self):
        fragment, elf = extractor.extract(RETAINED / 'preload.so', self.fragment['preload_sha256'])
        # The retained path changes module spelling only; compare actual offsets.
        self.assertEqual([r['relative_pc'] for r in fragment['caller_suffix_frames']],
                         [r['relative_pc'] for r in self.fragment['caller_suffix_frames']])
        owners = ['invoke', 'run_bound', 'c6_linux_full_manager_fixture_run',
                  'c6_linux_full_manager_profile_probe', 'prepare',
                  'c6_linux_action_prepare_full_manager_fixture', 'c6_linux_game_load', 'mod_probe_main']
        for frame, owner in zip(fragment['caller_suffix_frames'], owners):
            calls = [r for r in elf.code[owner] if r['asm'].startswith('call ') and r['return_pc'] == frame['relative_pc']]
            self.assertEqual(len(calls), 1)
            self.assertEqual(len(bytes.fromhex(calls[0]['raw'])), calls[0]['size'])
        self.assertEqual(set(fragment['marker_parent_frames']), set(map(str, range(1, 10))))
        for alternatives in fragment['marker_parent_frames'].values():
            for chain in alternatives:
                self.assertEqual(chain[-1], fragment['caller_suffix_frames'][-1])
                self.assertEqual(chain[-6:], fragment['caller_suffix_frames'][-6:])


if __name__ == '__main__':
    unittest.main()
