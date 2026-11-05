//! Ordinary gold building purchase: current cost, Treasury, shared CreateBuilding.
use super::{cell, discounted, player, Definitions, Error, Rejection};
use crate::city_production_turn::{buildings, selection};
use c6_core::{loaded_state::LoadedState, unit::ComponentId};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Rejected(Rejection),
    Purchased { created: bool, gold_cost: i32 },
}
/// Static current-speed building definitions are the same owner used by turns.
pub fn gold_cost<E>(
    state: &LoadedState,
    defs: &Definitions,
    buildings: &buildings::Definitions,
    owner: i32,
    city: u32,
    building: u32,
) -> Result<i32, Error<E>> {
    defs.check(state)?;
    buildings.validate(state).map_err(Error::Production)?;
    let definition = buildings.building(building).map_err(Error::Production)?;
    let c = state.city(owner, city).map_err(Error::State)?;
    let p = player(state, owner)?;
    let base = buildings.adjust_cost(
        definition
            .base_cost
            .wrapping_mul(defs.multiplier)
            .wrapping_mul(defs.gold_equivalent),
    );
    let local = cell(&c.gold.building_cost_modifier, building)?;
    let government = if !defs.discount_defined || p.culture.anarchy_end_turn >= 0 {
        0
    } else {
        cell(&p.bonuses.flat_bonuses, 0x7530bd65)?
    };
    //2887ef4 tests signed majority>0 before the live religion-specific discount.
    if (c.religion.majority_religion_hash as i32) > 0 {
        return Err(Error::Missing(
            "religion-enabled building purchase discount",
        ));
    }
    let modifier = local.wrapping_add(government);
    // Building discounts apply only when positive; Unit discounts use nonzero.
    let price = if modifier > 0 {
        discounted(base, modifier)
    } else {
        base
    };
    Ok(price.wrapping_sub(price.wrapping_rem(defs.divisor)))
}
pub trait Effects {
    type Error: std::fmt::Debug;
    fn gold_changed(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Self::Error>;
    fn create_building(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        building: u32,
    ) -> Result<bool, Self::Error>;
    fn purchased_report(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        building: u32,
    ) -> Result<(), Self::Error>;
    fn command_complete(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
    ) -> Result<(), Self::Error>;
}
#[allow(clippy::too_many_arguments)]
pub fn purchase<E: Effects, Q: selection::Queries>(
    state: &mut LoadedState,
    defs: &Definitions,
    buildings: &buildings::Definitions,
    choices: &selection::Definitions,
    current: &selection::Current<'_>,
    queries: &Q,
    owner: i32,
    city: u32,
    building: u32,
    effects: &mut E,
) -> Result<Outcome, Error<E::Error>> {
    defs.check(state)?;
    let c = state.city(owner, city).map_err(Error::State)?;
    if c.buildings
        .building_locations
        .iter()
        .find(|v| v.definition_hash == building)
        .is_some_and(|v| v.value != -1)
    {
        return Ok(Outcome::Rejected(Rejection::AlreadyPresent));
    }
    if !selection::can_construct(
        state, owner, city, building, choices, buildings, current, queries,
    )
    .map_err(Error::Production)?
    {
        return Ok(Outcome::Rejected(Rejection::CannotConstruct));
    }
    let cost = gold_cost(state, defs, buildings, owner, city, building)?;
    let p = player(state, owner)?;
    if cost > p.treasury.gold_balance_q8 >> 8 {
        return Ok(Outcome::Rejected(Rejection::InsufficientGold));
    }
    // The 100-percent CreateBuilding path owns ordinary no-queue creation. A
    // matching queued building needs its native removal/progress-recovery path.
    if !c.build_queue.entries.is_empty() {
        return Err(Error::Missing(
            "purchase current queue removal/revalidation",
        ));
    }
    let next = p
        .treasury
        .gold_balance_q8
        .saturating_add(cost.wrapping_neg().wrapping_shl(8));
    if next != p.treasury.gold_balance_q8 {
        state
            .treasury_mut(owner)
            .map_err(Error::State)?
            .gold_balance_q8 = next;
        effects.gold_changed(state, owner).map_err(Error::Effect)?;
    }
    let city = ComponentId {
        owner: owner as i16,
        component_type: 2,
        id: city,
    };
    let created = effects
        .create_building(state, city, building)
        .map_err(Error::Effect)?;
    effects
        .purchased_report(state, city, building)
        .map_err(Error::Effect)?;
    effects
        .command_complete(state, city)
        .map_err(Error::Effect)?;
    Ok(Outcome::Purchased {
        created,
        gold_cost: cost,
    })
}
