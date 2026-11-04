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

impl LoadedState {
    /// Install the source-created canonical City and factory initialization
    /// prefix. The returned City still requires signal/CityBuild continuation.
    /// Failures after slot allocation retain the explicit incomplete prefix.
    pub fn begin_city_creation(
        &mut self,
        definitions: &crate::city_initialization::Definitions<'_>,
        owner: i32,
        parameters: crate::city_initialization::Parameters,
    ) -> Result<
        Option<crate::unit::ComponentId>,
        crate::city_initialization::CreationError<std::convert::Infallible>,
    > {
        use crate::city_initialization::{self as initialization, CreationError};
        self.world
            .player(owner)
            .map_err(|error| CreationError::State(Error::World(error)))?;
        let player = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(CreationError::State(Error::Binding))?;
        let next = self
            .revision
            .checked_add(1)
            .ok_or(CreationError::State(Error::ResourceLimit))?;
        let Some(allocation) =
            initialization::allocate(&mut self.retained.players[player].cities.records)
                .map_err(CreationError::Initialization)?
        else {
            return Ok(None);
        };
        self.revision = next;
        let value =
            initialization::constructor::construct(definitions, owner, allocation.id, parameters)
                .map_err(CreationError::Initialization)?;
        allocation
            .insert(&mut self.retained.players[player].cities.records, value)
            .map_err(CreationError::Initialization)?;
        let cities = &mut self.retained.players[player].cities;
        cities.peak_cities = cities.peak_cities.max(cities.records.entries.len() as u32);
        Ok(Some(crate::unit::ComponentId {
            owner: owner as i16,
            component_type: 2,
            id: allocation.id,
        }))
    }
}
