//! The ranged resolver changes only defender HP. Death uses the same canonical
//! Unit.Manager ChangeDamage/Kill lifecycle and retained callback owners.
use crate::{
    loaded_upkeep,
    melee_attack::{
        execution::RequiresResolve,
        resolve::{Error, RequiresExperience},
    },
    unit_kill, unit_placement_state as placement, unit_sight_state as sight,
};
use c6_core::loaded_state::LoadedState;
type EffectsError<P> = <P as sight::Effects>::Error;
#[allow(clippy::too_many_arguments)]
pub fn apply_with_continuations<P, C>(
    state: &mut LoadedState,
    sight: &mut sight::Runtime,
    providers: &mut P,
    continuations: &mut C,
    definitions: &placement::Definitions<'_, '_>,
    queries: &placement::Queries<'_>,
    pending: RequiresResolve,
) -> Result<RequiresExperience, Error<EffectsError<P>, C::Error>>
where
    P: placement::Effects + loaded_upkeep::FormationRestoreEffects<Error = EffectsError<P>>,
    C: unit_kill::Continuations<P>,
{
    let members = pending.members;
    let attacker = members.damage.attacker;
    let defender = members.damage.defender;
    super::melee::actor(state, attacker).map_err(Error::Actor)?;
    let current = super::melee::actor(state, defender).map_err(Error::Actor)?;
    let location = if state
        .world()
        .map()
        .plot_at(members.location.x, members.location.y)
        .is_some()
    {
        members.location
    } else {
        current.k_location
    };
    if state
        .world()
        .map()
        .plot_at(location.x, location.y)
        .is_none()
    {
        return Err(Error::Missing("ranged Resolve current combat plot"));
    }
    placement::change_damage(
        state,
        definitions,
        queries,
        sight,
        providers,
        (defender.player(), defender.id),
        members.damage.defender_received,
    )
    .map_err(Error::Damage)?;
    let current = super::melee::actor(state, defender).map_err(Error::Actor)?;
    let maximum = definitions
        .movement
        .maximum_damage(current)
        .map_err(Error::Maximum)?;
    if current.i_damage >= maximum {
        unit_kill::kill_with_continuations(
            state,
            sight,
            providers,
            continuations,
            definitions,
            queries,
            (defender.player(), defender.id),
            true,
            attacker.player(),
        )
        .map_err(|e| Error::Kill(Box::new(e)))?;
    }
    Ok(RequiresExperience { members })
}
