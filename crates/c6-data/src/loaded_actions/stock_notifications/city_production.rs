//! Natural ordinary-building progress on the persistent stock turn runtime.
//!
//! This adapter shares current city queries with upkeep. It supports progress,
//! including ordinary construction-plot initialization and externally chosen
//! ordinary-building queue selection and completion through the same mutable
//! modifier/requirement owners. Additional gameplay consumers remain explicit.
//! Errors retain any mutation
//! prefix already performed by the production kernel.
pub mod completion;
pub mod unit;
use super::{Decision, Runtime};
use crate::city_production_turn::{self as production, appeal, buildings, selection};
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

pub struct Effects<'runtime, 'inputs, 'definitions, P> {
    runtime: &'runtime mut Runtime<'inputs, P>,
    definitions: &'definitions buildings::Definitions,
    appeal: &'definitions appeal::Definitions,
    pub reports: Vec<Record>,
    completion: Option<completion::Services<'definitions>>,
}
impl<'runtime, 'inputs, 'definitions, P> Effects<'runtime, 'inputs, 'definitions, P> {
    pub fn new(
        runtime: &'runtime mut Runtime<'inputs, P>,
        definitions: &'definitions buildings::Definitions,
        appeal: &'definitions appeal::Definitions,
        _: Scope,
    ) -> Self {
        Self {
            runtime,
            definitions,
            appeal,
            reports: Vec::new(),
            completion: None,
        }
    }
    /// Borrow the current completion registrations; all mutable gameplay owners
    /// remain in this SAME stock runtime.
    pub fn with_completion(mut self, services: completion::Services<'definitions>) -> Self {
        self.completion = Some(services);
        self
    }
    /// Apply an external building decision to this SAME current City/runtime.
    /// Legality is evaluated before queue writes; no production policy runs.
    /// Reached stock queue reports use the already-bound current city queries.
    #[allow(clippy::too_many_arguments)]
    pub fn choose_building(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        building_hash: u32,
        choices: &selection::Definitions,
        current: &selection::Current<'_>,
        queries: &impl selection::Queries,
    ) -> Result<(), production::Error> {
        self.check(state)?;
        if self.runtime.city_query_inputs().is_none() {
            return Err(production::Error::Unsupported(
                "current selection report query services",
            ));
        }
        let definitions = self.definitions;
        selection::append_building(
            state,
            owner,
            city,
            building_hash,
            choices,
            definitions,
            current,
            queries,
            self,
        )?;
        // A successful live queue choice resolves this City's semantic prompt.
        // Other decisions and native notification storage are untouched.
        resolve_city_production(&mut self.runtime.decisions, owner, city);
        Ok(())
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
impl<P> production::Effects for Effects<'_, '_, '_, P> {
    fn population_services(&mut self) -> Option<&mut dyn crate::city_turn::PopulationEffects> {
        Some(self.runtime)
    }
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
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        _: &QueueEntry,
    ) -> Result<bool, production::Error> {
        self.check(state)?;
        if self.completion.is_none() {
            return Err(production::Error::Unsupported(
                "stock building completion services",
            ));
        }
        buildings::finish_current(state, owner, city, self.definitions, self)
    }
    fn new_front(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<(), production::Error> {
        self.check(state)?;
        let definitions = self.definitions;
        selection::new_front(state, owner, city, definitions, self)
    }
}
impl<P> buildings::Effects for Effects<'_, '_, '_, P> {
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
            | buildings::Report::ProductionChanged
            | buildings::Report::QueueEntryChanged { .. }
            | buildings::Report::BuildingProgress { .. }
            | buildings::Report::BuildingCompleted { .. } => {
                if matches!(
                    report,
                    buildings::Report::QueueChanged
                        | buildings::Report::ProductionChanged
                        | buildings::Report::QueueEntryChanged { .. }
                ) {
                    production::run_build_queue_report_queries(state, owner, city, self)?;
                }
                self.reports.push(Record {
                    owner,
                    city,
                    building,
                    location,
                    report,
                });
                Ok(())
            }
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
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        building: u32,
        plot: i32,
    ) -> Result<(), production::Error> {
        self.check(state)?;
        let services = self
            .completion
            .as_ref()
            .ok_or(production::Error::Unsupported(
                "stock building completion services",
            ))?;
        completion::dispatch(
            self.runtime,
            state,
            crate::city_production_turn::building_callbacks::Event {
                owner,
                city,
                building,
            },
            plot,
            self.definitions,
            services,
        )
    }
}

impl<P> production::QueueEffects for Effects<'_, '_, '_, P> {
    fn queue_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        index: usize,
    ) -> Result<(), production::Error> {
        self.check(state)?;
        let value = state.city(owner, city)?;
        let location = [value.header.location_x, value.header.location_y];
        let entry = value
            .build_queue
            .entries
            .get(index)
            .ok_or(production::Error::Unsupported("current queue entry"))?;
        let hash = building(entry)?;
        buildings::Effects::report(
            self,
            state,
            owner,
            city,
            hash,
            location,
            buildings::Report::QueueEntryChanged { index },
        )
    }
}
impl<P> production::QueueReportQueries for Effects<'_, '_, '_, P> {
    fn get_yield_q8(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        yield_index: i32,
        kind: Option<u32>,
    ) -> Result<i32, production::Error> {
        self.check(state)?;
        self.runtime
            .current_city_yield(state, owner, city, yield_index, kind.unwrap_or(u32::MAX))
            .map_err(|_| production::Error::Unsupported("stock queue report current yield query"))
    }
}

fn resolve_city_production(decisions: &mut Vec<Decision>, owner: i32, city: u32) {
    decisions.retain(|decision| {
        decision.type_hash != 0x40fb_81ed
            || decision.owner != owner
            || !decision.component.is_some_and(|id| {
                id.component_type == 2 && id.owner == owner as i16 && id.id == city
            })
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    use c6_core::notifications::ComponentId;
    fn decision(owner: i32, city: u32, type_hash: u32) -> Decision {
        Decision {
            owner,
            type_hash,
            location: [18, 11],
            component: Some(ComponentId {
                owner: owner as i16,
                component_type: 2,
                id: city,
            }),
        }
    }
    #[test]
    fn selecting_one_city_keeps_other_players_cities_and_research_decisions() {
        let selected = decision(0, 65536, 0x40fb81ed);
        let other_city = decision(0, 131073, 0x40fb81ed);
        let other_owner = decision(1, 65536, 0x40fb81ed);
        let research = decision(0, 65536, 0x592df4c8);
        let mut decisions = vec![
            selected,
            other_city.clone(),
            other_owner.clone(),
            research.clone(),
        ];
        resolve_city_production(&mut decisions, 0, 65536);
        assert_eq!(decisions, vec![other_city, other_owner, research]);
    }
}
