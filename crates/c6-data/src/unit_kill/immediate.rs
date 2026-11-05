//! Player.Units.Destroy2ba5310 for an already-delayed ordinary singleton.
//! Uses the same placement, sight, object/effect and canonical roster owners.
//! Non-delayed destruction and surviving multi-member formations remain explicit.
use super::{objects, EffectError, Error, Providers};
use crate::{loaded_actions::city_range_attack, unit_sight_state as sight};
use c6_core::{loaded_state::LoadedState, map::UnitReference};

/// The current notification domain must prove absence before the stock
/// FindType(...PROMOTE_UNIT) null branch is taken. Native specialized payloads
/// are not reconstructed from a Unit's experience or from an absent import.
fn promotion_absent<E, P>(state: &LoadedState, owner: i32) -> Result<(), Error<E, P>> {
    let collections = state.player_notifications().map_err(Error::State)?;
    let player = collections
        .collection(owner)
        .map_err(|_| Error::Missing("current promotion notification owner"))?;
    if player.records.iter().any(|n| n.type_hash == 0x2ba6e111) {
        return Err(Error::Missing(
            "current PromoteUnit.RemovePromotableUnit payload",
        ));
    }
    Ok(())
}
/// Source branch used by Player.Units.DoEndTurn's gathered delayed list.
/// All UnitRemoved consumers must already have run; they are never dispatched
/// twice here. Destruction releases the normal object only after real cleanup.
#[allow(clippy::too_many_arguments)]
pub fn destroy_delayed<P, C>(
    state: &mut LoadedState,
    sight: &mut sight::Runtime,
    providers: &mut P,
    continuations: &mut C,
    definitions: &city_range_attack::Definitions,
    actor: (i32, u32),
) -> Result<(), Error<C::Error, EffectError<P>>>
where
    P: Providers,
    C: objects::Services<P>,
{
    let unit = super::unit(state, actor)?;
    if unit.id != actor.1 {
        return Err(Error::Missing("current destruction identity"));
    }
    if !unit.b_delayed_death {
        return Err(Error::Missing("non-delayed Destroy UnitRemoved branch"));
    }
    super::remove_placement_inner(state, sight, providers, definitions, actor)?;
    let formation = state
        .unit_runtime(actor.0, actor.1)
        .map_err(Error::State)?
        .formation;
    if let Some(formation) = formation {
        state
            .detach_destroyed_singleton(actor.0, actor.1, formation.resolved_id)
            .map_err(Error::State)?;
        // Empty Formation destructor's member loop is empty; remaining writes
        // destroy archived fields/references, with no gameplay/RNG dispatch.
        state
            .release_destroyed_empty_formation(actor.0, formation.resolved_id)
            .map_err(Error::Roster)?;
    }
    if state.player_slot_status(actor.0).map_err(Error::State)? == 3 {
        promotion_absent(state, actor.0)?;
    }
    continuations
        .unit_cleanup_runtime_mut()
        .map_err(Error::CleanupQuery)?
        .release_destroyed_unit_wrapper(
            state,
            UnitReference {
                owner: actor.0,
                id: actor.1,
            },
        )
        .map_err(Error::CleanupQuery)?;
    // Unit destructor validation233dde0 is diagnostic object/reference logging;
    // stock component/archive/owned-vector destruction has no gameplay RNG.
    // The arena retains semantic references, not native weak_ptr/pool storage.
    state
        .release_destroyed_unit_storage(actor.0, actor.1)
        .map_err(Error::Roster)
}
