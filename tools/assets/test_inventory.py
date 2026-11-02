"""Boundary tests for read-only metadata inventory, using synthetic files only."""
import importlib.util
from pathlib import Path
import sqlite3
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('inventory', Path(__file__).with_name('inventory.py'))
inventory = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(inventory)


class InventoryTests(unittest.TestCase):
    def test_relocation_identity_and_content_sensitivity(self):
        with tempfile.TemporaryDirectory() as directory:
            roots = [Path(directory) / name for name in ('a', 'b')]
            for root in roots:
                root.mkdir()
                (root / 'data.xml').write_bytes(b'<GameData/>')
            before = inventory.inventory(roots[0])[0]['asset_tree_sha256']
            self.assertEqual(before, inventory.inventory(roots[1])[0]['asset_tree_sha256'])
            (roots[1] / 'data.xml').write_bytes(b'<GameData> </GameData>')
            self.assertNotEqual(before, inventory.inventory(roots[1])[0]['asset_tree_sha256'])

    def test_sqlite_magic_readonly_and_quoted_table_names(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'snapshot.no_extension'
            with sqlite3.connect(path) as db:
                db.execute('CREATE TABLE "a""b" (id INTEGER PRIMARY KEY)')
                db.execute('INSERT INTO "a""b" VALUES (3)')
            before = inventory.digest(path)
            result, _ = inventory.inventory(Path(directory))
            table = result['sqlite_databases'][0]['tables'][0]
            self.assertEqual((table['name'], table['rows']), ('a"b', 1))
            self.assertEqual(before, inventory.digest(path))

    def test_declared_order_criteria_and_missing_file_are_not_guessed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'x.modinfo').write_text('''<Mod id="test"><ActionCriteria>
                <Criteria id="XP2"/></ActionCriteria><InGameActions>
                <UpdateDatabase id="core" criteria="XP2"><Properties>
                <LoadOrder>-100</LoadOrder></Properties><File Priority="2">schema.sql</File>
                <File Priority="1">missing.xml</File></UpdateDatabase>
                </InGameActions></Mod>''')
            (root / 'schema.sql').write_text('SELECT 1;')
            result, _ = inventory.inventory(root)
            action = result['mods'][0]['actions'][0]
            self.assertEqual(action['load_order'], '-100')
            self.assertEqual(action['criteria'], 'XP2')
            self.assertEqual([item['exists'] for item in action['files']], [True, False])
            self.assertEqual([item['priority'] for item in action['files']], ['2', '1'])

    def test_reject_symlink_escape(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'assets'
            root.mkdir()
            (root / 'outside').symlink_to(Path(directory))
            with self.assertRaisesRegex(ValueError, 'symlinks'):
                inventory.inventory(root)


if __name__ == '__main__':
    unittest.main()
