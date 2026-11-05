//! Bounded Linux XP2 numerical queries for an existing domestic route.
//!
//! Computes from current rules, city districts, player/city Trade and the actual
//! active route bonuses. This does not qualify the transitive effects of native
//! reporting queries or admit omitting their callbacks/RNG. Unsupported nonzero
//! coefficients remain explicit errors; this is not complete trade simulation.
use super::{Definitions, Error};
use crate::{registry::linux_type_hash, Rules, SqlValue};
use c6_core::{
    loaded_state::LoadedState,
    trade::{ComponentId, YieldValue},
    unit::HashValue,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Amounts {
    pub origin: Vec<YieldValue>,
    pub destination: Vec<YieldValue>,
}

fn pair_value(table: &[(u32, i32)], hash: u32, default: i32) -> Result<i32, Error> {
    let mut found = None;
    for &(key, value) in table {
        if key == hash && found.replace(value).is_some() {
            return Err(Error::Requires("unambiguous current Trade yield overlay"));
        }
    }
    Ok(found.unwrap_or(default))
}
fn city_value(table: &[HashValue<i32>], hash: u32) -> Result<i32, Error> {
    let mut found = None;
    for entry in table {
        if entry.definition_hash == hash && found.replace(entry.value).is_some() {
            return Err(Error::Requires(
                "unambiguous current City.Trade yield overlay",
            ));
        }
    }
    Ok(found.unwrap_or(0))
}
fn zero(table: &[(u32, i32)], reason: &'static str) -> Result<(), Error> {
    if table.iter().any(|&(_, value)| value != 0) {
        return Err(Error::Requires(reason));
    }
    Ok(())
}
fn nested_zero(table: &[(u32, Vec<(u32, i32)>)], reason: &'static str) -> Result<(), Error> {
    for (_, values) in table {
        zero(values, reason)?;
    }
    Ok(())
}
fn multiply(a: i32, b: i32) -> i32 {
    ((i64::from(a) * i64::from(b)) >> 8) as i32
}

// Native 1fceb70 floors the integer part, shifts it, then ORs the low
// byte of trunc((x-floor(x))*256). Reject the SSE invalid-conversion domain.
fn from_float(value: f32) -> Result<i32, Error> {
    let integer = value.floor();
    if !integer.is_finite() || !(-2147483648.0..2147483648.0).contains(&integer) {
        return Err(Error::Requires("finite in-range district trade yield"));
    }
    Ok((integer as i32).wrapping_shl(8) | (((value - integer) * 256.0) as i32 & 255))
}
fn number(value: &SqlValue) -> Result<f32, Error> {
    match value {
        SqlValue::Integer(v) => Ok(*v as f32),
        SqlValue::RealBits(v) => Ok(f64::from_bits(*v) as f32),
        _ => Err(Error::Requires("numeric district trade definition")),
    }
}
fn count(state: &LoadedState, city: ComponentId, district: u32) -> Result<i32, Error> {
    let current = state
        .city(i32::from(city.owner), city.id)
        .map_err(Error::State)?;
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == i32::from(city.owner))
        .ok_or(Error::Missing("Trade city owner"))?;
    // 2885670(true,false): complete only, exact type, no replacement matching.
    // Pillaged districts are included. Iterate the city's actual ID list.
    Ok(current
        .districts
        .district_ids
        .iter()
        .fold(0i32, |total, id| {
            if player.districts.records.entries.iter().any(|entry| {
                entry.value.id == *id
                    && entry.value.is_complete
                    && entry.value.district_hash == district
            }) {
                total.wrapping_add(1)
            } else {
                total
            }
        }))
}

/// Numerical Economic origin/destination query with includeTemporary=false.
/// Requires an active forward route and current nonempty path. Nonzero path,
/// specialty, resource, follower and plot policy terms are not implemented.
/// Native query effect closure must be established separately by the caller
/// before using this numerical result to finish an action/report transition.
pub fn active_domestic(
    state: &LoadedState,
    rules: &Rules,
    origin: ComponentId,
    destination: ComponentId,
) -> Result<Amounts, Error> {
    let definitions = Definitions::from_rules(state, rules)?;
    definitions.validate(state)?;
    if origin.owner != destination.owner
        || origin.component_type != 2
        || destination.component_type != 2
    {
        return Err(Error::Requires("domestic city route yield query"));
    }
    let origin_city = state
        .city(i32::from(origin.owner), origin.id)
        .map_err(Error::State)?;
    let destination_city = state
        .city(i32::from(destination.owner), destination.id)
        .map_err(Error::State)?;
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == i32::from(origin.owner))
        .ok_or(Error::Missing("Trade player"))?;
    let trade = &player.trade;
    let graph = state.trade_graph().map_err(Error::State)?;
    let edge = graph
        .graph()
        .get_edge(
            c6_core::trade_graph_state::city_vertex(origin),
            c6_core::trade_graph_state::city_vertex(destination),
        )
        .filter(|edge| edge.has_active_route)
        .ok_or(Error::Requires("active domestic route"))?;
    if edge.most_recent_path.is_empty() {
        return Err(Error::Requires("current forward trade path"));
    }
    let route = &edge.most_recent_route;
    if route.origin_city != origin || route.destination_city != destination {
        return Err(Error::Requires("active route endpoint identity"));
    }
    for (table, reason) in [
        (
            &trade.yield_bonus_per_trading_post_in_foreign_city,
            "foreign trading-post path yields",
        ),
        (
            &trade.yield_bonus_per_trading_post_in_own_city,
            "domestic trading-post path yields",
        ),
        (&trade.yield_bonus_per_path_tile, "per-tile path yields"),
        (
            &trade.per_specialty_district_yield_bonus_for_domestic,
            "domestic specialty district yields",
        ),
        (
            &trade.flat_yield_bonus_for_intercontinental_domestic,
            "intercontinental domestic yields",
        ),
        (&trade.yield_bonus_per_follower, "domestic follower yields"),
    ] {
        zero(table, reason)?;
    }
    for (table, reason) in [
        (
            &trade.flat_origin_yield_per_improvement_at_destination,
            "origin destination-improvement yields",
        ),
        (
            &trade.flat_destination_yield_per_improvement_at_destination,
            "destination improvement yields",
        ),
        (
            &trade.flat_origin_yield_per_terrain_for_domestic,
            "origin domestic terrain yields",
        ),
        (
            &trade.flat_destination_yield_per_terrain_for_domestic,
            "destination domestic terrain yields",
        ),
    ] {
        nested_zero(table, reason)?;
    }
    let table = rules
        .gameplay()
        .table("District_TradeRouteYields")
        .ok_or(Error::Missing("District_TradeRouteYields"))?;
    let mut amounts = Amounts {
        origin: Vec::new(),
        destination: Vec::new(),
    };
    for hash in definitions.yields {
        let mut potential = 0i32;
        for row in table.rows() {
            if linux_type_hash(row.text("YieldType").map_err(Error::Rules)?.as_bytes()) != hash {
                continue;
            }
            let district =
                linux_type_hash(row.text("DistrictType").map_err(Error::Rules)?.as_bytes());
            let from = number(row.get("YieldChangeAsOrigin").map_err(Error::Rules)?)?;
            let to = number(
                row.get("YieldChangeAsDomesticDestination")
                    .map_err(Error::Rules)?,
            )?;
            potential =
                potential.wrapping_add(from_float(count(state, origin, district)? as f32 * from)?);
            potential = potential.wrapping_add(from_float(
                count(state, destination, district)? as f32 * to,
            )?);
        }
        // 2c08d49 reaches transportation efficiency only for Gold. Its
        // contribution multiplies potential Gold, which must be zero here.
        if hash == linux_type_hash(b"YIELD_GOLD") && potential != 0 {
            return Err(Error::Requires("transportation efficiency gold yield"));
        }
        if city_value(
            &origin_city
                .trade
                .yield_per_local_bonus_resource_for_domestic,
            hash,
        )? > 0
        {
            return Err(Error::Requires("domestic local bonus resource yields"));
        }
        if city_value(
            &origin_city
                .trade
                .yield_per_destination_strategic_resource_for_domestic,
            hash,
        )? > 0
        {
            return Err(Error::Requires(
                "domestic destination strategic resource yields",
            ));
        }
        let mut modifiers = pair_value(&trade.flat_yield_bonus, hash, 0)?;
        modifiers =
            modifiers.wrapping_add(pair_value(&trade.flat_yield_bonus_for_domestic, hash, 0)?);
        modifiers = modifiers.wrapping_add(city_value(
            &origin_city.trade.flat_yield_bonus_for_domestic,
            hash,
        )?);
        modifiers = modifiers.wrapping_add(city_value(
            &destination_city
                .trade
                .flat_yield_bonus_to_others_for_domestic,
            hash,
        )?);
        let mut bonus = None;
        for entry in &route.origin_yields {
            if entry.type_hash == hash && bonus.replace(entry.raw).is_some() {
                return Err(Error::Requires("unambiguous current route yield overlay"));
            }
        }
        let bonus = bonus.ok_or(Error::Missing("current active route origin yield"))?;
        modifiers = modifiers.wrapping_add(bonus);
        amounts.origin.push(YieldValue {
            type_hash: hash,
            raw: multiply(
                potential.wrapping_add(modifiers),
                pair_value(&trade.origin_yield_modifier, hash, 256)?,
            ),
        });
        // Destination potential/path/attaching-resource helpers return zero;
        // the domestic plot terms were bounded above. Native 2c0b40f reads
        // GetOriginYieldBonus here too, despite this being destination output.
        amounts.destination.push(YieldValue {
            type_hash: hash,
            raw: multiply(
                bonus,
                pair_value(&trade.destination_yield_modifier, hash, 256)?,
            ),
        });
    }
    Ok(amounts)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_float_fraction_and_wrapping_product() {
        assert_eq!(from_float(-1.1).unwrap(), -282);
        assert_eq!(from_float(1.1).unwrap(), 281);
        assert_eq!(from_float(8388608.0).unwrap(), i32::MIN);
        assert!(from_float(f32::NAN).is_err());
        assert_eq!(multiply(-1, 128), -1);
        assert_eq!(multiply(i32::MAX, 512), -2);
    }
}
