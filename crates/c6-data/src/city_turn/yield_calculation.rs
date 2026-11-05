//! Current City.GetYield arithmetic. Native presentation cache entries are
//! recomputable; query inputs are borrowed from the same live gameplay owners.
use super::{
    amenities, district_yields, economy, resources, tiles, yield_definitions, Error, GrowthRules,
};
use crate::{
    city_production_turn::{building_yields, buildings},
    loaded_actions::trade_route_actions::{self, yields as trade_yields},
    Rules,
};
use c6_core::{
    city_production::execution, loaded_state::LoadedState, movement_world::DefinitionLookup,
    unit::HashValue,
};

pub struct Definitions {
    pub tiles: yield_definitions::Definitions,
    pub economy: economy::Definitions,
    pub amenities: amenities::Definitions,
    pub resources: resources::Definitions,
    pub buildings: buildings::Definitions,
    pub districts: district_yields::Definitions,
    pub growth: GrowthRules,
    pub trade: trade_yields::Definitions,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        start_era: u32,
        game_speed: u32,
    ) -> Result<Self, Error> {
        Ok(Self {
            tiles: yield_definitions::Definitions::from_rules(rules)?,
            economy: economy::Definitions::from_rules(rules)?,
            amenities: amenities::Definitions::from_rules(rules)?,
            resources: resources::Definitions::from_rules(rules)?,
            buildings: buildings::Definitions::from_current_rules(
                state, rules, start_era, game_speed,
            )
            .map_err(Error::Production)?,
            districts: district_yields::Definitions::from_rules(rules)?,
            growth: GrowthRules::from_rules(rules, game_speed)?,
            trade: trade_yields::Definitions::from_rules(state, rules).map_err(trade_error)?,
        })
    }
    pub fn yield_index(&self, name: &[u8]) -> Option<i32> {
        self.economy.yield_index(name)
    }
}
/// These are current shared owners, not a cache of native yield answers. An
/// unavailable climate collection remains distinct from a measured empty one.
pub struct Current<'a> {
    pub territory: &'a crate::city_territory::Runtime,
    pub drought_ids: Option<&'a [i32]>,
    pub trade: &'a c6_core::trade_graph_state::GraphState,
    pub capabilities: &'a c6_core::game_capabilities::Membership,
    pub current_era_index: i32,
}
fn pair(rows: &[HashValue<i32>], hash: u32) -> Result<i32, Error> {
    rows.iter()
        .rfind(|r| r.definition_hash == hash)
        .map(|r| r.value)
        .ok_or(Error::Unsupported(
            "current definition-sized city yield array",
        ))
}
fn round(raw: i32) -> i32 {
    (raw >> 8).wrapping_add(((raw as u32 >> 7) & 1) as i32)
}
fn citizen_gold(population: i32, coefficient: i32) -> i32 {
    if coefficient > 0 {
        population.wrapping_mul(coefficient).wrapping_shl(8)
    } else {
        0
    }
}
fn production(city: &c6_core::city::City, kind: u32) -> (i32, i32) {
    let (flat, multiplier) = match kind {
        0x15d11e18 => (
            city.header.production_unit_changes,
            city.build_queue.city_unit_production_modifier,
        ),
        0xd8a30267 => (
            city.header.production_building_changes,
            city.build_queue.building_production_modifier,
        ),
        0x080d3734 => (
            city.header.production_district_changes,
            city.build_queue.district_production_modifier,
        ),
        _ => return (0, 0),
    };
    let percentage = if multiplier == 256 {
        0
    } else {
        round(execution::multiply(multiplier.wrapping_sub(256), 100 << 8)).wrapping_shl(8)
    };
    (flat.wrapping_shl(8), percentage)
}
fn trade_error(error: trade_route_actions::Error) -> Error {
    match error {
        trade_route_actions::Error::Rules(error) => Error::Rules(error),
        trade_route_actions::Error::State(error) => Error::State(error),
        trade_route_actions::Error::Requires(reason)
        | trade_route_actions::Error::Missing(reason) => Error::Unsupported(reason),
        _ => Error::Unsupported("current trade yield input binding"),
    }
}

fn route_yields(
    state: &LoadedState,
    vertex: c6_core::trade::Vertex,
    hash: u32,
    definitions: &trade_yields::Definitions,
    current: &Current<'_>,
) -> Result<i32, Error> {
    let mut total = 0i32;
    // City.CalculateYield 28950a2/28950d7 adds outgoing, then incoming.
    // Graph 2bfae90/2bfb010 filters has_active_route before calling the
    // requested Route getter. All admitted query branches are read-only;
    // their wrapping sums commute within each native hashed neighbor list.
    for outgoing in [true, false] {
        for origin in current.trade.graph().origins() {
            for destination in &origin.destinations {
                if !destination.edge.has_active_route
                    || (if outgoing {
                        origin.vertex
                    } else {
                        destination.vertex
                    }) != vertex
                {
                    continue;
                }
                let route = &destination.edge.most_recent_route;
                if c6_core::trade_graph_state::city_vertex(route.origin_city) != origin.vertex
                    || c6_core::trade_graph_state::city_vertex(route.destination_city)
                        != destination.vertex
                {
                    return Err(Error::Unsupported("current route keyed endpoint identity"));
                }
                let amounts = trade_yields::active_domestic_on_graph(
                    state,
                    definitions,
                    current.trade,
                    route.origin_city,
                    route.destination_city,
                )
                .map_err(trade_error)?;
                let values = if outgoing {
                    &amounts.origin
                } else {
                    &amounts.destination
                };
                let value = values
                    .iter()
                    .find(|v| v.type_hash == hash)
                    .ok_or(Error::Unsupported("current route yield definition"))?;
                total = total.wrapping_add(value.raw);
            }
        }
    }
    Ok(total)
}

/// CalculateYield(kind,false), including the current production-kind modifier.
/// Positive branches whose current gameplay owners are not implemented return
/// an error; no cached original result is substituted.
pub fn calculate(
    state: &LoadedState,
    owner: i32,
    id: u32,
    yield_index: i32,
    kind: u32,
    definitions: &Definitions,
    current: &Current<'_>,
) -> Result<i32, Error> {
    let hash = definitions
        .tiles
        .yields
        .type_hash(yield_index as u32 as usize)
        .ok_or(Error::Unsupported("current Yield definition"))?;
    let city = state.city(owner, id)?;
    let mut base = tiles::get_yield_from_tiles_q8(
        state,
        owner,
        id,
        yield_index,
        &definitions.tiles,
        &tiles::Current {
            territory: current.territory,
            drought_ids: current.drought_ids,
            resource_visibility: &definitions.resources,
        },
    )?;
    base = base.wrapping_add(
        building_yields::calculate(
            state,
            owner,
            id,
            yield_index,
            false,
            &definitions.buildings,
            &building_yields::Runtime {
                current_era_index: current.current_era_index,
                fully_powered: definitions.amenities.fully_powered(city)?,
            },
        )
        .map_err(Error::Production)?,
    );
    base = base.wrapping_add(definitions.economy.population_yield(
        state,
        owner,
        id,
        yield_index,
    )?);
    base = base.wrapping_add(district_yields::calculate(
        state,
        owner,
        id,
        yield_index,
        false,
        &definitions.districts,
    )?);
    let vertex = c6_core::trade::Vertex {
        kind: 0,
        data1: owner,
        data2: id as i32,
    };
    base = base.wrapping_add(route_yields(
        state,
        vertex,
        hash,
        &definitions.trade,
        current,
    )?);
    // 2896330 skips the major-partner count unless this coefficient is positive.
    // Trade v10 explicitly zero-resets the omitted late Yield list.
    if city
        .trade
        .yield_per_major_trade_partner
        .iter()
        .any(|v| v.definition_hash == hash && v.value > 0)
    {
        return Err(Error::Unsupported("current major trade partner yields"));
    }
    let passing = pair(&city.trade.yield_from_foreign_routes_passing_through, hash)?;
    base = base.wrapping_add(
        passing
            .wrapping_mul(city.trade.num_foreign_routes_passing_through)
            .wrapping_shl(8),
    );
    if city
        .buildings
        .great_work_buildings
        .iter()
        .any(|b| b.slots.iter().any(|s| s.great_work_index != -1))
    {
        return Err(Error::Unsupported("current GreatWork yield owner"));
    }
    if pair(&city.resources.resource_yield_by_count, hash)? != 0
        || pair(&city.resources.yield_by_number_of_resources, hash)? != 0
    {
        return Err(Error::Unsupported(
            "current resource count yield calculation",
        ));
    }
    if city
        .header
        .yield_per_terrain_type
        .iter()
        .any(|r| r.yield_hash == hash && r.amount_q8 != 0)
    {
        return Err(Error::Unsupported(
            "current per-terrain city yield calculation",
        ));
    }
    base = base.wrapping_add(pair(&city.header.yield_changes, hash)?.wrapping_shl(8));
    base = base.wrapping_add(pair(&city.header.project_yield_q8, hash)?);
    // Version44 explicitly zero-resets the omitted faith-city array.
    let faith = definitions
        .yield_index(b"YIELD_FAITH")
        .ok_or(Error::Unsupported("Faith definition"))?;
    let faith_offset = if city.header.wire_version == 44 {
        0
    } else {
        pair(&city.header.yield_modifier_from_faith_city, hash)?
    };
    if yield_index != faith && faith_offset > 0 {
        return Err(Error::Unsupported("current recursive faith yield offset"));
    }
    let mut modifier = 0i32;
    if Some(yield_index) == definitions.yield_index(b"YIELD_PRODUCTION") {
        let (flat, percent) = production(city, kind);
        base = base.wrapping_add(flat);
        modifier = modifier.wrapping_add(percent);
    }
    if Some(yield_index) == definitions.yield_index(b"YIELD_GOLD") {
        base = base.wrapping_add(citizen_gold(
            city.header.population,
            city.gold.gold_from_citizens,
        ));
    }
    if city.religion.majority_religion_hash != c6_core::city_religion::NO_RELIGION {
        return Err(Error::Unsupported(
            "current religion building/follower yield modifiers",
        ));
    }
    if pair(&city.header.yield_modifiers_per_governor_title_q8, hash)? != 0 {
        return Err(Error::Unsupported(
            "current assigned Governor title yield modifier",
        ));
    }
    modifier = modifier.wrapping_add(pair(&city.header.yield_modifiers, hash)?.wrapping_shl(8));
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("current yield Player"))?;
    if player
        .diplomacy
        .yield_modifiers
        .iter()
        .any(|r| r.yield_type == yield_index)
    {
        return Err(Error::Unsupported(
            "current diplomatic yield expiry/stack evaluation",
        ));
    }
    // Economic.GetPowerYieldModifier is identically zero in this Linux target.
    if Some(yield_index) != definitions.yield_index(b"YIELD_FOOD") {
        let amenities = definitions.amenities.amenities(
            state,
            owner,
            id,
            current.capabilities,
            &definitions.growth,
        )?;
        let needed = definitions
            .economy
            .amenities_needed(city.header.population, current.capabilities);
        let happiness = definitions
            .economy
            .happiness_from_surplus(amenities.wrapping_sub(needed));
        if city
            .header
            .happiness_modifiers
            .iter()
            .any(|r| r.yield_type_index == yield_index && r.yield_amount != 0)
        {
            return Err(Error::Unsupported(
                "current runtime happiness modifier indices",
            ));
        }
        modifier = modifier.wrapping_add(definitions.economy.happiness_yield_modifier(happiness));
        modifier = modifier.wrapping_add(
            definitions
                .economy
                .loyalty_yield_modifier(state, owner, id)?,
        );
    }
    Ok(c6_core::research_turn::yield_attribute_value(
        base,
        modifier.max(-25600),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn citizen_gold_uses_positive_coefficient_and_wrapping_population_product() {
        assert_eq!(citizen_gold(4, 3), 12 * 256);
        assert_eq!(citizen_gold(4, -3), 0);
        assert_eq!(citizen_gold(i32::MAX, 2), -512);
    }
    #[test]
    fn native_round_half_ties_are_toward_positive_infinity() {
        for (raw, expected) in [
            (127, 0),
            (128, 1),
            (384, 2),
            (-128, 0),
            (-129, -1),
            (-384, -1),
        ] {
            assert_eq!(round(raw), expected);
        }
    }
}
