//! Current amenities from canonical allocations, buildings, districts and city modifiers.
use super::{Error, GrowthRules};
use crate::{
    registry::linux_type_hash, unit_initialization_definitions::collection, Rules, SqlValue,
};
use c6_core::{
    city::City, city_production::Order, game_capabilities::Membership, loaded_state::LoadedState,
    movement_world, unit::PlotCoord,
};
struct Building {
    hash: u32,
    amount: i32,
    powered: i32,
    range: i32,
}
struct Project {
    hash: u32,
    amenities: i32,
    fully_powered: bool,
}
pub struct Definitions {
    buildings: Vec<Building>,
    districts: Vec<(u32, i32)>,
    projects: Vec<Project>,
    resources: Vec<(u32, bool)>,
    resource_visibility: super::resources::Definitions,
    improvements: crate::registry::LinuxDefinitionRegistry,
}
fn integer(row: crate::Row<'_>, field: &'static str) -> Result<i32, Error> {
    match row.get(field).map_err(Error::Rules)? {
        SqlValue::Null => Ok(0),
        v => crate::game_parameters::sql_integer(v).ok_or(Error::Unsupported(field)),
    }
}
impl Definitions {
    pub fn from_rules(rules: &Rules) -> Result<Self, Error> {
        let table = |name| rules.gameplay().table(name).ok_or(Error::Unsupported(name));
        let registry = |name, col| collection(rules, name, col).map_err(Error::Definitions);
        let mut buildings = Vec::new();
        for entry in registry("Buildings", "BuildingType")?.entries() {
            let row = table("Buildings")?
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Unsupported("amenity Building"))?;
            let mut powered = 0;
            for xp in table("Buildings_XP2")?.rows() {
                if xp.text("BuildingType").map_err(Error::Rules)?
                    == row.text("BuildingType").map_err(Error::Rules)?
                {
                    powered = integer(xp, "EntertainmentBonusWithPower")?;
                }
            }
            buildings.push(Building {
                hash: linux_type_hash(&entry.type_name),
                amount: integer(row, "Entertainment")?,
                range: integer(row, "RegionalRange")?,
                powered,
            });
        }
        let mut districts = Vec::new();
        for entry in registry("Districts", "DistrictType")?.entries() {
            let row = table("Districts")?
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Unsupported("amenity District"))?;
            districts.push((
                linux_type_hash(&entry.type_name),
                integer(row, "Entertainment")?,
            ));
        }
        let mut projects = Vec::new();
        for entry in registry("Projects", "ProjectType")?.entries() {
            let row = table("Projects")?
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Unsupported("amenity Project"))?;
            let mut fully_powered = false;
            for xp in table("Projects_XP2")?.rows() {
                if xp.text("ProjectType").map_err(Error::Rules)?
                    == row.text("ProjectType").map_err(Error::Rules)?
                {
                    fully_powered = integer(xp, "FullyPoweredWhileActive")? != 0;
                }
            }
            projects.push(Project {
                hash: linux_type_hash(&entry.type_name),
                amenities: integer(row, "AmenitiesWhileActive")?,
                fully_powered,
            });
        }
        let mut resources = Vec::new();
        for entry in registry("Resources", "ResourceType")?.entries() {
            let row = table("Resources")?
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Unsupported("amenity Resource"))?;
            let luxury = row
                .text("ResourceClassType")
                .map_err(Error::Rules)?
                .as_bytes();
            resources.push((
                linux_type_hash(&entry.type_name),
                linux_type_hash(luxury) == 0xa9f06a65,
            ));
        }
        Ok(Self {
            buildings,
            districts,
            projects,
            resources,
            resource_visibility: super::resources::Definitions::from_rules(rules)?,
            improvements: registry("Improvements", "ImprovementType")?,
        })
    }
    fn project(&self, city: &City) -> Result<Option<&Project>, Error> {
        let Some(Order::Advance { project_hash }) =
            city.build_queue.entries.first().map(|e| e.order)
        else {
            return Ok(None);
        };
        self.projects
            .iter()
            .find(|p| p.hash == project_hash)
            .map(Some)
            .ok_or(Error::Unsupported("current Project definition"))
    }
    pub fn fully_powered(&self, city: &City) -> Result<bool, Error> {
        let p = &city.power;
        let free = if p.modified_free_power > 0 {
            ((i64::from(p.free_power_raw_q8)
                * i64::from(p.modified_free_power.wrapping_add(100).wrapping_shl(8)))
                / 25600) as i32
        } else {
            p.free_power_raw_q8
        };
        if p.required_power_total_raw_q8
            .wrapping_sub(free)
            .wrapping_sub(p.temporary_power_raw_q8)
            <= 0
        {
            return Ok(true);
        }
        Ok(self.project(city)?.is_some_and(|p| p.fully_powered))
    }
    pub fn entertainment(&self, state: &LoadedState, owner: i32, id: u32) -> Result<i32, Error> {
        let player = player(state, owner)?;
        let city = state.city(owner, id)?;
        let mut result = 0i32;
        for definition in &self.buildings {
            if definition.amount <= 0 && definition.powered <= 0 {
                continue;
            }
            if definition.range > 0 {
                let mut maximum = 0;
                for entry in &player.cities.records.entries {
                    let source = &entry.value;
                    let Some(index) = building_location(source, definition.hash)? else {
                        continue;
                    };
                    let location = c6_core::city_citizen_assignment::plot_location(
                        state.world().map(),
                        PlotCoord {
                            x: source.header.location_x,
                            y: source.header.location_y,
                        },
                        i32::from(index),
                    )
                    .map_err(|_| Error::Unsupported("regional building plot"))?
                    .ok_or(Error::Unsupported("regional building location"))?;
                    let (_, plot) = movement_world::plot_at(state.world().map(), location)
                        .map_err(|_| Error::Unsupported("regional building plot"))?;
                    let district = plot
                        .city_attributes
                        .as_ref()
                        .and_then(|a| {
                            player
                                .districts
                                .records
                                .entries
                                .iter()
                                .find(|e| e.index == i32::from(a.district as u16))
                        })
                        .map(|e| &e.value);
                    let range = definition
                        .range
                        .wrapping_add(district.map_or(0, |d| d.extra_regional_range));
                    if crate::loaded_actions::city_range_attack::plot_distance(
                        state,
                        (location.x, location.y),
                        (city.header.location_x, city.header.location_y),
                    ) <= range
                    {
                        let amount = definition
                            .amount
                            .wrapping_add(if self.fully_powered(source)? {
                                definition.powered
                            } else {
                                0
                            })
                            .wrapping_add(district.map_or(0, |d| d.extra_regional_entertainment));
                        maximum = maximum.max(amount);
                    }
                }
                result = result.wrapping_add(maximum);
            } else if building_location(city, definition.hash)?.is_some() {
                result = result.wrapping_add(definition.amount.wrapping_add(
                    if self.fully_powered(city)? {
                        definition.powered
                    } else {
                        0
                    },
                ));
            }
        }
        for &(hash, amount) in &self.districts {
            let instances: Vec<_> = city
                .districts
                .district_ids
                .iter()
                .filter_map(|id| {
                    player
                        .districts
                        .records
                        .entries
                        .iter()
                        .find(|e| e.index == i32::from(*id as u16))
                        .map(|e| &e.value)
                })
                .filter(|d| d.district_hash == hash)
                .collect();
            if instances.iter().any(|d| d.is_complete) && instances.iter().any(|d| !d.pillaged) {
                if let Some(first) = instances.first() {
                    result = result
                        .wrapping_add(first.extra_entertainment)
                        .wrapping_add(amount.max(0));
                }
            }
        }
        Ok(result.wrapping_add(self.project(city)?.map_or(0, |p| p.amenities)))
    }
    pub fn amenities(
        &self,
        state: &LoadedState,
        owner: i32,
        id: u32,
        capabilities: &Membership,
        growth_rules: &GrowthRules,
    ) -> Result<i32, Error> {
        if !capabilities.has(0xc7762424) {
            return Ok(0);
        }
        let player = player(state, owner)?;
        let city = state.city(owner, id)?;
        let growth = &city.growth;
        let mut result = player
            .resources
            .luxury_allocations
            .iter()
            .filter(|r| r.city_id == id)
            .fold(0i32, |n, _| n.wrapping_add(1));
        if growth.add_amenity_per_adjacent_luxury > 0 {
            for index in 1..=6 {
                if let Some(location) = c6_core::city_citizen_assignment::plot_location(
                    state.world().map(),
                    PlotCoord {
                        x: city.header.location_x,
                        y: city.header.location_y,
                    },
                    index,
                )
                .map_err(|_| Error::Unsupported("adjacent luxury plot"))?
                {
                    let (_, plot) = movement_world::plot_at(state.world().map(), location)
                        .map_err(|_| Error::Unsupported("adjacent luxury plot"))?;
                    if self
                        .resources
                        .iter()
                        .any(|r| r.0 == plot.resource_hash && r.1)
                    {
                        result = result.wrapping_add(1);
                    }
                }
            }
        }
        if city.resources.extra_amenity_modifier > 0 {
            for (resource_index, &(_, luxury)) in self.resources.iter().enumerate() {
                if !luxury {
                    continue;
                }
                for index in 0..c6_core::city_citizen_assignment::NUM_PLOTS as i32 {
                    let Some(location) = c6_core::city_citizen_assignment::plot_location(
                        state.world().map(),
                        PlotCoord {
                            x: city.header.location_x,
                            y: city.header.location_y,
                        },
                        index,
                    )
                    .map_err(|_| Error::Unsupported("local luxury city plot"))?
                    else {
                        continue;
                    };
                    let (_, plot) = movement_world::plot_at(state.world().map(), location)
                        .map_err(|_| Error::Unsupported("local luxury plot"))?;
                    if !plot
                        .city_attributes
                        .as_ref()
                        .is_some_and(|a| a.working_city == id && i32::from(a.player) == owner)
                    {
                        continue;
                    }
                    let resource = self
                        .resources
                        .iter()
                        .position(|r| r.0 == plot.resource_hash)
                        .map_or(-1, |i| i as i32);
                    if self
                        .resource_visibility
                        .visible_resource(state, owner, resource)?
                        != resource_index as i32
                    {
                        continue;
                    }
                    if self
                        .improvements
                        .find_index(plot.improvement_hash)
                        .is_some()
                        && !plot.flags.get(c6_core::map::PlotFlag::ImprovementPillaged)
                    {
                        result = result.wrapping_add(city.resources.extra_amenity_modifier);
                        break;
                    }
                }
            }
        }
        result = result.wrapping_add(self.entertainment(state, owner, id)?);
        for value in [
            growth.amenities_from_entertainment_modifier,
            growth.amenities_from_civics_modifier,
            growth.amenities_from_great_people_modifier,
            growth.amenities_from_city_states_modifier,
        ] {
            result = result.wrapping_add(value);
        }
        result = result.wrapping_add(growth.amenities_from_religion_modifier);
        result = result.wrapping_add(religion_amenities(
            &city.religion.religion_status,
            growth_rules
                .religions
                .entries()
                .iter()
                .map(|r| linux_type_hash(&r.type_name)),
            growth.amenities_from_religion_followers,
            growth.amenities_from_religion_minimum_followers,
        ));
        for value in [
            growth.amenities_from_national_parks_modifier,
            growth.amenities_from_starting_era_modifier,
            growth.amenities_from_improvements_modifier,
            growth.amenities_from_governors_modifier,
            growth.amenities_from_districts_modifier,
            growth.amenities_from_natural_wonders_modifier,
            growth.amenities_from_traits_modifier,
        ] {
            result = result.wrapping_add(value);
        }
        let weariness = war_weariness(&player.diplomacy.city_war_weariness, id);
        Ok(result
            .wrapping_sub(weariness)
            .wrapping_sub(player.treasury.bankruptcy_amenities))
    }
}
fn player(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::loaded_components::RetainedPlayer, Error> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("current amenities player"))
}
fn building_location(city: &City, hash: u32) -> Result<Option<i16>, Error> {
    let location = city
        .buildings
        .building_locations
        .iter()
        .rfind(|r| r.definition_hash == hash)
        .ok_or(Error::Unsupported("current amenity building presence"))?
        .value;
    if location < 0 {
        return Ok(None);
    }
    let pillaged = city
        .buildings
        .pillaged
        .iter()
        .rfind(|r| r.definition_hash == hash)
        .ok_or(Error::Unsupported("current amenity building pillage"))?
        .value;
    Ok((!pillaged).then_some(location))
}

fn war_weariness(rows: &[(u32, i32)], city: u32) -> i32 {
    rows.iter().rfind(|r| r.0 == city).map_or(0, |r| r.1)
}
fn religion_amenities(
    rows: &[c6_core::city_services::ReligionInCity],
    definitions: impl Iterator<Item = u32>,
    bonus: i32,
    minimum: i32,
) -> i32 {
    if bonus <= 0 || minimum <= 0 {
        return 0;
    }
    definitions
        .filter(|&hash| c6_core::city_religion::followers(rows, hash) >= minimum)
        .fold(0i32, |total, _| total.wrapping_add(bonus))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weariness_uses_last_full_city_identity_and_preserves_negative_value() {
        assert_eq!(
            war_weariness(&[(65536, 7), (131072, 99), (65536, -2)], 65536),
            -2
        );
        assert_eq!(war_weariness(&[(65536, 7)], 131072), 0);
    }
    #[test]
    fn followers_count_each_current_definition_at_inclusive_threshold() {
        let row = |religion_hash, followers| c6_core::city_services::ReligionInCity {
            religion_hash,
            followers,
            pressure_raw_q8: 0,
            initiation_bonus_paid: false,
        };
        let rows = vec![row(10, 3), row(20, 4), row(30, 2), row(10, 99)];
        assert_eq!(
            religion_amenities(&rows, [10, 20, 30, 40].into_iter(), 2, 3),
            4
        );
        assert_eq!(religion_amenities(&rows, [10, 20].into_iter(), 2, 0), 0);
        assert_eq!(religion_amenities(&rows, [10, 20].into_iter(), -1, 3), 0);
    }
}
