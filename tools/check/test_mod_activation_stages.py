"""Parser fixtures excerpted from closed original logs; no native execution."""
import importlib.util
from pathlib import Path
import unittest

path=Path(__file__).resolve().parents[1]/'linux/mod_activation_probe.py'
spec=importlib.util.spec_from_file_location('mod_activation_probe',path)
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)

# Lifecycle marker excerpts from native movement control1. Data payloads,
# addresses and private artifact contents are deliberately absent.
LOADED='''C6_LINUX_GAME_LOAD module /fixture/libGameCore_XP2.so context_initialized 1 host_finalization 0
C6_LINUX_GAME_LOAD startup_return
C6_LINUX_STREAM load_return result 0 offset 76556938 size 76556938
C6_LINUX_ACTION_LIFECYCLE initialize_cache_return
'''
CLEAN='''C6_LINUX_ACTION_LIFECYCLE uninitialize_return initialized 0 context_bound 0 executing 0 cleanup 1 dispatch_mode 0
C6_LINUX_GAME_LOAD shutdown_return
C6_LINUX_MOD client_shutdown_only_return unpublished 1
C6_LINUX_MOD cleanup_return 0
'''
class Stages(unittest.TestCase):
    def test_failed_query_keeps_successful_load_and_cleanup(self):
        result=module.lifecycle_report(206,LOADED+CLEAN+'C6_LINUX_MOD boundary_return result 206 borrowed_context 0\n')
        self.assertFalse(result['fixture_success']);self.assertEqual(result['native_boundary_result'],206)
        self.assertTrue(result['framework_boundary_returned']);self.assertTrue(result['initialized_game'])
        self.assertTrue(result['framework_cleanup_proven']);self.assertTrue(result['client_cleanup_proven'])
        self.assertTrue(result['lifecycle_stages']['controller_uninitialized_cleanly'])
    def test_successful_boundary(self):
        self.assertTrue(module.lifecycle_report(0,LOADED+CLEAN+'C6_LINUX_MOD boundary_return result 0\n')['fixture_success'])
    def test_fatal_prefix_does_not_invent_cleanup(self):
        result=module.lifecycle_report(240,LOADED)
        self.assertTrue(result['initialized_game']);self.assertFalse(result['fixture_success'])
        self.assertFalse(result['framework_cleanup_proven']);self.assertFalse(result['framework_boundary_returned'])
        self.assertFalse(result['lifecycle_stages']['game_shutdown_returned'])
    def test_truncation_and_untrusted_nonmarker_text(self):
        result=module.lifecycle_report(0,'diagnostic mentions C6_LINUX_MOD cleanup_return 0\nC6_LINUX_STREAM load_return result 0 offset 8 size 9\n')
        self.assertFalse(result['fixture_success']);self.assertFalse(result['initialized_game'])
        self.assertFalse(result['framework_cleanup_proven']);self.assertIsNone(result['native_boundary_result'])
    def test_constructor_only_does_not_claim_loaded_game(self):
        result=module.lifecycle_report(0,CLEAN+'C6_LINUX_MOD boundary_return result 0\n')
        self.assertTrue(result['fixture_success']);self.assertFalse(result['initialized_game'])
    def test_native_load_failure_does_not_claim_loaded_game(self):
        result=module.lifecycle_report(61,LOADED.replace('load_return result 0','load_return result 2'))
        self.assertFalse(result['initialized_game']);self.assertEqual(result['lifecycle_stages']['native_core_load_result'],2)

if __name__=='__main__':unittest.main()
