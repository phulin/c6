//! Executable first TestAttackInto stage over canonical loaded actors/Formations.
//! This is NOT complete attack legality, an applied attack, or combat resolution.
use crate::{formation_path_state as path, formation_rules, movement_placement_state};
use c6_core::{
    loaded_state::LoadedState,
    map::UnitReference,
    melee_attack as kernel,
    movement_shore::PlacementRequest,
    unit::{PlotCoord, Unit},
};

#[derive(Debug)]
pub enum Error {
    World(c6_core::world::MissingInput),
    State(c6_core::loaded_state::Error),
    MissingActor(UnitReference),
    Combat(crate::loaded_actions::Error),
    Path(path::Error),
    Placement(movement_placement_state::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "melee attack query stage: {self:?}")
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Eq, PartialEq)]
pub enum Prefix {
    Rejected(kernel::Rejected),
    RequiresTargetSelection(TargetSelection),
}
/// Internal query continuation; it conveys NO successful action/admission.
/// Next comes religious/district/attacking-defender selection, CanAttackTarget,
/// GenerateCombatResults with its real RNG, ResolveCombat and all callbacks.
#[derive(Debug, Eq, PartialEq)]
pub struct TargetSelection {
    actor: UnitReference,
    next: kernel::TargetSelection,
}
impl TargetSelection {
    pub fn actor(&self) -> UnitReference {
        self.actor
    }
    pub fn destination(&self) -> PlotCoord {
        self.next.destination()
    }
}

struct Current<'s, 'a, 'r> {
    state: &'s LoadedState,
    actor: UnitReference,
    combat: &'a formation_rules::Definitions,
    path: &'a path::Definitions<'a, 'r>,
    runtime: &'s path::Runtime<'a>,
}
fn actor(state: &LoadedState, id: UnitReference) -> Result<&Unit, Error> {
    state
        .world()
        .unit(id)
        .map_err(Error::World)?
        .map(|u| u.record)
        .ok_or(Error::MissingActor(id))
}
impl kernel::Context for Current<'_, '_, '_> {
    type Error = Error;
    fn attacks_remaining(&mut self) -> Result<i32, Error> {
        Ok(actor(self.state, self.actor)?.i_attacks_remaining)
    }
    fn can_attack(&mut self) -> Result<bool, Error> {
        self.combat
            .can_attack(self.state, actor(self.state, self.actor)?)
            .map_err(Error::Combat)
    }
    fn formation_placement(&mut self, destination: PlotCoord) -> Result<i32, Error> {
        let u = actor(self.state, self.actor)?;
        // Construct only after native attack-count/CanAttack guards; unknown
        // placement inputs cannot block a source-proven early rejection.
        let placement = path::placement_context(self.state, self.path, self.runtime)
            .map_err(Error::Path)?
            .with_combat_definitions(self.combat);
        placement
            .formation_placement(
                u.e_owner,
                u.formation_id,
                destination,
                PlacementRequest::from_validate_options(1),
            )
            .map_err(Error::Placement)
    }
}

/// The coordinate overload's first real gameplay query stage (28cc330).
/// No native answer, pathfinder result, defender choice, or fabricated damage is
/// supplied. Formation placement reads the SAME current World and actual rules.
#[allow(clippy::too_many_arguments)]
pub fn test_attack_into_prefix<'a, 'r>(
    state: &LoadedState,
    combat: &'a formation_rules::Definitions,
    definitions: &'a path::Definitions<'a, 'r>,
    runtime: &path::Runtime<'a>,
    actor: UnitReference,
    destination: PlotCoord,
) -> Result<Prefix, Error> {
    let mut current = Current {
        state,
        actor,
        combat,
        path: definitions,
        runtime,
    };
    match kernel::test_attack_into_prefix(&mut current, destination)? {
        kernel::Prefix::Rejected(why) => Ok(Prefix::Rejected(why)),
        kernel::Prefix::RequiresTargetSelection(next) => {
            Ok(Prefix::RequiresTargetSelection(TargetSelection {
                actor,
                next,
            }))
        }
    }
}

struct Domain<'a> {
    state: &'a LoadedState,
    combat: &'a formation_rules::Definitions,
    actor: UnitReference,
}
impl kernel::DomainContext for Domain<'_> {
    type Error = Error;
    fn domain(&mut self) -> Result<i32, Error> {
        let u = actor(self.state, self.actor)?;
        Ok(self
            .state
            .unit_runtime(u.e_owner, u.id)
            .map_err(Error::State)?
            .domain)
    }
    fn bombard(&mut self) -> Result<i32, Error> {
        self.combat
            .bombard(self.state, actor(self.state, self.actor)?)
            .map_err(Error::Combat)
    }
    fn ranged(&mut self) -> Result<i32, Error> {
        self.combat
            .ranged(self.state, actor(self.state, self.actor)?)
            .map_err(Error::Combat)
    }
}
/// Rules.CanAttackDomain, separately reusable at its later original callsite.
/// It must not be moved before the prefix's mandatory placement/target queries.
pub fn can_attack_domain(
    state: &LoadedState,
    combat: &formation_rules::Definitions,
    actor: UnitReference,
    target_domain: i32,
    flags: u32,
) -> Result<bool, Error> {
    kernel::can_attack_domain(
        &mut Domain {
            state,
            combat,
            actor,
        },
        target_domain,
        flags,
    )
}
