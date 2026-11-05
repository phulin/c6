//! Ordinary human Player.Update invoked synchronously by a player command.
//! This does not advance the enclosing Game.Update player cursor.
use super::{core, Error, LoadedState, PlayerState};

/// 2d21c9e -> 2b5c720. Reuse the normal scalar body after confirming that
/// activation already completed at this ordinary action site.
pub fn prepare_human(state: &mut LoadedState, owner: i32) -> Result<(), Error> {
    let index = usize::try_from(owner)
        .ok()
        .filter(|i| *i < 64)
        .ok_or(Error::Unsupported("command Player.Update owner"))?;
    if !state.turn_active(owner).map_err(super::missing)?
        || state.player_slot_status(owner).map_err(super::missing)? != 3
        || state.retained.core.player_processor.activation_event[index]
    {
        return Err(Error::Unsupported(
            "command requires ordinary active human Player.Update",
        ));
    }
    match core::prepare_player(&mut PlayerState { state }, owner)? {
        core::PlayerEffect::Blocking { player } if player == owner => Ok(()),
        _ => Err(Error::Unsupported(
            "command Player.Update unexpected continuation",
        )),
    }
}
