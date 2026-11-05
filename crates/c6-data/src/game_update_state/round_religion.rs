//! Linux XP2 Game.Religion.DoTurn2a160c0: rotate pantheon eligibility.
//!
//! This world-manager leaf does not spread religion or execute player policy.
//! It selects from the current Player.Manager alive-major order and reads the
//! SAME canonical Player.Religion faith/pantheon fields used by player upkeep.
use crate::unit_placement_services::Runtime;
use c6_core::{diplomacy_meeting::MAJOR_LEVEL, loaded_state::LoadedState};

#[derive(Debug)]
pub enum Error {
    Missing(&'static str),
    State(c6_core::loaded_state::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "round religion: {self:?}")
    }
}
impl std::error::Error for Error {}

fn religion(state: &LoadedState, owner: i32) -> Result<&c6_core::player_religion::Religion, Error> {
    state
        .world()
        .player(owner)
        .map_err(|_| Error::Missing("Religion current Player"))?;
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .map(|p| &p.religion)
        .ok_or(Error::Missing("Religion current Player.Religion"))
}

/// `players` is the current shared Player.Manager owner, initialized from the
/// same BEFORE as `state`. InitializeLists2b51c10 builds GetAliveMajors by
/// filtering GetAlive in physical slot order; no ID sort or saved-roster
/// substitute is used here. No rules lookup, callback or RNG draw is reached.
pub fn run(state: &mut LoadedState, players: &Runtime) -> Result<(), Error> {
    if players.binding != *state.binding() {
        return Err(Error::Missing("Religion Player.Manager binding"));
    }
    let previous = state.retained().religion.next_pantheon_founder;
    // The reset occurs before traversing players, including failure prefixes.
    state.set_next_pantheon_founder(-1).map_err(Error::State)?;
    let (list, count) = players
        .alive_begin::<()>()
        .map_err(|_| Error::Missing("Religion current GetAlive"))?;
    let mut best_faith = 0;
    for index in 0..count {
        let owner = players
            .alive_at::<()>(list, index)
            .map_err(|_| Error::Missing("Religion current GetAlive order"))?;
        let level = players
            .player::<()>(owner)
            .map_err(|_| Error::Missing("Religion current civilization level"))?
            .civilization_level
            .ok_or(Error::Missing("Religion current civilization level"))?;
        if level != MAJOR_LEVEL {
            continue;
        }
        let player = religion(state, owner)?;
        if player.pantheon_hash != u32::MAX || owner == previous {
            continue;
        }
        let faith = player.faith_balance_q8;
        if state.retained().religion.next_pantheon_founder == -1 || faith > best_faith {
            state
                .set_next_pantheon_founder(owner)
                .map_err(Error::State)?;
            best_faith = faith;
        }
    }
    if state.retained().religion.next_pantheon_founder == -1
        && previous != -1
        && religion(state, previous)?.pantheon_hash == u32::MAX
    {
        state
            .set_next_pantheon_founder(previous)
            .map_err(Error::State)?;
    }
    Ok(())
}
