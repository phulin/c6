//! The stock MakeTradeRoute CheckComplete callback (Linux XP2 0x2da0c30).
//! The same-turn origin check cannot expire a newly started route: native
//! GetEarliestEndTurnForRoute floors its result at push_turn+1. Later expiry
//! and route-end effects remain explicit when not composed.
use super::*;

pub(super) fn check_complete<E: Effects, M: Movement<E>>(
    state: &LoadedState,
    actor: (i32, u32),
    node: &OperationNode,
    data: &mut Processing,
    movement: &M,
) -> Result<(), Error<EffectError<E>, M::Error>> {
    let OperationParameters::MakeTradeRoute {
        location0,
        location1,
        ..
    } = node.parameters
    else {
        return Err(Error::Unsupported("MakeTradeRoute parameters"));
    };
    if !c6_core::skip_turn::placement::coordinate_valid(location1) {
        data.delete = true;
        return Ok(());
    }
    let origin = movement
        .city_at(state, location1)
        .map_err(Error::Movement)?;
    // Native queries location0 even when the origin lookup returned null.
    let destination = movement
        .city_at(state, location0)
        .map_err(Error::Movement)?;
    let (Some(origin), Some(destination)) = (origin, destination) else {
        data.delete = true;
        return Ok(());
    };
    let trader = identity(state, actor)?;
    if !state
        .trade_graph()
        .map_err(Error::State)?
        .has_outgoing_route(
            origin,
            destination,
            c6_core::trade::ComponentId {
                owner: trader.owner,
                component_type: trader.component_type,
                id: trader.id,
            },
        )
    {
        data.delete = true;
        return Ok(());
    }
    if unit(state, actor)?.k_location != location1 {
        return Ok(());
    }
    // GetEarliestEndTurnForRoute2c0be40 returns either a value strictly
    // greater than push_turn or wrapping(push_turn+1). Its source only reads
    // rules/speed/current-era definitions and performs integer arithmetic.
    // Thus ordinary same-turn Start cannot reach ExpireRoute. MAX is kept
    // explicit because the fallback wraps rather than satisfying the floor.
    if node.push_turn == state.retained().core.current_turn && node.push_turn != i32::MAX {
        return Ok(());
    }
    let earliest = movement
        .trade_earliest_end_turn(state, node.push_turn)
        .map_err(Error::Movement)?
        .ok_or(Error::Unsupported(
            "MakeTradeRoute current earliest end turn",
        ))?;
    if state.retained().core.current_turn < earliest {
        return Ok(());
    }
    Err(Error::Unsupported("MakeTradeRoute route expiry effects"))
}

/// MakeTradeRoute.Update2da0ae0. Native ignores ProcessingData: the movement
/// continuation changes the actual Formation/Units but sets no moved/delete bit.
pub(super) fn update<E: Effects, M: Movement<E>>(
    state: &mut LoadedState,
    actor: (i32, u32),
    node: &OperationNode,
    sight: &mut unit_sight_state::Runtime,
    effects: &mut E,
    movement: &mut M,
) -> Result<(), Error<EffectError<E>, M::Error>> {
    let OperationParameters::MakeTradeRoute {
        location0,
        location1,
        ..
    } = node.parameters
    else {
        return Err(Error::Unsupported("MakeTradeRoute parameters"));
    };
    if !skip_turn::placement::coordinate_valid(location1) {
        return Ok(());
    }
    let Some(origin) = movement
        .city_at(state, location1)
        .map_err(Error::Movement)?
    else {
        return Ok(());
    };
    let Some(destination) = movement
        .city_at(state, location0)
        .map_err(Error::Movement)?
    else {
        return Ok(());
    };
    let Some(formation) = state
        .unit_runtime(actor.0, actor.1)
        .map_err(Error::State)?
        .formation
    else {
        return Ok(());
    };
    let Some(mut edge) = state
        .trade_graph_mut()
        .map_err(Error::State)?
        .edit_city_edge(origin, destination)
    else {
        return Ok(());
    };
    let next = edge
        .advance_path_to_next_plot()
        .map_err(|_| Error::Unsupported("Trade path cursor outside vector"))?;
    let map = state.world().map();
    if next < 0 || next as usize >= map.plots.len() || map.header.width <= 0 {
        return Ok(());
    }
    let at = PlotCoord {
        x: next % map.header.width,
        y: next / map.header.width,
    };
    if !skip_turn::placement::coordinate_valid(at) {
        return Ok(());
    }
    movement
        .trade_move(
            state,
            sight,
            (formation.owner, formation.requested_id),
            at,
            effects,
        )
        .map_err(Error::Movement)?
        .ok_or(Error::Unsupported("Trade.Movement.MoveTo composition"))
}
