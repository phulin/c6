//! Borrow the existing current faith/culture calculations for opinion rules.
//! No yield answers or additional mutable gameplay owners are retained here.
use super::social;
use crate::{loaded_actions::stock_notifications::city_queries, research};
use c6_core::loaded_state::LoadedState;

pub struct Current<'s, 'a> {
    pub runtime: &'s mut research::Runtime,
    pub definitions: &'a research::Definitions,
    pub social_definitions: &'a social::Definitions,
    pub social_queries: &'a social::Queries<'a>,
}
struct Culture<'s, 'a> {
    runtime: &'s mut research::Runtime,
    cities: Option<city_queries::Inputs<'a>>,
}
impl research::Effects for Culture<'_, '_> {
    type Error = &'static str;
    fn runtime(&self) -> &research::Runtime {
        self.runtime
    }
    fn runtime_mut(&mut self) -> &mut research::Runtime {
        self.runtime
    }
    fn effect(
        &mut self,
        _: &mut LoadedState,
        _: &research::Definitions,
        _: research::Effect,
    ) -> Result<(), Self::Error> {
        Err("opinion yield unexpected research effect")
    }
    fn query(
        &mut self,
        state: &mut LoadedState,
        _: &research::Definitions,
        query: research::Query,
    ) -> Result<i32, Self::Error> {
        match query {
            research::Query::CityYield { city, yield_index } => {
                let cities = self.cities.ok_or("opinion current city yields")?;
                cities
                    .current
                    .calculate(
                        state,
                        i32::from(city.owner),
                        city.id,
                        yield_index,
                        0,
                        cities.definitions,
                    )
                    .map_err(|_| "opinion current city yield calculation")
            }
            _ => Err("opinion positive culture-yield contribution"),
        }
    }
}
impl crate::loaded_actions::stock_notifications::first_contact::opinion_callbacks::CurrentYields
    for Current<'_, '_>
{
    fn faith_culture_strength(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
    ) -> Result<i32, &'static str> {
        let faith = social::faith_yield(state, self.social_definitions, self.social_queries, owner)
            .map_err(|error| match error {
                c6_core::game_update::Error::Unsupported(cause) => cause,
                _ => "opinion current Faith calculation",
            })?;
        let mut effects = Culture {
            runtime: self.runtime,
            cities: self.social_queries.cities,
        };
        let culture = research::Context {
            state,
            definitions: self.definitions,
            effects: &mut effects,
        }
        .culture_yield(owner)
        .map_err(|error| match error {
            research::Error::Missing(cause) | research::Error::Effect(cause) => cause,
            _ => "opinion current Culture calculation",
        })?;
        // GetFaithCultureStrength2361440 truncates each Q8 independently,
        // wraps their whole-integer sum, then clamps nonpositive to one.
        Ok((faith / 256).wrapping_add(culture / 256).max(1))
    }
}
