"""Refuse drift from the named native XP2 diagnostic fixture; no game parity claim."""
import copy
import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).parent))
from rules_xp2_inspect import ROOT, identity, validate_report


class LinuxXp2RulesGateTests(unittest.TestCase):
    def report(self):
        return json.loads((ROOT / 'evidence/linux/mod-activation-native-xp2-export-1.json').read_text())

    def test_original_report_matches_named_diagnostic_profile(self):
        configuration, backups = validate_report(self.report())
        self.assertTrue(configuration['cleanup_requested'])
        self.assertEqual([(b['tables'], b['rows']) for b in backups], [(78, 1865), (428, 41700)])

    def test_recomputed_configuration_cannot_change_source_or_callback_policy(self):
        for change in (
                lambda c: c['assets'].update({'libGameCore_XP2.so': '0' * 64}),
                lambda c: c.update({'update_callback_profile': 'saved Begin'}),
                lambda c: c.update({'ui_constructor_only': False}),
                lambda c: c['localization_profile'].update({'locale': 'other'}),
                lambda c: c.update({'saved_configuration_sha256': '0' * 64}),
                lambda c: c['original_filesystem_profile']['validated_launch'].update({'format': 'v1'})):
            report = self.report()
            change(report['configuration'])
            report['configuration_sha256'] = identity(report['configuration'])
            with self.assertRaisesRegex(ValueError, 'unsupported Linux XP2'):
                validate_report(report)

    def test_success_flags_without_native_transition_are_refused(self):
        report = self.report()
        report['stderr'] = '\n'.join(line for line in report['stderr'].splitlines()
                                     if not line.startswith('C6_LINUX_MOD updated_active_context_module '))
        with self.assertRaisesRegex(ValueError, 'actual native XP2 context module'):
            validate_report(report)

    def test_same_filename_from_another_module_path_is_refused(self):
        report = self.report()
        lines = report['stderr'].splitlines()
        lines = [('C6_LINUX_MOD updated_active_context_module /wrong/path/libGameCore_XP2.so'
                  if line.startswith('C6_LINUX_MOD updated_active_context_module ') else line)
                 for line in lines]
        report['stderr'] = '\n'.join(lines)
        with self.assertRaisesRegex(ValueError, 'actual native XP2 context module'):
            validate_report(report)

    def test_cleanup_requires_all_owners_and_order(self):
        report = self.report()
        report['client_cleanup_proven'] = False
        with self.assertRaisesRegex(ValueError, 'cleanup is unproven'):
            validate_report(report)
        prefixes = ('C6_LINUX_MOD framework_unload_before_configuration_destroy_return ',
                    'C6_LINUX_MOD framework_destroy_return ',
                    'C6_LINUX_MOD controller_shutdown_return ',
                    'C6_LINUX_MOD ui_construct_only_destroy_return ',
                    'C6_LINUX_MOD art_manager_shutdown_return',
                    'C6_LINUX_MOD content_paths_shutdown_return ',
                    'C6_LINUX_MOD client_shutdown_only_return ',
                    'C6_LINUX_LOCALIZATION shutdown_return ',
                    'C6_LINUX_HOST archive_factory_destroy_return',
                    'C6_LINUX_HOST utility_destroy_return',
                    'C6_LINUX_HOST lua_system_destroy_return ',
                    'C6_LINUX_HOST app_destroy_return ',
                    'C6_LINUX_FILESYSTEM shutdown_return',
                    'C6_LINUX_TEMPHEAP shutdown_return ',
                    'C6_LINUX_MOD boundary_return ')
        for prefix in prefixes:
            report = self.report()
            report['stderr'] = '\n'.join(line for line in report['stderr'].splitlines()
                                         if not line.startswith(prefix))
            with self.subTest(missing=prefix):
                with self.assertRaisesRegex(ValueError, 'cleanup is unproven'):
                    validate_report(report)
        report = self.report()
        lines = report['stderr'].splitlines()
        first = lines.index('C6_LINUX_MOD framework_destroy_return singleton 0 context 0')
        second = lines.index('C6_LINUX_HOST app_destroy_return unpublished')
        lines[first], lines[second] = lines[second], lines[first]
        report['stderr'] = '\n'.join(lines)
        with self.assertRaisesRegex(ValueError, 'cleanup ownership order differs'):
            validate_report(report)

    def test_missing_native_backup_or_cleanup_is_refused(self):
        for prefix, message in (
                ('C6_LINUX_RULES ', 'two successful original Linux BackupTo'),
                ('C6_LINUX_MOD cleanup_return ', 'cleanup is unproven')):
            report = self.report()
            report['stderr'] = '\n'.join(line for line in report['stderr'].splitlines()
                                         if not line.startswith(prefix))
            with self.assertRaisesRegex(ValueError, message):
                validate_report(report)

    def test_changed_resolved_selection_cannot_be_relabelled(self):
        report = self.report()
        selection = report['native_activation_selection']['after_update']['active_components']
        original = copy.deepcopy(selection)
        selection['entries'][0]['component_id'] = 'unreviewed-component'
        report['stderr'] = report['stderr'].replace(
            json.dumps(original, separators=(',', ':')), json.dumps(selection, separators=(',', ':')))
        with self.assertRaisesRegex(ValueError, 'effective selection identity'):
            validate_report(report)

    def test_rules_exports_cannot_claim_gameplay_parity(self):
        report = self.report()
        report['gameplay_parity'] = True
        with self.assertRaisesRegex(ValueError, 'unsupported Linux XP2 session/parity'):
            validate_report(report)


if __name__ == '__main__':
    unittest.main()
