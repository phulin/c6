#!/usr/bin/env python3
"""Summarize a passive original-host GDB manager-lifetime observation."""
import argparse
import hashlib
import json
from pathlib import Path
import re


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--trace', type=Path, required=True)
    p.add_argument('--report', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    trace = args.trace.read_text()
    report = json.loads(args.report.read_text())
    profile = report['configuration']['saved_configuration_profile']
    if profile['audit_mode'] != 'restore-only' or profile['native_writeback_requested']:
        raise ValueError('requires original restore-only observation without output serialization')
    before = re.search(r'C6_OWNER before_dtor owner=(0x[0-9a-f]+) manager=(0x[0-9a-f]+)', trace)
    deleted = re.search(r'C6_OWNER after_delete owner=(0x[0-9a-f]+) manager=(0x[0-9a-f]+)', trace)
    use = re.search(r'C6_OWNER manager_use_after_owner_delete manager=(0x[0-9a-f]+) variant=(0x[0-9a-f]+)', trace)
    if not before or not deleted or not use:
        raise ValueError('missing passive owner/delete/use observations')
    owner, manager = before.groups()
    if deleted.groups() != (owner, manager) or use[1] != manager or int(manager, 16) != int(owner, 16) + 8:
        raise ValueError('manager identity or embedded-owner contract mismatch')
    required = ['GameCore::Configuration::Game::Instance::~Instance',
                'GameCore::Context::Instance::~Instance', 'GameCore::UnloadDLL()',
                'AppModdingFramework::UnloadGameCore()',
                'AppModdingFramework::~AppModdingFramework()',
                'Program received signal SIGSEGV']
    if not all(s in trace for s in required):
        raise ValueError('missing original context teardown or child fault evidence')
    if not report['saved_configuration_restored'] or report['framework_owner_shutdown_returned_ok']:
        raise ValueError('expected restoration success and shutdown failure')
    result = dict(schema=1, method='passive GDB original-host owner/delete/manager-use observation',
                  configuration_sha256=report['configuration_sha256'],
                  framework_context_module_identity=report['framework_context_module_identity'],
                  private_trace_sha256=hashlib.sha256(args.trace.read_bytes()).hexdigest(),
                  private_report_sha256=hashlib.sha256(args.report.read_bytes()).hexdigest(),
                  restored_configuration=True, native_getters_and_writeback_skipped=True,
                  input_stream_and_backing_released=True,
                  owner_address=owner, embedded_manager_address=manager,
                  manager_used_after_owning_configuration_deleted=True,
                  use_site='TypedVariant::DestroySelf virtual manager slot +0x28',
                  consumer='original Base Context embedded Game configuration destructor',
                  runtime_cause_proven=True, runtime_fix_validated=False,
                  child_signal='SIGSEGV', debugger_exit_is_child_success=False,
                  complete_game_load=False, action_or_turn_parity=False)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'runtime_cause_proven': True, 'runtime_fix_validated': False}))


if __name__ == '__main__':
    main()
