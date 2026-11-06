//! Stock headless joins for current-player deactivation and next activation.
//! These are internal phases; no function certifies a complete turn by itself.
use super::{turn_reports::Scope, Runtime};
use crate::loaded_actions::configuration::Configuration;
use crate::{game_update_state as data, Rules};
use c6_core::{
    game_update::Error,
    loaded_state::{game_update as loaded, LoadedState},
};

/// Borrowed current movement/visibility services for the mandatory eager
/// InfluenceMap rebuild. Missing positive path jobs remain explicit.
pub struct InfluenceInputs<'a, 'r> {
    pub definitions: &'a crate::influence_map_update::Definitions,
    pub capabilities: &'a c6_core::game_capabilities::Membership,
    pub visibility: &'a std::cell::RefCell<crate::unit_sight_state::Runtime>,
    pub movement: &'a crate::formation_movement_cost::Definitions<'r>,
    pub formation: &'a crate::formation_rules::Definitions,
    /// Current bound damage-strength rules, required by a live garrison.
    pub strength: Option<&'a crate::melee_attack::strength::Definitions>,
    pub cities: &'a crate::loaded_actions::city_range_attack::Queries<'a>,
}
struct ResourceEffects {
    speed: u32,
    uninitializing: bool,
}
impl data::deactivation::resources::Effects for ResourceEffects {
    type Error = &'static str;
    fn game_speed(&mut self, _: &LoadedState) -> Result<u32, Self::Error> {
        Ok(self.speed)
    }
    fn game_uninitializing(&mut self, _: &LoadedState) -> Result<bool, Self::Error> {
        Ok(self.uninitializing)
    }
    fn allocate_luxuries(&mut self, _: &mut LoadedState, _: i32) -> Result<(), Self::Error> {
        Err("current luxury allocation")
    }
    fn resource_changed(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: i32,
        _: i32,
        _: i32,
    ) -> Result<(), Self::Error> {
        Err("current ResourceChanged signal")
    }
    fn report_resource(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: i32,
        _: u32,
    ) -> Result<(), Self::Error> {
        Err("current resource report consumers")
    }
}
struct FormationContinuation<'b, 'a, P> {
    backend: &'b mut Runtime<'a, P>,
    placement: &'b dyn super::movement::PlacementQueries,
}
impl<P> crate::loaded_upkeep::Continuations for FormationContinuation<'_, '_, P> {
    type Error = super::movement::Error;
    fn preflight_movement(
        &self,
        state: &LoadedState,
        actor: crate::loaded_upkeep::UnitHandle,
        branch: bool,
    ) -> Result<(), Self::Error> {
        crate::loaded_upkeep::Effects::preflight_movement(self.backend, state, actor, branch)
    }
    fn preflight_formation(
        &self,
        state: &LoadedState,
        owner: i32,
        id: u32,
    ) -> Result<(), Self::Error> {
        crate::loaded_upkeep::Effects::preflight_formation(self.backend, state, owner, id)
    }
    fn can_embark(
        &mut self,
        state: &mut LoadedState,
        actor: crate::loaded_upkeep::UnitHandle,
    ) -> Result<bool, Self::Error> {
        let levels = self
            .backend
            .player_services
            .as_ref()
            .ok_or(Self::Error::Missing("current player levels"))?
            .players
            .iter()
            .map(|p| {
                Ok((
                    p.owner,
                    p.civilization_level
                        .ok_or(Self::Error::Missing("current player level"))?,
                ))
            })
            .collect::<Result<Vec<_>, Self::Error>>()?;
        self.placement
            .can_embark(
                state,
                actor.get(state).map_err(Self::Error::World)?,
                &levels,
            )
            .map_err(Self::Error::Placement)
    }
    fn report(
        &mut self,
        state: &mut LoadedState,
        report: c6_core::unit_turn_upkeep::MovementReport,
    ) -> Result<(), Self::Error> {
        crate::loaded_upkeep::Effects::report(self.backend, state, report)
    }
    fn owner_is_barbarian(&self, state: &LoadedState, owner: i32) -> Result<bool, Self::Error> {
        crate::loaded_upkeep::Effects::owner_is_barbarian(self.backend, state, owner)
    }
}
impl<P> crate::loaded_upkeep::FormationRestoreContinuations for FormationContinuation<'_, '_, P> {
    fn formation_restored(
        &mut self,
        state: &mut LoadedState,
        report: c6_core::unit_turn_start::FormationRestoredReport,
    ) -> Result<(), Self::Error> {
        if report.event_hash != 0xc52231ab {
            return Err(Self::Error::Missing("stock FormationRestored event"));
        }
        // The stock consumer publishes each CURRENT member snapshot. Its DTO
        // and presentation suspension do not mutate gameplay or RNG here.
        self.backend
            .reporting
            .observe_formation_snapshot(state, report.identity)
            .map_err(Self::Error::Rules)
    }
}
impl<P> Runtime<'_, P> {
    pub fn select_next_player(
        &self,
        state: &LoadedState,
        configuration: &Configuration<'_>,
        phases: &data::activation::Phases,
    ) -> Result<loaded::activation::Selection, Error> {
        let alive = self
            .player_services
            .as_ref()
            .and_then(|s| s.alive_players.as_ref())
            .ok_or(Error::Unsupported("current next-player alive order"))?;
        let network = configuration
            .is_network_multiplayer(state.binding())
            .map_err(|_| Error::Unsupported("current multiplayer mode"))?;
        loaded::activation::select(state, network, alive.values(), |state| phases.mode(state))
    }

    pub fn update_turn_influence(
        &self,
        state: &mut LoadedState,
        owner: i32,
        input: &InfluenceInputs<'_, '_>,
    ) -> Result<(), c6_core::influence_map_update::Error<crate::influence_map_update::Error>> {
        let alive = self
            .player_services
            .as_ref()
            .and_then(|s| s.alive_players.as_ref())
            .ok_or(c6_core::influence_map_update::Error::Backend(
                crate::influence_map_update::Error::Missing("current Influence alive order"),
            ))?;
        let visibility = input.visibility.try_borrow().map_err(|_| {
            c6_core::influence_map_update::Error::Backend(
                crate::influence_map_update::Error::Missing("current Influence sight borrow"),
            )
        })?;
        crate::influence_map_update::update(
            state,
            input.definitions,
            &crate::influence_map_update::Queries {
                alive_players: alive,
                capabilities: Some(input.capabilities),
                visibility: &visibility,
                movement: input.movement,
                formation: input.formation,
                city_range: input.cities,
                path: None,
                path_runtime: None,
                strength: input.strength,
            },
            owner,
        )
    }
    /// After automation, diplomacy, visibility recording and controller cleanup.
    pub fn finish_deactivation_resources(
        &mut self,
        state: &mut LoadedState,
        rules: &Rules,
        owner: i32,
        speed: u32,
        uninitializing: bool,
        scope: Scope,
    ) -> Result<usize, Error> {
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Unsupported("current deactivation player"))?;
        // Player.Cities.DoTurnDeactivate2a5b280 tailcalls
        // CheckUnhandledCityDecisions2a5b290. Its first pass only collects
        // cities whose current conquest/rebellion decision is outstanding.
        // Ordinary cities do not run production or citizen work here. Positive
        // decisions must reach an external policy, never an automatic default
        // destruction/flag clear in this headless adapter.
        for entry in &player.cities.records.entries {
            let city = &entry.value.header;
            if city.just_conquered_from != -1 || city.just_rebelled_from != -1 {
                self.last_error = Some(format!(
                    "unhandled City decision: owner {owner}, city {}, conquered from {}, rebelled from {}",
                    city.id, city.just_conquered_from, city.just_rebelled_from
                ));
                return Err(Error::Unsupported(
                    "pending city conquest/rebellion decision",
                ));
            }
        }
        let order = player
            .resources
            .resource_ownership
            .iter()
            .map(|e| e.resource_hash)
            .collect::<Vec<_>>();
        let definitions =
            data::deactivation::resources::Definitions::from_rules::<&'static str>(rules, &order)
                .map_err(|_| Error::Unsupported("current deactivation resource definitions"))?;
        let changed = data::deactivation::resources::deactivate(
            state,
            owner,
            &definitions,
            &mut ResourceEffects {
                speed,
                uninitializing,
            },
        )
        .map_err(|error| {
            self.last_error = Some(format!("resource deactivation: {error:?}"));
            Error::Unsupported("current resource deactivation effect")
        })?;
        self.deliver_deactivated_turn_report(state, owner, scope)
            .map_err(|_| Error::Unsupported("current deactivated turn report"))?;
        Ok(changed)
    }
    /// Source unit suffix followed by Formation.RestoreMovement, after the
    /// earlier activation subsystems and resource notifications have returned.
    #[allow(clippy::too_many_arguments)]
    pub fn restore_player_movement(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        units: &[u32],
        formations: &[u32],
        movement: &crate::unit_movement_state::Inputs<'_>,
        formation_definitions: &crate::formation_rules::Definitions,
        placement: &dyn super::movement::PlacementQueries,
    ) -> Result<(), Error> {
        for &id in units {
            crate::loaded_upkeep::activate_unit(
                state,
                owner,
                id,
                movement,
                formation_definitions,
                self,
            )
            .map_err(|error| {
                self.last_error = Some(format!("activation restore unit {owner}/{id}: {error:?}"));
                Error::Unsupported("activation current Unit restore suffix")
            })?;
        }
        let mut effects = crate::loaded_upkeep::RulesEffects {
            definitions: formation_definitions,
            continuations: FormationContinuation {
                backend: self,
                placement,
            },
        };
        for &id in formations {
            if let Err(error) = crate::loaded_upkeep::restore_formation_movement(
                state,
                owner,
                id,
                movement,
                &mut effects,
            ) {
                effects.continuations.backend.last_error = Some(format!("{error:?}"));
                return Err(Error::Unsupported(
                    "activation current Formation restore suffix",
                ));
            }
        }
        Ok(())
    }
    /// GameSignals+0x9d8 stock order: Lua PlayerTurnStartComplete, then the
    /// achievement observer (Linux 28fc620). Its city, district and existing
    /// path queries feed only achievement counters, tracking sets and platform
    /// unlock output. They have no gameplay callback, mutation or RNG draw.
    pub fn player_initialization_complete(
        &mut self,
        state: &LoadedState,
        owner: i32,
        _scope: Scope,
    ) -> Result<(), Error> {
        if let Err(error) = self
            .script
            .call_empty(state, "PlayerTurnStartComplete", &[owner])
        {
            self.last_error = Some(format!("{error:?}"));
            return Err(Error::Unsupported(
                "PlayerTurnStartComplete script callback",
            ));
        }
        self.reached
            .push("PlayerTurnStartComplete same script registry");
        self.reached
            .push("stock achievement observer output omitted");
        Ok(())
    }
    /// Processor.DoTurnActivate's final8953f126(owner,first). Stock consumers
    /// are cache producers/host translation; current route-domain queries run
    /// before their presentation publications are omitted.
    pub fn processor_activation_complete(
        &mut self,
        state: &LoadedState,
        owner: i32,
        first: bool,
        _scope: Scope,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("processor activation report binding"))?;
        // Player snapshot consumers resolve EventData1.player (+0x50), not
        // every player's City roster. The global Victory/relations snapshots
        // read existing manager fields; their output does not visit City yield
        // providers. A foreign City cannot require this owner's City queries.
        let _player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Unsupported("processor activation report player"))?;
        // OnPlayerTurnActivated27d29b0 -> PublishCacheUpdate27d2a90 ->
        // Add27d2520 fills a separate PlayerEntry, copies dynamic properties,
        // and publishes its cache ID. Fill27e2790 reads score/era/player flags
        // and agenda visibility only: it does not visit a City's providers,
        // change gameplay or draw RNG. The InfluenceMap consumer similarly
        // snapshots the existing map. These DTOs stay outside this boundary.
        self.turn_report_records
            .push(super::turn_reports::Record::ProcessorActivation {
                player: owner,
                first,
            });
        crate::loaded_actions::reporting::yield_monitor::warm_route_domain(state)
            .map_err(|_| Error::Unsupported("processor activation current route queries"))?;
        self.reached
            .push("stock processor activation cache publication");
        Ok(())
    }
    /// Set active, apply the source hotseat local-player/observer change, then
    /// emit activation reporting. First-activation upkeep still follows.
    pub fn activate_selected_player(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut loaded::Runtime,
        configuration: &Configuration<'_>,
        owner: i32,
        visibility_messages_disabled: bool,
        scope: Scope,
    ) -> Result<(), Error> {
        let Some(active) = loaded::activation::begin_active(state, owner)? else {
            return Ok(());
        };
        let output = loaded::activation::after_stock_log(
            state,
            runtime,
            active,
            loaded::deactivation::LoggingScope::StockFileSink,
            |state| {
                configuration
                    .is_hotseat(state.binding())
                    .map_err(|_| Error::Unsupported("current hotseat mode"))
            },
            |state, runtime, owner| {
                let change = loaded::activation::local_player::set(state, runtime, owner)?;
                let backend = std::cell::RefCell::new(&mut *self);
                change
                    .deliver(
                        state,
                        runtime,
                        |_| Ok(visibility_messages_disabled),
                        |state, event, owner| {
                            backend
                                .borrow_mut()
                                .deliver_turn_report(state, event, owner, scope)
                        },
                        |state, observer| {
                            backend
                                .borrow_mut()
                                .restore_turn_visibility(state, observer, scope)
                        },
                    )
                    .map_err(|_| Error::Unsupported("hotseat observer refresh"))
            },
        )?;
        output
            .deliver(state, |state, event, owner| {
                self.deliver_turn_report(state, event, owner, scope)
            })
            .map_err(|_| Error::Unsupported("current activation report"))
    }
}
