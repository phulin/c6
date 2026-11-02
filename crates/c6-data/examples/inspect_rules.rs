//! Load actual private exports, compare optional independent run, benchmark lookups.
use c6_data::{xp2_source_identity, Rules};
use serde_json::json;
use std::{hint::black_box, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let directory = arguments
        .next()
        .ok_or("usage: inspect_rules DIRECTORY [INDEPENDENT_DIRECTORY]")?;
    let started = Instant::now();
    let rules = Rules::load(directory, &xp2_source_identity())?;
    let load_us = started.elapsed().as_micros();
    let independent = if let Some(other) = arguments.next() {
        let other = Rules::load(other, &xp2_source_identity())?;
        if rules.configuration().fingerprint() != other.configuration().fingerprint()
            || rules.gameplay().fingerprint() != other.gameplay().fingerprint()
        {
            return Err("independent exports differ".into());
        }
        true
    } else {
        false
    };
    let warrior = rules
        .unit("UNIT_WARRIOR")
        .ok_or("missing original UnitType")?;
    let iterations = 1_000_000;
    let started = Instant::now();
    let mut checksum = 0_i64;
    for _ in 0..iterations {
        checksum = checksum.wrapping_add(
            black_box(rules.unit(black_box("UNIT_WARRIOR")))
                .ok_or("missing unit")?
                .combat,
        );
    }
    let lookup_ns = started.elapsed().as_nanos() as f64 / iterations as f64;
    let mut reports = Vec::new();
    for db in [rules.configuration(), rules.gameplay()] {
        reports.push(json!({"fingerprint": db.fingerprint(), "retained_sql_payload_bytes": db.sql_payload_bytes()}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "source": rules.source(), "sqlite_version": rusqlite::version(),
            "databases": reports, "independent_exports_compared": independent,
            "unit_type": warrior.unit_type, "rules_row_id": warrior.rules_row_id.0,
            "unit_fields_from_original_export": {"cost": warrior.cost, "base_moves": warrior.base_moves, "combat": warrior.combat, "ranged_combat": warrior.ranged_combat, "range": warrior.range, "maintenance": warrior.maintenance},
            "load_once_us": load_us, "lookup_iterations": iterations, "lookup_ns_mean": lookup_ns, "lookup_checksum": checksum,
            "limits": "Data load and immutable lookup only; excludes game startup, state, actions, modifiers and turns. Payload bytes exclude allocation overhead, numeric values and indexes."
        }))?
    );
    Ok(())
}
