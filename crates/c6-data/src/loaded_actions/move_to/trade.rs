//! The stock MakeTradeRoute CheckComplete callback (Linux XP2 0x2da0c30).
//! Update/movement and the reached-at-origin expiry continuation remain explicit
//! gaps; a stationary trader away from its origin requires neither operation.
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
    Err(Error::Unsupported(
        "MakeTradeRoute earliest end turn and route expiry",
    ))
}
