#!/usr/bin/env python3
"""Independent bounded local-gameplay scope checks; no original execution."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile

from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

ROOT = Path(__file__).resolve().parents[2]
DIRECT = {
    0x39D553C: 0x3C45E82,  # sync packet full-data hash
    0x39D5559: 0x3C45F34,  # sync packet dirty indices
    0x39DAE1D: 0x3C45C2C,  # snapshot delta union output
    0x39D82F7: 0x3C46036,  # snapshot live wrapper input
    0x39D836D: 0x3C470FC,  # selected desync callback lookup
    0x39D98F7: 0x3C45D18,  # clear registered archives
    0x32B7824: 0x32B6280,  # Lua Create -> receiver vector construction
}
INDIRECT = {
    0x3C45D3B: bytes.fromhex('ff5038'),  # typed clearDelta virtual slot
    0x3C4615F: bytes.fromhex('ff5218'),  # live wrapper delta reader
    0x39D83BE: bytes.fromhex('ffd1'),    # execute selected desync callback
}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    host = Elf(args.assets / 'Civ6', HOST_SHA)
    Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    edges = []
    for site, expected in DIRECT.items():
        instruction = host.read(site, 5)
        actual = site + 5 + struct.unpack('<i', instruction[1:])[0]
        if instruction[0] != 0xE8 or actual != expected:
            raise ValueError('direct target differs at ' + hex(site))
        edges.append(dict(site=hex(site), target=hex(actual)))
    for site, expected in INDIRECT.items():
        if host.read(site, len(expected)) != expected:
            raise ValueError('indirect dispatch differs at ' + hex(site))
    audits = {}
    with tempfile.TemporaryDirectory(prefix='c6-local-scope-review-') as temporary:
        for name in ['load_archive_gameplay_scope', 'reporting_gameplay_scope']:
            output = Path(temporary) / (name + '.json')
            subprocess.run(['python3', str(ROOT / 'tools/linux' / (name + '_audit.py')),
                            '--assets', str(args.assets), '--output', str(output)],
                           capture_output=True, text=True, check=True)
            report = json.loads(output.read_text())
            audits[name] = dict(
                sha256=digest(output), gamecore_sha256=report['gamecore_sha256'],
                host_sha256=report['host_sha256'])
            if name == 'load_archive_gameplay_scope':
                audits[name].update(checks=report['checks'],
                                    typed_clear_slots=len(report['typed_clear_slots']),
                                    host_direct_consumers=len(report['direct_consumers']['Civ6']),
                                    xp2_direct_consumers=len(report['direct_consumers']['libGameCore_XP2.so']))
            else:
                audits[name].update(methods=len(report['methods']),
                                    selected_direct_references=len(report['selected_direct_references']),
                                    lua_sources=report['lua_sources'],
                                    captured_send_inventory=report['skip_turn_dirty_consumer_classification'])
    source_paths = ['evidence/linux/skip-turn-reporting-closure.json',
                    'evidence/linux/reporting-provider-passive-1.json',
                    'docs/linux-reporting-effects-observation.md',
                    'tools/linux/local_gameplay_scope_review.py']
    result = dict(
        schema='c6-linux-local-gameplay-scope-review-v1',
        reviewed_commits=['bad86dafcbd08e1f55ca2f811e97fbbd86964e90',
                          '08998a5105b4bedac093730bd44bced079aa0a50'],
        gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA,
        independent_direct_target_checks=edges,
        indirect_dispatch_sites=[hex(site) for site in INDIRECT],
        rerun_audits=audits,
        source_sha256={path: digest(ROOT / path) for path in source_paths},
        manually_reviewed_stock_functions=[
            'UnitFlag.GetUnit', 'UnitFlag.UpdateReadyState', 'UnitFlag.SetDim',
            'UnitFlag.UpdateDimmedState', 'UnitFlag.UpdateFlagType', 'OnEventPlaybackComplete'],
        native_getters_not_blanket_classified_pure=[
            'IsHuman', 'IsReadyToSelect', 'IsEmbarked', 'GetFortifyTurns'],
        acceptance=dict(
            bounded_metadata_exclusions=True,
            native_resync_unsupported_required=True, local_authoritative_state_required=True,
            stock_profile_without_added_gameplay_dirty_consumer_required=True,
            source_hashes_prove_active_profile=False,
            indirect_inline_whole_program_absence_proven=False,
            gameplay_callbacks_reports_order_rng_remain_required=True,
            original_execution=False, native_parity=False, complete_action_turn_parity=False))
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print('bounded static scope checks passed; no original execution or action acceptance')


if __name__ == '__main__':
    main()
