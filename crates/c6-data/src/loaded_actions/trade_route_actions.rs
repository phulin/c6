//! Linux XP2 domestic Trade.Manager.StartRoute on the canonical warm graph.
//!
//! The queued operation owns admission, activity and subsequent trader movement.
//! This module owns the manager's route allocation and ordered gameplay signals.
//! A cold path or unimplemented subscriber returns an explicit incomplete result;
//! neither a captured path nor a successful empty callback is supplied here.
pub mod current;
pub mod path;
use crate::{game_update_state::round_trade, registry::linux_type_hash, Rules};
use c6_core::{
    core_player::initialization::InputIdentities,
    loaded_state::LoadedState,
    map::UnitReference,
    trade::{ComponentId, Route, YieldValue},
    trade_graph_state::city_vertex,
    unit::{OperationParameters, PlotCoord, Unit},
};

#[derive(Debug)]
pub enum Error<E = ()> {
    Binding,
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Rules(crate::Error),
    Manager(round_trade::Error),
    Graph(c6_core::trade_graph_state::Error),
    Path(Box<Error>),
    Missing(&'static str),
    Requires(&'static str),
    Effect(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "domestic trade route: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}

pub struct Definitions {
    inputs: InputIdentities,
    traders: Vec<u32>,
    yields: Vec<u32>,
    create_road: bool,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        if !matches!(
            rules.profile(),
            crate::RulesProfile::LinuxXp2Base | crate::RulesProfile::LinuxXp2ActivatedDiagnostic
        ) {
            return Err(Error::Requires("Linux XP2 trade rules"));
        }
        let table = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Missing("Units"))?;
        let mut traders = vec![];
        for row in table.rows() {
            if row.integer("MakeTradeRoute").map_err(Error::Rules)? != 0 {
                traders.push(linux_type_hash(
                    row.text("UnitType").map_err(Error::Rules)?.as_bytes(),
                ));
            }
        }
        let yields =
            crate::unit_initialization_definitions::collection(rules, "Yields", "YieldType")
                .map_err(|_| Error::Missing("Yield definitions"))?
                .entries()
                .iter()
                .map(|e| linux_type_hash(&e.type_name))
                .collect();
        let parameters = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Missing("GlobalParameters"))?;
        let create_road = parameters
            .rows()
            .find(|r| r.text("Name").ok() == Some("TRADE_ROUTE_PLACES_ROADS"))
            .and_then(|r| r.get("Value").ok())
            .and_then(crate::game_parameters::sql_integer)
            .ok_or(Error::Missing("TRADE_ROUTE_PLACES_ROADS"))?
            != 0;
        Ok(Self {
            inputs: state.binding().inputs.clone(),
            traders,
            yields,
            create_road,
        })
    }
    fn validate<E>(&self, state: &LoadedState) -> Result<(), Error<E>> {
        if self.inputs != state.binding().inputs {
            return Err(Error::Binding);
        }
        Ok(())
    }
}

fn city<E>(state: &LoadedState, at: PlotCoord) -> Result<Option<ComponentId>, Error<E>> {
    let Ok((_, plot)) = c6_core::movement_world::plot_at(state.world().map(), at) else {
        return Ok(None);
    };
    let Some(attrs) = &plot.city_attributes else {
        return Ok(None);
    };
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == i32::from(attrs.player))
        .ok_or(Error::Missing("city plot owner"))?;
    let Some(district) = player
        .districts
        .records
        .entries
        .iter()
        .find(|d| d.value.id == attrs.district)
    else {
        return Ok(None);
    };
    if district.value.district_hash != c6_core::city_district::CITY_CENTER_HASH {
        return Ok(None);
    }
    Ok(player
        .cities
        .records
        .entries
        .iter()
        .find(|c| c.value.header.id == district.value.city_id)
        .map(|c| ComponentId {
            owner: c.value.header.owner as i16,
            component_type: 2,
            id: c.value.header.id,
        }))
}
fn revealed<E>(state: &LoadedState, owner: i32, city: ComponentId) -> Result<bool, Error<E>> {
    let header = &state
        .city(i32::from(city.owner), city.id)
        .map_err(Error::State)?
        .header;
    let (index, _) = c6_core::movement_world::plot_at(
        state.world().map(),
        PlotCoord {
            x: header.location_x,
            y: header.location_y,
        },
    )
    .map_err(|_| Error::Missing("city plot location"))?;
    match state
        .world()
        .visibility(owner)
        .and_then(|v| v.revealed.get(index))
    {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => Err(Error::Missing("current revealed city byte")),
    }
}

/// Exact major-player domestic IsRouteAllowed branch, then its warm-path
/// HasExistingPath branch. A missing path requires the actual trade pathfinder.
/// `major` must come from the current player configuration, never player ID.
pub fn is_domestic_route_allowed<E>(
    state: &LoadedState,
    origin: ComponentId,
    destination: ComponentId,
    major: bool,
) -> Result<bool, Error<E>> {
    if origin == destination {
        return Ok(false);
    }
    if origin.owner != destination.owner {
        return Err(Error::Requires("international trade"));
    }
    if state.city(i32::from(origin.owner), origin.id).is_err()
        || state
            .city(i32::from(destination.owner), destination.id)
            .is_err()
    {
        return Ok(false);
    }
    let owner = i32::from(origin.owner);
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("Trade player"))?;
    if player.trade.domestic_trade_disabled_count > 0 {
        return Ok(false);
    }
    if !revealed(state, owner, origin)? || (major && !revealed(state, owner, destination)?) {
        return Ok(false);
    }
    let graph = state.trade_graph().map_err(Error::State)?;
    if graph
        .graph()
        .get_edge(city_vertex(origin), city_vertex(destination))
        .is_some_and(|e| e.has_active_route)
    {
        return Ok(false);
    }
    // Manager counts outgoing active edges over the CURRENT owned city roster.
    let count = player.cities.records.entries.iter().fold(0i32, |total, c| {
        let vertex = city_vertex(ComponentId {
            owner: origin.owner,
            component_type: 2,
            id: c.value.header.id,
        });
        graph
            .graph()
            .origins()
            .iter()
            .find(|o| o.vertex == vertex)
            .map_or(total, |o| {
                o.destinations
                    .iter()
                    .filter(|d| d.edge.has_active_route)
                    .fold(total, |n, _| n.wrapping_add(1))
            })
    });
    if count >= player.trade.outgoing_route_capacity {
        return Ok(false);
    }
    Ok(true)
}

/// Current cold pathfinder owner and rule inputs for the same action state.
/// Masks/capability facts come from the existing computed placement context.
pub struct Pathfinder<'a> {
    pub search: &'a mut c6_core::pathfinder::TwoLayer,
    pub definitions: &'a path::Definitions,
    pub inputs: path::Inputs<'a>,
}
impl Pathfinder<'_> {
    fn find(
        &mut self,
        state: &LoadedState,
        origin: ComponentId,
        destination: ComponentId,
        reverse: bool,
    ) -> Result<Option<Vec<i32>>, Error> {
        if reverse {
            if let Some(path) = state
                .trade_graph()
                .map_err(Error::State)?
                .reverse_city_path(origin, destination)
            {
                return Ok(Some(path.iter().collect()));
            }
        }
        let a = &state
            .city(i32::from(origin.owner), origin.id)
            .map_err(Error::State)?
            .header;
        let b = &state
            .city(i32::from(destination.owner), destination.id)
            .map_err(Error::State)?
            .header;
        path::find(
            state,
            self.definitions,
            path::Inputs {
                owner: self.inputs.owner,
                major: self.inputs.major,
                water_allowed: self.inputs.water_allowed,
                domains: self.inputs.domains,
                movement: self.inputs.movement,
            },
            self.search,
            PlotCoord {
                x: a.location_x,
                y: a.location_y,
            },
            PlotCoord {
                x: b.location_x,
                y: b.location_y,
            },
        )
    }
}
pub fn can_start_domestic<E>(
    state: &LoadedState,
    origin: ComponentId,
    destination: ComponentId,
    major: bool,
    pathfinder: Option<&mut Pathfinder<'_>>,
) -> Result<bool, Error<E>> {
    if !is_domestic_route_allowed(state, origin, destination, major)? {
        return Ok(false);
    }
    if state
        .trade_graph()
        .map_err(Error::State)?
        .city_path(origin, destination)
        .is_some()
    {
        return Ok(true);
    }
    let pathfinder = pathfinder.ok_or(Error::Requires("cold Trade.MovementPathFinder"))?;
    pathfinder
        .find(state, origin, destination, false)
        .map(|p| p.is_some())
        .map_err(|e| Error::Path(Box::new(e)))
}

/// The specified-destination branch of MakeTradeRoute.CanStart. Visible-only
/// returns after Trader/queue/movement checks, just as the original handler.
/// Complete Manager script tests and interruption checks precede this handler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationSite {
    Request,
    CurrentFront,
}
#[allow(clippy::too_many_arguments)]
pub fn handler_can_start<E>(
    state: &LoadedState,
    definitions: &Definitions,
    unit: &Unit,
    parameters: &OperationParameters,
    visible_only: bool,
    major: bool,
    site: OperationSite,
    pathfinder: Option<&mut Pathfinder<'_>>,
) -> Result<bool, Error<E>> {
    definitions.validate(state)?;
    if !definitions.traders.contains(&unit.e_type_index) {
        return Ok(false);
    }
    if site == OperationSite::Request
        && unit
            .k_operations
            .nodes
            .first()
            .is_some_and(|n| n.operation_hash == c6_core::unit::MAKE_TRADE_ROUTE_HASH)
    {
        return Ok(false);
    }
    if !c6_core::skip_turn::legality::can_move(unit) {
        return Ok(false);
    }
    if visible_only {
        return Ok(true);
    }
    let OperationParameters::MakeTradeRoute {
        location0,
        resource_hash,
        ..
    } = *parameters
    else {
        return Err(Error::Requires("MakeTradeRoute parameters"));
    };
    if !c6_core::skip_turn::placement::coordinate_valid(location0) {
        return Err(Error::Requires("unspecified trade destination enumeration"));
    }
    let origin = city(state, unit.k_location)?;
    let destination = city(state, location0)?;
    let (Some(origin), Some(destination)) = (origin, destination) else {
        return Ok(false);
    };
    if !can_start_domestic(state, origin, destination, major, pathfinder)? {
        return Ok(false);
    }
    // XP2 resource method first performs CanStartRoute, then accepts only NONE.
    Ok(resource_hash == u32::MAX)
}

/// All synchronous gameplay continuations borrow the SAME state and Trade
/// runtime. Implementations execute ordered subscribers/RNG or return an error.
/// Their return value is never substituted for route validity or player policy.
pub trait Effects {
    type Error;
    fn trader_route_changed(
        &mut self,
        state: &mut LoadedState,
        trader: ComponentId,
    ) -> Result<(), Self::Error>;
    fn resource_route_started(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut round_trade::Runtime,
        origin: ComponentId,
        destination: ComponentId,
    ) -> Result<(), Self::Error>;
    fn immediate_trading_post(
        &mut self,
        state: &mut LoadedState,
        origin: ComponentId,
        destination: ComponentId,
    ) -> Result<(), Self::Error>;
    fn route_started(
        &mut self,
        state: &mut LoadedState,
        origin: ComponentId,
        destination: ComponentId,
    ) -> Result<(), Self::Error>;
    fn route_reports(
        &mut self,
        state: &mut LoadedState,
        actor: (i32, u32),
        origin: ComponentId,
        destination: ComponentId,
    ) -> Result<(), Self::Error>;
}

/// Manager StartRoute's supported existing-path domestic branch. Queue/start
/// and handler XP2 favor/report tails remain owned by the operation adapter.
/// Any callback error retains ID allocation, Trader and Graph writes in order.
#[allow(clippy::too_many_arguments)]
pub fn start<E: Effects>(
    state: &mut LoadedState,
    definitions: &Definitions,
    runtime: &mut round_trade::Runtime,
    actor: (i32, u32),
    destination: PlotCoord,
    major: bool,
    effects: &mut E,
    mut pathfinder: Option<&mut Pathfinder<'_>>,
) -> Result<Option<i32>, Error<E::Error>> {
    definitions.validate(state)?;
    runtime.validate(state).map_err(Error::Manager)?;
    let unit = state
        .world()
        .unit(UnitReference::from_component_id(
            c6_core::unit::ComponentId {
                owner: actor.0 as i16,
                component_type: 1,
                id: actor.1,
            },
        ))
        .map_err(Error::World)?
        .ok_or(Error::Missing("Trader"))?;
    let origin = city(state, unit.record.k_location)?;
    let destination = city(state, destination)?;
    let (Some(origin), Some(destination)) = (origin, destination) else {
        return Ok(None);
    };
    if !can_start_domestic(state, origin, destination, major, pathfinder.as_deref_mut())? {
        return Ok(None);
    }
    if state
        .trade_graph()
        .map_err(Error::State)?
        .city_path(origin, destination)
        .is_none()
    {
        let path = pathfinder
            .as_mut()
            .ok_or(Error::Requires("cold Trade path insertion"))?
            .find(state, origin, destination, true)
            .map_err(|e| Error::Path(Box::new(e)))?
            .unwrap_or_default();
        state
            .trade_graph_mut()
            .map_err(Error::State)?
            .add_path_to_city(origin, destination, &path, &definitions.yields)
            .map_err(Error::Graph)?;
    }
    let trader = ComponentId {
        owner: i16::try_from(actor.0).map_err(|_| Error::Missing("Trader owner"))?,
        component_type: 1,
        id: actor.1,
    };
    let id = runtime.allocate_route_id().map_err(Error::Manager)?;
    let route = Route {
        id,
        start_turn: state.retained().core.current_turn,
        trader_unit: trader,
        resource_type_hash: u32::MAX,
        resource_quantity: 0,
        origin_city: origin,
        destination_city: destination,
        origin_yields: definitions
            .yields
            .iter()
            .map(|&type_hash| YieldValue { type_hash, raw: 0 })
            .collect(),
        destination_yields: definitions
            .yields
            .iter()
            .map(|&type_hash| YieldValue { type_hash, raw: 0 })
            .collect(),
    };
    let trade = &mut state
        .unit_mut(actor.0, actor.1)
        .map_err(Error::State)?
        .k_trade;
    trade.k_origin_trade_city = c6_core::unit::ComponentId {
        owner: origin.owner,
        component_type: origin.component_type,
        id: origin.id,
    };
    trade.k_destination_trade_city = c6_core::unit::ComponentId {
        owner: destination.owner,
        component_type: destination.component_type,
        id: destination.id,
    };
    effects
        .trader_route_changed(state, trader)
        .map_err(Error::Effect)?;
    state
        .trade_graph_mut()
        .map_err(Error::State)?
        .add_route_to_city(origin, destination, &route);
    effects
        .resource_route_started(state, runtime, origin, destination)
        .map_err(Error::Effect)?;
    if runtime
        .temporary_routes()
        .iter()
        .any(|r| r.origin_city == origin && r.destination_city == destination)
    {
        return Err(Error::Requires(
            "temporary route finalizing signal and destruction",
        ));
    }
    // UpdateCitiesAlongPath only writes foreign city centers. This supported
    // domain checks every node before omitting that branch; owned city centers
    // still participate in the path and are retained in the graph.
    let path = state
        .trade_graph()
        .map_err(Error::State)?
        .city_path(origin, destination)
        .ok_or(Error::Missing("current route path"))?;
    for &index in path {
        let width = state.world().map().header.width;
        if index < 0 || width <= 0 || index as usize >= state.world().map().plots.len() {
            return Err(Error::Missing("route path plot"));
        }
        if let Some(city) = city(
            state,
            PlotCoord {
                x: index % width,
                y: index / width,
            },
        )? {
            if city.owner != origin.owner {
                return Err(Error::Requires("foreign city route-passing update"));
            }
        }
    }
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == i32::from(origin.owner))
        .ok_or(Error::Missing("current Trade player"))?;
    if player.trade.immediate_trading_post_count > 0 {
        effects
            .immediate_trading_post(state, origin, destination)
            .map_err(Error::Effect)?;
    }
    effects
        .route_started(state, origin, destination)
        .map_err(Error::Effect)?;
    effects
        .route_reports(state, actor, origin, destination)
        .map_err(Error::Effect)?;
    Ok(Some(id))
}
