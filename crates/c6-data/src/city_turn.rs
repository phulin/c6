//! Current City growth updates over canonical LoadedState and cached rules.
//! Production, citizen assignment, religion and synchronous report subscribers
//! are shared turn/founding effects, never substituted by a population scalar.
use crate::{unit_initialization_definitions::collection, Rules, RulesProfile, SqlValue};
use c6_core::{city_growth, loaded_state::LoadedState};
pub mod religion;

#[derive(Debug)]
pub enum Error {
    Rules(crate::Error),
    Definitions(crate::unit_initialization_definitions::Error),
    State(c6_core::loaded_state::Error),
    Unsupported(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "city turn: {self:?}")
    }
}
impl std::error::Error for Error {}
impl From<c6_core::loaded_state::Error> for Error {
    fn from(v: c6_core::loaded_state::Error) -> Self {
        Self::State(v)
    }
}

/// Immutable parameters from the actual activated rules; no per-city answer
/// cache. A present unsupported SQL scalar or missing row remains an error.
#[derive(Clone, Debug)]
pub struct GrowthRules {
    religions: crate::registry::LinuxDefinitionRegistry,
    pub atheism_pressure_per_pop: i32,
    pub adjacent_religion_distance: i32,
    pub parameters: city_growth::Parameters,
}
fn numeric(value: &SqlValue) -> Option<f64> {
    match value {
        SqlValue::Integer(v) => Some(*v as f64),
        SqlValue::RealBits(v) => Some(f64::from_bits(*v)),
        SqlValue::Text(v) => std::str::from_utf8(v).ok()?.trim().parse().ok(),
        _ => None,
    }
}
impl GrowthRules {
    pub fn from_rules(rules: &Rules, game_speed_type: u32) -> Result<Self, Error> {
        if !matches!(
            rules.profile(),
            RulesProfile::LinuxXp2Base | RulesProfile::LinuxXp2ActivatedDiagnostic
        ) {
            return Err(Error::Unsupported("Linux growth rules"));
        }
        let parameters = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Unsupported("GlobalParameters"))?;
        let get = |name: &'static str| -> Result<f64, Error> {
            let mut found = None;
            for row in parameters.rows() {
                if row.text("Name").map_err(Error::Rules)? == name {
                    if found.is_some() {
                        return Err(Error::Unsupported("duplicate growth parameter"));
                    }
                    found = Some(
                        numeric(row.get("Value").map_err(Error::Rules)?)
                            .filter(|v| v.is_finite())
                            .ok_or(Error::Unsupported(name))?,
                    );
                }
            }
            found.ok_or(Error::Unsupported(name))
        };
        let int = |name| {
            let value = get(name)?;
            if value.fract() != 0.0 || value < i32::MIN as f64 || value > i32::MAX as f64 {
                Err(Error::Unsupported(name))
            } else {
                Ok(value as i32)
            }
        };
        let speeds =
            collection(rules, "GameSpeeds", "GameSpeedType").map_err(Error::Definitions)?;
        let speed = speeds.find(game_speed_type);
        let speed_cost_multiplier = match speed {
            None => 100,
            Some(row) => {
                let row = rules
                    .gameplay()
                    .table("GameSpeeds")
                    .and_then(|t| t.row_by_id(row.database_rowid))
                    .ok_or(Error::Unsupported("GameSpeeds row"))?;
                match row.get("CostMultiplier").map_err(Error::Rules)? {
                    SqlValue::Integer(v) => {
                        i32::try_from(*v).map_err(|_| Error::Unsupported("CostMultiplier"))?
                    }
                    _ => return Err(Error::Unsupported("CostMultiplier")),
                }
            }
        };
        Ok(Self {
            religions: collection(rules, "Religions", "ReligionType")
                .map_err(Error::Definitions)?,
            atheism_pressure_per_pop: int("RELIGION_SPREAD_ATHEISM_PRESSURE_PER_POP")?,
            adjacent_religion_distance: int("RELIGION_SPREAD_ADJACENT_CITY_DISTANCE")?,
            parameters: city_growth::Parameters {
                base: int("CITY_GROWTH_THRESHOLD")?,
                multiplier: int("CITY_GROWTH_MULTIPLIER")?,
                exponent: get("CITY_GROWTH_EXPONENT")? as f32,
                speed_cost_multiplier,
            },
        })
    }
}

pub const POPULATION_REPORT: u32 = 0x10419f1f;
/// Services execute against this same canonical state/Effects/Script owner.
/// Implementations must perform every reached subscriber and RNG effect.
pub trait PopulationEffects: religion::Effects {
    /// ChangeNumCitizens already added delta to the canonical unassigned count.
    /// `reallocate` selects the native negative-count branch; otherwise calls
    /// TryAssignAllWorkers(true), including its actual plot/yield selection.
    fn assign_citizens(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        reallocate: bool,
    ) -> Result<(), Error>;
    fn population_changed_signal(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        delta: i32,
    ) -> Result<(), Error>;
    fn report_population(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        event: u32,
        population: i32,
    ) -> Result<(), Error>;
}
/// 2894cf0: a nonpositive wrapping result returns without writes or callbacks.
/// The final report rereads population after all synchronous subscribers.
pub fn change_population(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    delta: i32,
    rules: &GrowthRules,
    effects: &mut impl PopulationEffects,
) -> Result<(), Error> {
    let next = state
        .city(owner, city)?
        .header
        .population
        .wrapping_add(delta);
    if next <= 0 {
        return Ok(());
    }
    state.city_mut(owner, city)?.header.population = next;
    let unassigned = state
        .city(owner, city)?
        .citizens
        .num_unassigned_citizens
        .wrapping_add(delta);
    state
        .city_mut(owner, city)?
        .citizens
        .num_unassigned_citizens = unassigned;
    effects.assign_citizens(state, owner, city, unassigned < 0)?;
    religion::population_changed(state, owner, city, delta, rules, effects)?;
    effects.population_changed_signal(state, owner, city, delta)?;
    let population = state.city(owner, city)?.header.population;
    effects.report_population(state, owner, city, POPULATION_REPORT, population)
}
struct GrowthContext<'a, E> {
    state: &'a mut LoadedState,
    owner: i32,
    city: u32,
    rules: &'a GrowthRules,
    effects: &'a mut E,
}
impl<E: PopulationEffects> city_growth::World for GrowthContext<'_, E> {
    type Error = Error;
    fn food_raw(&mut self) -> Result<i32, Error> {
        Ok(self.state.city(self.owner, self.city)?.growth.food_raw_q8)
    }
    fn set_food_raw(&mut self, value: i32) -> Result<(), Error> {
        self.state
            .city_mut(self.owner, self.city)?
            .growth
            .food_raw_q8 = value;
        Ok(())
    }
    fn population(&mut self) -> Result<i32, Error> {
        Ok(self.state.city(self.owner, self.city)?.header.population)
    }
    fn growth_threshold(&mut self, population: i32) -> Result<i32, Error> {
        Ok(self.rules.parameters.threshold(population))
    }
    fn change_population(&mut self, delta: i32) -> Result<(), Error> {
        change_population(
            self.state,
            self.owner,
            self.city,
            delta,
            self.rules,
            self.effects,
        )
    }
}
pub fn add_growth(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    delta_raw_q8: i32,
    rules: &GrowthRules,
    effects: &mut impl PopulationEffects,
) -> Result<(), Error> {
    state.city(owner, city)?;
    city_growth::add_growth(
        &mut GrowthContext {
            state,
            owner,
            city,
            rules,
            effects,
        },
        delta_raw_q8,
    )
}
