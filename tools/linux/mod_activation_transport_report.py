#!/usr/bin/env python3
"""Check the passive original-host transport replay; never executes game code."""
import argparse
import hashlib
import json
from pathlib import Path
import re


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def require(value, message):
    if not value:
        raise ValueError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--replay-directory', type=Path, required=True)
    parser.add_argument('--parent-report', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    directory = args.replay_directory
    metadata = json.loads((directory / 'replay.json').read_text())
    parent = json.loads(args.parent_report.read_text())
    config = parent['configuration']
    require(sha(args.parent_report) == metadata['parent_report_sha256'], 'parent report differs')
    canonical = json.dumps(config, sort_keys=True, separators=(',', ':')).encode()
    require(hashlib.sha256(canonical).hexdigest() == metadata['parent_configuration_sha256'] == parent['configuration_sha256'], 'parent configuration differs')
    require(sha(directory / 'hardware.gdb') == metadata['script_sha256'], 'GDB script differs')
    require(sha(directory / 'replay_driver.py') == metadata['replay_driver_sha256'], 'replay driver differs')
    require(sha(directory / 'transcript.txt') == metadata['transcript_sha256'], 'transcript differs')
    require(metadata['archived_preload_sha256'] == config['preload_sha256'], 'preload identity differs')
    script = (directory / 'hardware.gdb').read_text().splitlines()
    require([line for line in script if line.startswith('hbreak ')] == ['hbreak *0x32b4d00', 'hbreak *0x32b4d33', 'hbreak *0x39f2d1a'], 'game hardware breakpoints differ')
    require(not any(re.match(r'\s*(?:call|break|tbreak|finish|return|jump|set\s+\*)\b', line) for line in script), 'script contains active debugger commands')
    text = (directory / 'transcript.txt').read_text()
    require('shlib events' in text and 'hw breakpoint' in text, 'actual debugger instrumentation missing')
    start = text.index('C6_LINUX_ACTION_DISPATCH request_enter')
    returned = text.index('C6_LINUX_ACTION_DISPATCH request_return pending 0', start)
    update_returned = text.index('C6_LINUX_ACTION_DISPATCH network_update_zero_return pending 0', returned)
    def observed(part):
        return dict(engine_utility_entry=part.count('C6_GDB_HARDWARE engine_utility_enter'), engine_utility_normal_return=part.count('C6_GDB_HARDWARE engine_utility_normal_return'), normal_return_rax=0)
    request = observed(text[start:returned])
    update = observed(text[returned:update_returned])
    require(request == dict(engine_utility_entry=1, engine_utility_normal_return=1, normal_return_rax=0), 'request bridge observations differ')
    require(update == request, 'update bridge observations differ')
    require(text[returned:update_returned].count('C6_GDB_HARDWARE session_handle_message_enter') == 1, 'session callback observation differs')
    returns = re.findall(r'^C6_GDB_HARDWARE engine_utility_normal_return[^\n]*rax=([^\n]+)', text, re.MULTILINE)
    require(returns == ['(nil)', '(nil)'], 'original normal return differs')
    require('C6_LINUX_MOD cleanup_return 0' in text and 'boundary_return result 229' in text, 'failed boundary or cleanup evidence missing')
    events = [json.loads(line) for line in (directory / 'trace/events.jsonl').read_text().splitlines()]
    end = events[-1]
    require(end['event'] == 'end' and all(end[key] == 0 for key in ('queued', 'received', 'unit_executed', 'reports')), 'Core delivery unexpectedly occurred')
    expected = 'ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884'
    captures = {p.name: dict(sha256=sha(p), bytes=p.stat().st_size) for p in directory.glob('*.bin')}
    require(set(captures) == {'save.bin', 'before.bin', 'after.bin', 'after-repeat.bin'}, 'capture set differs')
    require(captures == metadata['captures'] and all(row == dict(sha256=expected, bytes=80657462) for row in captures.values()), 'native Save control differs')
    summary = dict(schema='c6-linux-original-host-transport-observation-v1', profile='passive GDB replay of failed original SkipTurn delivery attempt2',
        parent_configuration_sha256=metadata['parent_configuration_sha256'], parent_runtime_report_sha256=metadata['parent_report_sha256'],
        archived_preload_sha256=metadata['archived_preload_sha256'], original_assets=config['assets'],
        reporter_sha256=sha(__file__), private_evidence=dict(directory='evidence/linux-action-execution/transport-hardware-2', replay_metadata_sha256=sha(directory/'replay.json'), gdb_script_sha256=metadata['script_sha256'], transcript_sha256=metadata['transcript_sha256'], driver_sha256=metadata['replay_driver_sha256']),
        instrumentation=dict(game_address_breakpoints=metadata['gdb_breakpoints'], game_breakpoint_type='hardware', automatic_loader_software_breakpoint=True, loader_module_sha256=metadata['gdb_internal_breakpoint']['module_sha256'], debugger=metadata['debugger'], debugger_game_function_calls=False, game_code_patching=False, forced_returns=False, game_state_or_vtable_writes=False),
        observations=dict(during_original_request=request, during_original_network_update_zero=dict(session_handle_message_entry=1, **update), core_queued=0, core_received=0, unit_executed=0, native_boundary_result=229, native_cleanup_result=0, native_cleanup_marker_observed=True),
        save_captures=captures, all_captures_match_parent_control=True,
        limits=['Debugger stopping changes scheduling; this is an observation profile, not a timing-equivalent run.', 'The observed session HandleMessage payload type was not inspected and does not prove receipt of the action.', 'EngineUtility normally returns zero unconditionally; that return is not a delivery-success result.', 'No original action execution or gameplay/turn parity is established.'], gameplay_parity=False)
    args.output.write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(dict(passive_observation_checked=True, native_boundary_result=229, action_executed=False)))


if __name__ == '__main__':
    main()
