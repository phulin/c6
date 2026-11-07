//! Reached ordinary domestic and foreign route-end subscribers on the persistent backend.
//! Unknown subscriptions and positive unimplemented branches retain their prefix.
use super::{completion, stock::ResourceSubscriber, ComponentId};
use crate::{loaded_actions::stock_notifications, research};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState, trade::Route};

#[derive(Clone, Copy, Debug)]
pub enum PostChanged {
    History,
    Achievements { enabled: bool },
    Unsupported(u64),
}
#[derive(Clone, Copy, Debug)]
pub enum RouteEnded {
    Eras,
    Diplomacy { owner: i32 },
    CityTrade { owner: i32, city_ai_id: u32 },
    Unsupported(u64),
}
#[derive(Clone, Copy, Debug)]
pub enum Report {
    TradingPostNotification {
        city: ComponentId,
        owner: i32,
    },
    RouteLog {
        origin: ComponentId,
        destination: ComponentId,
    },
    DiplomaticVisibility {
        owner: i32,
        other: i32,
    },
    /// Execute normal Placement, operation clearing and FinishMoves only if
    /// source-selected relocation differs from the current location.
    RelocateTrader {
        trader: ComponentId,
        preferred: c6_core::unit::PlotCoord,
    },
    ManagerEnded {
        origin: ComponentId,
        destination: ComponentId,
        trader: ComponentId,
    },
}
pub trait Reports<P> {
    fn report(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'_, P>,
        event: Report,
    ) -> Result<(), String>;
}
pub struct Inputs<'a> {
    binding: WireBinding,
    pub resource_ended: &'a [ResourceSubscriber],
    pub post_creating_players: &'a [i32],
    pub post_changed: &'a [PostChanged],
    pub route_ended: &'a [RouteEnded],
    pub research: &'a research::Definitions,
    pub civilization_levels: &'a [(i32, u32)],
}
impl<'a> Inputs<'a> {
    pub fn from_initial(
        state: &LoadedState,
        resource_ended: &'a [ResourceSubscriber],
        post_creating_players: &'a [i32],
        post_changed: &'a [PostChanged],
        route_ended: &'a [RouteEnded],
        research: &'a research::Definitions,
        civilization_levels: &'a [(i32, u32)],
    ) -> Self {
        Self {
            binding: state.binding().clone(),
            resource_ended,
            post_creating_players,
            post_changed,
            route_ended,
            research,
            civilization_levels,
        }
    }
}
struct Current<'s, 'a, 'r, P> {
    backend: &'s mut stock_notifications::Runtime<'r, P>,
    inputs: &'s Inputs<'a>,
    reports: &'s mut dyn Reports<P>,
    completed: bool,
}
impl<P> completion::Effects for Current<'_, '_, '_, P> {
    type Error = String;
    fn trading_post_to_be_created(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        owner: i32,
    ) -> Result<(), String> {
        for &receiver in self.inputs.post_creating_players {
            state
                .world()
                .player(receiver)
                .map_err(|e| format!("{e:?}"))?;
            if receiver != owner
                || i32::from(city.owner) == owner
                || !(0..64).contains(&i32::from(city.owner))
            {
                continue;
            }
            let foreign = i32::from(city.owner);
            let level = self
                .inputs
                .civilization_levels
                .iter()
                .find(|r| r.0 == foreign)
                .ok_or("Trade post reward city civilization level")?
                .1;
            if level == 0xc2577075 {
                continue;
            }
            let trade = &state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == owner)
                .ok_or("Trade post reward current Player.Trade")?
                .trade;
            // Native reads each received flag only for a positive reward.
            // An unsupported random grant stops before its received-bit write.
            for (count, received, label) in [
                (
                    trade.free_tech_boost_first_trading_post_each_civ,
                    &trade.civ_received_tech_boost_first_trading_post,
                    "technology",
                ),
                (
                    trade.free_civic_boost_first_trading_post_each_civ,
                    &trade.civ_received_civic_boost_first_trading_post,
                    "civic",
                ),
            ] {
                if count > 0
                    && !*received
                        .get(foreign as usize)
                        .ok_or("Trade post reward received flag")?
                {
                    return Err(format!(
                        "Trade foreign first-post random {label} boost reward"
                    ));
                }
            }
        }
        Ok(())
    }
    fn trading_post_changed(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        owner: i32,
    ) -> Result<(), String> {
        for subscriber in self.inputs.post_changed {
            match subscriber {
                PostChanged::History if i32::from(city.owner) == owner => {}
                PostChanged::History => self.backend.current_trading_post_history(
                    state,
                    city,
                    owner,
                    self.inputs.civilization_levels,
                )?,
                PostChanged::Achievements { enabled: false } => {}
                PostChanged::Achievements { enabled: true } => {
                    // OnTradingPostChanged28f1940 can only unlock at count3.
                    // Its loop and HasTradingPostInOpponentCity2b970b0 query
                    // immutable definitions/current city bits, with no RNG.
                    let majors = self
                        .inputs
                        .civilization_levels
                        .iter()
                        .filter(|p| p.1 == 0x253718b0)
                        .count();
                    let owner_is_major = self
                        .inputs
                        .civilization_levels
                        .iter()
                        .any(|p| p.0 == owner && p.1 == 0x253718b0);
                    if !owner_is_major || majors > 3 {
                        return Err("Trade trading-post achievement positive count branch".into());
                    }
                }
                PostChanged::Unsupported(label) => {
                    return Err(format!("Trade post-changed subscriber {label:#x}"))
                }
            }
        }
        Ok(())
    }
    fn trading_post_notification(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        owner: i32,
    ) -> Result<(), String> {
        self.reports.report(
            state,
            self.backend,
            Report::TradingPostNotification { city, owner },
        )
    }
    fn cleanup_removed_route(
        &mut self,
        state: &mut LoadedState,
        route: &Route,
        origin: ComponentId,
        destination: ComponentId,
    ) -> Result<(), String> {
        if state
            .trade_graph()
            .map_err(|e| format!("{e:?}"))?
            .city_path(origin, destination)
            .is_some()
        {
            super::update_cities_along_path::<()>(state, origin, destination, -1)
                .map_err(|e| format!("{e:?}"))?;
        }
        self.reports.report(
            state,
            self.backend,
            Report::RouteLog {
                origin,
                destination,
            },
        )?;
        if route.resource_type_hash != u32::MAX {
            return Err("Trade attached-resource route end".into());
        }
        for subscriber in self.inputs.resource_ended {
            match subscriber {
                ResourceSubscriber::PlayerResources { .. } | ResourceSubscriber::CityEmpty => {}
                ResourceSubscriber::Unsupported { label } => {
                    return Err(format!("Trade resource-ended subscriber {label:#x}"))
                }
            }
        }
        let actor = c6_core::map::UnitReference::from_component_id(c6_core::unit::ComponentId {
            owner: route.trader_unit.owner,
            component_type: route.trader_unit.component_type,
            id: route.trader_unit.id,
        });
        if let Some(unit) = state.world().unit(actor).map_err(|e| format!("{e:?}"))? {
            let current = unit.record.k_location;
            let target = &state
                .city(i32::from(origin.owner), origin.id)
                .map_err(|e| format!("{e:?}"))?
                .header;
            let target = c6_core::unit::PlotCoord {
                x: target.location_x,
                y: target.location_y,
            };
            if current != target {
                self.reports.report(
                    state,
                    self.backend,
                    Report::RelocateTrader {
                        trader: route.trader_unit,
                        preferred: target,
                    },
                )?;
            }
            // RelocateTrader2c07800 selects the origin at distance0, then its
            // final location-equality guard omits Placement and moved effects.
        }
        for subscriber in self.inputs.route_ended {
            match *subscriber {
                RouteEnded::Eras if self.completed => {
                    let rate = state
                        .game_era_trade_completion_rate(i32::from(origin.owner))
                        .map_err(|e| format!("{e:?}"))?;
                    if rate.is_some_and(|v| v != 0) {
                        return Err("Trade completed-route positive era-score continuation".into());
                    }
                }
                RouteEnded::Eras => {}
                RouteEnded::Diplomacy { owner } if owner == i32::from(origin.owner) => {
                    research::diplomacy::recheck(
                        state,
                        self.inputs.research.diplomatic_visibility(),
                        self.inputs.civilization_levels,
                        owner,
                        i32::from(destination.owner),
                        |state, owner, other| {
                            self.reports.report(
                                state,
                                self.backend,
                                Report::DiplomaticVisibility { owner, other },
                            )
                        },
                    )
                    .map_err(|e| format!("Trade end diplomacy: {e:?}"))?;
                }
                RouteEnded::Diplomacy { .. } => {}
                RouteEnded::CityTrade { owner, city_ai_id } => state
                    .observe_city_trade_route_ended(owner, city_ai_id, origin)
                    .map_err(|e| format!("Trade end CityTrade: {e:?}"))?,
                RouteEnded::Unsupported(label) => {
                    return Err(format!("Trade route-ended subscriber {label:#x}"))
                }
            }
        }
        self.reports.report(
            state,
            self.backend,
            Report::ManagerEnded {
                origin,
                destination,
                trader: route.trader_unit,
            },
        )
    }
}
pub fn expire<P>(
    state: &mut LoadedState,
    backend: &mut stock_notifications::Runtime<'_, P>,
    inputs: &Inputs<'_>,
    reports: &mut dyn Reports<P>,
    origin: ComponentId,
    destination: ComponentId,
) -> Result<(), String> {
    if inputs.binding != *state.binding() || inputs.research.binding() != state.binding() {
        return Err("Trade completion binding".into());
    }
    completion::expire(
        state,
        origin,
        destination,
        &mut Current {
            backend,
            inputs,
            reports,
            completed: true,
        },
    )
    .map_err(|e| format!("Trade expiry: {e:?}"))
}

/// Trade.Manager.OnWarDeclared2c0c690: remove both directed player pairs before
/// any cleanup subscriber. Cancellation never constructs posts or dispatches a
/// successful-completion Eras callback. The shared edge-removal count still
/// increments, as it does in the original RemoveCurrentRoute leaf.
///
/// Native hash-table ordering for multiple edges in one direction is currently
/// unavailable. Reject that branch before any graph writes, rather than choose
/// wire order. One route in each direction has unambiguous source order.
pub fn interrupt_for_war<P>(
    state: &mut LoadedState,
    backend: &mut stock_notifications::Runtime<'_, P>,
    inputs: &Inputs<'_>,
    reports: &mut dyn Reports<P>,
    issuer: i32,
    target: i32,
) -> Result<(), String> {
    if inputs.binding != *state.binding() || inputs.research.binding() != state.binding() {
        return Err("Trade war interruption binding".into());
    }
    let graph = state.trade_graph().map_err(|e| format!("{e:?}"))?.graph();
    let mut pairs = Vec::new();
    for (owner, other) in [(issuer, target), (target, issuer)] {
        let mut direction = Vec::new();
        for origin in graph.origins() {
            for destination in &origin.destinations {
                let edge = &destination.edge;
                if !edge.has_active_route
                    || edge.from_vertex.kind != 0
                    || edge.to_vertex.kind != 0
                    || edge.from_vertex.data1 != owner
                    || edge.to_vertex.data1 != other
                {
                    continue;
                }
                direction.push((
                    ComponentId {
                        owner: owner as i16,
                        component_type: 2,
                        id: edge.from_vertex.data2 as u32,
                    },
                    ComponentId {
                        owner: other as i16,
                        component_type: 2,
                        id: edge.to_vertex.data2 as u32,
                    },
                ));
            }
        }
        if direction.len() > 1 {
            return Err("Trade war interruption multiple-edge native cleanup order".into());
        }
        pairs.extend(direction);
    }
    let mut removed = Vec::new();
    for (origin, destination) in pairs {
        if let Some(route) = state
            .trade_graph_mut()
            .map_err(|e| format!("{e:?}"))?
            .remove_route_by_city_pair(origin, destination)
        {
            removed.push((origin, destination, route));
        }
    }
    let mut current = Current {
        backend,
        inputs,
        reports,
        completed: false,
    };
    for (origin, destination, route) in removed {
        completion::Effects::cleanup_removed_route(
            &mut current,
            state,
            &route,
            origin,
            destination,
        )?;
    }
    Ok(())
}
