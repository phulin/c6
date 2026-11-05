//! Current declaration callbacks borrow the canonical diplomatic owners.
use super::{Error, LoadedState, MissingInput};
impl LoadedState {
    pub fn war_diplomacy_mut(
        &mut self,
        owner: i32,
    ) -> Result<&mut crate::player_diplomacy::Diplomacy, Error> {
        self.world.player(owner).map_err(Error::World)?;
        let index = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        Ok(&mut self.retained.players[index].diplomacy)
    }
    pub fn war_relations_mut(
        &mut self,
    ) -> Result<&mut crate::game_diplomatic_relations::GameDiplomaticRelations, Error> {
        if self.game_relations.is_none() {
            return Err(Error::Missing(MissingInput::GameRelations));
        }
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        Ok(self.game_relations.as_mut().expect("checked relations"))
    }
}
