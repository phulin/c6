#!/usr/bin/env python3
"""Check native configuration-owner cleanup controls without rerunning them."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import HOST_SHA, GAME_SHA


def check(path, saved):
    raw = path.read_bytes()
    report = json.loads(raw)
    err = report['stderr']
    assets = report['configuration']['assets']
    if assets['Civ6'] != HOST_SHA or assets['libGameCore_XP2.so'] != GAME_SHA:
        raise ValueError('native control identity mismatch')
    if report['returncode'] != 0 or not report['framework_cleanup_proven'] or not report['client_cleanup_proven']:
        raise ValueError('native control did not finish full cleanup')
    restore = 'restore_return result 0 offset 103901' in err
    if restore != saved:
        raise ValueError('saved configuration control mismatch')
    markers = ['framework_unload_before_configuration_destroy_enter',
               'framework_unload_before_configuration_destroy_return result 1 context_null 1',
               'framework_destroy_enter', 'framework_destroy_return singleton 0 context 0',
               'controller_shutdown_return active 0', 'app_destroy_return unpublished',
               'C6_LINUX_FILESYSTEM shutdown_return',
               'C6_LINUX_TEMPHEAP shutdown_return native_fields_clear 1',
               'C6_LINUX_MOD cleanup_return 0']
    positions = [err.index(marker) for marker in markers]
    if positions != sorted(positions):
        raise ValueError('native control cleanup order mismatch')
    return dict(configuration_sha256=report['configuration_sha256'],
                report_sha256=hashlib.sha256(raw).hexdigest(),
                saved_configuration_restored=restore, native_exit_code=0,
                original_core_unloaded_before_configuration_owner_destroyed=True,
                normal_framework_and_service_cleanup_complete=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--saved-only', type=Path, required=True)
    parser.add_argument('--no-saved', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = dict(schema=1, method='gated original-native cleanup control report checks',
                  saved_only=check(args.saved_only, True), no_saved=check(args.no_saved, False),
                  runtime_fix_validated=True, input_retention_added=False,
                  teardown_omitted=False, network_session_control_included=False,
                  complete_game_load=False, action_or_turn_parity=False)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'runtime_fix_validated': True, 'controls_passed': 2}))


if __name__ == '__main__':
    main()
