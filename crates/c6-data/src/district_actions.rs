//! External fresh ordinary district choices on the canonical City/Map state.
//! Campus is the initial supported type; unsupported placement branches fail
//! explicitly before selection rather than being treated as legal no-ops.
use crate::{
    city_production_turn::{self as production, selection},
    district_initialization,
    registry::linux_type_hash,
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::collection,
    Rules, SqlValue,
};
use c6_core::{
    city_production::{Order, QueueEntry, BUILD_TYPE_HASH},
    core_player::initialization::WireBinding,
    game_national_parks::ParkData,
    loaded_state::LoadedState,
    map::{Component, PlotFlag},
    unit::PlotCoord,
};
#[derive(Clone, Copy)]
struct Terrain {
    hash: u32,
    water: bool,
    mountain: bool,
    impassable: bool,
}
pub struct Definitions {
    binding: WireBinding,
    pub district_initialization: district_initialization::Definitions,
    pub campus_hash: u32,
    pub campus_index: i32,
    prerequisite_technology: Option<usize>,
    prerequisite_civic: Option<usize>,
    population_per_district: i32,
    specialty: Vec<u32>,
    terrains: Vec<Terrain>,
}
fn missing(s: &'static str) -> production::Error {
    production::Error::Unsupported(s)
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, production::Error> {
        let table = |name| rules.gameplay().table(name).ok_or(missing(name));
        let districts = collection(rules, "Districts", "DistrictType")
            .map_err(production::Error::Definitions)?;
        let techs = collection(rules, "Technologies", "TechnologyType")
            .map_err(production::Error::Definitions)?;
        let civics =
            collection(rules, "Civics", "CivicType").map_err(production::Error::Definitions)?;
        let hash = linux_type_hash(b"DISTRICT_CAMPUS");
        let native = districts
            .find_index(hash)
            .ok_or(missing("Campus definition"))?;
        let definition = districts
            .at(native as u32)
            .ok_or(missing("Campus definition index"))?;
        let row = table("Districts")?
            .row_by_id(definition.database_rowid)
            .ok_or(missing("Campus rule row"))?;
        for key in [
            "Coast",
            "InternalOnly",
            "CityCenter",
            "Aqueduct",
            "AdjacentToLand",
            "NoAdjacentCity",
        ] {
            if row.integer(key).map_err(production::Error::Rules)? != 0 {
                return Err(missing("additional Campus placement rule"));
            }
        }
        for key in ["RequiresPopulation", "RequiresPlacement", "OnePerCity"] {
            if row.integer(key).map_err(production::Error::Rules)? != 1 {
                return Err(missing("ordinary Campus rule shape"));
            }
        }
        match row.get("MaxPerPlayer").map_err(production::Error::Rules)? {
            SqlValue::Integer(-1) => {}
            SqlValue::RealBits(bits) if *bits == (-1.0f64).to_bits() => {}
            _ => return Err(missing("limited Campus per-Player rule")),
        }
        if !matches!(
            row.get("TraitType").map_err(production::Error::Rules)?,
            SqlValue::Null
        ) {
            return Err(missing("Campus required trait"));
        }
        for (name, column) in [
            ("District_RequiredFeatures", "DistrictType"),
            ("District_ValidTerrains", "DistrictType"),
            ("MutuallyExclusiveDistricts", "District"),
        ] {
            if table(name)?
                .rows()
                .any(|r| r.text(column).ok() == Some("DISTRICT_CAMPUS"))
            {
                return Err(missing("additional Campus terrain/exclusive reference"));
            }
        }
        let prerequisite = |key,
                            registry: &crate::registry::LinuxDefinitionRegistry|
         -> Result<Option<usize>, production::Error> {
            match row.get(key).map_err(production::Error::Rules)? {
                SqlValue::Null => Ok(None),
                SqlValue::Text(name) => registry
                    .find_index(linux_type_hash(name))
                    .map(Some)
                    .ok_or(missing("Campus prerequisite mapping")),
                _ => Err(missing("Campus prerequisite type")),
            }
        };
        let technology = prerequisite("PrereqTech", &techs)?;
        let civic = prerequisite("PrereqCivic", &civics)?;
        let mut specialty = Vec::new();
        for row in table("Districts")?.rows() {
            if row
                .integer("RequiresPopulation")
                .map_err(production::Error::Rules)?
                != 0
            {
                specialty.push(linux_type_hash(
                    row.text("DistrictType")
                        .map_err(production::Error::Rules)?
                        .as_bytes(),
                ));
            }
        }
        let mut terrains = Vec::new();
        for row in table("Terrains")?.rows() {
            terrains.push(Terrain {
                hash: linux_type_hash(
                    row.text("TerrainType")
                        .map_err(production::Error::Rules)?
                        .as_bytes(),
                ),
                water: row.integer("Water").map_err(production::Error::Rules)? != 0,
                mountain: row.integer("Mountain").map_err(production::Error::Rules)? != 0,
                impassable: row
                    .integer("Impassable")
                    .map_err(production::Error::Rules)?
                    != 0,
            });
        }
        let population = table("GlobalParameters")?
            .rows()
            .find(|r| r.text("Name").ok() == Some("DISTRICT_POPULATION_REQUIRED_PER"))
            .ok_or(missing("district population parameter"))?
            .text("Value")
            .map_err(production::Error::Rules)?
            .parse()
            .map_err(|_| missing("district population parameter value"))?;
        Ok(Self {
            binding: state.binding().clone(),
            district_initialization: district_initialization::Definitions::from_rules(
                state, rules, bound,
            )
            .map_err(|_| missing("district initialization definitions"))?,
            campus_hash: hash,
            campus_index: native as i32,
            prerequisite_technology: technology,
            prerequisite_civic: civic,
            population_per_district: population,
            specialty,
            terrains,
        })
    }
}
pub struct Current<'a> {
    pub district_maps: &'a [selection::DistrictMap],
    pub national_parks: &'a [ParkData],
    pub fallout_turns: &'a [i32],
}
fn player(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::loaded_components::RetainedPlayer, production::Error> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(missing("district Player"))
}
fn acquired(
    state: &LoadedState,
    owner: i32,
    index: usize,
    civic: bool,
) -> Result<bool, production::Error> {
    let p = state
        .world()
        .player(owner)
        .map_err(|_| missing("district prerequisite Player"))?;
    let mask = if civic { &p.civics } else { &p.technologies };
    mask.as_ref()
        .and_then(|m| m.get(index))
        .copied()
        .ok_or(missing("district prerequisite flags"))
}
/// Native2885620; a nonpositive divisor returns1 without reading population or extra.
fn population_capacity(population: i32, extra: i32, per: i32) -> i32 {
    if per < 1 {
        1
    } else {
        per.wrapping_add(population)
            .wrapping_sub(1)
            .wrapping_div(per)
            .wrapping_add(extra)
    }
}
pub fn can_choose(
    state: &LoadedState,
    owner: i32,
    city: u32,
    hash: u32,
    location: PlotCoord,
    defs: &Definitions,
    current: &Current<'_>,
) -> Result<bool, production::Error> {
    if state.binding() != &defs.binding {
        return Err(production::Error::Binding);
    }
    if hash != defs.campus_hash {
        return Err(missing("district action type other than Campus"));
    }
    let p = player(state, owner)?;
    let value = state.city(owner, city)?;
    if value.header.delayed_death {
        return Ok(false);
    }
    if let Some(i) = defs.prerequisite_technology {
        if !acquired(state, owner, i, false)? {
            return Ok(false);
        }
    }
    if let Some(i) = defs.prerequisite_civic {
        if !acquired(state, owner, i, true)? {
            return Ok(false);
        }
    }
    let map = current
        .district_maps
        .iter()
        .find(|m| m.owner == owner)
        .ok_or(missing("current district replacement map"))?;
    if map
        .values
        .iter()
        .rfind(|r| r.definition_hash == hash)
        .ok_or(missing("current Campus replacement cell"))?
        .value
        != Some(hash)
    {
        return Ok(false);
    }
    if p.cities.banned_district_hashes.contains(&hash) {
        return Ok(false);
    }
    let mut count = 0i32;
    for id in &value.districts.district_ids {
        let district = state
            .district(owner, *id)
            .map_err(production::Error::District)?;
        if district.district_hash == hash {
            if district.is_complete {
                return Ok(false);
            }
            return Err(missing("existing Campus resume/repair selection"));
        }
        if defs.specialty.contains(&district.district_hash) {
            count = count.wrapping_add(1);
        }
    }
    if count
        >= population_capacity(
            value.header.population,
            value.districts.extra_districts,
            defs.population_per_district,
        )
    {
        return Ok(false);
    }
    let map = state.world().map();
    if location.x < 0
        || location.y < 0
        || location.x >= map.header.width
        || location.y >= map.header.height
    {
        if map.header.wrap_x || map.header.wrap_y {
            return Err(missing("noncanonical Campus request coordinate"));
        }
        return Ok(false);
    }
    let Some(plot) = map.plot_at(location.x, location.y) else {
        return Ok(false);
    };
    if plot.flags.get(PlotFlag::HasCityAttributes) != plot.city_attributes.is_some() {
        return Err(missing("district plot attribute state"));
    }
    let index = location
        .y
        .wrapping_mul(map.header.width)
        .wrapping_add(location.x);
    let Some(attributes) = plot.city_attributes.as_ref() else {
        return Ok(false);
    };
    if i32::from(attributes.player) != owner || attributes.owning_city != city {
        return Ok(false);
    }
    if crate::loaded_actions::city_range_attack::plot_distance(
        state,
        (value.header.location_x, value.header.location_y),
        (location.x, location.y),
    ) >= 4
    {
        return Ok(false);
    }
    if attributes.district != u32::MAX {
        return Ok(false);
    }
    if attributes.wonder_building_hash != u32::MAX {
        return Err(missing("district placement on wonder construction"));
    }
    if current
        .national_parks
        .iter()
        .any(|p| p.plot_indices.contains(&index))
    {
        return Ok(false);
    }
    let terrain = map
        .components
        .iter()
        .find_map(|c| match c {
            Component::Terrain(t) => Some(t),
            _ => None,
        })
        .ok_or(missing("current district terrain manager"))?;
    if terrain
        .coastal_lowlands
        .iter()
        .any(|r| r.plot_index == index && r.flooded)
    {
        return Ok(false);
    }
    if current
        .fallout_turns
        .get(index as usize)
        .copied()
        .ok_or(missing("current district Fallout"))?
        > 0
    {
        return Ok(false);
    }
    let terrain = defs
        .terrains
        .iter()
        .find(|t| t.hash == plot.terrain_hash)
        .ok_or(missing("current district Terrain"))?;
    if terrain.water
        || terrain.mountain
        || terrain.impassable
        || plot.flags.get(PlotFlag::Impassable)
    {
        return Ok(false);
    }
    if plot.feature_hash != u32::MAX
        || plot.resource_hash != u32::MAX
        || plot.improvement_hash != u32::MAX
    {
        return Err(missing("Campus feature/resource/improvement removal"));
    }
    if p.districts
        .feature_required_for_specialty_districts
        .iter()
        .any(|r| r.value > 0)
    {
        return Err(missing("player specialty-district required feature"));
    }
    if p.districts
        .district_cannot_be_built_adjacent_to_city
        .iter()
        .any(|r| r.definition_hash == hash && r.value > 0)
    {
        return Err(missing("player district adjacent-city restriction"));
    }
    Ok(true)
}
/// Cost is locked in the existing District at placement; otherwise use the
/// current canonical Player.Districts cost, not a second rules-derived owner.
pub fn cost(
    state: &LoadedState,
    owner: i32,
    city: u32,
    hash: u32,
) -> Result<i32, production::Error> {
    for id in &state.city(owner, city)?.districts.district_ids {
        let d = state
            .district(owner, *id)
            .map_err(production::Error::District)?;
        if d.district_hash == hash && d.cost > 0 {
            return Ok(d.cost);
        }
    }
    player(state, owner)?
        .districts
        .current_district_cost
        .iter()
        .rfind(|r| r.definition_hash == hash)
        .map(|r| r.value)
        .ok_or(missing("current district cost"))
}
#[allow(clippy::too_many_arguments)]
pub fn choose(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    hash: u32,
    location: PlotCoord,
    defs: &Definitions,
    current: &Current<'_>,
    effects: &mut impl production::QueueEffects,
) -> Result<(), production::Error> {
    if !can_choose(state, owner, city, hash, location, defs, current)? {
        return Err(production::Error::IllegalSelection(Order::Zone {
            district_hash: hash,
        }));
    }
    production::append_selected(
        state,
        owner,
        city,
        QueueEntry {
            type_hash: BUILD_TYPE_HASH,
            node_type_hash: BUILD_TYPE_HASH,
            order: Order::Zone {
                district_hash: hash,
            },
            flags: 0,
            request_id: 0,
            location: [location.x, location.y],
        },
        effects,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn population_district_thresholds_preserve_native_rounding() {
        assert_eq!(
            [1, 3, 4, 6, 7].map(|p| population_capacity(p, 0, 3)),
            [1, 1, 2, 2, 3]
        );
        assert_eq!(population_capacity(4, 2, 3), 4);
    }
    #[test]
    fn disabled_population_divisor_ignores_extra_districts() {
        assert_eq!(population_capacity(100, 8, 0), 1);
        assert_eq!(population_capacity(-4, -8, -1), 1);
    }
}
