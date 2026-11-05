//! Current-state citizen reallocation shared by founding and population changes.
//! Queries must read the same evolving state; native endpoint scores or selected
//! worker indices are not inputs.
use c6_core::{city_citizen_assignment as workers, loaded_state::LoadedState, unit::PlotCoord};

#[derive(Debug)]
pub enum Error<E> {
    State(c6_core::loaded_state::Error),
    World(c6_core::movement_world::MissingInput),
    Workers(workers::Error),
    Missing(&'static str),
    Effect(E),
}
/// Reached current-state queries and synchronous callbacks. Appeal refresh runs
/// before capacity/yield reads for each plot in native CityHexspace order.
pub trait Context {
    type Error;
    fn food_surplus(
        &mut self,
        state: &LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<i32, Self::Error>;
    fn refresh_appeal(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        plot: usize,
    ) -> Result<(), Self::Error>;
    fn capacity(
        &mut self,
        state: &LoadedState,
        owner: i32,
        city: u32,
        point: PlotCoord,
    ) -> Result<i32, Self::Error>;
    /// Ordered current yields including current favored/disfavored flags.
    fn yields(
        &mut self,
        state: &LoadedState,
        owner: i32,
        city: u32,
        plot: usize,
    ) -> Result<Vec<workers::Yield>, Self::Error>;
    fn workers_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        index: i32,
        previous: i32,
    ) -> Result<(), Self::Error>;
}
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Outcome {
    pub candidates: usize,
    pub food_surplus: i32,
}
/// ReallocateCitizens286f900: clear ordinary workers, compute current food and
/// score slots in City plot order, solve allocation, then report changed plots.
#[allow(clippy::too_many_arguments)]
pub fn reallocate<C: Context>(
    state: &mut LoadedState,
    context: &mut C,
    owner: i32,
    city: u32,
    food_type: i32,
    gold_type: i32,
    allocation_limit: usize,
) -> Result<Outcome, Error<C::Error>> {
    let population = state
        .city(owner, city)
        .map_err(Error::State)?
        .header
        .population;
    let previous = workers::remove_all_without_forced(
        &mut state.city_mut(owner, city).map_err(Error::State)?.citizens,
        population,
    )
    .map_err(Error::Workers)?;
    let food = context
        .food_surplus(state, owner, city)
        .map_err(Error::Effect)?;
    let header = &state.city(owner, city).map_err(Error::State)?.header;
    let center = PlotCoord {
        x: header.location_x,
        y: header.location_y,
    };
    let mut scores = Vec::new();
    let mut points = Vec::new();
    for index in 0..workers::NUM_PLOTS as i32 {
        let Some(point) =
            workers::plot_location(state.world().map(), center, index).map_err(Error::World)?
        else {
            continue;
        };
        let (plot, _) =
            c6_core::movement_world::plot_at(state.world().map(), point).map_err(Error::World)?;
        context
            .refresh_appeal(state, owner, city, plot)
            .map_err(Error::Effect)?;
        let capacity = context
            .capacity(state, owner, city, point)
            .map_err(Error::Effect)?;
        if capacity == 0 {
            continue;
        }
        let score = workers::score(
            context
                .yields(state, owner, city, plot)
                .map_err(Error::Effect)?,
            food_type,
            gold_type,
        );
        let citizens = &state.city(owner, city).map_err(Error::State)?.citizens;
        let used = workers::plot_workers(citizens, state.world().map(), center, point)
            .map_err(Error::Missing)?;
        for _ in 0..capacity.wrapping_sub(used).max(0) {
            if scores.len() >= allocation_limit {
                return Err(Error::Workers(workers::Error::AllocationLimit));
            }
            scores.push(score);
            points.push(point);
        }
    }
    let count = state
        .city(owner, city)
        .map_err(Error::State)?
        .citizens
        .num_unassigned_citizens as u32;
    for selected in
        workers::choose_workers(&scores, count, food, allocation_limit).map_err(Error::Workers)?
    {
        let point = points[selected];
        let index =
            workers::plot_index(state.world().map(), center, point).map_err(Error::World)?;
        let capacity = context
            .capacity(state, owner, city, point)
            .map_err(Error::Effect)?;
        workers::assign_to_index(
            &mut state.city_mut(owner, city).map_err(Error::State)?.citizens,
            index,
            workers::NUM_PLOTS as i32,
            true,
            Some(capacity),
            false,
        )
        .map_err(Error::Workers)?;
    }
    for (index, old) in previous.into_iter().enumerate() {
        context
            .workers_changed(state, owner, city, index as i32, old)
            .map_err(Error::Effect)?;
    }
    Ok(Outcome {
        candidates: scores.len(),
        food_surplus: food,
    })
}
