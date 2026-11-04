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
        Ok(Self { categories: result })
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
#[derive(Default)]
pub struct GlobalInputs {
    pub civilization_levels: Vec<(i32, u32)>,
    pub great_people_received: Option<Vec<(i32, i32)>>,
    pub current_era_scores: Option<Vec<(i32, Vec<i32>)>>,
}
pub struct Runtime {
    pub binding: WireBinding,
    pub caches: Vec<(i32, Cache)>,
    pub globals: GlobalInputs,
}
impl Runtime {
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
            |kind| current_raw(state, &self.globals, owner, kind),
        )
    }
}
fn current_raw(
    state: &LoadedState,
    global: &GlobalInputs,
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
        Raw::Buildings | Raw::Wonders if cities.is_empty() => Ok(0),
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
            let active = state
                .world()
                .civic_availability
                .as_ref()
                .ok_or(Error::Missing("current civic availability"))?;
            if flags.len() != active.len() {
                return Err(Error::Missing("complete current civic availability"));
            }
            Ok(flags
                .iter()
                .zip(active)
                .fold(player.culture.num_civics_repeated, |n, (flag, active)| {
                    n.wrapping_add(i32::from(*flag && *active))
                }))
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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_order_rounding_and_failure_prefix() {
        let defs = Definitions {
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
}
