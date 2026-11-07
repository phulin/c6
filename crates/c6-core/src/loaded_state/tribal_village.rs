//! Canonical Player.Goody_Hut reward-history mutation.
use super::{Error, LoadedState, MissingInput};
impl LoadedState {
    pub fn player_goody_hut_mut(
        &mut self,
        owner: i32,
    ) -> Result<&mut crate::player_components::GoodyHut, Error> {
        self.world.player(owner).map_err(Error::World)?;
        let index = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        Ok(&mut self.retained.players[index].goody_hut)
    }
}
