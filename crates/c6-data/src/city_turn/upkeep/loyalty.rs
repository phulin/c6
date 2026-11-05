//! Normal non-Free-City CulturalIdentity activation28737e0. Current pressure
//! queries reuse the founding kernel; changed signals use the same requirement
//! and modifier owner. Free-city accumulation remains a distinct unsupported arm.
use crate::{
    city_turn::{economy, Error},
    found_city::cultural_identity as identity,
    unit_initialization_definitions::collection,
    Rules,
};
use c6_core::{
    core_player::initialization::WireBinding, loaded_state::LoadedState,
    movement_world::DefinitionLookup,
};
pub struct Definitions {
    binding: WireBinding,
    pub identity: identity::Definitions,
    economy: economy::Definitions,
    levels: crate::registry::LinuxDefinitionRegistry,
    maximum: i32,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let maximum = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Unsupported("GlobalParameters"))?
            .rows()
            .find(|r| r.text("Name").ok() == Some("LOYALTY_MAXIMUM"))
            .and_then(|r| r.get("Value").ok())
            .and_then(crate::game_parameters::sql_integer)
            .ok_or(Error::Unsupported("LOYALTY_MAXIMUM"))?;
        Ok(Self {
            binding: state.binding().clone(),
            identity: identity::Definitions::from_rules(state, rules)?,
            economy: economy::Definitions::from_rules(rules)?,
            levels: collection(rules, "LoyaltyLevels", "LoyaltyLevelType")
                .map_err(Error::Definitions)?,
            maximum,
        })
    }
}
pub trait Effects: identity::Context {
    fn loyalty_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<(), Error>;
    fn loyalty_report(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<(), Error>;
    fn losing_loyalty_notification(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<(), Error>;
}
pub fn do_turn_activate(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    defs: &Definitions,
    effects: &mut impl Effects,
) -> Result<(), Error> {
    if state.binding() != &defs.binding {
        return Err(Error::Unsupported("loyalty rules binding"));
    }
    if effects.civilization_level(owner)? == 0x04e6b0a0 {
        return Err(Error::Unsupported("FreeCity identity accumulation"));
    }
    let old_trend = state
        .city(owner, city)?
        .cultural_identity
        .conversion_outcome;
    let old_loyalty = state.city(owner, city)?.cultural_identity.loyalty_raw_q8;
    let always = identity::always_loyal(state, owner, city)?;
    let change = if always {
        defs.maximum.wrapping_shl(8).wrapping_sub(old_loyalty)
    } else {
        let mut rate = 0i32;
        for source in effects.alive_ids().to_vec() {
            let value =
                identity::identity_for(state, &defs.identity, effects, owner, city, source)?;
            rate = if source == owner {
                rate.wrapping_add(value)
            } else {
                rate.wrapping_sub(value)
            };
        }
        rate.wrapping_add(identity::pressure(
            state,
            &defs.identity,
            effects,
            owner,
            city,
        )?)
    };
    if change != 0 {
        let next = old_loyalty
            .wrapping_add(change)
            .min(defs.maximum.wrapping_shl(8))
            .max(0);
        let c = &mut state.city_mut(owner, city)?.cultural_identity;
        c.loyalty_raw_q8 = next;
        c.last_loyalty_change_raw_q8 = change;
        if next <= 0 {
            return Err(Error::Unsupported("loyalty potential transfer player"));
        }
        c.transfer_to = -1;
        effects.loyalty_changed(state, owner, city)?;
        effects.loyalty_report(state, owner, city)?;
        identity::do_city_created(state, &defs.identity, effects, owner, city)?;
    }
    if always {
        let c = &mut state.city_mut(owner, city)?.cultural_identity;
        c.conversion_outcome = Some(0);
        c.turns_to_conversion = Some(-1);
    } else {
        let index = defs.economy.loyalty_level(state, owner, city)?;
        let hash = if index < 0 {
            u32::MAX
        } else {
            defs.levels
                .type_hash(index as usize)
                .ok_or(Error::Unsupported("loyalty level identity"))?
        };
        state
            .city_mut(owner, city)?
            .cultural_identity
            .loyalty_level_hash = hash;
    }
    identity::do_city_created(state, &defs.identity, effects, owner, city)?;
    let c = state.city(owner, city)?;
    if (old_trend != c.cultural_identity.conversion_outcome
        && c.cultural_identity.conversion_outcome == Some(1))
        || (old_loyalty > (defs.maximum / 4).wrapping_shl(8)
            && c.cultural_identity.loyalty_raw_q8 <= (defs.maximum / 4).wrapping_shl(8))
    {
        effects.losing_loyalty_notification(state, owner, city)?;
    }
    Ok(())
}
