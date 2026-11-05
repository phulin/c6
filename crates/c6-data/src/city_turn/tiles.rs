//! City tile yields from current canonical plots/citizens and the shared Effects owner.
use super::{resources, yield_definitions::Definitions, Error};
use c6_core::{
    city_district::{
        CityRecord, ComponentId, DistrictRecord, PlayerRegistry, Registry, SlotTable, PLAYER_COUNT,
    },
    improvement_yield::{ImprovementAdjacency, ImprovementYieldError},
    loaded_state::LoadedState,
    movement_world::{self, DefinitionLookup},
    plot_yield as kernel,
    unit::PlotCoord,
    yields::{NativeDefinitionIndex, NativeFixedPoint8},
};
/// Borrow the SAME territory owner used by founding/modifier updates. Drought
/// IDs are the current ClimateData collection, not historical RandomEvents.
/// None remains unavailable. Positive IDs require current drought objects.
pub struct Current<'a> {
    pub territory: &'a crate::city_territory::Runtime,
    pub drought_ids: Option<&'a [i32]>,
    pub resource_visibility: &'a resources::Definitions,
}
pub(in crate::city_turn) struct Runtime<'a> {
    state: &'a LoadedState,
    definitions: &'a Definitions,
    current: &'a Current<'a>,
}
impl Runtime<'_> {
    fn city(&self, id: ComponentId) -> Result<&c6_core::city::City, kernel::Error> {
        self.state
            .city(i32::from(id.owner), id.id)
            .map_err(|_| kernel::Error::Provider("current yield City"))
    }
    fn player(
        &self,
        owner: i32,
    ) -> Result<&c6_core::loaded_components::RetainedPlayer, kernel::Error> {
        self.state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(kernel::Error::Provider("current yield Player"))
    }
    fn yield_hash(&self, index: i32) -> Result<u32, kernel::Error> {
        self.definitions
            .yields
            .type_hash(index as u32 as usize)
            .ok_or(kernel::Error::DefinitionIndex)
    }
    fn location(&self, id: ComponentId, building: i32) -> Result<i32, kernel::Error> {
        let hash = self
            .definitions
            .buildings
            .type_hash(building as u32 as usize)
            .ok_or(kernel::Error::DefinitionIndex)?;
        self.city(id)?
            .buildings
            .building_locations
            .iter()
            .rfind(|r| r.definition_hash == hash)
            .map(|r| i32::from(r.value))
            .ok_or(kernel::Error::Provider("current building location"))
    }
}
impl kernel::Runtime for Runtime<'_> {
    fn terrain_work_impassable(&self, owner: i32, terrain: i32) -> Result<bool, kernel::Error> {
        let tail = self
            .player(owner)?
            .tail
            .as_ref()
            .ok_or(kernel::Error::Provider("current terrain-work flags"))?;
        Ok(tail.terrain_work_impassable.iter().any(|&t| {
            self.definitions
                .terrains
                .find_index(t as u32)
                .map_or(-1, |i| i as i32)
                == terrain
        }))
    }
    fn visible_resource(&self, owner: i32, resource: i32) -> Result<i32, kernel::Error> {
        self.current
            .resource_visibility
            .visible_resource(self.state, owner, resource)
            .map_err(|_| kernel::Error::Provider("current resource visibility"))
    }
    fn city_effect_yield(&self, plot: u32, kind: i32) -> Result<Option<i32>, kernel::Error> {
        let p = self
            .state
            .world()
            .map()
            .plot_by_index(plot)
            .ok_or(kernel::Error::Plot(plot))?;
        if p.city_attributes.is_none() {
            return Ok(None);
        }
        self.current
            .territory
            .plot_yield_data(plot)
            .map(|object| object.map(|o| o.yield_value(kind)))
            .map_err(kernel::Error::Provider)
    }
    fn city_buildings_at_location(
        &self,
        city: ComponentId,
        plot: u32,
    ) -> Result<Vec<u32>, kernel::Error> {
        let mut result = Vec::new();
        for index in 0..self.definitions.buildings.count() {
            // Native28645d0 tests location only, including pillaged buildings.
            if self.location(city, index as i32)? == plot as i32 {
                result.push(
                    self.definitions
                        .buildings
                        .type_hash(index)
                        .ok_or(kernel::Error::DefinitionIndex)?,
                );
            }
        }
        Ok(result)
    }
    fn building_location(&self, city: ComponentId, building: i32) -> Result<i32, kernel::Error> {
        self.location(city, building)
    }
    fn building_pillaged(&self, city: ComponentId, building: i32) -> Result<bool, kernel::Error> {
        let hash = self
            .definitions
            .buildings
            .type_hash(building as u32 as usize)
            .ok_or(kernel::Error::DefinitionIndex)?;
        self.city(city)?
            .buildings
            .pillaged
            .iter()
            .rfind(|r| r.definition_hash == hash)
            .map(|r| r.value)
            .ok_or(kernel::Error::Provider("current building pillage"))
    }
    fn great_person_resource_yield(
        &self,
        owner: i32,
        kind: i32,
        resource: i32,
    ) -> Result<Option<i32>, kernel::Error> {
        if self
            .state
            .world()
            .valid_player(owner)
            .map_err(|_| kernel::Error::Provider("current Player mapping"))?
            .is_none()
        {
            return Ok(None);
        }
        let yield_hash = self.yield_hash(kind)?;
        let resource_hash = self
            .definitions
            .resources
            .type_hash(resource as u32 as usize)
            .ok_or(kernel::Error::DefinitionIndex)?;
        let mut classes = Vec::new();
        for row in &self
            .player(owner)?
            .great_people_points
            .yield_change_per_earned_great_person_class
        {
            if row.yield_hash == yield_hash
                && row.resource_hash == resource_hash
                && !classes.contains(&row.great_person_class_hash)
            {
                classes.push(row.great_person_class_hash);
            }
        }
        for class in classes {
            let coefficient = self
                .player(owner)?
                .great_people_points
                .yield_change_per_earned_great_person_class
                .iter()
                .filter(|r| {
                    r.yield_hash == yield_hash
                        && r.resource_hash == resource_hash
                        && r.great_person_class_hash == class
                })
                .fold(0i32, |n, r| n.wrapping_add(r.amount));
            if coefficient > 0 {
                return Err(kernel::Error::Provider(
                    "current received GreatPeople owner",
                ));
            }
        }
        Ok(Some(0))
    }
    fn city_yield_per_flood(&self, city: ComponentId, kind: i32) -> Result<i32, kernel::Error> {
        let hash = self.yield_hash(kind)?;
        self.city(city)?
            .header
            .yield_per_flood
            .iter()
            .rfind(|r| r.definition_hash == hash)
            .map(|r| r.value)
            .ok_or(kernel::Error::Provider("current yield-per-flood modifier"))
    }
    fn has_city_terrain_adjacent_improvement(
        &self,
        city: ComponentId,
        terrain: i32,
        kind: i32,
    ) -> Result<bool, kernel::Error> {
        Ok(self.city(city)?.header.terrain_modifiers.iter().any(|r| {
            r.yield_type_index == kind
                && self
                    .definitions
                    .terrains
                    .find_index(r.terrain_hash)
                    .map_or(-1, |i| i as i32)
                    == terrain
        }))
    }
    fn city_terrain_adjacent_improvement(
        &self,
        city: ComponentId,
        terrain: i32,
        improvement: i32,
        kind: i32,
    ) -> Result<NativeFixedPoint8, kernel::Error> {
        let value = self
            .city(city)?
            .header
            .terrain_modifiers
            .iter()
            .filter(|r| {
                r.yield_type_index == kind
                    && self
                        .definitions
                        .terrains
                        .find_index(r.terrain_hash)
                        .map_or(-1, |i| i as i32)
                        == terrain
                    && self
                        .definitions
                        .improvements
                        .find_index(r.adjacent_improvement_hash)
                        .map_or(-1, |i| i as i32)
                        == improvement
            })
            .fold(0i32, |n, r| n.wrapping_add(r.yield_amount.wrapping_shl(8)));
        Ok(NativeFixedPoint8(value))
    }
    fn city_yield_per_terrain_class_improvement(
        &self,
        city: ComponentId,
        kind: i32,
        improvement: i32,
    ) -> Result<i32, kernel::Error> {
        Ok(self
            .city(city)?
            .header
            .terrain_class_yield_improvement_modifiers
            .iter()
            .filter(|r| {
                r.yield_type_index == kind
                    && self
                        .definitions
                        .improvements
                        .find_index(r.improvement_hash)
                        .map_or(-1, |i| i as i32)
                        == improvement
            })
            .fold(0i32, |n, r| n.wrapping_add(r.amount)))
    }
    fn active_drought_type(&self, _plot: u32) -> Result<i32, kernel::Error> {
        match self.current.drought_ids {
            Some([]) => Ok(-1),
            Some(_) => Err(kernel::Error::Provider("current drought objects")),
            None => Err(kernel::Error::Provider("current Climate drought IDs")),
        }
    }
}
fn adjacency_amount(change: i32, tiles: i32, count: i32) -> Option<i32> {
    let raw = change.wrapping_shl(8).wrapping_mul(count);
    let raw = if tiles > 1 {
        let divisor = tiles.wrapping_shl(8);
        if divisor == 0 {
            return None;
        }
        (((raw as i64) << 8) / i64::from(divisor)) as i32
    } else {
        raw
    };
    // Native AsInt is arithmetic >>8, including negative fractional yields.
    Some(raw >> 8)
}
impl ImprovementAdjacency for Runtime<'_> {
    fn player_adjacency_yield(
        &self,
        player: i32,
        definition: NativeDefinitionIndex,
        requested_yield: i32,
        plot: Option<u32>,
    ) -> Result<i32, ImprovementYieldError> {
        use c6_core::yields::{PrerequisiteKind, PrerequisiteState};
        let Some(plot) = plot else {
            return Ok(0);
        };
        let missing = || ImprovementYieldError::AdjacencyState {
            definition: definition.get(),
        };
        let rule = self
            .definitions
            .adjacency
            .get(definition.get() as usize)
            .ok_or_else(missing)?;
        let world = self.state.world();
        if !rule.valid(|technology, index| {
            let kind = if technology {
                PrerequisiteKind::Technology
            } else {
                PrerequisiteKind::Civic
            };
            let acquired = world.acquired_mask(player, kind)?;
            if index as u32 as usize >= acquired.len() {
                return Ok::<_, c6_core::yields::MissingInput>(false);
            }
            Ok(world.globally_available(kind, index)? && acquired[index as usize])
        })? {
            return Ok(0);
        }
        if rule.excluded {
            return Err(missing());
        }
        if requested_yield != -1 && requested_yield != rule.yield_index {
            return Ok(0);
        }
        if !rule.supported {
            return Err(missing());
        }
        let map = world.map();
        if map.plot_by_index(plot).is_none() {
            return Err(missing());
        }
        let mut count = i32::from(rule.self_yield);
        if let Some(improvement) = rule.improvement {
            let location = (
                plot as i32 % map.header.width,
                plot as i32 / map.header.width,
            );
            for direction in 0..6 {
                let Some((x, y)) = crate::loaded_actions::city_range_attack::adjacent_location(
                    self.state, location, direction,
                ) else {
                    continue;
                };
                let neighbor = map
                    .plot_by_index((y * map.header.width + x) as u32)
                    .ok_or_else(missing)?;
                if self
                    .definitions
                    .improvements
                    .find_index(neighbor.improvement_hash)
                    .map(|v| v as i32)
                    == Some(improvement)
                    && !neighbor
                        .flags
                        .get(c6_core::map::PlotFlag::ImprovementPillaged)
                {
                    count = count.wrapping_add(1);
                }
            }
        }
        let value =
            adjacency_amount(rule.change, rule.tiles_required, count).ok_or_else(missing)?;
        if requested_yield == -1 {
            return Err(ImprovementYieldError::Normalization {
                yield_index: rule.yield_index,
            });
        }
        Ok(value)
    }
}
#[cfg(test)]
mod adjacency_tests {
    use super::*;
    #[test]
    fn farm_groups_and_negative_fraction_follow_fixed_point_order() {
        assert_eq!(adjacency_amount(1, 2, 1), Some(0));
        assert_eq!(adjacency_amount(1, 2, 3), Some(1));
        assert_eq!(adjacency_amount(1, 1, 3), Some(3));
        assert_eq!(adjacency_amount(-1, 2, 1), Some(-1));
        assert_eq!(adjacency_amount(i32::MAX, 2, 2), Some(-1));
    }
}
struct PlayerRecords {
    cities: Vec<CityRecord>,
    districts: Vec<DistrictRecord>,
}
fn records(
    state: &LoadedState,
    definitions: &Definitions,
) -> Result<[Option<PlayerRecords>; PLAYER_COUNT], Error> {
    let mut result = std::array::from_fn(|_| None);
    for player in &state.retained().players {
        let slot = usize::try_from(player.requested_owner)
            .ok()
            .filter(|&i| i < PLAYER_COUNT)
            .ok_or(Error::Unsupported("current City owner slot"))?;
        let cities = player
            .cities
            .records
            .entries
            .iter()
            .map(|entry| CityRecord {
                id: entry.value.header.id,
                owner: entry.value.header.owner,
                location: PlotCoord {
                    x: entry.value.header.location_x,
                    y: entry.value.header.location_y,
                },
            })
            .collect();
        let mut districts = Vec::new();
        for entry in &player.districts.records.entries {
            let d = &entry.value;
            let location = PlotCoord {
                x: d.location.0,
                y: d.location.1,
            };
            let (_, plot) = movement_world::plot_at(state.world().map(), location)
                .map_err(|_| Error::Unsupported("current district plot"))?;
            districts.push(DistrictRecord {
                id: d.id,
                owner: plot
                    .city_attributes
                    .as_ref()
                    .map_or(-1, |a| i32::from(a.player)),
                location,
                parent_city: d.city_id,
                type_index: definitions
                    .districts
                    .find_index(d.district_hash)
                    .map_or(-1, |i| i as i32),
                complete: d.is_complete,
            });
        }
        result[slot] = Some(PlayerRecords { cities, districts });
    }
    Ok(result)
}
pub(in crate::city_turn) fn with_context<T>(
    state: &LoadedState,
    definitions: &Definitions,
    current: &Current<'_>,
    query: impl FnOnce(&kernel::Context<'_, Runtime<'_>, Runtime<'_>>) -> Result<T, Error>,
) -> Result<T, Error> {
    let owned = records(state, definitions)?;
    let mut players = [PlayerRegistry::Unavailable; PLAYER_COUNT];
    for (slot, records) in owned.iter().enumerate() {
        if let Some(records) = records {
            players[slot] = PlayerRegistry::Valid {
                cities: Some(
                    SlotTable::new(&records.cities)
                        .map_err(|_| Error::Unsupported("current City slot table"))?,
                ),
                districts: Some(
                    SlotTable::new(&records.districts)
                        .map_err(|_| Error::Unsupported("current District slot table"))?,
                ),
            };
        } else if state
            .world()
            .valid_player(slot as i32)
            .map_err(|_| Error::Unsupported("current Player mapping"))?
            .is_none()
        {
            players[slot] = PlayerRegistry::Invalid;
        }
    }
    let registry = Registry::new(players);
    let map = state.world().map();
    let mut adjacency = Vec::with_capacity(map.plots.len());
    for index in 0..map.plots.len() {
        let location = (
            index as i32 % map.header.width,
            index as i32 / map.header.width,
        );
        let mut adjacent = [None; 6];
        for (direction, value) in adjacent.iter_mut().enumerate() {
            if let Some((x, y)) = crate::loaded_actions::city_range_attack::adjacent_location(
                state, location, direction,
            ) {
                *value = Some((y * map.header.width + x) as u32);
            }
        }
        adjacency.push(adjacent);
    }
    let feature_yields =
        kernel::FeatureYieldStorage::from_world(state.world(), &definitions.yields)
            .map_err(Error::PlotYield)?;
    let runtime = Runtime {
        state,
        definitions,
        current,
    };
    definitions.with_rules(|rules| {
        let context = kernel::Context {
            world: state.world(),
            cities: &registry,
            rules,
            runtime: &runtime,
            improvement_adjacency: &runtime,
            feature_yields: Some(&feature_yields),
            adjacency: Some(&adjacency),
        };
        query(&context)
    })
}

/// Ordered current integer plot yields for citizen scoring. Rows follow the
/// supplied plots; columns follow the supplied native Yield indices. Duplicate
/// plots and definitions are evaluated in place, not deduplicated or sorted.
/// Query the same Effects/territory owner used by assignment mutations.
pub fn get_plot_yields(
    state: &LoadedState,
    owner: i32,
    plot_indices: &[u32],
    yield_indices: &[i32],
    definitions: &Definitions,
    current: &Current<'_>,
) -> Result<Vec<Vec<i32>>, Error> {
    with_context(state, definitions, current, |context| {
        plot_indices
            .iter()
            .map(|&plot| {
                yield_indices
                    .iter()
                    .map(|&kind| {
                        kernel::get_yield(context, plot, kind, owner, false)
                            .map_err(Error::PlotYield)
                    })
                    .collect()
            })
            .collect()
    })
}

/// Source2895950: current37 plot order, BOTH forced and ordinary workers,
/// integer plot yield*worker count before Q8 conversion. Nothing imports final
/// native tile-yield answers; the existing core kernel computes every value.
pub fn get_yield_from_tiles_q8(
    state: &LoadedState,
    owner: i32,
    id: u32,
    kind: i32,
    definitions: &Definitions,
    current: &Current<'_>,
) -> Result<i32, Error> {
    let map = state.world().map();
    let city = state.city(owner, id)?;
    let center = PlotCoord {
        x: city.header.location_x,
        y: city.header.location_y,
    };
    with_context(state, definitions, current, |context| {
        let mut result = 0i32;
        for index in 0..c6_core::city_citizen_assignment::NUM_PLOTS as i32 {
            let Some(plot) = c6_core::city_citizen_assignment::plot_location(map, center, index)
                .map_err(|_| Error::Unsupported("current City tile"))?
            else {
                continue;
            };
            let workers =
                c6_core::city_citizen_assignment::plot_workers(&city.citizens, map, center, plot)
                    .map_err(Error::Unsupported)?;
            if workers <= 0 {
                continue;
            }
            let (at, _) = movement_world::plot_at(map, plot)
                .map_err(|_| Error::Unsupported("current City tile"))?;
            let value = kernel::get_yield(context, at as u32, kind, owner, false)
                .map_err(Error::PlotYield)?;
            result = result.wrapping_add(value.wrapping_mul(workers).wrapping_shl(8));
        }
        if city.header.capital {
            let improvements = &state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == owner)
                .ok_or(Error::Unsupported("current capital owner"))?
                .improvements;
            for &hash in &definitions.outside_improvements {
                let Some(kind_index) = definitions.improvements.find_index(hash) else {
                    continue;
                };
                for &plot_index in &improvements.plot_indices {
                    let Some(plot) = map.plot_by_index(plot_index as u32) else {
                        continue;
                    };
                    if definitions.improvements.find_index(plot.improvement_hash)
                        != Some(kind_index)
                    {
                        continue;
                    }
                    let value = kernel::get_yield(context, plot_index as u32, kind, owner, false)
                        .map_err(Error::PlotYield)?;
                    result = result.wrapping_add(value.wrapping_shl(8));
                }
            }
        }
        Ok(result)
    })
}
