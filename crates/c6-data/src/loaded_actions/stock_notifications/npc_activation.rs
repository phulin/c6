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

/// Highest ordinary action boundary after nonhuman activation. Research
/// indices belong only to research choices; ordinary actions use their own
/// public legality APIs, including turn_completion for EndTurn.
pub fn player_actions(
    state: &LoadedState,
    runtime: &loaded::Runtime,
    owner: i32,
) -> Result<ActivationOutcome, Error> {
    if runtime.failed()
        || runtime.pending_player_effect()
            != Some(c6_core::game_update::PlayerEffect::ActiveControl { player: owner })
        || core::controller(runtime).is_none_or(|frame| frame.owner() != owner)
        || core::pending(runtime).is_some()
        || !state
            .turn_active(owner)
            .map_err(|_| Error::Unsupported("NPC current activity"))?
        || state
            .player_slot_status(owner)
            .map_err(|_| Error::Unsupported("NPC current role"))?
            == 3
    {
        return Err(Error::Unsupported("NPC player-action continuation differs"));
    }
    Ok(ActivationOutcome::Choice(PolicyChoice {
        player: owner,
        kind: PolicyKind::PlayerActions,
        options: Vec::new(),
    }))
}

/// Ordinary active-control entry using the current completion scope. The
/// same leaf owner joins outstanding synchronous work before returning from
/// a nested update, without replaying activation or starting another action.
pub fn active<P>(
    state: &mut LoadedState,
    backend: &mut Runtime<'_, P>,
    runtime: &mut loaded::Runtime,
    owner: i32,
    leaves: &mut impl Leaves<P>,
) -> Result<ActivationOutcome, Error> {
    if backend.completion_inflight == Some(owner) {
        finish_active_return(state, backend, runtime, owner, leaves)
    } else {
        player_actions(state, runtime, owner)
    }
}

/// Finish only the current control return after an external message was queued.
/// The queued completion itself is drained by the outer message continuation.
pub fn finish_active_return<P>(
    state: &mut LoadedState,
    backend: &mut Runtime<'_, P>,
    runtime: &mut loaded::Runtime,
    owner: i32,
    leaves: &mut impl Leaves<P>,
) -> Result<ActivationOutcome, Error> {
    player_actions(state, runtime, owner)?;
    leaves.wait(state, backend, owner)?;
    Ok(ActivationOutcome::Complete)
}

/// Required source leaves; unknown subscribers have no successful default.
pub trait Leaves<P> {
    fn callbacks(&self) -> &[Kind];
    fn barbarian_reached(
        &self,
        _state: &LoadedState,
        _owner: i32,
        _full: bool,
    ) -> Result<bool, Error> {
        Err(Error::Unsupported("NPC current Barbarian callback binding"))
    }
    fn barbarian_after_research(
        &mut self,
        _state: &mut LoadedState,
        _backend: &mut Runtime<'_, P>,
        _runtime: &mut loaded::Runtime,
        _owner: i32,
    ) -> Result<(), Error> {
        Err(Error::Unsupported("NPC Barbarian camp activation"))
    }
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
    /// Nested updates inside an already-entered completion return through the
    /// ordinary wait suffix; they do not enqueue another EndTurn command.
    pub fn active(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut loaded::Runtime,
        owner: i32,
    ) -> Result<ActivationOutcome, Error> {
        active(state, self.backend, runtime, owner, self.leaves)
    }
    /// Return from the already-reached active-control decision after enqueueing
    /// an external action. This does not execute that action or decrement the
    /// activation processing depth; ordinary deactivation owns that decrement.
    pub fn finish_active_return(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut loaded::Runtime,
        owner: i32,
    ) -> Result<ActivationOutcome, Error> {
        if runtime.failed()
            || runtime.pending_player_effect()
                != Some(c6_core::game_update::PlayerEffect::ActiveControl { player: owner })
            || core::controller(runtime).is_none_or(|frame| frame.owner() != owner)
            || core::pending(runtime).is_some()
        {
            return Err(Error::Unsupported("NPC active return continuation differs"));
        }
        self.leaves.wait(state, self.backend, owner)?;
        Ok(ActivationOutcome::Complete)
    }
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
        if kind == Kind::Culture {
            let level = self
                .research
                .civilization_levels
                .iter()
                .find(|v| v.0 == owner)
                .ok_or(Error::Unsupported("NPC civic current civilization level"))?
                .1;
            if level == 0xd959_ec24 {
                self.suffix(state, owner, kind)?;
                return Ok(core::CallbackProgress::Continue);
            }
        }

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
            } => {
                if decision_kind == core::DecisionKind::Technology
                    && self
                        .backend
                        .deferred_research_response(state, player, &legal_indices)
                        .map_err(|e| self.fail(e))?
                        == research::policy::AcquisitionResponse::Defer
                {
                    self.suffix(state, owner, kind)?;
                    return Ok(core::CallbackProgress::Continue);
                }
                Ok(core::CallbackProgress::ChoicePending(core::Decision {
                    player,
                    kind: decision_kind,
                    legal_indices,
                }))
            }
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
        runtime: &mut loaded::Runtime,
        owner: i32,
        kind: Kind,
        full: bool,
    ) -> Result<core::CallbackProgress, Error> {
        self.leaves
            .callback(state, self.backend, owner, kind, full)?;
        if kind == Kind::Barbarian && self.leaves.barbarian_reached(state, owner, full)? {
            let mut effects = stock_research::Effects {
                backend: self.backend,
                runtime: self.research,
                choices: self.choices,
                choice_graph: None,
                culture: self.culture,
            };
            research::Context {
                state,
                definitions: self.definitions,
                effects: &mut effects,
            }
            .acquire_barbarian_research(owner)
            .map_err(|error| self.fail(error))?;
            self.leaves
                .barbarian_after_research(state, self.backend, runtime, owner)?;
            return Ok(core::CallbackProgress::Continue);
        }
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
