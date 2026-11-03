//! Offline comparison of explicit real native movement-cost query facts.
//! This never executes original code and does not resolve a world or move a unit.
use c6_core::movement::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{env, fs};

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
fn run() -> Result<bool, String> {
    let mut args = env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: c6-movement-cost-candidate native.jsonl")?;
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
    if meta.len() > 256 * 1024 * 1024 {
        return Err("native corpus exceeds 256 MiB budget".into());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let text = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
    let records = text
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let header = records.first().ok_or("empty native corpus")?;
    if header["kind"] != "header"
        || header["schema"] != "c6-linux-movement-cost-native-v1"
        || header["game_sha256"]
            != "a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa"
        || header["host_sha256"]
            != "106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4"
        || !boolean(header, "execute")?
        || unsigned(header, "options")? != 0
    {
        return Err("unsupported corpus schema/options or control-only corpus".into());
    }
    let profile = header["profile_sha256"]
        .as_str()
        .filter(|s| {
            s.len() == 64
                && s.bytes()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        })
        .ok_or("missing configuration profile identity")?;
    if unsigned(header, "mxcsr")? & 0xffc0 != 0x1f80 {
        return Err("unsupported native floating-point environment".into());
    }
    let summary = records.last().ok_or("missing summary")?;
    if summary["kind"] != "summary" || integer(summary, "status")? != 0 {
        return Err("native probe incomplete or failed".into());
    }
    let mut compared = 0_u32;
    let mut mismatches = 0_u32;
    for case in records.iter().filter(|v| v["kind"] == "case") {
        let result = check_case(case)?;
        compared += 1;
        if result["equal"] != true {
            mismatches += 1;
        }
        println!("{result}");
    }
    if compared == 0 || compared != unsigned(summary, "cases")? {
        return Err("empty or inconsistent native case count".into());
    }
    println!(
        "{}",
        json!({"kind":"comparison_summary","input_sha256":format!("{:x}",Sha256::digest(&bytes)),"profile_sha256":profile,"compared":compared,"mismatches":mismatches,"known_rng_equal":boolean(summary,"known_rng_equal")?,"complete_affected_state":false,"complete_rng_inventory":false,"full_movement_parity":false})
    );
    Ok(mismatches == 0)
}
fn main() {
    match run() {
        Ok(true) => (),
        Ok(false) => std::process::exit(1),
        Err(error) => {
            eprintln!("movement candidate rejected input: {error}");
            std::process::exit(2);
        }
    }
}
