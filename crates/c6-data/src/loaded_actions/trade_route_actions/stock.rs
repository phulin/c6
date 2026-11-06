//! Source-qualified Linux XP2 Trade callbacks on the persistent stock backend.
//! Target SHA-256 a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa.
//! The caller supplies the ordered live subscriptions; unknown recipients and
//! positive unsupported branches retain the applied prefix and return an error.
use super::current::{Continuations, Event};
use crate::{
    found_city::{route_callbacks, routes},
    game_update_state::{round_quests, round_trade},
    loaded_actions::{
        choose_tech, stock_notifications, stock_notifications::research as stock_research,
    },
    melee_attack::resolve::completion::boosts::{ResearchTrigger, Trigger},
    research,
    unit_placement_services::Services,
    Rules,
};
use c6_core::{
    core_player::initialization::WireBinding, loaded_state::LoadedState, trade::ComponentId,
    unit::PlotCoord,
};

#[derive(Clone, Copy, Debug)]
pub enum ResourceSubscriber {
    /// Player.EventManager -> Resources.OnResourceTraded2b707b0. Resource=-1
    /// returns before any lookup, callback or RNG draw.
    PlayerResources {
        owner: i32,
    },
    /// City.EventManager's actual empty invoke2887210.
    CityEmpty,
    Unsupported {
        label: u64,
    },
}
#[derive(Clone, Copy, Debug)]
pub enum RouteSubscriber {
    QuestSendTradeRoute {
        quest_type: i32,
    },
    Boost {
        id: i32,
    },
    Diplomacy {
        owner: i32,
    },
    PlayerInfluence {
        owner: i32,
    },
    /// Actual Context.Achievements eligibility getter01f104e0. The enabled
    /// branch reads the actual origin player's immutable configuration name.
    Achievements {
        enabled: bool,
        leader_hash: u32,
    },
    CityTrade {
        owner: i32,
        city_ai_id: u32,
    },
    Unsupported {
        label: u64,
    },
}
struct BoostRule {
    id: i32,
    index: usize,
    threshold: i32,
}
pub struct Definitions {
    binding: WireBinding,
    boosts: Vec<BoostRule>,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, String> {
        if !matches!(
            rules.profile(),
            crate::RulesProfile::LinuxXp2Base | crate::RulesProfile::LinuxXp2ActivatedDiagnostic
        ) {
            return Err("Trade callback Linux XP2 rules".into());
        }
        let table = rules.gameplay().table("Boosts").ok_or("Trade Boosts")?;
        let mut boosts = vec![];
        for (index, row) in table.rows().enumerate() {
            if row.text("BoostClass").map_err(|e| format!("{e:?}"))?
                != "BOOST_TRIGGER_MAINTAIN_X_TRADE_ROUTES"
            {
                continue;
            }
            boosts.push(BoostRule {
                id: row.integer("BoostID").map_err(|e| format!("{e:?}"))? as i32,
                index,
                threshold: row.integer("NumItems").map_err(|e| format!("{e:?}"))? as i32,
            });
        }
        Ok(Self {
            binding: state.binding().clone(),
            boosts,
        })
    }
}
pub struct RoadInputs<'a> {
    pub definitions: &'a routes::Definitions,
    pub callbacks: &'a route_callbacks::Inputs,
    pub terrains: &'a [(u32, bool)],
}
pub struct Inputs<'a> {
    pub definitions: &'a Definitions,
    pub resource_started: &'a [ResourceSubscriber],
    pub route_started: &'a [RouteSubscriber],
    pub roads: RoadInputs<'a>,
}
/// Reporting invokes synchronous current queries before headless publication.
/// Implementations must qualify each reached typed event and reject unknown
/// consumers; there is deliberately no successful default implementation.
pub trait Reports<P> {
    fn report(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'_, P>,
        event: Event,
    ) -> Result<(), String>;
}
pub struct Current<'s, 'a, P> {
    pub inputs: &'s Inputs<'a>,
    pub research: &'s mut research::Runtime,
    pub research_definitions: &'s research::Definitions,
    pub choices: &'s choose_tech::Definitions,
    pub culture: Option<&'s stock_research::CultureInputs<'s>>,
    pub quests: &'s std::cell::RefCell<round_quests::Runtime>,
    pub reports: &'s mut dyn Reports<P>,
}
impl<P> Current<'_, '_, P> {
    fn check(&self, state: &LoadedState) -> Result<(), String> {
        if state.binding() != &self.inputs.definitions.binding
            || state.binding() != &self.research.binding
        {
            return Err("Trade callback binding".into());
        }
        Ok(())
    }
    fn route_started(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'_, P>,
        origin: ComponentId,
        destination: ComponentId,
    ) -> Result<(), String> {
        for subscriber in self.inputs.route_started {
            match *subscriber {
                RouteSubscriber::QuestSendTradeRoute { quest_type } => {
                    let quests = self.quests.borrow();
                    quests.validate(state).map_err(|e| format!("{e:?}"))?;
                    if origin.owner != -1
                        && destination.owner != -1
                        && quests.current.iter().any(|q| {
                            q.quest_type == quest_type
                                && q.actor == i32::from(origin.owner)
                                && q.client == i32::from(destination.owner)
                        })
                    {
                        return Err("Trade quest completion/rewards continuation".into());
                    }
                }
                RouteSubscriber::Boost { id } => {
                    if origin.owner == -1 {
                        continue;
                    }
                    let rule = self
                        .inputs
                        .definitions
                        .boosts
                        .iter()
                        .find(|r| r.id == id)
                        .ok_or("Trade BoostID/class")?;
                    // Manager.CountOutgoingRoutes: one active directed edge for
                    // each origin/destination city pair; temp routes excluded.
                    let graph = state.trade_graph().map_err(|e| format!("{e:?}"))?;
                    let mut count = 0i32;
                    let player = state
                        .retained()
                        .players
                        .iter()
                        .find(|p| p.header.owner == i32::from(origin.owner))
                        .ok_or("Trade boost source player")?;
                    for city in &player.cities.records.entries {
                        let vertex = c6_core::trade_graph_state::city_vertex(ComponentId {
                            owner: origin.owner,
                            component_type: 2,
                            id: city.value.header.id,
                        });
                        if let Some(row) = graph
                            .graph()
                            .origins()
                            .iter()
                            .find(|row| row.vertex == vertex)
                        {
                            for edge in &row.destinations {
                                if edge.edge.has_active_route {
                                    count = count.wrapping_add(1);
                                }
                            }
                        }
                    }
                    if count >= rule.threshold {
                        let mut effects = stock_research::Effects::new(
                            backend,
                            self.research,
                            self.choices,
                            self.culture,
                        );
                        ResearchTrigger {
                            definitions: self.research_definitions,
                            effects: &mut effects,
                        }
                        .trigger(state, i32::from(origin.owner), rule.index)
                        .map_err(|e| format!("Trade boost: {e:?}"))?;
                    }
                }
                RouteSubscriber::Diplomacy { owner } => {
                    if owner == i32::from(origin.owner) {
                        research::diplomacy::recheck(
                            state,
                            self.research_definitions.diplomatic_visibility(),
                            &self.research.civilization_levels,
                            owner,
                            i32::from(destination.owner),
                            |state, owner, other| {
                                self.reports.report(
                                    state,
                                    backend,
                                    Event::DiplomaticVisibilityChanged { owner, other },
                                )
                            },
                        )
                        .map_err(|e| format!("Trade diplomacy: {e:?}"))?;
                    }
                }
                RouteSubscriber::PlayerInfluence { owner } => {
                    if owner != i32::from(origin.owner) {
                        continue;
                    }
                    let player = state
                        .retained()
                        .players
                        .iter()
                        .find(|p| p.header.owner == owner)
                        .ok_or("Trade influence owner")?;
                    // The native final loop executes zero iterations for zero
                    // tokens regardless of its preceding diplomatic predicates.
                    if player.influence.tokens_on_trade_route_started > 0 {
                        return Err("Trade positive influence predicates/tokens".into());
                    }
                }
                RouteSubscriber::Achievements { enabled: false, .. } => {}
                RouteSubscriber::Achievements {
                    enabled: true,
                    leader_hash,
                } => {
                    // Achievements.OnTradeRouteStarted28fb6a0 reads immutable
                    // origin configuration first. Only Mansa Musa reaches the
                    // destination-city-name and route-yield queries.
                    if leader_hash == crate::registry::linux_type_hash(b"LEADER_MANSA_MUSA") {
                        return Err("Trade achievement current city/yield queries".into());
                    }
                }
                RouteSubscriber::CityTrade { owner, city_ai_id } => state
                    .observe_city_trade_route_started(owner, city_ai_id, origin)
                    .map_err(|e| format!("Trade CityAI: {e:?}"))?,
                RouteSubscriber::Unsupported { label } => {
                    return Err(format!("Trade route subscriber {label:#x}"))
                }
            }
        }
        Ok(())
    }
}
impl<P> Continuations<stock_notifications::Runtime<'_, P>> for Current<'_, '_, P> {
    fn event(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'_, P>,
        event: Event,
    ) -> Result<(), String> {
        self.check(state)?;
        match event {
            Event::RouteStarted {
                origin,
                destination,
            } => self.route_started(state, backend, origin, destination),
            Event::ResourceStarted { .. } => Err("Trade resource callback requires manager".into()),
            Event::ImmediateTradingPost { .. } => {
                Err("Trade immediate trading post subscribers".into())
            }
            _ => self.reports.report(state, backend, event),
        }
    }
    fn resource_started(
        &mut self,
        state: &mut LoadedState,
        _: &mut stock_notifications::Runtime<'_, P>,
        manager: &mut round_trade::Runtime,
        _: ComponentId,
        _: ComponentId,
    ) -> Result<(), String> {
        self.check(state)?;
        manager.validate(state).map_err(|e| format!("{e:?}"))?;
        for subscriber in self.inputs.resource_started {
            match subscriber {
                ResourceSubscriber::PlayerResources { .. } | ResourceSubscriber::CityEmpty => {}
                ResourceSubscriber::Unsupported { label } => {
                    return Err(format!("Trade resource subscriber {label:#x}"))
                }
            }
        }
        Ok(())
    }
    fn road(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'_, P>,
        sight: &crate::unit_sight_state::Runtime,
        plot: u32,
        owner: i32,
    ) -> Result<(), String> {
        self.check(state)?;
        complete_road(state, backend, sight, &self.inputs.roads, plot, owner)
    }
}
struct Road<'s, 'r, 'a, P> {
    backend: &'s mut stock_notifications::Runtime<'r, P>,
    inputs: &'s RoadInputs<'a>,
}
impl<P> routes::Effects for Road<'_, '_, '_, P> {
    type Error = String;
    fn water(&self, state: &LoadedState, plot: u32) -> Result<bool, String> {
        let hash = state
            .world()
            .map()
            .plots
            .get(plot as usize)
            .ok_or("Trade road plot")?
            .terrain_hash;
        self.inputs
            .terrains
            .iter()
            .find(|r| r.0 == hash)
            .map(|r| r.1)
            .ok_or("Trade road terrain".into())
    }
    fn alive_ids(&self) -> Result<Vec<i32>, String> {
        Ok(self
            .backend
            .placement_service_runtime()
            .map_err(|e| format!("{e:?}"))?
            .alive_players
            .as_ref()
            .ok_or("Trade road AlivePlayers")?
            .values()
            .to_vec())
    }
    fn visible(
        &self,
        state: &LoadedState,
        sight: &crate::unit_sight_state::Runtime,
        owner: i32,
        plot: u32,
    ) -> Result<bool, String> {
        let (x, y) = state
            .world()
            .map()
            .coordinates(plot)
            .ok_or("Trade road coordinate")?;
        let width = self
            .backend
            .placement_service_runtime()
            .map_err(|e| format!("{e:?}"))?
            .player::<String>(owner)
            .map_err(|e| format!("{e:?}"))?
            .native_visibility_width
            .ok_or("Trade road visibility width")?;
        crate::loaded_upkeep::placement_tails::plot_visible(sight, owner, width, PlotCoord { x, y })
            .map_err(str::to_string)
    }
    fn report(&mut self, _: &LoadedState, _: u32, event: u32) -> Result<(), String> {
        match event {
            0x5d0f4014 | 0x33d8725a => {
                self.backend
                    .reached
                    .push("Trade road headless connectivity publication");
                Ok(())
            }
            _ => Err(format!("Trade road report {event:#x}")),
        }
    }
    fn route_changed(
        &mut self,
        state: &mut LoadedState,
        plot: u32,
        route: i32,
        owner: i32,
    ) -> Result<(), String> {
        let (x, y) = state
            .world()
            .map()
            .coordinates(plot)
            .ok_or("Trade road callback coordinate")?;
        route_callbacks::route_changed(
            state,
            self.inputs.callbacks,
            PlotCoord { x, y },
            route,
            owner,
        )
        .map_err(|e| format!("{e:?}"))
    }
}

/// Reached automatic movement continuations require only current lifetime and
/// road effects. Starting a fresh route still requires the full action owner.
pub struct Ongoing<'s, 'a, 'reports, P> {
    pub lifetime: &'s super::lifetime::Definitions,
    pub ending: Option<&'s super::ending_stock::Inputs<'a>>,
    pub ending_reports: Option<&'s mut (dyn super::ending_stock::Reports<P> + 'reports)>,
    pub roads: &'s RoadInputs<'a>,
}
impl<P> Continuations<stock_notifications::Runtime<'_, P>> for Ongoing<'_, '_, '_, P> {
    fn expire_route(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'_, P>,
        origin: ComponentId,
        destination: ComponentId,
    ) -> Result<Option<()>, String> {
        let (Some(inputs), Some(reports)) = (self.ending, self.ending_reports.as_deref_mut())
        else {
            return Ok(None);
        };
        super::ending_stock::expire(state, backend, inputs, reports, origin, destination).map(Some)
    }

    fn earliest_end_turn(
        &self,
        state: &LoadedState,
        push_turn: i32,
    ) -> Result<Option<i32>, String> {
        self.lifetime
            .earliest_end_turn(state, push_turn)
            .map(Some)
            .map_err(|e| format!("{e:?}"))
    }
    fn event(
        &mut self,
        _: &mut LoadedState,
        _: &mut stock_notifications::Runtime<'_, P>,
        event: Event,
    ) -> Result<(), String> {
        Err(format!(
            "Trade automatic continuation cannot start a new route: {event:?}"
        ))
    }
    fn resource_started(
        &mut self,
        _: &mut LoadedState,
        _: &mut stock_notifications::Runtime<'_, P>,
        _: &mut round_trade::Runtime,
        _: ComponentId,
        _: ComponentId,
    ) -> Result<(), String> {
        Err("Trade automatic continuation cannot start a resource trade".into())
    }
    fn road(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'_, P>,
        sight: &crate::unit_sight_state::Runtime,
        plot: u32,
        owner: i32,
    ) -> Result<(), String> {
        complete_road(state, backend, sight, self.roads, plot, owner)
    }
}
fn complete_road<P>(
    state: &mut LoadedState,
    backend: &mut stock_notifications::Runtime<'_, P>,
    sight: &crate::unit_sight_state::Runtime,
    inputs: &RoadInputs<'_>,
    plot: u32,
    owner: i32,
) -> Result<(), String> {
    routes::complete(
        state,
        sight,
        inputs.definitions,
        plot,
        owner,
        &mut Road { backend, inputs },
    )
    .map(|_| ())
    .map_err(|e| format!("Trade road: {e:?}"))
}
