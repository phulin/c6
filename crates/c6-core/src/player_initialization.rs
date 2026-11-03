//! Attributed initialization inputs for conditional saved-wire decoding.
//!
//! Raw observer JSONL is not a snapshot by declaration. The capture child checks
//! bounded finalized artifacts/events before constructing role-local snapshots.
//! This validates supplied-capture consistency, not authenticity/native gameplay.
//! Legacy Candidate preflight remains shape/binding only.
use crate::player_ai_military;
#[path = "player_initialization_capture.rs"]
pub mod capture;

pub type Digest = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DatabaseIdentity {
    Snapshot(Digest),
    /// Actual gameplay DB is live/in-memory; an input gate is not its byte hash.
    LiveSnapshotUnavailable,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputIdentities {
    pub configuration_sha256: Digest,
    pub rules_inputs_sha256: Digest,
    pub database: DatabaseIdentity,
    pub dlc_inputs_sha256: Digest,
    pub mods_inputs_sha256: Digest,
    pub content_inputs_sha256: Digest,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireRole {
    ReadInputCore,
    SavedOutputCore,
    #[cfg(test)]
    SourceOnlyReadCase,
    #[cfg(test)]
    SourceOnlyWriteCase,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireBinding {
    pub role: WireRole,
    /// Raw Core identity; never the .Civ6Save container identity.
    pub core_sha256: Digest,
    /// Actual STREAM occurrence, canonical decimal bytes from raw
    /// `stream_occurrence`; per-Military occurrences remain separate metadata.
    /// Legacy SourceOnly test bindings are explicitly tagged test values.
    pub occurrence_id: Vec<u8>,
    pub output_id: Option<Vec<u8>>,
    pub inputs: InputIdentities,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlayerIdentity {
    pub manager_index: u32,
    pub requested_owner: i32,
    pub record_owner: i32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    ReadEntry,
    InitializeEntry,
    InitializeReturn,
    ReadInitialized,
    ReadReturn,
    WriteEntry,
    WriteGuard,
    DefaultQuery,
    DefaultNotReached,
    DefinitionCount,
    WriteAllowed,
    WritePerWar,
    WriteCurrent,
    SavedOutputBinding,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Witness {
    pub stage: Stage,
    pub observation_sha256: Digest,
    pub event_ordinal: u64,
    pub frame_id: u64,
    pub military_frame_id: u64,
    pub parent_frame_id: Option<u64>,
    pub thread_id: u64,
    /// Actual outer saved-stream pointer, inherited by nested helper witnesses.
    pub stream_pointer: u64,
    pub occurrence_id: Vec<u8>,
    pub lifecycle_id: Vec<u8>,
    pub owner_lifetime_id: Vec<u8>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VectorState {
    pub data_pointer: u64,
    pub capacity_u32: u32,
    pub size_u32: u32,
    pub padding_u32: u32,
    pub tail_padding_u32: u32,
    pub values_i32: Vec<i32>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Vectors {
    pub allowed: VectorState,
    pub per_war: VectorState,
    pub current: VectorState,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerObservation {
    pub player_pointer: u64,
    pub player_id_i32: i32,
    pub was_ever_alive_raw: u8,
    pub handicap_i32: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateObservation {
    pub witness: Witness,
    pub military_pointer: u64,
    /// Read-entry prior vectors need not dereference a not-yet-installed owner.
    pub player: Option<PlayerObservation>,
    pub vectors: Vectors,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefinitionEntry {
    pub pointer: u64,
    pub database_row_i32: i32,
    pub collection_index_u32: u32,
    pub operation_type_hash_u32: u32,
    pub default_value_i32: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefinitionCollection {
    pub collection_pointer: u64,
    pub begin_pointer: u64,
    pub end_pointer: u64,
    pub capacity_pointer: u64,
    pub ordered: Vec<DefinitionEntry>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DefaultObservation {
    Observed {
        witness: Witness,
        value_i32: i32,
    },
    /// Requires the actual branch witness; no fabricated handler query/default.
    NotReached {
        witness: Witness,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountObservation {
    pub witness: Witness,
    pub count_u32: u32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteGuard {
    pub witness: Witness,
    pub allowed_size_u32: u32,
    pub branch_taken: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArrayWrite {
    pub witness: Witness,
    pub data_pointer: u64,
    pub definitions: DefinitionCollection,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavedOutput {
    pub witness: Witness,
    pub output_id: Vec<u8>,
    pub output_core_sha256: Digest,
    pub military_record_start: u64,
    pub military_record_end: u64,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotCandidate {
    pub player: PlayerIdentity,
    pub gamecore_sha256: Digest,
    pub host_sha256: Digest,
    pub producer_source_commit: [u8; 20],
    pub producer_binary_sha256: Digest,
    pub methods_and_layout_audit_sha256: Digest,
    pub raw_observation_sha256: Digest,
    /// Nullable while attribution is pending; it is never an admission boolean.
    pub attribution_audit_sha256: Option<Digest>,
    pub input_save_sha256: Digest,
    pub input_core_sha256: Digest,
    pub inputs: InputIdentities,
    pub read_entry_prior: StateObservation,
    pub initialize_entry_prior: StateObservation,
    pub initialize_return: StateObservation,
    pub read_initialized: StateObservation,
    pub read_return: StateObservation,
    pub write_entry: StateObservation,
    pub default_observation: DefaultObservation,
    pub definition_count: Option<CountObservation>,
    pub write_guard: WriteGuard,
    /// Empty only for an actually skipped write branch; otherwise allowed/per-war/current.
    pub array_writes: Vec<ArrayWrite>,
    pub saved_output: SavedOutput,
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_values: usize,
    pub max_identity_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_values: 1_000_000,
            max_identity_bytes: 4096,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    ResourceLimit,
    TargetIdentity,
    MissingIdentity,
    Stage,
    WitnessBinding,
    VectorSize,
    WriteGuard,
    Definitions,
    InputBinding,
    OwnerBinding,
    CursorBinding,
    SourceOnly,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Player initialization input: {self:?}")
    }
}
impl std::error::Error for Error {}

fn present(d: &Digest) -> bool {
    *d != [0; 32]
}
fn token(v: &[u8], limits: Limits) -> Result<(), Error> {
    if v.is_empty() {
        return Err(Error::MissingIdentity);
    }
    if v.len() > limits.max_identity_bytes {
        return Err(Error::ResourceLimit);
    }
    Ok(())
}
fn inputs(v: &InputIdentities) -> Result<(), Error> {
    if ![
        v.configuration_sha256,
        v.rules_inputs_sha256,
        v.dlc_inputs_sha256,
        v.mods_inputs_sha256,
        v.content_inputs_sha256,
    ]
    .iter()
    .all(present)
    {
        return Err(Error::MissingIdentity);
    }
    if matches!(v.database,DatabaseIdentity::Snapshot(d) if !present(&d)) {
        return Err(Error::MissingIdentity);
    }
    Ok(())
}
impl SnapshotCandidate {
    /// Shape/provenance-reference checks only. Passing does NOT admit a native
    /// snapshot or turn a raw observer event into reconstructed initialization.
    pub fn preflight(&self, limits: Limits) -> Result<(), Error> {
        const CORE: Digest = CORE_IDENTITY;
        const HOST: Digest = HOST_IDENTITY;
        if self.gamecore_sha256 != CORE || self.host_sha256 != HOST {
            return Err(Error::TargetIdentity);
        }
        if self.producer_source_commit == [0; 20]
            || ![
                self.producer_binary_sha256,
                self.methods_and_layout_audit_sha256,
                self.raw_observation_sha256,
                self.input_save_sha256,
                self.input_core_sha256,
                self.saved_output.output_core_sha256,
            ]
            .iter()
            .all(present)
            || self.attribution_audit_sha256.is_some_and(|d| !present(&d))
        {
            return Err(Error::MissingIdentity);
        }
        inputs(&self.inputs)?;
        let read = &self.read_initialized.witness;
        let write = &self.write_entry.witness;
        let mut values = 0usize;
        let stages = [
            (&self.read_entry_prior, Stage::ReadEntry),
            (&self.initialize_entry_prior, Stage::InitializeEntry),
            (&self.initialize_return, Stage::InitializeReturn),
            (&self.read_initialized, Stage::ReadInitialized),
            (&self.read_return, Stage::ReadReturn),
            (&self.write_entry, Stage::WriteEntry),
        ];
        for (state, stage) in stages {
            let group = if stage == Stage::WriteEntry {
                write
            } else {
                read
            };
            self.witness(&state.witness, stage, group, limits)?;
            if state.military_pointer != self.read_initialized.military_pointer {
                return Err(Error::WitnessBinding);
            }
            if matches!(
                stage,
                Stage::ReadInitialized | Stage::ReadReturn | Stage::WriteEntry
            ) && state
                .player
                .as_ref()
                .is_none_or(|p| p.player_id_i32 != self.player.record_owner)
            {
                return Err(Error::OwnerBinding);
            }
            for v in [
                &state.vectors.allowed,
                &state.vectors.per_war,
                &state.vectors.current,
            ] {
                if v.values_i32.len() != v.size_u32 as usize {
                    return Err(Error::VectorSize);
                }
                values = values
                    .checked_add(v.values_i32.len())
                    .ok_or(Error::ResourceLimit)?;
            }
        }
        match &self.default_observation {
            DefaultObservation::Observed { witness, .. } => {
                self.witness(witness, Stage::DefaultQuery, read, limits)?
            }
            DefaultObservation::NotReached { witness } => {
                self.witness(witness, Stage::DefaultNotReached, read, limits)?
            }
        }
        if let Some(v) = &self.definition_count {
            self.witness(&v.witness, Stage::DefinitionCount, read, limits)?;
        }
        self.witness(&self.write_guard.witness, Stage::WriteGuard, write, limits)?;
        let allowed = self.write_entry.vectors.allowed.size_u32;
        if self.write_guard.allowed_size_u32 != allowed
            || self.write_guard.branch_taken != (allowed != 0)
        {
            return Err(Error::WriteGuard);
        }
        if self.array_writes.len() != if self.write_guard.branch_taken { 3 } else { 0 } {
            return Err(Error::Definitions);
        }
        for (a, (stage, v)) in self.array_writes.iter().zip([
            (Stage::WriteAllowed, &self.write_entry.vectors.allowed),
            (Stage::WritePerWar, &self.write_entry.vectors.per_war),
            (Stage::WriteCurrent, &self.write_entry.vectors.current),
        ]) {
            self.witness(&a.witness, stage, write, limits)?;
            if a.data_pointer != v.data_pointer || a.definitions.ordered.len() > v.values_i32.len()
            {
                return Err(Error::Definitions);
            }
            values = values
                .checked_add(a.definitions.ordered.len())
                .ok_or(Error::ResourceLimit)?;
        }
        if values > limits.max_values {
            return Err(Error::ResourceLimit);
        }
        self.witness(
            &self.saved_output.witness,
            Stage::SavedOutputBinding,
            write,
            limits,
        )?;
        token(&self.saved_output.output_id, limits)?;
        if self.saved_output.military_record_start >= self.saved_output.military_record_end {
            return Err(Error::InputBinding);
        }
        Ok(())
    }
    fn witness(
        &self,
        w: &Witness,
        stage: Stage,
        group: &Witness,
        limits: Limits,
    ) -> Result<(), Error> {
        if w.stage != stage {
            return Err(Error::Stage);
        }
        if w.observation_sha256 != self.raw_observation_sha256
            || w.thread_id != group.thread_id
            || w.military_frame_id != group.military_frame_id
            || w.stream_pointer != group.stream_pointer
            || w.occurrence_id != group.occurrence_id
            || w.lifecycle_id != group.lifecycle_id
            || w.owner_lifetime_id != self.read_initialized.witness.owner_lifetime_id
        {
            return Err(Error::WitnessBinding);
        }
        for t in [&w.occurrence_id, &w.lifecycle_id, &w.owner_lifetime_id] {
            token(t, limits)?;
        }
        if w.frame_id == 0 || w.military_frame_id == 0 || w.thread_id == 0 || w.stream_pointer == 0
        {
            return Err(Error::WitnessBinding);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Origin {
    /// Source-validated consistency of supplied finalized capture artifacts.
    /// Execution authenticity and native/gameplay acceptance remain separate.
    AttributedObservation,
    #[cfg(test)]
    SourceOnly,
}
#[derive(Debug, Eq, PartialEq)]
pub struct InitializationSnapshot {
    candidate: Option<SnapshotCandidate>,
    role: Option<capture::RoleSnapshot>,
    origin: Origin,
}
impl InitializationSnapshot {
    pub fn candidate(&self) -> Option<&SnapshotCandidate> {
        self.candidate.as_ref()
    }
    pub fn captured_role(&self) -> Option<&capture::RoleSnapshot> {
        self.role.as_ref()
    }
    fn from_role(role: capture::RoleSnapshot, origin: Origin) -> Self {
        Self {
            candidate: None,
            role: Some(role),
            origin,
        }
    }
    pub(super) fn check_cursor(&self, cursor: usize) -> Result<(), Error> {
        if self
            .role
            .as_ref()
            .is_some_and(|role| role.core_range[0] != cursor)
        {
            return Err(Error::CursorBinding);
        }
        Ok(())
    }
    pub fn origin(&self) -> Origin {
        self.origin
    }
    pub const fn initialization_reconstructed(&self) -> bool {
        false
    }
    /// Derive the unsaved wire branch from an attributed state, not a count in
    /// the stream. The binding is supplied by an externally attributed Core
    /// reader. Capture ingestion performs import-time checks once; this lookup
    /// only reuses its immutable role/stream/owner facts.
    pub(super) fn context_for(
        &self,
        player: PlayerIdentity,
        binding: &WireBinding,
    ) -> Result<player_ai_military::Context, Error> {
        #[cfg(test)]
        if self.origin == Origin::SourceOnly
            && !matches!(
                binding.role,
                WireRole::SourceOnlyReadCase | WireRole::SourceOnlyWriteCase
            )
        {
            return Err(Error::SourceOnly);
        }
        if let Some(role) = &self.role {
            return role.context_for(player, binding);
        }
        let candidate = self.candidate.as_ref().ok_or(Error::InputBinding)?;
        if player != candidate.player {
            return Err(Error::OwnerBinding);
        }
        if binding.inputs != candidate.inputs {
            return Err(Error::InputBinding);
        }
        #[cfg(test)]
        if self.origin == Origin::SourceOnly
            && !matches!(
                binding.role,
                WireRole::SourceOnlyReadCase | WireRole::SourceOnlyWriteCase
            )
        {
            return Err(Error::SourceOnly);
        }
        let output = match binding.role {
            WireRole::ReadInputCore => false,
            WireRole::SavedOutputCore => true,
            #[cfg(test)]
            WireRole::SourceOnlyReadCase => false,
            #[cfg(test)]
            WireRole::SourceOnlyWriteCase => true,
        };
        let expected = if output {
            candidate.saved_output.output_core_sha256
        } else {
            candidate.input_core_sha256
        };
        let witness = if output {
            &candidate.write_entry.witness
        } else {
            &candidate.read_initialized.witness
        };
        if binding.core_sha256 != expected
            || binding.occurrence_id != witness.occurrence_id
            || if output {
                binding.output_id.as_ref() != Some(&candidate.saved_output.output_id)
            } else {
                binding.output_id.is_some()
            }
        {
            return Err(Error::InputBinding);
        }
        let size = if output {
            candidate.write_guard.allowed_size_u32
        } else {
            candidate.read_initialized.vectors.allowed.size_u32
        };
        Ok(player_ai_military::Context {
            operation_type_arrays_present: size != 0,
        })
    }
    pub(super) fn check_saved_player(
        &self,
        player_id: i32,
        binding: &WireBinding,
    ) -> Result<(), Error> {
        if let Some(role) = &self.role {
            if role.binding != *binding {
                return Err(Error::InputBinding);
            }
            return if player_id == role.player_id() {
                Ok(())
            } else {
                Err(Error::OwnerBinding)
            };
        }
        let candidate = self.candidate.as_ref().ok_or(Error::InputBinding)?;
        let output = match binding.role {
            WireRole::ReadInputCore => false,
            WireRole::SavedOutputCore => true,
            #[cfg(test)]
            WireRole::SourceOnlyReadCase => false,
            #[cfg(test)]
            WireRole::SourceOnlyWriteCase => true,
        };
        let state = if output {
            &candidate.write_entry
        } else {
            &candidate.read_initialized
        };
        if state
            .player
            .as_ref()
            .is_none_or(|player| player.player_id_i32 != player_id)
        {
            return Err(Error::OwnerBinding);
        }
        Ok(())
    }
    /// Test-only construction is visibly tagged and cannot bind to native roles.
    /// This legacy constructor cannot promote a source test to native roles.
    #[cfg(test)]
    pub(super) fn source_only(candidate: SnapshotCandidate) -> Result<Self, Error> {
        candidate.preflight(Limits::default())?;
        Ok(Self {
            candidate: Some(candidate),
            role: None,
            origin: Origin::SourceOnly,
        })
    }
}

pub const CORE_IDENTITY: Digest = [
    160, 182, 97, 31, 69, 237, 40, 250, 123, 246, 225, 141, 96, 140, 175, 221, 189, 213, 177, 194,
    12, 253, 136, 12, 181, 9, 55, 175, 115, 28, 31, 170,
];
pub const HOST_IDENTITY: Digest = [
    16, 109, 73, 134, 86, 124, 246, 10, 173, 120, 125, 250, 201, 144, 208, 137, 228, 23, 117, 224,
    75, 95, 64, 224, 115, 245, 154, 58, 143, 254, 116, 244,
];

#[cfg(test)]
pub(super) fn source_case(
    player: PlayerIdentity,
    read_present: bool,
    write_present: bool,
) -> (InitializationSnapshot, WireBinding, WireBinding) {
    // Every reference and pointer below is synthetic, never an observed event.
    let input = InputIdentities {
        configuration_sha256: [6; 32],
        rules_inputs_sha256: [7; 32],
        database: DatabaseIdentity::LiveSnapshotUnavailable,
        dlc_inputs_sha256: [8; 32],
        mods_inputs_sha256: [9; 32],
        content_inputs_sha256: [10; 32],
    };
    let owner_lifetime = format!("source-owner-{}", player.manager_index).into_bytes();
    let witness = |stage, write| Witness {
        stage,
        observation_sha256: [2; 32],
        event_ordinal: stage as u64,
        frame_id: if write { 22 } else { 11 },
        military_frame_id: if write { 22 } else { 11 },
        parent_frame_id: None,
        thread_id: 1,
        stream_pointer: if write { 0xa000 } else { 0x9000 },
        occurrence_id: if write {
            b"source-write".to_vec()
        } else {
            b"source-read".to_vec()
        },
        lifecycle_id: if write {
            b"source-write-life".to_vec()
        } else {
            b"source-read-life".to_vec()
        },
        owner_lifetime_id: owner_lifetime.clone(),
    };
    let vectors = |present: bool, seed: i32| {
        let vector = |offset: u64| VectorState {
            data_pointer: 0x1000 + offset,
            capacity_u32: 8,
            size_u32: if present { 2 } else { 0 },
            padding_u32: 0x12345678,
            tail_padding_u32: u32::MAX,
            values_i32: if present {
                vec![seed, seed.wrapping_add(1)]
            } else {
                vec![]
            },
        };
        Vectors {
            allowed: vector(0),
            per_war: vector(16),
            current: vector(32),
        }
    };
    let state = |stage, present, seed, write| StateObservation {
        witness: witness(stage, write),
        military_pointer: 0x5000,
        player: if stage == Stage::ReadEntry {
            None
        } else {
            Some(PlayerObservation {
                player_pointer: 0x6000,
                player_id_i32: player.record_owner,
                was_ever_alive_raw: 1,
                handicap_i32: -1,
            })
        },
        vectors: vectors(present, seed),
    };
    let arrays = if write_present {
        [Stage::WriteAllowed, Stage::WritePerWar, Stage::WriteCurrent]
            .into_iter()
            .enumerate()
            .map(|(i, stage)| ArrayWrite {
                witness: witness(stage, true),
                data_pointer: 0x1000 + (i as u64) * 16,
                definitions: DefinitionCollection {
                    collection_pointer: 0x7000,
                    begin_pointer: 0x7100,
                    end_pointer: 0x7110,
                    capacity_pointer: 0x7120,
                    ordered: vec![
                        DefinitionEntry {
                            pointer: 0x8000,
                            database_row_i32: -1,
                            collection_index_u32: 9,
                            operation_type_hash_u32: u32::MAX,
                            default_value_i32: 42,
                        },
                        DefinitionEntry {
                            pointer: 0x8100,
                            database_row_i32: 17,
                            collection_index_u32: 3,
                            operation_type_hash_u32: 0x80000000,
                            default_value_i32: -7,
                        },
                    ],
                },
            })
            .collect()
    } else {
        vec![]
    };
    let candidate = SnapshotCandidate {
        player,
        gamecore_sha256: CORE_IDENTITY,
        host_sha256: HOST_IDENTITY,
        producer_source_commit: [1; 20],
        producer_binary_sha256: [11; 32],
        methods_and_layout_audit_sha256: [12; 32],
        raw_observation_sha256: [2; 32],
        attribution_audit_sha256: None,
        input_save_sha256: [3; 32],
        input_core_sha256: [4; 32],
        inputs: input.clone(),
        read_entry_prior: state(Stage::ReadEntry, false, -1, false),
        initialize_entry_prior: state(Stage::InitializeEntry, true, -2, false),
        initialize_return: state(Stage::InitializeReturn, read_present, 11, false),
        read_initialized: state(Stage::ReadInitialized, read_present, 11, false),
        read_return: state(Stage::ReadReturn, read_present, 33, false),
        write_entry: state(Stage::WriteEntry, write_present, 33, true),
        default_observation: if read_present {
            DefaultObservation::Observed {
                witness: witness(Stage::DefaultQuery, false),
                value_i32: 42,
            }
        } else {
            DefaultObservation::NotReached {
                witness: witness(Stage::DefaultNotReached, false),
            }
        },
        definition_count: read_present.then(|| CountObservation {
            witness: witness(Stage::DefinitionCount, false),
            count_u32: 2,
        }),
        write_guard: WriteGuard {
            witness: witness(Stage::WriteGuard, true),
            allowed_size_u32: if write_present { 2 } else { 0 },
            branch_taken: write_present,
        },
        array_writes: arrays,
        saved_output: SavedOutput {
            witness: witness(Stage::SavedOutputBinding, true),
            output_id: b"source-output".to_vec(),
            output_core_sha256: [5; 32],
            military_record_start: 100,
            military_record_end: 200,
        },
    };
    let read = WireBinding {
        role: WireRole::SourceOnlyReadCase,
        core_sha256: [4; 32],
        occurrence_id: b"source-read".to_vec(),
        output_id: None,
        inputs: input.clone(),
    };
    let write = WireBinding {
        role: WireRole::SourceOnlyWriteCase,
        core_sha256: [5; 32],
        occurrence_id: b"source-write".to_vec(),
        output_id: Some(b"source-output".to_vec()),
        inputs: input,
    };
    (
        InitializationSnapshot::source_only(candidate).unwrap(),
        read,
        write,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn identity() -> PlayerIdentity {
        PlayerIdentity {
            manager_index: 0,
            requested_owner: 64,
            record_owner: 64,
        }
    }
    #[test]
    fn source_only_framing_uses_bound_actual_state_and_never_transfers_streams() {
        let (snapshot, read, write) = source_case(identity(), false, true);
        assert_eq!(snapshot.origin(), Origin::SourceOnly);
        assert!(!snapshot.initialization_reconstructed());
        assert!(
            !snapshot
                .context_for(identity(), &read)
                .unwrap()
                .operation_type_arrays_present
        );
        assert!(
            snapshot
                .context_for(identity(), &write)
                .unwrap()
                .operation_type_arrays_present
        );
        let mut wrong = read.clone();
        wrong.core_sha256 = write.core_sha256;
        assert_eq!(
            snapshot.context_for(identity(), &wrong),
            Err(Error::InputBinding)
        );
        let mut native = read;
        native.role = WireRole::ReadInputCore;
        assert_eq!(
            snapshot.context_for(identity(), &native),
            Err(Error::SourceOnly)
        );
        assert_eq!(
            snapshot.candidate().unwrap().inputs.database,
            DatabaseIdentity::LiveSnapshotUnavailable
        );
        assert_ne!(
            snapshot
                .candidate()
                .unwrap()
                .initialize_entry_prior
                .vectors
                .allowed
                .values_i32,
            snapshot
                .candidate()
                .unwrap()
                .read_initialized
                .vectors
                .allowed
                .values_i32
        );
    }
    #[test]
    fn candidate_preflight_preserves_raw_order_and_refuses_missing_cross_bindings() {
        let (snapshot, _, _) = source_case(identity(), true, true);
        assert_eq!(
            snapshot.candidate().unwrap().array_writes[0]
                .definitions
                .ordered[0]
                .collection_index_u32,
            9
        );
        let mut missing = snapshot.candidate().unwrap().clone();
        missing.read_return.witness.occurrence_id = b"another-input".to_vec();
        assert_eq!(
            missing.preflight(Limits::default()),
            Err(Error::WitnessBinding)
        );
        let mut wrong_stream = snapshot.candidate().unwrap().clone();
        wrong_stream.read_return.witness.stream_pointer ^= 1;
        assert_eq!(
            wrong_stream.preflight(Limits::default()),
            Err(Error::WitnessBinding)
        );
        let mut incomplete = snapshot.candidate().unwrap().clone();
        incomplete
            .read_initialized
            .vectors
            .per_war
            .values_i32
            .clear();
        assert_eq!(
            incomplete.preflight(Limits::default()),
            Err(Error::VectorSize)
        );
        let mut guard = snapshot.candidate().unwrap().clone();
        guard.write_guard.branch_taken = false;
        assert_eq!(guard.preflight(Limits::default()), Err(Error::WriteGuard));
        assert_eq!(
            snapshot.candidate().unwrap().preflight(Limits {
                max_values: 0,
                ..Limits::default()
            }),
            Err(Error::ResourceLimit)
        );
    }
}
