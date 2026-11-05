//! Current Player.Stats counters used by combat boost consumers.
use super::{Error, LoadedState, MissingInput};
impl LoadedState {
    /// One native wrapping increment, in the same retained Player record.
    pub fn change_barbarians_killed(&mut self, owner: i32, delta: i32) -> Result<i32, Error> {
        let player = self
            .retained
            .players
            .iter_mut()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        let next = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        player.stats.barbarians_killed = player.stats.barbarians_killed.wrapping_add(delta);
        self.revision = next;
        Ok(player.stats.barbarians_killed)
    }
}
