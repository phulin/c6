//! Canonical revision-tracked component writes used by turn activation.
use crate::loaded_state::{Error, LoadedState, MissingInput};
impl LoadedState {
    fn maintenance_player(&mut self, owner: i32) -> Result<usize, Error> {
        self.world.player(owner).map_err(Error::World)?;
        let index = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        Ok(index)
    }
    pub fn player_stats_mut(
        &mut self,
        owner: i32,
    ) -> Result<&mut crate::player_stats::Stats, Error> {
        let i = self.maintenance_player(owner)?;
        Ok(&mut self.retained.players[i].stats)
    }
    pub fn player_eras_mut(&mut self, owner: i32) -> Result<&mut crate::player_eras::Eras, Error> {
        let i = self.maintenance_player(owner)?;
        Ok(&mut self.retained.players[i].eras)
    }
    pub fn player_unit_maintenance_mut(
        &mut self,
        owner: i32,
    ) -> Result<&mut crate::loaded_components::RetainedUnits, Error> {
        let i = self.maintenance_player(owner)?;
        Ok(&mut self.retained.players[i].units)
    }
    pub fn player_ai_military_mut(
        &mut self,
        owner: i32,
    ) -> Result<&mut Option<crate::player_ai_military::Military>, Error> {
        let i = self.maintenance_player(owner)?;
        Ok(&mut self.retained.players[i].ai_military)
    }
}
