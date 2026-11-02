"""Failure boundaries for the private schema materializer; synthetic inputs only."""
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parent))
from inventory import digest
import schema_bootstrap


class SchemaBootstrapTests(unittest.TestCase):
    def setup_fixture(self, directory, sql='CREATE TABLE examples(id INTEGER);'):
        root = Path(directory) / 'assets'
        root.mkdir()
        source = root / 'schema.sql'
        source.write_text(sql)
        return root, {'sources': [{'path': source.name, 'sha256': digest(source)}]}

    def test_identity_failure_creates_no_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root, gate = self.setup_fixture(directory)
            output = Path(directory) / 'result'
            (root / 'schema.sql').write_text('SELECT 1;')
            with patch.object(schema_bootstrap, 'GROUPS', {'configuration': ['schema.sql']}):
                with self.assertRaisesRegex(ValueError, 'identity mismatch'):
                    schema_bootstrap.materialize(root, output, gate)
            self.assertFalse(output.exists())

    def test_sql_failure_publishes_no_partial_database(self):
        with tempfile.TemporaryDirectory() as directory:
            root, gate = self.setup_fixture(directory, 'CREATE TABLE x(id); invalid sql;')
            output = Path(directory) / 'result'
            with patch.object(schema_bootstrap, 'GROUPS', {'configuration': ['schema.sql']}):
                with self.assertRaises(schema_bootstrap.sqlite3.Error):
                    schema_bootstrap.materialize(root, output, gate)
            self.assertFalse(output.exists())
            self.assertEqual(sorted(path.name for path in Path(directory).iterdir()), ['assets'])

    def test_original_assets_are_not_modified_and_existing_output_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root, gate = self.setup_fixture(directory)
            output = Path(directory) / 'result'
            before = digest(root / 'schema.sql')
            with patch.object(schema_bootstrap, 'GROUPS', {'configuration': ['schema.sql']}):
                report = schema_bootstrap.materialize(root, output, gate)
                self.assertFalse(report['initialized_game'])
                self.assertEqual(report['databases'][0]['tables'][0]['rows'], 0)
                with self.assertRaisesRegex(ValueError, 'already exists'):
                    schema_bootstrap.materialize(root, output, gate)
            self.assertEqual(before, digest(root / 'schema.sql'))


if __name__ == '__main__':
    unittest.main()
