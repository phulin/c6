//! Internal AttackInto execution prefix, distinct from TestAttackInto or outer
//! UnitOperation entitlement. Full generation, damage/RNG and Resolve remain
//! mandatory; no AppliedAttack result is provided.
use super::{
    proposals, results,
    target_selection::{self, Defender, Inputs},
};
use crate::{formation_rules, loaded_upkeep};
use c6_core::{
    loaded_state::LoadedState,
    map::UnitReference,
    melee_attack::{proposals::registry, results as kernel},
    unit::PlotCoord,
};

#[derive(Debug)]
pub enum Error<E> {
    Selection(target_selection::Error),
    Clear(Box<loaded_upkeep::Error<E>>),
    Members(results::Error),
    MissingDistrictDefinitions,
}
pub enum Outcome<'s, 'e, E> {
    NoTarget,
    RequiresGeneration(Generation<'s, 'e, E>),
}
pub struct Generation<'s, 'e, E> {
    state: &'s mut LoadedState,
    actor: UnitReference,
    defender: Defender,
    members: kernel::Members,
    attempted: bool,
    effects: &'e mut E,
}
#[derive(Debug)]
pub enum GenerateError<E> {
    AlreadyAttempted,
    Members(results::Error),
    Unsupported(&'static str),
    Proposal(Box<registry::DispatchError<proposals::Error<E>>>),
}
#[must_use]
pub struct RequiresStrengthModifiers {
    pub proposal_id: u32,
    pub proposal: registry::Handle,
    pub base_strengths: kernel::BaseStrengths,
}

/// Fresh source selection: religion, enemy defensible District, otherwise
/// CanAttackDomain/GetBestDefender flags9/19. No readiness or CanAttackTarget
/// query is repeated and no previously tested defender is accepted as input.
pub fn begin<'s, 'e, E: loaded_upkeep::OperationReports>(
    state: &'s mut LoadedState,
    inputs: &Inputs<'_>,
    actor: UnitReference,
    destination: PlotCoord,
    effects: &'e mut E,
) -> Result<Outcome<'s, 'e, E>, Error<E::Error>> {
    let Some(defender) =
        target_selection::select_execution_target(state, inputs, actor, destination)
            .map_err(Error::Selection)?
    else {
        return Ok(Outcome::NoTarget);
    };
    let mut members = kernel::Members::new(c6_core::formation_path_cache::INVALID_CACHE_LOCATION);
    match defender {
        Defender::Unit(unit) => {
            let id = unit.component_id();
            // Source28c9800: clear the freshly selected DEFENDER before member
            // construction/Generate. Callbacks may mutate this same live state.
            loaded_upkeep::clear_unit_operations(state, i32::from(id.owner), id.id, effects)
                .map_err(|e| Error::Clear(Box::new(e)))?;
            let _next = results::prepare_units(state, &mut members, actor, unit, 0)
                .map_err(Error::Members)?;
        }
        Defender::District(_) => {
            // District branch has no pre-Generate Unit Clear.
            let definitions = inputs.districts.ok_or(Error::MissingDistrictDefinitions)?;
            let _next =
                results::prepare_district(state, definitions, &mut members, actor, destination, 0)
                    .map_err(Error::Members)?;
        }
    }
    Ok(Outcome::RequiresGeneration(Generation {
        state,
        actor,
        defender,
        members,
        attempted: false,
        effects,
    }))
}
impl<E> Generation<'_, '_, E> {
    pub fn state(&self) -> &LoadedState {
        self.state
    }
    pub fn members(&self) -> &kernel::Members {
        &self.members
    }
    pub fn selected_defender(&self) -> Defender {
        self.defender
    }
    /// Continue source-ordered base strengths and actual proposal registration.
    /// Failures are not replayable: the defender Clear, registration and earlier
    /// callback effects remain in the authoritative state for diagnosis.
    pub fn advance_proposal(
        &mut self,
        definitions: &formation_rules::Definitions,
    ) -> Result<RequiresStrengthModifiers, GenerateError<<E as proposals::Effects>::Error>>
    where
        E: proposals::Effects,
    {
        if self.attempted {
            return Err(GenerateError::AlreadyAttempted);
        }
        self.attempted = true;
        let Defender::Unit(defender) = self.defender else {
            return Err(GenerateError::Unsupported("Unit/District generation"));
        };
        let strengths = results::current_unit_strengths(
            self.state,
            definitions,
            self.actor,
            defender,
            self.members.location,
        )
        .map_err(GenerateError::Members)?;
        let kernel::UnitGeneration::RequiresProposeCombat(base_strengths) = strengths else {
            return Err(GenerateError::Unsupported(
                "support FormationClass generation",
            ));
        };
        let (proposal_id, proposal) = proposals::Context::new(self.state, self.effects)
            .propose_units(self.actor, defender, self.members.location)
            .map_err(|e| GenerateError::Proposal(Box::new(e)))?;
        // Attacker fortify reset, attacker Clear and CompleteMovement occur
        // only AFTER successful complete Generate, not after this prefix.
        Ok(RequiresStrengthModifiers {
            proposal_id,
            proposal,
            base_strengths,
        })
    }
}
