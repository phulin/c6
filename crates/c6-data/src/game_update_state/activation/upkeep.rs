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
}
impl<P> Context<'_, '_, P> {
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
