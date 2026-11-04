//! City.Growth.DoCityGrowth over canonical city fields and current query effects.
use super::{add_growth, Error, GrowthRules, PopulationEffects};
use c6_core::loaded_state::LoadedState;

/// Queries must compute from this same current state/rules and preserve reached
/// subscribers; native answer caches are not a production implementation.
pub trait Effects: PopulationEffects {
    fn food_yield_q8(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<i32, Error>;
    fn other_growth_modifier_q8(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<i32, Error>;
    fn happiness_growth_modifier_q8(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<i32, Error>;
    fn housing_growth_modifier_q8(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<i32, Error>;
    fn loyalty_growth_modifier_q8(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<i32, Error>;
    fn amenities(&mut self, state: &mut LoadedState, owner: i32, city: u32) -> Result<i32, Error>;
    fn amenities_needed(&mut self, population: i32) -> Result<i32, Error>;
    /// Ordinary canonical notification creation/publication, not presentation.
    fn growth_notification(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        notification: u32,
    ) -> Result<(), Error>;
}
fn product(a: i32, b: i32) -> i32 {
    ((i64::from(a) * i64::from(b)) >> 8) as i32
}
/// GetFoodSurplus reads population AFTER City.GetYield has returned.
pub fn food_surplus(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    rules: &GrowthRules,
    effects: &mut impl Effects,
) -> Result<i32, Error> {
    let yield_raw = effects.food_yield_q8(state, owner, city)?;
    let population = state.city(owner, city)?.header.population;
    let consumption = population
        .wrapping_mul(rules.food_consumption_per_population)
        .wrapping_shl(8);
    Ok(yield_raw.wrapping_sub(consumption))
}
/// The housing/loyalty queries are skipped if the pre-housing modifier is
/// negative, exactly as the native GetOverallGrowthModifier short circuit.
pub fn overall_modifier(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    effects: &mut impl Effects,
) -> Result<i32, Error> {
    let happiness = effects.happiness_growth_modifier_q8(state, owner, city)?;
    let other = effects.other_growth_modifier_q8(state, owner, city)?;
    let modifier = 256i32.wrapping_add(happiness / 100).wrapping_add(other);
    if modifier < 0 {
        return Ok(0);
    }
    let housing = effects.housing_growth_modifier_q8(state, owner, city)?;
    let modifier = product(modifier, housing);
    let loyalty = effects.loyalty_growth_modifier_q8(state, owner, city)?;
    Ok(product(modifier, loyalty))
}
fn due(turn: i32, last: i32) -> bool {
    turn >= last.wrapping_add(10)
}

/// 288a2b0. Food/population/religion updates precede notification checks.
/// Query/report errors preserve the already executed canonical prefix.
pub fn do_city_growth(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    rules: &GrowthRules,
    effects: &mut impl Effects,
) -> Result<(), Error> {
    let mut surplus = food_surplus(state, owner, city, rules, effects)?;
    if surplus > 0 {
        surplus = product(surplus, overall_modifier(state, owner, city, effects)?);
    }
    add_growth(state, owner, city, surplus, rules, effects)?;
    let turn = state.retained().core.current_turn;
    if surplus < 0 {
        if due(
            turn,
            state
                .city(owner, city)?
                .growth
                .turn_last_notified_starvation,
        ) {
            state
                .city_mut(owner, city)?
                .growth
                .turn_last_notified_starvation = turn;
            effects.growth_notification(state, owner, city, 0xba4af0fb)?;
        }
    } else if surplus == 0 {
        let food_hash = crate::registry::linux_type_hash(b"YIELD_FOOD");
        let favored = state
            .city(owner, city)?
            .citizens
            .favored_yields
            .iter()
            .find(|p| p.definition_hash == food_hash)
            .map(|p| p.value)
            .ok_or(Error::Unsupported("current favored Food yield"))?;
        if !favored
            && due(
                turn,
                state
                    .city(owner, city)?
                    .growth
                    .turn_last_notified_food_focus,
            )
        {
            state
                .city_mut(owner, city)?
                .growth
                .turn_last_notified_food_focus = turn;
            effects.growth_notification(state, owner, city, 0x9b27fd19)?;
        }
    }
    let housing = effects.housing_growth_modifier_q8(state, owner, city)?;
    if housing < 256
        && due(
            turn,
            state
                .city(owner, city)?
                .growth
                .turn_last_notified_need_housing,
        )
    {
        state
            .city_mut(owner, city)?
            .growth
            .turn_last_notified_need_housing = turn;
        effects.growth_notification(state, owner, city, 0x9efbc217)?;
    }
    let amenities = effects.amenities(state, owner, city)?;
    let population = state.city(owner, city)?.header.population;
    let needed = effects.amenities_needed(population)?;
    if amenities < needed
        && due(
            turn,
            state
                .city(owner, city)?
                .growth
                .turn_last_notified_need_amenities,
        )
    {
        state
            .city_mut(owner, city)?
            .growth
            .turn_last_notified_need_amenities = turn;
        effects.growth_notification(state, owner, city, 0xffad0d16)?;
    }
    Ok(())
}
