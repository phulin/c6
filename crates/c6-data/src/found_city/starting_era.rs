//! Source-ordered initial population, housing and amenities for a placed City.
use crate::{registry::linux_type_hash, Rules};
use c6_core::loaded_state::LoadedState;
#[derive(Debug)]
pub enum Error {
    State(c6_core::loaded_state::Error),
    Rules(crate::Error),
    Missing(&'static str),
}
/// Read the current start-era definition. The target's absent-definition
/// defaults are population1/housing1/amenities0. Housing and amenity additions
/// precede the caller's center-District creation and population callbacks.
pub fn apply(
    state: &mut LoadedState,
    rules: &Rules,
    owner: i32,
    city: u32,
    start_era: u32,
) -> Result<i32, Error> {
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("current founding Player"))?;
    let first = player.cities.records.entries.len() == 1;
    let row = rules
        .gameplay()
        .table("StartEras")
        .ok_or(Error::Missing("StartEras"))?
        .rows()
        .find(|row| {
            row.text("EraType")
                .ok()
                .is_some_and(|name| linux_type_hash(name.as_bytes()) == start_era)
        });
    let value = |field, missing| match row {
        Some(row) => row.integer(field).map(|v| v as i32).map_err(Error::Rules),
        None => Ok(missing),
    };
    // The native other-city population getter runs even for the first City.
    let mut population = value("StartingPopulationOtherCities", 1)?;
    let housing = if first {
        population = value("StartingPopulationCapital", 1)?;
        value("StartingHousingCapital", 1)?
    } else {
        value("StartingHousingOtherCities", 1)?
    };
    let amenities = if first {
        value("StartingAmenitiesCapital", 0)?
    } else {
        value("StartingAmenitiesOtherCities", 0)?
    };
    let growth = &mut state.city_mut(owner, city).map_err(Error::State)?.growth;
    growth.housing_from_starting_era_modifier = growth
        .housing_from_starting_era_modifier
        .wrapping_add(housing);
    growth.amenities_from_starting_era_modifier = growth
        .amenities_from_starting_era_modifier
        .wrapping_add(amenities);
    Ok(population)
}
