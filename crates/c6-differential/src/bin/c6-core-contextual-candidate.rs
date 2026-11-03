//! Offline finalized-capture consistency and contextual saved-wire import.
//! Never executes original code or establishes native Read/gameplay semantics.
use c6_core::core_player::initialization::{capture::*, WireBinding, WireRole};
use c6_core::{core_stream::ThroughRandom, map, unit};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read, path::Path};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|v| format!("{v:02x}")).collect()
}
fn hash(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
fn named(values: &BTreeMap<String, Vec<u8>>) -> Vec<NamedArtifact<'_>> {
    values
        .iter()
        .map(|(name, bytes)| NamedArtifact { name, bytes })
        .collect()
}
fn read(path: &Path, maximum: usize) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let size = usize::try_from(file.metadata()?.len())?;
    if size > maximum {
        return Err(format!("artifact exceeds resource bound: {}", path.display()).into());
    }
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(size)?;
    file.take(maximum as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > maximum {
        return Err("artifact grew beyond resource bound".into());
    }
    Ok(bytes)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 2 {
        return Err("usage: c6-core-contextual-candidate ARTIFACT_MANIFEST.json".into());
    }
    let manifest_path = Path::new(&args[1]);
    let manifest_bytes = read(manifest_path, 1024 * 1024)?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    let base = manifest_path.parent().unwrap_or(Path::new("."));
    let limits = CaptureLimits::default();
    let mut total = 0usize;
    let mut load = |value: &Value| -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let path = value.as_str().ok_or("artifact path must be a string")?;
        let remaining = limits
            .max_total_artifact_bytes
            .checked_sub(total)
            .ok_or("aggregate artifact resource bound")?;
        let bytes = read(&base.join(path), limits.max_artifact_bytes.min(remaining))?;
        total = total
            .checked_add(bytes.len())
            .ok_or("aggregate artifact overflow")?;
        Ok(bytes)
    };
    let mut fields = BTreeMap::new();
    for name in [
        "observations_jsonl",
        "runtime_plan_json",
        "static_plan_json",
        "prepared_report_json",
        "exit_json",
        "target_module",
        "target_host",
        "input_save",
        "input_core",
    ] {
        fields.insert(
            name,
            load(manifest.get(name).ok_or("missing artifact field")?)?,
        );
    }
    let mut outputs = BTreeMap::new();
    let mut sources = BTreeMap::new();
    for (field, destination) in [("outputs", &mut outputs), ("source_files", &mut sources)] {
        let object = manifest
            .get(field)
            .and_then(Value::as_object)
            .ok_or("named artifacts must be an object")?;
        if object.len() > limits.max_artifacts {
            return Err("too many named artifacts".into());
        }
        for (name, path) in object {
            destination.insert(name.clone(), load(path)?);
        }
    }
    let named_outputs = named(&outputs);
    let named_sources = named(&sources);
    let capture = admit_capture(
        CaptureArtifacts {
            observations_jsonl: &fields["observations_jsonl"],
            runtime_plan_json: &fields["runtime_plan_json"],
            static_plan_json: &fields["static_plan_json"],
            prepared_report_json: &fields["prepared_report_json"],
            exit_json: &fields["exit_json"],
            target_module: &fields["target_module"],
            target_host: &fields["target_host"],
            input_save: &fields["input_save"],
            input_core: &fields["input_core"],
            outputs: &named_outputs,
            source_files: &named_sources,
        },
        limits,
    )?;
    let mut bindings: Vec<WireBinding> = Vec::new();
    for snapshot in capture.snapshots() {
        let binding = &snapshot
            .captured_role()
            .ok_or("missing captured role")?
            .binding;
        if !bindings.contains(binding) {
            bindings.push(binding.clone());
        }
    }
    let shared = map::Limits {
        max_bytes: 256 * 1024 * 1024,
        max_items: 64 * 1024 * 1024,
        ..map::Limits::default()
    };
    let player_limits = unit::Limits {
        max_bytes: 128 * 1024 * 1024,
        max_items: 64 * 1024 * 1024,
    };
    let mut streams = Vec::new();
    for binding in &bindings {
        let bytes: &[u8] = match binding.role {
            WireRole::ReadInputCore => &fields["input_core"],
            WireRole::SavedOutputCore => {
                let name = std::str::from_utf8(
                    binding.output_id.as_deref().ok_or("missing output name")?,
                )?;
                outputs.get(name).ok_or("missing bound output artifact")?
            }
        };
        let provider = capture.for_binding(binding);
        let parsed =
            ThroughRandom::decode(bytes, shared, player_limits, binding.clone(), &provider)?;
        let players = &parsed.global_ai.players;
        let (encoded, items) = players
            .player_prefix
            .encode_prefix_with_items(player_limits)?;
        if encoded != bytes[players.core.map_end..players.consumed]
            || items + players.core.items_used != players.items_used
        {
            return Err("complete PlayerManager roundtrip/item charge differs".into());
        }
        let (global, global_items) = parsed
            .global_ai
            .global_ai
            .encode_with_items(player_limits)?;
        if global != bytes[players.consumed..parsed.global_ai.consumed]
            || global_items + players.items_used != parsed.global_ai.items_used
        {
            return Err("GlobalAi roundtrip/item charge differs".into());
        }
        if !players.player_prefix.complete_player_manager_wire()
            || !std::ptr::eq(parsed.tail.as_ptr(), bytes[parsed.consumed..].as_ptr())
        {
            return Err("incomplete Manager or nonborrowed tail".into());
        }
        let roster: Vec<_> = players.player_prefix.players().map(|p| json!({
            "requested_owner":p.requested_owner,"record_owner":p.header.owner,
            "saved_is_alive":p.header.saved_is_alive(),"saved_was_ever_alive":p.header.saved_was_ever_alive(),
            "formation_slots":p.formations.entries.iter().map(|e|e.index).collect::<Vec<_>>(),
            "city_slots":p.cities.records.entries.iter().map(|e|e.index).collect::<Vec<_>>(),
            "district_slots":p.districts.records.entries.iter().map(|e|e.index).collect::<Vec<_>>(),
            "units":p.units.records.entries.iter().map(|e|json!({"slot":e.index,"id":e.value.id})).collect::<Vec<_>>()
        })).collect();
        streams.push(json!({"role":format!("{:?}",binding.role),"stream_occurrence":hex(&binding.occurrence_id),"output_id":binding.output_id.as_deref().map(hex),
            "core_sha256":hex(&binding.core_sha256),"core_bytes":bytes.len(),"map_end":players.core.map_end,
            "player_manager_end":players.consumed,"global_ai_end":parsed.global_ai.consumed,"consumed":parsed.consumed,"items":parsed.items_used,
            "post_map_roundtrip_sha256":hash(&encoded),"global_ai_sha256":hash(&global),"tail_sha256":hash(parsed.tail),"tail_bytes":parsed.tail.len(),
            "barbarian_instances":parsed.barbarian.instances.len(),
            "free_city_instances":parsed.free_cities.instances.len(),
            "sync_random_record":{"seed":parsed.random.record.seed,"calls":parsed.random.record.calls,"resets":parsed.random.record.resets,"synchronous_byte":parsed.random.record.synchronous_byte},
            "rosters":roster,"initialization_reconstructed":false,"player_manager_global_ai_wire_roundtrip":true}));
    }
    println!(
        "{}",
        serde_json::to_string(
            &json!({"schema":"c6-contextual-wire-candidate-v2","candidate_sha256":hash(&read(&std::env::current_exe()?,64*1024*1024)?),
        "manifest_sha256":hash(&manifest_bytes),"raw_observation_sha256":hex(&capture.identity.raw_observation_sha256),
        "capture_consistency_checked":true,"native_acceptance":false,"complete_world_import":false,"gameplay_parity":false,"native_read_effects_reproduced":false,"streams":streams})
        )?
    );
    Ok(())
}
