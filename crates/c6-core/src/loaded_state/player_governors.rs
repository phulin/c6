//! Canonical current Governor messaging state.
use super::{Error, LoadedState, MissingInput};
impl LoadedState {
    /// Governors.SetTitleConsidered. Appoint/promote policies remain external.
    pub fn set_governor_title_considered(&mut self, owner: i32, value: bool) -> Result<(), Error> {
        self.world.player(owner).map_err(Error::World)?;
        let index = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        self.retained.players[index].governors.title_considered = value;
        Ok(())
    }
}
