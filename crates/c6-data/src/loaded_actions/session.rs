//! Persistent owner for the supported stock action and turn paths.
//!
//! Initialization supplies typed state and current runtime facts once. Each
//! request mutates the same state, script/RNG, notifications and turn cursor.
//! This bounded surface does not implement arbitrary Civ VI actions or turns.
use super::{
    current_aftermath, handlers,
    stock_notifications::{self, deactivation, turn_loop},
    Inputs,
};
use crate::game_update_state::{
    self as data,
    activation::upkeep::{ActivationOutcome, PolicyChoice, PolicyKind},
};
use c6_core::{
    loaded_state::{game_update as loaded, LoadedState},
    map::UnitReference,
};

#[derive(Debug)]
pub enum Error<E> {
    /// A previous request retained a partial prefix. Inspect the session rather
    /// than issuing another request against an incompletely advanced world.
    Failed(String),
    /// Resolve the pending research choice before issuing a unit order.
    PendingResearch,
    Action(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "persistent stock session: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}

/// Borrows the canonical LoadedState through its existing aftermath context;
/// owns its current game-update/research runtimes and visibility history.
/// No world snapshot, reload or replacement backend is created per action.
pub struct Session<'s, 'a, P = c6_core::unit::OperationParameters> {
    context: current_aftermath::Context<'s, stock_notifications::Runtime<'a, P>>,
    update: loaded::Runtime,
    research: crate::research::Runtime,
    visibility: data::deactivation::VisibilityHistory,
    failed: Option<String>,
}
impl<'s, 'a, P> Session<'s, 'a, P> {
    pub fn new(
        context: current_aftermath::Context<'s, stock_notifications::Runtime<'a, P>>,
        update: loaded::Runtime,
        research: crate::research::Runtime,
        visibility: data::deactivation::VisibilityHistory,
    ) -> Self {
        let failed = if update.failed() {
            Some("Game.Update already failed".to_owned())
        } else if let Some(stage) = context.failed_at() {
            Some(format!("action aftermath already failed at {stage:?}"))
        } else {
            context
                .notification_failure()
                .map(|error| format!("notification continuation already failed: {error:?}"))
        };
        Self {
            context,
            update,
            research,
            visibility,
            failed,
        }
    }
    pub fn state(&self) -> &LoadedState {
        self.context.notifications.state
    }
    pub fn context(&self) -> &current_aftermath::Context<'s, stock_notifications::Runtime<'a, P>> {
        &self.context
    }
    pub fn pending_choice(&self) -> Option<&PolicyChoice> {
        self.context.notifications.backend.pending_policy_choice()
    }
    pub fn failure(&self) -> Option<&str> {
        self.failed.as_deref()
    }
    /// Recover every owner, including a failed prefix, without rebuilding state.
    pub fn into_parts(
        self,
    ) -> (
        current_aftermath::Context<'s, stock_notifications::Runtime<'a, P>>,
        loaded::Runtime,
        crate::research::Runtime,
        data::deactivation::VisibilityHistory,
    ) {
        (self.context, self.update, self.research, self.visibility)
    }
    fn check<E>(&self) -> Result<(), Error<E>> {
        match &self.failed {
            Some(failure) => Err(Error::Failed(failure.clone())),
            None => Ok(()),
        }
    }
    fn retain<T, E: std::fmt::Debug>(
        &mut self,
        result: Result<T, E>,
        partial_failure: bool,
    ) -> Result<T, Error<E>> {
        result.map_err(|error| {
            if partial_failure {
                self.failed = Some(format!("{error:?}"));
            }
            Error::Action(error)
        })
    }
    /// Issue one explicit unit order. Research choices must be resolved first;
    /// a suspended player-action callback remains in place across unit orders.
    pub fn skip_turn(
        &mut self,
        issuer: i32,
        actor: UnitReference,
        inputs: &Inputs<'_, '_>,
        handlers: &handlers::BoundHandlers<'_>,
        budget: &mut usize,
    ) -> Result<stock_notifications::skip_turn::Outcome, Error<stock_notifications::skip_turn::Error>>
    {
        self.check()?;
        if matches!(
            self.pending_choice().map(|choice| choice.kind),
            Some(PolicyKind::Research(_))
        ) {
            return Err(Error::PendingResearch);
        }
        let result = stock_notifications::skip_turn::execute(
            &mut self.context,
            issuer,
            actor,
            inputs,
            handlers,
            budget,
        );
        let partial_failure = matches!(
            result,
            Err(stock_notifications::skip_turn::Error::Manager(_)
                | stock_notifications::skip_turn::Error::Aftermath(_))
        );
        self.retain(result, partial_failure)
    }
    /// Uses the existing stock continuation; callback services must retain their
    /// initialized gameplay owners across requests. Choices are never selected here.
    #[allow(clippy::too_many_arguments)]
    pub fn end_turn<E: std::fmt::Debug>(
        &mut self,
        inputs: &deactivation::Inputs<'_, '_, '_>,
        player: i32,
        reason: u32,
        budget: &mut usize,
        activation: impl FnMut(
            &mut LoadedState,
            &mut stock_notifications::Runtime<'a, P>,
            &mut loaded::Runtime,
            &mut crate::research::Runtime,
            i32,
            bool,
        ) -> Result<ActivationOutcome, E>,
        inactive: impl FnMut(
            &mut LoadedState,
            &mut stock_notifications::Runtime<'a, P>,
            i32,
        ) -> Result<(), E>,
        active: impl FnMut(
            &mut LoadedState,
            &mut stock_notifications::Runtime<'a, P>,
            &mut loaded::Runtime,
            i32,
        ) -> Result<ActivationOutcome, E>,
    ) -> Result<turn_loop::Outcome, Error<turn_loop::Error<E>>> {
        self.check()?;
        let result = turn_loop::execute(
            &mut self.context,
            &mut self.update,
            &mut self.research,
            inputs,
            &mut self.visibility,
            player,
            reason,
            budget,
            activation,
            inactive,
            active,
        );
        let partial_failure = !matches!(result, Err(turn_loop::Error::Policy(_)));
        self.retain(result, partial_failure)
    }
    /// Uses the existing stock continuation; callback services must retain their
    /// initialized gameplay owners across requests. Choices are never selected here.
    #[allow(clippy::too_many_arguments)]
    pub fn resume_end_turn<E: std::fmt::Debug>(
        &mut self,
        inputs: &deactivation::Inputs<'_, '_, '_>,
        budget: &mut usize,
        activation: impl FnMut(
            &mut LoadedState,
            &mut stock_notifications::Runtime<'a, P>,
            &mut loaded::Runtime,
            &mut crate::research::Runtime,
            i32,
            bool,
        ) -> Result<ActivationOutcome, E>,
        inactive: impl FnMut(
            &mut LoadedState,
            &mut stock_notifications::Runtime<'a, P>,
            i32,
        ) -> Result<(), E>,
        active: impl FnMut(
            &mut LoadedState,
            &mut stock_notifications::Runtime<'a, P>,
            &mut loaded::Runtime,
            i32,
        ) -> Result<ActivationOutcome, E>,
        return_active: impl FnMut(
            &mut LoadedState,
            &mut stock_notifications::Runtime<'a, P>,
            &mut loaded::Runtime,
            i32,
        ) -> Result<ActivationOutcome, E>,
    ) -> Result<turn_loop::Outcome, Error<turn_loop::Error<E>>> {
        self.check()?;
        let result = turn_loop::resume_end_turn(
            &mut self.context,
            &mut self.update,
            &mut self.research,
            inputs,
            &mut self.visibility,
            budget,
            activation,
            inactive,
            active,
            return_active,
        );
        let partial_failure = !matches!(result, Err(turn_loop::Error::Policy(_)));
        self.retain(result, partial_failure)
    }
    /// Uses the existing stock continuation; callback services must retain their
    /// initialized gameplay owners across requests. Choices are never selected here.
    #[allow(clippy::too_many_arguments)]
    pub fn resume_research<E: std::fmt::Debug>(
        &mut self,
        inputs: &deactivation::Inputs<'_, '_, '_>,
        budget: &mut usize,
        definitions: &crate::research::Definitions,
        choices: &crate::loaded_actions::choose_tech::Definitions,
        culture: Option<&stock_notifications::research::CultureInputs<'_>>,
        index: i32,
        activation: impl FnMut(
            &mut LoadedState,
            &mut stock_notifications::Runtime<'a, P>,
            &mut loaded::Runtime,
            &mut crate::research::Runtime,
            i32,
            bool,
        ) -> Result<ActivationOutcome, E>,
        inactive: impl FnMut(
            &mut LoadedState,
            &mut stock_notifications::Runtime<'a, P>,
            i32,
        ) -> Result<(), E>,
        active: impl FnMut(
            &mut LoadedState,
            &mut stock_notifications::Runtime<'a, P>,
            &mut loaded::Runtime,
            i32,
        ) -> Result<ActivationOutcome, E>,
    ) -> Result<turn_loop::Outcome, Error<turn_loop::Error<E>>> {
        self.check()?;
        let result = turn_loop::resume_research(
            &mut self.context,
            &mut self.update,
            inputs,
            &mut self.visibility,
            budget,
            definitions,
            &mut self.research,
            choices,
            culture,
            index,
            activation,
            inactive,
            active,
        );
        let partial_failure = !matches!(result, Err(turn_loop::Error::Policy(_)));
        self.retain(result, partial_failure)
    }
}

/// Failure from the built-in computed movement and stock callback path.
pub type MoveToError = super::move_to::ActionError<
    stock_notifications::movement::Error,
    super::move_to::ComputedError<stock_notifications::movement::Error>,
    super::notifications::processing::Error,
>;

impl<'s, 'a> Session<'s, 'a, c6_core::unit::OperationParameters> {
    /// Execute an explicit MoveTo using the existing computed path/placement
    /// implementation and this session's stock callbacks and aftermath.
    ///
    /// The caller retains search and visibility owners between requests. Stock
    /// movement services must already be bound on the supplied backend. Missing
    /// movement/combat branches return their actual partial error; this method
    /// does not establish complete MoveTo parity beyond compared profiles.
    #[allow(clippy::too_many_arguments)]
    pub fn move_to(
        &mut self,
        issuer: i32,
        actor: (i32, u32),
        request: super::move_to::Request,
        inputs: &Inputs<'_, '_>,
        handlers: &handlers::BoundHandlers<'_>,
        runtime: &mut super::move_to::Runtime<'_>,
        movement: &mut super::move_to::ComputedMovement<'_, '_>,
        budget: &mut usize,
    ) -> Result<super::move_to::ActionOutcome, Error<MoveToError>> {
        self.check()?;
        if matches!(
            self.pending_choice().map(|choice| choice.kind),
            Some(PolicyKind::Research(_))
        ) {
            return Err(Error::PendingResearch);
        }
        let result = super::move_to::execute_active_request(
            &mut self.context,
            issuer,
            actor,
            request,
            inputs,
            handlers,
            runtime,
            movement,
            budget,
        );
        self.retain(result, true)
    }
}
