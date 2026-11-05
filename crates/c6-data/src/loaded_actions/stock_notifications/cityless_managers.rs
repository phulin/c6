//! Same stock owner for the reached activation manager effects.
use super::Runtime;
use crate::game_update_state::activation::{cityless_managers as data, maintenance};
use c6_core::{game_update::Error, loaded_state::LoadedState};
impl<P> data::Effects for Runtime<'_, P> {
    fn allocate_luxuries(
        &mut self,
        state: &mut LoadedState,
        definitions: &crate::luxury_allocation::Definitions,
        owner: i32,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("luxury allocation binding"))?;
        if state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Unsupported("luxury allocation player"))?
            .cities
            .records
            .entries
            .is_empty()
        {
            state
                .player_resources_mut(owner)
                .map_err(|_| Error::Unsupported("luxury resource owner"))?
                .luxury_allocations
                .clear();
            return Ok(());
        }
        let cities = self
            .city_query_inputs()
            .ok_or(Error::Unsupported("luxury allocation current city queries"))?;
        crate::luxury_allocation::allocate(
            state,
            definitions,
            owner,
            cities.definitions,
            cities.current.capabilities,
        )
        .map_err(|e| {
            self.last_error = Some(e.to_string());
            Error::Unsupported("luxury allocation")
        })?;
        // 3f5e7f8d refreshes resource DTOs/cache only; no gameplay or RNG.
        Ok(())
    }
    fn congress_event(
        &mut self,
        state: &mut LoadedState,
        _owner: i32,
        _kind: u32,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("Congress report binding"))?;
        Err(Error::Unsupported("Congress phase report"))
    }
    fn report_block(
        &mut self,
        state: &mut LoadedState,
        hash: u32,
        _begin: bool,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("activation resource report binding"))?;
        maintenance::stock_fuel_report_block(hash).map_err(Error::Unsupported)
    }
    fn diplomacy_tracker_changed(
        &mut self,
        state: &mut LoadedState,
        _owner: i32,
        _other: i32,
        _key: u32,
        _old: i32,
        _new: i32,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("diplomacy tracker report binding"))?;
        Err(Error::Unsupported("changed diplomacy tracker logging"))
    }
    fn congress_points_changed(
        &mut self,
        state: &mut LoadedState,
        _owner: i32,
        _delta: i32,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("Congress points report binding"))?;
        Err(Error::Unsupported("Congress points changed notification"))
    }
    fn notification(
        &mut self,
        state: &mut LoadedState,
        _owner: i32,
        _kind: u32,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("activation notification binding"))?;
        Err(Error::Unsupported(
            "activation manager positive notification",
        ))
    }
}
