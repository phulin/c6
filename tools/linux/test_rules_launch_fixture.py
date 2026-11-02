"""Synthetic launch-layout validation; no original native code executes here."""
from pathlib import Path
import hashlib
import json
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0,str(Path(__file__).parent))
import rules_launch_fixture as fixture


class LaunchFixtureTests(unittest.TestCase):
    def inputs(self, root):
        assets=root/'assets';assets.mkdir()
        for name in ('Base','DLC','aspyrassets'):(assets/name).mkdir()
        (assets/'Base/assets').mkdir()
        (assets/'Civ6').write_bytes(b'synthetic-host-not-an-executable')
        (assets/'Base/assets/data.xml').write_bytes(b'<Synthetic/>')
        host=hashlib.sha256((assets/'Civ6').read_bytes()).hexdigest()
        repo=root/'repo';(repo/'evidence/linux').mkdir(parents=True)
        gate={'modules':{'Civ6':host},'files':{'Base/assets/data.xml':fixture.digest(assets/'Base/assets/data.xml')},'source_inventory_sha256':'synthetic'}
        (repo/'evidence/linux/rules-content-inputs.json').write_text(json.dumps(gate))
        return assets,repo,host

    def test_regular_copy_and_gated_aliases_then_source_change_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);assets,repo,host=self.inputs(root)
            with patch.object(fixture,'ROOT',repo),patch.object(fixture,'HOST',host):
                fixture.stage(assets,root/'launch')
                result=fixture.validate(assets,root/'launch')
                self.assertEqual(result['validated_source_inventory_sha256'],'synthetic')
                self.assertFalse((root/'launch/Civ6').is_symlink())
                self.assertEqual((assets/'Base/assets/data.xml').read_bytes(),b'<Synthetic/>')
                (assets/'Base/assets/data.xml').write_bytes(b'changed')
                with self.assertRaisesRegex(ValueError,'content input mismatch'):
                    fixture.validate(assets,root/'launch')

    def test_module_siblings_use_a_separate_mapping_version(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);assets,repo,host=self.inputs(root)
            gate_path=repo/'evidence/linux/rules-content-inputs.json'
            gate=json.loads(gate_path.read_text())
            for name in fixture.MODULE_ALIASES:
                (assets/name).write_bytes(('synthetic:'+name).encode())
                gate['modules'][name]=fixture.digest(assets/name)
            gate_path.write_text(json.dumps(gate))
            with patch.object(fixture,'ROOT',repo),patch.object(fixture,'HOST',host):
                fixture.stage(assets,root/'base')
                fixture.stage(assets,root/'modules',gamecore_siblings=True)
                base=fixture.validate(assets,root/'base')
                modules=fixture.validate(assets,root/'modules')
                self.assertNotEqual(base['mapping_sha256'],modules['mapping_sha256'])
                self.assertTrue(base['format'].endswith('.v1'))
                self.assertTrue(modules['format'].endswith('.v2'))
                self.assertEqual(len(modules['mapping']),6)

    def test_v3_cwd_skeleton_remains_inside_native_assets_prefix(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);assets,repo,host=self.inputs(root)
            gate_path=repo/'evidence/linux/rules-content-inputs.json'
            gate=json.loads(gate_path.read_text())
            for name in fixture.MODULE_ALIASES:
                (assets/name).write_bytes(('synthetic:'+name).encode())
                gate['modules'][name]=fixture.digest(assets/name)
            gate_path.write_text(json.dumps(gate))
            with patch.object(fixture,'ROOT',repo),patch.object(fixture,'HOST',host):
                fixture.stage(assets,root/'launch',preserve_cwd=True)
                result=fixture.validate(assets,root/'launch')
                self.assertTrue(result['format'].endswith('.v3'))
                cwd=root/'launch/steamassets/base/binaries/linux'
                self.assertTrue(cwd.resolve().is_relative_to(root/'launch/steamassets'))
                self.assertFalse((root/'launch/steamassets/base').is_symlink())
                self.assertTrue((root/'launch/steamassets/base/assets').is_symlink())
                cwd.rmdir();cwd.symlink_to(assets/'Base',target_is_directory=True)
                with self.assertRaisesRegex(ValueError,'real fixture directories'):
                    fixture.validate(assets,root/'launch')

    def test_executable_symlink_cannot_replace_exact_copy(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);assets,repo,host=self.inputs(root)
            with patch.object(fixture,'ROOT',repo),patch.object(fixture,'HOST',host):
                fixture.stage(assets,root/'launch')
                (root/'launch/Civ6').chmod(0o600);(root/'launch/Civ6').unlink()
                (root/'launch/Civ6').symlink_to(assets/'Civ6')
                with self.assertRaisesRegex(ValueError,'regular-file host copy'):
                    fixture.validate(assets,root/'launch')

    def test_real_steamassets_root_and_no_unknown_input_entries(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);assets,repo,host=self.inputs(root)
            with patch.object(fixture,'ROOT',repo),patch.object(fixture,'HOST',host):
                fixture.stage(assets,root/'launch')
                (root/'launch/unreviewed').write_bytes(b'unknown')
                with self.assertRaisesRegex(ValueError,'undeclared launch'):
                    fixture.validate(assets,root/'launch')
                (root/'launch/unreviewed').unlink()
                with self.assertRaisesRegex(ValueError,'new private directory'):
                    fixture.stage(assets,root/'launch')


if __name__=='__main__':unittest.main()
