#!/usr/bin/env python3
"""Read-only ownership checks for pinned Linux configuration reader and teardown."""
import argparse
import json
from pathlib import Path
import struct
from save_lifecycle_audit import Elf, HOST_SHA


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    host = Elf(args.assets / 'Civ6', HOST_SHA)
    calls = [
        (0x3968319, 0x3c4298b, 'ArrayUnserialize_allocate_payload'),
        (0x3968341, 0x2f66ca0, 'ArrayUnserialize_construct_read_request'),
        (0x3968377, 0x3246a20, 'ArrayUnserialize_set_owned_array'),
        (0x3c541b7, 0x3c3d50c, 'RawRead_wrap_destination_buffer'),
        (0x3c54288, 0x3c42640, 'BufferRead_copy_payload'),
        (0x31e5bfa, 0x395e230, 'GameConfig_construct_embedded_manager'),
        (0x31ead70, 0x31e5ed0, 'ShutdownConfiguration_destroy_owner'),
        (0x31ead78, 0x3c4296c, 'ShutdownConfiguration_free_owner'),
        (0x337f3cf, 0x31ead60, 'FrameworkDestructor_shutdown_configuration'),
        (0x337f3d4, 0x337f5a0, 'FrameworkDestructor_unload_core'),
    ]
    for address, target, _ in calls:
        host.call(address, target)
    checks = [
        (0x396832d, '4c8b6838', 'ArrayUnserialize_raw_Read_slot38'),
        (0x3c541e2, 'b901000000', 'RawRead_force_copy_true'),
        (0x3961666, '498b7520ff5018', 'MapCopy_inherit_source_manager'),
        (0x31e5bf3, '4c8d7308', 'GameConfig_embedded_manager_offset8'),
        (0x395ff80, '488b4720', 'MapClear_manager_offset20'),
        (0x395ddba, 'ff5028', 'TypedDestroy_manager_free_slot28'),
    ]
    for address, expected, label in checks:
        if host.read(address, len(bytes.fromhex(expected))) != bytes.fromhex(expected):
            raise ValueError(f'original ownership contract mismatch: {label}')
    raw_read = struct.unpack('<Q', host.read(0x53cc710 + 16 + 0x38, 8))[0]
    free_array = struct.unpack('<Q', host.read(0x53b3618 + 16 + 0x28, 8))[0]
    if raw_read != 0x3c5419c or free_array != 0x396a490:
        raise ValueError('native stream/manager vtable mismatch')
    report = dict(schema=1, method='SHA-gated original Linux host static ownership audit',
                  host_sha256=HOST_SHA, checks_passed=True,
                  direct_calls=[dict(call=hex(a), target=hex(t), contract=n) for a,t,n in calls],
                  byte_contracts=[n for _,_,n in checks],
                  primitive_array_input_read_forces_copy=True,
                  map_copy_inherits_source_manager=True,
                  manager_lifetime_requires_owner=True,
                  runtime_cause_proven=False, runtime_fix_validated=False,
                  complete_game_load=False, action_or_turn_parity=False)
    args.output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps({'checks_passed': True, 'direct_calls_checked': len(calls)}))


if __name__ == '__main__':
    main()
