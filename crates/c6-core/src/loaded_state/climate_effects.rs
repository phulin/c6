//! Canonical player-name mutation reached by Linux Climate.GetStormName.
use super::{Error, LoadedState, MissingInput};
impl LoadedState {
    pub fn player_citizen_name_usage_mut(&mut self, owner: i32) -> Result<&mut [i16], Error> {
        let index = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        Ok(&mut self.retained.players[index]
            .name_manager
            .citizen_name_usage_counts)
    }
}
