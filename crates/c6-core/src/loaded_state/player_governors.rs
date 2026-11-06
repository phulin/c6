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

impl LoadedState {
    /// Bind the actual unsaved native maximum once. It cannot be reconstructed
    /// from the Governors saved record after titles have been spent.
    pub fn bind_governor_current(
        &mut self,
        owner: i32,
        current: crate::governor_lifecycle::Current,
    ) -> Result<(), Error> {
        self.world.player(owner).map_err(Error::World)?;
        if self.governor_current.iter().any(|r| r.0 == owner) {
            return Err(Error::RuntimeIdentity);
        }
        self.governor_current.push((owner, current));
        Ok(())
    }
    pub fn governor_current(&self, owner: i32) -> Option<&crate::governor_lifecycle::Current> {
        self.governor_current
            .iter()
            .find(|r| r.0 == owner)
            .map(|r| &r.1)
    }
    pub fn governors_mut(
        &mut self,
        owner: i32,
    ) -> Result<&mut crate::player_governors::Governors, Error> {
        self.world.player(owner).map_err(Error::World)?;
        let index = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        Ok(&mut self.retained.players[index].governors)
    }
    pub fn change_governor_points(
        &mut self,
        owner: i32,
        major: bool,
        delta: i32,
    ) -> Result<(), Error> {
        if !major {
            return Ok(());
        }
        let index = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        let current = self
            .governor_current
            .iter_mut()
            .find(|r| r.0 == owner)
            .ok_or(Error::RuntimeIdentity)?;
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        crate::governor_lifecycle::change_points(
            &mut self.retained.players[index].governors,
            &mut current.1,
            major,
            delta,
        );
        Ok(())
    }
}
