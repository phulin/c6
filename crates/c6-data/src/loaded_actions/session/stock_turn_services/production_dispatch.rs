//! Select a production adapter over the same current gameplay owners.
use crate::{
    city_production_turn as production, district_actions::stock as district,
    loaded_actions::stock_notifications::city_production as building,
};
use c6_core::{city_production::QueueEntry, loaded_state::LoadedState};

pub(super) enum Effects<'s, 'runtime, 'a, P> {
    Building(building::Effects<'s, 'runtime, 'a, P>),
    District(district::Effects<'s, 'runtime, 'a, P>),
}
macro_rules! dispatch {
    ($self:ident, $method:ident($($arg:expr),*)) => {
        match $self {
            Effects::Building(e) => production::Effects::$method(e, $($arg),*),
            Effects::District(e) => production::Effects::$method(e, $($arg),*),
        }
    };
}
impl<P> production::Effects for Effects<'_, '_, '_, P> {
    fn population_services(&mut self) -> Option<&mut dyn crate::city_turn::PopulationEffects> {
        dispatch!(self, population_services())
    }
    fn calculate_yield_q8(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        y: i32,
        k: u32,
    ) -> Result<i32, production::Error> {
        dispatch!(self, calculate_yield_q8(s, o, c, y, k))
    }
    fn current_build(
        &mut self,
        s: &LoadedState,
        o: i32,
        c: u32,
        e: &QueueEntry,
    ) -> Result<production::CurrentBuild, production::Error> {
        dispatch!(self, current_build(s, o, c, e))
    }
    fn start_progress(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        e: &QueueEntry,
    ) -> Result<(), production::Error> {
        dispatch!(self, start_progress(s, o, c, e))
    }
    fn progress_report(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        e: &QueueEntry,
    ) -> Result<(), production::Error> {
        dispatch!(self, progress_report(s, o, c, e))
    }
    fn finish(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        e: &QueueEntry,
    ) -> Result<bool, production::Error> {
        dispatch!(self, finish(s, o, c, e))
    }
    fn new_front(&mut self, s: &mut LoadedState, o: i32, c: u32) -> Result<(), production::Error> {
        dispatch!(self, new_front(s, o, c))
    }
}
