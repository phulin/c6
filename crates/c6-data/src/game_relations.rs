//! Admitted native War/Alliance definitions and attributed owned relation load.
//! This is a current saved-prefix query input, not complete diplomacy gameplay.
use crate::{
    loaded_actions::resources::Bound,
    registry::LinuxDefinitionRegistry,
    unit_initialization_definitions::{self, collection},
    Rules,
};
use c6_core::{
    core_player::initialization::{InputIdentities, WireRole},
    loaded_state::{ComponentSnapshotWitness, Error as StateError, LoadedState},
    movement_world::DefinitionLookup,
    unit::Limits,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

const CORE: &str = "ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884";
const TRACE: &str = "528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae";
const OWNER: &str = "393347b11100e83ca439a3967d851df8a543f55a006aa3b68b8a516098e9d34b";
const PREFIX: &str = "757c912ba40bee8a7dd460d59cc2041c04c86d12b3fa638464caf063b87e41af";
const PREFIX_BYTES: usize = 121_005;
const CORE_BYTES: usize = 80_657_462;

#[derive(Debug)]
pub enum Error {
    Resources(unit_initialization_definitions::Error),
    State(StateError),
    Binding,
    Definitions,
    Attribution,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Game.DiplomaticRelations admission: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Complete native-order definition inputs. War ValidateHash resolves to the
/// definition's type hash; Alliance HashToIndex resolves to collection index.
/// Both retain native Find's ordinal-first and last-inserted duplicate semantics.
pub struct Definitions {
    identities: InputIdentities,
    wars: LinuxDefinitionRegistry,
    alliances: LinuxDefinitionRegistry,
    war_hashes: Vec<u32>,
    alliance_entries: Vec<(u32, i32)>,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &Bound<'_>,
        rules_runtime: &[u8],
    ) -> Result<Self, Error> {
        if bound.destination_binding() != state.binding() {
            return Err(Error::Binding);
        }
        unit_initialization_definitions::Definitions::validate_resources(
            rules,
            bound,
            rules_runtime,
        )
        .map_err(Error::Resources)?;
        // Exact original SQL: complete unfiltered collections ORDER BY rowid.
        // The shared builder retains rowid separately from size-before-append.
        let wars = collection(rules, "Wars", "WarType").map_err(Error::Resources)?;
        let alliances = collection(rules, "Alliances", "AllianceType").map_err(Error::Resources)?;
        let war_hashes = (0..wars.count())
            .map(|i| wars.type_hash(i).ok_or(Error::Definitions))
            .collect::<Result<_, _>>()?;
        let alliance_entries = (0..alliances.count())
            .map(|i| {
                Ok((
                    alliances.type_hash(i).ok_or(Error::Definitions)?,
                    i32::try_from(i).map_err(|_| Error::Definitions)?,
                ))
            })
            .collect::<Result<_, Error>>()?;
        Ok(Self {
            identities: state.binding().inputs.clone(),
            wars,
            alliances,
            war_hashes,
            alliance_entries,
        })
    }
    pub fn wars(&self) -> &LinuxDefinitionRegistry {
        &self.wars
    }
    pub fn alliances(&self) -> &LinuxDefinitionRegistry {
        &self.alliances
    }
    pub fn war_hashes(&self) -> &[u32] {
        &self.war_hashes
    }
    pub fn alliance_entries(&self) -> &[(u32, i32)] {
        &self.alliance_entries
    }

    /// Admit the existing ff31 component using its genuine serializer-owner
    /// trace. This is an explicit supported fixture, not a fixed-offset general
    /// Core parser. Complete source bytes and every unknown suffix remain owned.
    pub fn install_attributed_snapshot(
        &self,
        state: &mut LoadedState,
        owner_trace: &[u8],
        limits: Limits,
    ) -> Result<(), Error> {
        if self.identities != state.binding().inputs {
            return Err(Error::Binding);
        }
        let witness = snapshot_witness(state, owner_trace)?;
        state
            .install_game_relations(witness, &self.war_hashes, &self.alliance_entries, limits)
            .map_err(Error::State)
    }
}
fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn matches_hash(bytes: &[u8], expected: &str) -> bool {
    format!("{:x}", Sha256::digest(bytes)) == expected
}
fn snapshot_witness(state: &LoadedState, trace: &[u8]) -> Result<ComponentSnapshotWitness, Error> {
    let bytes = state.source_image();
    if bytes.len() != CORE_BYTES
        || !matches_hash(bytes, CORE)
        || state.binding().core_sha256 != digest(bytes)
        || !matches_hash(trace, TRACE)
    {
        return Err(Error::Attribution);
    }
    let mut frame = None;
    for line in trace.split(|b| *b == b'\n').filter(|line| !line.is_empty()) {
        let row: Value = serde_json::from_slice(line).map_err(|_| Error::Attribution)?;
        if row["event"] == "owner"
            && row["save"] == 1
            && row["owner"] == "GameCore::Game::DiplomaticRelations"
        {
            if frame.is_some() || row["depth"] != 1 || row["parent"] != 1 || row["call"] != 175316 {
                return Err(Error::Attribution);
            }
            let offset = |field: &str| {
                row[field]
                    .as_u64()
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or(Error::Attribution)
            };
            frame = Some([offset("start")?, offset("end")?]);
        }
    }
    let range = frame.ok_or(Error::Attribution)?;
    let owner = bytes.get(range[0]..range[1]).ok_or(Error::Attribution)?;
    if !matches_hash(owner, OWNER)
        || !owner
            .get(..PREFIX_BYTES)
            .is_some_and(|prefix| matches_hash(prefix, PREFIX))
    {
        return Err(Error::Attribution);
    }
    Ok(ComponentSnapshotWitness {
        source_role: WireRole::SavedOutputCore,
        snapshot_sha256: digest(bytes),
        snapshot_bytes: bytes.len(),
        owner_trace_sha256: digest(trace),
        owner_range: range,
        decoded_prefix_bytes: PREFIX_BYTES,
    })
}
