#!/usr/bin/env python3
"""Static admission for the genuine registered SkipTurn handler query."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
from unit_operation_legality_audit import METHODS as LEGALITY, CALLS
from placement_metadata_audit import METHODS as METADATA
from unit_placement_audit import METHODS as PLACEMENT

ROOT = Path(__file__).resolve().parents[2]
EXTRA = [
    ('component_at', 0x264c060, 42, 'e864d685fed8e09fe6089b219d16d885221ad68c732a5c70a0d3a9d2167fac11'),
    ('manager_ctor', 0x2da2e10, 119, '18fed24ab0e82efa36eab366345e17ce8e0b9880b977fa39231d36175e000b9d'),
    ('map_find', 0x2da3020, 51, 'aaf03ca7fd20001939179a57fa4d626a38d80cdd473f3f9b10d46fda96301c81'),
    ('map_end', 0x2da3070, 19, '7bcc6425ec30e5ab81388544f73ff1ecdbd50fa3ffca50ce5f3d2f9c20c1d4bd'),
    ('layered_ctor', 0x21a3f90, 23, 'cb93ebef973fd5906a7e7ec33e2dc71b19833883b3d46329ee0aa617fdcd2b05'),
    ('player_manager', 0x2b53260, 8, '3767e99c9e2a663768e78a24c0669349430acf05be01ce1ee8f273e1f62679e7'),
    ('player_at', 0x1fcc1b0, 12, '47fe7c239b1abbb63533198084d4b834d08e20f997a7b46ccce8700c65b4a2fb'),
    ('config_player', 0x28e4ea0, 34, 'f08bf18d4625c5777f771dc4221dcae4643e68c027f474b6a682816d8e168406'),
    ('operation_ctor', 0x213d4a0, 24, '4c67da5b259652b60282f0b2537cb9ad58ffe092118c741793120955b33014a3'),
    ('operation_dtor', 0x213cac0, 9, '602378b0f64b1db59b55f1d15e148f2e1fc33dec4f30cb6f94be62391d4d1251'),
    ('shared_dtor', 0x1fe6f50, 48, 'ba934e9e2c29aed297d3e9b31c814d9d3c9dc377c67dc51c13a0ebdfb9ac17fd'),
    ('layered_location', 0x21a3c40, 46, 'c042869ea69ddb8d86d2c428a298739008082bcbf286fd29a6382442717e7d42'),
    ('layered_units', 0x21a3c30, 15, '24941d2b182a588dd6ac5e63b1dddaea1c4bf3f2a5d4c6d99a88d661467d0f44'),
    ('player_is_human', 0x2b31530, 32, '7b956c7055aea66551b2309c8d001e35a098fc6e19355bd959b5a46241245c95'),
]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    elf = Elf(a.assets / 'libGameCore_XP2.so', GAME_SHA)
    Elf(a.assets / 'Civ6', HOST_SHA)
    methods = LEGALITY + PLACEMENT + EXTRA
    methods += [(n, r, s, h) for n, r, s, _, h in METADATA]
    registry = json.loads((ROOT / 'evidence/linux/operation-registry-abi.json').read_text())
    assert registry['gamecore_sha256'] == GAME_SHA
    methods += [(n, int(v['address'], 16), v['bytes'], v['sha256'])
                for n, v in registry['methods'].items()]
    rows = []
    for name, address, size, fingerprint in methods:
        if sha(elf.read(address, size)) != fingerprint:
            raise ValueError('method mismatch: ' + name)
        rows.append(dict(name=name, address=hex(address), size=size, sha256=fingerprint))
    for address, target in CALLS:
        elf.call(address, target)
    for address, target in [(0x3b00db0 + 0x18, 0x2dac950),
                            (0x3afb160 + 0xc0, 0x2a48750),
                            (0x3aff880 + 0xa0, 0x2da1720),
                            (0x3aff880 + 0x88, 0x2da1650)]:
        if elf.relocated_pointer(address) != target:
            raise ValueError('virtual binding mismatch')
    relocations = subprocess.check_output(['readelf', '-rW',
        str(a.assets / 'libGameCore_XP2.so')], text=True)
    manager_got = [line.split() for line in relocations.splitlines()
                   if line.startswith('0000000003b09aa0 ')]
    if len(manager_got) != 1 or manager_got[0][2:] != [
            'R_X86_64_GLOB_DAT', '0000000003aff870',
            '_ZTVN8GameCore4Unit10Operations7ManagerE', '+', '0']:
        raise ValueError('Manager constructor GOT identity mismatch')
    elf.call(0x2da1273, 0x1f74b70)
    # Original ctor writes every semantic field and a null shared_ptr. The
    # handler never accesses its Operation argument, so the original destructor
    # follows its null-control-block path without release or allocation.
    if elf.read(0x213d4a0, 24).hex() != '8937c74704ffffffff0f57c00f114708c7471800000000c3':
        raise ValueError('Operation constructor changed')
    dwarf = subprocess.check_output(['gdb', '-q', '-nx', '-batch',
        str(a.assets / 'libGameCore_XP2.so'), '-ex',
        "ptype /o 'GameCore::Unit::Operations::Operation'"], text=True)
    for required in ['total size (bytes):   32', '8      |      16', '24      |       4']:
        if required not in dwarf:
            raise ValueError('Operation DWARF mismatch')
    source = ['native/linux/skip_turn_handler_query.c', 'native/linux/skip_turn_handler_query.h',
              'tools/linux/skip_turn_handler_query_audit.py',
              'tools/linux/unit_operation_legality_audit.py',
              'tools/linux/unit_placement_audit.py', 'tools/linux/placement_metadata_audit.py']
    report = dict(schema='c6-linux-skip-turn-handler-query-v1', checks_passed=True,
        gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA, original_execution_performed=False,
        methods=rows, dwarf_sha256=sha(dwarf.encode()),
        source_sha256={x: sha((ROOT / x).read_bytes()) for x in source},
        receiver='borrowed original Manager.GetHandlerFromHash(0x2c681908); manager owns handler',
        abi=['const handler*', 'const Unit*', 'const Operation&', 'const Plot* nullable',
             'bool visible_only', 'TypedVariantMap* null'], return_abi='bool AL',
        operation='complete 32-byte 8-aligned local; original ctor(runtime ID), original dtor; null parameters',
        ignored_arguments=['Operation', 'Plot', 'visible_only', 'results'],
        query_scope='handler only; NULL Unit is recorded without original handler invocation',
        purity='source-derived read-only getter/count chain; complete live purity requires native controls',
        native_comparison_available=False, full_manager_can_start_supported=False,
        complete_state_action_turns_supported=False, historical_raw_save_load_failure_bytes=89198)
    a.output.parent.mkdir(parents=True, exist_ok=True)
    a.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(checks_passed=True, checked_method_contracts=len(rows), original_execution=False)))


if __name__ == '__main__':
    main()
