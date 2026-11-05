//! Natural ordinary-building progress on the persistent stock turn runtime.
//!
//! This adapter shares current city queries with upkeep. It supports progress,
//! including ordinary construction-plot initialization, but does not complete
//! buildings or choose the next production order. Errors retain any mutation
//! prefix already performed by the production kernel.
use super::Runtime;
use crate::city_production_turn::{self as production, appeal, buildings};
use c6_core::{
    city_production::{Order, QueueEntry},
    loaded_state::LoadedState,
};

/// Source-qualified stock report consumers only: numeric previews, read-only
/// queries and presentation caches. Added subscribers are outside this scope.
#[derive(Clone, Copy, Debug)]
pub enum Scope {
    StockPlainHeadlessNoAddedConsumers,
}

/// Semantic publications in source order; no native UI queue is reconstructed.
#[derive(Clone, Copy, Debug)]
pub struct Record {
    pub owner: i32,
    pub city: u32,
    pub building: u32,
    pub location: [i32; 2],
    pub report: buildings::Report,
}

pub struct Effects<'runtime, 'inputs, P> {
    runtime: &'runtime mut Runtime<'inputs, P>,
    definitions: &'inputs buildings::Definitions,
    appeal: &'inputs appeal::Definitions,
    pub reports: Vec<Record>,
}
impl<'runtime, 'inputs, P> Effects<'runtime, 'inputs, P> {
    pub fn new(
        runtime: &'runtime mut Runtime<'inputs, P>,
        definitions: &'inputs buildings::Definitions,
        appeal: &'inputs appeal::Definitions,
        _: Scope,
    ) -> Self {
        Self {
            runtime,
            definitions,
            appeal,
            reports: Vec::new(),
        }
    }
    fn check(&self, state: &LoadedState) -> Result<(), production::Error> {
        self.runtime
            .check(state)
            .map_err(|_| production::Error::Binding)?;
        self.definitions.validate(state)
    }
}
fn building(entry: &QueueEntry) -> Result<u32, production::Error> {
    match entry.order {
        Order::Construct { building_hash } => Ok(building_hash),
        _ => Err(production::Error::Unsupported(
            "stock ordinary building production",
        )),
    }
}
impl<P> production::Effects for Effects<'_, '_, P> {
    fn calculate_yield_q8(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        yield_index: i32,
        production_kind: u32,
    ) -> Result<i32, production::Error> {
        self.check(state)?;
        self.runtime
            .current_city_yield(state, owner, city, yield_index, production_kind)
            .map_err(|_| production::Error::Unsupported("stock current production yield query"))
    }
    fn current_build(
        &mut self,
        state: &LoadedState,
        owner: i32,
        city: u32,
        entry: &QueueEntry,
    ) -> Result<production::CurrentBuild, production::Error> {
        self.check(state)?;
        self.definitions
            .building_parameters(state, owner, city, building(entry)?)
    }
    fn start_progress(
        &mut self,
        state: &mut LoadedState,
        _: i32,
        _: u32,
        entry: &QueueEntry,
    ) -> Result<(), production::Error> {
        self.check(state)?;
        self.definitions
            .start_building_progress(state, building(entry)?)
    }
    fn progress_report(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        entry: &QueueEntry,
    ) -> Result<(), production::Error> {
        self.check(state)?;
        buildings::progress_changed(state, owner, city, building(entry)?, self.definitions, self)
    }
    fn finish(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
        _: &QueueEntry,
    ) -> Result<bool, production::Error> {
        Err(production::Error::Unsupported("stock building completion"))
    }
    fn new_front(&mut self, _: &mut LoadedState, _: i32, _: u32) -> Result<(), production::Error> {
        Err(production::Error::Unsupported("stock new production front"))
    }
}
impl<P> buildings::Effects for Effects<'_, '_, P> {
    fn current_era(&mut self, state: &LoadedState) -> Result<i32, production::Error> {
        self.check(state)?;
        state
            .game_eras()
            .map_err(|_| production::Error::Unsupported("current game era owner"))?
            .current_era()
            .ok_or(production::Error::Unsupported("current game era"))
    }
    fn check_active_production(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
    ) -> Result<(), production::Error> {
        Err(production::Error::Unsupported(
            "stock active production recheck",
        ))
    }
    fn set_as_capital(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
    ) -> Result<(), production::Error> {
        Err(production::Error::Unsupported(
            "stock building capital signals",
        ))
    }
    fn grant_yield(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
        _: i32,
    ) -> Result<(), production::Error> {
        Err(production::Error::Unsupported(
            "stock building completion yield grant",
        ))
    }
    fn report(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        building: u32,
        location: [i32; 2],
        report: buildings::Report,
    ) -> Result<(), production::Error> {
        self.check(state)?;
        state.city(owner, city)?;
        // These progress publications reach the source-qualified stock numeric,
        // read-only and cache consumers. Gameplay construction-plot writes are
        // already performed by pre_create/progress_changed, including the
        // explicit exclusion of coastal-flooding construction effects.
        match report {
            buildings::Report::ConstructionStarted
            | buildings::Report::QueueChanged
            | buildings::Report::BuildingProgress { .. } => {
                self.reports.push(Record {
                    owner,
                    city,
                    building,
                    location,
                    report,
                });
                Ok(())
            }
            _ => Err(production::Error::Unsupported(
                "stock building non-progress report",
            )),
        }
    }
    fn update_appeal(
        &mut self,
        state: &mut LoadedState,
        location: [i32; 2],
    ) -> Result<(), production::Error> {
        self.check(state)?;
        appeal::refresh_unchanged(state, location, self.appeal)
    }
    fn building_signals(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
        _: u32,
        _: i32,
    ) -> Result<(), production::Error> {
        Err(production::Error::Unsupported(
            "stock building completion signals",
        ))
    }
}
