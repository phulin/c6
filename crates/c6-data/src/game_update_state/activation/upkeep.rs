//! Ordered first-activation computation after the TurnStarted signal.
//! A failed stage preserves its completed prefix; it never completes Game.Update.
use super::treasury_deals;
use crate::{
    loaded_actions::{
        choose_tech,
        stock_notifications::{self, research::CultureInputs, scoring},
    },
    research,
};
use c6_core::{game_capabilities::Membership, loaded_state::LoadedState};

#[derive(Debug)]
pub struct Error {
    pub stage: &'static str,
    pub cause: String,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.stage, self.cause)
    }
}
impl std::error::Error for Error {}
fn failure(stage: &'static str, cause: impl std::fmt::Debug) -> Error {
    Error {
        stage,
        cause: format!("{cause:?}"),
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyKind {
    Research(research::Kind),
    PlayerActions,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyChoice {
    pub player: i32,
    pub kind: PolicyKind,
    pub options: Vec<i32>,
}
#[derive(Debug, Eq, PartialEq)]
pub enum ActivationOutcome {
    Complete,
    Choice(PolicyChoice),
}
#[derive(Debug)]
pub(crate) struct ActivationProgress {
    player: i32,
    first: bool,
}
fn prefix_once(
    progress: &mut Option<ActivationProgress>,
    player: i32,
    first: bool,
    prefix: impl FnOnce() -> Result<(), Error>,
) -> Result<(), Error> {
    if let Some(current) = progress {
        if current.player != player || current.first != first {
            return Err(failure(
                "activation continuation",
                "current player/activation differs",
            ));
        }
        return Ok(());
    }
    prefix()?;
    *progress = Some(ActivationProgress { player, first });
    Ok(())
}
pub type Controller<'s, 'a, P> = dyn FnMut(
        &mut LoadedState,
        &mut stock_notifications::Runtime<'a, P>,
        &mut research::Runtime,
        i32,
    ) -> Result<ActivationOutcome, Error>
    + 's;
/// All runtimes belong to the same current world. Callers retain these owners
/// across turns and supply current graphs lazily through the existing services.
pub struct Context<'s, 'a, P> {
    pub state: &'s mut LoadedState,
    pub backend: &'s mut stock_notifications::Runtime<'a, P>,
    pub scoring: &'s mut scoring::Runtime,
    pub scoring_definitions: &'s scoring::Definitions,
    pub research: &'s mut research::Runtime,
    pub research_definitions: &'s research::Definitions,
    pub choices: &'s choose_tech::Definitions,
    pub culture: &'s CultureInputs<'s>,
    pub treasury_definitions: &'s treasury_deals::Definitions,
    pub deals: &'s mut treasury_deals::CurrentDeals,
    pub capabilities: &'s Membership,
    pub social_definitions: &'s super::social::Definitions,
    pub social_queries: &'s super::social::Queries<'s>,
    pub notification_definitions: &'s crate::loaded_actions::notifications::Definitions,
    pub managers_definitions: &'s super::cityless_managers::Definitions,
    pub managers_queries: &'s super::cityless_managers::Queries<'s>,
    pub managers: &'s mut super::cityless_managers::Runtime,
    pub maintenance_definitions: &'s stock_notifications::maintenance::Definitions,
    pub movement: &'s crate::unit_movement_state::Inputs<'s>,
    pub formation_definitions: &'s crate::formation_rules::Definitions,
    pub placement: &'s dyn stock_notifications::movement::PlacementQueries,
    pub phases: &'s super::Phases,
    pub forced_rebase: &'s super::forced_rebase::Definitions,
    pub cities: Option<CityServices<'s, 'a, P>>,
    pub controller: Option<&'s mut Controller<'s, 'a, P>>,
}
/// The current source iteration order and concrete public city implementation.
/// The callback binds current owners and calls current_upkeep::Context::activate;
/// production outcomes remain visible while the remaining player upkeep runs.
pub type CityActivation<'s, 'a, P> = dyn FnMut(
        &mut LoadedState,
        &mut stock_notifications::Runtime<'a, P>,
        i32,
        u32,
    ) -> Result<crate::city_production_turn::Outcome, crate::city_turn::Error>
    + 's;
pub struct CityServices<'s, 'a, P> {
    pub order: &'s [u32],
    pub activate: &'s mut CityActivation<'s, 'a, P>,
    pub outcomes: &'s mut Vec<(i32, u32, crate::city_production_turn::Outcome)>,
}
pub struct StartSignal<'a> {
    pub requirements: &'a mut super::requirement_changes::Runtime,
    pub listeners: &'a [super::turn_started::Listener],
    pub relationships: &'a super::relationship_requirements::Definitions,
    pub victory_definitions: &'a super::victory_requirements::Definitions,
    pub victory: &'a super::victory_requirements::Inputs,
}
pub struct Tail<'a, 'r> {
    pub influence: stock_notifications::turn::InfluenceInputs<'a, 'r>,
    pub human: &'a stock_notifications::human_activation::Inputs,
    pub scout: &'a super::maintenance::ScoutInputs,
    pub global_ai_initialized: bool,
}
impl<P> Context<'_, '_, P> {
    /// Complete TurnManager.DoTurnActivate and its Processor report on the
    /// same state/services, in source order. This bounded implementation admits
    /// first human activation; repeated/nonhuman continuations remain explicit.
    #[allow(clippy::too_many_arguments)]
    pub fn activate(
        &mut self,
        owner: i32,
        first: bool,
        signal: &mut StartSignal<'_>,
        tail: &Tail<'_, '_>,
        major: bool,
        units: &[u32],
        formations: &[u32],
    ) -> Result<ActivationOutcome, Error> {
        self.backend
            .check(self.state)
            .map_err(|e| failure("activation runtime", e))?;
        let mut progress = self.backend.activation_progress.take();
        let prefix_result = prefix_once(&mut progress, owner, first, || {
            if first {
                super::turn_started::dispatch(
                    self.state,
                    self.backend,
                    signal.requirements,
                    super::turn_started::Inputs {
                        scope: super::turn_started::Scope::StockHeadlessExternalPlayerPolicies,
                        listeners: signal.listeners,
                        relationship_definitions: signal.relationships,
                        alive_majors: self
                            .research
                            .alive_majors
                            .as_deref()
                            .ok_or_else(|| failure("turn started", "current major order"))?,
                        victory: Some(super::turn_started::VictoryInputs {
                            definitions: signal.victory_definitions,
                            current: signal.victory,
                            scoring: self.scoring,
                            scoring_definitions: self.scoring_definitions,
                        }),
                    },
                    owner,
                )
                .map_err(|e| failure("turn-start signal", e))?;
                self.through_activation_signals(owner, major, units, formations)?;
            } else {
                super::cityless_managers::on_congress_turn_started(
                    self.state,
                    self.managers_definitions,
                    self.managers_queries,
                    self.managers,
                    self.phases,
                    self.backend,
                    owner,
                )
                .map_err(|e| failure("turn activation Congress signal", e))?;
            }
            self.backend
                .update_turn_influence(self.state, owner, &tail.influence)
                .map_err(|e| failure("influence rebuild", e))?;
            Ok(())
        });
        self.backend.activation_progress = progress;
        prefix_result?;
        if self
            .state
            .player_slot_status(owner)
            .map_err(|e| failure("activation role", e))?
            != 3
        {
            let controller = self
                .controller
                .as_mut()
                .ok_or_else(|| failure("controller activation", "current nonhuman controller"))?;
            match controller(self.state, self.backend, self.research, owner)? {
                ActivationOutcome::Choice(choice) => {
                    if choice.player != owner {
                        return Err(failure("controller decision", "different current player"));
                    }
                    return Ok(ActivationOutcome::Choice(choice));
                }
                ActivationOutcome::Complete => {}
            }
        } else {
            stock_notifications::human_activation::run(
                self.state,
                self.backend,
                self.maintenance_definitions,
                tail.human,
                units,
                major,
                tail.global_ai_initialized,
                tail.scout,
                true,
                tail.influence.visibility,
            )
            .map_err(|e| failure("human controller activation", e))?;
        }
        self.backend.processor_activation_complete(self.state,owner,first,
            stock_notifications::turn_reports::Scope::StockPlainHeadlessExternalPoliciesNoAddedConsumers)
            .map_err(|e|failure("processor activation report",e))?;
        self.backend.activation_progress = None;
        Ok(ActivationOutcome::Complete)
    }
    /// Complete the first-activation managers through movement restoration.
    /// InfluenceMap and controller activation remain mandatory.
    pub fn through_activation_signals(
        &mut self,
        owner: i32,
        major: bool,
        units: &[u32],
        formations: &[u32],
    ) -> Result<(), Error> {
        self.through_social(owner)?;
        super::cityless_managers::before_units(
            self.state,
            self.managers_definitions,
            self.managers_queries,
            self.managers,
            self.backend,
            owner,
        )
        .map_err(|e| failure("diplomacy and player managers", e))?;
        stock_notifications::maintenance::Context {
            state: self.state,
            backend: self.backend,
            definitions: self.maintenance_definitions,
        }
        .units(owner, units, major)
        .map_err(|e| failure("unit resource activation", e))?;
        super::cityless_managers::after_units(self.state, self.managers_definitions, owner)
            .map_err(|e| failure("governors and trade", e))?;
        let player = self
            .state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or_else(|| failure("statistics", "current player"))?;
        let city_order = if player.cities.records.entries.is_empty() {
            vec![]
        } else {
            let current = self
                .cities
                .as_ref()
                .ok_or_else(|| failure("statistics", "current City iteration order"))?;
            let order = current.order.to_vec();
            if order.len() != player.cities.records.entries.len()
                || order.iter().enumerate().any(|(i, id)| {
                    order[..i].contains(id)
                        || !player
                            .cities
                            .records
                            .entries
                            .iter()
                            .any(|c| c.value.header.id == *id)
                })
            {
                return Err(failure("statistics", "current City iteration membership"));
            }
            order
        };
        {
            let mut maintenance = stock_notifications::maintenance::Context {
                state: self.state,
                backend: self.backend,
                definitions: self.maintenance_definitions,
            };
            maintenance
                .stats(owner, units, Some(&city_order))
                .map_err(|e| failure("statistics", e))?;
            maintenance.eras(owner).map_err(|e| failure("eras", e))?;
        }
        placement_prelude(self.state, self.forced_rebase, owner)?;
        for city in city_order {
            let services = self
                .cities
                .as_mut()
                .ok_or_else(|| failure("city activation", "current city services"))?;
            let outcome = (services.activate)(self.state, self.backend, owner, city)
                .map_err(|e| failure("city activation", e))?;
            services.outcomes.push((owner, city, outcome));
        }
        super::cityless_managers::after_cities(
            self.state,
            self.managers_definitions,
            self.backend,
            owner,
        )
        .map_err(|e| failure("resource maintenance and notifications", e))?;
        self.backend
            .restore_player_movement(
                self.state,
                owner,
                units,
                formations,
                self.movement,
                self.formation_definitions,
                self.placement,
            )
            .map_err(|e| failure("unit and formation restoration", e))?;
        self.backend.player_initialization_complete(self.state,owner,
            stock_notifications::turn_reports::Scope::StockPlainHeadlessExternalPoliciesNoAddedConsumers)
            .map_err(|e|failure("initialization-complete signal",e))?;
        super::cityless_managers::on_congress_turn_started(
            self.state,
            self.managers_definitions,
            self.managers_queries,
            self.managers,
            self.phases,
            self.backend,
            owner,
        )
        .map_err(|e| failure("turn activation Congress signal", e))
    }
    /// The next ordered group is Religion, Great People, Influence and Quests.
    pub fn through_social(&mut self, owner: i32) -> Result<(), Error> {
        self.through_culture(owner)?;
        stock_notifications::social::run(
            self.state,
            self.backend,
            self.social_definitions,
            self.social_queries,
            self.notification_definitions,
            c6_core::player_notifications_owner::OccurrenceLimits::default(),
            owner,
        )
        .map_err(|e| failure("religion, great people, influence and quests", e))
    }
    /// Native order: score, research, tax, deals, then culture. Later activation
    /// subsystems must still execute before the player is ready for decisions.
    pub fn through_culture(&mut self, owner: i32) -> Result<(), Error> {
        self.scoring
            .calculate(self.state, self.scoring_definitions, owner, false)
            .map_err(|e| failure("score", e))?;
        {
            let mut effects = stock_notifications::research::Effects {
                backend: self.backend,
                runtime: self.research,
                choices: self.choices,
                choice_graph: None,
                culture: Some(self.culture),
            };
            research::Context {
                state: self.state,
                definitions: self.research_definitions,
                effects: &mut effects,
            }
            .do_research(owner)
            .map_err(|e| failure("research", e))?;
        }
        treasury_deals::run(
            self.state,
            self.treasury_definitions,
            &treasury_deals::Queries {
                capabilities: self.capabilities,
                cities: self.backend.city_query_inputs(),
                city_order: self.cities.as_ref().map(|c| c.order),
            },
            self.deals,
            self.backend,
            owner,
        )
        .map_err(|e| failure("treasury and deals", e))?;
        let mut effects = stock_notifications::research::Effects {
            backend: self.backend,
            runtime: self.research,
            choices: self.choices,
            choice_graph: None,
            culture: Some(self.culture),
        };
        research::Context {
            state: self.state,
            definitions: self.research_definitions,
            effects: &mut effects,
        }
        .do_culture(owner)
        .map_err(|e| failure("culture", e))
    }
}

/// Stock Placement virtual +0x78 (forced rebasing), improvement captures and
/// the subsequent City activation pass are separate. Current districts are
/// checked for real air capacity before entering aircraft continuations.
pub fn placement_prelude(
    state: &LoadedState,
    definitions: &super::forced_rebase::Definitions,
    owner: i32,
) -> Result<(), Error> {
    super::forced_rebase::check_districts(state, definitions, owner)
        .map_err(|e| failure("forced district rebasing", e))?;
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or_else(|| failure("placement", "current player"))?;
    if !player.improvements.plot_indices.is_empty() {
        return Err(failure(
            "forced improvement rebasing",
            "current improvement callbacks",
        ));
    }
    // ExecuteImprovementCaptures traverses the same current improvement list.
    Ok(())
}

#[cfg(test)]
mod continuation_tests {
    use super::*;
    #[test]
    fn external_choice_resume_preserves_upkeep_and_rng_prefix() {
        let mut checkpoint = None;
        let mut balance = 0i32;
        let mut random = c6_core::rng::RandomState::new(17);
        prefix_once(&mut checkpoint, 62, true, || {
            balance = balance.wrapping_add(100);
            random.draw(19);
            Ok(())
        })
        .unwrap();
        let after_upkeep = random;
        // A validated external choice changes current state between calls.
        balance = balance.wrapping_sub(7);
        prefix_once(&mut checkpoint, 62, true, || {
            balance = balance.wrapping_add(100);
            random.draw(19);
            Ok(())
        })
        .unwrap();
        assert_eq!(balance, 93);
        assert_eq!(random, after_upkeep);
        assert_eq!(random.calls, 1);
        assert!(prefix_once(&mut checkpoint, 63, true, || {
            balance = 0;
            Ok(())
        })
        .is_err());
        assert_eq!(balance, 93);
    }
}
