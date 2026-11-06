//! Player.Diplomacy.OnPlayersMet (Linux 0x2aa1980).
use super::Error;
use c6_core::loaded_state::LoadedState;
/// Only the receiver whose parent is the first participant performs the two
/// writes. Each endpoint records its own current era at the other player's ID.
pub fn on_met(
    state: &mut LoadedState,
    eras: &crate::registry::LinuxDefinitionRegistry,
    receiver: i32,
    first: i32,
    second: i32,
) -> Result<(), Error> {
    if receiver != first {
        return Ok(());
    }
    for (owner, other) in [(first, second), (second, first)] {
        let hash = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("first-contact era player"))?
            .eras
            .era_hash;
        let era = if hash == u32::MAX {
            -1
        } else {
            eras.find_index(hash)
                .ok_or(Error::Missing("first-contact current era definition"))? as i32
        };
        let index =
            usize::try_from(other).map_err(|_| Error::Missing("first-contact era target index"))?;
        *state
            .war_diplomacy_mut(owner)
            .map_err(Error::State)?
            .era_player_met
            .get_mut(index)
            .ok_or(Error::Missing("first-contact era target cell"))? = era;
    }
    Ok(())
}
