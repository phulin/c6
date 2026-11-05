//! Nonhuman activation on the same stock owners, retaining the Core callback
//! cursor across external technology and civic decisions.
use super::{research as stock_research, Runtime};
use crate::{
    game_update_state::activation::upkeep::{ActivationOutcome, PolicyChoice, PolicyKind},
    loaded_actions::choose_tech,
    research::{self, Effects as _},
};
use c6_core::{
    game_update::Error,
    loaded_state::{
        game_update::{self as loaded, npc_activation as core},
        LoadedState,
    },
    relic_creation::boost,
};
pub use core::Kind;

/// Required source leaves; unknown subscribers have no successful default.
pub trait Leaves<P> {
    fn callbacks(&self) -> &[Kind];
    fn research_reached(
        &self,
        state: &LoadedState,
        owner: i32,
        kind: Kind,
        full: bool,
    ) -> Result<bool, Error>;
    fn wait(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        owner: i32,
    ) -> Result<(), Error>;
    fn off_turn(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        owner: i32,
    ) -> Result<bool, Error>;
    fn callback(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        owner: i32,
        kind: Kind,
        full: bool,
    ) -> Result<(), Error>;
    fn research_suffix(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        owner: i32,
        kind: Kind,
    ) -> Result<(), Error>;
    fn report(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        owner: i32,
        event: u32,
    ) -> Result<(), Error>;
}
/// Borrows the existing research, script and reporting owners. Policy requests
/// are applied through turn_loop::resume_research before reentering this frame.
pub struct Context<'s, 'a, P, L> {
    pub backend: &'s mut Runtime<'a, P>,
    pub definitions: &'s research::Definitions,
    pub research: &'s mut research::Runtime,
    pub choices: &'s choose_tech::Definitions,
    pub culture: Option<&'s stock_research::CultureInputs<'s>>,
    pub leaves: &'s mut L,
}
impl<P, L: Leaves<P>> Context<'_, '_, P, L> {
    fn fail(&mut self, cause: impl std::fmt::Debug) -> Error {
        self.backend.last_error = Some(format!("NPC activation: {cause:?}"));
        Error::Unsupported("stock NPC activation; inspect backend.last_error")
    }
    fn selected(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        kind: core::DecisionKind,
    ) -> Result<Option<i32>, Error> {
        let mut effects = stock_research::Effects {
            backend: self.backend,
            runtime: self.research,
            choices: self.choices,
            choice_graph: None,
            culture: self.culture,
        };
        let context = research::Context {
            state,
            definitions: self.definitions,
            effects: &mut effects,
        };
        let result = match kind {
            core::DecisionKind::Technology => context.selected_technology(owner),
            core::DecisionKind::Civic => context.selected_civic(owner),
            core::DecisionKind::PlayerActions => {
                return Err(Error::Unsupported("player action is not research"))
            }
        };
        result.map_err(|e| self.fail(e))
    }
    fn policy(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        kind: Kind,
    ) -> Result<core::CallbackProgress, Error> {
        let mut effects = stock_research::Effects {
            backend: self.backend,
            runtime: self.research,
            choices: self.choices,
            choice_graph: None,
            culture: self.culture,
        };
        let context = research::Context {
            state,
            definitions: self.definitions,
            effects: &mut effects,
        };
        let (decision_kind, result) = match kind {
            Kind::Research => (
                core::DecisionKind::Technology,
                context.technology_policy_boundary(owner),
            ),
            Kind::Culture => (
                core::DecisionKind::Civic,
                context.civic_policy_boundary(owner),
            ),
            _ => return Err(Error::Unsupported("NPC policy callback kind")),
        };
        match result.map_err(|e| self.fail(e))? {
            research::policy::ResearchPolicy::Continue => {
                self.suffix(state, owner, kind)?;
                Ok(core::CallbackProgress::Continue)
            }
            research::policy::ResearchPolicy::ChoicePending {
                player,
                legal_indices,
            } => Ok(core::CallbackProgress::ChoicePending(core::Decision {
                player,
                kind: decision_kind,
                legal_indices,
            })),
        }
    }
    fn suffix(&mut self, state: &mut LoadedState, owner: i32, kind: Kind) -> Result<(), Error> {
        // QueueNextTech's caller publishes this after the policy site, also
        // when an already-populated queue bypassed selection.
        if kind == Kind::Research {
            let mut effects = stock_research::Effects {
                backend: self.backend,
                runtime: self.research,
                choices: self.choices,
                choice_graph: None,
                culture: self.culture,
            };
            effects
                .effect(
                    state,
                    self.definitions,
                    research::Effect::Report(boost::Report {
                        event: 0x48062f97,
                        specialization: None,
                        player: owner,
                        index: Some(-1),
                        old_progress: None,
                        source: None,
                    }),
                )
                .map_err(|e| self.fail(e))?;
        }
        self.leaves
            .research_suffix(state, self.backend, owner, kind)
    }
    pub fn run(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut loaded::Runtime,
        owner: i32,
        full_turn: bool,
    ) -> Result<ActivationOutcome, Error> {
        self.backend.check(state).map_err(|e| self.fail(e))?;
        if core::controller(runtime).is_none() {
            core::begin_controller(
                state,
                runtime,
                owner,
                Vec::new(),
                full_turn,
                self.leaves.callbacks().to_vec(),
            )?;
        }
        if core::controller(runtime).is_none_or(|frame| frame.owner() != owner) {
            return Err(Error::Unsupported("NPC continuation player differs"));
        }
        let progress = if let Some(decision) = core::pending(runtime).cloned() {
            if let Some(index) = self.selected(state, owner, decision.kind)? {
                core::resume_choice(state, runtime, index, self)?
            } else {
                core::advance(state, runtime, self)?
            }
        } else {
            core::advance(state, runtime, self)?
        };
        Ok(match progress {
            core::Progress::PlayerTurn => ActivationOutcome::Complete,
            core::Progress::ChoicePending(decision) => ActivationOutcome::Choice(PolicyChoice {
                player: decision.player,
                kind: match decision.kind {
                    core::DecisionKind::Technology => {
                        PolicyKind::Research(research::Kind::Technology)
                    }
                    core::DecisionKind::Civic => PolicyKind::Research(research::Kind::Civic),
                    core::DecisionKind::PlayerActions => PolicyKind::PlayerActions,
                },
                options: decision.legal_indices,
            }),
        })
    }
}
impl<P, L: Leaves<P>> core::Effects for Context<'_, '_, P, L> {
    fn wait_for_jobs(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Error> {
        self.leaves.wait(state, self.backend, owner)
    }
    fn off_turn(&mut self, state: &mut LoadedState, owner: i32) -> Result<bool, Error> {
        self.leaves.off_turn(state, self.backend, owner)
    }
    fn callback(
        &mut self,
        state: &mut LoadedState,
        _: &mut loaded::Runtime,
        owner: i32,
        kind: Kind,
        full: bool,
    ) -> Result<core::CallbackProgress, Error> {
        self.leaves
            .callback(state, self.backend, owner, kind, full)?;
        if matches!(kind, Kind::Research | Kind::Culture)
            && self.leaves.research_reached(state, owner, kind, full)?
        {
            self.policy(state, owner, kind)
        } else {
            Ok(core::CallbackProgress::Continue)
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn resume_choice(
        &mut self,
        state: &mut LoadedState,
        _: &mut loaded::Runtime,
        owner: i32,
        kind: Kind,
        _: bool,
        decision: &core::Decision,
        index: i32,
    ) -> Result<core::CallbackProgress, Error> {
        if self.selected(state, owner, decision.kind)? != Some(index) {
            return Err(Error::Unsupported("NPC policy request was not applied"));
        }
        self.suffix(state, owner, kind)?;
        Ok(core::CallbackProgress::Continue)
    }
    fn report(&mut self, state: &mut LoadedState, owner: i32, event: u32) -> Result<(), Error> {
        self.leaves.report(state, self.backend, owner, event)
    }
}
