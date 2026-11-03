#!/usr/bin/env python3
"""Offline activated notification projection comparison; no original execution."""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess
from save_lifecycle_audit import Elf, GAME_SHA
ROOT = Path(__file__).resolve().parents[2]
SQL = ('SELECT rowid, AutoActivate, AutoNotify, ExpiresEndOfNextTurn, ExpiresEndOfTurn, '
       'GroupType, Icon, Message, NotificationType, SeverityType, ShowIconSinglePlayer, '
       'SubType, Summary, VisibleInUI FROM Notifications ORDER BY rowid')
METHODS = [('BuildDefinition', 0x2204bb0, 0x2a9, 'e61aaf7e606617a6ae9a4577a8a70a356831217408c06fdcd3f9124b6e4722ef'),
           ('GetBuilderSql', 0x2204e70, 8, 'e7bb8f644da89ab4728e396e6ba1e310f5c5d8d50eda1fcecb73126774f7da1c'),
           ('IsAutoNotify', 0x2d2fad0, 11, '3d5f14c6b2e6de52063915e7e9d4c6d36ac3322459ecc6cf39b9e7d8a7035544')]
FIELDS = ['row_id', 'auto_activate', 'auto_notify', 'expires_end_of_next_turn', 'expires_end_of_turn',
          'group_type', 'icon', 'message', 'notification_type', 'severity_type',
          'show_icon_single_player', 'sub_type', 'summary', 'visible_in_ui']
BOOLEAN = {1, 2, 3, 4, 10, 13}

def digest(data):
    return hashlib.sha256(data).hexdigest()

def native_hash(name):
    value = 0xffffffff
    for byte in name.encode():
        value ^= byte
        for _ in range(8):
            value = (value >> 1) ^ (0xedb88320 if value & 1 else 0)
    return value

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--fixture-output', type=Path)
    args = parser.parse_args()
    module = Elf(ROOT.parent/'c6-assets-linux/libGameCore_XP2.so', GAME_SHA)
    for name, address, size, expected in METHODS:
        if digest(module.read(address, size)) != expected:
            raise ValueError(f'original method identity mismatch: {name}')
    if module.read(0x2e3ebfb, len(SQL)+1) != SQL.encode()+b'\0':
        raise ValueError('native definition SQL differs')
    fixture = args.fixture_output or ROOT/'target/linux/loaded-notification-definitions-fixture.json'
    if args.fixture_output is None:
        subprocess.run(['python3', str(ROOT/'tools/linux/loaded_handler_adapter_check.py'), '--output', str(fixture)], check=True)
    actual = json.loads(fixture.read_bytes())
    snapshot = ROOT/actual['retained_source_snapshot']
    for name, expected in actual['source_sha256'].items():
        if digest((snapshot/name).read_bytes()) != expected:
            raise ValueError(f'retained source differs: {name}')
    if digest((ROOT/actual['retained_candidate']).read_bytes()) != actual['candidate_sha256']:
        raise ValueError('retained candidate differs')
    private = ROOT.parent/'c6-ghidra/evidence/linux-rules/mod-activation-xp2-constructor-final-1'
    connection = sqlite3.connect(f'file:{private / "gameplay.sqlite"}?mode=ro', uri=True)
    expected_rows = []
    for index, row in enumerate(connection.execute(SQL)):
        projected = {name: bool(value) if at in BOOLEAN else value for at, (name, value) in enumerate(zip(FIELDS, row))}
        if any(row[at] not in (0, 1) for at in BOOLEAN):
            raise ValueError('non-boolean native SQL value requires explicit conversion proof')
        projected.update(collection_index=index, type_hash=native_hash(projected['notification_type']))
        expected_rows.append(projected)
    if actual['notification_definitions'] != expected_rows:
        raise ValueError('production notification fields/order/hash differ from actual native SQL projection')
    autos = [row for row in expected_rows if row['auto_notify']]
    report = dict(schema='c6-linux-loaded-notification-definitions-v1', checks_passed=True,
                  checker_sha256=digest(Path(__file__).read_bytes()),
                  gamecore_sha256=GAME_SHA, original_execution=False,
                  methods=[dict(name=n,rva=hex(a),size=s,sha256=h) for n,a,s,h in METHODS],
                  builder_sql=SQL, fields_compared=len(FIELDS), definitions_compared=len(expected_rows),
                  auto_notify=autos, fixture_sha256=digest(fixture.read_bytes()),
                  candidate_sha256=actual['candidate_sha256'], retained_fixture_result=str(Path(actual['retained_source_snapshot']).parent/'result.json'), retained_source_snapshot=actual['retained_source_snapshot'],
                  retained_candidate=actual['retained_candidate'],
                  production_source_sha256=actual['source_sha256']['crates/c6-data/src/loaded_actions/notifications.rs'],
                  native_definition_object_comparison=False, native_creation_or_update_comparison=False,
                  complete_action_state_rng_parity=False,
                  unresolved=['actual creator registration and live pool authority', 'concrete Create/IsNeeded and synchronous effects',
                              'live notification locks/default blocking and subtype predicates', 'full Update and native affected-state/RNG comparison'])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,definitions=191,auto_notify=len(autos),original_execution=False)))
if __name__ == '__main__':
    main()
