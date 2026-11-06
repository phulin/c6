//! Bounded Linux XP2 numerical queries for an existing domestic or bounded international route.
//!
//! Computes from current rules, city districts, player/city Trade and the actual
//! active route bonuses. This does not qualify the transitive effects of native
//! reporting queries or admit omitting their callbacks/RNG. Unsupported nonzero
//! coefficients remain explicit errors; this is not complete trade simulation.
use super::Error;
use crate::{registry::linux_type_hash, Rules, SqlValue};
use c6_core::{
    loaded_state::LoadedState,
    trade::{ComponentId, YieldValue},
    unit::HashValue,
};

/// Immutable definition inputs; contains no captured route or yield answers.
pub struct Definitions {
    inputs: c6_core::core_player::initialization::InputIdentities,
    yields: Vec<u32>,
    district_yields: Vec<DistrictYield>,
    transport: Transport,
}
struct DistrictYield {
    yield_hash: u32,
    district: u32,
    origin: f32,
    domestic_destination: f32,
    foreign_destination: f32,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        if !matches!(
            rules.profile(),
            crate::RulesProfile::LinuxXp2Base | crate::RulesProfile::LinuxXp2ActivatedDiagnostic
        ) {
            return Err(Error::Requires("Linux XP2 trade rules"));
        }
        let yields =
            crate::unit_initialization_definitions::collection(rules, "Yields", "YieldType")
                .map_err(|_| Error::Missing("Yield definitions"))?
                .entries()
                .iter()
                .map(|entry| linux_type_hash(&entry.type_name))
                .collect();
        let table = rules
            .gameplay()
            .table("District_TradeRouteYields")
            .ok_or(Error::Missing("District_TradeRouteYields"))?;
        let mut district_yields = Vec::new();
        for row in table.rows() {
            district_yields.push(DistrictYield {
                yield_hash: linux_type_hash(
                    row.text("YieldType").map_err(Error::Rules)?.as_bytes(),
                ),
                district: linux_type_hash(
                    row.text("DistrictType").map_err(Error::Rules)?.as_bytes(),
                ),
                origin: number(row.get("YieldChangeAsOrigin").map_err(Error::Rules)?)?,
                foreign_destination: number(
                    row.get("YieldChangeAsInternationalDestination")
                        .map_err(Error::Rules)?,
                )?,
                domestic_destination: number(
                    row.get("YieldChangeAsDomesticDestination")
                        .map_err(Error::Rules)?,
                )?,
            });
        }
        Ok(Self {
            inputs: state.binding().inputs.clone(),
            yields,
            district_yields,
            transport: Transport::from_rules(rules)?,
        })
    }
}

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
    active_domestic_with_definitions(state, &definitions, origin, destination)
}

/// The same query using immutable rules prepared for this input identity.
pub fn active_domestic_with_definitions(
    state: &LoadedState,
    definitions: &Definitions,
    origin: ComponentId,
    destination: ComponentId,
) -> Result<Amounts, Error> {
    let graph = state.trade_graph().map_err(Error::State)?;
    active_domestic_on_graph(state, definitions, graph, origin, destination)
}

pub(crate) fn active_domestic_on_graph(
    state: &LoadedState,
    definitions: &Definitions,
    graph: &c6_core::trade_graph_state::GraphState,
    origin: ComponentId,
    destination: ComponentId,
) -> Result<Amounts, Error> {
    if definitions.inputs != state.binding().inputs {
        return Err(Error::Binding);
    }
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
    let mut amounts = Amounts {
        origin: Vec::new(),
        destination: Vec::new(),
    };
    for &hash in &definitions.yields {
        let mut potential = 0i32;
        for row in &definitions.district_yields {
            if row.yield_hash != hash {
                continue;
            }
            potential = potential.wrapping_add(from_float(
                count(state, origin, row.district)? as f32 * row.origin,
            )?);
            potential = potential.wrapping_add(from_float(
                count(state, destination, row.district)? as f32 * row.domestic_destination,
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

struct Transport {
    best_route: i32,
    best_score: i32,
    water_score: i32,
    portal_score: i32,
    maximum: i32,
    waters: Vec<u32>,
    terrains: Vec<u32>,
    foreign_post_gold: i32,
    own_post_gold: i32,
    extended_domains: Vec<u32>,
}
impl Transport {
    fn from_rules(rules: &Rules) -> Result<Self, Error> {
        let table = |name| rules.gameplay().table(name).ok_or(Error::Missing(name));
        let parameter = |name: &'static str| -> Result<f32, Error> {
            for row in table("GlobalParameters")?.rows() {
                if row.text("Name").map_err(Error::Rules)? == name {
                    return match row.get("Value").map_err(Error::Rules)? {
                        SqlValue::Text(v) => std::str::from_utf8(v)
                            .ok()
                            .and_then(|v| v.parse().ok())
                            .ok_or(Error::Missing(name)),
                        v => number(v),
                    };
                }
            }
            Err(Error::Missing(name))
        };
        let parameter_int = |name: &'static str| -> Result<i32, Error> {
            for row in table("GlobalParameters")?.rows() {
                if row.text("Name").map_err(Error::Rules)? == name {
                    return crate::game_parameters::sql_integer(
                        row.get("Value").map_err(Error::Rules)?,
                    )
                    .ok_or(Error::Missing(name));
                }
            }
            Err(Error::Missing(name))
        };
        let registry =
            crate::unit_initialization_definitions::collection(rules, "Routes", "RouteType")
                .map_err(|_| Error::Missing("Trade Routes registry"))?;
        let mut best_route = -1;
        let mut best_value = -1;
        for (index, entry) in registry.entries().iter().enumerate() {
            let value = table("Routes")?
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Missing("Trade Route row"))?
                .integer("PlacementValue")
                .map_err(Error::Rules)? as i32;
            if value > best_value {
                best_value = value;
                best_route = index as i32;
            }
        }
        let mut waters = Vec::new();
        let registry =
            crate::unit_initialization_definitions::collection(rules, "Terrains", "TerrainType")
                .map_err(|_| Error::Missing("Trade Terrains registry"))?;
        let mut terrains = Vec::new();
        for (index, entry) in registry.entries().iter().enumerate() {
            let hash = linux_type_hash(&entry.type_name);
            terrains.push(hash);
            let row = table("Terrains")?
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Missing("Trade Terrain row"))?;
            if index == 15 || index == 16 || row.integer("Water").map_err(Error::Rules)? != 0 {
                waters.push(hash);
            }
        }
        let mut extended_domains = Vec::new();
        for row in table("Districts")?.rows() {
            if row.integer("InternalOnly").map_err(Error::Rules)? != 0 {
                extended_domains.push(linux_type_hash(
                    row.text("DistrictType").map_err(Error::Rules)?.as_bytes(),
                ));
            }
        }
        if let Some(xp2) = rules.gameplay().table("Districts_XP2") {
            for row in xp2.rows() {
                if row.integer("Canal").map_err(Error::Rules)? != 0 {
                    extended_domains.push(linux_type_hash(
                        row.text("DistrictType").map_err(Error::Rules)?.as_bytes(),
                    ));
                }
            }
        }
        Ok(Self {
            best_route,
            waters,
            terrains,
            extended_domains,
            best_score: parameter_int(
                "TRADE_ROUTE_TRANSPORTATION_EFFICIENCY_SCORE_BEST_ROUTE_TILE",
            )?,
            water_score: parameter_int("TRADE_ROUTE_TRANSPORTATION_EFFICIENCY_SCORE_WATER_TILE")?,
            portal_score: parameter_int("TRADE_ROUTE_TRANSPORTATION_EFFICIENCY_SCORE_PORTAL_USE")?,
            foreign_post_gold: parameter_int("TRADING_POST_GOLD_IN_FOREIGN_CITY")?.wrapping_shl(8),
            own_post_gold: parameter_int("TRADING_POST_GOLD_IN_OWN_CITY")?.wrapping_shl(8),
            maximum: from_float(parameter(
                "TRADE_ROUTE_TRANSPORTATION_EFFICIENCY_MAX_RATIO",
            )?)?,
        })
    }
    fn ratio(&self, state: &LoadedState, path: &[i32]) -> Result<i32, Error> {
        use c6_core::map::PlotFlag;
        let map = state.world().map();
        if map.header.width <= 0
            || map.header.height <= 0
            || map.header.wrap_y
            || (map.header.width as usize).checked_mul(map.header.height as usize)
                != Some(map.plots.len())
        {
            return Err(Error::Requires("canonical trade transport geometry"));
        }
        // Domain rebuilding starts with terrain (coastal city centers are
        // excluded by CountTransportationFeatures). Complete canal/internal
        // districts can extend domains on other plots; preserve that boundary.
        if state.retained().players.iter().any(|p| {
            p.districts.records.entries.iter().any(|d| {
                d.value.is_complete && self.extended_domains.contains(&d.value.district_hash)
            })
        }) {
            return Err(Error::Requires(
                "canal/wonder transport domain reconstruction",
            ));
        }
        let mut score = 0i32;
        // 2c08b35/2c08bc5 pass includeOrigin=false: start at index one,
        // while the denominator remains the complete path vector length.
        for (position, &index) in path.iter().enumerate().skip(1) {
            let index = usize::try_from(index).map_err(|_| Error::Missing("Trade path plot"))?;
            let plot = map
                .plots
                .get(index)
                .ok_or(Error::Missing("Trade path plot"))?;
            if plot.flags.get(PlotFlag::HasCityAttributes) != plot.city_attributes.is_some() {
                return Err(Error::Requires("canonical transport CityAttributes"));
            }
            if plot.route_type != -1
                && !plot.flags.get(PlotFlag::RoutePillaged)
                && i32::from(plot.route_type) == self.best_route
            {
                score = score.wrapping_add(self.best_score);
            }
            if !self.terrains.contains(&plot.terrain_hash) {
                return Err(Error::Missing("Trade transport terrain definition"));
            }
            if self.waters.contains(&plot.terrain_hash) {
                score = score.wrapping_add(self.water_score);
            }
            let previous = path[position - 1];
            if previous < 0 || previous as usize >= map.plots.len() {
                return Err(Error::Missing("Trade previous path plot"));
            }
            if c6_core::geometry::plot_distance(
                map.header.width,
                map.header.wrap_x,
                previous % map.header.width,
                previous / map.header.width,
                index as i32 % map.header.width,
                index as i32 / map.header.width,
            ) > 1
            {
                score = score.wrapping_add(self.portal_score);
            }
        }
        let length =
            i32::try_from(path.len()).map_err(|_| Error::Requires("bounded Trade path length"))?;
        if length <= 0 {
            return Err(Error::Requires("current forward trade path"));
        }
        let numerator = score.wrapping_shl(8);
        let denominator = length.wrapping_shl(8);
        if denominator == 0 {
            return Err(Error::Requires("nonzero Q8 Trade path length"));
        }
        Ok((((i64::from(numerator) << 8) / i64::from(denominator)) as i32).min(self.maximum))
    }
}

/// Current economic yield query, dispatching by endpoint ownership.
pub fn active(
    state: &LoadedState,
    rules: &Rules,
    origin: ComponentId,
    destination: ComponentId,
) -> Result<Amounts, Error> {
    active_with_definitions(
        state,
        &Definitions::from_rules(state, rules)?,
        origin,
        destination,
    )
}
pub fn active_with_definitions(
    state: &LoadedState,
    definitions: &Definitions,
    origin: ComponentId,
    destination: ComponentId,
) -> Result<Amounts, Error> {
    active_on_graph(
        state,
        definitions,
        state.trade_graph().map_err(Error::State)?,
        origin,
        destination,
    )
}
pub(crate) fn active_on_graph(
    state: &LoadedState,
    definitions: &Definitions,
    graph: &c6_core::trade_graph_state::GraphState,
    origin: ComponentId,
    destination: ComponentId,
) -> Result<Amounts, Error> {
    if origin.owner == destination.owner {
        active_domestic_on_graph(state, definitions, graph, origin, destination)
    } else {
        active_international_on_graph(state, definitions, graph, origin, destination)
    }
}
/// Bounded foreign economic query. Class-dependent, alliance, suzerain,
/// cultural-dominance and resource/policy branches require explicit zero terms.
pub fn active_international(
    state: &LoadedState,
    rules: &Rules,
    origin: ComponentId,
    destination: ComponentId,
) -> Result<Amounts, Error> {
    active_international_with_definitions(
        state,
        &Definitions::from_rules(state, rules)?,
        origin,
        destination,
    )
}
pub fn active_international_with_definitions(
    state: &LoadedState,
    definitions: &Definitions,
    origin: ComponentId,
    destination: ComponentId,
) -> Result<Amounts, Error> {
    active_international_on_graph(
        state,
        definitions,
        state.trade_graph().map_err(Error::State)?,
        origin,
        destination,
    )
}
pub(crate) fn active_international_on_graph(
    state: &LoadedState,
    definitions: &Definitions,
    graph: &c6_core::trade_graph_state::GraphState,
    origin: ComponentId,
    destination: ComponentId,
) -> Result<Amounts, Error> {
    if definitions.inputs != state.binding().inputs {
        return Err(Error::Binding);
    }
    if origin.owner == destination.owner
        || origin.component_type != 2
        || destination.component_type != 2
    {
        return Err(Error::Requires("international city route yield query"));
    }
    let origin_city = state
        .city(i32::from(origin.owner), origin.id)
        .map_err(Error::State)?;
    let destination_city = state
        .city(i32::from(destination.owner), destination.id)
        .map_err(Error::State)?;
    let player = |owner| {
        state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("Trade player"))
    };
    let source = player(i32::from(origin.owner))?;
    let target = player(i32::from(destination.owner))?;
    let trade = &source.trade;
    let other = &target.trade;
    if state
        .game_relations()
        .map_err(Error::State)?
        .is_at_war(i32::from(origin.owner), i32::from(destination.owner))
        .map_err(|_| Error::Missing("Trade peace relationship"))?
    {
        return Err(Error::Requires("peaceful international yields"));
    }
    // 2c0ad70 reads destination ally/suzerain additions from the origin
    // player's Trade, while 2be4120 reads destination multipliers from the
    // destination player. All four relationship getters (2b967f0/2b968b0/
    // 2b96970/2b96a30) return only their table value or zero, without defaults.
    if source
        .culture
        .dominant_over
        .get(destination.owner as usize)
        .copied()
        .ok_or(Error::Missing("Trade cultural dominance"))?
    {
        return Err(Error::Requires("cultural dominance trade yields"));
    }
    if trade.techs_civics_per_yield != 0 {
        return Err(Error::Requires("progress difference trade yields"));
    }
    for table in [
        &trade.yield_bonus_per_trading_post_in_foreign_city,
        &trade.yield_bonus_per_trading_post_in_own_city,
        &trade.yield_bonus_per_path_tile,
        &trade.per_specialty_district_yield_bonus_for_international,
        &trade.yield_bonus_per_follower,
        &trade.origin_yield_for_ally_route,
        &trade.origin_yield_for_suzerain_route,
        &trade.destination_yield_for_ally_route,
        &trade.destination_yield_for_suzerain_route,
        &trade.flat_yield_bonus_for_city_state,
        &trade.yield_bonus_for_districts_city_state,
    ] {
        zero(
            table,
            "unsupported international path/specialty/follower/alliance/suzerain/city-state yields",
        )?;
    }
    for table in [
        &trade.flat_origin_yield_per_improvement_at_destination,
        &other.flat_destination_yield_per_improvement_at_destination,
        &trade.flat_international_yield_per_improvement_at_origin,
        &trade.flat_origin_yield_per_terrain_for_international,
        &other.flat_destination_yield_per_terrain_for_international,
    ] {
        nested_zero(table, "international improvement/terrain yields")?;
    }
    let edge = graph
        .graph()
        .get_edge(
            c6_core::trade_graph_state::city_vertex(origin),
            c6_core::trade_graph_state::city_vertex(destination),
        )
        .filter(|e| e.has_active_route)
        .ok_or(Error::Requires("active international route"))?;
    let route = &edge.most_recent_route;
    if route.origin_city != origin || route.destination_city != destination {
        return Err(Error::Requires("active route endpoint identity"));
    }
    if route.resource_type_hash != u32::MAX {
        return Err(Error::Requires("resource-attached international yields"));
    }
    let ratio = definitions.transport.ratio(state, &edge.most_recent_path)?;
    let map = state.world().map();
    let mut post_gold = 0i32;
    for &index in edge.most_recent_path.iter().skip(1) {
        let city = state
            .retained()
            .players
            .iter()
            .flat_map(|p| p.cities.records.entries.iter())
            .find(|c| {
                c.value.header.location_x == index % map.header.width
                    && c.value.header.location_y == index / map.header.width
            });
        if let Some(city) = city {
            if city
                .value
                .trade
                .constructed_trading_posts
                .get(origin.owner as usize)
                .copied()
                .unwrap_or(false)
                && !state
                    .game_relations()
                    .map_err(Error::State)?
                    .is_at_war(city.value.header.owner, i32::from(origin.owner))
                    .map_err(|_| Error::Missing("Trade trading-post peace relationship"))?
            {
                post_gold =
                    post_gold.wrapping_add(if city.value.header.owner == i32::from(origin.owner) {
                        definitions.transport.own_post_gold
                    } else {
                        definitions.transport.foreign_post_gold
                    });
            }
        }
    }
    // Intercontinental status is irrelevant only when its coefficient is zero.
    zero(
        &trade.flat_yield_bonus_for_intercontinental_international,
        "intercontinental international yields",
    )?;
    let mut amounts = Amounts {
        origin: vec![],
        destination: vec![],
    };
    for &hash in &definitions.yields {
        for table in [
            &origin_city
                .trade
                .yield_per_local_bonus_resource_for_international,
            &origin_city
                .trade
                .yield_per_destination_luxury_resource_for_international,
            &origin_city
                .trade
                .yield_per_destination_strategic_resource_for_international,
            &origin_city
                .trade
                .yield_per_local_luxury_resource_for_international,
            &origin_city
                .trade
                .yield_per_local_strategic_resource_for_international,
        ] {
            if city_value(table, hash)? > 0 {
                return Err(Error::Requires("international resource yields"));
            }
        }
        // This term is scaled by Congress for some minor classes. Bound it to
        // zero so no missing civilization-level fact selects a foreign branch.
        if city_value(&destination_city.trade.flat_yield_bonus_to_others, hash)? != 0 {
            return Err(Error::Requires(
                "destination city international bonus class",
            ));
        }
        let mut potential = 0i32;
        for row in &definitions.district_yields {
            if row.yield_hash == hash {
                potential = potential.wrapping_add(from_float(
                    count(state, origin, row.district)? as f32 * row.origin,
                )?);
                potential = potential.wrapping_add(from_float(
                    count(state, destination, row.district)? as f32 * row.foreign_destination,
                )?);
            }
        }
        let mut value = potential;
        if hash == linux_type_hash(b"YIELD_GOLD") {
            value = value.wrapping_add(post_gold);
            if ratio > 0 {
                value = value.wrapping_add(multiply(potential, ratio));
            }
        }
        for amount in [
            pair_value(&trade.flat_yield_bonus, hash, 0)?,
            pair_value(&trade.flat_yield_bonus_for_international, hash, 0)?,
            pair_value(&other.flat_yield_bonus_to_others, hash, 0)?,
            city_value(&origin_city.trade.flat_yield_bonus_for_international, hash)?,
        ] {
            value = value.wrapping_add(amount);
        }
        let mut bonus = None;
        for entry in &route.origin_yields {
            if entry.type_hash == hash && bonus.replace(entry.raw).is_some() {
                return Err(Error::Requires("unambiguous current route yield overlay"));
            }
        }
        let bonus = bonus.ok_or(Error::Missing("current active route origin yield"))?;
        value = value.wrapping_add(bonus);
        value = multiply(value, pair_value(&trade.origin_yield_modifier, hash, 256)?);
        value = multiply(
            value,
            pair_value(&trade.international_yield_modifier, hash, 256)?,
        );
        amounts.origin.push(YieldValue {
            type_hash: hash,
            raw: value,
        });
        let incoming = city_value(&destination_city.trade.flat_yield_bonus_from_others, hash)?
            .wrapping_add(bonus);
        amounts.destination.push(YieldValue {
            type_hash: hash,
            raw: multiply(
                multiply(
                    incoming,
                    pair_value(&other.international_yield_modifier, hash, 256)?,
                ),
                pair_value(&other.destination_yield_modifier, hash, 256)?,
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
