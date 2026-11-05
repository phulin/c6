//! Concrete components of Linux City destruction. The complete destruction
//! transition is unfinished; these operations do not release the City identity.
//! Positive unsupported components fail before silently discarding gameplay.
use c6_core::{loaded_state::LoadedState, unit::ComponentId};

#[derive(Debug)]
pub enum Error {
    State(c6_core::loaded_state::Error),
    Definitions(crate::city_production_turn::Error),
    District(c6_core::loaded_state::district_initialization::DistrictError),
    Missing(&'static str),
    BuildQueueClearRequired,
    BuildingRemovalRequired { index: usize, hash: u32, plot: i16 },
}

fn canonical_city(state: &LoadedState, city: ComponentId) -> Result<&c6_core::city::City, Error> {
    if city.component_type != 2 {
        return Err(Error::Missing("City destruction component type"));
    }
    state
        .city(i32::from(city.owner), city.id)
        .map_err(Error::State)
}

/// BuildQueue.RemoveAll284d880 only calls Clear when current GetSize!=0.
/// An actual empty canonical queue returns without callback, state or RNG
/// changes. Nonempty queues retain every entry until Clear is implemented.
pub fn remove_all_queue(state: &mut LoadedState, city: ComponentId) -> Result<(), Error> {
    if canonical_city(state, city)?.build_queue.entries.is_empty() {
        Ok(())
    } else {
        Err(Error::BuildQueueClearRequired)
    }
}

/// Buildings.RemoveAllBuildings2863880 visits every current definition index.
/// RemoveBuilding28642c0 returns immediately for an actual location of -1.
/// Require the complete effective initialized array: a sparse/unknown wire
/// vector cannot establish an absent building or permit native destruction.
/// Positive removal remains explicit, including negative sentinels other than
/// -1, which native still routes through building-removal callbacks.
pub fn remove_all_buildings(
    state: &mut LoadedState,
    city: ComponentId,
    definitions: &crate::city_production_turn::buildings::Definitions,
) -> Result<(), Error> {
    definitions.validate(state).map_err(Error::Definitions)?;
    let buildings = &canonical_city(state, city)?.buildings;
    if buildings.building_locations.len() != definitions.buildings.len() {
        return Err(Error::Missing(
            "City destruction effective Building locations",
        ));
    }
    for (index, (location, definition)) in buildings
        .building_locations
        .iter()
        .zip(&definitions.buildings)
        .enumerate()
    {
        if location.definition_hash != definition.hash {
            return Err(Error::Missing("City destruction Building definition index"));
        }
        if location.value != -1 {
            return Err(Error::BuildingRemovalRequired {
                index,
                hash: definition.hash,
                plot: location.value,
            });
        }
    }
    Ok(())
}

/// District.MarkForDelete2886df0 precedes the District removal report and both
/// ordered player/game signal dispatches. The record stays canonical/alive.
pub fn mark_district_for_delete(
    state: &mut LoadedState,
    district: ComponentId,
) -> Result<(), Error> {
    if district.component_type != 3 {
        return Err(Error::Missing("District destruction component type"));
    }
    state
        .district_mut(i32::from(district.owner), district.id)
        .map_err(Error::District)?
        .marked_for_delete = Some(true);
    Ok(())
}
