//! Revision-tracked canonical Player.Techs and Player.Culture access.
use super::*;
impl LoadedState {
    pub fn research_components(
        &self,
        owner: i32,
    ) -> Result<(&crate::player_techs::Techs, &crate::player_culture::Culture), Error> {
        self.world.player(owner).map_err(Error::World)?;
        let player = self
            .retained
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        Ok((&player.techs, &player.culture))
    }
    /// Reserve the revision only after validating the owner. These are the
    /// canonical components, shared by current queries and subsequent effects.
    pub fn research_components_mut(
        &mut self,
        owner: i32,
    ) -> Result<
        (
            &mut crate::player_techs::Techs,
            &mut crate::player_culture::Culture,
        ),
        Error,
    > {
        self.world.player(owner).map_err(Error::World)?;
        let index = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        let next = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        self.revision = next;
        let player = &mut self.retained.players[index];
        Ok((&mut player.techs, &mut player.culture))
    }
}
