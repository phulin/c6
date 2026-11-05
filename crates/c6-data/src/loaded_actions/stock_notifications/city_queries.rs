//! Borrow current city query services for the persistent stock turn runtime.
use super::Runtime;
use crate::city_turn::{current_upkeep, yield_calculation, Error};
use c6_core::loaded_state::LoadedState;

#[derive(Clone, Copy)]
pub struct Inputs<'a> {
    pub definitions: &'a yield_calculation::Definitions,
    pub current: &'a current_upkeep::Current<'a>,
}
impl<'a, P> Runtime<'a, P> {
    pub fn city_query_inputs(&self) -> Option<Inputs<'a>> {
        self.city_queries
    }
    pub fn current_city_yield(
        &self,
        state: &LoadedState,
        owner: i32,
        city: u32,
        yield_index: i32,
        kind: u32,
    ) -> Result<i32, Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("current city stock runtime binding"))?;
        let inputs = self
            .city_queries
            .as_ref()
            .ok_or(Error::Unsupported("current city query services"))?;
        inputs
            .current
            .calculate(state, owner, city, yield_index, kind, inputs.definitions)
    }
}
impl<'a, P> Runtime<'a, P> {
    pub fn bind_city_queries(
        &mut self,
        state: &LoadedState,
        inputs: Inputs<'a>,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("current city stock runtime binding"))?;
        if self.city_queries.is_some() {
            return Err(Error::Unsupported("city query services already bound"));
        }
        if inputs.current.current_era_index.is_some() {
            return Err(Error::Unsupported(
                "stock turns require the canonical current game era",
            ));
        }
        if inputs.current.trade.is_some() {
            return Err(Error::Unsupported(
                "stock turns require the canonical current trade graph",
            ));
        }
        self.city_queries = Some(inputs);
        Ok(())
    }
}
