//! Ordered score recomputation over current gameplay inputs.
//!
//! The cache is unsaved runtime state. Logging labels and file output are
//! omitted; category/raw queries and cache writes retain their source order.
use crate::{
    resource_binding::ResourceAuthority, unit_initialization_definitions as init, Rules, SqlValue,
};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Raw {
    Cities,
    Civics,
    Districts,
    GreatPeople,
    Population,
    Religion,
    Technologies,
    Wonders,
    Pillage,
    Trade,
    GoldPerTurn,
    Scenario1,
    Scenario2,
    Scenario3,
    EraScore,
    Buildings,
    Converted,
    Zero,
}
const FLAGS: [(&str, Raw); 17] = [
    ("Cities", Raw::Cities),
    ("Civics", Raw::Civics),
    ("Districts", Raw::Districts),
    ("GreatPeople", Raw::GreatPeople),
    ("Population", Raw::Population),
    ("Religion", Raw::Religion),
    ("Techs", Raw::Technologies),
    ("Wonders", Raw::Wonders),
    ("Trade", Raw::Trade),
    ("Pillage", Raw::Pillage),
    ("GoldPerTurn", Raw::GoldPerTurn),
    ("ScoringScenario1", Raw::Scenario1),
    ("ScoringScenario2", Raw::Scenario2),
    ("ScoringScenario3", Raw::Scenario3),
    ("EraScore", Raw::EraScore),
    ("Buildings", Raw::Buildings),
    ("Converted", Raw::Converted),
];
#[derive(Debug)]
pub enum Error {
    Definitions(init::Error),
    Rules(crate::Error),
    Binding,
    Missing(&'static str),
    Unsupported(Raw),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "player scoring: {self:?}")
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Debug)]
pub struct Line {
    pub multiplier: f32,
    pub raw: Raw,
}
#[derive(Clone, Debug)]
pub struct Category {
    pub multiplier: f32,
    pub lines: Vec<Line>,
}
pub struct Definitions {
    pub categories: Vec<Category>,
    buildings: Vec<(u32, bool)>,
    specialty_districts: Vec<u32>,
    /// Native line-item order, before descending priority sorting.
    pub tie_breakers: Vec<(i32, Raw)>,
}
fn multiplier(value: &SqlValue) -> Result<f32, Error> {
    match value {
        SqlValue::RealBits(bits) => Ok(f64::from_bits(*bits) as f32),
        SqlValue::Integer(value) => Ok(*value as f32),
        _ => Err(Error::Missing("scoring multiplier")),
    }
}
impl Definitions {
    pub fn from_rules(rules: &Rules, resources: &ResourceAuthority<'_>) -> Result<Self, Error> {
        init::Definitions::validate_resources(rules, resources).map_err(Error::Definitions)?;
        let categories = init::collection(rules, "ScoringCategories", "CategoryType")
            .map_err(Error::Definitions)?;
        let lines = init::collection(rules, "ScoringLineItems", "LineItemType")
            .map_err(Error::Definitions)?;
        let category_rows = rules
            .gameplay()
            .table("ScoringCategories")
            .ok_or(Error::Missing("ScoringCategories"))?;
        let line_rows = rules
            .gameplay()
            .table("ScoringLineItems")
            .ok_or(Error::Missing("ScoringLineItems"))?;
        let mut result = Vec::new();
        for entry in categories.entries() {
            let row = category_rows
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Missing("scoring category"))?;
            let name = row.text("CategoryType").map_err(Error::Rules)?;
            let mut category = Category {
                multiplier: multiplier(row.get("Multiplier").map_err(Error::Rules)?)?,
                lines: Vec::new(),
            };
            for entry in lines.entries() {
                let row = line_rows
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Missing("scoring line"))?;
                if row.text("Category").map_err(Error::Rules)? != name {
                    continue;
                }
                let mut raw = Raw::Zero;
                for (flag, kind) in FLAGS {
                    if row.integer(flag).map_err(Error::Rules)? != 0 {
                        raw = kind;
                        break;
                    }
                }
                category.lines.push(Line {
                    multiplier: multiplier(row.get("Multiplier").map_err(Error::Rules)?)?,
                    raw,
                });
            }
            result.push(category);
        }
        let mut tie_breakers = Vec::new();
        for entry in lines.entries() {
            let row = line_rows
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Missing("scoring tie-break line"))?;
            let mut raw = Raw::Zero;
            for (flag, kind) in FLAGS {
                if row.integer(flag).map_err(Error::Rules)? != 0 {
                    raw = kind;
                    break;
                }
            }
            tie_breakers.push((
                row.integer("TieBreakerPriority").map_err(Error::Rules)? as i32,
                raw,
            ));
        }
        let buildings =
            init::collection(rules, "Buildings", "BuildingType").map_err(Error::Definitions)?;
        let districts =
            init::collection(rules, "Districts", "DistrictType").map_err(Error::Definitions)?;
        let flags = |registry: &crate::registry::LinuxDefinitionRegistry,
                     table: &str,
                     flag: &str|
         -> Result<Vec<(u32, bool)>, Error> {
            let table = rules
                .gameplay()
                .table(table)
                .ok_or(Error::Missing("scoring definition table"))?;
            registry
                .entries()
                .iter()
                .map(|entry| {
                    let row = table
                        .row_by_id(entry.database_rowid)
                        .ok_or(Error::Missing("scoring definition row"))?;
                    let enabled =
                        crate::game_parameters::sql_integer(row.get(flag).map_err(Error::Rules)?)
                            .ok_or(Error::Missing("scoring definition flag"))?
                            != 0;
                    Ok((crate::registry::linux_type_hash(&entry.type_name), enabled))
                })
                .collect()
        };
        Ok(Self {
            categories: result,
            tie_breakers,
            buildings: flags(&buildings, "Buildings", "IsWonder")?,
            specialty_districts: flags(&districts, "Districts", "RequiresPopulation")?
                .into_iter()
                .filter_map(|(hash, yes)| yes.then_some(hash))
                .collect(),
        })
    }
}
/// CVTTSS2SI returns the indefinite integer for NaN and out-of-range values;
/// Rust's saturating float cast alone would differ at this boundary.
fn trunc(value: f32) -> i32 {
    if !value.is_finite() || !(-2147483648.0..2147483648.0).contains(&value) {
        i32::MIN
    } else {
        value as i32
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cache {
    pub turn: i32,
    pub score: i32,
}
impl Cache {
    /// An error retains the actual recomputation prefix, including the cache
    /// reset and already-completed categories. It is not a completed action.
    pub fn calculate<E>(
        &mut self,
        definitions: &Definitions,
        turn: i32,
        force: bool,
        mut raw: impl FnMut(Raw) -> Result<i32, E>,
    ) -> Result<i32, E> {
        if !force && self.turn == turn {
            return Ok(self.score);
        }
        self.score = 0;
        self.turn = turn;
        for category in &definitions.categories {
            let mut sum = 0i32;
            if category.multiplier > 0.0 {
                for line in &category.lines {
                    if line.multiplier > 0.0 {
                        sum = sum.wrapping_add(trunc((raw(line.raw)? as f32) * line.multiplier));
                    }
                }
            }
            self.score = self
                .score
                .wrapping_add(trunc((sum as f32) * category.multiplier));
        }
        Ok(self.score)
    }
}
/// Current global owners not yet stored in LoadedState. Callers retain and
/// update these with the owning gameplay subsystem, rather than score answers.
pub type DistrictBuildMaps = Vec<(i32, Vec<(u32, Option<u32>)>)>;
#[derive(Default)]
pub struct GlobalInputs {
    pub civilization_levels: Vec<(i32, u32)>,
    pub great_people_received: Option<Vec<(i32, i32)>>,
    pub current_era_scores: Option<Vec<(i32, Vec<i32>)>>,
    /// Current native GetBuildDistrict vector, hash keyed; None values are INVALID.
    pub district_builds: Option<DistrictBuildMaps>,
}
pub struct Runtime {
    pub binding: WireBinding,
    pub caches: Vec<(i32, Cache)>,
    pub globals: GlobalInputs,
}
impl Runtime {
    /// GetScore computes a stale cache without storing its result. Only the
    /// scalar cache is copied; constituent queries read the current world.
    pub fn get_score(
        &self,
        state: &LoadedState,
        definitions: &Definitions,
        owner: i32,
    ) -> Result<i32, Error> {
        if self.binding != *state.binding() {
            return Err(Error::Binding);
        }
        let mut cache = self
            .caches
            .iter()
            .find(|p| p.0 == owner)
            .ok_or(Error::Missing("current scoring cache"))?
            .1;
        cache.calculate(
            definitions,
            state.retained().core.current_turn,
            false,
            |kind| current_raw(state, &self.globals, Some(definitions), owner, kind),
        )
    }
    pub fn raw_score(&self, state: &LoadedState, owner: i32, kind: Raw) -> Result<i32, Error> {
        if self.binding != *state.binding() {
            return Err(Error::Binding);
        }
        current_raw(state, &self.globals, None, owner, kind)
    }
    pub fn raw_score_with_definitions(
        &self,
        state: &LoadedState,
        definitions: &Definitions,
        owner: i32,
        kind: Raw,
    ) -> Result<i32, Error> {
        if self.binding != *state.binding() {
            return Err(Error::Binding);
        }
        current_raw(state, &self.globals, Some(definitions), owner, kind)
    }
    pub fn calculate(
        &mut self,
        state: &LoadedState,
        definitions: &Definitions,
        owner: i32,
        force: bool,
    ) -> Result<i32, Error> {
        if self.binding != *state.binding() {
            return Err(Error::Binding);
        }
        let cache = self
            .caches
            .iter_mut()
            .find(|p| p.0 == owner)
            .ok_or(Error::Missing("current scoring cache"))?;
        cache.1.calculate(
            definitions,
            state.retained().core.current_turn,
            force,
            |kind| current_raw(state, &self.globals, Some(definitions), owner, kind),
        )
    }
}
fn count_completed_districts(
    specialty: &[u32],
    districts: &[c6_core::map::Slot<c6_core::player_districts::District>],
    replacements: Option<&[(u32, Option<u32>)]>,
) -> Result<i32, Error> {
    let mut count = 0i32;
    for &hash in specialty {
        for district in districts {
            if !district.value.is_complete {
                continue;
            }
            let matches = if district.value.district_hash == hash {
                true
            } else {
                replacements
                    .ok_or(Error::Missing("current scoring district replacements"))?
                    .iter()
                    .rev()
                    .find(|r| r.0 == hash)
                    .ok_or(Error::Missing("current scoring replacement cell"))?
                    .1
                    == Some(district.value.district_hash)
            };
            // GetNumDistrictsCompleted passes includePillaged=true.
            count = count.wrapping_add(i32::from(matches));
        }
    }
    Ok(count)
}
fn current_raw(
    state: &LoadedState,
    global: &GlobalInputs,
    definitions: Option<&Definitions>,
    owner: i32,
    kind: Raw,
) -> Result<i32, Error> {
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("current scoring Player"))?;
    let cities = &player.cities.records.entries;
    let religion = state
        .retained()
        .religion
        .records
        .iter()
        .find(|r| r.founder == owner);
    match kind {
        Raw::Zero => Ok(0),
        Raw::Cities => Ok(cities.len() as i32),
        Raw::Population => Ok(cities
            .iter()
            .fold(0i32, |n, c| n.wrapping_add(c.value.header.population))),
        Raw::Religion => Ok(religion.map_or(0, |r| r.belief_hashes.len() as i32)),
        Raw::Converted if religion.is_none() => Ok(0),
        Raw::Districts if player.districts.records.entries.is_empty() => Ok(0),
        Raw::Districts => {
            let definitions = definitions.ok_or(Error::Missing("scoring district definitions"))?;
            count_completed_districts(
                &definitions.specialty_districts,
                &player.districts.records.entries,
                global
                    .district_builds
                    .as_ref()
                    .and_then(|rows| rows.iter().find(|r| r.0 == owner))
                    .map(|r| r.1.as_slice()),
            )
        }
        Raw::Buildings | Raw::Wonders if cities.is_empty() => Ok(0),
        Raw::Buildings | Raw::Wonders => {
            let definitions = definitions.ok_or(Error::Missing("scoring building definitions"))?;
            let mut count = 0i32;
            for &(hash, wonder) in &definitions.buildings {
                if kind == Raw::Wonders && !wonder {
                    continue;
                }
                for city in cities {
                    let present = city
                        .value
                        .buildings
                        .building_locations
                        .iter()
                        .rev()
                        .find(|r| r.definition_hash == hash)
                        .ok_or(Error::Missing("current scoring building location"))?
                        .value
                        >= 0;
                    count = count.wrapping_add(i32::from(present));
                }
            }
            Ok(count)
        }
        Raw::Pillage => Ok(player.stats.tiles_pillaged),
        Raw::Trade => Ok(player.trade.outgoing_route_capacity),
        Raw::Technologies => {
            let flags = state
                .world()
                .player(owner)
                .map_err(|_| Error::Missing("current scoring Player"))?
                .technologies
                .as_ref()
                .ok_or(Error::Missing("current technology flags"))?;
            Ok(flags
                .iter()
                .fold(player.techs.num_techs_repeated, |n, flag| {
                    n.wrapping_add(i32::from(*flag))
                }))
        }
        Raw::Civics => {
            let flags = state
                .world()
                .player(owner)
                .map_err(|_| Error::Missing("current scoring Player"))?
                .civics
                .as_ref()
                .ok_or(Error::Missing("current civic flags"))?;
            count_civics(
                flags,
                state.world().civic_availability.as_deref(),
                player.culture.num_civics_repeated,
            )
        }
        Raw::GreatPeople => global
            .great_people_received
            .as_ref()
            .and_then(|v| v.iter().find(|p| p.0 == owner))
            .map(|p| p.1)
            .ok_or(Error::Missing("current great people received")),
        Raw::EraScore => {
            // Source major-player guard is supplied by the actual current
            // civilization-level identity, not a numerical owner convention.
            let level = global
                .civilization_levels
                .iter()
                .find(|p| p.0 == owner)
                .map(|p| p.1)
                .ok_or(Error::Missing("current civilization level"))?;
            if level != 0x253718b0 {
                return Ok(0);
            }
            global
                .current_era_scores
                .as_ref()
                .and_then(|v| v.iter().find(|p| p.0 == owner))
                .map(|p| p.1.iter().fold(0i32, |n, v| n.wrapping_add(*v)))
                .ok_or(Error::Missing("current era score vector"))
        }
        other => Err(Error::Unsupported(other)),
    }
}
/// HasCivic is the conjunction of two pure current flags. If every acquired
/// flag is known false, missing global availability cannot change the count;
/// no availability value is initialized or assumed by this projection.
fn count_civics(flags: &[bool], active: Option<&[bool]>, repeated: i32) -> Result<i32, Error> {
    if !flags.iter().any(|flag| *flag) {
        return Ok(repeated);
    }
    let active = active.ok_or(Error::Missing("current civic availability"))?;
    if flags.len() != active.len() {
        return Err(Error::Missing("complete current civic availability"));
    }
    Ok(flags
        .iter()
        .zip(active)
        .fold(repeated, |n, (flag, active)| {
            n.wrapping_add(i32::from(*flag && *active))
        }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completed_score_counts_specialty_replacements_and_pillaged_but_not_unfinished() {
        let mut district = c6_core::district_initialization::construct(
            c6_core::district_initialization::Definitions {
                yields: &[],
                great_person_classes: &[],
            },
        )
        .unwrap();
        district.district_hash = 7;
        district.is_complete = true;
        district.pillaged = true;
        let mut replacement = district.clone();
        replacement.district_hash = 8;
        let mut unfinished = district.clone();
        unfinished.is_complete = false;
        let rows = vec![
            c6_core::map::Slot {
                index: 0,
                value: district,
            },
            c6_core::map::Slot {
                index: 1,
                value: replacement,
            },
            c6_core::map::Slot {
                index: 2,
                value: unfinished,
            },
        ];
        assert_eq!(
            count_completed_districts(&[7], &rows, Some(&[(7, Some(8))])).unwrap(),
            2
        );
        assert_eq!(
            count_completed_districts(&[7], &rows, Some(&[(7, None)])).unwrap(),
            1
        );
        assert_eq!(count_completed_districts(&[], &rows, None).unwrap(), 0); // CityCenter is not specialty
        assert!(count_completed_districts(&[7], &rows, None).is_err());
    }
    #[test]
    fn cache_order_rounding_and_failure_prefix() {
        let defs = Definitions {
            buildings: Vec::new(),
            specialty_districts: Vec::new(),
            tie_breakers: Vec::new(),
            categories: vec![
                Category {
                    multiplier: 0.5,
                    lines: vec![
                        Line {
                            multiplier: 1.5,
                            raw: Raw::Cities,
                        },
                        Line {
                            multiplier: 0.0,
                            raw: Raw::Trade,
                        },
                    ],
                },
                Category {
                    multiplier: 1.0,
                    lines: vec![Line {
                        multiplier: 1.0,
                        raw: Raw::EraScore,
                    }],
                },
            ],
        };
        let mut cache = Cache {
            turn: -1,
            score: 99,
        };
        let mut seen = Vec::new();
        assert_eq!(
            cache.calculate(&defs, 2, false, |kind| {
                seen.push(kind);
                if kind == Raw::Cities {
                    Ok(3)
                } else {
                    Err("missing")
                }
            }),
            Err("missing")
        );
        assert_eq!(seen, [Raw::Cities, Raw::EraScore]);
        assert_eq!(cache, Cache { turn: 2, score: 2 });
        assert_eq!(
            cache.calculate::<()>(&defs, 2, false, |_| panic!("cache hit")),
            Ok(2)
        );
        assert_eq!(cache.calculate(&defs, 2, true, |_| Ok::<_, ()>(3)), Ok(5));
    }
    #[test]
    fn truncation_and_wrapping_match_integer_boundaries() {
        assert_eq!(trunc(f32::NAN), i32::MIN);
        assert_eq!(trunc(2147483648.0), i32::MIN);
        assert_eq!(trunc(-1.9), -1);
        let defs = Definitions {
            buildings: Vec::new(),
            specialty_districts: Vec::new(),
            tie_breakers: Vec::new(),
            categories: vec![Category {
                multiplier: 1.0,
                lines: vec![
                    Line {
                        multiplier: 1.0,
                        raw: Raw::Zero
                    };
                    2
                ],
            }],
        };
        let mut cache = Cache { turn: -1, score: 0 };
        assert_eq!(
            cache.calculate(&defs, 0, false, |_| Ok::<_, ()>(i32::MIN)),
            Ok(0)
        );
    }
    #[test]
    fn known_false_civics_do_not_require_or_invent_global_availability() {
        assert_eq!(count_civics(&[false, false], None, 7).unwrap(), 7);
        assert!(matches!(
            count_civics(&[false, true], None, 7),
            Err(Error::Missing("current civic availability"))
        ));
        assert_eq!(
            count_civics(&[true, false, true], Some(&[false, true, true]), 7).unwrap(),
            8
        );
    }
}
