//! Linux XP2 Game.GreatPeople::DoTurn (29cf480).
//!
//! The initialized, unclaimed timeline path checks guarantees and lapsed claims
//! without regenerating candidates or consuming RNG. Initial generation,
//! matching guarantees and recruitment remain explicit unsupported transitions.
use crate::{
    registry::LinuxDefinitionRegistry, unit_initialization_definitions::collection, Rules,
};
use c6_core::{
    core_player::initialization::WireBinding,
    game_great_people::{GreatPeople, TimelineEntry},
    loaded_state::LoadedState,
};

pub struct Definitions {
    classes: LinuxDefinitionRegistry,
    individuals: LinuxDefinitionRegistry,
    eras: LinuxDefinitionRegistry,
}
impl Definitions {
    pub fn from_rules(rules: &Rules) -> Result<Self, Error> {
        // GetBuilderSql21fc580/21fd3c0/21f7e90 all order by rowid.
        let get = |table, column| collection(rules, table, column).map_err(Error::Definitions);
        Ok(Self {
            classes: get("GreatPersonClasses", "GreatPersonClassType")?,
            individuals: get("GreatPersonIndividuals", "GreatPersonIndividualType")?,
            eras: get("Eras", "EraType")?,
        })
    }
    fn entry(&self, entry: &TimelineEntry) -> Result<(), Error> {
        for (hash, registry) in [
            (entry.individual_hash, &self.individuals),
            (entry.class_hash, &self.classes),
            (entry.era_hash, &self.eras),
        ] {
            if hash != u32::MAX && registry.find_index(hash).is_none() {
                return Err(Error::Missing("GreatPeople timeline definition"));
            }
        }
        Ok(())
    }
}

pub struct Initial {
    pub binding: WireBinding,
    /// The current-class overlays must contain the complete live dense array,
    /// including invalid entries, in native class order. A sparse save overlay
    /// alone does not establish the current timeline.
    pub wire: GreatPeople,
}
pub struct Runtime {
    binding: WireBinding,
    pub wire: GreatPeople,
}
#[derive(Debug)]
pub enum Error {
    Missing(&'static str),
    Definitions(crate::unit_initialization_definitions::Error),
    Requires(Transition),
}
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Transition {
    InitializeTimeline,
    Guarantee { class_hash: u32, player: i32 },
    Recruit { individual_hash: u32, player: i32 },
}
impl Runtime {
    pub fn from_initial(
        state: &LoadedState,
        definitions: &Definitions,
        input: Initial,
    ) -> Result<Self, Error> {
        if input.binding != *state.binding() {
            return Err(Error::Missing("GreatPeople initial binding"));
        }
        if !matches!(input.wire.wire_version, 4 | 5) {
            return Err(Error::Missing("GreatPeople wire version"));
        }
        let current = &input.wire.current_timeline_by_class;
        if current.len() != definitions.classes.entries().len()
            || current
                .iter()
                .zip(definitions.classes.entries())
                .any(|(entry, class)| {
                    entry.definition_hash != crate::registry::linux_type_hash(&class.type_name)
                })
        {
            return Err(Error::Missing("GreatPeople complete live class order"));
        }
        for entry in input
            .wire
            .timeline
            .iter()
            .chain(current.iter().map(|e| &e.value))
        {
            definitions.entry(entry)?;
        }
        for guarantee in &input.wire.guarantees {
            if definitions
                .classes
                .find_index(guarantee.class_hash)
                .is_none()
                || guarantee
                    .era_hash
                    .is_some_and(|h| h != u32::MAX && definitions.eras.find_index(h).is_none())
            {
                return Err(Error::Missing("GreatPeople guarantee definition"));
            }
        }
        Ok(Self {
            binding: input.binding,
            wire: input.wire,
        })
    }
}
fn valid(entry: &TimelineEntry) -> bool {
    // TimelineEntry::IsValid29ce2b0 checks only these three native enums.
    entry.individual_hash != u32::MAX && entry.class_hash != u32::MAX && entry.era_hash != u32::MAX
}

pub fn run<P>(
    state: &mut LoadedState,
    _definitions: &Definitions,
    runtime: &mut Runtime,
    backend: &mut crate::loaded_actions::stock_notifications::Runtime<'_, P>,
) -> Result<(), Error> {
    if runtime.binding != *state.binding() {
        return Err(Error::Missing("GreatPeople current binding"));
    }
    if runtime.wire.initialized_raw == 0 {
        return Err(Error::Requires(Transition::InitializeTimeline));
    }
    // CheckGuarantees29cf630 is called with era=-1/player=-1. Entries bound
    // to another era are reinserted unchanged; only unbound guarantees execute.
    for guarantee in &runtime.wire.guarantees {
        if guarantee.era_hash.is_none_or(|era| era == u32::MAX) {
            return Err(Error::Requires(Transition::Guarantee {
                class_hash: guarantee.class_hash,
                player: guarantee.player_ordinal,
            }));
        }
    }
    // CheckLapsedClaims29cf880 iterates the complete current dense timeline.
    // It neither tests points nor rerolls unclaimed candidates.
    for current in &runtime.wire.current_timeline_by_class {
        let entry = &current.value;
        if valid(entry) && entry.claimant_ordinal != -1 {
            return Err(Error::Requires(Transition::Recruit {
                individual_hash: entry.individual_hash,
                player: entry.claimant_ordinal,
            }));
        }
    }
    backend.reached.push("Game.GreatPeople.DoTurn");
    Ok(())
}
