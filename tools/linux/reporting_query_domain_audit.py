#!/usr/bin/env python3
"""Derive monitor plot-query domains from pinned original code and typed Map; no execution."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

from map_state_audit import Reader
from save_lifecycle_audit import Elf, GAME_SHA

ROOT = Path(__file__).resolve().parents[2]
MAP_SHA = 'cd2ec75b4e342a6ebccc1b96889335352657b9ea67f83d5f6d56f6a94eaf2de9'
PARSER_SHA = 'f43b2712ef67a4e2fdb1ab0ba578bc4796adc6241d232cb020ed9a35cf13f96b'
SNAPSHOT_SHA = '076f4f86c611ce5e99cde882552bcde70e2bc5394bc2675edb9ec3dc961e635e'
BODY_SHA = '0f1a191dda92973708f6ec1de0ec41dbb829c192651d4d84aca1310738fb2dd2'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def audit(args):
    for path, expected in [(args.native_map, MAP_SHA), (args.snapshot, SNAPSHOT_SHA),
                           (ROOT / 'tools/linux/map_state_audit.py', PARSER_SHA)]:
        if digest(path) != expected:
            raise ValueError('input identity differs: ' + str(path))
    image = Elf(args.module, GAME_SHA)
    body = image.read(0x2c665f0, 0x14d)
    if hashlib.sha256(body).hexdigest() != BODY_SHA:
        raise ValueError('original plot-loop body differs')
    gates = [(0x2c6662d, '31db', 'first_plot_zero'),
             (0x2c66674, '4531f6', 'first_yield_zero'),
             (0x2c66680, '31c9', 'GetYield_bool_false'),
             (0x2c66685, '4489f6', 'yield_index_argument'),
             (0x2c66688, '89ea', 'local_observer_argument'),
             (0x2c6668a, 'e881aa20ff', 'GetYield_before_cache_compare'),
             (0x2c666b2, '4183c601', 'next_yield'),
             (0x2c666c0, '7cbe', 'yield_bound_loop'),
             (0x2c666ff, '4883c301', 'next_plot'),
             (0x2c66708, '0f8532ffffff', 'plot_count_bound_loop')]
    for address, expected, _ in gates:
        if image.read(address, len(bytes.fromhex(expected))).hex() != expected:
            raise ValueError('original query-domain gate differs')
    reader = Reader(args.native_map.read_bytes())
    value = reader.map()
    if reader.pos != reader.end:
        raise ValueError('Map not fully consumed')
    plots = value['plots']
    snapshot = json.loads(args.snapshot.read_text())
    if len(plots) != len(snapshot['plots']):
        raise ValueError('Map and captured cache dimensions differ')
    owner = lambda plot: plot['city']['owner'] if plot['city'] is not None else -1
    properties = [plot['properties'] for plot in plots if plot['properties'] is not None]
    empty_improvement_maps = all(
        len(prop['entries']) == 1 and prop['entries'][0]['key'] == 0xd9a34c75 and
        prop['entries'][0]['value']['kind'] == 10 and
        prop['entries'][0]['value']['value']['payload']['entries'] == []
        for prop in properties)
    projection = [dict(index=i, owner=owner(plot), improvement_hash=plot['improvement_hash'],
                       properties=plot['properties']) for i, plot in enumerate(plots)]
    domains = Counter(('unowned' if owner(plot) == -1 else 'owned') + '_' +
                      ('improved_hash' if plot['improvement_hash'] != 0xffffffff else 'no_improvement_hash')
                      for plot in plots)
    return dict(schema='c6-linux-reporting-derived-query-domain-v1', source_sha256=digest(Path(__file__)),
                gamecore_sha256=GAME_SHA, native_map_sha256=MAP_SHA, map_parser_sha256=PARSER_SHA,
                failed_interval_snapshot_sha256=SNAPSHOT_SHA, map_bytes=reader.end,
                original_body=dict(rva='0x2c665f0', bytes=0x14d, sha256=BODY_SHA),
                gates=[dict(rva=hex(a), bytes=b, name=n) for a,b,n in gates],
                evidence_kind='Static original loop plus typed pre-action Map and captured cache; not directly traced calls',
                query_domain=dict(plot_count=len(plots), all_plot_indices_in_ascending_order=True,
                    owner_visibility_or_cache_value_prefilter=False, yields='0 <= yield index < actual NUM_YIELD_TYPES',
                    fallback_player='original local observer', bool_argument=False,
                    captured_cache_entry_size_counts=dict(sorted(Counter(str(x['size']) for x in snapshot['plots']).items())),
                    actual_NUM_YIELD_TYPES_independently_captured=False),
                plot_domain_counts=dict(sorted(domains.items())), property_bearing_plots=len(properties),
                all_serialized_property_maps_only_improvement_key_with_empty_nested_map=empty_improvement_maps,
                query_attribute_projection_sha256=hashlib.sha256(json.dumps(projection,sort_keys=True,separators=(',',':')).encode()).hexdigest(),
                conclusions=['All plot indices are in the original loop domain regardless of ownership or cached zero yields',
                    'Unowned and improved domains are nonempty; neither can justify excluding their conditional rule branches',
                    'Serialized empty nested property contents do not identify actual live nested virtual tables'],
                original_code_executed=False, full_action_acceptance=False,
                limits=['Map is pre-action state, not a direct plot-by-plot runtime call trace',
                    'Yield definition guards and deeper rule branches require their own contracts',
                    'Snapshot equality does not prove transitive purity or complete gameplay/RNG state',
                    'Third capture failed in GDB after monitor return; normal action completion remains unproven'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--module',type=Path,default=ROOT.parent/'c6-assets-linux/libGameCore_XP2.so')
    parser.add_argument('--native-map',type=Path,default=Path('/tmp/c6-core-map-indexed-1.bin'))
    parser.add_argument('--snapshot',type=Path,default=ROOT.parent/'c6-ghidra/evidence/linux-reporting-trade-effects/native-passive-3/monitor-0001-before.json')
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    args.output.write_text(json.dumps(audit(args),indent=2,sort_keys=True)+'\n')


if __name__ == '__main__':
    main()
