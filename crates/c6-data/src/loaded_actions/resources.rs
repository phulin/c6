//! Concrete input adapter for the already closed Linux placement/Manager
//! captures. Immutable definitions can bind across observer profiles only after
//! actual resource/configuration comparison. Mutable captured state is available
//! solely as an explicitly associated saved-snapshot diagnostic, never Read/live
//! state inferred from matching configuration.
use crate::{
    registry::LinuxDefinitionHashRegistry,
    unit_operation_manager::{
        OperationDefinition, OperationDefinitions, PhaseDefinitions, TurnPhaseDefinition,
        TurnSegmentDefinition,
    },
    RulesRowId,
};
use c6_core::{
    core_player::initialization::{WireBinding, WireRole},
    loaded_state::{ControlInputs, LoadedState, ObservedSnapshotWitness, PlayerControls},
    movement_world::{DefinitionLookup, DefinitionTable},
    skip_turn::placement::{UnitProperties, WarRelations},
    unit_initialization::UnitDefinition,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
const GAME: &str = "a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa";
const HOST: &str = "106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4";
const SNAPSHOT: &str = "ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884";
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Json(serde_json::Error),
    Identity(&'static str),
    Shape(&'static str),
    Definitions(crate::unit_operation_manager::Error),
    World(c6_core::movement_world::MissingInput),
    State(c6_core::loaded_state::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "loaded action resources: {self:?}")
    }
}
impl std::error::Error for Error {}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn parse(bytes: &[u8]) -> Result<Value, Error> {
    serde_json::from_slice(bytes).map_err(Error::Json)
}
fn read(path: &Path, expected: &str) -> Result<Vec<u8>, Error> {
    let bytes = std::fs::read(path).map_err(Error::Io)?;
    if hash(&bytes) != expected {
        return Err(Error::Identity("closed artifact hash"));
    }
    Ok(bytes)
}
fn u32v(v: &Value, key: &'static str) -> Result<u32, Error> {
    v[key]
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .ok_or(Error::Shape(key))
}
fn i32v(v: &Value, key: &'static str) -> Result<i32, Error> {
    v[key]
        .as_i64()
        .and_then(|v| i32::try_from(v).ok())
        .ok_or(Error::Shape(key))
}
fn boolv(v: &Value, key: &'static str) -> Result<bool, Error> {
    v[key].as_bool().ok_or(Error::Shape(key))
}
fn array<'a>(v: &'a Value, key: &'static str) -> Result<&'a [Value], Error> {
    v[key]
        .as_array()
        .map(Vec::as_slice)
        .ok_or(Error::Shape(key))
}
fn one<'a>(rows: &'a [Value], kind: &'static str) -> Result<&'a Value, Error> {
    let mut found = rows.iter().filter(|r| r["kind"] == kind);
    let row = found.next().ok_or(Error::Shape(kind))?;
    if found.next().is_some() {
        return Err(Error::Shape(kind));
    }
    Ok(row)
}
fn rows(bytes: &[u8]) -> Result<Vec<Value>, Error> {
    bytes
        .split(|b| *b == b'\n')
        .filter(|l| !l.is_empty())
        .map(parse)
        .collect()
}
fn identity(runtime: &Value) -> Result<Value, Error> {
    let c = &runtime["configuration"];
    if c["assets"]["libGameCore_XP2.so"] != GAME || c["assets"]["Civ6"] != HOST {
        return Err(Error::Identity("Linux target/host"));
    }
    let mut fields = serde_json::Map::new();
    for key in [
        "assets",
        "base_content_gate_sha256",
        "saved_configuration_sha256",
        "dlc_descriptors",
    ] {
        if c[key].is_null() {
            return Err(Error::Shape(key));
        }
        fields.insert(key.into(), c[key].clone());
    }
    let content = &c["original_filesystem_profile"]["full_content_input_gate_sha256"];
    if content.as_str().is_none() {
        return Err(Error::Shape("actual content gate"));
    }
    let activation = &runtime["native_activation_selection"];
    if !activation.is_object() {
        return Err(Error::Shape("actual activated mods/components"));
    }
    fields.insert("full_content_input_gate_sha256".into(), content.clone());
    fields.insert("native_activation_selection".into(), activation.clone());
    Ok(Value::Object(fields))
}
fn closed_runtime(path: &Path, runtime_hash: &str, config_hash: &str) -> Result<Value, Error> {
    let runtime = parse(&read(&path.join("runtime.json"), runtime_hash)?)?;
    let config = parse(&read(&path.join("configuration.json"), config_hash)?)?;
    if runtime["configuration"] != config
        || runtime["returncode"] != 0
        || runtime["fixture_success"] != true
        || runtime["framework_cleanup_proven"] != true
        || runtime["client_cleanup_proven"] != true
    {
        return Err(Error::Identity("closed successful capture"));
    }
    if hash(&serde_json::to_vec(&config).map_err(Error::Json)?)
        != runtime["configuration_sha256"]
            .as_str()
            .ok_or(Error::Shape("configuration_sha256"))?
    {
        return Err(Error::Identity("canonical capture configuration"));
    }
    if runtime["game_saves"]["save"]["sha256"] != SNAPSHOT
        || runtime["game_saves"]["repeat"]["sha256"] != SNAPSHOT
    {
        return Err(Error::Identity("associated saved snapshot"));
    }
    Ok(runtime)
}

/// Owns only definitions and the separately tagged original diagnostic snapshot
/// facts. No expected handler/placement answers or saved byte masks are inputs.
pub struct Resources {
    identity: Value,
    unit_lookup: LinuxDefinitionHashRegistry,
    units: Vec<Option<UnitProperties>>,
    unit_initialization: Vec<UnitDefinition>,
    operation_lookup: LinuxDefinitionHashRegistry,
    operations: OperationDefinitions,
    phases: PhaseDefinitions,
    configured_phase_hash: u32,
    placement_rows: Vec<Value>,
    manager_rows: Vec<Value>,
    war_types: Vec<i32>,
    war_player_count: i32,
    plot_unit_limit: i32,
}
impl Resources {
    /// These exact closed artifacts are admitted in placement-count-native-1
    /// and unit-operation-manager-reconstruction-admission. No native code runs.
    pub fn from_closed_captures(placement: &Path, manager: &Path) -> Result<Self, Error> {
        let p = closed_runtime(
            placement,
            "e70d0b5f9fdd62d720f6c6929165da7a4f2e290c382b89bf595171b1f935e62a",
            "11e4268904a9a002a723260396eee9fbd36ede8b366f2541495166db6c7c4b54",
        )?;
        let m = closed_runtime(
            manager,
            "5b68cfff7a3c88033694df0ef5f882a43dc780700e5ca4b194e312ca115b3740",
            "6b34093bbf7b7407972758daeba9e7d2f2885472a48cd6ba7097a1d2d2b78cdf",
        )?;
        let resource_identity = identity(&p)?;
        if resource_identity != identity(&m)? {
            return Err(Error::Identity("actual placement/Manager resources differ"));
        }
        let placement_rows = rows(&read(
            &placement.join("facts.jsonl"),
            "b4922e0312310664d7ba59034d32b3118ab3ea78cdad45321926fa73e9cefc23",
        )?)?;
        let manager_rows = rows(&read(
            &manager.join("facts.jsonl"),
            "3fd5b6e64b5f81e06fc06187b9dae53691ddb536a6fc0b03f6f728d759dea585",
        )?)?;
        let defs = one(&placement_rows, "unit_definitions")?;
        if !boolv(defs, "complete")?
            || u32v(defs, "count")? as usize != array(defs, "entries")?.len()
        {
            return Err(Error::Shape("complete unit definitions"));
        }
        let mut unit_hashes = Vec::new();
        let mut units = Vec::new();
        let mut unit_initialization = Vec::new();
        for (index, row) in array(defs, "entries")?.iter().enumerate() {
            if u32v(row, "ordinal")? as usize != index || u32v(row, "index")? as usize != index {
                return Err(Error::Shape("Unit native definition order"));
            }
            unit_hashes.push(u32v(row, "hash")?);
            let domain_hash = u32v(row, "domain_hash")?;
            units.push(Some(UnitProperties {
                domain_hash,
                formation_class_hash: u32v(row, "formation_class_hash")?,
                make_trade_route: boolv(row, "make_trade_route")?,
                spy: boolv(row, "spy")?,
                combat: i32v(row, "combat")?,
                range: i32v(row, "range")?,
            }));
            unit_initialization.push(UnitDefinition { domain_hash });
        }
        let defs = one(&manager_rows, "operation_definitions")?;
        let mut op_defs = Vec::new();
        for (index, row) in array(defs, "entries")?.iter().enumerate() {
            if u32v(row, "ordinal")? as usize != index {
                return Err(Error::Shape("operation native order"));
            }
            op_defs.push(OperationDefinition {
                collection_index: u32v(row, "collection_index")?,
                hash: u32v(row, "hash")?,
            });
        }
        if u32v(defs, "count")? as usize != op_defs.len() {
            return Err(Error::Shape("operation count"));
        }
        let operation_lookup = LinuxDefinitionHashRegistry::from_native_order(
            op_defs.iter().map(|d| d.hash).collect(),
        );
        let operations =
            OperationDefinitions::from_native_order(&op_defs).map_err(Error::Definitions)?;
        let phases_row = one(&manager_rows, "phases")?;
        let mut slots = Vec::new();
        for (index, row) in array(phases_row, "slots")?.iter().enumerate() {
            if u32v(row, "slot")? as usize != index {
                return Err(Error::Shape("phase native order"));
            }
            if row["pointer"] == 0 {
                slots.push(None);
                continue;
            }
            let segment = if row["active_segment"] == 0 {
                None
            } else {
                Some(TurnSegmentDefinition {
                    row_id: RulesRowId(
                        row["segment_rowid"]
                            .as_i64()
                            .ok_or(Error::Shape("segment_rowid"))?,
                    ),
                    flags: u8::try_from(u32v(row, "segment_flags_raw")?)
                        .map_err(|_| Error::Shape("segment_flags_raw"))?,
                })
            };
            slots.push(Some(TurnPhaseDefinition {
                row_id: RulesRowId(row["rowid"].as_i64().ok_or(Error::Shape("rowid"))?),
                phase_type_hash: u32v(row, "type_hash")?,
                phase_order: i32v(row, "phase_order")?,
                active_segment: segment,
            }));
        }
        let configured_phase_hash = u32v(
            one(&manager_rows, "configuration")?,
            "configured_phase_hash",
        )?;
        if slots
            .iter()
            .flatten()
            .any(|p| p.phase_type_hash != configured_phase_hash)
        {
            return Err(Error::Shape("selected configured phases"));
        }
        let rel = one(&placement_rows, "relations")?;
        if !boolv(rel, "complete")? {
            return Err(Error::Shape("complete Game war relations"));
        }
        let war_player_count = i32v(rel, "player_count")?;
        let war_types = array(rel, "war_types")?
            .iter()
            .map(|v| {
                v.as_i64()
                    .and_then(|v| i32::try_from(v).ok())
                    .ok_or(Error::Shape("war_types"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if war_player_count < 0
            || war_types.len()
                != ((war_player_count as usize) * (war_player_count as usize).saturating_sub(1)) / 2
        {
            return Err(Error::Shape("triangular Game war storage"));
        }
        let plot_unit_limit = i32v(one(&placement_rows, "header")?, "plot_unit_limit")?;
        Ok(Self {
            identity: resource_identity,
            unit_lookup: LinuxDefinitionHashRegistry::from_native_order(unit_hashes),
            units,
            unit_initialization,
            operation_lookup,
            operations,
            phases: PhaseDefinitions::from_native_slots(slots),
            configured_phase_hash,
            placement_rows,
            manager_rows,
            war_types,
            war_player_count,
            plot_unit_limit,
        })
    }
    /// Check a target runtime's actual resources and its canonical configuration
    /// against the LoadedState binding. Observer/profile hashes may differ from
    /// the two source captures; resource and activated-content values must match.
    pub fn bind<'a>(
        &'a self,
        state: &LoadedState,
        target_runtime: &[u8],
    ) -> Result<Bound<'a>, Error> {
        self.bind_identity(state.binding(), target_runtime)?;
        Ok(Bound {
            resources: self,
            binding: state.binding().clone(),
        })
    }
    fn bind_identity(&self, binding: &WireBinding, target_runtime: &[u8]) -> Result<(), Error> {
        let runtime = parse(target_runtime)?;
        let configuration = serde_json::to_vec(&runtime["configuration"]).map_err(Error::Json)?;
        if Sha256::digest(&configuration).as_slice() != binding.inputs.configuration_sha256
            || runtime["configuration_sha256"] != hash(&configuration)
        {
            return Err(Error::Identity(
                "target runtime versus loaded configuration",
            ));
        }
        if identity(&runtime)? != self.identity {
            return Err(Error::Identity(
                "actual target resource/content/configuration mismatch",
            ));
        }
        Ok(())
    }
}
/// Only immutable resource methods are available after cross-profile binding.
pub struct Bound<'a> {
    resources: &'a Resources,
    binding: WireBinding,
}
impl Bound<'_> {
    /// Reuse only the immutable resource comparison after a caller has admitted
    /// a separately pinned rules capture. This does not issue mutable facts.
    pub(crate) fn validate_static_runtime(&self, runtime_bytes: &[u8]) -> Result<(), Error> {
        let runtime = parse(runtime_bytes)?;
        if identity(&runtime)? != self.resources.identity {
            return Err(Error::Identity(
                "actual static resource/content/configuration mismatch",
            ));
        }
        Ok(())
    }
    pub fn placement_definitions(
        &self,
    ) -> Result<DefinitionTable<'_, Option<UnitProperties>>, Error> {
        DefinitionTable::new(&self.resources.unit_lookup, &self.resources.units)
            .map_err(Error::World)
    }
    pub fn unit_definitions(&self) -> Result<DefinitionTable<'_, UnitDefinition>, Error> {
        DefinitionTable::new(
            &self.resources.unit_lookup,
            &self.resources.unit_initialization,
        )
        .map_err(Error::World)
    }
    pub fn operations(&self) -> &OperationDefinitions {
        &self.resources.operations
    }
    pub fn operation_lookup(&self) -> &dyn DefinitionLookup {
        &self.resources.operation_lookup
    }
    pub fn phases(&self) -> &PhaseDefinitions {
        &self.resources.phases
    }
    pub fn configured_phase_hash(&self) -> u32 {
        self.resources.configured_phase_hash
    }
    /// Admit mutable captured state exactly once on a pristine matching image.
    /// The observed source remains SavedOutput provenance even when its bytes
    /// were framed by a separately attributed native Read; decode role is kept.
    /// This installs controls, and transfers one owned mutable war sidecar.
    pub fn admit_observed_snapshot(&self, state: &mut LoadedState) -> Result<SnapshotState, Error> {
        if !state.is_pristine() {
            return Err(Error::State(c6_core::loaded_state::Error::NotPristine));
        }
        if state.binding() != &self.binding {
            return Err(Error::Identity("snapshot destination binding differs"));
        }
        self.snapshot_binding(state.binding(), state.source_image())?;
        let facts = SnapshotFacts {
            resources: self.resources,
            binding: state.binding().clone(),
        };
        let controls = facts.controls()?;
        let witness = ObservedSnapshotWitness {
            source_role: WireRole::SavedOutputCore,
            snapshot_sha256: digest_literal(SNAPSHOT),
            snapshot_bytes: 80_657_462,
            capture_runtime_sha256: digest_literal(
                "e70d0b5f9fdd62d720f6c6929165da7a4f2e290c382b89bf595171b1f935e62a",
            ),
            capture_facts_sha256: digest_literal(
                "b4922e0312310664d7ba59034d32b3118ab3ea78cdad45321926fa73e9cefc23",
            ),
            capture_configuration_sha256: digest_literal(
                "9022128238f6209173eec2532b47405721780f1e2a105808ee9084306ebc2f33",
            ),
        };
        // Prepare owned data before consuming core's one-shot admission. A
        // failure never leaves reusable controls or a half-installed sidecar.
        let sidecar = SnapshotState {
            war_types: self.resources.war_types.clone(),
            war_player_count: self.resources.war_player_count,
            plot_unit_limit: self.resources.plot_unit_limit,
            provenance: facts.provenance(),
        };
        state
            .bind_observed_snapshot_controls(witness, controls)
            .map_err(Error::State)?;
        Ok(sidecar)
    }
    fn snapshot_binding(&self, binding: &WireBinding, source: &[u8]) -> Result<(), Error> {
        if binding.core_sha256 != digest_literal(SNAPSHOT)
            || source.len() != 80_657_462
            || hash(source) != SNAPSHOT
        {
            return Err(Error::Identity(
                "mutable facts require full matching associated snapshot bytes",
            ));
        }
        Ok(())
    }
}
fn digest_literal(value: &str) -> [u8; 32] {
    std::array::from_fn(|i| {
        u8::from_str_radix(&value[i * 2..i * 2 + 2], 16).expect("compiled original digest")
    })
}
/// Authoritative mutable sidecar transferred once from its original snapshot.
/// Keep it with the LoadedState on which it was admitted. This is state access,
/// not an API for reissuing the original controls or war vector after mutation.
/// No native Read/live-state acceptance or script backend is established.
pub struct SnapshotState {
    war_types: Vec<i32>,
    war_player_count: i32,
    plot_unit_limit: i32,
    provenance: Value,
}
impl SnapshotState {
    pub fn relations(&self) -> WarRelations<'_> {
        WarRelations {
            player_count: self.war_player_count,
            war_types: &self.war_types,
        }
    }
    /// Direct access to this owned gameplay state for a faithful provider. It
    /// does not implement or admit any diplomatic operation by itself.
    pub fn war_types_mut(&mut self) -> &mut [i32] {
        &mut self.war_types
    }
    pub fn plot_unit_limit(&self) -> i32 {
        self.plot_unit_limit
    }
    pub fn provenance(&self) -> &Value {
        &self.provenance
    }
}
// Only the one-shot installation path can construct original controls.
struct SnapshotFacts<'a> {
    resources: &'a Resources,
    binding: WireBinding,
}
impl SnapshotFacts<'_> {
    fn controls(&self) -> Result<ControlInputs, Error> {
        let mut mapping = [-1; 65];
        let mut present = [false; 65];
        let mut players = Vec::new();
        let rows = self
            .resources
            .placement_rows
            .iter()
            .filter(|r| r["kind"] == "player")
            .collect::<Vec<_>>();
        if rows.len() != 65 {
            return Err(Error::Shape("65 Player queries"));
        }
        for (query, row) in rows.into_iter().enumerate() {
            if u32v(row, "query")? as usize != query {
                return Err(Error::Shape("Player query order"));
            }
            mapping[query] = i32v(row, "mapped_index")?;
            present[query] = boolv(row, "stored_nonnull")?;
            if !boolv(row, "resolved")? {
                continue;
            }
            let owner = i32v(row, "actual_owner")?;
            let active = self
                .resources
                .manager_rows
                .iter()
                .find(|r| r["kind"] == "player" && r["query"] == owner)
                .ok_or(Error::Shape("Manager Player turn state"))?;
            let slot = self
                .resources
                .manager_rows
                .iter()
                .find(|r| r["kind"] == "player_slot" && r["query"] == owner)
                .ok_or(Error::Shape("Manager Player mapping"))?;
            if owner != query as i32
                || i32v(active, "actual_id")? != owner
                || i32v(active, "slot_status")? != i32v(row, "slot_status")?
                || i32v(slot, "mapped_index")? != mapping[query]
            {
                return Err(Error::Shape("snapshot Player cross-capture identity"));
            }
            let turn = i32v(active, "turn_active_raw")?;
            if !(0..=1).contains(&turn) {
                return Err(Error::Shape("turn_active_raw"));
            }
            players.push(PlayerControls {
                owner,
                slot_status: i32v(row, "slot_status")?,
                team: i32v(row, "team")?,
                is_alive: boolv(row, "alive")?,
                turn_active: turn != 0,
            });
        }
        let globals = one(&self.resources.manager_rows, "globals")?;
        let pause = u8::try_from(u32v(globals, "game_pause_raw")?)
            .map_err(|_| Error::Shape("game_pause_raw"))?;
        if pause > 1 {
            return Err(Error::Shape("game_pause_raw"));
        }
        Ok(ControlInputs {
            binding: self.binding.clone(),
            player_mapping: mapping,
            player_slots: present,
            players,
            processing_automation: Some(i32v(globals, "automation_player_raw")?),
            pause_storage: Some(pause),
        })
    }
    fn provenance(&self) -> Value {
        json!({"scope":"associated_saved_snapshot_diagnostic","snapshot_sha256":SNAPSHOT,"decode_role_unchanged":true,"observed_controls_source_role":"SavedOutputCore","native_read_process_controls_claimed":false,"script_callbacks_inspected":false,"native3_live_state_acceptance":false,"mutable_source_facts":{"placement":"b4922e0312310664d7ba59034d32b3118ab3ea78cdad45321926fa73e9cefc23","manager":"3fd5b6e64b5f81e06fc06187b9dae53691ddb536a6fc0b03f6f728d759dea585"},"same_saved_snapshot_independent_capture_processes":true,"unsaved_controls_are_not_native3_admission":true})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c6_core::core_player::initialization::{DatabaseIdentity, InputIdentities};
    fn roots() -> (std::path::PathBuf, std::path::PathBuf) {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../c6-ghidra/evidence");
        (
            root.join("linux-placement-metadata/native-1"),
            root.join("linux-unit-operation-manager-metadata/native-1"),
        )
    }
    // Bind only the resource comparison under test; these dummy other digests
    // never constitute a LoadedState or an admitted gameplay context.
    fn binding(runtime: &Value, role: WireRole, core_sha256: [u8; 32]) -> WireBinding {
        let configuration_sha256 =
            Sha256::digest(serde_json::to_vec(&runtime["configuration"]).unwrap()).into();
        WireBinding {
            role,
            core_sha256,
            occurrence_id: b"1".to_vec(),
            output_id: Some(b"1".to_vec()),
            inputs: InputIdentities {
                configuration_sha256,
                rules_inputs_sha256: [0; 32],
                database: DatabaseIdentity::LiveSnapshotUnavailable,
                dlc_inputs_sha256: [0; 32],
                mods_inputs_sha256: [0; 32],
                content_inputs_sha256: [0; 32],
            },
        }
    }
    #[test]
    #[ignore = "requires admitted private placement/Manager capture artifacts; no original execution"]
    fn concrete_resources_bind_by_actual_content_and_reject_changed_resources() {
        let (placement, manager) = roots();
        let resources = Resources::from_closed_captures(&placement, &manager).unwrap();
        assert_eq!(resources.units.len(), 116);
        assert_eq!(resources.operations.count(), 63);
        assert_eq!(resources.phases.slots().len(), 4);
        let runtime = parse(&std::fs::read(manager.join("runtime.json")).unwrap()).unwrap();
        let b = binding(
            &runtime,
            WireRole::SavedOutputCore,
            digest_literal(SNAPSHOT),
        );
        resources
            .bind_identity(&b, &serde_json::to_vec(&runtime).unwrap())
            .unwrap();
        // Different observer profiles may share actual resources.
        let native3 = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../c6-ghidra/evidence/linux-core-component/native-execute-3/runtime.json");
        let target = parse(&std::fs::read(native3).unwrap()).unwrap();
        assert_ne!(
            runtime["configuration_sha256"],
            target["configuration_sha256"]
        );
        let b = binding(&target, WireRole::ReadInputCore, [0; 32]);
        resources
            .bind_identity(&b, &serde_json::to_vec(&target).unwrap())
            .unwrap();
        let mut changed = target;
        changed["configuration"]["saved_configuration_sha256"] = json!("changed");
        changed["configuration_sha256"] = json!(hash(
            &serde_json::to_vec(&changed["configuration"]).unwrap()
        ));
        let b = binding(&changed, WireRole::ReadInputCore, [0; 32]);
        assert!(matches!(
            resources.bind_identity(&b, &serde_json::to_vec(&changed).unwrap()),
            Err(Error::Identity(
                "actual target resource/content/configuration mismatch"
            ))
        ));
    }
    #[test]
    #[ignore = "requires admitted private placement/Manager capture artifacts; no original execution"]
    fn mutable_snapshot_binding_checks_full_bytes_and_keeps_observed_source_separate() {
        let (placement, manager) = roots();
        let resources = Resources::from_closed_captures(&placement, &manager).unwrap();
        let runtime = parse(&std::fs::read(manager.join("runtime.json")).unwrap()).unwrap();
        let b = binding(&runtime, WireRole::ReadInputCore, digest_literal(SNAPSHOT));
        let bound = Bound {
            resources: &resources,
            binding: b.clone(),
        };
        let source = std::fs::read(placement.join("save.bin")).unwrap();
        bound.snapshot_binding(&b, &source).unwrap();
        assert!(bound
            .snapshot_binding(&b, &source[..source.len() - 1])
            .is_err());
        let mut wrong = b.clone();
        wrong.core_sha256 = [0; 32];
        assert!(bound.snapshot_binding(&wrong, &source).is_err());
        let facts = SnapshotFacts {
            resources: &resources,
            binding: b,
        };
        let controls = facts.controls().unwrap();
        assert_eq!(controls.players.len(), 43);
        assert_eq!(
            controls
                .players
                .iter()
                .filter(|p| p.slot_status == 3)
                .count(),
            1
        );
        assert_eq!(controls.binding.role, WireRole::ReadInputCore); // destination decode role survives
        assert_eq!(controls.processing_automation, Some(-1));
        assert_eq!(controls.pause_storage, Some(0));
        assert_eq!(resources.war_types.len(), 2016);
        assert_eq!(facts.provenance()["native3_live_state_acceptance"], false);
    }
}
