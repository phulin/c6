#!/usr/bin/env python3
"""Independent review runner: existing original Save bytes, unchanged exported archive.
No original code execution. Existing native contract audits remain explicit inputs;
this review adds exact (unhashed) Targets scalar projection comparison.
"""
import argparse
import gzip
import hashlib
import json
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_PIN = 'eb5bbd03bdb9fbf5394a7a45a29240a3ac8b8eb5'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def run(command, **kw):
    return subprocess.check_output(command, **kw)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--source-commit', default=DEFAULT_PIN)
    ap.add_argument('--output', type=Path, required=True)
    args = ap.parse_args()
    review_driver_sha = sha(Path(__file__).read_bytes())
    pin = run(['git', 'rev-parse', args.source_commit + '^{commit}'], cwd=ROOT, text=True).strip()
    archive = run(['git', 'archive', pin], cwd=ROOT)
    host_sha = sha((ROOT.parent / 'c6-assets-linux/Civ6').read_bytes())
    assert host_sha == '106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
    with tempfile.TemporaryDirectory(prefix='c6-targets-behavior-review-') as temporary:
        tmp = Path(temporary)
        source = tmp / 'source'
        source.mkdir()
        subprocess.run(['tar', '-x', '-C', str(source)], input=archive, check=True)
        paths = run(['git', 'ls-tree', '-r', '--name-only', pin], cwd=ROOT, text=True).splitlines()
        identities = {}
        for name in paths:
            path = source / name
            if path.is_symlink():
                continue
            expected = run(['git', 'show', pin + ':' + name], cwd=ROOT)
            assert path.read_bytes() == expected, name
            identities[name] = sha(expected)
        lib = (source / 'crates/c6-core/src/lib.rs').read_text()
        for module in ['player_ai_targets', 'player_ai_behavior_tree']:
            assert 'pub mod ' + module + ';' in lib
        # Import grammar/independent native evidence audits from the SAME archive.
        sys.path.insert(0, str(source / 'tools/linux'))
        import player_ai_targets_audit as targets
        import player_ai_behavior_tree_audit as behavior
        native_reports = {}
        for name in ['player_ai_targets', 'player_ai_behavior_tree']:
            output = tmp / (name + '.json')
            run([sys.executable, str(source / ('tools/linux/' + name + '_audit.py')),
                 '--private-root', str(ROOT.parent / 'c6-ghidra'),
                 '--' + ('assets' if name == 'player_ai_targets' else 'binary'),
                 str(ROOT.parent / ('c6-assets-linux' if name == 'player_ai_targets' else 'c6-assets-linux/libGameCore_XP2.so')),
                 '--output', str(output)])
            native_reports[name] = json.loads(output.read_text())
        test_outputs = {}
        for filter_name, expected_count in [('player_ai_targets', 3), ('player_ai_behavior_tree', 4)]:
            test_output = run(['cargo', 'test', '--offline', '-p', 'c6-core', filter_name], cwd=source, text=True)
            assert f'{expected_count} passed; 0 failed' in test_output
            test_outputs[filter_name] = test_output
        run(['cargo', 'clippy', '--offline', '-p', 'c6-core', '--all-targets', '--', '-D', 'warnings'], cwd=source)
        run(['cargo', 'build', '--offline', '--release', '-p', 'c6-core'], cwd=source)
        wires = {name: bytearray() for name in ['targets', 'behavior']}
        projections = {name: bytearray() for name in wires}
        records = {name: [] for name in wires}
        nodes = set()
        for identity, trace_name, trace_sha in targets.FIXTURES:
            raw = gzip.decompress((ROOT.parent / 'c6-ghidra/evidence/linux-game-load/canonical' / (identity + '.bin.gz')).read_bytes())
            trace = gzip.decompress((ROOT.parent / 'c6-ghidra/evidence/linux-save-normalization/checkpoint-203-owners' / trace_name).read_bytes())
            assert sha(raw) == identity and sha(trace) == trace_sha
            owners = {x['call']: x for x in map(json.loads, trace.splitlines()) if x.get('event') == 'owner' and x.get('save') == 1}
            for key, owner_name in [('targets', 'GameCore::AI::Targets'), ('behavior', 'GameCore::AI::BehaviorTree::Manager')]:
                for row in sorted(owners.values(), key=lambda x: x['start']):
                    if row['owner'] != owner_name:
                        continue
                    blob = raw[row['start']:row['end']]
                    parent = owners[row['parent']]
                    assert parent['owner'] == 'GameCore::Player::Instance'
                    version, player = struct.unpack_from('<Ii', raw, parent['start'])
                    assert version == 47
                    if key == 'targets':
                        value, items, values, _, cuts, used, hostile, _ = targets.projection(blob)
                        assert used == len(blob) and value['player_id'] == player
                        assert struct.unpack_from('<Ii', raw, row['end']) == (1, player)
                        projections[key] += struct.pack('<' + 'q' * len(values), *values)
                        wires[key] += struct.pack('<I', len(blob)) + blob + struct.pack('<II', items, len(cuts)) + b''.join(struct.pack('<I', c) for c in cuts) + struct.pack('<I', len(hostile)) + b''.join(struct.pack('<I', c) for c in hostile)
                        mutations = len(hostile)
                    else:
                        reader = behavior.Reader(blob)
                        value = reader.read('BehaviorTreeManager')
                        assert reader.pos == len(blob) and dict(value.fields)['player_id'] == player
                        items = reader.items
                        nodes.update(reader.node_counts)
                        ordered = sorted(x for x in reader.cuts if x < len(blob))
                        cuts = sorted(set(range(min(40, len(blob)))) | {ordered[i * len(ordered) // 64] for i in range(64)} | {i * len(blob) // 64 for i in range(64)})
                        versions = sorted(set(reader.versions))
                        versions = [versions[i * len(versions) // min(32, len(versions))] for i in range(min(32, len(versions)))]
                        counts = sorted(set(reader.counts))
                        counts = [counts[i * len(counts) // min(32, len(counts))] for i in range(min(32, len(counts)))]
                        wires[key] += struct.pack('<III', len(blob), items, len(cuts)) + b''.join(struct.pack('<I', c) for c in cuts) + struct.pack('<I', len(versions)) + b''.join(struct.pack('<I', c) for c in versions) + struct.pack('<I', len(counts)) + b''.join(struct.pack('<I', c) for c in counts) + blob
                        projections[key] += (behavior.debug(value) + '\n').encode()
                        mutations = len(versions) + len(counts)
                    records[key].append(dict(stream=identity, start=row['start'], end=row['end'], bytes=len(blob), items=items, cuts=len(cuts), mutations=mutations, sha256=sha(blob)))
        # Retain every typed scalar exactly rather than reducing Targets to FNV64.
        harness = targets.HARNESS
        old_mix = 'fn mix(h:&mut u64,v:i64){for b in v.to_le_bytes(){*h=(*h^u64::from(b)).wrapping_mul(1099511628211)}}'
        assert old_mix in harness
        harness = harness.replace(old_mix, 'fn mix(h:&mut Vec<i64>,v:i64){h.push(v)}').replace('h:&mut u64', 'h:&mut Vec<i64>')
        harness = harness.replace('let mut digest=14695981039346656037u64;', 'let mut digest=Vec::<i64>::new();')
        harness = harness.replace('println!("{records} {digest}");', 'let out:Vec<u8>=digest.iter().flat_map(|v|v.to_le_bytes()).collect();std::fs::write(&args[2],out).unwrap();println!("{records}");')
        results = {}
        for key, driver in [('targets', harness), ('behavior', behavior.HARNESS)]:
            driver_path = tmp / (key + '.rs')
            driver_path.write_text(driver)
            wire_path = tmp / (key + '.wire')
            wire_path.write_bytes(wires[key])
            executable = tmp / key
            run(['rustc', '--edition=2021', '-Dwarnings', '-O', str(driver_path), '--extern', 'c6_core=' + str(source / 'target/release/libc6_core.rlib'), '-L', 'dependency=' + str(source / 'target/release/deps'), '-o', str(executable)])
            projection_path = tmp / (key + '.projection')
            assert int(run([str(executable), str(wire_path), str(projection_path)], text=True)) == 86
            actual = projection_path.read_bytes()
            assert actual == projections[key], key + ' full projection disagreement'
            results[key] = dict(records=records[key], full_projection_bytes=len(actual), full_projection_sha256=sha(actual), projection_equality='full bytes compared, not digest-only', harness_sha256=sha(driver.encode()), executable_sha256=sha(executable.read_bytes()))
        # Exercise graph-looking values without treating them as live graphs.
        graph_driver = r'''use c6_core::{player_ai_behavior_tree::*,unit::Limits};
fn main(){let lim=Limits{max_bytes:1_000_000,max_items:10000};
let mut v=BehaviorTreeManager::default();
v.blackboard.ints.data=vec![BlackboardData{value:i32::MIN,next_unused_index:0},BlackboardData{value:i32::MAX,next_unused_index:0}];
v.blackboard.ints.first_unused_index=0;
v.blackboard.bools.data.push(BlackboardData{value:BoolByte(255),next_unused_index:-1});
let definitions=NODE_SCHEMAS.iter().map(|(hash,_,fields,_)|NodeDefinition{shape:i32::MIN,node_hash:*hash,fields:fields.iter().map(|f|match f{FieldKind::Shared=>NodeValue::Shared(SharedData{data_type:i32::MIN,index:u32::MAX}),FieldKind::Word=>NodeValue::Word(u32::MAX),FieldKind::Byte=>NodeValue::Byte(BoolByte(255)),FieldKind::Plot=>NodeValue::Plot(PlotCoord{x:i32::MIN,y:i32::MAX})}).collect(),num_shared_data:u32::MAX}).collect();
v.behavior_trees.entries.push(BehaviorTreeEntry{index:i32::MIN,tree:BehaviorTree{definitions,decision_states:vec![DecisionState{first_child:0,jump:0,last_child:u32::MAX,parent:0,start_at:0}],..BehaviorTree::default()}});
v.behavior_trees.free_indices=vec![0,0,u32::MAX];
let(wire,items)=v.encode_with_items(lim).unwrap();
let exact=Limits{max_bytes:wire.len(),max_items:items};
assert_eq!(BehaviorTreeManager::decode(&wire,exact).unwrap(),v);
assert_eq!(v.encode_with_items(exact).unwrap(),(wire.clone(),items));
for cap in [0,1,2,3,items-1]{let small=Limits{max_items:cap,..exact};assert_eq!(BehaviorTreeManager::decode(&wire,small),Err(Error::ResourceLimit));assert_eq!(v.encode(small),Err(Error::ResourceLimit));}
let first_hash=NODE_SCHEMAS[0].0;let mut marker=Vec::new();marker.extend_from_slice(&3u32.to_le_bytes());marker.extend_from_slice(&i32::MIN.to_le_bytes());marker.extend_from_slice(&first_hash.to_le_bytes());marker.extend_from_slice(&33u32.to_le_bytes());
let offsets:Vec<_>=wire.windows(marker.len()).enumerate().filter_map(|(i,b)|if b==marker{Some(i)}else{None}).collect();assert_eq!(offsets.len(),1);
let mut unknown=wire.clone();unknown[offsets[0]+8..offsets[0]+12].copy_from_slice(&0u32.to_le_bytes());assert_eq!(BehaviorTreeManager::decode(&unknown,exact),Err(Error::UnsupportedNodeType(0)));
println!("{items}");}
'''
        graph_path = tmp / 'graph.rs'
        graph_path.write_text(graph_driver)
        graph_executable = tmp / 'graph'
        run(['rustc', '--edition=2021', '-Dwarnings', '-O', str(graph_path), '--extern', 'c6_core=' + str(source / 'target/release/libc6_core.rlib'), '-L', 'dependency=' + str(source / 'target/release/deps'), '-o', str(graph_executable)])
        graph_items = int(run([str(graph_executable)], text=True))
        for path, expected in identities.items():
            assert sha((source / path).read_bytes()) == expected, path
        report = dict(schema='c6-linux-player-ai-targets-behavior-independent-review-v1', checks_passed=True,
                      review_driver_sha256=review_driver_sha, source_commit=pin, archive_sha256=sha(archive), host_sha256=host_sha, all_archived_regular_sources=identities,
                      actual_exports=True, source_injection=False, original_execution=False,
                      cargo_test_outputs=test_outputs, strict_clippy_all_targets=True,
                      synthetic_graph_carrier_items=graph_items, synthetic_graph_harness_sha256=sha(graph_driver.encode()),
                      graph_semantics='cyclic child/free-list and aliased/out-of-range SharedData indices retained as wire values; no recursion/dereference',
                      recursion_depth_limit='not applicable: statically finite type grammar, no recursive wire node field',
                      targets=results['targets'], behavior=results['behavior'],
                      original_static_audit_reports=native_reports,
                      observed_node_dispatches=len(nodes), source_dispatches=len(behavior.NODE_CONTRACTS),
                      source_only_dispatches=sorted({v['name'] for v in behavior.NODE_CONTRACTS.values()} - nodes),
                      native_read_reconstruction=False, world_import=False, gameplay_state_acceptance=False,
                      policy_execution=False, planning_only_exclusion=False, action_turn_rng_acceptance=False,
                      unknown_bytes_masked=0, recorded_raw_save_load_difference_bytes=89198)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2) + '\n')
        print(json.dumps(dict(checks_passed=True, pin=pin, targets_records=86, behavior_records=86, actual_dispatches=len(nodes))))


if __name__ == '__main__':
    main()
