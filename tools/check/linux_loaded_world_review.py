#!/usr/bin/env python3
"""Offline consuming-World review against a second direct typed decode.

Uses the completed frozen build's rlibs; never builds or executes native code.
The extra decode intentionally makes this process unsuitable for memory/timing
measurements. Generated sources, executable and output belong in a private or
temporary work directory, since assertion diagnostics concern real game state.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess


def fields(root, path, name):
    text = (root / path).read_text()
    body = re.search(r"pub struct " + name + r"(?:<'a>)? \{\n(.*?)\n\}", text, re.S)
    if body is None:
        raise ValueError(f"missing struct {name}")
    return re.findall(r"^    pub ([a-z_]+):", body[1], re.M)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--deps", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path, required=True)
    args = parser.parse_args()
    root = args.source_root.resolve()
    work = args.work_dir.resolve()
    work.mkdir(parents=True, exist_ok=True)
    core = "crates/c6-core/src/"
    players = fields(root, core + "core_player.rs", "FirstPlayer")
    retained = fields(root, core + "loaded_components.rs", "RetainedPlayer")
    assert set(players) == set(retained) | {"formations"}
    units = fields(root, core + "player_units.rs", "Units")
    retained_units = fields(root, core + "loaded_components.rs", "RetainedUnits")
    assert set(units) == set(retained_units) | {"records"}
    core_fields = fields(root, core + "core_stream.rs", "CorePrefix")
    retained_core = fields(root, core + "loaded_components.rs", "OwnedCore")
    assert set(core_fields) == set(retained_core) | {"map", "tail"}
    checks = []
    for field in retained_core:
        checks.append(f'assert!(expected.global_ai.players.core.{field} == state.retained().core.{field}, "Core.{field}");')
    for field in ["religion"]:
        checks.append(f'assert!(expected.global_ai.players.player_prefix.{field} == state.retained().{field}, "{field}");')
    checks.append('assert!(expected.global_ai.global_ai == state.retained().global_ai, "GlobalAi");')
    for field in ["barbarian", "free_cities", "random"]:
        checks.append(f'assert!(expected.{field} == state.retained().{field}, "{field}");')
    player_checks = []
    for field in retained:
        if field != "units":
            player_checks.append(f'assert!(p.{field} == r.{field}, "Player.{field}");')
    for field in retained_units:
        player_checks.append(f'assert!(p.units.{field} == r.units.{field}, "Units.{field}");')
    review = "\n".join(checks) + '''
    assert!(expected.global_ai.players.core.map == *state.world().map(), "complete Map including ordered layers/references");
    assert!(expected.tail == state.unparsed_tail(), "unknown tail");
    let expected_players: Vec<_> = expected.global_ai.players.player_prefix.players().collect();
    assert_eq!(expected_players.len(), state.world().players().len());
    let mut review_busy = 0usize;
    let mut review_movable = 0usize;
    let mut review_units = 0usize;
    let mut review_formations = 0usize;
    for ((p, w), r) in expected_players.iter().zip(state.world().players()).zip(&state.retained().players) {
''' + "\n".join(player_checks) + '''
        assert_eq!(w.owner(), p.header.owner);
        let actual = w.units()?;
        let source = &p.units.records;
        assert_eq!(actual.capacity, source.capacity);
        assert_eq!(actual.current_id_bits, source.current_id_bits);
        assert_eq!(actual.free_head, source.free_head);
        assert_eq!(actual.free_tail, source.free_tail);
        assert!(actual.free_links == source.free_links, "Unit free links");
        assert_eq!(actual.entries.len(), source.entries.len());
        for (a, b) in actual.entries.iter().zip(&source.entries) {
            assert_eq!(a.index, b.index);
            assert!(a.value == b.value, "complete ordered Unit");
            let request = b.value.id ^ 0xffff0000;
            if request != u32::MAX {
                let found = w.unit(request)?.expect("low16 Unit");
                assert_eq!(found.resolved_id, b.value.id);
                assert!(std::ptr::eq(found.record, &a.value));
            }
            review_units += 1;
            review_busy += usize::from(b.value.i_operation_timer > 0);
            review_movable += usize::from(b.value.x_moves_remaining > 0);
        }
        assert!(*w.formations()? == p.formations, "complete ordered Formations and allocator");
        review_formations += p.formations.entries.len();
    }
    assert_eq!((expected_players.len(), review_units, review_formations), (43, 1262, 1250));
    let review_subject = state.world().players().iter().find_map(|p| p.unit_records().ok()?.first().map(|u| (p.owner(), u.value.id))).expect("fixture subject");
    let u = state.unit_mut(review_subject.0, review_subject.1)?;
    let saved = (u.i_operation_timer, u.x_moves_remaining);
    for n in [i32::MIN, -1, 0, 1, 255, i32::MAX] {
        u.i_operation_timer = n;
        u.x_moves_remaining = n;
        assert_eq!(c6_core::skip_turn::legality::is_busy(u), n > 0);
        assert_eq!(c6_core::skip_turn::legality::can_move(u), n > 0);
        assert_eq!((u.i_operation_timer, u.x_moves_remaining), (n, n));
    }
    (u.i_operation_timer, u.x_moves_remaining) = saved;
    assert!(state.unit_runtime(review_subject.0, review_subject.1).is_err(), "no invented initialization");
'''
    path = root / "crates/c6-differential/src/bin/c6-core-contextual-candidate.rs"
    source = path.read_text()
    marker = "    let source_pointer = bytes.as_ptr();"
    assert source.count(marker) == 1
    source = source.replace(marker, '''    // Review-only second decode; excluded from production measurements.
    let review_bytes = bytes.clone();
    let review_provider = capture.for_binding(&binding);
    let expected = c6_core::core_stream::ThroughRandom::decode(
        &review_bytes, shared, player_limits, binding.clone(), &review_provider)?;
''' + marker)
    marker = "    let assembly_seconds = start.elapsed().as_secs_f64();"
    assert source.count(marker) == 1
    source = source.replace(marker, review + marker)
    marker = '"schema":"c6-loaded-state-candidate-v3",'
    assert source.count(marker) == 1
    source = source.replace(marker, marker + '''
        "independent_review":{"second_decode_equal":true,"all_retained_fields_equal":true,
        "unit_boundary_mutations":6,"expected_busy":review_busy,"expected_movable":review_movable,
        "measurement_valid":false,"original_execution":false},''')
    output = work / "review.rs"
    output.write_text(source)
    binary = work / "review"
    command = ["rustc", "--edition=2021", "-O", str(output), "-L", f"dependency={args.deps.resolve()}", "-o", str(binary)]
    for name in ["c6_core", "serde_json", "sha2"]:
        libraries = list(args.deps.glob(f"lib{name}-*.rlib"))
        if len(libraries) != 1:
            raise ValueError(f"expected one frozen {name} rlib, got {libraries}")
        command.extend(["--extern", f"{name}={libraries[0].resolve()}"])
    subprocess.run(command, check=True)
    result = subprocess.run([str(binary), str(args.manifest.resolve())], check=True, capture_output=True, text=True)
    report = json.loads(result.stdout)
    assert report["role"] == "ReadInputCore"
    assert report["source_core_sha256"] == "ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884"
    assert report["derived_field_queries"]["busy_units"] == report["independent_review"]["expected_busy"]
    assert report["derived_field_queries"]["units_with_moves"] == report["independent_review"]["expected_movable"]
    report["independent_review"]["base_candidate_source_sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
    (work / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report["independent_review"], indent=2))


if __name__ == "__main__":
    main()
