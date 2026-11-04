//! Current canonical query providers for FoundCity-created requirements.
use c6_core::{
    found_city::requirement_queries as kernel,
    game_national_parks::{NationalParks, ParkData},
    loaded_state::{self, LoadedState},
    map::PlotFlag,
};
pub use kernel::Age;
#[derive(Debug)]
pub enum Error {
    State(loaded_state::Error),
    Missing(&'static str),
    Age(kernel::Error),
}
/// Current logical vector bytes, not saved/captured Test answers.
pub struct Runtime {
    pub golden_ages: Option<Vec<u8>>,
    pub dark_ages: Option<Vec<u8>>,
    pub national_parks: Option<NationalParks>,
}
pub fn has_age(runtime: &Runtime, owner: i32, age: Age) -> Result<bool, Error> {
    if owner == -1 {
        return Ok(false);
    }
    let values = match age {
        Age::Golden => runtime.golden_ages.as_deref(),
        Age::Dark => runtime.dark_ages.as_deref(),
    }
    .ok_or(Error::Missing("current Game.Eras age vector"))?;
    kernel::has_age(values, owner).map_err(Error::Age)
}
struct Tourism<'a> {
    state: &'a LoadedState,
}
impl kernel::TourismContext for Tourism<'_> {
    type Error = Error;
    fn plot_appeal(&mut self, index: i32) -> Result<Option<i32>, Error> {
        let Some(plot) = self.state.world().map().plot_by_index(index as u32) else {
            return Ok(None);
        };
        if plot.flags.get(PlotFlag::HasCityAttributes) != plot.city_attributes.is_some() {
            return Err(Error::Missing("park plot city attributes"));
        }
        if plot.city_attributes.as_ref().is_none_or(|a| a.player == -1) {
            return Err(Error::Missing(
                "Rules.Appeal.GetAppeal for unowned park plot",
            ));
        }
        Ok(Some(i32::from(plot.appeal)))
    }
    fn national_park_modifier_q8(&mut self, owner: i32) -> Result<i32, Error> {
        self.state
            .world()
            .player(owner)
            .map_err(|_| Error::Missing("park founder Player"))?;
        self.state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .map(|p| p.culture.tourism_national_parks_modifier_bits)
            .ok_or(Error::Missing("park founder Culture"))
    }
}
/// Current owning-city membership is read from each park's FIRST plot; its
/// owner is compared before the city ID. Empty park plot arrays are native UB.
pub fn tourism_for_city(
    state: &LoadedState,
    runtime: &Runtime,
    owner: i32,
    id: u32,
) -> Result<i32, Error> {
    state.city(owner, id).map_err(Error::State)?;
    let parks = &runtime
        .national_parks
        .as_ref()
        .ok_or(Error::Missing("current NationalParks"))?
        .parks;
    let mut sum = 0_i32;
    for park in parks {
        let &first = park
            .plot_indices
            .first()
            .ok_or(Error::Missing("native empty ParkData plot array"))?;
        let p = state
            .world()
            .map()
            .plot_by_index(first as u32)
            .ok_or(Error::Missing("native null first park Plot"))?;
        if p.flags.get(PlotFlag::HasCityAttributes) != p.city_attributes.is_some() {
            return Err(Error::Missing("park first plot attributes"));
        }
        let Some(attributes) = p.city_attributes.as_ref() else {
            continue;
        };
        if i32::from(attributes.player) != owner {
            continue;
        }
        if attributes.owning_city != id {
            continue;
        }
        sum = sum.wrapping_add(kernel::tourism_from_park(&mut Tourism { state }, park)?);
    }
    Ok(sum)
}
pub fn tourism_from_park(state: &LoadedState, park: &ParkData) -> Result<i32, Error> {
    kernel::tourism_from_park(&mut Tourism { state }, park)
}
