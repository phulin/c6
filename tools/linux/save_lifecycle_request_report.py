#!/usr/bin/env python3
"""Check a gated native complete-stream deferred RequestLoad control report."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import HOST_SHA, GAME_SHA


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--report', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    raw = args.report.read_bytes()
    r = json.loads(raw)
    c = r['configuration']
    profile = c['full_save_request']
    if c['assets']['Civ6'] != HOST_SHA or c['assets']['libGameCore_XP2.so'] != GAME_SHA:
        raise ValueError('native original identity mismatch')
    if profile['sha256'] != 'a328d643201cb11ba226eee46c782f85b0df36b2a7a4e1a32372da8abe9cddb8' or profile['bytes'] != 7348717:
        raise ValueError('requires complete pinned Linux benchmark')
    if r['returncode'] or not r['framework_cleanup_proven'] or not r['client_cleanup_proven']:
        raise ValueError('native full cleanup did not succeed')
    if c['configure_requested'] or c['native_saved_session_control_requested'] or c.get('game_load_profile'):
        raise ValueError('requires isolated deferred-request control')
    markers = [
        'C6_LINUX_REQUEST prepare_enter complete_bytes 7348717 current_state_null 1 game_view_null 1 device 4',
        'C6_LINUX_REQUEST prepare_return accepted 1 kind 2 in_progress 0 progress_getter 0 retained_type 1 complete_copy 1',
        'C6_LINUX_REQUEST supplied_stream_released retained_valid 1',
        'C6_LINUX_REQUEST clear_return kind 0 stream_null 1',
        'framework_unload_before_configuration_destroy_return result 1 context_null 1',
        'framework_destroy_return singleton 0 context 0',
        'C6_LINUX_TEMPHEAP shutdown_return native_fields_clear 1',
        'C6_LINUX_MOD cleanup_return 0',
    ]
    positions = [r['stderr'].index(marker) for marker in markers]
    if positions != sorted(positions):
        raise ValueError('native request/cleanup marker order mismatch')
    result = dict(schema=1, method='gated original-native deferred RequestLoad control',
                  configuration_sha256=r['configuration_sha256'],
                  original_report_sha256=hashlib.sha256(raw).hexdigest(),
                  host_sha256=HOST_SHA, gamecore_target_sha256=GAME_SHA,
                  input=profile, native_exit_code=0,
                  request_accepted=True, native_request_kind=2,
                  native_request_in_progress=False, native_progress_getter=0,
                  native_retained_save_type=1, complete_retained_copy_equal=True,
                  caller_input_stream_and_bytes_released=True,
                  original_Reset_and_ClearLastStream_succeeded=True,
                  full_original_owner_cleanup_succeeded=True,
                  CheckRequest_executed=False, Begin_executed=False,
                  saved_game_censorship_profile_established=False,
                  complete_save_load=False, action_or_turn_parity=False)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'native_request_boundary_passed': True, 'complete_save_load': False}))


if __name__ == '__main__':
    main()
