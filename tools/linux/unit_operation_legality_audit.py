#!/usr/bin/env python3
"""Pinned original Linux operation predicate audit; no original-code execution."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import Elf, GAME_SHA
ROOT = Path(__file__).resolve().parents[2]
METHODS = [
    ('Unit.IsBusy', 0x2c27360, 13, '670e18b08c2b17b2781aa026faf057fdb0fc2963f7bef934e590da5fda8cad30'),
    ('Unit.OperationTimer', 0x2c2c2c0, 7, 'f556d300a8d89adaffc772457586759f83790839af4114aed582c498ec6d9a90'),
    ('Unit.CanMove', 0x2c27930, 60, 'b46d22d98040b2db4bf384a1374692b3025952b8424b16a70323c73ca8718c57'),
    ('Unit.MovesRemaining', 0x213cc90, 21, 'cacb37411e9aab85d6dd05c1d07fb2f2a36acd2877aeee83ac8bd09f08e92c13'),
    ('FixedPoint.CopyCtor', 0x1fcc8c0, 5, '0dbb0cb08dfc52a8874d7e85a1b0b4bc164f0b6501356e0185faffc434c3cd8b'),
    ('FixedPoint.Copy', 0x1fcdc00, 8, '775143daff273e7ffd54aaf1fdd8ee35101783a6286e5c6a3d9b76fed694472a'),
    ('FixedPoint.IntCtor', 0x1fcc8b0, 5, 'a5050360463a3fe04639a9ad47d842d6a2c680138f54cfbbbe95eaf24f73e912'),
    ('FixedPoint.IntAssign', 0x1fcdc10, 9, '289ee59e91d4f68ddbe5f570b5002092702cd7aae484d25f21ca0576db55b96f'),
    ('FixedPoint.Greater', 0x1fe2b30, 8, 'ab00271575857734f9deafa1606796d9d87586ae02bc14e6486fe4c79f0cb86d'),
    ('SkipTurn.CanStart', 0x2dac950, 218, '43cc02ab9b2f7976450820d0b623b903f2f90b2029a6431463776996245a8f83'),
    ('Unit.IsHuman', 0x2c1c740, 32, 'f8f8d2a77f089f5a5bda429951a2d12c52171701aeae66ecf4b57e7c08c234a5'),
    ('Unit.FortifyTurns', 0x20f33b0, 7, '414030f95dfa6ad16a2d09baf4b1e503e3ed605578cbea5f0ab0d9822f1e3030'),
    ('Unit.Activity', 0x265b300, 7, 'e39c923bce0c545f73489d73292f45689f4114aa9f2c4d56131f54a161c3a0a6'),
    ('Unit.Espionage', 0x206dfd0, 8, '5c7112bbb45d99c13d453ce214e766b9173d04cbc39a957d4a90882a926953eb'),
    ('Espionage.Operation', 0x206dfe0, 4, '464ad404f57fdf46d34aca82663f5924e733344590a2d76cd2ff81e199b23f83'),
    ('Placement.FormationFriendlyCount', 0x2c433f0, 184, '4906e83b115c6514f33383cb3c172897b42ca41380facd5fd9f11b46adad1048'),
    ('Placement.UnitFriendlyCount', 0x2c432a0, 322, 'c78099a9a470bf120e00faa147c00660d78abdb0603819d5020fde18450899e4'),
]
CALLS = [
    (0x2c27361, 0x1f2fb30), (0x2c27945, 0x1e750e0),
    (0x2c27954, 0x1f567e0), (0x2c2795f, 0x1e6bce0),
    (0x213cc9b, 0x1e43290),
    (0x2dac95e, 0x1fa4020), (0x2dac96e, 0x1f58db0),
    (0x2dac9db, 0x1f60130), (0x2dac9eb, 0x1e6f280),
    (0x2daca13, 0x1fb0a50),
    (0x2c4345c, 0x1f331b0), (0x2c4346c, 0x1ecadb0),
    (0x2c43481, 0x1e35a90), (0x2c43321, 0x1fb8b98),
    (0x2c43343, 0x1eb91d0), (0x2c43399, 0x1e723a0),
]
def sha(data):
    return hashlib.sha256(data).hexdigest()
def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--module', type=Path, default=ROOT.parent / 'c6-assets-linux/libGameCore_XP2.so')
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    elf = Elf(a.module, GAME_SHA)
    rows = []
    for name, address, size, fingerprint in METHODS:
        if sha(elf.read(address, size)) != fingerprint:
            raise ValueError('method mismatch: ' + name)
        rows.append(dict(name=name, address=hex(address), size=size, sha256=fingerprint))
    for address, target in CALLS:
        elf.call(address, target)
    # Compare operators and getter offsets; not merely symbol-name discovery.
    for address, expected in [(0x2c27366, '85c00f9fc0'),
                              (0x2c2c2c0, '8b8728050000c3'),
                              (0x1fe2b30, '8b073b060f9fc0c3'),
                              (0x2dac9f0, '4139c6772a'),
                              (0x2c43486, '4439f8440f47f8')]:
        if elf.read(address, len(expected)//2).hex() != expected:
            raise ValueError('branch mismatch: ' + hex(address))
    source = ['crates/c6-core/src/unit_operation_legality.rs', 'crates/c6-core/src/skip_turn.rs',
              'crates/c6-core/examples/skip_turn_kernel.rs', 'tools/linux/unit_operation_legality_audit.py']
    report = dict(schema='c6-linux-unit-operation-legality-v1', checks_passed=True,
        gamecore_sha256=GAME_SHA, original_execution_performed=False,
        methods=rows, checked_call_edges=[dict(address=hex(x), target=hex(y)) for x,y in CALLS],
        source_sha256={x: sha((ROOT/x).read_bytes()) for x in source},
        busy='signed typed operation timer > 0; evaluated after Clear in replacement phase',
        can_move='signed raw FixedPoint<8> moves > 0, including positive fractions',
        handler='ordered IsHuman; human fortify/activity/count/unsigned-limit; INTERCEPT; espionage==-1',
        handler_is_full_manager_can_start=False,
        handler_native_boundary_fixtures_available=False,
        live_providers=['actual player IsHuman', 'actual layered Formation friendly count', 'PLOT_UNIT_LIMIT'],
        placement_count='Formation returns unsigned maximum of per-member counts; missing/dead members skipped; per-unit requires war, type(false), cargo, trade/spy, combat rivalry',
        full_manager_gaps=['tactical/turn/automation/pause controls', 'interruption', 'disabled operation', 'ordered scripting veto/effects', 'actual plot and registered handler'],
        engine_gameplay_apply_supported=False, complete_state_action_turns_supported=False,
        historical_raw_save_load_failure_bytes=89198)
    a.output.parent.mkdir(parents=True, exist_ok=True)
    a.output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True, original_methods=len(rows), checked_call_edges=len(CALLS))))
if __name__ == '__main__':
    main()
