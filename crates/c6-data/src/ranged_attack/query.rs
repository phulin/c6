//! Original explicit-target ranged readiness over current canonical inputs.
use super::{error, Error};
use crate::{loaded_actions as actions, melee_attack as melee, unit_sight_state as sight};
use c6_core::{loaded_state::LoadedState, map::UnitReference, unit::PlotCoord};
pub fn test(
    state: &LoadedState,
    definitions: &melee::target_selection::Definitions,
    inputs: &melee::target_selection::Inputs<'_>,
    actor: UnitReference,
    destination: PlotCoord,
) -> Result<bool, Error> {
    let a = melee::actor(state, actor).map_err(|e| error("test", e))?;
    if a.i_attacks_remaining <= 0 {
        return Ok(false);
    }
    let ranged = inputs
        .combat
        .ranged(state, a)
        .map_err(|e| error("test", e))?;
    let bombard = inputs
        .combat
        .bombard(state, a)
        .map_err(|e| error("test", e))?;
    if ranged <= bombard {
        return Err(error("test", "equal/bombard combat type unsupported"));
    }
    if melee::target_selection::enemy_defensible_district_with_definitions(
        state,
        inputs.districts,
        a.e_owner,
        destination,
    )
    .map_err(|e| error("test", e))?
    .is_some()
    {
        return Err(error("test", "District target unsupported"));
    }
    let Some(defender) =
        melee::target_selection::ranged_defender(state, inputs, actor, destination)
            .map_err(|e| error("test", e))?
    else {
        return Ok(false);
    };
    melee::target_selection::can_attack_ranged(state, definitions, inputs, actor, defender)
        .map_err(|e| error("test", e))
}
/// Original CanStart for an explicit ordinary target. Target enumeration and
/// fresh raycasting beyond the retained current sight list remain explicit.
#[allow(clippy::too_many_arguments)]
pub fn can_start(
    state: &LoadedState,
    definitions: &melee::target_selection::Definitions,
    inputs: &melee::target_selection::Inputs<'_>,
    operations: &actions::Inputs<'_, '_>,
    sight: &sight::Runtime,
    actor: UnitReference,
    destination: PlotCoord,
    visible_only: bool,
) -> Result<bool, Error> {
    let a = melee::actor(state, actor).map_err(|e| error("can start", e))?;
    let runtime = state
        .unit_runtime(a.e_owner, a.id)
        .map_err(|e| error("can start", e))?;
    if runtime.formation.is_none() {
        return Ok(false);
    }
    if !melee::target_selection::ranged_attack_after_moving(state, definitions, inputs, actor)
        .map_err(|e| error("can start", e))?
        && crate::unit_movement_state::unit_has_moved(
            state,
            a,
            &crate::unit_movement_state::Inputs {
                definitions: inputs.movement,
            },
        )
        .map_err(|e| error("can start", e))?
    {
        return Ok(false);
    }
    if crate::unit_movement_state::unit_is_embarked(a)
        && !a.k_combat_modifers.b_fight_while_embarked
    {
        return Ok(false);
    }
    if runtime.domain == 1 {
        return Ok(false);
    }
    if inputs
        .combat
        .ranged(state, a)
        .map_err(|e| error("can start", e))?
        <= 0
    {
        return Ok(false);
    }
    if visible_only {
        return Ok(true);
    }
    let friendly = actions::move_to::friendly_count::<
        std::convert::Infallible,
        std::convert::Infallible,
    >(state, (actor.player(), actor.id), operations)
    .map_err(|e| error("can start", e))?;
    if friendly
        > operations
            .plot_unit_limit
            .ok_or_else(|| error("can start", "plot unit limit"))? as u32
    {
        return Ok(false);
    }
    let range = a.k_attack_range_modifiers.i_attack_range;
    if !c6_core::geometry::plot_valid(destination.x, destination.y) {
        return Err(error("can start", "target enumeration unsupported"));
    }
    let distance = actions::city_range_attack::plot_distance(
        state,
        (a.k_location.x, a.k_location.y),
        (destination.x, destination.y),
    );
    if distance > range {
        return Ok(false);
    }
    let (index, _) = c6_core::movement_world::plot_at(state.world().map(), destination)
        .map_err(|e| error("can start", e))?;
    if !crate::formation_path_state::shared_plot_visible(sight, a.e_owner, index)
        .map_err(|e| error("can start", e))?
    {
        return Ok(false);
    }
    if range <= 2 {
        if a.k_sight_modifiers.i_sight_range < range {
            return Err(error("can start", "fresh CanSeePlot raycast unsupported"));
        }
        if !a.k_sight_plots.iter().any(|p| p.ui_index == index as u32) {
            return Ok(false);
        }
    }
    test(state, definitions, inputs, actor, destination)
}
