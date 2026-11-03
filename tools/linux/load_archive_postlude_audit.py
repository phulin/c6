#!/usr/bin/env python3
"""Audit Linux load archive lifecycle from pinned ELF and closed evidence only."""
import argparse
import json
import struct
import subprocess
from pathlib import Path
from definition_ids_audit import ROOT, TARGET, HOST, elf, sha

GAME_METHODS = {
    'unit_constructor': 0x2c24f10, 'unit_reset': 0x2c23dc0,
    'unit_read': 0x2c231f0, 'unit_initialize_definition': 0x2c1c6e0,
    'unit_interface_postload': 0x2c1c850, 'unit_concrete_postload': 0x2c12d00,
    'unit_domain': 0x2c12bc0, 'formation_constructor': 0x293b6f0,
    'formation_read': 0x293b560, 'player_postload': 0x2b32be0,
    'game_read': 0x2d7aaa0, 'populate_sight': 0x2c46af0,
    'get_sync_manager': 0x2824770,
}
HOST_METHODS = {
    'archive_constructor': 0x3c45760, 'archive_set_id': 0x3c470a6,
    'archive_register': 0x3c45b12, 'archive_clear_delta': 0x3c45d18,
    'archive_reset': 0x3c4600e, 'archive_load_delta': 0x3c46036,
    'sync_register': 0x39d3834, 'sync_complete': 0x39d96de,
    'engine_utility_sync_getter': 0x32b4e10,
}
RESET_CALLS = [19, 19, 20, 20, 21, 21, 22, 22, 23, 23, 24, 24,
               25, 26, 62, 63, 64, 65, 66, 67, 68, 71]
# Unconditional explicit edit calls in the successful current Unit28 Read path;
# this records top-level calls, not arbitrary nested or callback effects.
READ_CALLS = [31, 34, 35, 36, 37, 39, 40, 47, 49, 54, 55, 57, 57,
              19, 19, 20, 20, 21, 22, 23, 24, 21, 22, 23, 24, 25, 26, 8]
BEFORE = [1, 8, 12, 19, 20, 21, 22, 23, 24, 25, 26, 31, 34, 35,
          36, 37, 39, 40, 44, 47, 49, 54, 55, 57, 62, 63, 64, 65,
          66, 67, 68, 71]


def symbol_table(path):
    result = {}
    for line in subprocess.check_output(['nm', '-D', '-S', str(path)], text=True).splitlines():
        f = line.split()
        if len(f) == 4:
            result.setdefault(int(f[0], 16), []).append((int(f[1], 16), f[3]))
    return result


def methods(path, expected_hash, addresses, private):
    read = elf(path, expected_hash)
    symbols = symbol_table(path)
    report, assemblies = {}, {}
    for label, address in addresses.items():
        entries = symbols[address]
        sizes = {size for size, _ in entries}
        if len(sizes) != 1:
            raise ValueError(f'ambiguous extent: {label}')
        size = sizes.pop()
        report[label] = dict(address=hex(address), bytes=size,
                             sha256=sha(read(address, size)),
                             symbols=sorted(name for _, name in entries))
        assembly = subprocess.check_output(['objdump', '-d', '-C',
            f'--start-address={address}', f'--stop-address={address + size}', str(path)], text=True)
        assemblies[label] = assembly
        if private:
            private.mkdir(parents=True, exist_ok=True)
            (private / f'{label}.asm').write_text(assembly)
    return report, assemblies, read


def collect_observations(value):
    if isinstance(value, dict):
        if 'dirty_indexes' in value:
            yield value
        for child in value.values():
            yield from collect_observations(child)
    elif isinstance(value, list):
        for child in value:
            yield from collect_observations(child)


def audit(assets, private):
    gm, ga, gr = methods(assets / 'libGameCore_XP2.so', TARGET, GAME_METHODS, private)
    hm, ha, hr = methods(assets / 'Civ6', HOST, HOST_METHODS, private)
    checks = {
        'unit_set_id_before_virtual_postload': ga['unit_read'].index('2c23d54:') < ga['unit_read'].index('2c23d5f:'),
        'formation_set_id': '293b6cf:' in ga['formation_read'],
        'set_id_register_virtual': 'call   *(%rax)' in ha['archive_set_id'],
        'set_id_unregister_virtual': 'call   *0x8(%rax)' in ha['archive_set_id'],
        'set_id_never_directly_clears_delta': 'clearDelta' not in ha['archive_set_id'],
        'sync_registration_never_directly_clears_delta': 'clearDelta' not in ha['sync_register'],
        'sync_completion_clears_delta': '39d98f7:' in ha['sync_complete'] and 'FAutoArchive::clearDelta()' in ha['sync_complete'],
        'load_delta_clears_before_apply': '3c4604f:' in ha['archive_load_delta'],
        'clear_calls_wrapper_slot_38': 'call   *0x38(%rax)' in ha['archive_clear_delta'],
        'player_postload_populates_missing_sight': '2b32d1d:' in ga['player_postload'] and 'PolulateSight' in ga['player_postload'],
        'sync_controller_from_utility_slot_108': 'jmp    *0x108(%rcx)' in ga['get_sync_manager'],
        'historical_move_after_attack_branch': gr(0x2c236a4, 3) == bytes.fromhex('83f81c'),
    }
    if not all(checks.values()):
        raise ValueError(checks)
    # Verify key native direct calls by encoded relative target, independent of labels.
    for site, target in [(0x39d98f7, 0x3c45d18), (0x3c4604f, 0x3c45d18)]:
        b = hr(site, 5)
        if b[0] != 0xe8 or site + 5 + struct.unpack('<i', b[1:])[0] != target:
            raise ValueError('clearDelta direct call mismatch')
    witness_path = ROOT / 'evidence/linux/skip-turn-state-execute-3.json'
    witness = json.loads(witness_path.read_text())
    before = [x for x in collect_observations(witness) if x.get('stage') == 'before_a']
    if len(before) != 1 or before[0]['dirty_indexes'] != BEFORE or not before[0]['has_deltas']:
        raise ValueError('closed idle Unit dirty witness differs')
    lower = sorted(set(RESET_CALLS + READ_CALLS + [1, 12]))
    return dict(schema='c6-linux-load-archive-postlude-v1', gamecore_sha256=TARGET,
                host_sha256=HOST, static_checks=checks, methods=gm, host_methods=hm,
                unit_reset=dict(ordered_explicit_calls=RESET_CALLS,
                    conditional_repeated_index=71, unregistered_hero_edit=True),
                unit28_read=dict(ordered_explicit_calls=READ_CALLS,
                    postload_explicit_calls=[1, 12], guaranteed_indices=lower,
                    complete_dirty_baseline=False),
                formation1_read=dict(explicit_calls=[4, 13], loaded_idle_baseline_closed=False),
                closed_idle_unit_witness=dict(path=str(witness_path.relative_to(ROOT)),
                    sha256=sha(witness_path.read_bytes()), stage='before_a',
                    dirty_indices=BEFORE, extra_over_explicit_unit28=sorted(set(BEFORE) - set(lower)),
                    source_read_version_attributed=False, registration_vector_crosschecked=False),
                lifecycle=dict(set_id='changed identity unregisters old registration, stores ID, conditionally registers through supplied virtual controller; no direct dirty clear',
                    sync_complete='nonnull FireWire session required; clear every archive in actual registered map, then notify session',
                    clear_delta='visits dirty wrappers slot+0x38 before clearing dirty pointer tree',
                    reset='archive.reset is a different per-variable virtual reset; Unit.Reset is gameplay reset and marks dirty'),
                acceptance=dict(original_execution=False, complete_loaded_idle_baseline=False,
                    complete_live_state=False, full_state_action_turn_parity=False))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--private-output', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = audit(args.assets, args.private_output)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')


if __name__ == '__main__':
    main()
