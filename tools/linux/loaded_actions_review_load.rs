//! Offline finalized-capture consistency and contextual saved-wire import.
//! Never executes original code or establishes native Read/gameplay semantics.
use c6_core::core_player::initialization::{capture::*, WireBinding, WireRole};
use c6_core::{
    loaded_state::{AssemblyLimits, CoreImage, LoadedState},
    map, unit,
};
use serde_json::Value;
use std::{collections::BTreeMap, io::Read, path::Path};

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
pub fn load_state(manifest_path: &Path) -> Result<LoadedState, Box<dyn std::error::Error>> {
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
    let provider = capture.for_binding(&binding);
    let state = LoadedState::decode(
        CoreImage::Unique(bytes),
        shared,
        player_limits,
        binding.clone(),
        &provider,
        AssemblyLimits::default(),
    )?;
    Ok(state)
}
