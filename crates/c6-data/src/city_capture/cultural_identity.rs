//! Combat transfer loyalty on the same current founding query services.
//! The surviving-major branch still expels other major players' units from
//! the tile before recalculating conversion trends. An empty eligible unit
//! set reaches no placement callback or random draw.
use crate::{city_turn::Error, found_city::cultural_identity as loyalty};
use c6_core::{loaded_state::LoadedState, unit::ComponentId};

/// The ordinary empty-expulsion branch of Diplomacy.ExpelFromTile2aaa710.
/// UnitContainer.GetUnits is read in layer/cell order, then owner-filtered;
/// native Units.Find ignores the generation and checks the low16 slot.
/// A live matching slot needs the full formation-placement continuation.
pub fn expel_other_majors_empty(
    state: &LoadedState,
    owner: i32,
    city: u32,
    alive_majors: &[i32],
) -> Result<(), Error> {
    let value = state.city(owner, city)?;
    let location = c6_core::unit::PlotCoord {
        x: value.header.location_x,
        y: value.header.location_y,
    };
    let (plot, _) = c6_core::movement_world::plot_at(state.world().map(), location)
        .map_err(|_| Error::Unsupported("transfer expulsion plot"))?;
    for &other in alive_majors {
        if other == owner {
            continue;
        }
        let player = state
            .world()
            .player(other)
            .map_err(|_| Error::Unsupported("transfer expulsion Player"))?;
        let units = player
            .units()
            .map_err(|_| Error::Unsupported("transfer expulsion Units owner"))?;
        for reference in state
            .world()
            .map()
            .units
            .layers
            .iter()
            .flat_map(|layer| &layer.cells)
            .filter(|cell| cell.plot_index == plot as u32)
            .flat_map(|cell| &cell.units)
        {
            if reference.player() == other
                && reference.id != u32::MAX
                && units
                    .entries
                    .iter()
                    .any(|entry| entry.index == (reference.id & 0xffff) as i32)
            {
                return Err(Error::Unsupported(
                    "combat transfer unit formation expulsion placement",
                ));
            }
        }
    }
    Ok(())
}

struct Transfer<'a, Q> {
    queries: &'a mut Q,
    alive_majors: &'a [i32],
}
impl<Q: loyalty::Context> loyalty::Context for Transfer<'_, Q> {
    fn alive_ids(&self) -> &[i32] {
        self.queries.alive_ids()
    }
    fn civilization_level(&self, owner: i32) -> Result<u32, Error> {
        self.queries.civilization_level(owner)
    }
    fn has_martial_law_garrison(
        &mut self,
        state: &LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<bool, Error> {
        self.queries.has_martial_law_garrison(state, owner, city)
    }
    fn combat_transfer_events(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<(), Error> {
        expel_other_majors_empty(state, owner, city, self.alive_majors)
    }
    fn combat_transfer_output(&mut self, _: &mut LoadedState, _: i32, _: u32) -> Result<(), Error> {
        // HandleLogging2873560 queries identity breakdowns and formats a file
        // record. Its reached output has no gameplay writes/callbacks or RNG.
        Ok(())
    }
    fn food_surplus_q8(
        &mut self,
        state: &LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<i32, Error> {
        self.queries.food_surplus_q8(state, owner, city)
    }
    fn happiness(&mut self, state: &LoadedState, owner: i32, city: u32) -> Result<i32, Error> {
        self.queries.happiness(state, owner, city)
    }
    fn player_property_int(
        &mut self,
        state: &LoadedState,
        owner: i32,
        key: u32,
    ) -> Result<Option<i32>, Error> {
        self.queries.player_property_int(state, owner, key)
    }
    fn trend_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        outcome: i32,
        changed: Option<bool>,
    ) -> Result<(), Error> {
        self.queries
            .trend_changed(state, owner, city, outcome, changed)
    }
}

/// Called after transferred population, with the fresh City's current
/// owner-before-occupation value. Reuse the SAME ordinary food, happiness,
/// property and trend-report query context used during founding.
pub fn transferred(
    state: &mut LoadedState,
    definitions: &loyalty::Definitions,
    queries: &mut impl loyalty::Context,
    alive_majors: &[i32],
    city: ComponentId,
    previous_owner: i32,
) -> Result<(), Error> {
    for &owner in alive_majors {
        if queries.civilization_level(owner)? != 0x253718b0 || !state.player_alive(owner)? {
            return Err(Error::Unsupported("current alive-major expulsion order"));
        }
    }
    loyalty::do_combat_city_transferred(
        state,
        definitions,
        &mut Transfer {
            queries,
            alive_majors,
        },
        i32::from(city.owner),
        city.id,
        previous_owner,
    )
}
