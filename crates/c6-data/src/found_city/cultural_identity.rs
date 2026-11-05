//! Founding CulturalIdentity.DoCityCreated2872f00 / conversion trends2872f10.
//! Current single-city pressure is computed from its real population, ownership,
//! age and modifier fields. Nearby multi-city pressure remains a separate branch.
use crate::{
    city_turn::Error, game_parameters::sql_integer, unit_initialization_definitions::collection,
    Rules,
};
use c6_core::{
    city::City, core_player::initialization::WireBinding, loaded_components::RetainedPlayer,
    loaded_state::LoadedState,
};
use std::collections::BTreeMap;
pub struct Definitions {
    binding: WireBinding,
    parameters: BTreeMap<String, i32>,
    happiness_loyalty: Vec<i32>,
}
impl Definitions {
    /// Identity of the loaded state used to build these loyalty definitions.
    pub fn binding(&self) -> &WireBinding {
        &self.binding
    }
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let mut parameters = BTreeMap::new();
        for row in rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Unsupported("loyalty parameters"))?
            .rows()
        {
            let Ok(name) = row.text("Name") else { continue };
            if let Some(value) = row.get("Value").ok().and_then(sql_integer) {
                parameters.entry(name.to_owned()).or_insert(value);
            }
        }
        let happiness =
            collection(rules, "Happinesses", "HappinessType").map_err(Error::Definitions)?;
        let xp = rules
            .gameplay()
            .table("Happinesses_XP1")
            .ok_or(Error::Unsupported("Happinesses_XP1"))?;
        let mut happiness_loyalty = vec![];
        for h in happiness.entries() {
            let row = xp.rows().find(|r| {
                r.text("HappinessType").ok().map(|s| s.as_bytes()) == Some(h.type_name.as_slice())
            });
            // Native Find(hash)==nullptr returns0.
            happiness_loyalty.push(match row {
                None => 0,
                Some(r) => r
                    .get("IdentityPerTurnChange")
                    .ok()
                    .and_then(sql_integer)
                    .ok_or(Error::Unsupported("happiness loyalty definition"))?,
            });
        }
        Ok(Self {
            binding: state.binding().clone(),
            parameters,
            happiness_loyalty,
        })
    }
    fn parameter(&self, name: &'static str) -> Result<i32, Error> {
        self.parameters
            .get(name)
            .copied()
            .ok_or(Error::Unsupported(name))
    }
    fn q8(&self, name: &'static str) -> Result<i32, Error> {
        Ok(self.parameter(name)?.wrapping_shl(8))
    }
}
/// Each numeric method delegates to existing current gameplay query kernels,
/// rather than importing native GetLoyalty/Test answers. The property method
/// queries the same mutable player property owner used by gameplay effects.
pub trait Context {
    fn alive_ids(&self) -> &[i32];
    fn civilization_level(&self, owner: i32) -> Result<u32, Error>;
    fn food_surplus_q8(&mut self, state: &LoadedState, owner: i32, city: u32)
        -> Result<i32, Error>;
    fn happiness(&mut self, state: &LoadedState, owner: i32, city: u32) -> Result<i32, Error>;
    fn player_property_int(
        &mut self,
        state: &LoadedState,
        owner: i32,
        key: u32,
    ) -> Result<Option<i32>, Error>;
    /// 0242e68c(City.ComponentID, conversionOutcome), after BOTH cache writes.
    /// None denotes unknown emission from unsaved initial cache bytes; only
    /// source-qualified presentation consumers may be omitted in that case.
    fn trend_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        outcome: i32,
        changed: Option<bool>,
    ) -> Result<(), Error>;
}
fn player(state: &LoadedState, owner: i32) -> Result<&RetainedPlayer, Error> {
    state
        .world()
        .player(owner)
        .map_err(|_| Error::Unsupported("loyalty current player"))?;
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("loyalty canonical player"))
}
fn city(state: &LoadedState, owner: i32, id: u32) -> Result<&City, Error> {
    Ok(state.city(owner, id)?)
}
fn no_nearby_effects(state: &LoadedState, owner: i32) -> Result<(), Error> {
    let p = player(state, owner)?;
    if !p.governors.entries.is_empty() {
        return Err(Error::Unsupported("nearby governor identity"));
    }
    let c = &p.cultural_identity;
    if c.foreign_identity_per_turn_from_nearby_great_works_raw != 0
        || c.domestic_identity_per_turn_from_nearby_great_works_raw != 0
        || c.foreign_identity_per_turn_from_nearby_wonders_raw != 0
        || c.domestic_identity_per_turn_from_nearby_wonders_raw != 0
    {
        return Err(Error::Unsupported("nearby great-work/wonder identity"));
    }
    Ok(())
}
pub(crate) fn always_loyal(state: &LoadedState, owner: i32, id: u32) -> Result<bool, Error> {
    let c = city(state, owner, id)?;
    if c.cultural_identity.always_loyal_flag_count > 0 {
        return Ok(true);
    }
    if owner == -1 || owner != c.header.original_owner {
        return Ok(false);
    }
    if player(state, owner)?
        .cultural_identity
        .always_loyal_coastal_home_continent_count
        > 0
    {
        return Err(Error::Unsupported("coastal home-continent always loyal"));
    }
    Ok(false)
}
fn capital(state: &LoadedState, owner: i32) -> Result<Option<&City>, Error> {
    Ok(player(state, owner)?
        .cities
        .records
        .entries
        .iter()
        .map(|c| &c.value)
        .find(|c| c.header.capital))
}
pub(crate) fn identity_for(
    state: &LoadedState,
    defs: &Definitions,
    cx: &mut impl Context,
    owner: i32,
    id: u32,
    source: i32,
) -> Result<i32, Error> {
    if source == -1 || !state.player_alive(source)? {
        return Ok(0);
    }
    let c = city(state, owner, id)?;
    let mut result = 0i32;
    let level = cx.civilization_level(owner)?;
    let free = level == 0x04e6b0a0;
    let minor = level == 0xc2577075;
    if source == owner {
        result = result.wrapping_add(defs.q8(if c.header.capital {
            "OWNERSHIP_IDENTITY_PRESSURE_FROM_CAPITAL_TO_DOMESTIC_CITY"
        } else {
            "OWNERSHIP_IDENTITY_PRESSURE_FROM_CITY_TO_DOMESTIC_CITY"
        })?);
        if c.header.occupied {
            return Err(Error::Unsupported("martial law identity"));
        }
        if c.header.original_owner != owner {
            return Err(Error::Unsupported("occupation identity"));
        }
        if !free {
            let surplus = cx.food_surplus_q8(state, owner, id)?;
            // GetTurnsUntilStarvation288ada0 returns -1 at population<2,
            // including negative surplus. Its query still happens first.
            if surplus < 0 && city(state, owner, id)?.header.population >= 2 {
                result = result.wrapping_add(defs.q8("IDENTITY_PER_TURN_FROM_STARVATION")?);
            }
        }
        let p = player(state, owner)?;
        if p.cultural_identity
            .identity_per_turn_for_domestic_trade_route_origin_raw
            != 0
        {
            return Err(Error::Unsupported("domestic trade-route identity"));
        }
        if !p.governors.entries.is_empty() {
            return Err(Error::Unsupported("assigned governor identity"));
        }
        if !free {
            let happiness = cx.happiness(state, owner, id)?;
            // GetHappiness supplies the actual runtime ordinal, not a save hash.
            if happiness != -1 {
                let value = defs
                    .happiness_loyalty
                    .get(happiness as u32 as usize)
                    .ok_or(Error::Unsupported("current happiness index"))?;
                result = result.wrapping_add(value.wrapping_shl(8));
            }
        }
        if state
            .retained()
            .religion
            .records
            .iter()
            .any(|r| r.founder == owner)
        {
            return Err(Error::Unsupported("founded-religion identity"));
        }
        if minor {
            return Err(Error::Unsupported("highest envoy identity"));
        }
        result = result.wrapping_add(
            city(state, owner, id)?
                .cultural_identity
                .identity_per_turn_from_modifiers_raw_q8,
        );
        if free {
            result = result.wrapping_add(defs.q8("IDENTITY_PER_TURN_FROM_FREE_CITIES")?);
        }
        if minor {
            result = result.wrapping_add(defs.q8("IDENTITY_PER_TURN_FROM_CITY_STATES")?);
        }
    }
    if free && source == c.header.original_owner {
        result =
            result.wrapping_add(defs.q8("IDENTITY_PER_TURN_FROM_ORIGINAL_OWNER_ON_FREE_CITY")?);
    }
    if free && source == c.header.owner_before_occupation {
        result = result
            .wrapping_add(defs.q8("IDENTITY_PER_TURN_FROM_OWNER_BEFORE_OCCUPATION_ON_FREE_CITY")?);
    }
    if let Some(cap) = capital(state, source)? {
        if cap.header.location_x != c.header.location_x
            || cap.header.location_y != c.header.location_y
        {
            return Err(Error::Unsupported("nonlocal capital pressure radius"));
        }
        // Same location has distance0, so source takes the minimum-radius arm.
        if defs.parameter("CAPITAL_IDENTITY_PRESSURE_TILE_RADIUS_MINIMUM")? < 0 {
            return Err(Error::Unsupported("negative capital minimum radius"));
        }
        result = result.wrapping_add(defs.q8("CAPITAL_IDENTITY_PRESSURE_MAXIMUM")?);
    }
    no_nearby_effects(state, source)?;
    Ok(result)
}
pub(crate) fn pressure(
    state: &LoadedState,
    defs: &Definitions,
    cx: &mut impl Context,
    owner: i32,
    id: u32,
) -> Result<i32, Error> {
    let c = city(state, owner, id)?;
    let ids = cx.alive_ids().to_vec();
    // This exact branch has only the current city. No foreign pressure is
    // manufactured from absent cities or a prior native pressure answer.
    for source in ids {
        if player(state, source)?
            .cities
            .records
            .entries
            .iter()
            .any(|r| source != owner || r.value.header.id != id)
        {
            return Err(Error::Unsupported("multiple-city citizen pressure"));
        }
    }
    if c.cultural_identity
        .identities_present
        .iter()
        .any(|r| r.player == owner && r.rejected_city)
        || c.header.liberator == owner
    {
        return Ok(0);
    }
    if defs.parameter("CITIZEN_IDENTITY_PRESSURE_RADIUS_CUTOFF")? <= 0 {
        return Err(Error::Unsupported("nonpositive citizen pressure radius"));
    }
    if c.culture.loyalty_from_great_works_citizen_modifier != 0 {
        return Err(Error::Unsupported("great-work citizens pressure"));
    }
    if c.build_queue
        .entries
        .first()
        .is_some_and(|entry| matches!(entry.order, c6_core::city_production::Order::Advance { .. }))
    {
        return Err(Error::Unsupported("current project citizen pressure"));
    }
    let score = cx
        .player_property_int(state, owner, 0xdee04aee)?
        .unwrap_or(0);
    if score != 0 {
        return Err(Error::Unsupported("excess score citizen pressure"));
    }
    let per_citizen = defs
        .q8("CITIZEN_IDENTITY_PRESSURE_BASE")?
        .wrapping_add(
            c.cultural_identity
                .extra_identity_per_turn_per_citizen_raw_q8,
        )
        .wrapping_add(if c.header.capital {
            defs.q8("CITIZEN_IDENTITY_PRESSURE_CAPITAL")?
        } else {
            0
        });
    // Target is this city's exact plot: source distance factor is 1Q8.
    let population = c6_core::city_production::execution::multiply(
        c.header.population.wrapping_shl(8),
        per_citizen,
    )
    .max(0);
    let ownership = defs
        .q8(if c.header.capital {
            "OWNERSHIP_IDENTITY_PRESSURE_FROM_CAPITAL_TO_DOMESTIC_CITY"
        } else {
            "OWNERSHIP_IDENTITY_PRESSURE_FROM_CITY_TO_DOMESTIC_CITY"
        })?
        .max(0);
    let total = ownership
        .wrapping_add(population)
        .wrapping_add(
            c.cultural_identity
                .identity_pressure_from_ages_raw_q8
                .max(0),
        )
        .wrapping_add(c.cultural_identity.identity_pressure_from_modifiers_raw_q8);
    // 2873f7d: both zero returns0; otherwise 287406f sees foreign==0
    // and chooses MAX_LOYALTY directly, before ratio interpolation.
    if total == 0 {
        Ok(0)
    } else {
        defs.q8("LOYALTY_PER_TURN_FROM_NEARBY_CITIZEN_PRESSURE_MAX_LOYALTY")
    }
}
/// Source Q8 divide, wrapping Abs and Ceil, including overflow to negative.
fn turns(numerator: i32, denominator: i32) -> i32 {
    let quotient = ((i64::from(numerator) << 8) / i64::from(denominator)) as i32;
    let absolute = quotient.wrapping_abs();
    (absolute >> 8).wrapping_add(i32::from(absolute & 255 != 0))
}
fn trend(loyalty: i32, maximum: i32, rate: i32) -> (i32, i32) {
    if rate > 0 {
        (
            2,
            turns(maximum.wrapping_shl(8).wrapping_sub(loyalty), rate),
        )
    } else if rate < 0 {
        (1, turns(loyalty, rate))
    } else {
        (0, -1)
    }
}
fn emission(old: (Option<i32>, Option<i32>), new: (i32, i32)) -> Option<bool> {
    old.0.zip(old.1).map(|old| old != new)
}
pub fn do_city_created(
    state: &mut LoadedState,
    defs: &Definitions,
    cx: &mut impl Context,
    owner: i32,
    id: u32,
) -> Result<(), Error> {
    if state.binding() != &defs.binding {
        return Err(Error::Unsupported("cultural identity definition binding"));
    }
    let old = {
        let c = city(state, owner, id)?;
        (
            c.cultural_identity.conversion_outcome,
            c.cultural_identity.turns_to_conversion,
        )
    };
    let new = if always_loyal(state, owner, id)? {
        (0, -1)
    } else {
        let mut rate = 0i32;
        for source in cx.alive_ids().to_vec() {
            let value = identity_for(state, defs, cx, owner, id, source)?;
            rate = if source == owner {
                rate.wrapping_add(value)
            } else {
                rate.wrapping_sub(value)
            };
        }
        rate = rate.wrapping_add(pressure(state, defs, cx, owner, id)?);
        if rate == 0 {
            (0, -1)
        } else {
            let maximum = if rate > 0 {
                defs.parameter("LOYALTY_MAXIMUM")?
            } else {
                0
            };
            trend(
                city(state, owner, id)?.cultural_identity.loyalty_raw_q8,
                maximum,
                rate,
            )
        }
    };
    let c = state.city_mut(owner, id)?;
    c.cultural_identity.conversion_outcome = Some(new.0);
    c.cultural_identity.turns_to_conversion = Some(new.1);
    // Unsaved constructor cache bytes are genuinely unknown. Do not turn
    // unknown emission into a native changed=true answer. The caller qualifies
    // the source presentation-only report separately from gameplay cache writes.
    let changed = emission(old, new);
    if changed != Some(false) && city(state, owner, id)?.cultural_identity.loyalty_raw_q8 > 0 {
        cx.trend_changed(state, owner, id, new.0, changed)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_constructor_cache_does_not_invent_report_emission() {
        assert_eq!(emission((None, None), (2, 0)), None);
        assert_eq!(emission((Some(2), None), (2, 0)), None);
        assert_eq!(emission((Some(2), Some(0)), (2, 0)), Some(false));
        assert_eq!(emission((Some(0), Some(-1)), (2, 0)), Some(true));
    }
    #[test]
    fn signed_native_q8_division_and_ceil() {
        assert_eq!(turns(100 * 256, 3 * 256), 34);
        assert_eq!(turns(256, -3 * 256), 1);
        assert_eq!(trend(100 * 256, 100, 256), (2, 0));
        assert_eq!(trend(100 * 256, 100, 0), (0, -1));
        assert_eq!(trend(100 * 256, 100, -3 * 256), (1, 34));
    }
}
