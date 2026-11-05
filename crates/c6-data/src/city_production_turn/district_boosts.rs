//! Reached DistrictCompleted boost checks over canonical current districts.
//! Each invocation represents one ordered native subscription. The manager-level
//! boost dispatcher keeps research callbacks, RNG, logging and telemetry on the
//! caller's existing research context; no progress or triggered flags are copied.
use super::{selection::DistrictMap, Error};
use crate::{
    district_initialization::DistrictReference, registry::linux_type_hash, research,
    unit_initialization_definitions::collection, Rules, SqlValue,
};
use c6_core::{loaded_state::LoadedState, player_districts::District, relic_creation::boost};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Handler {
    NumDistricts,
    UniqueSpecialtyDistricts,
    MinimumAppeal,
    AirbaseForeignContinent,
}
struct Rule {
    id: i32,
    index: usize,
    handler: Handler,
    district: Option<u32>,
    threshold: i32,
}
struct DistrictDefinition {
    hash: u32,
    specialty: bool,
    air_slots: i32,
    city_center: bool,
}
pub struct Definitions {
    rules: Vec<Rule>,
    districts: Vec<DistrictDefinition>,
    replacements: Vec<(u32, u32)>,
}
fn text(row: crate::Row<'_>, column: &'static str) -> Result<Option<u32>, Error> {
    match row.get(column).map_err(Error::Rules)? {
        SqlValue::Null => Ok(None),
        SqlValue::Text(v) => Ok(Some(linux_type_hash(v))),
        _ => Err(Error::Unsupported(column)),
    }
}
fn integer(row: crate::Row<'_>, column: &'static str) -> Result<i32, Error> {
    crate::game_parameters::sql_integer(row.get(column).map_err(Error::Rules)?)
        .ok_or(Error::Unsupported(column))
}
impl Definitions {
    pub fn from_rules(rules: &Rules) -> Result<Self, Error> {
        let table = |name| rules.gameplay().table(name).ok_or(Error::Unsupported(name));
        let collection =
            collection(rules, "Districts", "DistrictType").map_err(Error::Definitions)?;
        let mut districts = Vec::new();
        for entry in collection.entries() {
            let row = table("Districts")?
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Unsupported("district boost definition"))?;
            districts.push(DistrictDefinition {
                hash: linux_type_hash(&entry.type_name),
                specialty: integer(row, "RequiresPopulation")? != 0,
                air_slots: integer(row, "AirSlots")?,
                city_center: integer(row, "CityCenter")? != 0,
            });
        }
        let mut cached = Vec::new();
        // research::Definitions builds its boost vector from these same rows in
        // this same order. BoostID is a key, not an assumed index or endpoint.
        for (index, row) in table("Boosts")?.rows().enumerate() {
            let handler = match row.get("BoostClass").map_err(Error::Rules)? {
                SqlValue::Text(v) => match v.as_slice() {
                    b"BOOST_TRIGGER_HAVE_X_DISTRICTS" => Handler::NumDistricts,
                    b"BOOST_TRIGGER_HAVE_X_UNIQUE_SPECIALTY_DISTRICTS" => {
                        Handler::UniqueSpecialtyDistricts
                    }
                    b"BOOST_TRIGGER_DISTRICT_APPEAL_LEVEL_MINIMUM_X" => Handler::MinimumAppeal,
                    b"BOOST_TRIGGER_AIRBASE_FOREIGN_CONTINENT" => Handler::AirbaseForeignContinent,
                    _ => continue,
                },
                _ => return Err(Error::Unsupported("district boost class")),
            };
            let district =
                text(row, "DistrictType")?.filter(|hash| collection.find(*hash).is_some());
            cached.push(Rule {
                id: integer(row, "BoostID")?,
                index,
                handler,
                district,
                threshold: integer(row, "NumItems")?,
            });
        }
        let replacements = table("DistrictReplaces")?
            .rows()
            .map(|row| {
                Ok((
                    text(row, "ReplacesDistrictType")?
                        .ok_or(Error::Unsupported("district boost replacement base"))?,
                    text(row, "CivUniqueDistrictType")?
                        .ok_or(Error::Unsupported("district boost replacement"))?,
                ))
            })
            .collect::<Result<_, Error>>()?;
        Ok(Self {
            rules: cached,
            districts,
            replacements,
        })
    }
    /// Used when binding an observed callback; unknown IDs/classes stay explicit.
    pub fn handler(&self, boost_id: i32) -> Option<Handler> {
        self.rules
            .iter()
            .find(|r| r.id == boost_id)
            .map(|r| r.handler)
    }
    fn same_or_replacement(&self, a: u32, b: u32) -> bool {
        a == b
            || self.replacements.iter().any(|&(base, replacement)| {
                (base == a && replacement == b) || (base == b && replacement == a)
            })
    }
    fn eligible(
        &self,
        state: &LoadedState,
        maps: &[DistrictMap],
        reference: DistrictReference,
        rule: &Rule,
    ) -> Result<bool, Error> {
        let district = state
            .district(reference.owner, reference.id)
            .map_err(|_| Error::Unsupported("district boost event identity"))?;
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == reference.owner)
            .ok_or(Error::Unsupported("district boost owner"))?;
        match rule.handler {
            Handler::NumDistricts => {
                let Some(base) = rule.district else {
                    return Ok(false);
                };
                let build = build_district(maps, reference.owner, base)?;
                if build != Some(district.district_hash) {
                    return Ok(false);
                }
                // Stats.GetNumDistrictsOfType(type,false,true,true): include
                // the owner's replacement, exclude unfinished, include pillaged.
                let count = count_type(
                    &player
                        .districts
                        .records
                        .entries
                        .iter()
                        .map(|s| &s.value)
                        .collect::<Vec<_>>(),
                    district.district_hash,
                    || build_district(maps, reference.owner, district.district_hash),
                )?;
                Ok(count >= rule.threshold)
            }
            Handler::UniqueSpecialtyDistricts => {
                let roster = player
                    .districts
                    .records
                    .entries
                    .iter()
                    .map(|s| &s.value)
                    .collect::<Vec<_>>();
                let count = unique_specialties(&self.districts, &roster);
                Ok(count >= rule.threshold)
            }
            Handler::MinimumAppeal => {
                let Some(base) = rule.district else {
                    return Ok(false);
                };
                if !self.same_or_replacement(base, district.district_hash) {
                    return Ok(false);
                }
                let plot = state
                    .world()
                    .map()
                    .plot_at(district.location.0, district.location.1)
                    .ok_or(Error::Unsupported("district boost appeal plot"))?;
                Ok(i32::from(plot.appeal) >= rule.threshold)
            }
            Handler::AirbaseForeignContinent => {
                let definition = self
                    .districts
                    .iter()
                    .find(|d| d.hash == district.district_hash)
                    .ok_or(Error::Unsupported("district boost airbase definition"))?;
                // CheckAirbaseForeignContinent rejects these before its capital
                // lookup. This is the reached Campus guard, not an empty grant.
                if definition.air_slots <= 0 || definition.city_center {
                    return Ok(false);
                }
                Err(Error::Unsupported(
                    "district boost current capital continent",
                ))
            }
        }
    }
}
fn build_district(maps: &[DistrictMap], owner: i32, hash: u32) -> Result<Option<u32>, Error> {
    maps.iter()
        .find(|m| m.owner == owner)
        .and_then(|m| m.values.iter().rev().find(|v| v.definition_hash == hash))
        .map(|v| v.value)
        .ok_or(Error::Unsupported(
            "district boost current replacement cell",
        ))
}
fn count_type(
    districts: &[&District],
    hash: u32,
    mut replacement: impl FnMut() -> Result<Option<u32>, Error>,
) -> Result<i32, Error> {
    let mut count = 0i32;
    for district in districts {
        let same = if district.district_hash == hash {
            true
        } else {
            replacement()? == Some(district.district_hash)
        };
        if same && district.is_complete {
            count = count.wrapping_add(1)
        }
    }
    Ok(count)
}
fn unique_specialties(definitions: &[DistrictDefinition], districts: &[&District]) -> i32 {
    definitions
        .iter()
        .filter(|definition| {
            definition.specialty
                && districts
                    .iter()
                    .any(|d| d.district_hash == definition.hash && d.is_complete)
        })
        .count() as i32
}
#[derive(Debug)]
pub enum CompletionError<E> {
    Query(Error),
    Boost(Box<boost::Error<research::Error<E>>>),
}
impl<E: std::fmt::Debug> std::fmt::Display for CompletionError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "district boost: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for CompletionError<E> {}
/// Execute ONE captured DistrictCompleted subscription at its original position.
/// The native wrappers ignore `bought`; ordering relative to other subscribers
/// is retained by the caller. Failure preserves the already applied boost prefix.
pub fn completed<E: research::Effects>(
    context: &mut research::Context<'_, E>,
    definitions: &Definitions,
    maps: &[DistrictMap],
    district: DistrictReference,
    boost_id: i32,
    _bought: bool,
) -> Result<(), CompletionError<E::Error>> {
    let rule =
        definitions
            .rules
            .iter()
            .find(|r| r.id == boost_id)
            .ok_or(CompletionError::Query(Error::Unsupported(
                "district boost captured BoostID",
            )))?;
    if definitions
        .eligible(context.state, maps, district, rule)
        .map_err(CompletionError::Query)?
    {
        boost::trigger_boost(context, Some(district.owner), rule.index)
            .map_err(|e| CompletionError::Boost(Box::new(e)))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn district(hash: u32, complete: bool, pillaged: bool) -> District {
        let mut district = c6_core::district_initialization::construct(
            c6_core::district_initialization::Definitions {
                yields: &[],
                great_person_classes: &[],
            },
        )
        .unwrap();
        district.district_hash = hash;
        district.is_complete = complete;
        district.pillaged = pillaged;
        district
    }
    #[test]
    fn number_of_districts_includes_pillaged_and_replacement_but_not_unfinished() {
        let same = district(1, true, true);
        let replacement = district(2, true, false);
        let unfinished = district(1, false, false);
        let unrelated = district(3, true, false);
        assert_eq!(
            count_type(&[&same, &replacement, &unfinished, &unrelated], 1, || Ok(
                Some(2)
            ))
            .unwrap(),
            2
        );
        assert_eq!(
            count_type(&[&same], 1, || panic!("unreached replacement query")).unwrap(),
            1
        );
        assert!(count_type(&[&unrelated], 1, || Err(Error::Unsupported(
            "unbound actual map"
        )))
        .is_err());
    }
    #[test]
    fn unique_completed_specialties_count_types_once_and_include_pillage() {
        let definitions = [
            DistrictDefinition {
                hash: 1,
                specialty: true,
                air_slots: 0,
                city_center: false,
            },
            DistrictDefinition {
                hash: 2,
                specialty: true,
                air_slots: 0,
                city_center: false,
            },
            DistrictDefinition {
                hash: 3,
                specialty: false,
                air_slots: 0,
                city_center: true,
            },
        ];
        let a = district(1, true, false);
        let duplicate = district(1, true, true);
        let unfinished = district(2, false, false);
        let center = district(3, true, false);
        assert_eq!(
            unique_specialties(&definitions, &[&a, &duplicate, &unfinished, &center]),
            1
        );
        let completed = district(2, true, true);
        assert_eq!(unique_specialties(&definitions, &[&a, &completed]), 2);
    }
    #[test]
    fn minimum_appeal_replacement_relation_is_symmetric_and_one_hop() {
        let definitions = Definitions {
            rules: vec![],
            districts: vec![],
            replacements: vec![(1, 2), (2, 3)],
        };
        assert!(definitions.same_or_replacement(1, 1));
        assert!(definitions.same_or_replacement(1, 2));
        assert!(definitions.same_or_replacement(2, 1));
        assert!(!definitions.same_or_replacement(1, 3));
    }
}
