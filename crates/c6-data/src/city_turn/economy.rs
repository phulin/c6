//! Concrete current City economy queries from canonical state and activated rules.
use super::{numeric, Error};
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    unit_initialization_definitions::collection,
    Rules, SqlValue,
};
use c6_core::movement_world::DefinitionLookup;
use c6_core::{city_production::execution, loaded_state::LoadedState};
#[derive(Clone, Debug)]
struct Happiness {
    minimum: Option<i32>,
    maximum: Option<i32>,
    growth: Option<f32>,
    nonfood: Option<f32>,
}
#[derive(Clone, Debug)]
struct Loyalty {
    minimum: i32,
    maximum: i32,
    growth: f32,
    yield_change: f32,
}
/// Static native-order definitions; current population/pressure/configuration
/// remain in their current owners and are never cached as query answers.
pub struct Definitions {
    yields: LinuxDefinitionRegistry,
    happiness: Vec<Happiness>,
    loyalty: Vec<Loyalty>,
    start_eras: LinuxDefinitionRegistry,
    eras: LinuxDefinitionRegistry,
    growth_rates: Vec<i32>,
    housing_zero: i32,
    housing_quarter: i32,
    housing_half: i32,
    population_per_amenity: i32,
    free_amenities: i32,
    science_percent: i32,
    culture_percent: i32,
    unassigned_gold_percent: i32,
}
fn int(value: &SqlValue, name: &'static str) -> Result<i32, Error> {
    crate::game_parameters::sql_integer(value).ok_or(Error::Unsupported(name))
}
fn float(value: &SqlValue, name: &'static str) -> Result<f32, Error> {
    numeric(value)
        .map(|v| v as f32)
        .ok_or(Error::Unsupported(name))
}
impl Definitions {
    pub fn from_rules(rules: &Rules) -> Result<Self, Error> {
        let table =
            |name: &'static str| rules.gameplay().table(name).ok_or(Error::Unsupported(name));
        let registry = |name, column| collection(rules, name, column).map_err(Error::Definitions);
        let parameter = |name: &'static str| -> Result<i32, Error> {
            let mut rows = table("GlobalParameters")?
                .rows()
                .filter(|row| row.text("Name").ok() == Some(name));
            let row = rows.next().ok_or(Error::Unsupported(name))?;
            if rows.next().is_some() {
                return Err(Error::Unsupported("duplicate economic parameter"));
            }
            int(row.get("Value").map_err(Error::Rules)?, name)
        };
        let happiness_registry = registry("Happinesses", "HappinessType")?;
        let happiness = happiness_registry
            .entries()
            .iter()
            .map(|entry| {
                let row = table("Happinesses")?
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Unsupported("Happiness row"))?;
                let optional_int = |name| -> Result<Option<i32>, Error> {
                    match row.get(name).map_err(Error::Rules)? {
                        SqlValue::Null => Ok(None),
                        value => int(value, name).map(Some),
                    }
                };
                let growth = match row.get("GrowthModifier").map_err(Error::Rules)? {
                    SqlValue::Null => None,
                    value => Some(float(value, "GrowthModifier")?),
                };
                Ok(Happiness {
                    minimum: optional_int("MinimumAmenityScore")?,
                    maximum: optional_int("MaximumAmenityScore")?,
                    growth,
                    nonfood: match row.get("NonFoodYieldModifier").map_err(Error::Rules)? {
                        SqlValue::Null => None,
                        value => Some(float(value, "NonFoodYieldModifier")?),
                    },
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let loyalty_registry = registry("LoyaltyLevels", "LoyaltyLevelType")?;
        let loyalty = loyalty_registry
            .entries()
            .iter()
            .map(|entry| {
                let row = table("LoyaltyLevels")?
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Unsupported("Loyalty row"))?;
                Ok(Loyalty {
                    minimum: int(row.get("LoyaltyMin").map_err(Error::Rules)?, "LoyaltyMin")?,
                    maximum: int(row.get("LoyaltyMax").map_err(Error::Rules)?, "LoyaltyMax")?,
                    growth: float(
                        row.get("GrowthChange").map_err(Error::Rules)?,
                        "GrowthChange",
                    )?,
                    yield_change: float(
                        row.get("YieldChange").map_err(Error::Rules)?,
                        "YieldChange",
                    )?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let start_eras = registry("StartEras", "EraType")?;
        let growth_rates = start_eras
            .entries()
            .iter()
            .map(|entry| {
                let row = table("StartEras")?
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Unsupported("StartEra row"))?;
                int(row.get("GrowthRate").map_err(Error::Rules)?, "GrowthRate")
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self {
            yields: registry("Yields", "YieldType")?,
            happiness,
            loyalty,
            start_eras,
            eras: registry("Eras", "EraType")?,
            growth_rates,
            housing_zero: parameter("CITY_HOUSING_LEFT_ZERO_GROWTH")?,
            housing_quarter: parameter("CITY_HOUSING_LEFT_25PCT_GROWTH")?,
            housing_half: parameter("CITY_HOUSING_LEFT_50PCT_GROWTH")?,
            population_per_amenity: parameter("CITY_POP_PER_AMENITY")?,
            free_amenities: parameter("CITY_AMENITIES_FOR_FREE")?,
            science_percent: parameter("SCIENCE_PERCENTAGE_YIELD_PER_POP")?,
            culture_percent: parameter("CULTURE_PERCENTAGE_YIELD_PER_POP")?,
            unassigned_gold_percent: parameter("GOLD_PERCENTAGE_YIELD_PER_UNASSIGNED_POP")?,
        })
    }
    pub fn housing_surplus_modifier(&self, surplus: i32) -> i32 {
        if surplus < self.housing_zero {
            0
        } else if surplus <= self.housing_quarter {
            64
        } else if surplus == self.housing_half {
            128
        } else {
            256
        }
    }
    pub fn amenities_needed(
        &self,
        population: i32,
        capabilities: &c6_core::game_capabilities::Membership,
    ) -> i32 {
        if !capabilities.has(0xc7762424) {
            return 0;
        }
        let divisor = self.population_per_amenity.max(1);
        let numerator = population.wrapping_add(divisor).wrapping_sub(1);
        (numerator / divisor)
            .wrapping_sub(self.free_amenities)
            .max(0)
    }
    /// First matching native definition, including inclusive optional bounds.
    pub fn happiness_from_surplus(&self, surplus: i32) -> i32 {
        self.happiness
            .iter()
            .position(|row| {
                row.minimum.is_none_or(|n| surplus >= n) && row.maximum.is_none_or(|n| surplus <= n)
            })
            .map_or(-1, |i| i as i32)
    }
    pub fn happiness_growth_modifier(&self, happiness: i32) -> i32 {
        self.happiness
            .get(happiness as u32 as usize)
            .and_then(|row| row.growth)
            .map_or(0, execution::from_float)
    }
    pub fn happiness_yield_modifier(&self, happiness: i32) -> i32 {
        self.happiness
            .get(happiness as u32 as usize)
            .and_then(|r| r.nonfood)
            .map_or(0, execution::from_float)
    }
    pub fn loyalty_level(&self, state: &LoadedState, owner: i32, city: u32) -> Result<i32, Error> {
        let loyalty = state.city(owner, city)?.cultural_identity.loyalty_raw_q8 >> 8;
        Ok(self
            .loyalty
            .iter()
            .position(|row| loyalty <= row.maximum && loyalty >= row.minimum)
            .map_or(-1, |i| i as i32))
    }
    pub fn loyalty_growth_modifier(
        &self,
        state: &LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<i32, Error> {
        let level = self.loyalty_level(state, owner, city)?;
        Ok(self
            .loyalty
            .get(level as u32 as usize)
            .map_or(256, |row| execution::from_float(row.growth)))
    }
    pub fn loyalty_yield_modifier(
        &self,
        state: &LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<i32, Error> {
        let level = self.loyalty_level(state, owner, city)?;
        Ok(self
            .loyalty
            .get(level as u32 as usize)
            .map_or(0, |row| loyalty_percentage(row.yield_change)))
    }
    pub fn other_growth_modifier(
        &self,
        state: &LoadedState,
        owner: i32,
        city: u32,
        configuration: &crate::loaded_actions::configuration::Configuration<'_>,
    ) -> Result<i32, Error> {
        configuration
            .require_binding(state.binding())
            .map_err(|_| Error::Unsupported("current growth configuration owner"))?;
        let era = configuration
            .game_fields()
            .start_era
            .ok_or(Error::Unsupported("current start era"))?;
        let index = if era > 0x7ff {
            self.eras
                .find_index(era)
                .map(|i| i as u32)
                .unwrap_or(u32::MAX)
        } else {
            era
        };
        let rate = self
            .start_eras
            .find_index(index)
            .and_then(|i| self.growth_rates.get(i))
            .copied()
            .unwrap_or(1);
        Ok(state
            .city(owner, city)?
            .growth
            .other_growth_modifier_raw_q8
            .wrapping_add(execution::from_float(
                rate as f32 * f32::from_bits(0x3c23d70a),
            )))
    }
    /// GetYieldFromPopulation: integer percent multiplication/division happens
    /// in Q8 before adding the current per-population Q8 modifier.
    pub fn population_yield(
        &self,
        state: &LoadedState,
        owner: i32,
        city: u32,
        yield_index: i32,
    ) -> Result<i32, Error> {
        let definition = self
            .yields
            .type_hash(yield_index as u32 as usize)
            .ok_or(Error::Unsupported("current Yield index"))?;
        let city = state.city(owner, city)?;
        let population = city.header.population;
        let (count, percent) = match definition {
            0x3a81f8ab => (population, self.science_percent),
            0xeb3f55e8 => (population, self.culture_percent),
            0xb761634a => (
                city.citizens.num_unassigned_citizens.max(0),
                self.unassigned_gold_percent,
            ),
            _ => (0, 0),
        };
        let base = population_percent(count, percent);
        let extra = city
            .header
            .extra_yield_per_population_q8
            .iter()
            .rfind(|row| row.definition_hash == definition)
            .map(|row| row.value)
            .ok_or(Error::Unsupported("current extra yield per population"))?;
        Ok(base.wrapping_add(execution::multiply(extra, population.wrapping_shl(8))))
    }
    pub fn yield_index(&self, name: &[u8]) -> Option<i32> {
        self.yields
            .find_index(linux_type_hash(name))
            .map(|i| i as i32)
    }
}

fn population_percent(count: i32, percent: i32) -> i32 {
    let product = execution::multiply(count.wrapping_shl(8), percent.wrapping_shl(8));
    (i64::from(product) * 256 / 25600) as i32
}
fn loyalty_percentage(value: f32) -> i32 {
    let raw = execution::from_float(value);
    let percent = f64::from(raw as f32 / 256.0) * 100.0;
    let integer = if percent.is_finite() && (-2147483648.0..2147483648.0).contains(&percent) {
        percent as i32
    } else {
        i32::MIN
    };
    integer.wrapping_shl(8)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn rules() -> Definitions {
        Definitions {
            yields: LinuxDefinitionRegistry::from_native_order(vec![]),
            happiness: vec![
                Happiness {
                    minimum: None,
                    maximum: Some(-1),
                    growth: Some(-15.0),
                    nonfood: None,
                },
                Happiness {
                    minimum: Some(0),
                    maximum: Some(2),
                    growth: Some(0.0),
                    nonfood: None,
                },
                Happiness {
                    minimum: Some(2),
                    maximum: None,
                    growth: Some(20.0),
                    nonfood: None,
                },
            ],
            loyalty: vec![],
            start_eras: LinuxDefinitionRegistry::from_native_order(vec![]),
            eras: LinuxDefinitionRegistry::from_native_order(vec![]),
            growth_rates: vec![],
            housing_zero: -4,
            housing_quarter: 0,
            housing_half: 1,
            population_per_amenity: 2,
            free_amenities: 0,
            science_percent: 50,
            culture_percent: 30,
            unassigned_gold_percent: 50,
        }
    }
    #[test]
    fn loyalty_rounds_to_q8_before_integer_percentage() {
        assert_eq!(loyalty_percentage(0.101), 9 * 256);
        assert_eq!(loyalty_percentage(-0.101), -10 * 256);
        assert_eq!(loyalty_percentage(0.001), 0);
        assert_eq!(loyalty_percentage(-0.25), -25 * 256);
    }
    #[test]
    fn housing_zero_boundary_is_strict_and_half_reduction_is_exact_equality() {
        let r = rules();
        assert_eq!(
            [-5, -4, -1, 0, 1, 2].map(|n| r.housing_surplus_modifier(n)),
            [0, 64, 64, 64, 128, 256]
        );
    }
    #[test]
    fn happiness_keeps_optional_bounds_and_first_overlap_in_definition_order() {
        let r = rules();
        assert_eq!(
            [-9, 0, 2, 3].map(|n| r.happiness_from_surplus(n)),
            [0, 1, 1, 2]
        );
        assert_eq!(r.happiness_growth_modifier(0), -3840);
        assert_eq!(r.happiness_growth_modifier(-1), 0);
    }
    #[test]
    fn amenities_ceil_division_and_capability_short_circuit() {
        let r = rules();
        let caps = c6_core::game_capabilities::Membership::from_sorted(vec![0xc7762424]).unwrap();
        assert_eq!(
            [0, 1, 2, 3].map(|n| r.amenities_needed(n, &caps)),
            [0, 1, 1, 2]
        );
        assert_eq!(
            r.amenities_needed(
                100,
                &c6_core::game_capabilities::Membership::from_sorted(vec![]).unwrap()
            ),
            0
        );
    }
    #[test]
    fn population_yield_keeps_fractional_q8_and_signed_division() {
        assert_eq!(population_percent(1, 50), 128);
        assert_eq!(population_percent(3, 30), 230);
        assert_eq!(population_percent(-3, 30), -230);
    }
}
