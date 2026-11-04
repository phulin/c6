//! Ordinary melee resolution over the current owned Units. Partial failures keep
//! the native prefix; generated damage is never reported as a resolved attack.
use super::execution::{GeneratedMembers, RequiresResolve};
use crate::{
    loaded_upkeep, unit_kill, unit_placement_state as placement, unit_sight_state as sight,
};
use c6_core::{loaded_state::LoadedState, map::UnitReference};
type EffectsError<E> = <E as sight::Effects>::Error;
#[derive(Debug)]
pub enum Error<E> {
    Damage(c6_core::unit_healing::UnitHealingError<placement::Error<E>>),
    Actor(super::Error),
    Maximum(crate::formation_movement_cost::Error),
    Kill(Box<unit_kill::Error<E>>),
}
/// HP changes and delayed death have completed. XP, joint-war awards and the
/// remaining combat aftermath must still run before an action can complete.
#[must_use]
pub struct RequiresExperience {
    pub members: GeneratedMembers,
}
#[allow(clippy::too_many_arguments)]
fn change_damage<E>(
    state: &mut LoadedState,
    sight: &mut sight::Runtime,
    effects: &mut E,
    definitions: &placement::Definitions<'_, '_>,
    queries: &placement::Queries<'_>,
    unit: UnitReference,
    delta: i32,
    killed_by: i32,
) -> Result<(), Error<EffectsError<E>>>
where
    E: placement::Effects
        + unit_kill::Effects
        + loaded_upkeep::FormationRestoreEffects<Error = EffectsError<E>>,
{
    placement::change_damage(
        state,
        definitions,
        queries,
        sight,
        effects,
        (unit.player(), unit.id),
        delta,
    )
    .map_err(Error::Damage)?;
    // Report subscribers may have changed damage, max HP, placement or queues.
    let damage = super::actor(state, unit).map_err(Error::Actor)?.i_damage;
    let maximum = definitions
        .movement
        .maximum_damage(super::actor(state, unit).map_err(Error::Actor)?)
        .map_err(Error::Maximum)?;
    if damage >= maximum {
        unit_kill::kill(
            state,
            sight,
            effects,
            definitions,
            queries,
            (unit.player(), unit.id),
            true,
            killed_by,
        )
        .map_err(|e| Error::Kill(Box::new(e)))?;
    }
    Ok(())
}
/// ResolveMeleeUnitVsUnitCombat28d64ec: defender damage/kill precedes attacker
/// damage/kill. Both retained Units remain available for subsequent XP queries.
pub fn apply_damage<E>(
    state: &mut LoadedState,
    sight: &mut sight::Runtime,
    effects: &mut E,
    definitions: &placement::Definitions<'_, '_>,
    queries: &placement::Queries<'_>,
    pending: RequiresResolve,
) -> Result<RequiresExperience, Error<EffectsError<E>>>
where
    E: placement::Effects
        + unit_kill::Effects
        + loaded_upkeep::FormationRestoreEffects<Error = EffectsError<E>>,
{
    let members = pending.members;
    let d = &members.damage;
    change_damage(
        state,
        sight,
        effects,
        definitions,
        queries,
        d.defender,
        d.defender_received,
        d.attacker.player(),
    )?;
    change_damage(
        state,
        sight,
        effects,
        definitions,
        queries,
        d.attacker,
        d.attacker_received,
        d.defender.player(),
    )?;
    Ok(RequiresExperience { members })
}
