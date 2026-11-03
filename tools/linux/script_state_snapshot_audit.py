#!/usr/bin/env python3
"""Source/immutable-memory checks for the opt-in natural script snapshot.
No original execution or new profile/capture admission.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import unittest
from save_lifecycle_audit import Elf,HOST_SHA,GAME_SHA
from script_state_snapshot import ARGUMENT_BODIES,SCHEMA
from test_script_state_snapshot import Tests
from test_script_snapshot_assemble import Tests as AssemblyTests

ROOT=Path(__file__).resolve().parents[2]


def sha(raw):return hashlib.sha256(raw).hexdigest()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    parser.add_argument('--adapter-audit',type=Path,required=True)
    parser.add_argument('--closed-replay',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();host=Elf(args.assets/'Civ6',HOST_SHA)
    bodies=[]
    for name,address,size,expected in ARGUMENT_BODIES:
        if sha(host.read(address,size))!=expected:raise ValueError('argument layout source differs: '+name)
        bodies.append(dict(name=name,address=hex(address),size=size,sha256=expected))
    stream=io.StringIO();result=unittest.TextTestRunner(stream=stream).run(unittest.defaultTestLoader.loadTestsFromTestCase(Tests))
    if not result.wasSuccessful():raise ValueError(stream.getvalue())
    assembly=unittest.TextTestRunner(stream=stream).run(unittest.defaultTestLoader.loadTestsFromTestCase(AssemblyTests))
    if not assembly.wasSuccessful():raise ValueError(stream.getvalue())
    adapter=json.loads(args.adapter_audit.read_text())
    current=sha((ROOT/'tools/linux/full_manager_observer_runner.py').read_bytes())
    if adapter['sources']['tools/linux/full_manager_observer_runner.py']!=current:raise ValueError('adapter audit not current')
    replay=json.loads(args.closed_replay.read_text())
    if replay!=json.loads((ROOT/'evidence/linux/full-manager-raw-capture.json').read_text()):raise ValueError('closed native2 legacy report changed')
    sources=['tools/linux/script_state_snapshot.py','tools/linux/test_script_state_snapshot.py',
        'tools/linux/script_state_snapshot_audit.py','tools/linux/full_manager_observer_runner.py',
        'tools/linux/script_manager_observer.py','tools/linux/script_hks_registry.py',
        'tools/linux/full_manager_observer_plan_assemble.py','tools/linux/test_script_snapshot_assemble.py']
    report=dict(schema='c6-linux-script-natural-snapshot-source-v1',snapshot_schema=SCHEMA,
        source_checks_passed=True,original_execution=False,native_snapshot_created=False,
        host_sha256=HOST_SHA,gamecore_sha256=GAME_SHA,
        source_files={path:sha((ROOT/path).read_bytes()) for path in sources},
        checked_argument_layout_bodies=bodies,synthetic_cases=result.testsRun,assembly_optin_cases=assembly.testsRun,
        existing_adapter_checks=adapter['checks'],
        adapter_audit_sha256=sha(args.adapter_audit.read_bytes()),
        sealed_native2_replay_equal_to_committed_report=True,
        closed_replay_sha256=sha(args.closed_replay.read_bytes()),
        natural_checkpoints=['acquired','resolved','before_release'],
        natural_resolved_PC='0x36b3c44',CCall_parent_PC='0x3bf2308',
        actual_parent_registers=['rsp','r14','r15','rbx'],
        rooted_lifetimes='Actual selected synchronous CCall context/state/args before its release; singleton roots are separately observed, not assumed equal.',
        arbitrary_native_calls=False,additional_locks=False,ASL_registration_required=False,
        unknown_GC_blocks_raw_copy=False,GC_classification='unresolved',
        partial_sections_and_images_preserved=True,empty_callback_inference=False,
        semantic_state_or_callback_effects_admitted=False,Rust_provider_added=False,
        pending=['independent collector/adapter source review',
          'fresh sole-runner source/compiled profile with script_snapshot opt-in and argument live body gates',
          'actual captured callback registration/method storage and source-backed behavior/effects reconstruction'],
        complete_state_actions_rng_turns=False)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')


if __name__=='__main__':main()
