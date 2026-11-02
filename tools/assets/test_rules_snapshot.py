"""Canonical rules fingerprint boundaries; fixtures contain synthetic data only."""
from pathlib import Path
import sqlite3
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parent))
from rules_snapshot import TextBytes, field, fingerprint, snapshot


class RulesSnapshotTests(unittest.TestCase):
    def test_value_tags_preserve_null_integer_float_text_blob_and_float_bits(self):
        values = [None, 0, 0.0, -0.0, TextBytes(b'0'), b'0', TextBytes(b'\xff'), b'\xff']
        self.assertEqual(len({field(value) for value in values}), len(values))
        self.assertEqual(field(-0.0)[1:], struct.pack('>d', -0.0))
        self.assertNotEqual(field(TextBytes(b'a\x00b')), field(b'a\x00b'))

    def test_rowids_and_schema_trigger_changes_affect_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'fixture.sqlite'
            with sqlite3.connect(path) as db:
                db.execute('CREATE TABLE samples(value)')
                db.execute('INSERT INTO samples(rowid,value) VALUES(3,?)', (b'abc',))
            initial = fingerprint(path)
            with sqlite3.connect(path) as db:
                db.execute('UPDATE samples SET rowid=4')
            moved = fingerprint(path)
            self.assertNotEqual(initial['semantic_sha256'], moved['semantic_sha256'])
            with sqlite3.connect(path) as db:
                db.execute('CREATE TRIGGER retained AFTER INSERT ON samples BEGIN SELECT 1; END')
            triggered = fingerprint(path)
            self.assertNotEqual(moved['semantic_sha256'], triggered['semantic_sha256'])
            self.assertEqual(triggered['row_count'], initial['row_count'])

    def test_sql_storage_types_with_equal_numeric_values_change_semantic_digest(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'fixture.sqlite'
            with sqlite3.connect(path) as db:
                db.execute('CREATE TABLE x(value)')
                db.execute('INSERT INTO x VALUES(NULL)')
            identities = []
            for value in (None, 1, 1.0, '1', b'1'):
                with sqlite3.connect(path) as db:
                    db.execute('UPDATE x SET value=?', (value,))
                identities.append(fingerprint(path)['semantic_sha256'])
            self.assertEqual(len(set(identities)), len(identities))

    def test_shadowed_alias_and_without_rowid_preserve_real_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'fixture.sqlite'
            with sqlite3.connect(path) as db:
                db.execute('CREATE TABLE shadowed(rowid TEXT, value)')
                db.execute("INSERT INTO shadowed(_rowid_,rowid,value) VALUES(17,'fake',8)")
                db.execute('CREATE TABLE composite(a TEXT,b INT,PRIMARY KEY(a,b)) WITHOUT ROWID')
                db.executemany('INSERT INTO composite VALUES(?,?)', [('z', 2), ('a', 1)])
            result = fingerprint(path)
            identities = {table['name']: table['row_identity'] for table in result['semantic']['tables']}
            self.assertEqual(identities['shadowed']['alias'], '_rowid_')
            self.assertEqual(identities['composite']['columns'], ['a', 'b'])
            with sqlite3.connect(path) as db:
                db.execute('CREATE TABLE inaccessible(rowid, _rowid_, oid)')
            with self.assertRaisesRegex(ValueError, 'all intrinsic rowid'):
                fingerprint(path)

    def test_independent_identical_builds_have_same_semantic_identity_and_no_mutation(self):
        with tempfile.TemporaryDirectory() as directory:
            reports = []
            for name in ('a', 'b'):
                path = Path(directory) / (name + '.sqlite')
                with sqlite3.connect(path) as db:
                    db.execute('CREATE TABLE x(id INTEGER PRIMARY KEY, textvalue, blobvalue, nullvalue)')
                    db.execute('INSERT INTO x VALUES(5,CAST(? AS TEXT),?,NULL)', (b'\xff\x00', b'\xff\x00'))
                    db.execute('CREATE INDEX retained_index ON x(blobvalue)')
                before = path.read_bytes()
                reports.append(fingerprint(path))
                self.assertEqual(before, path.read_bytes())
            self.assertEqual(reports[0]['semantic_sha256'], reports[1]['semantic_sha256'])

    def test_missing_genuine_backup_evidence_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            evidence = Path(directory) / 'fake.json'
            evidence.write_text('{"returncode":0,"observations":[]}')
            with self.assertRaisesRegex(ValueError, 'genuine native backup'):
                snapshot(Path(directory), evidence)


if __name__ == '__main__':
    unittest.main()
