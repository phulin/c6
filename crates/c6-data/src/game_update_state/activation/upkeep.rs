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
struct GoldEffects;
impl treasury_deals::Effects for GoldEffects {
    fn gold_changed(
        &mut self,
        _: &mut LoadedState,
        _: i32,
    ) -> Result<(), c6_core::game_update::Error> {
        Err(c6_core::game_update::Error::Unsupported(
            "activation gold report and GoldChanged signal",
        ))
    }
}
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
    ) -> Result<(), Error> {
        if !first {
            return Err(failure("activation", "repeated activation continuation"));
        }
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
        self.backend
            .update_turn_influence(self.state, owner, &tail.influence)
            .map_err(|e| failure("influence rebuild", e))?;
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
        )
        .map_err(|e| failure("human controller activation", e))?;
        self.backend.processor_activation_complete(self.state,owner,first,
            stock_notifications::turn_reports::Scope::StockPlainHeadlessExternalPoliciesNoAddedConsumers)
            .map_err(|e|failure("processor activation report",e))
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
        if !self
            .state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or_else(|| failure("statistics", "current player"))?
            .cities
            .records
            .entries
            .is_empty()
        {
            return Err(failure("statistics", "current City iteration order"));
        }
        {
            let mut maintenance = stock_notifications::maintenance::Context {
                state: self.state,
                backend: self.backend,
                definitions: self.maintenance_definitions,
            };
            maintenance
                .stats(owner, units, Some(&[]))
                .map_err(|e| failure("statistics", e))?;
            maintenance.eras(owner).map_err(|e| failure("eras", e))?;
        }
        placement_and_city_pass(self.state, owner)?;
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
            },
            self.deals,
            &mut GoldEffects,
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
/// the subsequent City activation pass. Empty collections return at their real
/// source loop boundaries; positive collections require their gameplay bodies.
pub fn placement_and_city_pass(state: &LoadedState, owner: i32) -> Result<(), Error> {
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or_else(|| failure("placement", "current player"))?;
    if !player.districts.records.entries.is_empty() {
        return Err(failure(
            "forced district rebasing",
            "current district callbacks",
        ));
    }
    if !player.improvements.plot_indices.is_empty() {
        return Err(failure(
            "forced improvement rebasing",
            "current improvement callbacks",
        ));
    }
    // ExecuteImprovementCaptures traverses the same current improvement list.
    if !player.cities.records.entries.is_empty() {
        return Err(failure(
            "city activation",
            "current City upkeep and loyalty transfers",
        ));
    }
    Ok(())
}
