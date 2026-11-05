//! Outer ResolveCombat's real attacks/movement expenditure, after inner Resolve.
use super::{GeneratedMembers, RequiresOuterResolution};
use crate::{loaded_upkeep as upkeep, unit_movement_state::Inputs};
use c6_core::{
    loaded_state::LoadedState,
    unit_placement_transition::{PlacementPayload, PlacementReport},
};
type ProviderError<E> = <E as upkeep::Effects>::Error;
#[derive(Debug)]
pub enum Error<E> {
    Actor(super::super::Error),
    State(c6_core::loaded_state::Error),
    Effect(E),
    Finish(c6_core::movement_execution::FormationMovementError<upkeep::Error<E>>),
    Sync(c6_core::formation_upkeep::FormationUpkeepError<upkeep::Error<E>>),
    MissingFormation,
    ForcedRetreat,
}
#[must_use]
pub struct RequiresPostCombatEffects {
    pub members: GeneratedMembers,
}

/// ChangeAttacksRemaining(-1) precedes CanMoveAfterAttacking and fresh attacks
/// queries, including when the attacker is already dead. FinishMoves and the
/// subsequent extra Sync are distinct native calls; neither moves its location.
pub fn consume_attack<E>(
    state: &mut LoadedState,
    inputs: &Inputs<'_>,
    effects: &mut E,
    pending: RequiresOuterResolution,
) -> Result<RequiresPostCombatEffects, Error<ProviderError<E>>>
where
    E: upkeep::Effects + upkeep::OperationReports<Error = ProviderError<E>>,
{
    let members = pending.members;
    let reference = members.damage.attacker;
    consume_reference_attack(state, inputs, effects, reference)?;
    if super::super::actor(state, reference)
        .map_err(Error::Actor)?
        .k_combat_modifers
        .b_can_force_retreat
    {
        return Err(Error::ForcedRetreat);
    }
    Ok(RequiresPostCombatEffects { members })
}

/// Shared actual Unit budget tail. The enclosing ResolveCombat checks both
/// Unit pointers before forced retreat, so District defenders skip that branch.
pub(crate) fn consume_reference_attack<E>(
    state: &mut LoadedState,
    inputs: &Inputs<'_>,
    effects: &mut E,
    reference: c6_core::map::UnitReference,
) -> Result<(), Error<ProviderError<E>>>
where
    E: upkeep::Effects + upkeep::OperationReports<Error = ProviderError<E>>,
{
    let current = super::super::actor(state, reference).map_err(Error::Actor)?;
    let old = current.i_attacks_remaining;
    let next = old.wrapping_sub(1).max(0);
    if next != old {
        state
            .unit_mut(reference.player(), reference.id)
            .map_err(Error::State)?
            .i_attacks_remaining = next;
        let current = super::super::actor(state, reference).map_err(Error::Actor)?;
        let identity = c6_core::unit::ComponentId {
            owner: current.e_owner as i16,
            component_type: 1,
            id: current.id,
        };
        effects
            .placement_report(
                state,
                PlacementReport {
                    event_hash: 0x35597c21,
                    identity,
                    payload: PlacementPayload::Integer(current.i_attacks_remaining),
                },
            )
            .map_err(Error::Effect)?;
    }
    let current = super::super::actor(state, reference).map_err(Error::Actor)?;
    if current.i_move_after_attacking <= 0 {
        if current.i_attacks_remaining <= 0 {
            let formation = state
                .unit_runtime(current.e_owner, current.id)
                .map_err(Error::State)?
                .formation
                .ok_or(Error::MissingFormation)?;
            upkeep::finish_formation_moves(
                state,
                formation.owner,
                formation.requested_id,
                inputs,
                effects,
            )
            .map_err(Error::Finish)?;
        }
        // Reread current EditFormation after all FinishMoves subscribers.
        let current = super::super::actor(state, reference).map_err(Error::Actor)?;
        let formation = state
            .unit_runtime(current.e_owner, current.id)
            .map_err(Error::State)?
            .formation
            .ok_or(Error::MissingFormation)?;
        upkeep::update_formation(
            state,
            formation.owner,
            formation.requested_id,
            inputs,
            effects,
        )
        .map_err(Error::Sync)?;
    }
    Ok(())
}
