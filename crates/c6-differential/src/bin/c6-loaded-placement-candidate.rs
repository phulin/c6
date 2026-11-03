//! Real loaded-World placement projection. Native expected answers are not inputs.
#[path = "../../../../tools/linux/loaded_actions_review_load.rs"]
mod fixture;
use c6_core::{
    skip_turn::placement::{self, Inputs},
    unit,
};
use c6_data::loaded_actions::resources::Resources;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path};

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect()
}
fn bounded(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut out = Vec::new();
    std::fs::File::open(path)?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut out)?;
    if out.len() > 64 * 1024 * 1024 {
        return Err("runtime report resource bound".into());
    }
    Ok(out)
}
fn row(
    identity: (&str, i32, usize, u32),
    anchor: unit::PlotCoord,
    layer: Option<u32>,
    direction: i32,
    plot: Option<unit::PlotCoord>,
    count: Option<u32>,
) -> Value {
    let (kind, owner, ordinal, id) = identity;
    let mut value = json!({"kind":"count_result","member_kind":kind,"owner":owner,"ordinal":ordinal,"id":id,
        "anchor":[anchor.x,anchor.y],"anchor_valid":placement::coordinate_valid(anchor),"layer":layer.map_or(-1,|v|v as i32),"direction":direction,"plot_present":plot.is_some()});
    if let Some(p) = plot {
        value["plot"] = json!([p.x, p.y]);
        value["bool_argument"] = json!(false);
        value["count"] = json!(count.expect("present plot count"));
    }
    value
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 5 {
        return Err("usage: c6-loaded-placement-candidate CORE_CAPTURE_MANIFEST TARGET_RUNTIME PLACEMENT_CAPTURE MANAGER_CAPTURE".into());
    }
    let mut state = fixture::load_state(Path::new(&args[1]))?;
    let resources = Resources::from_closed_captures(Path::new(&args[3]), Path::new(&args[4]))?;
    let runtime = bounded(Path::new(&args[2]))?;
    let bound = resources.bind(&state, &runtime)?;
    let snapshot = bound.admit_observed_snapshot(&mut state)?;
    let definitions = bound.placement_definitions()?;
    let relations = snapshot.relations();
    let inputs = Inputs {
        world: state.world(),
        definitions: Some(&definitions),
        relations: Some(&relations),
        plot_unit_limit: Some(snapshot.plot_unit_limit()),
    };
    let mut counts = Vec::new();
    let mut bindings = Vec::new();
    let start = std::time::Instant::now();
    for player in state.world().players() {
        for (ordinal, entry) in player.unit_records()?.iter().enumerate() {
            let u = &entry.value;
            let layer = placement::unit_layer(state.world(), u)?;
            let index = definitions
                .lookup
                .find_index(u.e_type_index)
                .map_or(u32::MAX, |v| v as u32);
            let definition_index = definitions.lookup.find_index(index);
            let formation_id = player.formation(u.formation_id)?.map(|v| v.resolved_id);
            bindings.push(json!({"kind":"unit_binding_result","owner":player.owner(),"ordinal":ordinal,"id":u.id,"definition_index":definition_index,"formation_id":formation_id}));
            for (i, plot) in placement::query_plots(state.world().map(), u.k_location)?
                .into_iter()
                .enumerate()
            {
                let count = plot
                    .map(|p| inputs.unit_count(u, placement::units_at(state.world(), layer, p)?))
                    .transpose()?;
                counts.push(row(
                    ("unit", player.owner(), ordinal, u.id),
                    u.k_location,
                    layer,
                    i as i32 - 1,
                    plot,
                    count,
                ));
            }
        }
        for (ordinal, entry) in player.formation_records()?.iter().enumerate() {
            let f = &entry.value;
            let layer = placement::formation_layer(state.world(), f)?;
            for (i, plot) in placement::query_plots(state.world().map(), f.location)?
                .into_iter()
                .enumerate()
            {
                let count = plot
                    .map(|p| {
                        inputs.formation_count(f, placement::units_at(state.world(), layer, p)?)
                    })
                    .transpose()?;
                counts.push(row(
                    ("formation", player.owner(), ordinal, f.id),
                    f.location,
                    layer,
                    i as i32 - 1,
                    plot,
                    count,
                ));
            }
        }
    }
    let seconds = start.elapsed().as_secs_f64();
    if state.revision() != 0 {
        return Err("read-only placement projection advanced World".into());
    }
    let witness = state
        .observed_snapshot()
        .ok_or("missing observed snapshot provenance")?;
    println!(
        "{}",
        serde_json::to_string(
            &json!({"schema":"c6-loaded-placement-results-v1","candidate_sha256":hash(&bounded(&std::env::current_exe()?)?),
        "decode_role":format!("{:?}",state.binding().role),"source_core_sha256":hash(state.source_image()),"snapshot_source_role":format!("{:?}",witness.source_role),
        "snapshot_facts_sha256":hash(&std::fs::read(Path::new(&args[3]).join("facts.jsonl"))?),"count_results":counts,"unit_binding_results":bindings,
        "observed_snapshot":snapshot.provenance(),"world_revision":state.revision(),"query_and_result_construction_seconds":seconds,
        "expected_answers_used_as_inputs":false,"native_execution":false,"full_action_parity":false,"native_read_equivalence":false})
        )?
    );
    Ok(())
}
