//! Source-ordered current housing from map, buildings, improvements and districts.
use super::Error;
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    unit_initialization_definitions::collection,
    Rules,
};
use c6_core::{
    loaded_state::LoadedState,
    map::{Map, PlotFlag},
    movement_world::{self, DefinitionLookup},
    unit::PlotCoord,
};
struct Terrain {
    water: bool,
    mountain: bool,
}
struct District {
    housing: i32,
    internal: bool,
    specialty: bool,
    aqueduct: bool,
    appeal: Vec<(i32, i32)>,
}
struct Improvement {
    housing: i32,
    tiles: i32,
}
pub struct Definitions {
    terrains: LinuxDefinitionRegistry,
    terrain: Vec<Terrain>,
    features: LinuxDefinitionRegistry,
    feature_fresh: Vec<bool>,
    districts: LinuxDefinitionRegistry,
    district: Vec<District>,
    improvements: LinuxDefinitionRegistry,
    improvement: Vec<Improvement>,
    no_water: i32,
    fresh_water: i32,
    coastal: i32,
    aqueduct_boost: i32,
    aqueduct_min: i32,
    lake_max: i32,
    ocean_min: i32,
}
fn sql_int(row: crate::Row<'_>, name: &'static str) -> Result<i32, Error> {
    crate::game_parameters::sql_integer(row.get(name).map_err(Error::Rules)?)
        .ok_or(Error::Unsupported(name))
}
impl Definitions {
    pub fn from_rules(rules: &Rules) -> Result<Self, Error> {
        let table = |name| rules.gameplay().table(name).ok_or(Error::Unsupported(name));
        let registry = |name, column| collection(rules, name, column).map_err(Error::Definitions);
        let terrains = registry("Terrains", "TerrainType")?;
        let terrain = terrains
            .entries()
            .iter()
            .map(|e| {
                let r = table("Terrains")?
                    .row_by_id(e.database_rowid)
                    .ok_or(Error::Unsupported("Terrain row"))?;
                Ok(Terrain {
                    water: sql_int(r, "Water")? != 0,
                    mountain: sql_int(r, "Mountain")? != 0,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let features = registry("Features", "FeatureType")?;
        let feature_fresh = features
            .entries()
            .iter()
            .map(|e| {
                let r = table("Features")?
                    .row_by_id(e.database_rowid)
                    .ok_or(Error::Unsupported("Feature row"))?;
                Ok(sql_int(r, "AddsFreshWater")? != 0)
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let districts = registry("Districts", "DistrictType")?;
        let mut appeal_rows = table("AppealHousingChanges")?
            .rows()
            .map(|r| {
                Ok((
                    r.row_id()
                        .ok_or(Error::Unsupported("AppealHousing rowid"))?,
                    r,
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        appeal_rows.sort_by_key(|(id, _)| id.0);
        let district = districts
            .entries()
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let r = table("Districts")?
                    .row_by_id(e.database_rowid)
                    .ok_or(Error::Unsupported("District row"))?;
                let hash = districts
                    .type_hash(i)
                    .ok_or(Error::Unsupported("District hash"))?;
                let mut appeal = vec![];
                for (_, row) in &appeal_rows {
                    if linux_type_hash(row.text("DistrictType").map_err(Error::Rules)?.as_bytes())
                        == hash
                    {
                        appeal.push((
                            sql_int(*row, "MinimumValue")?,
                            sql_int(*row, "AppealChange")?,
                        ));
                    }
                }
                Ok(District {
                    housing: sql_int(r, "Housing")?,
                    internal: sql_int(r, "InternalOnly")? != 0,
                    specialty: sql_int(r, "RequiresPopulation")? != 0,
                    aqueduct: sql_int(r, "Aqueduct")? != 0,
                    appeal,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let improvements = registry("Improvements", "ImprovementType")?;
        let improvement = improvements
            .entries()
            .iter()
            .map(|e| {
                let r = table("Improvements")?
                    .row_by_id(e.database_rowid)
                    .ok_or(Error::Unsupported("Improvement row"))?;
                Ok(Improvement {
                    housing: sql_int(r, "Housing")?,
                    tiles: sql_int(r, "TilesRequired")?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let parameter = |name| -> Result<i32, Error> {
            let mut rows = table("GlobalParameters")?
                .rows()
                .filter(|r| r.text("Name").ok() == Some(name));
            let r = rows.next().ok_or(Error::Unsupported(name))?;
            if rows.next().is_some() {
                return Err(Error::Unsupported("duplicate housing parameter"));
            }
            sql_int(r, "Value")
        };
        Ok(Self {
            terrains,
            terrain,
            features,
            feature_fresh,
            districts,
            district,
            improvements,
            improvement,
            no_water: parameter("CITY_POPULATION_NO_WATER")?,
            fresh_water: parameter("CITY_POPULATION_RIVER_LAKE")?,
            coastal: parameter("CITY_POPULATION_COAST")?,
            aqueduct_boost: parameter("CITY_POPULATION_AQUEDUCT_BOOST")?,
            aqueduct_min: parameter("CITY_POPULATION_AQUEDUCT_MIN")?,
            lake_max: parameter("LAKE_MAX_AREA_SIZE")?,
            ocean_min: parameter("OCEAN_MIN_WATER_SIZE")?,
        })
    }
    fn terrain(&self, map: &Map, plot: usize) -> Result<&Terrain, Error> {
        let p = map
            .plots
            .get(plot)
            .ok_or(Error::Unsupported("housing plot"))?;
        let i = self
            .terrains
            .find_index(p.terrain_hash)
            .ok_or(Error::Unsupported("housing terrain"))?;
        self.terrain
            .get(i)
            .ok_or(Error::Unsupported("housing terrain index"))
    }
    fn lake(&self, map: &Map, plot: usize) -> bool {
        let id = map.plots[plot].landmass_id;
        map.landmasses
            .entries
            .iter()
            .find(|e| e.value.id == id)
            .is_some_and(|e| e.value.water && e.value.plot_count <= self.lake_max)
    }
    pub fn fresh_water(&self, map: &Map, location: PlotCoord) -> Result<bool, Error> {
        let index =
            plot_index(map, location).map_err(|_| Error::Unsupported("freshwater location"))?;
        let p = &map.plots[index];
        let t = self.terrain(map, index)?;
        if t.water || p.flags.get(PlotFlag::Impassable) || t.mountain {
            return Ok(false);
        }
        if p.river_crossing_count > 0 {
            return Ok(true);
        }
        // Native range-one delta enumeration includes the center; these seven
        // membership queries have no side effects and only return a boolean.
        for index in 0..7 {
            let Some(location) =
                c6_core::city_citizen_assignment::plot_location(map, location, index)
                    .map_err(|_| Error::Unsupported("freshwater adjacent plot"))?
            else {
                continue;
            };
            let plot =
                plot_index(map, location).map_err(|_| Error::Unsupported("freshwater plot"))?;
            if self.lake(map, plot) {
                return Ok(true);
            }
            if let Some(i) = self.features.find_index(map.plots[plot].feature_hash) {
                if self.feature_fresh[i] {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    fn coastal(&self, map: &Map, location: PlotCoord) -> Result<bool, Error> {
        let index =
            plot_index(map, location).map_err(|_| Error::Unsupported("coastal location"))?;
        if self.terrain(map, index)?.water {
            return Ok(false);
        }
        for i in 1..7 {
            let Some(p) = c6_core::city_citizen_assignment::plot_location(map, location, i)
                .map_err(|_| Error::Unsupported("coastal adjacency"))?
            else {
                continue;
            };
            let p = plot_index(map, p).map_err(|_| Error::Unsupported("coastal plot"))?;
            if self.terrain(map, p)?.water
                && !map.plots[p].flags.get(PlotFlag::Impassable)
                && map
                    .landmasses
                    .entries
                    .iter()
                    .find(|e| e.value.id == map.plots[p].landmass_id)
                    .is_some_and(|e| e.value.plot_count >= self.ocean_min)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
    fn district<'s>(
        &self,
        state: &'s LoadedState,
        owner: i32,
        id: u32,
    ) -> Option<&'s c6_core::player_districts::District> {
        if id == u32::MAX {
            return None;
        }
        state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)?
            .districts
            .records
            .entries
            .iter()
            .find(|e| e.index as u32 == id & 0xffff)
            .map(|e| &e.value)
    }
    fn has_aqueduct(&self, state: &LoadedState, location: PlotCoord) -> Result<bool, Error> {
        let map = state.world().map();
        for i in 1..7 {
            let Some(p) = c6_core::city_citizen_assignment::plot_location(map, location, i)
                .map_err(|_| Error::Unsupported("aqueduct adjacent plot"))?
            else {
                continue;
            };
            let p = plot_index(map, p).map_err(|_| Error::Unsupported("aqueduct plot"))?;
            let Some(a) = &map.plots[p].city_attributes else {
                continue;
            };
            let Some(d) = self.district(state, i32::from(a.player), a.district) else {
                continue;
            };
            if !d.is_complete || d.pillaged {
                continue;
            }
            let i = self
                .districts
                .find_index(d.district_hash)
                .ok_or(Error::Unsupported("aqueduct district"))?;
            if self.district[i].aqueduct {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub fn from_water(&self, state: &LoadedState, owner: i32, city: u32) -> Result<i32, Error> {
        let c = state.city(owner, city)?;
        let location = PlotCoord {
            x: c.header.location_x,
            y: c.header.location_y,
        };
        let map = state.world().map();
        if plot_index(map, location).is_err() {
            return Ok(0);
        }
        let p = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Unsupported("housing player"))?;
        let mut result = self.no_water;
        if c.growth.no_fresh_water_housing <= 0 {
            if p.cities.fresh_water_housing_bonus || self.fresh_water(map, location)? {
                result = self.fresh_water;
            } else if self.coastal(map, location)? {
                result = self.coastal;
            }
        }
        if self.has_aqueduct(state, location)? {
            result = result
                .wrapping_add(self.aqueduct_boost)
                .max(self.aqueduct_min);
        }
        Ok(result.wrapping_add(c.growth.housing_from_water_modifier))
    }
    pub fn from_buildings(
        &self,
        state: &LoadedState,
        owner: i32,
        city: u32,
        definitions: &crate::city_production_turn::buildings::Definitions,
    ) -> Result<i32, Error> {
        definitions
            .validate(state)
            .map_err(|_| Error::Unsupported("housing building definitions"))?;
        let c = state.city(owner, city)?;
        let mut result = 0i32;
        for d in &definitions.buildings {
            let location = c
                .buildings
                .building_locations
                .iter()
                .rfind(|p| p.definition_hash == d.hash)
                .ok_or(Error::Unsupported("current building presence"))?
                .value;
            if location < 0 {
                continue;
            }
            let pillaged = c
                .buildings
                .pillaged
                .iter()
                .rfind(|p| p.definition_hash == d.hash)
                .ok_or(Error::Unsupported("current building pillage"))?
                .value;
            if !pillaged {
                result = result.wrapping_add(d.housing.max(0));
            }
        }
        Ok(result.wrapping_add(c.growth.housing_from_buildings_modifier))
    }
    pub fn from_improvements(
        &self,
        state: &LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<i32, Error> {
        let c = state.city(owner, city)?;
        let map = state.world().map();
        let center = PlotCoord {
            x: c.header.location_x,
            y: c.header.location_y,
        };
        let mut result = 0i32;
        let mut divisor = -1;
        for i in 0..c6_core::city_citizen_assignment::NUM_PLOTS as i32 {
            let Some(p) = c6_core::city_citizen_assignment::plot_location(map, center, i)
                .map_err(|_| Error::Unsupported("improvement housing city plot"))?
            else {
                continue;
            };
            let p = &map.plots
                [plot_index(map, p).map_err(|_| Error::Unsupported("housing improvement plot"))?];
            if !p
                .city_attributes
                .as_ref()
                .is_some_and(|a| a.working_city == city && i32::from(a.player) == owner)
                || p.flags.get(PlotFlag::ImprovementPillaged)
            {
                continue;
            }
            let Some(i) = self.improvements.find_index(p.improvement_hash) else {
                continue;
            };
            let d = &self.improvement[i];
            if d.housing > 0 {
                result = result.wrapping_add(d.housing);
                divisor = d.tiles;
            }
        }
        if divisor > 1 {
            result /= divisor;
        }
        Ok(result.wrapping_add(c.growth.housing_from_improvements_modifier))
    }
    pub fn from_districts(&self, state: &LoadedState, owner: i32, city: u32) -> Result<i32, Error> {
        let c = state.city(owner, city)?;
        let mut count = 0i32;
        let mut base = 0i32;
        for &id in &c.districts.district_ids {
            let Some(d) = self.district(state, owner, id) else {
                continue;
            };
            let i = self
                .districts
                .find_index(d.district_hash)
                .ok_or(Error::Unsupported("housing district definition"))?;
            let definition = &self.district[i];
            if !definition.internal && d.is_complete && definition.specialty && !d.pillaged {
                count = count.wrapping_add(1);
            }
            if !d.is_complete || d.pillaged || definition.housing == 0 {
                continue;
            }
            let location = PlotCoord {
                x: d.location.0,
                y: d.location.1,
            };
            let p = &state.world().map().plots[plot_index(state.world().map(), location)
                .map_err(|_| Error::Unsupported("housing district plot"))?];
            if !p.city_attributes.as_ref().is_some_and(|a| a.player != -1) {
                return Err(Error::Unsupported("current unowned district appeal"));
            }
            let appeal = i32::from(p.appeal);
            let adjustment = definition
                .appeal
                .iter()
                .find(|r| appeal >= r.0)
                .map_or(0, |r| r.1);
            base = base.wrapping_add(definition.housing.wrapping_add(adjustment));
        }
        Ok(c.header
            .housing_per_district
            .wrapping_mul(count)
            .wrapping_add(base)
            .wrapping_add(c.growth.housing_from_districts_modifier))
    }
    pub fn housing(
        &self,
        state: &LoadedState,
        owner: i32,
        city: u32,
        buildings: &crate::city_production_turn::buildings::Definitions,
    ) -> Result<i32, Error> {
        let mut result = self.from_water(state, owner, city)?;
        result = result.wrapping_add(self.from_buildings(state, owner, city, buildings)?);
        result = result.wrapping_add(self.from_improvements(state, owner, city)?);
        result = result.wrapping_add(self.from_districts(state, owner, city)?);
        let g = &state.city(owner, city)?.growth;
        for value in [
            g.housing_from_civics_modifier,
            g.housing_from_great_people_modifier,
            g.housing_from_starting_era_modifier,
            g.housing_from_great_works_modifier,
        ] {
            result = result.wrapping_add(value);
        }
        Ok(result.max(0))
    }
}

fn plot_index(map: &Map, location: PlotCoord) -> Result<usize, movement_world::MissingInput> {
    movement_world::plot_at(map, location).map(|(index, _)| index)
}
