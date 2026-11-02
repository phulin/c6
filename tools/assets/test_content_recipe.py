"""Check ordering boundaries independently of proprietary content."""
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parent))
from content_recipe import base_order, wide_key


class ContentRecipeTests(unittest.TestCase):
    def test_utf16_unit_order_differs_from_unicode_scalar_order(self):
        # A supplementary scalar sorts by its high surrogate before U+E000.
        self.assertLess(wide_key('\U00010000'), wide_key('\ue000'))
        self.assertLess(wide_key('Z'), wide_key('a'))

    def test_schema_phase_precedes_recursive_xml_content(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            data = root / 'Base/Assets/Gameplay/Data'
            schema = data / 'Schema'
            schema.mkdir(parents=True)
            (data / 'Z.xml').write_text('<GameData/>')
            (data / 'A.xml').write_text('<GameData/>')
            (data / 'ignored.sql').write_text('SELECT 1;')
            (schema / '01.sql').write_text('SELECT 1;')
            (schema / 'Tables.xml').write_text('<GameData/>')
            (data / 'Nested').mkdir()
            (data / 'Nested/B.xml').write_text('<GameData/>')
            sequence = [(phase, path.relative_to(data).as_posix())
                        for phase, path in base_order(root, 'gameplay')]
            self.assertEqual(sequence, [('schema', 'Schema/01.sql'),
                                        ('schema', 'Schema/Tables.xml'),
                                        ('content', 'A.xml'), ('content', 'Nested/B.xml'),
                                        ('content', 'Z.xml')])


if __name__ == '__main__':
    unittest.main()
