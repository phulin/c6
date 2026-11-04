//! Scoped edits of canonical activation manager records.
use super::{Error, LoadedState, MissingInput};
pub struct ActivationManagersMut<'a> {
    pub diplomacy: &'a mut crate::player_diplomacy::Diplomacy,
    pub congress: &'a mut crate::player_components::Congress,
    pub espionage: &'a mut crate::player_espionage::Espionage,
    pub trade: &'a mut crate::player_trade::Trade,
}
impl LoadedState {
    pub fn activation_managers_mut(
        &mut self,
        owner: i32,
    ) -> Result<ActivationManagersMut<'_>, Error> {
        self.world.player(owner).map_err(Error::World)?;
        let index = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        let p = &mut self.retained.players[index];
        Ok(ActivationManagersMut {
            diplomacy: &mut p.diplomacy,
            congress: &mut p.congress,
            espionage: &mut p.espionage,
            trade: &mut p.trade,
        })
    }
}
