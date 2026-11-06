//! Concrete components of Linux City destruction. The complete destruction
//! transition is unfinished; these operations do not release the City identity.
//! Positive unsupported components fail before silently discarding gameplay.
pub mod callbacks;
pub mod effects;
pub mod lifetime;
pub mod requirements;
pub mod transition;
use c6_core::{loaded_state::LoadedState, unit::ComponentId};

#[derive(Debug)]
pub enum Error {
    State(c6_core::loaded_state::Error),
    Knowledge(c6_core::loaded_state::game_statistics::AccessError),
    Definitions(crate::city_production_turn::Error),
    District(c6_core::loaded_state::district_initialization::DistrictError),
    Missing(&'static str),
    DistrictDefinitions(crate::loaded_actions::Error),
    ResourceRemovalRequired { plot: u32, resource: u32 },
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

/// Player.EventManager.OnDistrictDestroyed2b018b0 forwards to Resources2b70e10.
/// The actual center removal with no resource has old/new extractable resource
/// both NO_RESOURCE; HandleResourceChangeOnTile returns before gameplay/RNG.
/// Other observers, internal-only, pillaged or incomplete Districts preserve
/// their source guards. Reached resource deltas remain explicit.
pub fn district_resources_removed(
    state: &mut LoadedState,
    definitions: &crate::loaded_actions::city_range_attack::Definitions,
    observer: i32,
    district: ComponentId,
) -> Result<(), Error> {
    if district.component_type != 3 {
        return Err(Error::Missing("District resource removal component type"));
    }
    let record = state
        .district(i32::from(district.owner), district.id)
        .map_err(Error::District)?;
    let location = c6_core::unit::PlotCoord {
        x: record.location.0,
        y: record.location.1,
    };
    let Some(plot) =
        crate::unit_placement_services::plot_index_at::<std::convert::Infallible>(state, location)
            .map_err(|_| Error::Missing("District resource removal map"))?
    else {
        return Ok(());
    };
    // This reads an actual complete initialized definition, not the CityCenter
    // class name. CityCenter.InternalOnly is false in the combined fixture.
    let internal = definitions
        .district_is_internal_only(state, record.district_hash)
        .map_err(Error::DistrictDefinitions)?;
    let tile = state
        .world()
        .map()
        .plot_by_index(plot)
        .ok_or(Error::Missing("District resource removal plot"))?;
    let owner = tile
        .city_attributes
        .as_ref()
        .map_or(-1, |attributes| i32::from(attributes.player));
    if owner != observer || internal || record.pillaged || !record.is_complete {
        return Ok(());
    }
    if tile.resource_hash != u32::MAX {
        return Err(Error::ResourceRemovalRequired {
            plot,
            resource: tile.resource_hash,
        });
    }
    Ok(())
}

/// AI.Targets.CityDestroyed21aaf90 is gameplay knowledge upkeep, not policy.
/// The first current District revealed to this observer removes the first
/// matching full City identity from its existing CivKnowledge (2000fb0).
pub fn targets_city_destroyed(
    state: &mut LoadedState,
    observer: i32,
    city: ComponentId,
) -> Result<(), Error> {
    let current = canonical_city(state, city)?;
    let ids = current.districts.district_ids.clone();
    for id in ids {
        let district = match state.district(i32::from(city.owner), id) {
            Ok(district) => district,
            Err(
                c6_core::loaded_state::district_initialization::DistrictError::MissingDistrict {
                    ..
                },
            ) => continue,
            Err(error) => return Err(Error::District(error)),
        };
        let location = c6_core::unit::PlotCoord {
            x: district.location.0,
            y: district.location.1,
        };
        let Some(plot) = crate::unit_placement_services::plot_index_at::<std::convert::Infallible>(
            state, location,
        )
        .map_err(|_| Error::Missing("CityDestroyed Targets map geometry"))?
        else {
            continue;
        };
        let world = state.world();
        let visible = crate::unit_sight_state::mapped_visibility(
            &world.map().visibility,
            world.player_mapping.as_ref(),
            observer,
        )
        .map_err(Error::Missing)?;
        let revealed = *visible
            .revealed
            .get(plot as usize)
            .ok_or(Error::Missing("CityDestroyed Targets revealed plot"))?;
        if revealed == 0 {
            continue;
        }
        let targets = state
            .combat_ai_targets_mut(observer)
            .map_err(Error::Knowledge)?;
        if targets.player_id != observer || targets.knowledge_base.owner != observer {
            return Err(Error::Missing(
                "CityDestroyed Targets current observer identity",
            ));
        }
        if let Some(at) = targets
            .knowledge_base
            .known_cities
            .iter()
            .position(|known| known.owner == i32::from(city.owner) && known.id == city.id)
        {
            targets.knowledge_base.known_cities.remove(at);
        }
        // The semantic identity domain carries current known-City eligibility
        // when an imported native snapshot omitted policy-only scalar fields.
        if let Some(at) = targets
            .observed_city_identities
            .iter()
            .position(|known| *known == (i32::from(city.owner), city.id))
        {
            targets.observed_city_identities.remove(at);
        }
        break;
    }
    Ok(())
}

/// Rules.Placement.ExecuteForcedUnitRebase2bef910 copies the actual current
/// ObjectPairing children, then2bef760 only processes live Unit children.
/// Empty/non-Unit/missing-Unit branches return before gameplay or RNG; a live
/// Unit requires the concrete forced-rebase action rather than silent removal.
pub fn forced_unit_rebase(state: &LoadedState, parent: ComponentId) -> Result<(), Error> {
    let children = state
        .object_pairing()
        .map_err(Error::State)?
        .get_children(parent)
        .to_vec();
    for child in children {
        if child.component_type != 1 {
            continue;
        }
        let owner = state
            .world()
            .player(i32::from(child.owner))
            .map_err(|_| Error::Missing("forced rebase Unit owner"))?;
        if owner
            .unit_records()
            .map_err(|_| Error::Missing("forced rebase Unit roster"))?
            .iter()
            .any(|entry| entry.value.id == child.id)
        {
            return Err(Error::Missing("live child Unit forced rebase"));
        }
    }
    Ok(())
}

/// Player.Governors.RemoveGovernorFromCity is inert when the current owner has
/// no governor assigned to this exact City. Positive removal requires governor
/// pressure/establishment effects and is never treated as an empty callback.
pub fn remove_governor_from_city(
    state: &LoadedState,
    owner: i32,
    city: ComponentId,
) -> Result<(), Error> {
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("City destruction Governor owner"))?;
    if player.governors.entries.iter().any(|entry| {
        let assigned = &entry.governor.assigned_city;
        assigned.owner == city.owner && assigned.id == city.id
    }) {
        return Err(Error::Missing("assigned Governor removal effects"));
    }
    Ok(())
}

/// DistrictD1 validates references only with logging priority>=3; remaining
/// scalar/container FAuto destructors unregister sync metadata and free owned
/// storage. The transition drops that storage at its immediately following
/// canonical recycler removal, after capturing the reached residual scalars.
pub fn district_lifetime(
    state: &LoadedState,
    district: ComponentId,
    logging_priority: i32,
) -> Result<(), Error> {
    state
        .district(i32::from(district.owner), district.id)
        .map_err(Error::District)?;
    if logging_priority >= 3 {
        return Err(Error::Missing(
            "District destruction diagnostic reference validation",
        ));
    }
    Ok(())
}
