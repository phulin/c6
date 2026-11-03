//! Prepared-fact Linux movement arithmetic benchmark; no world queries or actions.
use c6_core::movement::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{env, fs, hint::black_box, time::Instant};

fn integer(v: &Value, key: &str) -> Result<i32, String> {
    v.get(key)
        .and_then(Value::as_i64)
        .and_then(|v| i32::try_from(v).ok())
        .ok_or_else(|| format!("missing or invalid i32 {key}"))
}
fn unsigned(v: &Value, key: &str) -> Result<u32, String> {
    v.get(key)
        .and_then(Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| format!("missing or invalid u32 {key}"))
}
fn boolean(v: &Value, key: &str) -> Result<bool, String> {
    v.get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("missing or invalid bool {key}"))
}
fn optional_integer(v: &Value, key: &str) -> Result<Option<i32>, String> {
    if v.get(key) == Some(&Value::Null) {
        Ok(None)
    } else {
        integer(v, key).map(Some)
    }
}
fn plot(v: &Value) -> Result<PlotFacts, String> {
    let route = match v.get("route") {
        Some(Value::Null) => None,
        Some(r) if r.is_object() => Some(RouteFacts {
            pillaged: boolean(r, "pillaged")?,
            bridged: boolean(r, "bridged")?,
            movement_cost_float_bits: match r.get("movement_cost_float_bits") {
                Some(Value::Null) => None,
                _ => Some(unsigned(r, "movement_cost_float_bits")?),
            },
        }),
        _ => return Err("missing or invalid route".into()),
    };
    Ok(PlotFacts {
        is_city: boolean(v, "is_city")?,
        is_water: boolean(v, "is_water")?,
        has_city_attributes: boolean(v, "has_city_attributes")?,
        city_domain_valid: boolean(v, "city_domain_valid")?,
        terrain_cost: optional_integer(v, "terrain_cost")?,
        feature_change: optional_integer(v, "feature_change")?,
        improvement_change: optional_integer(v, "improvement_change")?,
        route,
        district_complete: boolean(v, "district_complete")?,
        district_free_embark: match v.get("district_free_embark") {
            Some(Value::Null) => None,
            _ => Some(boolean(v, "district_free_embark")?),
        },
    })
}
fn state(v: CostAccumulator) -> Value {
    json!([
        v.hard_minimum.0,
        v.soft_maximum.0,
        v.regular.0,
        v.to_plot_exit.0
    ])
}
fn check_case(c: &Value) -> Result<Value, String> {
    let b = &c["budget"];
    let budget = MoveBudget {
        base_moves: integer(b, "base_moves")?,
        max_moves: MovementPoints(integer(b, "max_moves")?),
        remaining: MovementPoints(integer(b, "remaining")?),
        exit_moves: MovementPoints(integer(b, "exit_moves")?),
    };
    let f = &c["formation"];
    let formation = FormationFacts {
        ignore_terrain: boolean(f, "ignore_terrain")?,
        ignore_feature: boolean(f, "ignore_feature")?,
        ignore_rivers: boolean(f, "ignore_rivers")?,
        ignore_shores: boolean(f, "ignore_shores")?,
        can_embark: boolean(f, "can_embark")?,
    };
    let p = &c["parameters"];
    let parameters = RuleParameters {
        city_movement_cost: integer(p, "city_movement_cost")?,
        movement_river_cost: integer(p, "movement_river_cost")?,
        movement_portal_cost: integer(p, "movement_portal_cost")?,
        movement_embark_cost: integer(p, "movement_embark_cost")?,
    };
    let e = &c["edge"];
    let edge = EdgeFacts {
        river_crossing: boolean(e, "river_crossing")?,
        adjacent: boolean(e, "adjacent")?,
        source_portal_has_exit_at_destination: boolean(e, "source_portal_has_exit_at_destination")?,
    };
    let from = plot(&c["from"])?;
    let to = plot(&c["to"])?;
    let mut actual = CostAccumulator::default();
    let initial_equal = state(actual) == c["initial"];
    apply_rules(&mut actual, budget, formation, from, to, edge, parameters);
    let no_zoc = actual.final_cost().0;
    let phases = c["phases"]
        .as_array()
        .filter(|v| v.len() == 9)
        .ok_or("missing nine original phase snapshots")?;
    let accumulator_equal = state(actual) == phases[8];
    let no_zoc_equal =
        no_zoc == integer(c, "no_zoc_final")? && no_zoc == integer(c, "phased_final")?;
    let mut aware = CostAccumulator::default();
    apply_zoc_minimum(
        &mut aware,
        budget,
        ZocValue::Query,
        0,
        boolean(c, "is_slowed_by_zoc")?,
    );
    apply_rules(&mut aware, budget, formation, from, to, edge, parameters);
    let full = aware.final_cost().0;
    let full_equal = full == integer(c, "full_final")?
        && full == integer(c, "values_overload_final")?
        && state(aware) == c["aware_values"];
    let members = c["members"]
        .as_array()
        .ok_or("missing member observations")?;
    let mut aggregates = Vec::new();
    for key in ["terrain", "feature", "rivers", "shores"] {
        let observations = members
            .iter()
            .map(|m| {
                if boolean(m, "found")? {
                    Ok(Some(MemberIgnore {
                        ignores: boolean(m, key)?,
                        escort_mobility: boolean(m, "escort")?,
                    }))
                } else {
                    Ok(None)
                }
            })
            .collect::<Result<Vec<_>, String>>()?;
        aggregates.push(formation_ignores(observations));
    }
    let aggregate_equal = aggregates
        == [
            formation.ignore_terrain,
            formation.ignore_feature,
            formation.ignore_rivers,
            formation.ignore_shores,
        ];
    let equal = initial_equal
        && accumulator_equal
        && no_zoc_equal
        && full_equal
        && aggregate_equal
        && boolean(c, "native_paths_agree")?;
    Ok(
        json!({"owner":unsigned(c,"owner")?,"formation_id":unsigned(c,"formation_id")?,"unit_id":unsigned(c,"unit_id")?,"direction":unsigned(c,"direction")?,"equal":equal,"initial_equal":initial_equal,"final_accumulator_equal":accumulator_equal,"no_zoc_equal":no_zoc_equal,"full_equal":full_equal,"formation_aggregate_equal":aggregate_equal,"rust_full":full,"rust_no_zoc":no_zoc}),
    )
}

#[derive(Clone, Copy)]
struct Prepared {
    budget: MoveBudget,
    formation: FormationFacts,
    from: PlotFacts,
    to: PlotFacts,
    edge: EdgeFacts,
    parameters: RuleParameters,
    slowed: bool,
}
fn prepare(c: &Value) -> Result<Prepared, String> {
    let b = &c["budget"];
    let budget = MoveBudget {
        base_moves: integer(b, "base_moves")?,
        max_moves: MovementPoints(integer(b, "max_moves")?),
        remaining: MovementPoints(integer(b, "remaining")?),
        exit_moves: MovementPoints(integer(b, "exit_moves")?),
    };
    let f = &c["formation"];
    let formation = FormationFacts {
        ignore_terrain: boolean(f, "ignore_terrain")?,
        ignore_feature: boolean(f, "ignore_feature")?,
        ignore_rivers: boolean(f, "ignore_rivers")?,
        ignore_shores: boolean(f, "ignore_shores")?,
        can_embark: boolean(f, "can_embark")?,
    };
    let p = &c["parameters"];
    let parameters = RuleParameters {
        city_movement_cost: integer(p, "city_movement_cost")?,
        movement_river_cost: integer(p, "movement_river_cost")?,
        movement_portal_cost: integer(p, "movement_portal_cost")?,
        movement_embark_cost: integer(p, "movement_embark_cost")?,
    };
    let e = &c["edge"];
    let edge = EdgeFacts {
        river_crossing: boolean(e, "river_crossing")?,
        adjacent: boolean(e, "adjacent")?,
        source_portal_has_exit_at_destination: boolean(e, "source_portal_has_exit_at_destination")?,
    };
    let from = plot(&c["from"])?;
    let to = plot(&c["to"])?;
    Ok(Prepared {
        budget,
        formation,
        from,
        to,
        edge,
        parameters,
        slowed: boolean(c, "is_slowed_by_zoc")?,
    })
}
fn evaluate(c: Prepared, aware: bool) -> (CostAccumulator, MovementPoints) {
    let mut values = CostAccumulator::default();
    if aware {
        apply_zoc_minimum(&mut values, c.budget, ZocValue::Query, 0, c.slowed);
    }
    apply_rules(
        &mut values,
        c.budget,
        c.formation,
        c.from,
        c.to,
        c.edge,
        c.parameters,
    );
    (values, values.final_cost())
}
fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 5 {
        return Err(
            "usage: c6-movement-cost-bench corpus.jsonl rounds samples warmup_rounds".into(),
        );
    }
    let rounds: u64 = args[2].parse().map_err(|_| "invalid rounds")?;
    let samples: u32 = args[3].parse().map_err(|_| "invalid samples")?;
    let warmup: u64 = args[4].parse().map_err(|_| "invalid warmup")?;
    if !(1..=1000000).contains(&rounds) || !(1..=100).contains(&samples) || warmup > 1000000 {
        return Err("iteration limit exceeded".into());
    }
    #[cfg(target_arch = "x86_64")]
    {
        let mut mxcsr = 0_u32;
        // Read the floating control environment; exception status bits are ignored.
        unsafe {
            std::arch::asm!("stmxcsr [{address}]", address = in(reg) &mut mxcsr,
            options(nostack, preserves_flags));
        }
        if mxcsr & 0xffc0 != 0x1f80 {
            return Err("unsupported benchmark floating-point environment".into());
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    return Err("benchmark requires x86_64".into());
    let preparation = Instant::now();
    if fs::metadata(&args[1]).map_err(|e| e.to_string())?.len() > 256 * 1024 * 1024 {
        return Err("corpus exceeds 256 MiB".into());
    }
    let bytes = fs::read(&args[1]).map_err(|e| e.to_string())?;
    let records: Vec<Value> = std::str::from_utf8(&bytes)
        .map_err(|e| e.to_string())?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    let header = records.first().ok_or("empty corpus")?;
    if header["schema"] != "c6-linux-movement-cost-native-v1"
        || header["execute"] != true
        || header["options"] != 0
        || header["game_sha256"]
            != "a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa"
        || header["host_sha256"]
            != "106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4"
        || unsigned(header, "mxcsr")? & 0xffc0 != 0x1f80
    {
        return Err("unsupported native header".into());
    }
    let summary = records.last().ok_or("missing summary")?;
    if summary["kind"] != "summary" || summary["status"] != 0 || summary["known_rng_equal"] != true
    {
        return Err("incomplete or failed native corpus".into());
    }
    let cases: Vec<_> = records.iter().filter(|c| c["kind"] == "case").collect();
    if cases.is_empty() || cases.len() != unsigned(summary, "cases")? as usize {
        return Err("case count differs".into());
    }
    let prepared = cases
        .iter()
        .map(|c| prepare(c))
        .collect::<Result<Vec<_>, _>>()?;
    let preparation_ns = preparation.elapsed().as_nanos();
    let validation = Instant::now();
    for (index, (c, p)) in cases.iter().zip(&prepared).enumerate() {
        if check_case(c)?["equal"] != true {
            return Err(format!("native output mismatch before timing: {index}"));
        }
        for (aware, expected_values, expected_cost) in [
            (false, &c["phases"][8], &c["no_zoc_final"]),
            (true, &c["aware_values"], &c["full_final"]),
        ] {
            let (values, cost) = evaluate(*p, aware);
            if state(values) != *expected_values || json!(cost.0) != *expected_cost {
                return Err(format!("timed evaluator mismatch before timing: {index}"));
            }
        }
    }
    let validation_ns = validation.elapsed().as_nanos();
    let mut outputs = Vec::new();
    for aware in [false, true] {
        for _ in 0..warmup {
            for c in &prepared {
                black_box(evaluate(black_box(*c), aware));
            }
        }
        let mut measured = Vec::new();
        let mut checksum = 0_u64;
        for _ in 0..samples {
            let began = Instant::now();
            for _ in 0..rounds {
                for c in &prepared {
                    let result = black_box(evaluate(black_box(*c), aware));
                    checksum = checksum.wrapping_add(result.1 .0 as u32 as u64);
                }
            }
            let elapsed = began.elapsed().as_nanos();
            measured.push(json!({"elapsed_ns": elapsed, "ns_per_edge": elapsed as f64 / (rounds * prepared.len() as u64) as f64}));
        }
        outputs.push(json!({"mode": if aware { "zoc_aware" } else { "no_zoc" },
            "edges_per_sample": rounds * prepared.len() as u64, "samples": measured,
            "checksum": black_box(checksum)}));
    }
    println!(
        "{}",
        json!({"schema":"c6-linux-movement-cost-bench-run-v1",
        "input_sha256":format!("{:x}", Sha256::digest(&bytes)), "cases":prepared.len(),
        "validated_cases":prepared.len(), "profile_sha256":header["profile_sha256"],
        "native_scan_truncated":summary["truncated"], "rounds":rounds,"warmup_rounds":warmup,
        "preparation_ns":preparation_ns,"validation_ns":validation_ns,"measurements":outputs,
        "scope":"prepared per-edge arithmetic only", "full_movement_parity":false,
        "complete_gameplay_state":false,"complete_turns":false})
    );
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
