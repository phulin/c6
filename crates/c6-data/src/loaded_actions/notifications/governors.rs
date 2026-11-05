//! Current Linux XP2 GovernorAppointment notification query.
//! This reads gameplay eligibility only; appointment/promotion policy stays external.
use super::processing::Error;
use c6_core::{loaded_state::LoadedState, player_governors::Governors};

pub const TYPE_HASH: u32 = 0x77af5297;
pub const VTABLE_ADDRESSPOINT: u64 = 0x3af5f00;

/// GetGovernorPointsSpent2b0a240 sums GetNumTitles2a36de0, which counts
/// earned promotion flags (including the appointment title), with native i32
/// wrapping arithmetic. It does not count governor objects separately.
pub fn points_spent(governors: &Governors) -> i32 {
    governors
        .entries
        .iter()
        .flat_map(|entry| &entry.governor.promotions)
        .fold(0i32, |sum, promotion| {
            sum.wrapping_add(i32::from(promotion.earned))
        })
}

/// GovernorAppointment::IsExpired2d2cee0 reads current human status first,
/// then the player's current points/spent totals and title-considered flag.
/// CanAppoint and CanPromote are not consumers of this virtual query.
pub fn is_expired(state: &LoadedState, owner: i32) -> Result<bool, Error> {
    if state.player_slot_status(owner)? != 3 {
        return Ok(true);
    }
    let governors = &state
        .retained()
        .players
        .iter()
        .find(|player| player.header.owner == owner)
        .ok_or(Error::Unsupported("current GovernorAppointment owner"))?
        .governors;
    Ok(governors.governor_points <= points_spent(governors) || governors.title_considered)
}
