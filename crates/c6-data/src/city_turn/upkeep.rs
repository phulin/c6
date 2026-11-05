//! Normal City.TurnManager.DoTurnActivate28c8f50 over one canonical city.
//! PlayerCities' earlier activation prelude and later player maintenance are
//! separate stages. Policies select production; upkeep never invokes an AI.
pub mod buyable_plots;
pub mod citizens;
pub mod combat;
pub mod culture;
pub mod loyalty;

use super::{growth, Error, GrowthRules};
use crate::city_production_turn as production;
use c6_core::loaded_state::LoadedState;

pub trait Effects: production::Effects + growth::Effects {
    fn border_expansion(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<(), Error>;
    fn loyalty_upkeep(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<(), Error>;
    fn assign_citizens(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<(), Error>;
    fn restore_combat(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<(), Error>;
    fn activate_power(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<(), Error>;
}
/// A choose-production decision does not abort the remaining native upkeep.
/// Errors retain the real mutation prefix and must not be treated as completion.
pub fn do_turn_activate(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    growth_rules: &GrowthRules,
    effects: &mut impl Effects,
) -> Result<production::Outcome, Error> {
    state.city(owner, city)?;
    let outcome =
        production::do_turn_activate(state, owner, city, effects).map_err(Error::Production)?;
    growth::do_city_growth(state, owner, city, growth_rules, effects)?;
    effects.border_expansion(state, owner, city)?;
    effects.loyalty_upkeep(state, owner, city)?;
    Effects::assign_citizens(effects, state, owner, city)?;
    effects.restore_combat(state, owner, city)?;
    state.city_mut(owner, city)?.header.project_production = false;
    effects.activate_power(state, owner, city)?;
    Ok(outcome)
}
