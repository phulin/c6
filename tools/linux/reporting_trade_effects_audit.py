#!/usr/bin/env python3
"""Exact-target static Trade reporting effects plus read-only production-codec inspection.

No original code executes. This is not a Trade rules or pathfinding implementation.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

from save_lifecycle_audit import Elf, GAME_SHA

ROOT = Path(__file__).resolve().parents[2]
BEFORE_SHA = 'ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884'
METHODS = [
    ('Monitor.CheckTrade', 0x2c668b0, 0x52), ('Monitor.RouteCallback', 0x2c66a30, 0x186),
    ('Route.OriginYield', 0x2c11920, 0x72), ('Route.DestinationYield', 0x2c119a0, 0x72),
    ('Rules.OriginYield', 0x2be3b70, 0x26b), ('Rules.DestinationYield', 0x2be4120, 0x257),
    ('Manager.OriginModifiersScalar', 0x2c09600, 0xed), ('Manager.DestinationModifiersScalar', 0x2c0ac80, 0xed),
    ('Manager.OriginModifiers', 0x2c096f0, 0x139d), ('Manager.DestinationModifiers', 0x2c0ad70, 0x6ed),
    ('Manager.OriginPathYield', 0x2c08a80, 0x42f), ('Manager.DestinationPathYield', 0x2c095b0, 0x10),
    ('Manager.ExistingPath', 0x2c08eb0, 0x4f), ('Manager.FindNewPath', 0x2c06210, 0x204),
    ('Manager.CountPathFeatures', 0x2c08f00, 0x237), ('Manager.IterateRoutes', 0x2c66d40, 0xc),
    ('Graph.IterateRoutes', 0x2bfa860, 0xa6), ('Graph.CityPath', 0x2bfbbe0, 0x68),
    ('Graph.Path', 0x2bfbc50, 0x2e), ('Graph.Edge', 0x2bfc240, 0x8c),
    ('Movement.AcquireFinder', 0x2c10240, 0x42), ('Movement.TestPathPool', 0x2c10620, 0x62),
    ('Movement.TestPathFinder', 0x2c10690, 0x29), ('Movement.GeneratePath', 0x2c106c0, 0x64),
    ('Pathfinder.GeneratePath', 0x2a50940, 0x42b), ('Pathfinder.TryLock', 0x2a50920, 0xc),
    ('Pathfinder.ReleaseLock', 0x2a50930, 0xc), ('InitializeOnUse.Get', 0x2c0d740, 0x29),
] + [(f'Origin.LazyCount{i}', a, n) for i, (a, n) in enumerate([
    (0x2c0ca40, 0x27), (0x2c0cb40, 0x17), (0x2c0cc30, 0x17), (0x2c0cd20, 0x27),
    (0x2c0ce20, 0x17), (0x2c0cf10, 0x17), (0x2c0d000, 0x17)])]
GATES = [
    (0x2c11965, 'c7042400000000', 'Origin route getter passes includeTempRoute=false'),
    (0x2c119e5, 'c7042400000000', 'Destination route getter passes includeTempRoute=false'),
    (0x2c09701, '44894c2410', 'Origin modifier saves bool R9 to stack10'),
    (0x2c09a28, '807c241000', 'Origin tests includeTempRoute'),
    (0x2c09a32, '0f84e2010000', 'False skips route edits, temporary allocation and game signal'),
    (0x2c0ad81, '44894c2428', 'Destination modifier saves bool R9 to stack28'),
    (0x2c0aed6, '807c242800', 'Destination tests includeTempRoute'),
    (0x2c0aee0, '0f8434010000', 'False skips route edits, temporary allocation and game signal'),
    (0x2c08b17, '7440', 'Absent forward path enters local FindNewPath branch'),
    (0x2c062ad, '0f848c000000', 'Absent reverse path enters pooled pathfinder branch'),
    (0x2c106d0, '8993d0030000', 'Path generation stores player in actual reusable finder'),
    (0x2bfa8cb, '80be4001000000', 'Native ID-map traversal filters live active-route bool'),
    (0x2c66a8c, '4d3b67200f8412010000', 'Monitor only checks lower_bound result against end'),
    (0x2c0d751, 'c6433401', 'Lazy local initialized bit store'),
    (0x2c0d75d, '894330', 'Lazy local computed value store'),
]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inspect_graph(before):
    data = before.read_bytes()
    if hashlib.sha256(data).hexdigest() != BEFORE_SHA:
        raise ValueError('requires exact accepted before-Core stream')
    graph_bytes = data[71027800:71497438]
    codec = ROOT / 'crates/c6-core/src/trade.rs'
    codec_review_path = ROOT / 'evidence/linux/trade-rust-codec-1.json'
    codec_review = json.loads(codec_review_path.read_text())
    if digest(codec) != codec_review['source_sha256']['crates/c6-core/src/trade.rs']:
        raise ValueError('production Trade codec differs from native-boundary-reviewed source')
    source = '''#[path=CODEC_PATH] mod trade;
fn main(){let b=std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
let g=trade::Graph::decode(&b,trade::Limits::default()).unwrap();
assert_eq!(g.encode(trade::Limits::default()).unwrap(),b);
let(mut active,mut missing,mut nodes)=(0,0,0);
for o in g.origins(){for d in &o.destinations{if d.edge.has_active_route{
active+=1;missing+=usize::from(d.edge.most_recent_path.is_empty());nodes+=d.edge.most_recent_path.len();}}}
println!("{{\\\"edges\\\":{},\\\"active_routes\\\":{},\\\"active_routes_without_forward_path\\\":{},\\\"active_path_nodes\\\":{}}}",g.edge_count(),active,missing,nodes);}
'''.replace('CODEC_PATH', json.dumps(str(codec)))
    with tempfile.TemporaryDirectory(prefix='c6-reporting-trade-') as temporary:
        directory = Path(temporary)
        (directory / 'inspect.rs').write_text(source)
        (directory / 'graph.bin').write_bytes(graph_bytes)
        subprocess.run(['rustc', '--edition=2021', '-O', '--cap-lints', 'allow',
                        str(directory / 'inspect.rs'), '-o', str(directory / 'inspect')], check=True)
        result = json.loads(subprocess.check_output([str(directory / 'inspect'), str(directory / 'graph.bin')]))
    result.update(core_sha256=BEFORE_SHA, graph_sha256=hashlib.sha256(graph_bytes).hexdigest(),
                  codec_sha256=digest(codec), codec_review_sha256=digest(codec_review_path),
                  exact_production_codec_roundtrip=True,
                  graph_range=[71027800, 71497438], native_lookup_hit_observed=False,
                  live_idmap_order_observed=False,
                  scope='Captured typed serialized graph. Nonempty paths establish the supported forward-hit input domain, not observed execution of every live lookup.')
    return result


def audit(args):
    image = Elf(args.assets / 'libGameCore_XP2.so', GAME_SHA)
    for address, expected, name in GATES:
        if image.read(address, len(bytes.fromhex(expected))).hex() != expected:
            raise ValueError('original branch differs: ' + name)
    raw = args.private_evidence
    dwarf = (raw / 'types.dwarf').read_text()
    path_dwarf = (raw / 'pathfinder-types.dwarf').read_text()
    arg_dwarf = (raw / 'args.dwarf').read_text()
    for needle in ['m_kGraph', 'm_kEdgeMap', 'm_kOriginMap', 'm_tempRoutes', 'm_axOriginYields', 'm_TradeYields']:
        if needle not in dwarf:
            raise ValueError('missing exact-target DWARF owner: ' + needle)
    for needle in ['m_ePlayer', 'm_pOpen', 'm_pClosed', 'm_pBest', 'm_ppaaNodes', 'm_kLock']:
        if needle not in path_dwarf:
            raise ValueError('missing finder DWARF field: ' + needle)
    if 'Symbol includeTempRoute' not in arg_dwarf or 'DW_OP_breg7 16 [$rsp]' not in arg_dwarf:
        raise ValueError('missing bool parameter DWARF location')
    fixture = inspect_graph(args.core_before)
    return dict(schema='c6-linux-reporting-trade-effects-v1', checks_passed=True,
        original_code_executed=False, static_only_except_production_codec=True,
        gamecore_sha256=GAME_SHA, source_audit_sha256=digest(Path(__file__)),
        methods=[dict(name=n, rva=hex(a), bytes=z, sha256=hashlib.sha256(image.read(a, z)).hexdigest()) for n, a, z in METHODS],
        branches=[dict(rva=hex(a), bytes=b, result=n) for a, b, n in GATES],
        dwarf_sha256={name: digest(raw / name) for name in ['types.dwarf', 'pathfinder-types.dwarf', 'args.dwarf']},
        fixture=fixture,
        accepted_caller=dict(includeTempRoute=False, origin_destination_yields_recomputed=True,
            route_edit_temp_create_game_signal_branch_reachable=False,
            reason='Actual Route getters pass false through Rules scalar and vector modifier wrappers; exact bool branches bypass the complete mutating block.',
            true_branch_retained_in_scope=True),
        iteration=dict(order='Native Graph ID-map forward iteration; callback only when Edge live active bool+0x140 true',
            graph_manager_offset='0xc8', graph_id_map_offset='0xd0', route_edge_offset='0xc0',
            monitor='Lower_bound over current sorted monitor trade cache; stop only at end. No separate equality check appears before using returned record.',
            cache_key_state='Preserve origin/destination CityID ordering and complete current cache; do not sort routes or rebuild monitor cache from wire order'),
        existing_path=dict(graph_origin_map_offset='0xf8', edge_path_value_offset='0x188',
            effect='Lookup returns existing nonempty tracked path vector; lookup and hit body do not edit graph/path.',
            supported_domain='Valid cities and successful forward graph lookup with nonempty path; fixture has182 active routes, all nonempty forward paths.',
            graph_path_write_on_hit=False, pathfinder_invoked_on_hit=False),
        miss=dict(reverse_path='FindNewPath first clears caller vector, checks actual map/cities, then looks up destination->origin; reverse existing path is copied in reverse order into caller output.',
            cold='If reverse lookup also fails: acquire actual pooled finder, TestPath with flags0, copy result indices backward into caller local vector, release finder lock.',
            graph_path_insertion=False,
            persistent_mutations=['Finder critical section+0x180 lock/unlock', 'Trade finder player+0x3d0',
                'Pathfinder start/destination/info+0x120..0x130', 'Open/closed/best pointers+0x140/+0x150/+0x158, stack+0x160, node data via+0x178',
                'Force-reset/abort bytes+0x136/+0x139 and callback-owned state'],
            complete_cold_path_effects_proven=False,
            smallest_remaining_branch='Absent forward AND reverse graph paths enters actual pooled Pathfinder.GeneratePath delegate/Step closure. Actual registrations, node state and complete transitive/RNG effects need evidence; unsupported before mutation.'),
        outputs=dict(destination_path_yield='Exactly FixedPoint(0)',
            direct_monitor_mutation='Compare computed fixed8 origin/destination amounts with monitor cached lists; assign changed values; dispatch route-change reporting only when a comparison changed.',
            reporting_event='0x14deda5a, origin/destination ComponentIDs; synchronous downstream subscribers remain exact dependencies',
            direct_route_yield_array_writes_in_false_branch=False,
            local_lazy_caches='Seven InitializeOnUse<int> objects live on scalar call stack. Get sets its own initialized byte+0x34/value+0x30 and invokes known district/resource count closure. No persistent route cache is established by those locals.'),
        rng=dict(direct_rng_calls_in_inspected_trade_bodies=False,
            complete_transitive_no_rng_proof=False,
            scope='False includeTempRoute excludes its game-signal branch; forward-hit excludes finder delegates. Cold path generic callbacks cannot be declared RNG-free from names or lack of a direct Random call.'),
        required_rust_inputs=['Actual active-route ID-map iteration order, Route endpoints/resource identity and live active flags',
            'Complete ordered monitor TradeYields cache with current fixed8 values and yield-definition order',
            'Current city/player trade modifiers, resources/district counts, religion, diplomacy/alliance/suzerain, culture/progress and congress rules',
            'Graph lookup indexes and complete forward path nodes; current map/city/trading-post/road/water/domain data and global transportation parameters',
            'Actual reporting environment for changed-route events',
            'For unsupported double-miss: real pool state, lock/selection order, complete pathfinder nodes/delegate bindings and affected RNG/state'],
        performance='Preserve route ID-map/yield-definition order. Native scalar wrappers use stack fixed-buffer yield arrays and onlyYield filtering; local count caches last one invocation. A forward hit reads the retained graph path without rebuilding a path or touching the finder pool.',
        limitations=['No native run or new callback/value observations', 'Serialized active path coverage is not a live hash-table-order/lookup-hit observation',
            'General modifier/feature getters remain exact rules inputs; this audit does not implement their arithmetic or universally prove all transitive getter purity',
            'Full affected-state/RNG and reporting subscribers remain required',
            'Complete Rust gameplay state/actions/turns remain unimplemented; raw Save-to-Load89198-byte failure remains unclassified and unmasked'],
        rust_action_admission=False, full_host_admission=False, gameplay_parity=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--private-evidence', type=Path, default=ROOT.parent / 'c6-ghidra/evidence/linux-reporting-trade-effects')
    parser.add_argument('--core-before', type=Path, default=ROOT.parent / 'c6-ghidra/evidence/linux-reporting-provider/native-passive-2/before.bin')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    report = audit(args)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')


if __name__ == '__main__':
    main()
