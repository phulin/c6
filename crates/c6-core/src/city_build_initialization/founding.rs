//! GrandStrategic.OnFoundCity21232f0 over the existing canonical planner/map.
//! This executes the saved strategic writes. It does not execute CityBuild's
//! production policy, evaluation jobs or behavior trees.
use crate::{
    loaded_state::{ActionStamp, LoadedState},
    map::{Chokepoint, Map, Recycled, Region},
    player_ai_grand_strategic::GrandStrategic,
};
#[derive(Debug)]
pub enum Error {
    State(crate::loaded_state::Error),
    Geometry(crate::movement_world::MissingInput),
    Binding,
    MissingPlayer,
    MissingGrandStrategic,
    MissingCityPlot,
    MissingChokepoint(u32),
    ChokepointEndpoints(u32),
    Allocation,
    UnsupportedPlanningOwner,
    MissingCityBuild,
    PlanningDataRequired,
}
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    origin: ActionStamp,
    owner: i32,
    minimum: i32,
    additional_per_city: i32,
}
impl Parameters {
    /// Current GrandStrategic settlement parameter cells, not a precomputed
    /// expansion score. The caller owns their freshness within this state.
    pub fn bind(state: &LoadedState, owner: i32, minimum: i32, additional_per_city: i32) -> Self {
        Self {
            origin: state.action_stamp(),
            owner,
            minimum,
            additional_per_city,
        }
    }
}
fn find<T>(rows: &Recycled<T>, id: u32) -> Option<&T> {
    if id == u32::MAX {
        return None;
    }
    rows.entries
        .iter()
        .find(|r| r.index == (id & 0xffff) as i32)
        .map(|r| &r.value)
}
fn region_at(map: &Map, plot: &crate::map::Plot) -> Option<u32> {
    find(&map.regions, plot.region_id).map(|r| r.id)
}
fn add_primary(
    primary: &mut Vec<u32>,
    expansion: &mut Vec<u32>,
    regions: &Recycled<Region>,
    chokepoints: &Recycled<Chokepoint>,
    region: u32,
) -> Result<(), Error> {
    if primary.contains(&region) {
        return Ok(());
    }
    primary.try_reserve(1).map_err(|_| Error::Allocation)?;
    primary.push(region);
    expansion.retain(|r| *r != region);
    let Some(data) = find(regions, region) else {
        return Ok(());
    };
    // The historical codec names these fields joined_region_ids/joined_area_ids.
    // Native begin2127e90 enumerates the first vector as chokepoint IDs; native
    // GetFront/Back2127eb0/c0 read the final serialized +58 vector as region IDs.
    for choke in &data.joined_region_ids {
        let endpoints = &find(chokepoints, *choke)
            .ok_or(Error::MissingChokepoint(*choke))?
            .joined_area_ids;
        let front = *endpoints
            .first()
            .ok_or(Error::ChokepointEndpoints(*choke))?;
        let other = if front == region {
            *endpoints.get(1).ok_or(Error::ChokepointEndpoints(*choke))?
        } else {
            front
        };
        if !expansion.contains(&other) {
            expansion.try_reserve(1).map_err(|_| Error::Allocation)?;
            expansion.push(other);
        }
    }
    Ok(())
}
/// Apply at the original DoFoundCity position, after CalculateYields/YieldAdvantage
/// and before CalculateBestUnitsInCity. Score assignment precedes region lookup.
pub fn on_found_city(
    state: &mut LoadedState,
    owner: i32,
    city_id: u32,
    parameters: &Parameters,
) -> Result<(), Error> {
    if parameters.owner != owner || !parameters.origin.same_instance(state.action_stamp()) {
        return Err(Error::Binding);
    }
    let city = state.city(owner, city_id).map_err(Error::State)?;
    let center = crate::unit::PlotCoord {
        x: city.header.location_x,
        y: city.header.location_y,
    };
    let count = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::MissingPlayer)?
        .cities
        .records
        .entries
        .len() as i32;
    let (map, strategic) = state.city_founding_strategic_parts(owner)?;
    strategic.city_planner.expansion_score = count
        .wrapping_mul(parameters.additional_per_city)
        .wrapping_add(parameters.minimum);
    strategic.city_planner.turns_since_city_founding = 0;
    let plot = map
        .plot_at(center.x, center.y)
        .ok_or(Error::MissingCityPlot)?;
    let region = region_at(map, plot);
    let GrandStrategic {
        primary_regions,
        expansion_regions,
        ..
    } = strategic;
    add_primary(
        primary_regions,
        expansion_regions,
        &map.regions,
        &map.chokepoints,
        region.unwrap_or(u32::MAX),
    )?;
    let Some(region) = region else { return Ok(()) };
    for index in 0..crate::city_citizen_assignment::NUM_PLOTS {
        let Some(at) = crate::city_citizen_assignment::plot_location(map, center, index as i32)
            .map_err(Error::Geometry)?
        else {
            continue;
        };
        let Some(plot) = map.plot_at(at.x, at.y) else {
            continue;
        };
        if let Some(other) = region_at(map, plot) {
            if other != region {
                add_primary(
                    primary_regions,
                    expansion_regions,
                    &map.regions,
                    &map.chokepoints,
                    other,
                )?;
            }
        }
    }
    Ok(())
}

/// Stock human DoFoundCity in the external-policy arena. The native job's
/// CityPlotEvaluation/AI analyzer outputs and terrain/production preferences are
/// planning data; they remain explicitly unavailable. Canonical strategic writes
/// still execute above. The stock behavior-tree factory returns null for a human
/// before allocating or executing policy. Its non-human branch is not covered.
pub fn on_found_city_external_policy(
    state: &mut LoadedState,
    owner: i32,
    city_id: u32,
    parameters: &Parameters,
) -> Result<(), Error> {
    if state.player_slot_status(owner).map_err(Error::State)? != 3 {
        return Err(Error::UnsupportedPlanningOwner);
    }
    let city_ai = state
        .city(owner, city_id)
        .map_err(Error::State)?
        .header
        .city_ai_id;
    let manager = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .and_then(|p| p.ai_city_manager.as_ref())
        .ok_or(Error::MissingCityBuild)?;
    let entry = manager
        .city_ais
        .entries
        .iter()
        .find(|e| e.city_build.city_ai_id == city_ai)
        .ok_or(Error::MissingCityBuild)?;
    if entry.city_build.city_id != city_id || entry.city_build.player_owner != owner {
        return Err(Error::Binding);
    }
    if entry.city_build.planning_data
        != crate::player_ai_city_manager::PlanningData::ExternalPolicyOmitted
    {
        return Err(Error::PlanningDataRequired);
    }
    on_found_city(state, owner, city_id, parameters)
}
/// Source DoFoundCity's synchronous DoOffTurn suffix. The three immutable
/// current HasAnalyzed operands are read in Research/Culture/Diplomacy order.
/// Numeric planner jobs stay unavailable under ExternalPolicyOmitted, while
/// their completed flag must retain the native state transition.
pub fn found_city_off_turn_external_policy(
    state: &mut LoadedState,
    owner: i32,
    city_id: u32,
    analyzed: [bool; 3],
) -> Result<(), Error> {
    if state.player_slot_status(owner).map_err(Error::State)? != 3 {
        return Err(Error::UnsupportedPlanningOwner);
    }
    let city_ai = state
        .city(owner, city_id)
        .map_err(Error::State)?
        .header
        .city_ai_id;
    let manager = state
        .city_build_manager_mut(owner)
        .map_err(|_| Error::MissingCityBuild)?;
    let entry = manager
        .city_ais
        .entries
        .iter_mut()
        .find(|e| e.city_build.city_ai_id == city_ai)
        .ok_or(Error::MissingCityBuild)?;
    if entry.city_build.city_id != city_id || entry.city_build.player_owner != owner {
        return Err(Error::Binding);
    }
    if entry.city_build.planning_data
        != crate::player_ai_city_manager::PlanningData::ExternalPolicyOmitted
    {
        return Err(Error::PlanningDataRequired);
    }
    if analyzed.into_iter().all(|ready| ready) {
        entry.city_build.done_off_turn.0 = 1;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::Slot;
    fn rows<T>(values: Vec<(u32, T)>) -> Recycled<T> {
        Recycled {
            capacity: values.len() as u32,
            entries: values
                .into_iter()
                .map(|(index, value)| Slot {
                    index: index as i32,
                    value,
                })
                .collect(),
            current_id_bits: 0,
            free_head: u32::MAX,
            free_tail: u32::MAX,
            free_links: vec![],
        }
    }
    fn region(id: u32, joins: Vec<u32>) -> Region {
        Region {
            id,
            plot_count: 0,
            chokepoint: false,
            area_id: 0,
            joined_region_ids: joins,
        }
    }
    fn choke(id: u32, ends: Vec<u32>) -> Chokepoint {
        Chokepoint {
            id,
            initialized: true,
            endpoints: vec![],
            points: vec![],
            joined_area_ids: ends,
        }
    }
    #[test]
    fn source_promotes_stably_and_keeps_neighbor_order_without_primary_filter() {
        let r = rows(vec![(1, region(1, vec![8, 9, 10]))]);
        let c = rows(vec![
            (8, choke(8, vec![1, 3])),
            (9, choke(9, vec![2, 1])),
            (10, choke(10, vec![1, 3])),
        ]);
        let (mut primary, mut expansion) = (vec![2], vec![7, 1, 3, 1]);
        add_primary(&mut primary, &mut expansion, &r, &c, 1).unwrap();
        assert_eq!(primary, [2, 1]);
        assert_eq!(expansion, [7, 3, 2]);
        add_primary(&mut primary, &mut expansion, &r, &rows(vec![]), 1).unwrap();
        assert_eq!(expansion, [7, 3, 2]);
    }
    #[test]
    fn null_region_is_real_sentinel_and_missing_choke_retains_native_prefix() {
        let (mut p, mut x) = (vec![], vec![u32::MAX]);
        let r = rows(vec![(1, region(1, vec![8]))]);
        let c = rows(vec![]);
        add_primary(&mut p, &mut x, &r, &c, u32::MAX).unwrap();
        assert_eq!(p, [u32::MAX]);
        assert!(x.is_empty());
        assert!(matches!(
            add_primary(&mut p, &mut x, &r, &c, 1),
            Err(Error::MissingChokepoint(8))
        ));
        assert_eq!(p, [u32::MAX, 1]);
    }
}
