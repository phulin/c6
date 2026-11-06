//! External ordinary-building selection over current canonical City state.
//!
//! This implements the fresh, non-placement CanConstruct path. Repair, wonders,
//! limited-instance and additional prerequisite branches remain explicit errors.
//! Replacement maps and fallout are current gameplay inputs, never legality
//! answers captured from the reference engine.
use super::{buildings, Error};
use crate::{registry::linux_type_hash, Rules, SqlValue};
use c6_core::{
    city_production::{Order, QueueEntry},
    core_player::initialization::InputIdentities,
    loaded_state::LoadedState,
    map::Component,
    unit::HashValue,
};

struct Building {
    hash: u32,
    internal_only: bool,
    unsupported: bool,
    prerequisite_technology: Option<usize>,
}
pub struct Definitions {
    inputs: InputIdentities,
    buildings: Vec<Building>,
}
/// Current player-specific native replacement vector, expressed as hashes.
/// None is the actual native invalid type, not an unavailable vector element.
pub struct DistrictMap {
    pub owner: i32,
    pub values: Vec<HashValue<Option<u32>>>,
}
pub struct Current<'a> {
    pub buildings: &'a [super::building_callbacks::BuildingMap],
    pub districts: &'a [DistrictMap],
    /// Game.Fallout's current ordered Instance.turns_remaining values.
    pub fallout_turns: &'a [i32],
}
pub trait Queries {
    /// Rules.Movement.IsOccupiedByEnemy(owner,plot,true), over the same World.
    fn occupied_by_enemy(&self, state: &LoadedState, owner: i32, plot: i32) -> Result<bool, Error>;
}
fn int(row: crate::Row<'_>, column: &'static str) -> Result<i32, Error> {
    match row.get(column).map_err(Error::Rules)? {
        SqlValue::Integer(v) => Ok(*v as i32),
        SqlValue::Null => Ok(0),
        _ => Err(Error::Unsupported(column)),
    }
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let table = |name| rules.gameplay().table(name).ok_or(Error::Unsupported(name));
        let technologies = crate::unit_initialization_definitions::collection(
            rules,
            "Technologies",
            "TechnologyType",
        )
        .map_err(Error::Definitions)?;
        let mut buildings = Vec::new();
        for row in table("Buildings")?.rows() {
            let name = row.text("BuildingType").map_err(Error::Rules)?;
            let hash = linux_type_hash(name.as_bytes());
            let mut unsupported = false;
            for column in [
                "PrereqCivic",
                "AdjacentDistrict",
                "CityAdjacentTerrain",
                "GovernmentTierRequirement",
            ] {
                unsupported |= !matches!(row.get(column).map_err(Error::Rules)?, SqlValue::Null);
            }
            for column in [
                "RequiresPlacement",
                "IsWonder",
                "EnabledByReligion",
                "RequiresReligion",
                "RequiresAdjacentRiver",
                "OuterDefenseStrength",
            ] {
                unsupported |= int(row, column)? != 0;
            }
            unsupported |=
                int(row, "MaxPlayerInstances")? != -1 || int(row, "MaxWorldInstances")? != -1;
            for prerequisite in table("BuildingPrereqs")?.rows() {
                unsupported |= prerequisite.text("Building").map_err(Error::Rules)? == name;
            }
            for xp in table("Buildings_XP2")?.rows() {
                if xp.text("BuildingType").map_err(Error::Rules)? == name {
                    unsupported |= int(xp, "BlocksCoastalFlooding")? != 0;
                }
            }
            buildings.push(Building {
                hash,
                internal_only: int(row, "InternalOnly")? != 0,
                unsupported,
                prerequisite_technology: match row.get("PrereqTech").map_err(Error::Rules)? {
                    SqlValue::Text(name) => technologies.find_index(linux_type_hash(name)),
                    SqlValue::Null => None,
                    _ => return Err(Error::Unsupported("building prerequisite technology")),
                },
            });
        }
        Ok(Self {
            inputs: state.binding().inputs.clone(),
            buildings,
        })
    }
}
fn mapped(values: &[HashValue<Option<u32>>], hash: u32) -> Result<Option<u32>, Error> {
    let mut found = values.iter().filter(|v| v.definition_hash == hash);
    let value = found
        .next()
        .ok_or(Error::Unsupported("current replacement map entry"))?
        .value;
    if found.next().is_some() {
        return Err(Error::Unsupported("ambiguous replacement map entry"));
    }
    Ok(value)
}
/// Source284e410 with all prerequisite-ignore flags false and front-check true.
/// Unknown branches return an error; a definite illegal choice returns false.
#[allow(clippy::too_many_arguments)]
pub fn can_construct(
    state: &LoadedState,
    owner: i32,
    city: u32,
    hash: u32,
    definitions: &Definitions,
    buildings: &buildings::Definitions,
    current: &Current<'_>,
    queries: &impl Queries,
) -> Result<bool, Error> {
    if definitions.inputs != state.binding().inputs {
        return Err(Error::Binding);
    }
    buildings.validate(state)?;
    let value = state.city(owner, city)?;
    if value.header.delayed_death {
        return Ok(false);
    }
    let Some(definition) = definitions.buildings.iter().find(|v| v.hash == hash) else {
        return Ok(false);
    };
    let building = buildings.building(hash)?;
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("current production owner"))?;
    let map = current
        .buildings
        .iter()
        .find(|v| v.owner == owner)
        .ok_or(Error::Unsupported("current building replacement map"))?;
    if definition.internal_only
        || mapped(&map.values, hash)? != Some(hash)
        || player.cities.banned_building_hashes.contains(&hash)
        || building.capital
    {
        return Ok(false);
    }
    if building
        .prerequisite_district
        .is_some_and(|h| player.cities.banned_district_building_hashes.contains(&h))
    {
        return Ok(false);
    }
    if definition.unsupported || !building.mutually_exclusive.is_empty() {
        return Err(Error::Unsupported(
            "additional building selection prerequisites",
        ));
    }
    //284e55e..284e59a: a resolved technology reference tests the current
    // owner's acquired byte. No automatic research or prerequisite policy runs.
    if let Some(technology) = definition.prerequisite_technology {
        if !super::unit_selection::acquired(state, owner, technology, false)? {
            return Err(Error::Unsupported(
                "unfulfilled building technology prerequisite",
            ));
        }
    }
    buildings.start_building_progress(state, hash)?; // Reject unimplemented resource reservation before mutation.
    let location = value
        .buildings
        .building_locations
        .iter()
        .find(|v| v.definition_hash == hash)
        .ok_or(Error::Unsupported("current building presence domain"))?
        .value;
    if location != -1 {
        return Err(Error::Unsupported("building repair selection"));
    }
    let progress = value
        .build_queue
        .building_progress
        .iter()
        .find(|v| v.definition_hash == hash)
        .ok_or(Error::MissingProgress(Order::Construct {
            building_hash: hash,
        }))?
        .value;
    if progress != 0 {
        return Err(Error::Unsupported("resumed building selection"));
    }
    if let Some(required) = building.prerequisite_district {
        let map = current
            .districts
            .iter()
            .find(|v| v.owner == owner)
            .ok_or(Error::Unsupported("current district replacement map"))?;
        let Some(required) = mapped(&map.values, required)? else {
            return Ok(false);
        };
        let mut found = None;
        for id in &value.districts.district_ids {
            let district = state.district(owner, *id).map_err(Error::District)?;
            if district.district_hash == required {
                if found.is_some() {
                    return Err(Error::Unsupported("multiple prerequisite districts"));
                }
                found = Some(district);
            }
        }
        let Some(district) = found else {
            return Ok(false);
        };
        if !district.is_complete || district.pillaged {
            return Ok(false);
        }
        let plot = super::buildings::plot_index(state, [district.location.0, district.location.1])?;
        if current
            .fallout_turns
            .get(plot as usize)
            .is_some_and(|v| *v > 0)
            || queries.occupied_by_enemy(state, owner, plot)?
        {
            return Ok(false);
        }
        let mut terrain = state
            .world()
            .map()
            .components
            .iter()
            .filter_map(|c| match c {
                Component::Terrain(t) => Some(t),
                _ => None,
            });
        let terrain_value = terrain
            .next()
            .ok_or(Error::Unsupported("current coastal lowlands"))?;
        if terrain.next().is_some() {
            return Err(Error::Unsupported("ambiguous coastal lowlands"));
        }
        if terrain_value
            .coastal_lowlands
            .iter()
            .any(|v| v.plot_index == plot && v.flooded)
        {
            return Ok(false);
        }
    }
    Ok(true)
}
/// Resource-free ordinary OnNewFrontBuilding, after the caller's real legality
/// check. PreCreate uses the current prerequisite district, not request's invalid
/// placement coordinates. Report consumers remain the caller's shared service.
pub fn new_front(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    definitions: &buildings::Definitions,
    effects: &mut impl buildings::Effects,
) -> Result<(), Error> {
    let entry = state
        .city(owner, city)?
        .build_queue
        .entries
        .first()
        .cloned()
        .ok_or(Error::Unsupported("empty new-front building"))?;
    let Order::Construct { building_hash } = entry.order else {
        return Err(Error::Unsupported("new-front order"));
    };
    if entry.flags != 0 {
        return Err(Error::Unsupported("nonordinary new-front building flags"));
    }
    let building = definitions.building(building_hash)?;
    definitions.start_building_progress(state, building_hash)?;
    let location = buildings::pre_create(state, owner, city, building, None, effects)?
        .ok_or(Error::IllegalSelection(entry.order))?;
    effects.report(
        state,
        owner,
        city,
        building_hash,
        location,
        buildings::Report::ProductionChanged,
    )?;
    effects.report(
        state,
        owner,
        city,
        building_hash,
        location,
        buildings::Report::QueueEntryChanged { index: 0 },
    )?;
    Ok(())
}

/// Validate and append an external building decision using the actual request
/// defaults. No progress or overflow is consumed by queue selection.
#[allow(clippy::too_many_arguments)]
pub fn append_building(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    hash: u32,
    definitions: &Definitions,
    buildings: &buildings::Definitions,
    current: &Current<'_>,
    queries: &impl Queries,
    effects: &mut impl super::QueueEffects,
) -> Result<(), Error> {
    if !can_construct(
        state,
        owner,
        city,
        hash,
        definitions,
        buildings,
        current,
        queries,
    )? {
        return Err(Error::IllegalSelection(Order::Construct {
            building_hash: hash,
        }));
    }
    super::append_selected(
        state,
        owner,
        city,
        QueueEntry::construct_request(hash),
        effects,
    )
}
