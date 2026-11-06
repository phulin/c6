//! Trade.OnPlayersMet and its positive capacity mutation.
use super::Error;
use c6_core::loaded_state::LoadedState;
pub const CAPACITY_CHANGED: u32 = 0xa5423bfe;
/// The required report follows the canonical wrapping capacity write.
pub fn on_met<E: std::fmt::Debug>(
    state: &mut LoadedState,
    receiver: i32,
    first: i32,
    second: i32,
    levels: &[(i32, u32)],
    mut report: impl FnMut(&mut LoadedState, u32, i32) -> Result<(), E>,
) -> Result<(), Error> {
    for owner in [first, second] {
        if levels
            .iter()
            .find(|p| p.0 == owner)
            .ok_or(Error::Missing("first-contact trade civilization level"))?
            .1
            != c6_core::diplomacy_meeting::MAJOR_LEVEL
        {
            return Ok(());
        }
    }
    if receiver != first && receiver != second {
        return Ok(());
    }
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == receiver)
        .ok_or(Error::Missing("first-contact Trade parent"))?;
    if player.trade.trade_route_capacity_increase_on_meeting <= 0 {
        return Ok(());
    }
    let amount = player.trade.trade_route_capacity_increase_on_meeting;
    let trade = state
        .activation_managers_mut(receiver)
        .map_err(Error::State)?
        .trade;
    trade.outgoing_route_capacity = trade.outgoing_route_capacity.wrapping_add(amount);
    report(state, CAPACITY_CHANGED, receiver)
        .map_err(|e| Error::Requirement(format!("first-contact capacity report: {e:?}")))
}
