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
