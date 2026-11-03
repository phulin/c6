//! Admitted native War/Alliance definitions and attributed owned relation load.
//! This is a current saved-prefix query input, not complete diplomacy gameplay.
use crate::{
    registry::LinuxDefinitionRegistry,
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{self, collection},
    Rules,
};
use c6_core::{
    core_player::initialization::InputIdentities,
    loaded_state::{ComponentSnapshotWitness, Error as StateError, LoadedState},
    movement_world::DefinitionLookup,
    unit::Limits,
};

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
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        if bound.destination_binding() != state.binding() {
            return Err(Error::Binding);
        }
        unit_initialization_definitions::Definitions::validate_resources(rules, bound)
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

    /// Install a supplied component witness using the core once-pristine checks.
    /// The witness is a source description, not proof of native runtime state.
    pub fn install_component_snapshot(
        &self,
        state: &mut LoadedState,
        witness: ComponentSnapshotWitness,
        limits: Limits,
    ) -> Result<(), Error> {
        if self.identities != state.binding().inputs {
            return Err(Error::Binding);
        }
        state
            .install_game_relations(witness, &self.war_hashes, &self.alliance_entries, limits)
            .map_err(Error::State)
    }
}
