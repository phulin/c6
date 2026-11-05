//! Culture.DoBorderExpansion2880290. The tail selects the next buyable plot
//! even below the acquisition threshold; callers must preserve that RNG work.
use crate::{
    city_turn::{Error, GrowthRules},
    game_parameters::sql_integer,
    Rules,
};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};
pub struct Definitions {
    binding: WireBinding,
    first: i32,
    multiplier: i32,
    exponent: f32,
    speed: i32,
    levels: crate::registry::LinuxDefinitionRegistry,
    may_annex: Vec<bool>,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        growth: &GrowthRules,
    ) -> Result<Self, Error> {
        let table = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Unsupported("GlobalParameters"))?;
        let get = |key| {
            table
                .rows()
                .find(|r| r.text("Name").ok() == Some(key))
                .and_then(|r| r.get("Value").ok())
                .ok_or(Error::Unsupported(key))
        };
        let levels = crate::unit_initialization_definitions::collection(
            rules,
            "CivilizationLevels",
            "CivilizationLevelType",
        )
        .map_err(Error::Definitions)?;
        let table = rules
            .gameplay()
            .table("CivilizationLevels")
            .ok_or(Error::Unsupported("CivilizationLevels"))?;
        let may_annex = levels
            .entries()
            .iter()
            .map(|entry| {
                let row = table
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Unsupported("civilization level row"))?;
                sql_integer(row.get("CanAnnexTilesWithCulture").map_err(Error::Rules)?)
                    .map(|v| v != 0)
                    .ok_or(Error::Unsupported("CanAnnexTilesWithCulture"))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self {
            binding: state.binding().clone(),
            first: sql_integer(get("CULTURE_COST_FIRST_PLOT")?)
                .ok_or(Error::Unsupported("first plot culture cost"))?,
            multiplier: sql_integer(get("CULTURE_COST_LATER_PLOT_MULTIPLIER")?)
                .ok_or(Error::Unsupported("later plot culture multiplier"))?,
            exponent: super::super::numeric(get("CULTURE_COST_LATER_PLOT_EXPONENT")?)
                .ok_or(Error::Unsupported("plot culture exponent"))? as f32,
            speed: growth.parameters.speed_cost_multiplier,
            levels,
            may_annex,
        })
    }
    pub fn may_annex_with_culture(&self, level: u32) -> bool {
        self.levels
            .find_index(level)
            .is_some_and(|i| self.may_annex[i])
    }
    fn cost(&self, count: i32) -> i32 {
        let value = (count.wrapping_mul(self.multiplier) as f64).powf(f64::from(self.exponent));
        let power = if value.is_finite() && (-2147483648.0..2147483648.0).contains(&value) {
            value as i32
        } else {
            i32::MIN
        };
        let raw = power.wrapping_add(self.first).wrapping_shl(8) as i64;
        ((raw * i64::from(self.speed.wrapping_shl(8)) / 25_600)
            .clamp(i32::MIN as i64, i32::MAX as i64) as i32)
            >> 8
    }
}
pub trait Effects {
    fn enabled(&self) -> bool;
    fn may_annex_with_culture(&self, owner: i32) -> Result<bool, Error>;
    fn culture_yield_q8(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<i32, Error>;
    /// Current cached candidate validation and actual native tie-breaking draw.
    fn next_buyable_plot(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<Option<u32>, Error>;
    fn plot_is_unowned(&self, state: &LoadedState, plot: u32) -> Result<bool, Error>;
    fn annex_plot(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        plot: u32,
    ) -> Result<(), Error>;
}
pub fn do_border_expansion(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    defs: &Definitions,
    effects: &mut impl Effects,
) -> Result<(), Error> {
    if state.binding() != &defs.binding {
        return Err(Error::Unsupported("border rules binding"));
    }
    if !effects.enabled() || state.city(owner, city)?.culture.no_culture_border_expansion {
        return Ok(());
    }
    if effects.may_annex_with_culture(owner)? {
        let mut value = effects.culture_yield_q8(state, owner, city)?;
        let modifier = state.city(owner, city)?.culture.border_expansion_modifier;
        if modifier != 0 {
            let product = c6_core::city_production::execution::multiply(
                value,
                modifier.wrapping_add(100).wrapping_shl(8),
            );
            value = ((i64::from(product) * 256) / 25_600) as i32;
        }
        let c = &mut state.city_mut(owner, city)?.culture;
        c.culture_raw_q8 = c.culture_raw_q8.wrapping_add(value);
        let cost = defs.cost(c.plots_acquired_from_culture).wrapping_shl(8);
        if c.culture_raw_q8 >= cost {
            c.culture_raw_q8 = c.culture_raw_q8.wrapping_sub(cost);
            let cached = state.city(owner, city)?.culture.next_buyable_plot;
            let plot = if cached >= 0 && effects.plot_is_unowned(state, cached as u32)? {
                Some(cached as u32)
            } else {
                effects.next_buyable_plot(state, owner, city)?
            };
            if let Some(plot) = plot {
                effects.annex_plot(state, owner, city, plot)?;
                let c = &mut state.city_mut(owner, city)?.culture;
                c.plots_acquired_from_culture = c.plots_acquired_from_culture.wrapping_add(1);
            }
        }
    }
    state.city_mut(owner, city)?.culture.next_buyable_plot = -1;
    if let Some(plot) = effects.next_buyable_plot(state, owner, city)? {
        state.city_mut(owner, city)?.culture.next_buyable_plot = plot as i32;
    }
    Ok(())
}
