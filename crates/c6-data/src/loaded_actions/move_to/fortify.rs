//! Ordinary Fortify's parameterless stock handler, over the existing queue.
use super::*;

pub(super) fn can_start<E: Effects, M: Movement<E>>(
    state: &LoadedState, unit: &Unit, inputs: &Inputs<'_, '_>, movement: &M,
) -> Result<bool, super::super::Error> {
    use super::super::Error as Q;
    let actor = (unit.e_owner, unit.id);
    if state.unit_runtime(actor.0, actor.1).map_err(Q::State)?.formation.is_none() {
        return Ok(false);
    }
    if state.player_slot_status(actor.0).map_err(Q::State)? == 3 && unit.i_fortify_turns == 0 {
        let count = friendly_count::<EffectError<E>, M::Error>(state, actor, inputs)
            .map_err(|_| Q::Unsupported("Fortify friendly-unit count"))?;
        let limit = inputs.plot_unit_limit.ok_or(Q::Unsupported("Fortify plot unit limit"))?;
        if count > limit as u32 { return Ok(false); }
    }
    if !movement.is_fortifyable(state, actor)
        .map_err(|_| Q::Unsupported("Fortify IsFortifyable dependency"))? {
        return Ok(false);
    }
    Ok(!super::super::fortify::is_waiting(unit.e_activity) && !unit.b_embarked)
}
pub(super) fn start<E: Effects, M>(
    state: &mut LoadedState, actor: (i32, u32), effects: &mut E,
) -> Result<(), Error<EffectError<E>, M>> {
    let report = IdentityReport { event: 0x3d4006f2, subtype: c6_core::unit::FORTIFY_HASH,
        identity: identity(state, actor)?, first: Some(0), second: None };
    let handle = UnitHandle::resolve(state, actor.0, actor.1).map_err(Error::World)?
        .ok_or(Error::Unsupported("Fortify report actor"))?;
    effects.preflight_operation_report(state, handle, report).map_err(Error::Effect)?;
    effects.operation_report(state, report).map_err(Error::Effect)?;
    Ok(())
}
pub(super) fn update<E, M>(state: &mut LoadedState, actor: (i32, u32),
    data: &mut Processing, inputs: &Inputs<'_, '_>) -> Result<(), Error<E, M>> {
    let moved = crate::unit_movement_state::unit_has_moved(state, unit(state, actor)?,
        inputs.movement.ok_or(Error::Unsupported("Fortify effective HasMoved inputs"))?)
        .map_err(|e| Error::Upkeep(loaded_upkeep::Error::Movement(e)))?;
    if !moved {
        let turns = super::super::fortify::increment_turns(unit(state, actor)?.i_fortify_turns);
        state.unit_mut(actor.0, actor.1).map_err(Error::State)?.i_fortify_turns = turns;
    }
    data.delete = false;
    Ok(())
}
pub(super) fn check_complete<E: Effects, M: Movement<E>>(state: &mut LoadedState,
    actor: (i32, u32), data: &mut Processing, effects: &mut E, movement: &M,
) -> Result<(), Error<EffectError<E>, M::Error>> {
    let max = movement.fortify_turn_max();
    if unit(state, actor)?.i_fortify_turns < max {
        data.delete = false;
        data.stop = true;
    } else {
        state.unit_mut(actor.0, actor.1).map_err(Error::State)?.i_fortify_turns = max.max(0);
        data.delete = true;
        loaded_upkeep::set_unit_activity(state, actor.0, actor.1, effects, 0xd0aed661u32 as i32)
            .map_err(Error::Upkeep)?;
    }
    Ok(())
}
