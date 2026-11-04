//! Reached ordinary BuildingConstructed predicates over canonical current state.
//!
//! Actual subscription bindings and current replacement maps are inputs. These
//! are gameplay predicates, not native Test answers. A positive unsupported
//! branch fails explicitly and must use the shared trigger/factory service.
use super::{buildings, Error};
use crate::{registry::linux_type_hash, Rules, RulesRowId, SqlValue};
use c6_core::{loaded_state::LoadedState, simple_requirement::Requirement, unit::HashValue};
#[derive(Clone, Copy, Debug)]
pub enum BoostPredicate {
    WonderCount,
    WonderPastEra,
    BuildingCount,
    Building,
    BuildingMountain,
}
#[derive(Clone, Copy, Debug)]
pub struct BoostBinding {
    pub definition: RulesRowId,
    pub predicate: BoostPredicate,
}
#[derive(Debug)]
pub struct BuildingMap {
    pub owner: i32,
    pub values: Vec<HashValue<Option<u32>>>,
}
/// Borrow from the same live requirement owner, or from explicitly supplied
/// initial records. This module creates no second requirement store/counter.
pub struct BuildingRequirement<'a> {
    pub owner: i32,
    pub building: Option<u32>,
    pub value: &'a Requirement,
}
pub struct Current<'a> {
    pub boosts: &'a [BoostBinding],
    pub building_maps: &'a [BuildingMap],
    pub player_event_owners: &'a [i32],
    pub requirements: &'a [BuildingRequirement<'a>],
}
#[derive(Clone, Copy, Debug)]
pub struct Event {
    pub owner: i32,
    pub city: u32,
    pub building: u32,
}
fn integer(value: &SqlValue) -> Result<i32, Error> {
    match value {
        SqlValue::Integer(v) => Ok(*v as i32),
        _ => Err(Error::Unsupported("current integer definition field")),
    }
}
fn present(city: &c6_core::city::City, hash: u32) -> Result<bool, Error> {
    let location = city
        .buildings
        .building_locations
        .iter()
        .find(|v| v.definition_hash == hash)
        .ok_or(Error::Unsupported("current building presence domain"))?
        .value;
    if location < -1 {
        return Err(Error::Unsupported("negative building location"));
    }
    Ok(location != -1)
}
fn count(state: &LoadedState, owner: i32, hash: u32) -> Result<i32, Error> {
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("current building count owner"))?;
    let mut n = 0i32;
    for city in &player.cities.records.entries {
        n = n.wrapping_add(i32::from(present(&city.value, hash)?));
    }
    Ok(n)
}
/// Continue after global Stats, in native order: Boosts, Player EventManagers,
/// PlayerHasBuilding invalidations, then the bounded achievement predicates.
/// This supports the actual ordinary early-city path; positive boosts, awards,
/// policy unlocks or changed requirements require their shared continuations.
pub fn after_stats(
    state: &LoadedState,
    event: Event,
    defs: &buildings::Definitions,
    rules: &Rules,
    current: &Current<'_>,
) -> Result<(), Error> {
    let Event {
        owner,
        city,
        building,
    } = event;
    defs.validate(state)?;
    let own = state.city(owner, city)?;
    let definition = defs
        .buildings
        .iter()
        .find(|v| v.hash == building)
        .ok_or(Error::Unsupported("current building definition"))?;
    let boosts = rules
        .gameplay()
        .table("Boosts")
        .ok_or(Error::Unsupported("Boosts"))?;
    let map = current
        .building_maps
        .iter()
        .find(|v| v.owner == owner)
        .ok_or(Error::Unsupported("current owner building replacement map"))?;
    let mut wonders = 0i32;
    for def in defs.buildings.iter().filter(|v| v.wonder) {
        wonders = wonders.wrapping_add(count(state, owner, def.hash)?);
    }
    for binding in current.boosts {
        let row = boosts
            .row_by_id(binding.definition)
            .ok_or(Error::Unsupported("current Boost definition"))?;
        let num = integer(row.get("NumItems").map_err(Error::Rules)?)?;
        let building_ref = match row.get("BuildingType").map_err(Error::Rules)? {
            SqlValue::Null => None,
            SqlValue::Text(v) => Some(linux_type_hash(v)),
            _ => return Err(Error::Unsupported("Boost building reference")),
        };
        let mapped = if let Some(hash) = building_ref {
            map.values
                .iter()
                .find(|v| v.definition_hash == hash)
                .ok_or(Error::Unsupported("Boost building replacement input"))?
                .value
        } else {
            None
        };
        match binding.predicate {
            BoostPredicate::WonderCount if wonders >= num => {
                return Err(Error::Unsupported("positive wonder-count Boost"))
            }
            BoostPredicate::WonderPastEra if definition.wonder => {
                return Err(Error::Unsupported("positive wonder-era Boost"))
            }
            BoostPredicate::BuildingCount
            | BoostPredicate::Building
            | BoostPredicate::BuildingMountain
                if mapped == Some(building) =>
            {
                return Err(Error::Unsupported("positive building Boost"))
            }
            _ => {}
        }
    }
    // 2b018d0 visits Resources, Culture, Districts, Techs, Culture for every
    // subscribed Player manager. Each child first checks current City owner.
    if definition.requires_placement || definition.wonder {
        return Err(Error::Unsupported("positive player building effect"));
    }
    let row = rules
        .gameplay()
        .table("Buildings")
        .ok_or(Error::Unsupported("Buildings"))?
        .rows()
        .find(|r| {
            r.text("BuildingType")
                .ok()
                .map(|v| linux_type_hash(v.as_bytes()))
                == Some(building)
        })
        .ok_or(Error::Unsupported("current building row"))?;
    if integer(row.get("UnlocksGovernmentPolicy").map_err(Error::Rules)?)? != 0 {
        return Err(Error::Unsupported("positive government policy unlock"));
    }
    for &manager_owner in current.player_event_owners {
        if manager_owner != owner {
            continue;
        }
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Unsupported("current district owner"))?;
        // Writer2ae009c serializes FAutoVariable+268; its payload+10 is the
        // exact vector+278 traversed by Create...2ae1fba. Version4 omits it.
        if player.districts.wire_version != 8
            || !player
                .districts
                .district_and_buildings_creation_units
                .is_empty()
        {
            return Err(Error::Unsupported("district/building unit ability award"));
        }
    }
    for binding in current.requirements {
        let value = binding.value;
        if binding.owner != owner || !value.active {
            continue;
        }
        let tested = if let Some(hash) = binding.building {
            if count(state, owner, hash)? > 0 {
                2
            } else {
                1
            }
        } else {
            1
        };
        let tested = if value.inverse {
            c6_core::melee_attack::proposals::subject_sets::inverse(tested)
        } else {
            tested
        };
        let tested = if value.triggered && tested == 0 {
            4
        } else {
            tested
        };
        // Simple.SetState's identical non-triggered assignment has no child,
        // set or modifier dispatch. A changed record must use the same shared
        // subject_sets::changes owner; it is never replaced with a no-op.
        if value.state != tested || tested == 4 {
            return Err(Error::Unsupported("changed building requirement dispatch"));
        }
    }
    // Every sink in28fe130 requires one of its named nonordinary buildings,
    // a wonder count or >11 specialty districts. This bounded early-city path
    // reaches none; queries before those guards have no gameplay/RNG effects.
    let allowed = [
        linux_type_hash(b"BUILDING_PALACE"),
        linux_type_hash(b"BUILDING_MONUMENT"),
    ];
    for player in &state.retained().players {
        for city in &player.cities.records.entries {
            for item in &city.value.buildings.building_locations {
                if item.value != -1 && !allowed.contains(&item.definition_hash) {
                    return Err(Error::Unsupported("general building achievement predicate"));
                }
            }
        }
    }
    if own.districts.district_ids.len() > 1 {
        return Err(Error::Unsupported("general district achievement predicate"));
    }
    Ok(())
}
