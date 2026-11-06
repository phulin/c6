//! Reusable stock Linux turn composition over initialized, retained owners.
//!
//! This binds existing bounded upkeep implementations; unsupported reached
//! branches remain errors. Every player policy choice is returned to Session.
mod production_dispatch;
use super::advance::TurnServices;
use crate::game_update_state::{
    self as data,
    activation::{self, upkeep},
};
use crate::loaded_actions::stock_notifications::{
    self as stock, human_activation, inactive_control, npc_activation, npc_leaves,
};
use c6_core::loaded_state::{game_update as loaded, LoadedState};
use std::cell::RefCell;
pub type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

/// Per-human unsaved callback inputs, retained across all advances.
pub struct Human<'s> {
    pub inputs: &'s human_activation::Inputs,
    pub scout: &'s activation::maintenance::ScoutInputs,
    pub initialized: bool,
}
/// Current city caches and live yield providers. No world snapshot is copied.
pub struct City<'s> {
    pub definitions: &'s crate::city_turn::yield_calculation::Definitions,
    pub current: &'s crate::city_turn::current_upkeep::Current<'s>,
    pub housing: &'s crate::city_turn::housing::Definitions,
    pub configuration: &'s crate::loaded_actions::configuration::Configuration<'s>,
    pub culture: &'s crate::city_turn::upkeep::culture::Definitions,
    pub loyalty: &'s crate::city_turn::upkeep::loyalty::Definitions,
    pub power: &'s crate::city_power::Definitions,
    pub combat: &'s crate::loaded_actions::city_range_attack::Definitions,
    pub workers: &'s crate::city_turn::upkeep::citizens::Definitions,
    pub buyable: &'s crate::city_turn::upkeep::buyable_plots::Definitions,
    pub appeal: &'s crate::city_production_turn::appeal::Definitions,
    pub properties: &'s [(i32, Vec<(u32, i32)>)],
    pub alive: &'s [i32],
    pub levels: &'s [(i32, u32)],
    pub fallout: &'s [i32],
    pub loyalty_subscribers: Option<usize>,
    /// Current district completion services for naturally accrued production.
    pub district: Option<&'s crate::district_actions::stock::Inputs<'s>>,
}
/// Ordinary Train production borrows the ongoing factory and current legality
/// inputs. Sight is borrowed from `influence.visibility` only during activation;
/// creation callbacks and modifier owners remain on the Session backend.
pub struct UnitProduction<'s, 'd, 'r> {
    pub selection: &'s crate::city_production_turn::unit_selection::Definitions,
    pub current: crate::city_production_turn::unit_selection::Current<'s>,
    pub definitions: &'s crate::unit_creation::Definitions,
    pub resources: &'s crate::resource_binding::ResourceAuthority<'r>,
    pub combat: &'s crate::formation_rules::Definitions,
    pub costs: &'s crate::research_unit_costs::Definitions,
    pub placement: &'s crate::unit_placement_state::Definitions<'d, 'r>,
    pub queries: &'s crate::unit_placement_state::Queries<'d>,
    pub runtime: &'s crate::unit_creation::Runtime,
    pub tags: &'s crate::loaded_actions::reporting::hold_cycling::commands::Tags,
    pub finish: &'s crate::city_production_turn::units::Definitions,
    pub reports: Vec<stock::city_production::unit::Record>,
}
/// Camp factories and subscribers are retained, including their unsaved state.
pub struct Camp<'s, 'd, 'r> {
    pub definitions: &'s crate::barbarian_camps::queries::Definitions,
    pub placement: &'s crate::barbarian_camp_placement::Rules,
    pub factory: crate::barbarian_camp_units::FactoryInputs<'s, 'd, 'r>,
    pub alive: &'s [i32],
    pub alive_majors: &'s [i32],
    pub parameters: crate::barbarian_activation::Parameters,
    pub services: Option<crate::barbarian_camps::services::Current<'s>>,
}
/// Configure once after binding the Session backend. Mutable gameplay owners
/// remain borrowed here and are reused by activation, off-turn and resumption.
pub struct StockTurnServices<'s, 'd, 'r> {
    pub requirements: &'s RefCell<activation::requirement_changes::Runtime>,
    pub listeners: &'s [activation::turn_started::Listener],
    pub relationships: &'s activation::relationship_requirements::Definitions,
    pub victory_definitions: &'s activation::victory_requirements::Definitions,
    pub victory: &'s activation::victory_requirements::Inputs,
    pub influence: stock::turn::InfluenceInputs<'s, 'r>,
    pub humans: &'s [Human<'s>],
    pub npc: &'s [&'s npc_leaves::Inputs],
    pub inactive: &'s RefCell<Vec<inactive_control::Inputs>>,
    pub scoring: &'s RefCell<stock::scoring::Runtime>,
    pub scoring_definitions: &'s stock::scoring::Definitions,
    pub research_definitions: &'s crate::research::Definitions,
    pub choices: &'s crate::loaded_actions::choose_tech::Definitions,
    pub culture: &'s stock::research::CultureInputs<'s>,
    pub treasury: &'s activation::treasury_deals::Definitions,
    pub deals: &'s mut activation::treasury_deals::CurrentDeals,
    pub social_definitions: &'s activation::social::Definitions,
    pub social_queries: &'s activation::social::Queries<'s>,
    pub notifications: &'s crate::loaded_actions::notifications::Definitions,
    pub manager_definitions: &'s activation::cityless_managers::Definitions,
    pub manager_queries: &'s activation::cityless_managers::Queries<'s>,
    pub managers: &'s RefCell<activation::cityless_managers::Runtime>,
    pub maintenance: &'s stock::maintenance::Definitions,
    pub movement: &'s crate::unit_movement_state::Inputs<'d>,
    pub placement: &'s dyn stock::movement::PlacementQueries,
    pub phases: &'s activation::Phases,
    pub forced_rebase: &'s activation::forced_rebase::Definitions,
    pub city: Option<City<'s>>,
    pub unit_production: Option<UnitProduction<'s, 'd, 'r>>,
    pub camps: Option<Camp<'s, 'd, 'r>>,
    pub city_outcomes: Vec<(i32, u32, crate::city_production_turn::Outcome)>,
    pub city_reports: Vec<stock::city_production::Record>,
}
impl StockTurnServices<'_, '_, '_> {
    fn human(&self, owner: i32) -> Result<&Human<'_>> {
        self.humans
            .iter()
            .find(|h| h.inputs.owner == owner)
            .ok_or_else(|| format!("current human inputs absent for {owner}").into())
    }
    fn major(&self, owner: i32) -> Result<bool> {
        self.social_queries
            .levels
            .iter()
            .find(|r| r.0 == owner)
            .map(|r| r.1 == 0x253718b0)
            .ok_or_else(|| format!("current civilization level absent for {owner}").into())
    }
}
impl<'a> TurnServices<'a> for StockTurnServices<'_, '_, '_> {
    type Error = Error;
    fn activate(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock::Runtime<'a>,
        game_update: &mut loaded::Runtime,
        research_runtime: &mut crate::research::Runtime,
        history: &crate::game_update_state::deactivation::VisibilityHistory,
        owner: i32,
        first: bool,
    ) -> Result<upkeep::ActivationOutcome> {
        let major = self.major(owner)?;
        let human = if state.player_slot_status(owner)? == 3 {
            self.humans
                .iter()
                .find(|h| h.inputs.owner == owner)
                .ok_or("current human activation inputs absent")?
        } else {
            self.humans
                .first()
                .ok_or("current human tail inputs absent")?
        };
        let mut required_guard = self.requirements.borrow_mut();
        let required = &mut *required_guard;
        let mut signal = data::activation::upkeep::StartSignal {
            requirements: required,
            listeners: self.listeners,
            relationships: self.relationships,
            victory_definitions: self.victory_definitions,
            victory: self.victory,
        };
        let unit_order = state
            .world()
            .player(owner)?
            .unit_records()?
            .iter()
            .map(|r| r.value.id)
            .collect::<Vec<_>>();
        let formation_order = state
            .world()
            .player(owner)?
            .formation_records()?
            .iter()
            .map(|r| r.value.id)
            .collect::<Vec<_>>();
        let tail = upkeep::Tail {
            influence: stock::turn::InfluenceInputs {
                definitions: self.influence.definitions,
                capabilities: self.influence.capabilities,
                visibility: self.influence.visibility,
                movement: self.influence.movement,
                formation: self.influence.formation,
                cities: self.influence.cities,
            },
            human: human.inputs,
            scout: human.scout,
            global_ai_initialized: human.initialized,
        };
        let npc = self.npc;
        let inactive_inputs = self.inactive;
        let maintenance_definitions = self.maintenance;
        let visibility = self.influence.visibility;
        let research_definitions = self.research_definitions;
        let tech = self.choices;
        let culture_inputs = self.culture;
        let capabilities = self.influence.capabilities;
        let levels = self.social_queries.levels;
        let camps = &mut self.camps;
        let mut controller = |state: &mut c6_core::loaded_state::LoadedState,
                              backend: &mut crate::loaded_actions::stock_notifications::Runtime<
            '_,
            c6_core::unit::OperationParameters,
        >,
                              research: &mut crate::research::Runtime,
                              managers: &data::activation::cityless_managers::Runtime,
                              requirements: &mut data::activation::requirement_changes::Runtime,
                              player| {
            use crate::loaded_actions::stock_notifications::{npc_activation, npc_leaves};
            let input = npc.iter().find(|r| r.owner == player).ok_or_else(|| {
                data::activation::upkeep::Error {
                    stage: "NPC controller",
                    cause: "current input absent".into(),
                }
            })?;
            let mut off_turn = inactive_inputs.borrow_mut();
            let off_turn = off_turn
                .iter_mut()
                .find(|r| r.owner == player)
                .ok_or_else(|| data::activation::upkeep::Error {
                    stage: "NPC off-turn owner",
                    cause: format!("{player}"),
                })?;
            if player == state.retained().barbarian.player_ordinal {
                if let Some(camp) = camps.as_mut() {
                    let services = camp.services.take().ok_or_else(|| upkeep::Error {
                        stage: "camp services",
                        cause: "current owner already borrowed".into(),
                    })?;
                    let mut sight = visibility.borrow_mut();
                    let f = &camp.factory;
                    let mut camps = crate::barbarian_camps::stock::Camps {
                        definitions: camp.definitions,
                        inputs: crate::barbarian_camps::queries::Inputs {
                            alive_ids: camp.alive,
                            alive_major_ids: camp.alive_majors,
                            civilization_levels: levels,
                            national_parks: Some(&managers.parks),
                        },
                        placement: camp.placement,
                        factory: crate::barbarian_camp_units::FactoryInputs {
                            definitions: f.definitions,
                            creation: f.creation,
                            resources: f.resources,
                            movement: f.movement,
                            combat: f.combat,
                            costs: f.costs,
                            placement: f.placement,
                            queries: f.queries,
                            runtime: f.runtime,
                            tags: f.tags,
                            build_maps: f.build_maps,
                            clans_active: f.clans_active,
                        },
                        sight: &mut sight,
                        capabilities,
                        parameters: camp.parameters,
                        services,
                    };
                    let result = {
                        let mut leaves = npc_leaves::Leaves::new_with_camps(
                            input,
                            maintenance_definitions,
                            &mut camps,
                        )
                        .with_off_turn(off_turn)
                        .with_managers(managers)
                        .with_requirements(requirements.sets_mut());
                        npc_activation::Context {
                            backend,
                            definitions: research_definitions,
                            research,
                            choices: tech,
                            culture: Some(culture_inputs),
                            leaves: &mut leaves,
                        }
                        .run(state, game_update, player, true)
                    };
                    camp.services = Some(camps.services);
                    return result.map_err(|e| data::activation::upkeep::Error {
                        stage: "NPC controller",
                        cause: format!("{e:?}"),
                    });
                }
            }
            let sight = visibility.borrow();
            let mut leaves = npc_leaves::Leaves::new(input, maintenance_definitions, &sight)
                .with_off_turn(off_turn)
                .with_managers(managers);
            npc_activation::Context {
                backend,
                definitions: research_definitions,
                research,
                choices: tech,
                culture: Some(culture_inputs),
                leaves: &mut leaves,
            }
            .run(state, game_update, player, true)
            .map_err(|e| data::activation::upkeep::Error {
                stage: "NPC controller",
                cause: format!("{e:?}"),
            })
        };
        let city_order = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or("current City owner")?
            .cities
            .records
            .entries
            .iter()
            .map(|c| c.value.header.id)
            .collect::<Vec<_>>();
        let mut activate_city =
            |state: &mut c6_core::loaded_state::LoadedState,
             backend: &mut crate::loaded_actions::stock_notifications::Runtime<
                '_,
                c6_core::unit::OperationParameters,
            >,
             requirements: &mut data::activation::requirement_changes::Runtime,
             research_runtime: &mut crate::research::Runtime,
             player,
             city| {
                use crate::city_turn::{self, current_upkeep};
                use crate::loaded_actions::stock_notifications::city_production;
                let q = self
                    .city
                    .as_ref()
                    .ok_or(city_turn::Error::Unsupported("current City query owners"))?;
                let current = q.current;
                backend
                    .with_modifier_requirement_sets(requirements.sets_mut(), |backend| {
                        let kind = production_dispatch::Kind::for_order(
                            state
                                .city(player, city)
                                .map_err(|_| {
                                    city_turn::Error::Unsupported("current City production owner")
                                })?
                                .build_queue
                                .entries
                                .first()
                                .map(|e| e.order),
                        )
                        .map_err(city_turn::Error::Production)?;
                        // Borrow the actual shared sight owner only on the Train path.
                        // A conflicting loan is an explicit incomplete transition.
                        let mut sight = kind
                            .borrow_sight(self.influence.visibility)
                            .map_err(city_turn::Error::Production)?;
                        let mut production = match kind {
                            production_dispatch::Kind::Unit => {
                                let input = self.unit_production.as_ref().ok_or(
                                    city_turn::Error::Unsupported(
                                        "current Unit production services",
                                    ),
                                )?;
                                production_dispatch::Effects::Unit(city_production::unit::Effects {
                                    selection: city_production::unit::Selection {
                                        runtime: backend,
                                        definitions: input.selection,
                                        current:
                                            crate::city_production_turn::unit_selection::Current {
                                                build_maps: input.current.build_maps,
                                                fallout_turns: input.current.fallout_turns,
                                                civilization_levels: input
                                                    .current
                                                    .civilization_levels,
                                            },
                                        reports: vec![],
                                    },
                                    factory: city_production::unit::Factory {
                                        definitions: input.definitions,
                                        resources: input.resources,
                                        movement: self.movement,
                                        combat: input.combat,
                                        costs: input.costs,
                                        placement: input.placement,
                                        queries: input.queries,
                                        runtime: input.runtime,
                                        tags: input.tags,
                                        sight: sight.as_deref_mut().expect("Train sight loan"),
                                        finish: input.finish,
                                    },
                                })
                            }
                            production_dispatch::Kind::District => {
                                let district =
                                    q.district.as_ref().ok_or(city_turn::Error::Unsupported(
                                        "current district production services",
                                    ))?;
                                production_dispatch::Effects::District(
                                    crate::district_actions::stock::Effects {
                                        backend,
                                        research_runtime,
                                        inputs: district,
                                        gossip_history: &history.history,
                                        reports: vec![],
                                        workers: vec![],
                                    },
                                )
                            }
                            production_dispatch::Kind::Building => {
                                production_dispatch::Effects::Building(
                                    city_production::Effects::new(
                                        backend,
                                        &q.definitions.buildings,
                                        q.appeal,
                                        city_production::Scope::StockPlainHeadlessNoAddedConsumers,
                                    ),
                                )
                            }
                        };
                        let outcome = {
                            let mut effects = current_upkeep::Context {
                                production: &mut production,
                                definitions: q.definitions,
                                current,
                                housing: q.housing,
                                configuration: q.configuration,
                                culture: q.culture,
                                loyalty: q.loyalty,
                                power: q.power,
                                combat: q.combat,
                                workers: q.workers,
                                buyable: q.buyable,
                                appeal: q.appeal,
                                player_properties: q.properties,
                                loyalty_changed_subscribers: q.loyalty_subscribers,
                                alive: q.alive.to_vec(),
                                levels: q.levels.to_vec(),
                                fallout: q.fallout.to_vec(),
                                calls: vec![],
                                scope: current_upkeep::Scope::StockPlainHeadlessExternalPolicies,
                            };
                            effects.activate(state, player, city)
                        };
                        if let production_dispatch::Effects::Building(effects) = &mut production {
                            self.city_reports.append(&mut effects.reports);
                        }
                        if let production_dispatch::Effects::Unit(effects) = &mut production {
                            self.unit_production
                                .as_mut()
                                .expect("Train inputs retained")
                                .reports
                                .append(&mut effects.selection.reports);
                        }
                        drop(production);
                        drop(sight);
                        let outcome = outcome?;
                        if matches!(
                            outcome,
                            crate::city_production_turn::Outcome::NeedsProduction
                                | crate::city_production_turn::Outcome::Completed {
                                    needs_production: true
                                }
                        ) {
                            backend
                                .publish_city_production(state, player, city)
                                .map_err(|e| {
                                    backend.last_error =
                                        Some(format!("City production decision: {e:?}"));
                                    city_turn::Error::Unsupported("stock City production decision")
                                })?;
                        }
                        Ok(outcome)
                    })
                    .map_err(|_| {
                        city_turn::Error::Unsupported("current city modifier requirement owner")
                    })?
            };
        let city_services = self
            .city
            .as_ref()
            .map(|_| data::activation::upkeep::CityServices {
                order: &city_order,
                activate: &mut activate_city
                    as &mut data::activation::upkeep::CityActivation<
                        '_,
                        '_,
                        c6_core::unit::OperationParameters,
                    >,
                outcomes: &mut self.city_outcomes,
            });
        let activation_outcome = data::activation::upkeep::Context {
            state,
            backend,
            scoring: &mut self.scoring.borrow_mut(),
            scoring_definitions: self.scoring_definitions,
            research: research_runtime,
            research_definitions,
            choices: tech,
            culture: culture_inputs,
            treasury_definitions: self.treasury,
            deals: self.deals,
            capabilities,
            social_definitions: self.social_definitions,
            social_queries: self.social_queries,
            notification_definitions: self.notifications,
            managers_definitions: self.manager_definitions,
            managers_queries: self.manager_queries,
            managers: &mut self.managers.borrow_mut(),
            maintenance_definitions,
            movement: self.movement,
            formation_definitions: self.influence.formation,
            placement: self.placement,
            phases: self.phases,
            forced_rebase: self.forced_rebase,
            cities: city_services,
            controller: Some(&mut controller),
        }
        .activate(
            owner,
            first,
            &mut signal,
            &tail,
            major,
            &unit_order,
            &formation_order,
        )?;
        Ok(activation_outcome)
    }
    fn inactive(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock::Runtime<'a>,
        owner: i32,
    ) -> Result<()> {
        let mut inputs = self.inactive.borrow_mut();
        let input = inputs
            .iter_mut()
            .find(|r| r.owner == owner)
            .ok_or("current inactive controller input")?;
        if state.player_slot_status(owner)? == 3 {
            let human = self.human(owner)?;
            let units = state
                .world()
                .player(owner)?
                .unit_records()?
                .iter()
                .map(|u| u.value.id)
                .collect::<Vec<_>>();
            let sight = self.influence.visibility.borrow();
            inactive_control::run_human(
                state,
                backend,
                input,
                human_activation::OffTurn {
                    definitions: self.maintenance,
                    inputs: human.inputs,
                    units: &units,
                    major: self.major(owner)?,
                    initialized: human.initialized,
                    scout: human.scout,
                    visibility: &sight,
                },
            )
            .map_err(|e| format!("{e:?}"))?;
        } else {
            inactive_control::run(state, backend, input).map_err(|e| format!("{e:?}"))?;
        }
        Ok(())
    }
    fn active(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock::Runtime<'a>,
        update: &mut loaded::Runtime,
        owner: i32,
    ) -> Result<upkeep::ActivationOutcome> {
        let input = self
            .npc
            .iter()
            .find(|r| r.owner == owner)
            .ok_or("current NPC action input")?;
        let sight = self.influence.visibility.borrow();
        let mut leaves = npc_leaves::Leaves::new(input, self.maintenance, &sight);
        npc_activation::active(state, backend, update, owner, &mut leaves)
            .map_err(|e| format!("{e:?}").into())
    }
    fn return_active(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock::Runtime<'a>,
        update: &mut loaded::Runtime,
        owner: i32,
    ) -> Result<upkeep::ActivationOutcome> {
        let input = self
            .npc
            .iter()
            .find(|r| r.owner == owner)
            .ok_or("current NPC queued-return input")?;
        let sight = self.influence.visibility.borrow();
        let mut leaves = npc_leaves::Leaves::new(input, self.maintenance, &sight);
        npc_activation::finish_active_return(state, backend, update, owner, &mut leaves)
            .map_err(|e| format!("{e:?}").into())
    }
}
