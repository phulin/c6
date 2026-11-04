//! Exclusive access to canonical retained Cities for live creation effects.
use super::{Error, LoadedState, MissingInput};

impl LoadedState {
    /// Resolve full current identity before reserving a mutation revision.
    /// Callers keep any partially applied transition explicit on later failure.
    pub fn city_mut(&mut self, owner: i32, id: u32) -> Result<&mut crate::city::City, Error> {
        self.city(owner, id)?;
        let player = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(Error::Missing(MissingInput::City { owner, id }))?;
        let city = self.retained.players[player]
            .cities
            .records
            .entries
            .iter()
            .position(|entry| entry.value.header.owner == owner && entry.value.header.id == id)
            .ok_or(Error::Missing(MissingInput::City { owner, id }))?;
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        Ok(&mut self.retained.players[player].cities.records.entries[city].value)
    }
}
