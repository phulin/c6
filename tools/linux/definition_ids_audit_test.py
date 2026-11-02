#!/usr/bin/env python3
"""Evidence-gate regression tests; these do not execute native game code."""
import copy
import json
from pathlib import Path
import unittest

import definition_ids_audit as audit


class RuntimeGateTests(unittest.TestCase):
    def setUp(self):
        root = Path(__file__).resolve().parents[2]
        report = json.loads((root / 'evidence/linux/definition-ids-native-1.json').read_text())
        self.static = json.loads((root / 'evidence/linux/definition-ids-static.json').read_text())
        self.runtime = dict(configuration=report['native_query']['source_identity'],
                            configuration_sha256=report['native_query']['configuration_sha256'],
                            returncode=0, framework_cleanup_proven=True,
                            client_cleanup_proven=True,
                            stderr='C6_LINUX_GAME_LOAD module /original/libGameCore_XP2.so context_initialized 1 host_finalization 0\n')

    @staticmethod
    def rehash(runtime):
        runtime['configuration_sha256'] = audit.sha(json.dumps(
            runtime['configuration'], sort_keys=True, separators=(',', ':')).encode())

    def test_frozen_success_identity(self):
        audit.validate_runtime(self.runtime, self.static)

    def test_cleanup_must_be_explicit_true_in_supported_schema(self):
        for key in ('framework_cleanup_proven', 'client_cleanup_proven'):
            for value in (None, False, 1, 'true'):
                runtime = copy.deepcopy(self.runtime)
                runtime[key] = value
                with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                    audit.validate_runtime(runtime, self.static)
        runtime = copy.deepcopy(self.runtime)
        del runtime['returncode']
        runtime.update(exit_code=0, active_context_module='libGameCore_XP2.so',
                       framework_cleanup_proven=False, client_cleanup_proven=False)
        with self.assertRaises(ValueError):
            audit.validate_runtime(runtime, self.static)

    def test_asset_names_cannot_be_replaced_by_unrelated_digest_fields(self):
        for key in ('Civ6', 'libGameCore_XP2.so'):
            runtime = copy.deepcopy(self.runtime)
            assets = runtime['configuration']['assets']
            assets['unrelated_' + key] = assets[key]
            assets[key] = '0' * 64
            self.rehash(runtime)
            with self.subTest(key=key), self.assertRaises(ValueError):
                audit.validate_runtime(runtime, self.static)

    def test_source_identity_and_native_abi_gate_are_required(self):
        runtime = copy.deepcopy(self.runtime)
        runtime['configuration_sha256'] = '0' * 64
        with self.assertRaises(ValueError):
            audit.validate_runtime(runtime, self.static)
        for key, value in [('lower_core_definition_registry_observer', None),
                           ('cleanup_requested', False), ('schema', 'unsupported')]:
            runtime = copy.deepcopy(self.runtime)
            runtime['configuration'][key] = value
            self.rehash(runtime)
            with self.subTest(key=key), self.assertRaises(ValueError):
                audit.validate_runtime(runtime, self.static)


if __name__ == '__main__':
    unittest.main()
