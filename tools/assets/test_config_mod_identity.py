"""Validate distinct native-backed configuration key identities and strict selection."""
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parent))
from config_mod_identity import collections, config_key
from content_recipe import xp2_declared_recipe


class SavedModIdentityTests(unittest.TestCase):
    def test_native_configuration_names_differ_from_header_keys(self):
        self.assertEqual(config_key('ENABLED_MODS'), '1b8cd1c8')
        self.assertEqual(config_key('REQUIRED_MODS'), 'fed47f44')
        self.assertNotEqual(config_key('REQUIRED_MODS'), '8427ae5c')

    def test_reject_noncollection_and_ambiguous_record(self):
        with self.assertRaisesRegex(ValueError, 'not a collection'):
            list(collections({'key': '1b8cd1c8', 'offset': 1}))
        with self.assertRaisesRegex(ValueError, 'one GUID'):
            list(collections({'key': '1b8cd1c8', 'offset': 1, 'items': [{'entries': []}]}))

    def test_unknown_selection_refused_before_opening_sources(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(ValueError, 'pinned saved'):
                xp2_declared_recipe(Path(directory), {'steps': []},
                                    {'saved_config_mod_ids': {'ENABLED_MODS': ['unknown']}})


if __name__ == '__main__':
    unittest.main()
