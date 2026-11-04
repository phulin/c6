//! Exclusive mutation of the canonical retained Player influence map.
use super::{Error, LoadedState, MissingInput};
impl LoadedState {
    pub fn influence_map_mut(
        &mut self,
        owner: i32,
    ) -> Result<&mut crate::player_ai_influence_map::InfluenceMap, Error> {
        self.world.player(owner).map_err(Error::World)?;
        let index = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(Error::Binding)?;
        if self.retained.players[index].ai_influence_map.is_none() {
            return Err(Error::Missing(MissingInput::PlayerInfluenceMap(owner)));
        }
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        Ok(self.retained.players[index]
            .ai_influence_map
            .as_mut()
            .unwrap())
    }
}
