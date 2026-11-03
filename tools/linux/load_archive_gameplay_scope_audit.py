#!/usr/bin/env python3
"""Bound Unit/Formation archive bookkeeping scope; statically inspect pinned ELF."""
import argparse
import json
import re
import subprocess
from pathlib import Path
from definition_ids_audit import ROOT, TARGET, HOST, elf, sha
from load_archive_postlude_audit import methods

HOST_METHODS = {
    'has_deltas': 0x3c46fca, 'delta_indices': 0x3c45f34,
    'save_delta': 0x3c45b46, 'save_delta_union': 0x3c45c2c,
    'delta_hash': 0x3c45d9e, 'data_hash': 0x3c45e82,
    'clear_delta': 0x3c45d18, 'load_delta': 0x3c46036,
    'desync_callback': 0x3c470fc, 'sent_sync_check': 0x39d54c2,
    'process_sync_check': 0x39d998e, 'snapshot_delta': 0x39dadbc,
    'receive_snapshot': 0x39d5aa2, 'sync_complete': 0x39d96de,
}
GAME_METHODS = {'activity_assignment': 0x2da4010, 'queue_edit': 0x2da3100}
NEED = ['FAutoArchive::' + name + '(' for name in [
    'hasDeltas', 'getDeltaIndexes', 'calculateDeltaHash', 'calculateDataHash',
    'saveDelta', 'saveDeltaUnion', 'clearDelta', 'loadDelta', 'GetDeltaDesyncCallback']]


def direct_consumers(path):
    process = subprocess.Popen(['objdump', '-d', '-C', str(path)],
                               stdout=subprocess.PIPE, text=True)
    symbol = ''
    result = []
    for line in process.stdout:
        if re.match(r'^0[0-9a-f]+ <.*>:\s*$', line):
            symbol = line.strip()
        if (re.search(r'\b(call|jmp)\s', line) and any(n in line for n in NEED)
                and ('@plt>' in line or '@@Base>' in line)
                and not re.search(r'\+0x[0-9a-f]+>', line)):
            site = line.split(':', 1)[0].strip()
            target = line.split('<', 1)[1].rsplit('>', 1)[0]
            result.append(dict(caller=symbol, site='0x' + site, target=target))
    if process.wait():
        raise ValueError('objdump failed')
    return result


def clear_slots(module):
    read = elf(module, TARGET)
    symbols = subprocess.check_output(['c++filt'], input=subprocess.check_output(
        ['nm', '-D', '-S', str(module)], text=True), text=True).splitlines()
    tables, callbacks = {}, {}
    for line in symbols:
        f = line.split(maxsplit=3)
        if len(f) != 4:
            continue
        address, size, _, name = f
        if not any(owner in name for owner in [', GameCore::Unit::', ', GameCore::Formation::']):
            continue
        if name.startswith('vtable for FAutoVariable<'):
            tables[int(address, 16)] = name.removeprefix('vtable for ')
        if name.startswith('FAutoVariable<') and name.endswith('::clearDelta()'):
            callbacks[int(address, 16)] = (int(size, 16), name)
    relocations = {}
    for line in subprocess.check_output(['readelf', '-rW', str(module)], text=True).splitlines():
        f = line.split()
        if len(f) >= 5 and f[2] == 'R_X86_64_64':
            relocations[int(f[0], 16)] = int(f[3], 16)
    result = []
    for address, name in sorted(tables.items()):
        # ABI table prefix16, virtual clearDelta offset56 from address point.
        target = relocations[address + 0x48]
        size, callback = callbacks[target]
        if callback != name + '::clearDelta()' or read(target, size) != b'\xc3':
            raise ValueError('nonempty or mismatched typed clearDelta')
        result.append(dict(vtable=hex(address), type=name, clear_delta=hex(target),
                           bytes=size, sha256=sha(read(target, size))))
    if len(result) != 49 or len(callbacks) != 49:
        raise ValueError('Unit/Formation wrapper family differs')
    return result


def audit(assets, private):
    host, ha, _ = methods(assets / 'Civ6', HOST, HOST_METHODS, private)
    game, ga, _ = methods(assets / 'libGameCore_XP2.so', TARGET, GAME_METHODS, private)
    slots = clear_slots(assets / 'libGameCore_XP2.so')
    xrefs = {name: direct_consumers(assets / name) for name in ['Civ6', 'libGameCore_XP2.so']}
    if xrefs['libGameCore_XP2.so'] or len(xrefs['Civ6']) != 10:
        raise ValueError('direct archive consumer inventory differs')
    if any('FGameSyncManager::' not in x['caller'] and 'FAutoArchive::' not in x['caller'] for x in xrefs['Civ6']):
        raise ValueError('new direct consumer outside bounded family')
    checks = {
        'all_49_clear_slots_are_ret': len(slots) == 49,
        'queue_edit_only_tree_insert_and_return': '__emplace_unique_key_args' in ga['queue_edit'],
        'activity_assignment_inserts_then_writes': '__emplace_unique_key_args' in ga['activity_assignment'] and 'mov    %eax,(%rbx)' in ga['activity_assignment'],
        'sync_saves_hash_and_dirty_indices': '39d553c:' in ha['sent_sync_check'] and '39d5559:' in ha['sent_sync_check'],
        'snapshot_output_serializes_dirty_union': '39dae1d:' in ha['snapshot_delta'],
        'snapshot_input_applies_live_values': '39d82f7:' in ha['receive_snapshot'],
        'snapshot_input_invokes_desync_callback': '39d836d:' in ha['receive_snapshot'] and '39d83be:' in ha['receive_snapshot'],
        'load_delta_calls_live_wrapper_load': 'call   *0x18(%rdx)' in ha['load_delta'],
    }
    if not all(checks.values()):
        raise ValueError(checks)
    return dict(schema='c6-linux-load-archive-gameplay-scope-v1', gamecore_sha256=TARGET,
        host_sha256=HOST, original_execution=False, checks=checks,
        host_methods=host, game_methods=game, typed_clear_slots=slots,
        direct_consumers=xrefs,
        scope=dict(excluded_from_local_gameplay_acceptance=[
            'native dirty-pointer baseline/order', 'native sync registration bookkeeping',
            'native archive delta/hash wire production', 'native pointer/capacity allocation'],
            retained=['all logical field values and ordered gameplay mutations',
                'actual reports and gameplay-affecting subscribers', 'gameplay RNG and action legality',
                'source-required gameplay cache/projection updates'],
            conditions=['local in-process gameplay state is authoritative',
                'native snapshot/delta resynchronization is unsupported',
                'no blanket exclusion of other archive classes or reporting dirty components'],
            dirty_baseline_capture_required_for_local_gameplay=False,
            snapshot_input_is_gameplay_mutating=True,
            direct_scan_is_complete_indirect_inline_proof=False),
        complete_state_action_turn_parity=False)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    p.add_argument('--private-output', type=Path)
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    result = audit(a.assets, a.private_output)
    a.output.parent.mkdir(parents=True, exist_ok=True)
    a.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')


if __name__ == '__main__':
    main()
