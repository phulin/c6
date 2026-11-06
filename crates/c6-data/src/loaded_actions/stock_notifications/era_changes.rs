//! Ordered Player.Eras.SetEra callbacks on the current stock runtime.
//! Positive random boosts, pillaged-route upgrades and eligible Gossip delivery
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
/// Stock informational output scope, with no added synchronous consumers.
#[derive(Clone, Copy, Debug)]
pub enum RouteUpgradeNoticeScope {
    /// Actual stock default factory, plain resources and no gameplay consumers.
    StockPlainHeadlessExternalPoliciesNoAddedConsumers,
}

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
    route_upgrade_notice: Option<RouteUpgradeNoticeScope>,
}
struct RouteInputs {
    definitions: crate::found_city::routes::Definitions,
    callbacks: crate::found_city::route_callbacks::Inputs,
    terrains: Vec<(u32, bool)>,
}
impl Inputs {
    /// Qualify the default manual 5c608b27 notification path with stock plain
    /// resources, default callbacks and no added gameplay/report consumers.
    pub fn with_route_upgrade_notice(mut self, scope: RouteUpgradeNoticeScope) -> Self {
        self.route_upgrade_notice = Some(scope);
        self
    }

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
            route_upgrade_notice: None,
        })
    }
}
pub(super) struct State<'a> {
    sight: &'a std::cell::RefCell<crate::unit_sight_state::Runtime>,
    inputs: Inputs,
    history_empty: Option<bool>,
    gossip_visibility: Option<GossipVisibility>,
    gossip_definition: Option<Option<c6_core::city_power_activation::gossip_delivery::Definition>>,
    gossip_archive: Vec<EraGossip>,
    reports: Vec<CityReport>,
}
/// Semantic archive entry created before the synchronous recipient scan. It is
/// retained even if an eligible delivery later returns an unsupported effect.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EraGossip {
    pub turn: i32,
    pub owner: i32,
    pub old_era: i32,
    pub new_era: i32,
}
struct GossipVisibility {
    manager: c6_core::city_power_activation::gossip_delivery::Manager<()>,
    alive_majors: Vec<i32>,
}
impl GossipVisibility {
    fn require_no_delivery(
        &self,
        owner: i32,
        definition: Option<c6_core::city_power_activation::gossip_delivery::Definition>,
    ) -> Result<(), &'static str> {
        // EraChanged.GetOtherPlayer is exactly -1. ReportGossip therefore does
        // not reach its alternative-other-player visibility comparison.
        for &recipient in &self.alive_majors {
            let visibility = self.manager.most_recent(recipient, owner);
            let Some(definition) = definition else {
                continue;
            };
            if i32::from(visibility.visibility) < definition.visibility_level {
                continue;
            }
            if recipient == owner || recipient == -1 {
                continue;
            }
            return Err(if definition.filter {
                "era Gossip filtered occurrence continuation"
            } else {
                "era Gossip eligible recipient delivery"
            });
        }
        Ok(())
    }
}
impl<'a, P> Runtime<'a, P> {
    pub fn era_gossip_archive(&self) -> &[EraGossip] {
        self.era_changes
            .as_ref()
            .map_or(&[], |s| s.gossip_archive.as_slice())
    }

    /// Bind the current rules lookup for occurrence type d0b71d9f. None means
    /// the admitted definition collection has no such definition, not unknown.
    pub fn bind_era_gossip_definition(
        &mut self,
        state: &LoadedState,
        definition: Option<c6_core::city_power_activation::gossip_delivery::Definition>,
    ) -> Result<(), processing::Error> {
        self.check(state)?;
        if definition.is_some_and(|d| d.type_hash != 0xd0b71d9f) {
            return Err(processing::Error::Unsupported("era Gossip definition type"));
        }
        self.era_changes
            .as_mut()
            .ok_or(processing::Error::Unsupported("era callback registrations"))?
            .gossip_definition = Some(definition);
        Ok(())
    }

    /// Scope the actual canonical Session visibility owner for this activation.
    /// The copy expires on return; it is never an initial or fixture answer.
    /// Supported recipient skips contain no callback capable of changing it.
    pub fn with_era_gossip_visibility<T>(
        &mut self,
        visibility: &crate::game_update_state::deactivation::VisibilityHistory,
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let previous = self.era_changes.as_mut().map(|s| {
            let old_empty = s.history_empty.replace(visibility.history.is_empty());
            let old_visibility = s.gossip_visibility.replace(GossipVisibility {
                manager: c6_core::city_power_activation::gossip_delivery::Manager {
                    history: visibility.history.clone(),
                    fallback: visibility.fallback,
                    pending: Vec::new(),
                    filtered: Vec::new(),
                },
                alive_majors: visibility.alive_majors.clone(),
            });
            (old_empty, old_visibility)
        });
        let result = f(self);
        if let (Some(s), Some((old_empty, old_visibility))) = (&mut self.era_changes, previous) {
            s.history_empty = old_empty;
            s.gossip_visibility = old_visibility;
        }
        result
    }

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
            gossip_visibility: None,
            gossip_definition: None,
            gossip_archive: Vec::new(),
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
        self.action_gossip_history_empty.or_else(|| {
            self.era_changes
                .as_ref()
                .and_then(|state| state.history_empty)
        })
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
                                match current.inputs.route_upgrade_notice {
                                    Some(RouteUpgradeNoticeScope::StockPlainHeadlessExternalPoliciesNoAddedConsumers) => {
                                        // Send2d4b510 creates a manual default notice,
                                        // sets its target to Player.Trade component(18, id0) and text,
                                        // then sends it. Qualified stock OnAdd/supersede
                                        // and added-report consumers have no gameplay or
                                        // RNG effects; retain only the semantic output.
                                        self.publish_notice(state, super::Notice::RoutesUpgraded { owner, era: new })
                                            .map_err(|e| { self.last_error = Some(format!("{e:?}")); "era route-upgrade notice publication" })?;
                                    }
                                    None => return Err("era route-upgrade notification continuation"),
                                }
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
                        Some(false) => {
                            // Handler29bb5f0 constructs the occurrence for every
                            // owner, including NPCs. Add29b25d0 archives before
                            // Report29b2670 examines current alive-major history.
                            current.gossip_archive.push(EraGossip {
                                turn: state.retained().core.current_turn,
                                owner,
                                old_era: old,
                                new_era: new,
                            });
                            let visibility = current
                                .gossip_visibility
                                .as_ref()
                                .ok_or("era current Session Gossip visibility scope")?;
                            let definition = current
                                .gossip_definition
                                .ok_or("era current Gossip definition")?;
                            visibility.require_no_delivery(owner, definition)?;
                        }
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

#[cfg(test)]
mod gossip_tests {
    use super::*;
    use c6_core::city_power_activation::gossip_delivery::{Definition, Manager, TurnVisibility};

    fn scope(history: Vec<TurnVisibility>, fallback: i8) -> GossipVisibility {
        GossipVisibility {
            manager: Manager {
                history,
                fallback: TurnVisibility {
                    visibility: fallback,
                    ..TurnVisibility::constructed()
                },
                pending: Vec::new(),
                filtered: Vec::new(),
            },
            alive_majors: vec![0, 1],
        }
    }
    fn definition() -> Option<Definition> {
        Some(Definition {
            type_hash: 0xd0b71d9f,
            visibility_level: 0,
            filter: false,
        })
    }
    #[test]
    fn unrelated_trade_history_selects_actual_fallback_for_npc_era() {
        let history = vec![TurnVisibility {
            viewing_player: 0,
            viewed_player: 1,
            visibility: 1,
            ..TurnVisibility::constructed()
        }];
        assert!(scope(history.clone(), -1)
            .require_no_delivery(63, definition())
            .is_ok());
        // A loaded fallback is a live input, not an assumed constructor value.
        assert_eq!(
            scope(history, 0).require_no_delivery(63, definition()),
            Err("era Gossip eligible recipient delivery")
        );
    }
    #[test]
    fn latest_matching_insertion_and_owner_exclusion_control_delivery() {
        let history = vec![
            TurnVisibility {
                turn: 99,
                viewing_player: 1,
                viewed_player: 0,
                visibility: 3,
                ..TurnVisibility::constructed()
            },
            TurnVisibility {
                turn: 1,
                viewing_player: 1,
                viewed_player: 0,
                visibility: -1,
                ..TurnVisibility::constructed()
            },
        ];
        assert!(scope(history, 3)
            .require_no_delivery(0, definition())
            .is_ok());
        assert_eq!(
            scope(Vec::new(), 0).require_no_delivery(0, definition()),
            Err("era Gossip eligible recipient delivery")
        );
        assert!(scope(Vec::new(), 3).require_no_delivery(63, None).is_ok());
    }
}
