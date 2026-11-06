//! City.Citizens.ReallocateCitizens over the same canonical worker arrays.
//! Capacity/yield and changed-report callbacks share the caller's current rules,
//! territory, Script and modifier owners. No captured worker scores are inputs.
use crate::city_turn::Error;
use c6_core::{
    city_citizen_assignment as workers, loaded_state::LoadedState,
    movement_world::DefinitionLookup, unit::PlotCoord,
};

pub struct Definitions {
    improvements: crate::registry::LinuxDefinitionRegistry,
    workable: Vec<bool>,
    districts: crate::registry::LinuxDefinitionRegistry,
    district_slots: Vec<i32>,
    buildings: crate::registry::LinuxDefinitionRegistry,
    building_slots: Vec<i32>,
}
impl Definitions {
    pub(crate) fn improvement_registry(&self) -> &crate::registry::LinuxDefinitionRegistry {
        &self.improvements
    }

    pub fn from_rules(rules: &crate::Rules) -> Result<Self, Error> {
        let improvements = crate::unit_initialization_definitions::collection(
            rules,
            "Improvements",
            "ImprovementType",
        )
        .map_err(Error::Definitions)?;
        let table = rules
            .gameplay()
            .table("Improvements")
            .ok_or(Error::Unsupported("Improvements"))?;
        let workable = improvements
            .entries()
            .iter()
            .map(|entry| {
                let row = table
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Unsupported("worker Improvement definition"))?;
                crate::game_parameters::sql_integer(row.get("Workable").map_err(Error::Rules)?)
                    .map(|v| v != 0)
                    .ok_or(Error::Unsupported("Improvement.Workable"))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let (districts, district_slots) = citizen_slots(rules, "Districts", "DistrictType")?;
        let (buildings, building_slots) = citizen_slots(rules, "Buildings", "BuildingType")?;
        Ok(Self {
            improvements,
            workable,
            districts,
            district_slots,
            buildings,
            building_slots,
        })
    }
}
/// Both native getters return Optional<int>; an absent value contributes zero.
fn citizen_slots(
    rules: &crate::Rules,
    table_name: &'static str,
    type_column: &'static str,
) -> Result<(crate::registry::LinuxDefinitionRegistry, Vec<i32>), Error> {
    let registry =
        crate::unit_initialization_definitions::collection(rules, table_name, type_column)
            .map_err(Error::Definitions)?;
    let table = rules
        .gameplay()
        .table(table_name)
        .ok_or(Error::Unsupported(table_name))?;
    let slots = registry
        .entries()
        .iter()
        .map(|entry| {
            let row = table
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Unsupported("worker slot definition"))?;
            match row.get("CitizenSlots").map_err(Error::Rules)? {
                crate::SqlValue::Null => Ok(0),
                value => crate::game_parameters::sql_integer(value)
                    .ok_or(Error::Unsupported("CitizenSlots")),
            }
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok((registry, slots))
}
pub struct Current<'a> {
    pub definitions: &'a crate::city_turn::yield_calculation::Definitions,
    pub territory: &'a crate::city_territory::Runtime,
    pub drought_ids: Option<&'a [i32]>,
    pub fallout_turns: &'a [i32],
}
pub fn capacity(
    state: &LoadedState,
    owner: i32,
    city: u32,
    point: PlotCoord,
    defs: &Definitions,
    current: &Current<'_>,
) -> Result<i32, Error> {
    let c = state.city(owner, city)?;
    if point.x == c.header.location_x && point.y == c.header.location_y {
        return Ok(0);
    }
    let (plot, p) =
        c6_core::movement_world::plot_at(state.world().map(), point).map_err(worker_error)?;
    let Some(a) = p.city_attributes.as_ref() else {
        return Ok(0);
    };
    if i32::from(a.player) != owner || a.working_city != city || a.wonder_building_hash != u32::MAX
    {
        return Ok(0);
    }
    if p.flags.get(c6_core::map::PlotFlag::Impassable) {
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Unsupported("worker owner"))?;
        let tail = player
            .tail
            .as_ref()
            .ok_or(Error::Unsupported("current impassable worker types"))?;
        if !tail
            .terrain_work_impassable
            .contains(&(p.terrain_hash as i32))
        {
            return Ok(0);
        }
    }
    if current.fallout_turns.get(plot).is_some_and(|v| *v > 0) {
        return Err(Error::Unsupported("Rules.Players.IsAllowedToWorkFallout"));
    }
    let indices = current
        .definitions
        .tiles
        .yield_indices()
        .collect::<Vec<_>>();
    let values = crate::city_turn::tiles::get_plot_yields(
        state,
        owner,
        &[plot as u32],
        &indices,
        &current.definitions.tiles,
        &crate::city_turn::tiles::Current {
            territory: current.territory,
            drought_ids: current.drought_ids,
            resource_visibility: &current.definitions.resources,
        },
    )?;
    if !values[0].iter().any(|&v| v > 0) {
        return Ok(0);
    }
    if let Some(index) = defs.improvements.find_index(p.improvement_hash) {
        if !defs.workable[index] {
            return Ok(0);
        }
    }
    if a.district != u32::MAX {
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Unsupported("worker district owner"))?;
        // Player.Districts.Find uses the recycled low slot. Plot.GetDistrictType
        // returns NO_DISTRICT when that slot is absent.
        let find = |id: u32| {
            (id != u32::MAX)
                .then(|| {
                    player
                        .districts
                        .records
                        .entries
                        .iter()
                        .find(|entry| entry.index as u32 == id & 0xffff)
                        .map(|entry| &entry.value)
                })
                .flatten()
        };
        if let Some(district) = find(a.district).filter(|d| d.district_hash != u32::MAX) {
            if !district.is_complete {
                return Ok(0);
            }
            // GetDistrictAt visits this city's district IDs in their stored
            // order; the plot attribute need not name the first location match.
            if c.districts
                .district_ids
                .iter()
                .filter_map(|&id| find(id))
                .find(|d| d.location == (point.x, point.y))
                .is_some_and(|d| d.pillaged)
            {
                return Ok(0);
            }
            let mut slots = defs
                .districts
                .find_index(district.district_hash)
                .map_or(0, |index| defs.district_slots[index]);
            // GetBuildingsAtLocation iterates the native definition order and
            // tests signed current locations before checking pillage.
            for (index, _) in defs.buildings.entries().iter().enumerate() {
                let hash = defs
                    .buildings
                    .type_hash(index)
                    .ok_or(Error::Unsupported("worker Building definition"))?;
                let location = c
                    .buildings
                    .building_locations
                    .iter()
                    .rfind(|v| v.definition_hash == hash)
                    .ok_or(Error::Unsupported("current worker Building location"))?
                    .value;
                if i32::from(location) != plot as i32 {
                    continue;
                }
                let pillaged = c
                    .buildings
                    .pillaged
                    .iter()
                    .rfind(|v| v.definition_hash == hash)
                    .ok_or(Error::Unsupported("current worker Building pillage"))?
                    .value;
                if !pillaged {
                    slots = slots.wrapping_add(defs.building_slots[index]);
                }
            }
            return Ok(slots);
        }
    }
    Ok(current.definitions.tiles.normal_worker_capacity())
}
pub fn plot_yields(
    state: &LoadedState,
    owner: i32,
    city: u32,
    plot: usize,
    current: &Current<'_>,
) -> Result<Vec<workers::Yield>, Error> {
    let definitions = &current.definitions.tiles;
    let indices = definitions.yield_indices().collect::<Vec<_>>();
    let amounts = crate::city_turn::tiles::get_plot_yields(
        state,
        owner,
        &[plot as u32],
        &indices,
        definitions,
        &crate::city_turn::tiles::Current {
            territory: current.territory,
            drought_ids: current.drought_ids,
            resource_visibility: &current.definitions.resources,
        },
    )?;
    let c = state.city(owner, city)?;
    indices
        .into_iter()
        .zip(&amounts[0])
        .map(|(native_index, amount)| {
            let hash = definitions
                .yield_hash(native_index)
                .ok_or(Error::Unsupported("worker Yield type"))?;
            let favored = c
                .citizens
                .favored_yields
                .iter()
                .rev()
                .find(|v| v.definition_hash == hash)
                .ok_or(Error::Unsupported("favored worker Yield"))?
                .value;
            let disfavored = c
                .citizens
                .disfavored_yields
                .iter()
                .rev()
                .find(|v| v.definition_hash == hash)
                .ok_or(Error::Unsupported("disfavored worker Yield"))?
                .value;
            Ok(workers::Yield {
                native_index,
                amount: *amount,
                favored,
                disfavored,
            })
        })
        .collect()
}

pub trait Effects {
    fn food_surplus_q8(&mut self, state: &LoadedState, owner: i32, city: u32)
        -> Result<i32, Error>;
    /// Native candidate visit performs adjacent appeal updates before capacity.
    fn visit_plot(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        plot: usize,
    ) -> Result<(), Error>;
    fn capacity(
        &mut self,
        state: &LoadedState,
        owner: i32,
        city: u32,
        point: PlotCoord,
    ) -> Result<i32, Error>;
    /// Values in actual yield definition order, including zero cells and the
    /// current favored/disfavored flags from this city's canonical citizens.
    fn plot_yields(
        &mut self,
        state: &LoadedState,
        owner: i32,
        city: u32,
        plot: usize,
    ) -> Result<Vec<workers::Yield>, Error>;
    fn food_and_gold_indices(&self) -> Result<(i32, i32), Error>;
    fn workers_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        index: i32,
        old: i32,
    ) -> Result<(), Error>;
}
fn worker_error(_: impl std::fmt::Debug) -> Error {
    Error::Unsupported("current citizen assignment storage")
}
pub fn reallocate(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    effects: &mut impl Effects,
) -> Result<(), Error> {
    let population = state.city(owner, city)?.header.population;
    // Native ReallocateCitizens snapshots ORDINARY counts, even when forced
    // counts are present. Preserve this old-value payload literally.
    let previous = state.city(owner, city)?.citizens.working_plot.clone();
    remove_all(state, owner, city, population, effects)?;
    try_assign_all(state, owner, city, false, effects)?;
    for (index, old) in previous.into_iter().enumerate() {
        effects.workers_changed(state, owner, city, index as i32, old)?;
    }
    Ok(())
}
/// RemoveAllCitizenWorkers286ed80(false), including retained locks, fresh
/// capacity clamps and the source's ordered overpopulation trimming.
fn remove_all(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    population: i32,
    effects: &mut impl Effects,
) -> Result<(), Error> {
    let c = state.city(owner, city)?;
    let center = PlotCoord {
        x: c.header.location_x,
        y: c.header.location_y,
    };
    let mut unassigned = population;
    for &forced in &c.citizens.forced_working_plot {
        if forced > 0 {
            unassigned = unassigned.wrapping_sub(forced);
        }
    }
    let citizens = &mut state.city_mut(owner, city)?.citizens;
    citizens.num_unassigned_citizens = unassigned;
    citizens.num_citizens_working_plots = 0;
    let count = citizens.forced_working_plot.len();
    for index in 0..count {
        let forced = state.city(owner, city)?.citizens.forced_working_plot[index];
        if forced <= 0 {
            continue;
        }
        let Some(point) = workers::plot_location(state.world().map(), center, index as i32)
            .map_err(worker_error)?
        else {
            continue;
        };
        let capacity = effects.capacity(state, owner, city, point)?;
        if capacity < forced {
            // Native repeats the fresh capacity getter after reading the count.
            let capacity = effects.capacity(state, owner, city, point)?;
            let removed = forced.wrapping_sub(capacity);
            let c = &mut state.city_mut(owner, city)?.citizens;
            c.forced_working_plot[index] = c.forced_working_plot[index].wrapping_sub(removed);
            c.num_unassigned_citizens = c.num_unassigned_citizens.wrapping_add(removed);
        }
    }
    if state.city(owner, city)?.citizens.num_unassigned_citizens < 0 {
        for index in 0..count {
            let c = &mut state.city_mut(owner, city)?.citizens;
            let forced = c.forced_working_plot[index];
            if forced > 0 {
                let removed = c.num_unassigned_citizens.wrapping_neg().min(forced);
                c.forced_working_plot[index] = forced.wrapping_sub(removed);
                c.num_unassigned_citizens = c.num_unassigned_citizens.wrapping_add(removed);
                // Preserve the native signed branch, including its early exit.
                if c.num_unassigned_citizens < 1 {
                    break;
                }
            }
        }
        let c = &mut state.city_mut(owner, city)?.citizens;
        if c.num_unassigned_citizens != 0 {
            c.num_unassigned_citizens = 0;
        }
    }
    for (index, value) in state
        .city_mut(owner, city)?
        .citizens
        .working_plot
        .iter_mut()
        .enumerate()
    {
        if *value > 0 || (index == 0 && *value != 1) {
            *value = i32::from(index == 0);
        }
    }
    Ok(())
}
/// TryAssignAllWorkers286f050 retains existing and forced assignments. The
/// report flag is forwarded to each selected AssignCitizenToWorkPlot call.
pub fn try_assign_all(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    report: bool,
    effects: &mut impl Effects,
) -> Result<(), Error> {
    let food = effects.food_surplus_q8(state, owner, city)?;
    let c = state.city(owner, city)?;
    let center = PlotCoord {
        x: c.header.location_x,
        y: c.header.location_y,
    };
    let mut candidates = Vec::new();
    let mut points = Vec::new();
    for index in 0..workers::NUM_PLOTS as i32 {
        let Some(point) =
            workers::plot_location(state.world().map(), center, index).map_err(worker_error)?
        else {
            continue;
        };
        let (plot, _) =
            c6_core::movement_world::plot_at(state.world().map(), point).map_err(worker_error)?;
        effects.visit_plot(state, owner, city, plot)?;
        let capacity = effects.capacity(state, owner, city, point)?;
        if capacity == 0 {
            continue;
        }
        let yields = effects.plot_yields(state, owner, city, plot)?;
        let (food, gold) = effects.food_and_gold_indices()?;
        let score = workers::score(yields, food, gold);
        let used = workers::plot_workers(
            &state.city(owner, city)?.citizens,
            state.world().map(),
            center,
            point,
        )
        .map_err(worker_error)?;
        for _ in 0..capacity.wrapping_sub(used).max(0) {
            candidates.push(score);
            points.push(point);
        }
    }
    let count = state.city(owner, city)?.citizens.num_unassigned_citizens as u32;
    for at in workers::choose_workers(&candidates, count, food, 100_000).map_err(worker_error)? {
        let point = points[at];
        let index =
            workers::plot_index(state.world().map(), center, point).map_err(worker_error)?;
        let capacity = effects.capacity(state, owner, city, point)?;
        let changed = workers::assign_to_index(
            &mut state.city_mut(owner, city)?.citizens,
            index,
            workers::NUM_PLOTS as i32,
            true,
            Some(capacity),
            report,
        )
        .map_err(worker_error)?;
        if let Some(change) = changed {
            effects.workers_changed(
                state,
                owner,
                city,
                change.city_plot_index,
                change.previous_workers,
            )?;
        }
    }
    Ok(())
}
