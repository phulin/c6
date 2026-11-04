//! Bounded supplied semantic inputs for conditional Military wire framing.
//! Consistency checks do not authenticate observations or reconstruct initialization.
use crate::player_ai_military;
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
    /// A collection of original component exports, not a Civ VI Core Save.
    SavedOutputComponents,
    #[cfg(test)]
    SourceOnlyReadCase,
    #[cfg(test)]
    SourceOnlyWriteCase,
}
impl WireRole {
    pub fn is_saved_output(self) -> bool {
        matches!(self, Self::SavedOutputCore | Self::SavedOutputComponents)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireBinding {
    pub role: WireRole,
    /// Raw Core identity, or the exact component bundle identity for
    /// SavedOutputComponents; never the .Civ6Save container identity.
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
    MissingIdentity,
    VectorSize,
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefaultQueryFacts {
    pub query_present: bool,
    pub default_value_i32: i32,
    pub civilization_i32: i32,
    pub handicap_i32: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InitializationProvenance {
    Read {
        prior_vectors: Vectors,
        has_ai_personality: bool,
        leader: Option<i32>,
        definition_count: Option<u32>,
        default_query: Option<DefaultQueryFacts>,
    },
    Write,
}
/// Supplied semantic facts, including raw vector layouts and definition order.
/// Their consistency does not establish native observation or initialization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitializationFacts {
    pub binding: WireBinding,
    pub owner: i32,
    pub was_ever_alive_raw: u8,
    pub handicap_i32: i32,
    pub core_range: [usize; 2],
    pub vectors: Vectors,
    pub active_definitions: Option<DefinitionCollection>,
    pub provenance: InitializationProvenance,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Origin {
    SuppliedConsistency,
    #[cfg(test)]
    SourceOnly,
}
#[derive(Debug, Eq, PartialEq)]
pub struct InitializationSnapshot {
    facts: InitializationFacts,
    origin: Origin,
    #[cfg(test)]
    secondary: Option<Box<InitializationFacts>>,
}
fn validate_vectors(vectors: &Vectors, limits: Limits, total: &mut usize) -> Result<(), Error> {
    for v in [&vectors.allowed, &vectors.per_war, &vectors.current] {
        if v.size_u32 > v.capacity_u32
            || v.values_i32.len() != v.size_u32 as usize
            || (v.capacity_u32 != 0 && v.data_pointer == 0)
            || v.data_pointer
                .checked_add(u64::from(v.capacity_u32) * 4)
                .is_none()
        {
            return Err(Error::VectorSize);
        }
        *total = total
            .checked_add(v.values_i32.len())
            .ok_or(Error::ResourceLimit)?;
        if v.capacity_u32 as usize > limits.max_values || *total > limits.max_values {
            return Err(Error::ResourceLimit);
        }
    }
    Ok(())
}
impl InitializationSnapshot {
    /// Validate supplied facts once at the import boundary. This constructor
    /// establishes consistency only; callers authenticate their own evidence.
    pub fn from_supplied_facts(facts: InitializationFacts, limits: Limits) -> Result<Self, Error> {
        inputs(&facts.binding.inputs)?;
        if !present(&facts.binding.core_sha256) {
            return Err(Error::MissingIdentity);
        }
        token(&facts.binding.occurrence_id, limits)?;
        match facts.binding.role {
            WireRole::ReadInputCore => {
                if facts.binding.output_id.is_some()
                    || !matches!(facts.provenance, InitializationProvenance::Read { .. })
                {
                    return Err(Error::InputBinding);
                }
            }
            WireRole::SavedOutputCore | WireRole::SavedOutputComponents => {
                token(
                    facts
                        .binding
                        .output_id
                        .as_deref()
                        .ok_or(Error::MissingIdentity)?,
                    limits,
                )?;
                if !matches!(facts.provenance, InitializationProvenance::Write) {
                    return Err(Error::InputBinding);
                }
            }
            #[cfg(test)]
            WireRole::SourceOnlyReadCase | WireRole::SourceOnlyWriteCase => {
                return Err(Error::SourceOnly)
            }
        }
        if facts.core_range[0] >= facts.core_range[1] {
            return Err(Error::CursorBinding);
        }
        let mut total = 0;
        validate_vectors(&facts.vectors, limits, &mut total)?;
        if let InitializationProvenance::Read {
            prior_vectors,
            has_ai_personality,
            leader,
            definition_count,
            default_query,
        } = &facts.provenance
        {
            validate_vectors(prior_vectors, limits, &mut total)?;
            if !has_ai_personality
                && (facts.was_ever_alive_raw != 0
                    || leader.is_some()
                    || definition_count.is_some()
                    || default_query.is_some()
                    || prior_vectors != &facts.vectors)
            {
                return Err(Error::Definitions);
            }
            if let Some(count) = definition_count {
                if facts
                    .active_definitions
                    .as_ref()
                    .is_none_or(|defs| defs.ordered.len() != *count as usize)
                {
                    return Err(Error::Definitions);
                }
            }
            if *has_ai_personality && leader.is_none() {
                return Err(Error::Definitions);
            }
            if *has_ai_personality {
                let zero = |v: &VectorState, count: usize| {
                    v.values_i32.len() == count && v.values_i32.iter().all(|v| *v == 0)
                };
                if *leader == Some(-1) {
                    let count = definition_count.ok_or(Error::Definitions)? as usize;
                    if default_query.is_some()
                        || !zero(&facts.vectors.allowed, count)
                        || !zero(&facts.vectors.per_war, count)
                    {
                        return Err(Error::Definitions);
                    }
                } else {
                    let query = default_query.as_ref().ok_or(Error::Definitions)?;
                    if query.query_present {
                        if Some(facts.vectors.allowed.size_u32) != *definition_count {
                            return Err(Error::Definitions);
                        }
                    } else if definition_count.is_some()
                        || facts.vectors.allowed != prior_vectors.allowed
                    {
                        return Err(Error::Definitions);
                    }
                }
                if !zero(
                    &facts.vectors.current,
                    facts.vectors.allowed.size_u32 as usize,
                ) {
                    return Err(Error::Definitions);
                }
            }
            if let Some(query) = default_query {
                if query.handicap_i32 != facts.handicap_i32 {
                    return Err(Error::Definitions);
                }
            }
        }
        if let Some(defs) = &facts.active_definitions {
            if defs.begin_pointer > defs.end_pointer
                || defs.end_pointer > defs.capacity_pointer
                || (defs.end_pointer - defs.begin_pointer) % 8 != 0
                || (defs.capacity_pointer - defs.begin_pointer) % 8 != 0
                || (defs.end_pointer - defs.begin_pointer) / 8 != defs.ordered.len() as u64
            {
                return Err(Error::Definitions);
            }
            if (defs.capacity_pointer - defs.begin_pointer) / 8 > limits.max_values as u64
                || defs.ordered.len() > limits.max_values.saturating_sub(total)
            {
                return Err(Error::ResourceLimit);
            }
            if matches!(facts.provenance, InitializationProvenance::Write)
                && facts.vectors.allowed.size_u32 != 0
                && [
                    &facts.vectors.allowed,
                    &facts.vectors.per_war,
                    &facts.vectors.current,
                ]
                .iter()
                .any(|v| v.values_i32.len() < defs.ordered.len())
            {
                return Err(Error::Definitions);
            }
        } else if matches!(facts.provenance, InitializationProvenance::Write)
            && facts.vectors.allowed.size_u32 != 0
        {
            return Err(Error::Definitions);
        }
        Ok(Self {
            facts,
            origin: Origin::SuppliedConsistency,
            #[cfg(test)]
            secondary: None,
        })
    }
    pub fn facts(&self) -> &InitializationFacts {
        &self.facts
    }
    pub fn origin(&self) -> Origin {
        self.origin
    }
    pub const fn initialization_reconstructed(&self) -> bool {
        false
    }
    pub(super) fn check_cursor(&self, cursor: usize) -> Result<(), Error> {
        #[cfg(test)]
        if self.origin == Origin::SourceOnly {
            return Ok(());
        }
        if cursor != self.facts.core_range[0] {
            return Err(Error::CursorBinding);
        }
        Ok(())
    }
    fn facts_for(&self, binding: &WireBinding) -> Result<&InitializationFacts, Error> {
        #[cfg(test)]
        if self.origin == Origin::SourceOnly {
            if !matches!(
                binding.role,
                WireRole::SourceOnlyReadCase | WireRole::SourceOnlyWriteCase
            ) {
                return Err(Error::SourceOnly);
            }
            if let Some(secondary) = &self.secondary {
                if &secondary.binding == binding {
                    return Ok(secondary);
                }
            }
        }
        if &self.facts.binding != binding {
            return Err(Error::InputBinding);
        }
        Ok(&self.facts)
    }
    pub(super) fn context_for(
        &self,
        player: PlayerIdentity,
        binding: &WireBinding,
    ) -> Result<player_ai_military::Context, Error> {
        let facts = self.facts_for(binding)?;
        if facts.was_ever_alive_raw == 0
            || player.record_owner != facts.owner
            || player.requested_owner != facts.owner
        {
            return Err(Error::OwnerBinding);
        }
        Ok(player_ai_military::Context {
            operation_type_arrays_present: facts.vectors.allowed.size_u32 != 0,
        })
    }
    pub(super) fn check_saved_player(
        &self,
        owner: i32,
        binding: &WireBinding,
    ) -> Result<(), Error> {
        if self.facts_for(binding)?.owner != owner {
            return Err(Error::OwnerBinding);
        }
        Ok(())
    }
}
#[cfg(test)]
pub(super) fn source_case(
    player: PlayerIdentity,
    read_present: bool,
    write_present: bool,
) -> (InitializationSnapshot, WireBinding, WireBinding) {
    let input = InputIdentities {
        configuration_sha256: [6; 32],
        rules_inputs_sha256: [7; 32],
        database: DatabaseIdentity::LiveSnapshotUnavailable,
        dlc_inputs_sha256: [8; 32],
        mods_inputs_sha256: [9; 32],
        content_inputs_sha256: [10; 32],
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
    let facts = |binding: WireBinding, present: bool| {
        let v = VectorState {
            data_pointer: 0x1000,
            capacity_u32: 2,
            size_u32: if present { 2 } else { 0 },
            padding_u32: 0x12345678,
            tail_padding_u32: u32::MAX,
            values_i32: if present { vec![11, 12] } else { vec![] },
        };
        InitializationFacts {
            binding,
            owner: player.record_owner,
            was_ever_alive_raw: 1,
            handicap_i32: -1,
            core_range: [100, 200],
            vectors: Vectors {
                allowed: v.clone(),
                per_war: v.clone(),
                current: v,
            },
            active_definitions: None,
            provenance: InitializationProvenance::Write,
        }
    };
    (
        InitializationSnapshot {
            facts: facts(read.clone(), read_present),
            origin: Origin::SourceOnly,
            secondary: Some(Box::new(facts(write.clone(), write_present))),
        },
        read,
        write,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn facts() -> InitializationFacts {
        let player = PlayerIdentity {
            manager_index: 0,
            requested_owner: 13,
            record_owner: 13,
        };
        let (source, _, _) = source_case(player, false, false);
        let mut facts = source.facts;
        facts.binding.role = WireRole::ReadInputCore;
        facts.provenance = InitializationProvenance::Read {
            prior_vectors: facts.vectors.clone(),
            has_ai_personality: true,
            leader: Some(3),
            definition_count: None,
            default_query: Some(DefaultQueryFacts {
                query_present: false,
                default_value_i32: 0,
                civilization_i32: 4,
                handicap_i32: -1,
            }),
        };
        facts
    }
    #[test]
    fn supplied_consistency_is_bound_and_does_not_claim_reconstruction() {
        let facts = facts();
        let binding = facts.binding.clone();
        let snapshot =
            InitializationSnapshot::from_supplied_facts(facts, Limits::default()).unwrap();
        assert_eq!(snapshot.origin(), Origin::SuppliedConsistency);
        assert!(!snapshot.initialization_reconstructed());
        let player = PlayerIdentity {
            manager_index: 37,
            requested_owner: 13,
            record_owner: 13,
        };
        assert!(
            !snapshot
                .context_for(player, &binding)
                .unwrap()
                .operation_type_arrays_present
        );
        assert_eq!(snapshot.check_cursor(99), Err(Error::CursorBinding));
        assert_eq!(
            snapshot.check_saved_player(12, &binding),
            Err(Error::OwnerBinding)
        );
        let mut wrong = binding.clone();
        wrong.core_sha256[0] ^= 1;
        assert_eq!(
            snapshot.context_for(player, &wrong),
            Err(Error::InputBinding)
        );
        let mut wrong = binding;
        wrong.occurrence_id.push(b'0');
        assert_eq!(
            snapshot.context_for(player, &wrong),
            Err(Error::InputBinding)
        );
    }
    #[test]
    fn constructor_rejects_bad_identity_range_layout_and_provenance() {
        let check = |facts| {
            InitializationSnapshot::from_supplied_facts(facts, Limits::default()).unwrap_err()
        };
        let mut bad = facts();
        bad.binding.inputs.configuration_sha256 = [0; 32];
        assert_eq!(check(bad), Error::MissingIdentity);
        let mut bad = facts();
        bad.core_range = [100, 100];
        assert_eq!(check(bad), Error::CursorBinding);
        let mut bad = facts();
        bad.vectors.allowed.size_u32 = 1;
        assert_eq!(check(bad), Error::VectorSize);
        let mut bad = facts();
        bad.vectors.allowed.data_pointer = u64::MAX;
        assert_eq!(check(bad), Error::VectorSize);
        let mut bad = facts();
        bad.provenance = InitializationProvenance::Write;
        assert_eq!(check(bad), Error::InputBinding);
        let mut bad = facts();
        bad.binding.output_id = Some(vec![1]);
        assert_eq!(check(bad), Error::InputBinding);
        assert_eq!(
            InitializationSnapshot::from_supplied_facts(
                facts(),
                Limits {
                    max_values: 1,
                    ..Limits::default()
                }
            )
            .unwrap_err(),
            Error::ResourceLimit
        );
    }
    #[test]
    fn source_only_facts_cannot_bind_real_roles() {
        let player = PlayerIdentity {
            manager_index: 0,
            requested_owner: 13,
            record_owner: 13,
        };
        let (snapshot, mut read, write) = source_case(player, false, true);
        assert!(
            !snapshot
                .context_for(player, &read)
                .unwrap()
                .operation_type_arrays_present
        );
        assert!(
            snapshot
                .context_for(player, &write)
                .unwrap()
                .operation_type_arrays_present
        );
        read.role = WireRole::ReadInputCore;
        assert_eq!(snapshot.context_for(player, &read), Err(Error::SourceOnly));
    }
    #[test]
    fn read_query_miss_retains_nonempty_prior_without_definition_table() {
        let mut f = facts();
        f.vectors.allowed.size_u32 = 2;
        f.vectors.allowed.values_i32 = vec![41, -7];
        f.vectors.current.size_u32 = 2;
        f.vectors.current.values_i32 = vec![0, 0];
        if let InitializationProvenance::Read { prior_vectors, .. } = &mut f.provenance {
            *prior_vectors = f.vectors.clone();
        }
        let binding = f.binding.clone();
        let snapshot =
            InitializationSnapshot::from_supplied_facts(f.clone(), Limits::default()).unwrap();
        assert!(
            snapshot
                .context_for(
                    PlayerIdentity {
                        manager_index: 0,
                        requested_owner: f.owner,
                        record_owner: f.owner
                    },
                    &binding
                )
                .unwrap()
                .operation_type_arrays_present
        );
        if let InitializationProvenance::Read {
            definition_count, ..
        } = &mut f.provenance
        {
            *definition_count = Some(2);
        }
        assert_eq!(
            InitializationSnapshot::from_supplied_facts(f, Limits::default()).unwrap_err(),
            Error::Definitions
        );
    }
    #[test]
    fn no_personality_and_definition_count_contradictions_refuse() {
        let mut f = facts();
        f.provenance = InitializationProvenance::Read {
            prior_vectors: f.vectors.clone(),
            has_ai_personality: false,
            leader: None,
            definition_count: None,
            default_query: None,
        };
        assert_eq!(
            InitializationSnapshot::from_supplied_facts(f.clone(), Limits::default()).unwrap_err(),
            Error::Definitions
        );
        f.was_ever_alive_raw = 0;
        assert!(InitializationSnapshot::from_supplied_facts(f.clone(), Limits::default()).is_ok());
        if let InitializationProvenance::Read {
            definition_count, ..
        } = &mut f.provenance
        {
            *definition_count = Some(0);
        }
        assert_eq!(
            InitializationSnapshot::from_supplied_facts(f, Limits::default()).unwrap_err(),
            Error::Definitions
        );
    }
}
