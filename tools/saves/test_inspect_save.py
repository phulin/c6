#!/usr/bin/env python3
"""Pinned original-save regressions and explicitly schematic framing tests.

Synthetic player lists validate parser framing, not native malformed-input parity.
The real three benchmarks remain identity gated; no game simulation is exercised.
"""
import hashlib
from pathlib import Path
import struct
import tempfile
import unittest

from inspect_save import Reader, extract_xp2, inspect, read_player_settings

ROOT = Path(__file__).resolve().parents[2]
SAVES = ROOT.parent / 'c6-assets/Base/Assets/Benchmark/Saves'


def words(*values):
    return struct.pack('<' + 'I' * len(values), *values)


class SchematicPlayerFraming(unittest.TestCase):
    def test_nonzero_player_id_is_not_a_map_count(self):
        reader = Reader(words(1, 9, 0))
        result = read_player_settings(reader)
        self.assertEqual(result['count'], 1)
        self.assertEqual(result['players'][0]['player_id'], 9)
        self.assertEqual(result['players'][0]['map_start'], 8)
        self.assertEqual(result['end'], 12)

    def test_multiple_player_ids_are_interleaved_with_maps(self):
        # First player has one u32 scalar metadata entry; second has an empty map.
        reader = Reader(words(2, 7, 1, 0x1234, 3, 0, 0, 42, 23, 0))
        result = read_player_settings(reader)
        self.assertEqual([p['player_id'] for p in result['players']], [7, 23])
        self.assertEqual([p['player_id_offset'] for p in result['players']], [4, 32])
        self.assertEqual([p['map_start'] for p in result['players']], [8, 36])
        self.assertEqual(result['players'][0]['entries'][0]['unsigned'], 42)
        self.assertEqual(result['end'], 40)

    def test_zero_count_truncation_and_negative_count_refusal(self):
        self.assertEqual(read_player_settings(Reader(words(0)))['players'], [])
        for malformed in (words(1), words(1, 3), words(0xffffffff)):
            with self.subTest(malformed=malformed), self.assertRaises(ValueError):
                read_player_settings(Reader(malformed))


class PinnedOriginalBenchmarks(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if not SAVES.is_dir():
            raise unittest.SkipTest('original benchmark assets unavailable; fixture validation gap')

    def test_all_three_metadata_boundaries_with_exact_identity_gate(self):
        for name, expected in [('XP2Benchmark', (28, 8842)),
                               ('AIBenchmark', (21, 112955)),
                               ('GraphicsBenchmark', (21, 686))]:
            with self.subTest(save=name):
                result = inspect(SAVES / (name + '.Civ6Save'))
                self.assertEqual((len(result['metadata']), result['metadata_end']), expected)
                self.assertFalse(result['initialized_game'])

    def test_changed_save_bytes_are_refused_for_all_original_names(self):
        with tempfile.TemporaryDirectory() as temporary:
            for name in ['XP2Benchmark', 'AIBenchmark', 'GraphicsBenchmark']:
                original = (SAVES / (name + '.Civ6Save')).read_bytes()
                changed = Path(temporary) / (name + '.Civ6Save')
                changed.write_bytes(original[:-1] + bytes([original[-1] ^ 1]))
                with self.subTest(save=name), self.assertRaisesRegex(ValueError, 'identity'):
                    inspect(changed)

    def test_xp2_configuration_and_payload_identity_are_preserved(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / 'stream.bin'
            result = extract_xp2(SAVES / 'XP2Benchmark.Civ6Save', output)
            settings = result['configuration_player_settings']
            self.assertEqual((settings['count_offset'], settings['count'], settings['end']),
                             (65945, 1, 66257))
            player = settings['players'][0]
            self.assertEqual((player['player_id'], player['player_id_offset'],
                              player['map_start'], len(player['entries'])),
                             (0, 65949, 65953, 15))
            blocks = result['configuration_blocks']
            self.assertEqual(blocks[2]['kind'], 'player_settings_prefix')
            self.assertNotIn('entries', blocks[2])
            self.assertEqual((blocks[0]['start'], blocks[4]['start'], blocks[4]['end']),
                             (8914, 66257, 103901))
            self.assertEqual(result['gamecore_payload_sha256'],
                             'a66736f0423e9a59b11c77b1879b471d936fe3887342df5008940f763dbcc5aa')
            self.assertEqual(hashlib.sha256(output.read_bytes()).hexdigest(),
                             result['gamecore_payload_sha256'])


if __name__ == '__main__':
    unittest.main()
