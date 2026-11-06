//! Ordered Player.Eras.SetEra callbacks on the current stock runtime.
//! Positive random boosts, pillaged-route upgrades and Gossip visibility delivery
//! remain explicit; supported callbacks read current canonical gameplay owners.
use super::{processing, turn_reports, Runtime};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayerSubscriber {
    EventManager,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameSubscriber {
    Gossip,
}
/// Explicit stock report dispatcher scope, with no added synchronous consumers.
#[derive(Clone, Copy, Debug)]
pub enum CityReportScope {
    StockHeadlessExternalPoliciesNoAddedConsumers,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CityReport {
    pub owner: i32,
    pub city: u32,
    pub era: i32,
}
pub struct Inputs {
    binding: WireBinding,
    players: Vec<(i32, Vec<PlayerSubscriber>)>,
    game: Vec<GameSubscriber>,
    routes: Option<RouteInputs>,
    city_reports: Option<CityReportScope>,
}
struct RouteInputs {
    definitions: crate::found_city::routes::Definitions,
    callbacks: crate::found_city::route_callbacks::Inputs,
    terrains: Vec<(u32, bool)>,
}
impl Inputs {
    pub fn with_city_reports(mut self, scope: CityReportScope) -> Self {
        self.city_reports = Some(scope);
        self
    }

    /// Immutable rules/subscriptions only; all gameplay owners remain live.
    pub fn with_routes(
        mut self,
        definitions: crate::found_city::routes::Definitions,
        callbacks: crate::found_city::route_callbacks::Inputs,
        terrains: Vec<(u32, bool)>,
    ) -> Self {
        self.routes = Some(RouteInputs {
            definitions,
            callbacks,
            terrains,
        });
        self
    }

    pub fn from_initial(
        state: &LoadedState,
        players: Vec<(i32, Vec<PlayerSubscriber>)>,
        game: Vec<GameSubscriber>,
    ) -> Result<Self, processing::Error> {
        for (index, (owner, _)) in players.iter().enumerate() {
            state
                .world()
                .player(*owner)
                .map_err(|_| processing::Error::Unsupported("era callback player"))?;
            if players[..index].iter().any(|p| p.0 == *owner) {
                return Err(processing::Error::Unsupported(
                    "duplicate era callback owner",
                ));
            }
        }
        Ok(Self {
            binding: state.binding().clone(),
            players,
            game,
            routes: None,
            city_reports: None,
        })
    }
}
pub(super) struct State<'a> {
    sight: &'a std::cell::RefCell<crate::unit_sight_state::Runtime>,
    inputs: Inputs,
    history_empty: Option<bool>,
    reports: Vec<CityReport>,
}
impl<'a, P> Runtime<'a, P> {
    pub fn era_city_reports(&self) -> &[CityReport] {
        self.era_changes
            .as_ref()
            .map_or(&[], |s| s.reports.as_slice())
    }

    pub fn bind_era_changes(
        &mut self,
        state: &LoadedState,
        inputs: Inputs,
        sight: &'a std::cell::RefCell<crate::unit_sight_state::Runtime>,
    ) -> Result<(), processing::Error> {
        self.check(state)?;
        if inputs.binding != *state.binding() {
            return Err(processing::Error::Unsupported("era callback input binding"));
        }
        self.era_changes = Some(State {
            inputs,
            sight,
            history_empty: None,
            reports: Vec::new(),
        });
        Ok(())
    }
    /// Query the SAME Session history for this activation scope. No initial
    /// history answer is retained; the prior scope is restored on every result.
    pub fn with_era_gossip_history<T>(
        &mut self,
        history: &[c6_core::city_power_activation::gossip_delivery::TurnVisibility],
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let previous = self
            .era_changes
            .as_mut()
            .map(|s| s.history_empty.replace(history.is_empty()));
        let result = f(self);
        if let (Some(s), Some(previous)) = (&mut self.era_changes, previous) {
            s.history_empty = previous;
        }
        result
    }
    /// Current activation's borrowed Session history. This is deliberately
    /// unavailable outside that scope; an initial empty answer is not retained.
    pub(super) fn current_gossip_history_empty(&self) -> Option<bool> {
        self.era_changes
            .as_ref()
            .and_then(|state| state.history_empty)
    }
    pub(super) fn player_era_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        old: i32,
        new: i32,
    ) -> Result<(), &'static str> {
        self.check(state).map_err(|_| "era runtime binding")?;
        let mut current = self
            .era_changes
            .take()
            .ok_or("current era callback registrations")?;
        let result = (|| {
            if current.inputs.binding != *state.binding() {
                return Err("era current owner binding");
            }
            let listeners = &current
                .inputs
                .players
                .iter()
                .find(|p| p.0 == owner)
                .ok_or("era current player registrations")?
                .1;
            for subscriber in listeners {
                match subscriber {
                    PlayerSubscriber::EventManager => {
                        // EventManager2b01650 calls Techs, Culture, Trade in order.
                        let p = state
                            .retained()
                            .players
                            .iter()
                            .find(|p| p.header.owner == owner)
                            .ok_or("era retained player")?;
                        if p.techs.num_random_tech_boosts_on_new_era > 0 {
                            return Err("era random technology boosts");
                        }
                        if p.culture.num_random_civic_boosts_on_new_era > 0 {
                            return Err("era random civic boosts");
                        }
                        let input = current
                            .inputs
                            .routes
                            .as_ref()
                            .ok_or("era route definitions and subscriptions")?;
                        // Trade.OnPlayerEraChanged2b97b40 scans immutable route
                        // prerequisites before touching any owned map route.
                        if input.definitions.unlocks_route_in_era(new) {
                            let sight = current
                                .sight
                                .try_borrow()
                                .map_err(|_| "era current sight borrow")?;
                            let roads =
                                crate::loaded_actions::trade_route_actions::stock::RoadInputs {
                                    definitions: &input.definitions,
                                    callbacks: &input.callbacks,
                                    terrains: &input.terrains,
                                };
                            crate::loaded_actions::trade_route_actions::stock::upgrade_player_roads(state, self, &sight, &roads, owner)
                                .map_err(|e| { self.last_error = Some(e); "era owned-route upgrade continuation" })?;
                            // 2b97bb0 checks capabilities AFTER UpgradePlayerRoutes.
                            if self
                                .social_capabilities
                                .as_ref()
                                .ok_or("era route-upgrade capabilities")?
                                .has(0x1ea085e4)
                            {
                                return Err("era route-upgrade notification continuation");
                            }
                        }
                        self.reached
                            .push("era player Techs Culture Trade callbacks complete");
                    }
                }
            }
            for subscriber in &current.inputs.game {
                match subscriber {
                    GameSubscriber::Gossip => match current.history_empty {
                        // AddGossipItem29b25d0 returns before archive/report when
                        // the current TurnVisibility vector is empty. Occurrence
                        // construction only writes the temporary type/owner.
                        Some(true) => {}
                        Some(false) => return Err("era Gossip visibility delivery"),
                        None => return Err("era current Session Gossip history scope"),
                    },
                }
            }
            // SetEra2aee880 visits the current recycled City container after
            // both signals. Event8d005711 has no stock cache registrations; its
            // host translator only forwards identity/new-era to an empty hook.
            let cities = &state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == owner)
                .ok_or("era report player")?
                .cities
                .records
                .entries;
            for city in cities {
                match current.inputs.city_reports {
                    Some(CityReportScope::StockHeadlessExternalPoliciesNoAddedConsumers) => {
                        current.reports.push(CityReport {
                            owner,
                            city: city.value.header.id,
                            era: new,
                        });
                    }
                    None => return Err("era current City report dispatcher scope"),
                }
            }
            // SetEra's final8152ea report feeds the stock EventStream DTO, with
            // no synchronous gameplay setters/signals or RNG on this route.
            self.deliver_round_report(
                state,
                0x008152ea,
                owner,
                new,
                old,
                turn_reports::Scope::StockPlainHeadlessExternalPoliciesNoAddedConsumers,
            )
            .map_err(|_| "era changed report")?;
            Ok(())
        })();
        self.era_changes = Some(current);
        result
    }
}
