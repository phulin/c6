//! Offline finalized-capture consistency and contextual saved-wire import.
//! Never executes original code or establishes native Read/gameplay semantics.
use c6_core::core_player::initialization::{capture::*, WireBinding, WireRole};
use c6_core::{
    loaded_state::{AssemblyLimits, CoreImage, LoadedState},
    map, unit,
};
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
    let mut reads = bindings
        .into_iter()
        .filter(|b| b.role == WireRole::ReadInputCore);
    let binding = reads.next().ok_or("missing Read input binding")?;
    if reads.next().is_some() {
        return Err("multiple Read occurrences require an explicit selection".into());
    }
    drop(named_outputs);
    drop(named_sources);
    drop(outputs);
    drop(sources);
    let bytes = fields.remove("input_core").ok_or("missing owned Core")?;
    drop(fields);
    let source_pointer = bytes.as_ptr();
    let source_bytes = bytes.len();
    let resident_kib = |field: &str| -> Option<u64> {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        status.lines().find_map(|line| {
            line.strip_prefix(field)
                .and_then(|v| v.split_whitespace().next())
                .and_then(|v| v.parse().ok())
        })
    };
    let before_rss = resident_kib("VmRSS:");
    let start = std::time::Instant::now();
    let provider = capture.for_binding(&binding);
    let mut state = LoadedState::decode(
        CoreImage::Unique(bytes),
        shared,
        player_limits,
        binding.clone(),
        &provider,
        AssemblyLimits::default(),
    )?;
    let assembly_seconds = start.elapsed().as_secs_f64();
    let after_rss = resident_kib("VmRSS:");
    let peak_rss = resident_kib("VmHWM:");
    if state.source_image().as_ptr() != source_pointer || state.source_image().len() != source_bytes
    {
        return Err("input ownership was copied or truncated".into());
    }
    if state.world().players().len() != state.retained().players.len() {
        return Err("retained Player count differs".into());
    }
    let mut roster = Vec::new();
    let mut busy = 0usize;
    let mut movable = 0usize;
    for (player, retained) in state
        .world()
        .players()
        .iter()
        .zip(&state.retained().players)
    {
        if player.owner() != retained.header.owner || player.owner() != retained.requested_owner {
            return Err("owner order differs".into());
        }
        let units = player.unit_records()?;
        for u in units {
            busy += usize::from(c6_core::skip_turn::legality::is_busy(&u.value));
            movable += usize::from(c6_core::skip_turn::legality::can_move(&u.value));
        }
        roster.push(json!({"owner":player.owner(),"units":units.len(),"formations":player.formation_records()?.len(),
            "cities":retained.cities.records.entries.len(),"districts":retained.districts.records.entries.len(),
            "saved_is_alive":retained.header.saved_is_alive(),"live_is_alive_available":player.is_alive.is_some(),
            "live_configuration_available":player.slot_status.is_some(),"all_other_components_retained":true}));
    }
    let stats = state.statistics();
    let missing_controls = format!("{:?}", state.processing_automation().unwrap_err());
    let first = state.world().players().iter().find_map(|p| {
        p.unit_records()
            .ok()?
            .first()
            .map(|u| (p.owner(), u.value.id))
    });
    let state_query = if let Some((owner, id)) = first {
        let before = state.unit_mut(owner, id)?.i_operation_timer;
        let value = c6_core::skip_turn::legality::is_busy(state.unit_mut(owner, id)?);
        if state.unit_mut(owner, id)?.i_operation_timer != before {
            return Err("pure busy query mutated timer".into());
        }
        json!({"owner":owner,"unit_id":id,"is_busy":value,"runtime_unavailable":state.unit_runtime(owner,id).is_err(),"mutated":false})
    } else {
        Value::Null
    };
    println!(
        "{}",
        serde_json::to_string(&json!({"schema":"c6-loaded-state-candidate-v3",
        "candidate_sha256":hash(&read(&std::env::current_exe()?,64*1024*1024)?),"manifest_sha256":hash(&manifest_bytes),
        "raw_observation_sha256":hex(&capture.identity.raw_observation_sha256),"role":format!("{:?}",binding.role),
        "source_core_sha256":hex(&binding.core_sha256),"source_bytes":source_bytes,"source_buffer_moved":true,
        "players":stats.players,"units":stats.units,"formations":stats.formations,"consumed":stats.consumed,"items":stats.items_used,
        "retained_barbarian_instances":state.retained().barbarian.instances.len(),"retained_free_city_instances":state.retained().free_cities.instances.len(),
        "retained_sync_random":{"seed":state.retained().random.record.seed,"calls":state.retained().random.record.calls,"resets":state.retained().random.record.resets,"synchronous_byte":state.retained().random.record.synchronous_byte},
        "tail_bytes":state.unparsed_tail().len(),"tail_sha256":hash(state.unparsed_tail()),"rosters":roster,
        "derived_field_queries":{"busy_units":busy,"units_with_moves":movable,"first_subject":state_query,"new_native_comparison":false},
        "missing_live_controls":missing_controls,"assembly_seconds":assembly_seconds,"resident_kib_before":before_rss,"resident_kib_after":after_rss,
        "process_peak_kib_including_admission":peak_rss,"timing_scope":"decode and consuming assembly only; excludes artifact admission and actions",
        "native_acceptance":false,"complete_live_world":false,"native_read_effects_complete":false,"gameplay_parity":false}))?
    );
    Ok(())
}
