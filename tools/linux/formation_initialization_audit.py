#!/usr/bin/env python3
"""Pinned static Formation Read/constructor/archive application evidence.

No original execution, final global-load baseline, or action parity is claimed.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
from unit_archive_audit import registration_schema

ROOT = Path(__file__).resolve().parents[2]
GAME_METHODS = [
    (0x293B560, 0x181, 'Formation.Read'),
    (0x2B510F0, 0x84, 'Player.Manager.Read'),
    (0x2B51180, 0x82, 'Player.Manager.InitializePlayer'),
    (0x2B519D0, 0x23C, 'Player.Manager.AllocateInstance'),
    (0x2B4F490, 0xDBE, 'Player.Read'),
    (0x2B51C10, 0x2A9, 'Player.Manager.InitializeLists'),
    (0x2B51570, 0xC5, 'Player.Manager.ClearLists'),
    (0x290B930, 0x16, 'Globals.GetPlayer'),
    (0x2661AE0, 0xF, 'Game.GetPlayer'),
    (0x1FCC1B0, 0xC, 'Player.Manager.GetPlayer'),
    (0x293ED20, 0x32, 'Formation.archive_base.constructor'),
    (0x293EE30, 0xBD, 'Formation.archive.setVariableName'),
    (0x293B6F0, 0x6F6, 'Formation.constructor'),
    (0x293DDB0, 0x34, 'Formation.domain.edit'),
    (0x293ECE0, 0x34, 'Formation.bool.MoveAssignmentDo'),
    (0x293DE70, 0x1F, 'Formation.GetComponentID'),
    (0x1FE4710, 0xB, 'ComponentID.constructor'),
    (0x1FE5C30, 4, 'ComponentID.GetBits'),
    (0x293D8E0, 0x30, 'Formation.ClearPathCache'),
    (0x293EA30, 0x34, 'Formation.path.edit'),
    (0x293E400, 0x34, 'Formation.cache.edit'),
    (0x2B04DA0, 0x2E, 'Player.Formations.Read'),
    (0x2B050A0, 0x182, 'Formation.collection.Read'),
    (0x2B05830, 0x54, 'Formation.collection.Find'),
    (0x20E82E0, 0x1A, 'Unit.GetComponentID'),
    (0x20BCFC0, 0xA, 'UnitID.constructor'),
]
HOST_METHODS = [
    (0x3C45760, 0x15B, 'Archive.constructor'),
    (0x3C459E8, 0x27, 'Archive.add'),
    (0x3C49C28, 0x3E, 'VariableBase.constructor'),
    (0x3C49C66, 0x3E, 'VariableBase.constructor_bool'),
    (0x3C470A6, 0x55, 'Archive.SetID'),
    (0x3C45F34, 0xD9, 'Archive.getDeltaIndexes'),
]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    game_path = a.assets / 'libGameCore_XP2.so'
    game, host = Elf(game_path, GAME_SHA), Elf(a.assets / 'Civ6', HOST_SHA)
    unit, formation, unregistered = registration_schema(game_path, game)
    source = (ROOT / 'crates/c6-core/src/formation_initialization.rs').read_text()
    for name, fields in [('FORMATION_FIELDS', formation), ('UNIT_FIELDS', unit)]:
        match = re.search(r'pub const ' + name + r':.*?=\s*\[(.*?)\];', source, re.S)
        actual = re.findall(r'"([^"]+)"', match[1])
        if actual != [x['name'] for x in fields]:
            raise ValueError('Rust/native constructor registration mismatch: ' + name)
    if [int(x['wrapper_offset'], 16) for x in formation] != sorted(int(x['wrapper_offset'], 16) for x in formation):
        raise ValueError('inline field address order differs from registration order')
    # Assert the critical pinned instruction/call edges, not just symbol names.
    for address, target in [(0x293B5BF, 0x1F70080), (0x293B682, 0x1E1D6B0),
                            (0x293B6CF, 0x1ECE0B0), (0x293DDD2, 0x1EB3DC0),
                            (0x293ED01, 0x1EB3DC0), (0x293D8EB, 0x1E1FBE0),
                            (0x293D902, 0x1EA2B90), (0x2B0515D, 0x1E15E80),
                            (0x2B5114E, 0x1EAEDC0), (0x2B51159, 0x1F503A0),
                            (0x2B5119A, 0x1E74160), (0x2B4F9CA, 0x1E58EE0),
                            (0x2B4F9DD, 0x1F00530), (0x290B93F, 0x1EA2A50),
                            (0x2661AE8, 0x1E97AC0)]:
        game.call(address, target)
    for address, expected in [(0x293DE80, bytes.fromhex('b92c000000')),
                              (0x20BCFC0, bytes.fromhex('b901000000')),
                              (0x293B674, bytes.fromhex('c6042400')),
                              (0x2B05841, bytes.fromhex('440fb7fd')),
                              (0x2B51ADB, bytes.fromhex('498b442420')),
                              (0x2B51AE0, bytes.fromhex('4c8934d8')),
                              (0x1FCC1B3, bytes.fromhex('488b4f20')),
                              (0x1FCC1B7, bytes.fromhex('488b04c1'))]:
        if game.read(address, len(expected)) != expected:
            raise ValueError(f'pinned instruction mismatch at {address:#x}')
    symbols = subprocess.check_output(['nm', '-D', '-S', '-C', str(game_path)], text=True)
    methods = list(GAME_METHODS)
    for line in symbols.splitlines():
        parts = line.split(maxsplit=3)
        if len(parts) != 4:
            continue
        address, size, kind, name = parts
        if kind in ('T', 'W') and ('GameCore::Formation::Instance>::load(' in name
                                   or 'GameCore::Formation::Instance>::FAutoVariable(' in name):
            methods.append((int(address, 16), int(size, 16), name))
    def fingerprint(elf, rows):
        return [dict(address=hex(x), size=n, name=name,
                     sha256=hashlib.sha256(elf.read(x, n)).hexdigest()) for x, n, name in rows]
    report = dict(
        schema='c6-linux-formation-live-initialization-v1',
        static_checks_passed=True, original_execution=False,
        gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA,
        formation_registered_variables=formation,
        unit_registered_variables=unit,
        unit_unregistered_wrappers=unregistered,
        methods=fingerprint(game, methods), host_methods=fingerprint(host, HOST_METHODS),
        read_dirty_calls=[4, 13], evaluating_operation_after_read=False,
        formation_component_type=44, unit_component_type=1,
        owner_minus_one_skips_lookup=True,
        other_owner_uses_actual_globals_lookup=True,
        enclosing_player_owner_binding=dict(
            requested_header_identity_must_match=True,
            manager_publishes_player_before_player_read=True,
            direct_owner_table_offset='0x20',
            formation_read_follows_derived_list_refresh=True,
            inferred_unrelated_slots=False,
            cross_owner_without_provider='unknown, never assumed null',
            blanket_manager_slot_capture_required=False),
        read_preserves_member_path_cache_order=True,
        collection_find_uses_low16_without_generation_comparison=True,
        collection_read_clears_deltas=False,
        sync_manager_acceptance='live input; SetID calls provider if present and ID is valid',
        final_global_load_dirty_baseline='not established here',
        unit_initial_dirty_baseline='unavailable; attachment alone does not imply empty',
        candidate_source_sha256=hashlib.sha256(source.encode()).hexdigest(),
        audit_source_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        full_gameplay_state=False, accepted_action=False, complete_turn=False,
        native_parity=False, unknown_save_bytes_masked=False,
        recorded_raw_save_load_difference_bytes=89198,
    )
    a.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(static_checks_passed=True, methods=len(methods),
                          formation_wrappers=len(formation), unit_wrappers=len(unit))))


if __name__ == '__main__':
    main()
