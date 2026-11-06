//! Ordered transfer=true destruction. Services borrow the ongoing gameplay
//! owners; none has a successful default. Failure retains the applied prefix.
use super::{canonical_city, mark_district_for_delete, remove_all_queue};
use crate::city_capture::retired_district::RetiredDistrictScalars;
use c6_core::{
    loaded_state::LoadedState,
    unit::{ComponentId, PlotCoord},
};

/// Remaining domain operations are dispatched at their actual source position.
/// In particular CityDestroyed runs while City/location/Districts are live,
/// and City teardown precedes recycled slot release.
pub trait Context {
    fn state(&self) -> &LoadedState;
    fn state_mut(&mut self) -> &mut LoadedState;
    fn city_destroyed(&mut self, city: ComponentId) -> Result<(), String>;
    fn world_builder_active(&self) -> Result<bool, String>;
    fn mark_name_unused(&mut self, city: ComponentId) -> Result<(), String>;
    fn alive_major_ids(&self) -> Result<&[i32], String>;
    fn remove_improvement_placement(&mut self, plot: u32) -> Result<(), String>;
    fn remove_buildings(&mut self, city: ComponentId) -> Result<(), String>;
    fn is_shutting_down(&self) -> Result<bool, String>;
    fn report_district_removed(&mut self, district: ComponentId) -> Result<(), String>;
    fn player_district_removed(&mut self, district: ComponentId) -> Result<(), String>;
    fn game_district_removed(&mut self, district: ComponentId) -> Result<(), String>;
    fn district_is_canal(&self, district: ComponentId) -> Result<bool, String>;
    fn remove_canal_domain(&mut self, plot: u32) -> Result<(), String>;
    /// Destruction validation reads the same pool's actual logging priority.
    fn logging_priority(&self) -> Result<i32, String>;
    fn remove_improvement(&mut self, plot: u32) -> Result<(), String>;
    fn report_city_removed(&mut self, city: ComponentId, event: u32) -> Result<(), String>;
    fn clear_city_plots(&mut self, city: ComponentId) -> Result<(), String>;
    fn update_coastal_domains(&mut self, old_location: PlotCoord) -> Result<(), String>;
    /// Unsubscribe City/EventManager handles, release modifiers, CityBuild and
    /// current factory references. Canonical City is alive throughout this call.
    fn destroy_city_lifetime(&mut self, city: ComponentId) -> Result<(), String>;
}

#[derive(Debug)]
pub struct Outcome {
    pub retired_districts: Vec<(ComponentId, RetiredDistrictScalars)>,
}
#[derive(Debug)]
pub struct Error {
    pub operation: &'static str,
    pub detail: String,
    pub retired_districts: Vec<(ComponentId, RetiredDistrictScalars)>,
}
fn err(operation: &'static str, detail: impl std::fmt::Debug) -> Error {
    Error {
        operation,
        detail: format!("{detail:?}"),
        retired_districts: Vec::new(),
    }
}
fn call<T>(operation: &'static str, value: Result<T, String>) -> Result<T, Error> {
    value.map_err(|e| err(operation, e))
}
fn purchased(state: &LoadedState, city: ComponentId) -> Vec<u32> {
    state
        .world()
        .map()
        .plots
        .iter()
        .enumerate()
        .rev()
        .filter_map(|(i, p)| {
            p.city_attributes
                .as_ref()
                .filter(|a| i16::from(a.player) == city.owner && a.owning_city == city.id)
                .map(|_| i as u32)
        })
        .collect()
}

fn district_exists(state: &LoadedState, owner: i32, id: u32) -> Result<bool, Error> {
    use c6_core::loaded_state::district_initialization::DistrictError;
    match state.district(owner, id) {
        Ok(_) => Ok(true),
        Err(DistrictError::MissingDistrict { .. }) => {
            let player = state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == owner)
                .ok_or_else(|| err("District owner", "missing"))?;
            if player
                .districts
                .records
                .entries
                .iter()
                .any(|row| row.index as u32 == id & 0xffff)
            {
                return Err(err(
                    "District list identity",
                    "native low-slot generation mismatch",
                ));
            }
            Ok(false)
        }
        Err(error) => Err(err("District lookup", error)),
    }
}

pub fn destroy_for_transfer(
    context: &mut impl Context,
    city: ComponentId,
) -> Result<Outcome, Error> {
    let mut retired = Vec::new();
    let result = destroy(context, city, &mut retired);
    match result {
        Ok(()) => Ok(Outcome {
            retired_districts: retired,
        }),
        Err(mut error) => {
            error.retired_districts = retired;
            Err(error)
        }
    }
}
fn destroy(
    context: &mut impl Context,
    city: ComponentId,
    retired: &mut Vec<(ComponentId, RetiredDistrictScalars)>,
) -> Result<(), Error> {
    canonical_city(context.state(), city).map_err(|e| err("City identity", e))?;
    let owner = i32::from(city.owner);
    let _original = {
        let h = &context
            .state()
            .city(owner, city.id)
            .map_err(|e| err("City location", e))?
            .header;
        PlotCoord {
            x: h.location_x,
            y: h.location_y,
        }
    };
    {
        let h = &mut context
            .state_mut()
            .city_mut(owner, city.id)
            .map_err(|e| err("mark delayed death", e))?
            .header;
        h.delayed_death = true;
        h.transferring = true;
    }
    call("CityDestroyed", context.city_destroyed(city))?;
    if call("WorldBuilder active", context.world_builder_active())? {
        call("mark name unused", context.mark_name_unused(city))?;
    }
    let major_ids = call("AliveMajorIDs", context.alive_major_ids())?.to_vec();
    for owner in major_ids {
        super::remove_governor_from_city(context.state(), owner, city)
            .map_err(|e| err("remove governor", e))?;
    }
    // Native rereads the live District list while traversing its numeric index.
    let mut index = 0;
    loop {
        let ids = &canonical_city(context.state(), city)
            .map_err(|e| err("District list", e))?
            .districts
            .district_ids;
        let Some(id) = ids.get(index).copied() else {
            break;
        };
        let district = ComponentId {
            owner: city.owner,
            component_type: 3,
            id,
        };
        if district_exists(context.state(), owner, id)? {
            super::forced_unit_rebase(context.state(), district)
                .map_err(|e| err("forced District child rebase", e))?;
        }
        index += 1;
    }
    for plot in purchased(context.state(), city) {
        let tile = &context.state().world().map().plots[plot as usize];
        if i16::from(tile.improvement_owner) == city.owner && tile.improvement_hash != u32::MAX {
            call(
                "remove Improvement placement",
                context.remove_improvement_placement(plot),
            )?;
        }
    }
    // City.Placement.Remove snapshots location again after preceding callbacks.
    let location = {
        let h = &canonical_city(context.state(), city)
            .map_err(|e| err("placement City", e))?
            .header;
        PlotCoord {
            x: h.location_x,
            y: h.location_y,
        }
    };
    {
        let h = &mut context
            .state_mut()
            .city_mut(owner, city.id)
            .map_err(|e| err("invalidate location", e))?
            .header;
        h.location_x = -9999;
        h.location_y = -9999;
    }
    let original_plot = crate::unit_placement_services::plot_index_at::<std::convert::Infallible>(
        context.state(),
        location,
    )
    .map_err(|e| err("original plot", e))?;
    if let Some(original_plot) = original_plot {
        remove_all_queue(context.state_mut(), city).map_err(|e| err("BuildQueue.RemoveAll", e))?;
        // Real nonempty building removal remains a mandatory gameplay service.
        call("Buildings.RemoveAll", context.remove_buildings(city))?;
        while let Some(id) = canonical_city(context.state(), city)
            .map_err(|e| err("Districts.DestroyAll", e))?
            .districts
            .district_ids
            .last()
            .copied()
        {
            let district = ComponentId {
                owner: city.owner,
                component_type: 3,
                id,
            };
            if !district_exists(context.state(), owner, id)? {
                context
                    .state_mut()
                    .city_mut(owner, city.id)
                    .map_err(|e| err("pop missing District", e))?
                    .districts
                    .district_ids
                    .pop();
                continue;
            }
            mark_district_for_delete(context.state_mut(), district)
                .map_err(|e| err("MarkForDelete", e))?;
            call(
                "District removal report",
                context.report_district_removed(district),
            )?;
            if !call("IsShuttingDown", context.is_shutting_down())? {
                call(
                    "Player DistrictRemoved",
                    context.player_district_removed(district),
                )?;
                call(
                    "Game DistrictRemoved",
                    context.game_district_removed(district),
                )?;
            }
            let record = context
                .state()
                .district(owner, id)
                .map_err(|e| err("current removed District", e))?;
            let coord = PlotCoord {
                x: record.location.0,
                y: record.location.1,
            };
            if let Some(plot) = crate::unit_placement_services::plot_index_at::<
                std::convert::Infallible,
            >(context.state(), coord)
            .map_err(|e| err("District plot", e))?
            {
                let attrs = context
                    .state_mut()
                    .world_mut()
                    .map_err(|e| err("District attribute", e))?
                    .plot_mut(plot as usize)
                    .and_then(|p| p.city_attributes.as_mut())
                    .ok_or_else(|| err("District attribute", "missing"))?;
                attrs.district = u32::MAX;
                if call(
                    "District Canal definition",
                    context.district_is_canal(district),
                )? {
                    call("remove Canal domain", context.remove_canal_domain(plot))?;
                }
            }
            let ids = &mut context
                .state_mut()
                .city_mut(owner, city.id)
                .map_err(|e| err("erase City District", e))?
                .districts
                .district_ids;
            if let Some(at) = ids.iter().position(|v| *v == id) {
                ids.remove(at);
            }
            let priority = call("GameEffects logging priority", context.logging_priority())?;
            super::district_lifetime(context.state(), district, priority)
                .map_err(|e| err("District lifetime", e))?;
            let scalars = RetiredDistrictScalars::before_registry_remove(context.state(), district)
                .map_err(|e| err("District residual scalars", e))?;
            let records = &mut context
                .state_mut()
                .player_districts_mut(owner)
                .map_err(|e| err("District owner", e))?
                .records;
            c6_core::city_initialization::release_slot(records, id & 0xffff)
                .map_err(|e| err("District slot removal", e))?
                .ok_or_else(|| err("District slot removal", "missing"))?;
            retired.push((district, scalars));
        }
        for plot in purchased(context.state(), city) {
            if context.state().world().map().plots[plot as usize].improvement_hash != u32::MAX {
                call(
                    "remove remaining Improvement",
                    context.remove_improvement(plot),
                )?;
            }
        }
        call(
            "City removal report1",
            context.report_city_removed(city, 0x1d355e83),
        )?;
        call(
            "City removal report2",
            context.report_city_removed(city, 0x53f22816),
        )?;
        call("CityPlots.Clear", context.clear_city_plots(city))?;
        // Clear's SetOwner(NO_PLAYER) normally removes these attributes.
        // A reentrant owner callback can create new current attributes; only
        // then can the source's coastal-and-HasCityAttributes guard reach the
        // positive SetValidDomains continuation.
        if context.state().world().map().plots[original_plot as usize]
            .city_attributes
            .is_some()
        {
            call("coastal domains", context.update_coastal_domains(location))?;
        }
    }
    // transfer=true skips the post-removal unit relocation loop.
    call("City destructor", context.destroy_city_lifetime(city))?;
    let records = &mut context
        .state_mut()
        .cities_mut(owner)
        .map_err(|e| err("City owner", e))?
        .records;
    c6_core::city_initialization::release_slot(records, city.id & 0xffff)
        .map_err(|e| err("City slot removal", e))?
        .ok_or_else(|| err("City slot removal", "missing"))?;
    Ok(())
}
