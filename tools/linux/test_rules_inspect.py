"""Linux profile gate refusals; synthetic metadata is not native parity evidence."""
from pathlib import Path
import json
import sys
import tempfile
import unittest

sys.path.insert(0,str(Path(__file__).parent))
from rules_inspect import inspect, identity, HOST, GAME


class LinuxRulesGateTests(unittest.TestCase):
    def report(self, root, assets, passed=True):
        configuration={'original_assets':assets}
        path=root/'report.json'
        path.write_text(json.dumps({'configuration':configuration,
                                    'configuration_sha256':identity(configuration),
                                    'exit_code':0,'diagnostic_passed':passed,'stderr':''}))
        return path

    def test_windows_profile_cannot_be_substituted(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            path=self.report(root,{'Civ6':HOST,'libGameCore_XP2.so':'324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c'})
            with self.assertRaisesRegex(ValueError,'unsupported Linux'):
                inspect(root,path)

    def test_missing_native_backup_is_not_success(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            path=self.report(root,{'Civ6':HOST,'libGameCore_XP2.so':GAME})
            with self.assertRaisesRegex(ValueError,'native Linux backups'):
                inspect(root,path)

    def test_changed_configuration_identity_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            path=self.report(root,{'Civ6':HOST,'libGameCore_XP2.so':GAME})
            report=json.loads(path.read_text());report['configuration']['extra']='changed'
            path.write_text(json.dumps(report))
            with self.assertRaisesRegex(ValueError,'configuration identity'):
                inspect(root,path)


if __name__=='__main__':unittest.main()
