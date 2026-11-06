//! Canonical Game.Religion mutation used by mandatory world upkeep.
use super::{Error, LoadedState};

impl LoadedState {
    /// Update the retained manager, preserving all religion records and faith
    /// thresholds. Native auto-variable dirty bookkeeping is transport state.
    pub fn set_next_pantheon_founder(&mut self, owner: i32) -> Result<(), Error> {
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        self.retained.religion.next_pantheon_founder = owner;
        Ok(())
    }
}

impl crate::core_player::PlayerHeader {
    /// Linux Player::GetHealingFromDispersal263d330 reads value+248;
    /// Player47 writer2b4ef06 serializes wrapper+238 as ordered scalar 13.
    pub fn healing_from_dispersal(&self) -> i32 {
        self.ordered_scalars[13]
    }
}
impl LoadedState {
    /// Concrete Initiation Rites modifier leaves. Arithmetic wraps exactly as
    /// ChangeFaithFromDispersal and ChangeHealingFromDispersal242c4f0.
    pub fn change_pantheon_dispersal_effect(
        &mut self,
        owner: i32,
        faith: bool,
        amount: i32,
    ) -> Result<(), Error> {
        self.world.player(owner).map_err(Error::World)?;
        let index = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(Error::Missing(super::MissingInput::Player(owner)))?;
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        let p = &mut self.retained.players[index];
        let value = if faith {
            &mut p.religion.faith_from_dispersal
        } else {
            &mut p.header.ordered_scalars[13]
        };
        *value = value.wrapping_add(amount);
        Ok(())
    }
}
