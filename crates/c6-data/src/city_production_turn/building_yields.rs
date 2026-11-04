//! Numeric building contribution used by current City.CalculateYield.
//! Native TrackedValue labels are presentation; all arithmetic retains Q8 order.
use super::{buildings, Error};
use crate::{registry::linux_type_hash, Rules, SqlValue};
use c6_core::{
    city_production::{execution::multiply, BuildingYieldModifier},
    loaded_state::LoadedState,
};

#[derive(Clone, Debug, Default)]
pub(super) struct StaticYields {
    base: Vec<(u32, i32)>,
    powered: Vec<(u32, i32)>,
    per_era: Vec<(u32, i32)>,
    district_copy: bool,
    regional_range: i32,
}
impl StaticYields {
    pub(super) fn has_science_row(&self) -> bool {
        self.base
            .iter()
            .any(|&(kind, _)| kind == linux_type_hash(b"YIELD_SCIENCE"))
    }
}
pub(super) fn from_rules(
    rules: &Rules,
    building: u32,
    regional_range: i32,
) -> Result<StaticYields, Error> {
    let rows = |name| -> Result<Vec<(u32, i32)>, Error> {
        let table = rules
            .gameplay()
            .table(name)
            .ok_or(Error::Unsupported(name))?;
        let mut rows = table.rows().collect::<Vec<_>>();
        rows.sort_by_key(|r| r.row_id());
        let mut out = Vec::new();
        for row in rows {
            if linux_type_hash(row.text("BuildingType").map_err(Error::Rules)?.as_bytes())
                != building
            {
                continue;
            }
            let SqlValue::Text(yield_name) = row.get("YieldType").map_err(Error::Rules)? else {
                continue;
            };
            let amount =
                crate::game_parameters::sql_integer(row.get("YieldChange").map_err(Error::Rules)?)
                    .ok_or(Error::Unsupported("Building YieldChange"))?;
            out.push((linux_type_hash(yield_name), amount));
        }
        Ok(out)
    };
    let copies = rules
        .gameplay()
        .table("Building_YieldDistrictCopies")
        .ok_or(Error::Unsupported("Building_YieldDistrictCopies"))?;
    let mut district_copy = false;
    for row in copies.rows() {
        if linux_type_hash(row.text("BuildingType").map_err(Error::Rules)?.as_bytes()) == building {
            district_copy = true;
        }
    }
    Ok(StaticYields {
        base: rows("Building_YieldChanges")?,
        powered: rows("Building_YieldChangesBonusWithPower")?,
        per_era: rows("Building_YieldsPerEra")?,
        district_copy,
        regional_range,
    })
}

/// Actual current era and current City.Power.IsFullyPowered result. The caller
/// derives power from the same city (the shared amenities definitions expose
/// that source query); these are inputs, never cached native yield answers.
pub struct Runtime {
    pub current_era_index: i32,
    pub fully_powered: bool,
}
fn tuple(values: &[BuildingYieldModifier], building: u32, yield_hash: u32) -> i32 {
    values
        .iter()
        .find(|v| v.building_hash == building && v.yield_hash == yield_hash)
        .map_or(0, |v| v.amount)
}
fn location(city: &c6_core::city::City, building: u32) -> Result<i16, Error> {
    Ok(city.buildings.building_locations
        [buildings::index(&city.buildings.building_locations, building)?]
    .value)
}
fn pillaged(city: &c6_core::city::City, building: u32) -> Result<bool, Error> {
    Ok(city.buildings.pillaged[buildings::index(&city.buildings.pillaged, building)?].value)
}
fn modified(base: i32, change: i32, feature: i32, modifier: i32, city_states: i32) -> i32 {
    let mut value = base
        .wrapping_add(change.wrapping_shl(8))
        .wrapping_add(feature.wrapping_shl(8));
    if modifier != 0 {
        let product = multiply(value, modifier.wrapping_add(100).wrapping_shl(8));
        value = ((i64::from(product) << 8) / 25600) as i32;
    }
    value.wrapping_add(city_states.wrapping_shl(8))
}

/// CalculateYieldFromBuildings2895b60, scalar contribution for current ordinary
/// buildings. `track_steps` changes native presentation labels, not this value.
/// A live regional source or district-copy building remains explicit rather
/// than returning an invented zero. Missing/pillaged regional sources are zero.
pub fn calculate(
    state: &LoadedState,
    owner: i32,
    city_id: u32,
    yield_index: i32,
    _track_steps: bool,
    definitions: &buildings::Definitions,
    runtime: &Runtime,
) -> Result<i32, Error> {
    definitions.validate(state)?;
    let yield_hash = *definitions
        .yield_hashes
        .get(usize::try_from(yield_index).map_err(|_| Error::Unsupported("building yield index"))?)
        .ok_or(Error::Unsupported("building yield index"))?;
    let city = state.city(owner, city_id)?;
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("building yield owner"))?;
    let mut result = 0i32;
    for building in &definitions.buildings {
        let data = &building.yields;
        if data.regional_range > 0 {
            for source in &player.cities.records.entries {
                if location(&source.value, building.hash)? != -1
                    && !pillaged(&source.value, building.hash)?
                {
                    return Err(Error::Unsupported("current regional building yield source"));
                }
            }
            continue;
        }
        let plot = location(city, building.hash)?;
        if plot == -1 || pillaged(city, building.hash)? {
            continue;
        }
        if data.district_copy {
            return Err(Error::Unsupported("building district yield copy"));
        }
        let mut value = 0i32;
        for &(kind, amount) in &data.base {
            if kind == yield_hash {
                value = value.wrapping_add(amount.wrapping_shl(8));
            }
        }
        if runtime.fully_powered {
            for &(kind, amount) in &data.powered {
                if kind == yield_hash {
                    value = value.wrapping_add(amount.wrapping_shl(8));
                    let bonus = if city.header.wire_version == 44 {
                        0
                    } else {
                        city.yield_change_bonus_with_power_modifier[buildings::index(
                            &city.yield_change_bonus_with_power_modifier,
                            yield_hash,
                        )?]
                        .value
                    };
                    value = value.wrapping_add(bonus.wrapping_shl(8));
                }
            }
        }
        let era = city.buildings.era_created
            [buildings::index(&city.buildings.era_created, building.hash)?]
        .value;
        let age = runtime.current_era_index.wrapping_sub(i32::from(era));
        if age > 0 {
            for &(kind, amount) in &data.per_era {
                if kind == yield_hash {
                    value = value.wrapping_add(amount.wrapping_mul(age).wrapping_shl(8));
                }
            }
        }
        let mut change = tuple(&city.buildings.yield_changes, building.hash, yield_hash);
        if building.wonder {
            change = change.wrapping_add(
                city.buildings.wonder_yield_changes
                    [buildings::index(&city.buildings.wonder_yield_changes, yield_hash)?]
                .value,
            );
        }
        let mut feature = 0i32;
        if let Some(p) = usize::try_from(plot)
            .ok()
            .and_then(|i| state.world().map().plots.get(i))
        {
            if p.feature_hash != u32::MAX {
                for row in &city.buildings.yield_change_from_features {
                    if row.feature_hash == p.feature_hash && row.yield_type == yield_index {
                        feature = feature.wrapping_add(row.amount);
                    }
                }
            }
        }
        value = modified(
            value,
            change,
            feature,
            tuple(&city.buildings.yield_modifiers, building.hash, yield_hash),
            tuple(
                &city.buildings.yield_changes_city_states,
                building.hash,
                yield_hash,
            ),
        );
        result = result.wrapping_add(value);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_changes_precede_percentage_but_city_states_follow_it() {
        assert_eq!(modified(256, 1, 1, 50, 2), 1664);
        assert_eq!(modified(-1, 0, 0, 50, 0), -1);
        assert_eq!(modified(257, 0, 0, 25, 0), 321);
    }
}
