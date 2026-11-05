//! Current numeric City.CalculateYieldFromDistricts contribution.
//! TrackedValue labels are presentation; its raw Q8 additions are unconditional.
mod adjacency;
use super::Error;
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    unit_initialization_definitions::collection,
    Rules, SqlValue,
};
use c6_core::{
    loaded_state::LoadedState, movement_world::DefinitionLookup, player_districts::District,
    unit::HashValue,
};
struct DistrictDefinition {
    internal_only: bool,
    specialty: bool,
    adjacency_yields: Vec<Option<u32>>,
}
pub struct Definitions {
    yields: LinuxDefinitionRegistry,
    districts: LinuxDefinitionRegistry,
    district: Vec<DistrictDefinition>,
    adjacency: adjacency::RulesCache,
}
impl Definitions {
    pub fn from_rules(rules: &Rules) -> Result<Self, Error> {
        let yields = collection(rules, "Yields", "YieldType").map_err(Error::Definitions)?;
        let districts =
            collection(rules, "Districts", "DistrictType").map_err(Error::Definitions)?;
        let adjacency =
            collection(rules, "Adjacency_YieldChanges", "ID").map_err(Error::Definitions)?;
        let table = |name| rules.gameplay().table(name).ok_or(Error::Unsupported(name));
        let hash = |row: crate::Row<'_>, name| -> Result<u32, Error> {
            match row.get(name).map_err(Error::Rules)? {
                SqlValue::Text(value) => Ok(linux_type_hash(value)),
                SqlValue::Null => Ok(u32::MAX),
                _ => Err(Error::Unsupported("district yield definition text")),
            }
        };
        let mut district = Vec::with_capacity(districts.count());
        for entry in districts.entries() {
            let row = table("Districts")?
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Unsupported("district yield definition row"))?;
            let boolean = |name| {
                crate::game_parameters::sql_integer(row.get(name).map_err(Error::Rules)?)
                    .map(|value| value != 0)
                    .ok_or(Error::Unsupported(name))
            };
            let mut adjacency_yields = Vec::new();
            for reference in table("District_Adjacencies")?.rows() {
                if hash(reference, "DistrictType")? != linux_type_hash(&entry.type_name) {
                    continue;
                }
                adjacency_yields.push(
                    adjacency
                        .find_index(hash(reference, "YieldChangeId")?)
                        .map(|index| index as u32),
                );
            }
            district.push(DistrictDefinition {
                internal_only: boolean("InternalOnly")?,
                specialty: boolean("RequiresPopulation")?,
                adjacency_yields,
            });
        }
        Ok(Self {
            yields,
            districts,
            district,
            adjacency: adjacency::RulesCache::from_rules(rules)?,
        })
    }
    /// Bind actual current configuration identities used by native trait exclusions.
    /// Missing owners remain explicit when an excluded adjacency row is reached.
    pub fn bind_player_configuration(&mut self, players: &[(i32, u32, u32)]) {
        self.adjacency.bind_configuration(players);
    }
    fn yield_hash(&self, index: i32) -> Result<u32, Error> {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.yields.type_hash(i))
            .ok_or(Error::Unsupported("district yield index"))
    }
    fn district(&self, district: &District) -> Result<&DistrictDefinition, Error> {
        self.districts
            .find_index(district.district_hash)
            .and_then(|i| self.district.get(i))
            .ok_or(Error::Unsupported("district yield type"))
    }
}
fn value(rows: &[HashValue<i32>], hash: u32) -> i32 {
    // Native Reset(false) zeroes the current definition cells; the reader then
    // applies ordered hash pairs, with the last duplicate overwriting earlier.
    rows.iter()
        .rev()
        .find(|p| p.definition_hash == hash)
        .map_or(0, |p| p.value)
}
fn scaled(value: i32, modifier: i32) -> i32 {
    if modifier == 0 {
        value
    } else {
        value.wrapping_mul(modifier.wrapping_add(100)) / 100
    }
}
struct Current<'a> {
    state: &'a LoadedState,
    definitions: &'a Definitions,
}
impl Current<'_> {
    fn plot(&self, district: &District) -> Option<&c6_core::map::Plot> {
        self.state
            .world()
            .map()
            .plot_at(district.location.0, district.location.1)
    }
    fn appeal(&self, district: &District, yield_index: i32) -> Result<i32, Error> {
        appeal_yield(
            district.appeal_yield_changes.as_deref(),
            yield_index,
            || {
                self.plot(district)
                    .map(|plot| i32::from(plot.appeal))
                    .ok_or(Error::Unsupported("appeal district plot"))
            },
        )
    }
    fn map_adjacency(&self, district: &District, yield_index: i32) -> Result<i32, Error> {
        // GetYieldFrom first rejects NO_DISTRICT/null plot/NO_PLAYER before
        // resolving its static references or current dynamic City bonuses.
        if district.district_hash == u32::MAX {
            return Ok(0);
        }
        let Some(plot) = self.plot(district) else {
            return Ok(0);
        };
        let owner = crate::movement_placement_state::plot_owner(plot)
            .map_err(|_| Error::Unsupported("district adjacency plot owner"))?;
        if owner == -1 {
            return Ok(0);
        }
        let hash = self.definitions.yield_hash(yield_index)?;
        let definition = self.definitions.district(district)?;
        let mut total = 0i32;
        for &reference in &definition.adjacency_yields {
            if let Some(index) = reference {
                total = total.wrapping_add(self.definitions.adjacency.evaluate(
                    index,
                    self.state,
                    self.definitions,
                    owner,
                    district.location,
                    hash,
                )?);
            }
        }
        if self
            .state
            .world()
            .player_is_valid(owner)
            .map_err(|_| Error::Unsupported("district adjacency player validity"))?
        {
            let Some(attributes) = &plot.city_attributes else {
                return Ok(total);
            };
            let city = self
                .state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == owner)
                .and_then(|p| {
                    p.cities
                        .records
                        .entries
                        .iter()
                        .find(|e| e.value.header.id == attributes.owning_city)
                })
                .map(|e| &e.value);
            if city.is_some_and(|city| !city.header.dynamic_bonuses.is_empty()) {
                return Err(Error::Unsupported(
                    "current dynamic district adjacency bonuses",
                ));
            }
        }
        Ok(total)
    }
    fn adjacency(&self, district: &District, index: i32) -> Result<i32, Error> {
        self.adjacency_current(district, index, true)
    }
    fn adjacency_current(
        &self,
        district: &District,
        index: i32,
        apply_modifier: bool,
    ) -> Result<i32, Error> {
        if district.pillaged {
            return Ok(0);
        }
        let hash = self.definitions.yield_hash(index)?;
        let amount = self
            .map_adjacency(district, index)?
            .wrapping_add(value(&district.base_yield_changes, hash))
            .wrapping_add(self.appeal(district, index)?);
        Ok(if apply_modifier {
            scaled(amount, value(&district.yield_adjacency_modifiers, hash))
        } else {
            amount
        })
    }
    fn yield_from(&self, district: &District, index: i32) -> Result<i32, Error> {
        if district.pillaged {
            return Ok(0);
        }
        let hash = self.definitions.yield_hash(index)?;
        combine(district, hash, |source| {
            let index = self
                .definitions
                .yields
                .find_index(source)
                .ok_or(Error::Unsupported("district mirrored yield type"))?
                as i32;
            self.adjacency(district, index)
        })
    }
}
fn appeal_yield(
    rows: Option<&[c6_core::player_districts::AppealYieldChange]>,
    yield_index: i32,
    mut current_appeal: impl FnMut() -> Result<i32, Error>,
) -> Result<i32, Error> {
    let rows = rows.ok_or(Error::Unsupported(
        "current unsaved district appeal yield rows",
    ))?;
    let mut total = 0i32;
    for row in rows {
        if row.yield_index == yield_index && current_appeal()? >= row.minimum_appeal {
            total = total.wrapping_add(row.change);
        }
    }
    Ok(total)
}
fn combine(
    district: &District,
    hash: u32,
    mut adjacency: impl FnMut(u32) -> Result<i32, Error>,
) -> Result<i32, Error> {
    if district.pillaged {
        return Ok(0);
    }
    let mut base = value(&district.yield_changes, hash);
    for mirror in &district.yield_adjacency_mirrors {
        if mirror.yield_to_grant_hash == hash {
            // Native uses every retained row once. mirror_count is not an
            // arithmetic multiplier in GetBaseYield.
            base = base.wrapping_add(adjacency(mirror.yield_to_mirror_hash)?);
        }
    }
    Ok(base.wrapping_add(adjacency(hash)?))
}
/// Current District.GetAdjacencyYield in whole yield units. Native `false`
/// skips percentage modifiers but retains static/base/appeal terms; pillage is
/// checked before those current queries. No City yield, mirrors or Q8 scaling.
pub fn adjacency(
    state: &LoadedState,
    owner: i32,
    district: u32,
    yield_index: i32,
    apply_modifier: bool,
    definitions: &Definitions,
) -> Result<i32, Error> {
    let district = state
        .district(owner, district)
        .map_err(|_| Error::Unsupported("district adjacency identity"))?;
    Current { state, definitions }.adjacency_current(district, yield_index, apply_modifier)
}
/// Returns the effective TrackedValue raw Q8 scalar, not its localized labels.
pub fn calculate(
    state: &LoadedState,
    owner: i32,
    city: u32,
    yield_index: i32,
    _track_steps: bool,
    definitions: &Definitions,
) -> Result<i32, Error> {
    let city = state.city(owner, city)?;
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("district yield player"))?;
    let district = |id: u32| {
        if id == u32::MAX {
            return None;
        }
        player
            .districts
            .records
            .entries
            .iter()
            .find(|entry| entry.index as u32 == id & 0xffff)
            .map(|entry| &entry.value)
    };
    let current = Current { state, definitions };
    let mut total = 0i32;
    for &id in &city.districts.district_ids {
        let Some(district) = district(id) else {
            continue;
        };
        if district.is_complete {
            total = total.wrapping_add(current.yield_from(district, yield_index)?.wrapping_shl(8));
        }
    }
    let hash = definitions.yield_hash(yield_index)?;
    let per_district = value(&city.header.yield_per_district, hash);
    if per_district > 0 {
        let mut count = 0i32;
        for &id in &city.districts.district_ids {
            let Some(district) = district(id) else {
                continue;
            };
            let definition = definitions.district(district)?;
            if !definition.internal_only
                && district.is_complete
                && definition.specialty
                && !district.pillaged
            {
                count = count.wrapping_add(1);
            }
        }
        total = total.wrapping_add(count.wrapping_mul(per_district).wrapping_shl(8));
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsaved_appeal_unknown_is_distinct_from_empty_and_matching_rows_are_lazy() {
        use c6_core::player_districts::AppealYieldChange as Row;
        assert!(appeal_yield(None, 1, || panic!("unavailable")).is_err());
        assert_eq!(appeal_yield(Some(&[]), 1, || panic!("empty")).unwrap(), 0);
        let rows = [
            Row {
                yield_index: 2,
                minimum_appeal: 0,
                change: 99,
            },
            Row {
                yield_index: 1,
                minimum_appeal: 2,
                change: 7,
            },
            Row {
                yield_index: 1,
                minimum_appeal: 4,
                change: 20,
            },
        ];
        let mut calls = 0;
        assert_eq!(
            appeal_yield(Some(&rows), 1, || {
                calls += 1;
                Ok(3)
            })
            .unwrap(),
            7
        );
        assert_eq!(calls, 2);
    }
    #[test]
    fn native_wire_does_not_invent_unsaved_appeal_rows() {
        use c6_core::{
            district_initialization as init, player_districts::AppealYieldChange, unit::Limits,
        };
        let mut district = init::construct(init::Definitions {
            yields: &[1],
            great_person_classes: &[],
        })
        .unwrap();
        assert_eq!(district.appeal_yield_changes, Some(vec![]));
        district
            .appeal_yield_changes
            .as_mut()
            .unwrap()
            .push(AppealYieldChange {
                yield_index: 0,
                minimum_appeal: 2,
                change: 3,
            });
        let bytes = district.encode(Limits::default()).unwrap();
        let decoded = District::decode(&bytes, Limits::default()).unwrap();
        assert_eq!(decoded.appeal_yield_changes, None);
        assert_eq!(decoded.encode(Limits::default()).unwrap(), bytes);
    }
    #[test]
    fn mirrors_keep_source_order_and_count_is_not_a_multiplier() {
        use c6_core::{district_initialization as init, player_districts::AdjacencyBonusMirror};
        let mut district = init::construct(init::Definitions {
            yields: &[1, 2],
            great_person_classes: &[],
        })
        .unwrap();
        district.yield_changes[0].value = 4;
        district.yield_adjacency_mirrors = vec![
            AdjacencyBonusMirror {
                yield_to_mirror_hash: 2,
                yield_to_grant_hash: 1,
                mirror_count: 999,
            },
            AdjacencyBonusMirror {
                yield_to_mirror_hash: 2,
                yield_to_grant_hash: 1,
                mirror_count: 0,
            },
        ];
        let mut calls = Vec::new();
        let result = combine(&district, 1, |hash| {
            calls.push(hash);
            Ok(if hash == 2 { 3 } else { 2 })
        })
        .unwrap();
        assert_eq!(result, 12);
        assert_eq!(calls, [2, 2, 1]);
        district.pillaged = true;
        assert_eq!(
            combine(&district, 1, |_| panic!("pillaged adjacency")).unwrap(),
            0
        );
    }
    #[test]
    fn source_hash_overlay_and_wrapping_percent_before_q8() {
        let rows = [
            HashValue {
                definition_hash: 1,
                value: 7,
            },
            HashValue {
                definition_hash: 9,
                value: 400,
            },
            HashValue {
                definition_hash: 1,
                value: -3,
            },
        ];
        assert_eq!(value(&rows, 1), -3);
        assert_eq!(value(&rows, 2), 0);
        assert_eq!(scaled(-3, 50), -4);
        assert_eq!(scaled(3, -150), -1);
        assert_eq!(scaled(i32::MAX, 100), -2);
        assert_eq!(i32::MAX.wrapping_shl(8), -256);
    }
}
