#!/usr/bin/env python3
"""Offline exact launch-copy path regressions; never executes a host binary."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock
import full_manager_observer_runner as runner

from full_manager_observer_plan_assemble import launch_host
from full_manager_observer_runner import ROOT


class Tests(unittest.TestCase):
    def fixture(self,temp):
        first=Path(temp)/'asset-copy';actual=Path(temp)/'launch-copy'
        first.write_bytes(b'synthetic identical ELF identity stand-in')
        actual.write_bytes(first.read_bytes())
        sha=hashlib.sha256(actual.read_bytes()).hexdigest()
        cfg=dict(original_filesystem_profile=dict(launch_host=str(actual),validated_launch=dict(copied_host=dict(sha256=sha))))
        return first,actual,sha,cfg
    def test_actual_copy_path_wins_over_same_hash_asset(self):
        with tempfile.TemporaryDirectory() as temp:
            asset,actual,sha,cfg=self.fixture(temp)
            self.assertEqual(hashlib.sha256(asset.read_bytes()).hexdigest(),sha)
            self.assertEqual(launch_host(cfg,expected_sha256=sha),str(actual))
            self.assertNotEqual(str(asset),launch_host(cfg,expected_sha256=sha))
    def test_explicit_same_hash_other_path_refused(self):
        with tempfile.TemporaryDirectory() as temp:
            asset,_,sha,cfg=self.fixture(temp)
            with self.assertRaisesRegex(ValueError,'differs from configured launch path'):
                launch_host(cfg,asset,expected_sha256=sha)
    def test_missing_launch_metadata_has_no_asset_fallback(self):
        with self.assertRaisesRegex(ValueError,'asset-copy inference forbidden'):
            launch_host(dict(assets=dict(Civ6='0'*64)))
    def test_changed_host_content_refused(self):
        with tempfile.TemporaryDirectory() as temp:
            _,actual,sha,cfg=self.fixture(temp);actual.write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError,'bytes differ'):
                launch_host(cfg,expected_sha256=sha)
    def test_four_role_exact_files_and_distinct_copy_identities(self):
        with tempfile.TemporaryDirectory() as temp:
            paths={role:str(Path(temp)/role) for role in ('host','core','preload','libc')}
            for path in paths.values():Path(path).write_bytes(b'synthetic same-byte role copies')
            sha=hashlib.sha256(Path(paths['host']).read_bytes()).hexdigest()
            cfg=dict(lower_core_full_manager_observer=dict(runtime_module_paths=paths),
                original_filesystem_profile=dict(launch_host=paths['host']),
                assets={'Civ6':sha,'libGameCore_XP2.so':sha},preload_sha256=sha,
                runtime_dependencies={'libc.so.6':dict(path=paths['libc'],sha256=sha)})
            with mock.patch.object(runner,'HOST_SHA',sha),mock.patch.object(runner,'GAME_SHA',sha):
                resolved,identities=runner.runtime_module_contracts(cfg,paths['preload'])
                self.assertEqual(resolved,paths)
                self.assertEqual(len({row['inode'] for row in identities.values()}),4)
                extra=Path(temp)/'same-hash-other';extra.write_bytes(Path(paths['libc']).read_bytes())
                cfg['lower_core_full_manager_observer']['runtime_module_paths']=dict(paths,libc=str(extra))
                with self.assertRaisesRegex(ValueError,'libc differs'):
                    runner.runtime_module_contracts(cfg,paths['preload'])
                cfg['lower_core_full_manager_observer']['runtime_module_paths']=dict(paths,preload=str(extra))
                with self.assertRaisesRegex(ValueError,'preload differs'):
                    runner.runtime_module_contracts(cfg,paths['preload'])

    def test_fixture_hash_mismatch_refused(self):
        with tempfile.TemporaryDirectory() as temp:
            _,_,sha,cfg=self.fixture(temp)
            cfg['original_filesystem_profile']['validated_launch']['copied_host']['sha256']='0'*64
            with self.assertRaisesRegex(ValueError,'fixture host identity differs'):
                launch_host(cfg,expected_sha256=sha)


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--built-report',type=Path)
    args=parser.parse_args();stream=io.StringIO()
    result=unittest.TextTestRunner(stream=stream).run(unittest.defaultTestLoader.loadTestsFromTestCase(Tests))
    if not result.wasSuccessful():raise ValueError(stream.getvalue())
    actual=None
    if args.built_report:
        report=json.loads(args.built_report.read_text())
        actual=launch_host(report['configuration_without_final_plan'])
    paths=['tools/linux/full_manager_observer_plan_assemble.py','tools/linux/full_manager_observer_runner.py',
           'tools/linux/full_manager_observer_runner_audit.py','tools/linux/full_manager_launch_host_audit.py']
    output=dict(schema='c6-linux-full-manager-launch-host-source-v1',original_execution=False,
                source_sha256={path:hashlib.sha256((ROOT/path).read_bytes()).hexdigest() for path in paths},
                synthetic_cases=result.testsRun,actual_configured_launch_host=actual,
                same_hash_other_path_not_equivalent=True,asset_copy_inference_removed=True,
                native_1_outcome='closed strict host-path refusal at first marker, before predicate; not an engine predicate result',
                closed_native_artifacts_modified=False,native_2_profile_or_execution_admitted=False,
                full_manager_parity=False,complete_state_actions_rng_turns=False)
    args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(output,indent=2,sort_keys=True)+'\n')


if __name__=='__main__':main()
