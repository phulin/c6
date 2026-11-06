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

pub mod advance;
#[path = "session_chop.rs"]
pub mod chop;
#[path = "session_citizens.rs"]
pub mod citizens;
#[path = "session_city_disposition.rs"]
pub mod city_disposition;
#[path = "session_city_raze.rs"]
pub mod city_raze;
pub mod city_strike;
pub mod combat;
#[path = "session_declare_war.rs"]
pub mod declare_war;
pub mod delegation;
#[path = "session_district.rs"]
pub mod district;
pub mod first_contact;
pub mod founding;
#[path = "session_government.rs"]
pub mod government;
#[path = "session_governor.rs"]
pub mod governor;
pub mod initialized;
#[path = "session_pantheon.rs"]
pub mod pantheon;
#[path = "session_pillage.rs"]
pub mod pillage;
#[path = "session_plot_purchase.rs"]
pub mod plot_purchase;
#[path = "session_policy.rs"]
pub mod policy;
#[path = "session_promotion.rs"]
pub mod promotion;
#[path = "session_purchase.rs"]
pub mod purchase;
#[path = "session_repair.rs"]
pub mod repair;
pub mod stock_turn_services;
#[path = "session_trade_route.rs"]
pub mod trade_route;
#[path = "session_upgrade.rs"]
pub mod upgrade;

#[derive(Debug)]
pub enum Error<E> {
    /// A previous request retained a partial prefix. Inspect the session rather
    /// than issuing another request against an incompletely advanced world.
    Failed(String),
    /// Resolve the pending research choice before issuing a unit order.
    PendingResearch,
    /// Finish the captured city's external disposition decision before any
    /// further action or turn progression. The completed melee is retained.
    PendingCityDisposition(c6_core::unit::ComponentId),
    /// The initial diplomacy statement has reached an external response choice.
    PendingFirstMeeting(c6_core::diplomacy_first_meeting::PendingDecision),
    /// A command is paused at the recipient's genuine external decision.
    PendingDelegation(super::delegation_actions::PendingDecision),
    /// This request does not belong to the current ordinary player-action site.
    DecisionBoundary(&'static str),
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
    research: Option<crate::research::Runtime>,
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
            research: Some(research),
            visibility,
            failed,
        }
    }
    pub fn state(&self) -> &LoadedState {
        self.context.notifications.state
    }
    /// Inspect the same current update owner without extracting or replacing it.
    pub fn update_runtime(&self) -> &loaded::Runtime {
        &self.update
    }
    pub fn context(&self) -> &current_aftermath::Context<'s, stock_notifications::Runtime<'a, P>> {
        &self.context
    }
    pub fn pending_choice(&self) -> Option<&PolicyChoice> {
        self.context
            .notifications
            .backend
            .first_meeting_update
            .as_ref()
            .and_then(|pending| pending.choice.as_ref())
            .or_else(|| {
                self.context
                    .notifications
                    .backend
                    .delegation
                    .as_ref()
                    .and_then(|pending| pending.policy_choice())
            })
            .or_else(|| self.context.notifications.backend.pending_policy_choice())
    }
    pub fn pending_first_meetings(&self) -> &[c6_core::diplomacy_first_meeting::PendingDecision] {
        self.context.notifications.backend.pending_first_meetings()
    }
    /// Apply one externally selected legal response and resume this retained
    /// Session. Positive may yield a second explicit information-exchange choice.
    pub fn respond_first_meeting(
        &mut self,
        issuer: i32,
        choice: c6_core::diplomacy_first_meeting::Choice,
    ) -> Result<
        c6_core::diplomacy_first_meeting::ChoiceOutcome,
        Error<stock_notifications::movement::Error>,
    > {
        if let Some(failure) = &self.failed {
            return Err(Error::Failed(failure.clone()));
        }
        if self
            .context
            .notifications
            .backend
            .first_meeting_update
            .is_some()
        {
            return Err(Error::DecisionBoundary(
                "resume first-contact result update",
            ));
        }
        if self
            .pending_first_meetings()
            .first()
            .is_none_or(|d| d.player != issuer)
        {
            return Err(Error::DecisionBoundary("first-contact response owner"));
        }
        if self.first_meeting_requires_update(choice) {
            return Err(Error::DecisionBoundary(
                "first-contact response requires update services",
            ));
        }
        let before = self.state().action_stamp();
        let before_decision = self.pending_first_meetings().first().copied();
        let result = self.context.notifications.backend.respond_first_meeting(
            self.context.notifications.state,
            issuer,
            choice,
        );
        let partial_failure = self.state().action_stamp() != before
            || self.pending_first_meetings().first().copied() != before_decision;
        let result = result.and_then(|progress| match progress {
            c6_core::diplomacy_first_meeting::ChoiceProgress::Pending(decision) => Ok(
                c6_core::diplomacy_first_meeting::ChoiceOutcome::Pending(decision),
            ),
            c6_core::diplomacy_first_meeting::ChoiceProgress::Closed(session) => Ok(
                c6_core::diplomacy_first_meeting::ChoiceOutcome::Closed(session),
            ),
            c6_core::diplomacy_first_meeting::ChoiceProgress::RequiresResultActionUpdate(
                result,
            ) => {
                self.context.notifications.backend.first_meeting_update =
                    Some(stock_notifications::first_contact_update::Continuation {
                        result,
                        choice: None,
                    });
                Err(stock_notifications::movement::Error::Missing(
                    "first-contact response requires update services",
                ))
            }
        });
        self.retain(result, partial_failure)
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
        (
            self.context,
            self.update,
            self.research
                .expect("action research loan restored before Session returns"),
            self.visibility,
        )
    }
    /// Move the sole current research owner into synchronous movement callbacks.
    /// Entry refuses a nested loan before taking either owner.
    fn begin_action_research<E>(&mut self) -> Result<(), Error<E>> {
        if self.context.notifications.backend.action_research.is_some() {
            return Err(Error::DecisionBoundary("research owner already loaned"));
        }
        let owner = self
            .research
            .take()
            .ok_or(Error::DecisionBoundary("research owner temporarily loaned"))?;
        self.context.notifications.backend.action_research = Some(owner);
        Ok(())
    }
    fn end_action_research<E>(&mut self) -> Result<(), Error<E>> {
        let owner = self
            .context
            .notifications
            .backend
            .action_research
            .take()
            .ok_or(Error::DecisionBoundary(
                "movement research owner unavailable",
            ))?;
        self.research = Some(owner);
        Ok(())
    }
    fn check<E>(&self) -> Result<(), Error<E>> {
        if let Some(pending) = &self.context.notifications.backend.delegation {
            return Err(match pending {
                delegation::Continuation::Decision(request) => {
                    Error::PendingDelegation(request.decision)
                }
                delegation::Continuation::HumanDecision(_) => {
                    Error::DecisionBoundary("pending human delegation choice")
                }
                delegation::Continuation::Update { .. }
                | delegation::Continuation::HumanUpdate { .. } => {
                    Error::DecisionBoundary("resume delegation action update")
                }
            });
        }
        if self
            .context
            .notifications
            .backend
            .first_meeting_update
            .is_some()
        {
            return Err(Error::DecisionBoundary(
                "resume first-contact result update",
            ));
        }
        if let Some(decision) = self.pending_first_meetings().first() {
            return Err(Error::PendingFirstMeeting(*decision));
        }
        match &self.failed {
            Some(failure) => Err(Error::Failed(failure.clone())),
            None => match self.pending_city_disposition() {
                Some(choice) => Err(Error::PendingCityDisposition(choice.city)),
                None => Ok(()),
            },
        }
    }
    fn retain<T, E: std::fmt::Debug>(
        &mut self,
        result: Result<T, E>,
        partial_failure: bool,
    ) -> Result<T, Error<E>> {
        result.map_err(|error| {
            if partial_failure || self.update.failed() {
                // Keep the failure on an existing owner too: extracting and
                // rewrapping the same parts must not erase an incomplete prefix.
                self.update.fail_player_callback();
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
        self.skip_turn_with_delivery(issuer, actor, inputs, handlers, budget, false)
    }
    /// Deliver an explicit SkipTurn at the current player's ordinary policy
    /// boundary, including an active NPC while the local human is inactive.
    /// This uses UnitOperation.Execute, without the earlier Broadcast helper.
    pub fn skip_turn_message(
        &mut self,
        issuer: i32,
        actor: UnitReference,
        inputs: &Inputs<'_, '_>,
        handlers: &handlers::BoundHandlers<'_>,
        budget: &mut usize,
    ) -> Result<stock_notifications::skip_turn::Outcome, Error<stock_notifications::skip_turn::Error>>
    {
        self.skip_turn_with_delivery(issuer, actor, inputs, handlers, budget, true)
    }
    fn skip_turn_with_delivery(
        &mut self,
        issuer: i32,
        actor: UnitReference,
        inputs: &Inputs<'_, '_>,
        handlers: &handlers::BoundHandlers<'_>,
        budget: &mut usize,
        message: bool,
    ) -> Result<stock_notifications::skip_turn::Outcome, Error<stock_notifications::skip_turn::Error>>
    {
        self.check()?;
        if message {
            self.ordinary_player_action(issuer)?;
        }
        if matches!(
            self.pending_choice().map(|choice| choice.kind),
            Some(PolicyKind::Research(_))
        ) {
            return Err(Error::PendingResearch);
        }
        let execute = if message {
            stock_notifications::skip_turn::execute_message
        } else {
            stock_notifications::skip_turn::execute
        };
        let result = execute(&mut self.context, issuer, actor, inputs, handlers, budget);
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
            &crate::game_update_state::deactivation::VisibilityHistory,
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
            self.research
                .as_mut()
                .ok_or(Error::DecisionBoundary("research owner temporarily loaned"))?,
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
            &crate::game_update_state::deactivation::VisibilityHistory,
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
            self.research
                .as_mut()
                .ok_or(Error::DecisionBoundary("research owner temporarily loaned"))?,
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
            &crate::game_update_state::deactivation::VisibilityHistory,
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
            self.research
                .as_mut()
                .ok_or(Error::DecisionBoundary("research owner temporarily loaned"))?,
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
    /// Execute a supported typed unit operation on the current player decision.
    /// Unsupported handlers return an error and retain any completed prefix.
    #[allow(clippy::too_many_arguments)]
    pub fn operation(
        &mut self,
        issuer: i32,
        actor: (i32, u32),
        request: super::move_to::OperationRequest,
        inputs: &Inputs<'_, '_>,
        handlers: &handlers::BoundHandlers<'_>,
        runtime: &mut super::move_to::Runtime<'_>,
        movement: &mut super::move_to::ComputedMovement<'_, '_>,
        budget: &mut usize,
    ) -> Result<super::move_to::ActionOutcome, Error<MoveToError>> {
        self.ordinary_player_action(issuer)?;
        let result = super::move_to::execute_operation_active_request(
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
    /// Ordinary Fortify; later-turn progression is performed by queue activation.
    #[allow(clippy::too_many_arguments)]
    pub fn fortify(
        &mut self,
        issuer: i32,
        actor: (i32, u32),
        inputs: &Inputs<'_, '_>,
        handlers: &handlers::BoundHandlers<'_>,
        runtime: &mut super::move_to::Runtime<'_>,
        movement: &mut super::move_to::ComputedMovement<'_, '_>,
        budget: &mut usize,
    ) -> Result<super::move_to::ActionOutcome, Error<MoveToError>> {
        self.operation(
            issuer,
            actor,
            super::move_to::OperationRequest::Fortify,
            inputs,
            handlers,
            runtime,
            movement,
            budget,
        )
    }
    /// Execute an explicit MoveTo using the existing computed path/placement
    /// composition (including joined civilian capture), stock callbacks and aftermath.
    ///
    /// The caller retains search and visibility owners between requests. Stock
    /// movement services must already be bound on the supplied backend. Missing
    /// movement/combat branches return their actual partial error; this method
    /// does not establish complete MoveTo parity beyond compared profiles.
    #[allow(clippy::too_many_arguments)]
    pub fn move_to<M>(
        &mut self,
        issuer: i32,
        actor: (i32, u32),
        request: super::move_to::Request,
        inputs: &Inputs<'_, '_>,
        handlers: &handlers::BoundHandlers<'_>,
        runtime: &mut super::move_to::Runtime<'_>,
        movement: &mut M,
        budget: &mut usize,
    ) -> Result<super::move_to::ActionOutcome, Error<MoveToError>>
    where
        M: super::move_to::Movement<
            stock_notifications::Runtime<'a>,
            Error = super::move_to::ComputedError<stock_notifications::movement::Error>,
        >,
    {
        self.check()?;
        if matches!(
            self.pending_choice().map(|choice| choice.kind),
            Some(PolicyKind::Research(_))
        ) {
            return Err(Error::PendingResearch);
        }
        self.begin_action_research()?;
        // Unit creation reached by this action reads the live Session history;
        // it does not inherit an unavailable turn-activation-only scope.
        let previous_history = self
            .context
            .notifications
            .backend
            .action_gossip_history_empty
            .replace(self.visibility.history.is_empty());
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
        self.context
            .notifications
            .backend
            .action_gossip_history_empty = previous_history;
        self.end_action_research()?;
        self.retain(result, true)
    }
}

/// Failure from an ordinary Builder improvement and its mandatory aftermath.
pub type BuildImprovementError = super::move_to::ActionError<
    stock_notifications::movement::Error,
    crate::builder_improvement::stock::Error,
    super::notifications::processing::Error,
>;
impl Session<'_, '_, c6_core::unit::OperationParameters> {
    /// Build the selected supported improvement through this SAME retained
    /// state, stock services, visibility and HeadlessMessagesProcessed boundary.
    #[allow(clippy::too_many_arguments)]
    pub fn build_improvement(
        &mut self,
        issuer: i32,
        actor: (i32, u32),
        improvement_hash: u32,
        modifiers: u32,
        inputs: &Inputs<'_, '_>,
        handlers: &handlers::BoundHandlers<'_>,
        runtime: &mut super::move_to::Runtime<'_>,
        dispatcher: &mut crate::builder_improvement::stock::Dispatcher<'_, '_>,
        budget: &mut usize,
    ) -> Result<super::move_to::ActionOutcome, Error<BuildImprovementError>> {
        self.check()?;
        if matches!(
            self.pending_choice().map(|c| c.kind),
            Some(PolicyKind::Research(_))
        ) {
            return Err(Error::PendingResearch);
        }
        // The session's current visibility history is the canonical Gossip
        // owner. Reborrow all other services instead of copying/replacing them.
        let mut current = crate::builder_improvement::stock::Dispatcher {
            death: dispatcher.death,
            definitions: dispatcher.definitions,
            cached: dispatcher.cached,
            services: &mut *dispatcher.services,
            legality: crate::builder_improvement::legality::Inputs {
                fallout_turns: dispatcher.legality.fallout_turns,
                drought_ids: dispatcher.legality.drought_ids,
                national_parks: dispatcher.legality.national_parks,
            },
            sight_definitions: dispatcher.sight_definitions,
            upkeep: dispatcher.upkeep,
            gossip_history: &self.visibility.history,
            alive_major_ids: dispatcher.alive_major_ids,
            reports: &mut *dispatcher.reports,
            worker_reports: &mut *dispatcher.worker_reports,
        };
        let result = super::move_to::execute_build_improvement_active_request(
            &mut self.context,
            issuer,
            actor,
            improvement_hash,
            modifiers,
            inputs,
            handlers,
            runtime,
            &mut current,
            budget,
        );
        self.retain(result, true)
    }
}

/// Failure from an explicit stock technology or civic queue request.
pub type ResearchError = crate::research::Error<stock_notifications::research::Error>;

impl<P> Session<'_, '_, P> {
    fn ordinary_player_action<E>(&self, player: i32) -> Result<(), Error<E>> {
        self.check()?;
        let controls = self
            .state()
            .partial_player_controls(player)
            .map_err(|_| Error::DecisionBoundary("current player controls unavailable"))?;
        if controls.turn_active != Some(true) {
            return Err(Error::DecisionBoundary("requested player is not active"));
        }
        match self.pending_choice() {
            Some(choice)
                if choice.player == player
                    && choice.kind == PolicyKind::PlayerActions
                    && self.update.pending_player_effect()
                        == Some(c6_core::game_update::PlayerEffect::ActiveControl { player }) =>
            {
                Ok(())
            }
            Some(_) => Err(Error::DecisionBoundary(
                "request differs from suspended player-action owner",
            )),
            None if controls.slot_status == 3 && self.update.pending_player_effect().is_none() => {
                Ok(())
            }
            None => Err(Error::DecisionBoundary(
                "ordinary human action or suspended player action required",
            )),
        }
    }
    /// Replace an externally selected technology queue at an ordinary player
    /// action site. This leaves any suspended PlayerActions callback in place;
    /// use resume_research for a source-positioned acquisition/activation choice.
    /// Also resolves a matching research choice in a first-contact result update;
    /// resume_first_meeting_update then continues that same suspended callback.
    pub fn request_technology(
        &mut self,
        player: i32,
        index: i32,
        definitions: &crate::research::Definitions,
        choices: &super::choose_tech::Definitions,
        culture: Option<&stock_notifications::research::CultureInputs<'_>>,
    ) -> Result<(), Error<ResearchError>> {
        self.request_research(
            player,
            index,
            crate::research::Kind::Technology,
            definitions,
            choices,
            culture,
        )
    }
    /// Replace an externally selected civic queue on the same current research
    /// and stock owners. Gameplay legality is computed by the existing Context.
    pub fn request_civic(
        &mut self,
        player: i32,
        index: i32,
        definitions: &crate::research::Definitions,
        choices: &super::choose_tech::Definitions,
        culture: Option<&stock_notifications::research::CultureInputs<'_>>,
    ) -> Result<(), Error<ResearchError>> {
        self.request_research(
            player,
            index,
            crate::research::Kind::Civic,
            definitions,
            choices,
            culture,
        )
    }
    fn request_research(
        &mut self,
        player: i32,
        index: i32,
        kind: crate::research::Kind,
        definitions: &crate::research::Definitions,
        choices: &super::choose_tech::Definitions,
        culture: Option<&stock_notifications::research::CultureInputs<'_>>,
    ) -> Result<(), Error<ResearchError>> {
        let result_choice = self
            .context
            .notifications
            .backend
            .first_meeting_update
            .as_ref()
            .map(|pending| pending.choice.as_ref())
            .or_else(|| {
                self.context
                    .notifications
                    .backend
                    .delegation
                    .as_ref()
                    .and_then(|pending| match pending {
                        delegation::Continuation::Update { choice, .. }
                        | delegation::Continuation::HumanUpdate { choice, .. } => {
                            Some(choice.as_ref())
                        }
                        delegation::Continuation::Decision(_)
                        | delegation::Continuation::HumanDecision(_) => None,
                    })
            });
        let resolving_result_choice = result_choice.is_some();
        if let Some(pending) = result_choice {
            if let Some(failure) = &self.failed {
                return Err(Error::Failed(failure.clone()));
            }
            let Some(choice) = pending else {
                return Err(Error::DecisionBoundary("resume diplomatic action update"));
            };
            if choice.player != player
                || choice.kind != PolicyKind::Research(kind)
                || !choice.options.contains(&index)
            {
                return Err(Error::DecisionBoundary(
                    "request differs from diplomatic update research choice",
                ));
            }
            match self.update.pending_player_effect() {
                Some(
                    c6_core::game_update::PlayerEffect::Activation { player: owner, .. }
                    | c6_core::game_update::PlayerEffect::ActiveControl { player: owner },
                ) if owner == player => {}
                _ => {
                    return Err(Error::DecisionBoundary(
                        "diplomatic update research callback differs",
                    ))
                }
            }
        } else {
            self.ordinary_player_action(player)?;
        }
        let mut effects = stock_notifications::research::Effects::new(
            &mut self.context.notifications.backend,
            self.research
                .as_mut()
                .ok_or(Error::DecisionBoundary("research owner temporarily loaned"))?,
            choices,
            culture,
        );
        let mut research = crate::research::Context {
            state: self.context.notifications.state,
            definitions,
            effects: &mut effects,
        };
        let result = match kind {
            crate::research::Kind::Technology => {
                research.request_technology(player, index, crate::research::InsertMode::Replace)
            }
            crate::research::Kind::Civic => {
                research.request_civic(player, index, crate::research::InsertMode::Replace)
            }
        }
        .and_then(|()| {
            if resolving_result_choice
                && c6_core::research_turn::current(&research, player, kind)? != index
            {
                return Err(crate::research::Error::Missing(
                    "diplomatic update research request did not select the choice",
                ));
            }
            Ok(())
        });
        if result.is_ok() {
            effects.backend.resolve_research_choice(player, kind);
            if let Some(pending) = effects.backend.first_meeting_update.as_mut() {
                pending.choice = None;
            }
            if let Some(
                delegation::Continuation::Update { choice, .. }
                | delegation::Continuation::HumanUpdate { choice, .. },
            ) = effects.backend.delegation.as_mut()
            {
                *choice = None;
            }
        }
        let partial_failure = !matches!(
            result,
            Err(crate::research::Error::IllegalTechnology(_)
                | crate::research::Error::IllegalCivic(_))
        );
        self.retain(result, partial_failure)
    }
}

impl<P> Session<'_, '_, P> {
    /// Select an ordinary Unit on the same live City and stock services.
    /// Legality precedes queue writes; failures retain the exact partial state.
    pub fn choose_unit(
        &mut self,
        owner: i32,
        city: u32,
        unit_hash: u32,
        definitions: &crate::city_production_turn::unit_selection::Definitions,
        current: &crate::city_production_turn::unit_selection::Current<'_>,
    ) -> Result<
        Vec<stock_notifications::city_production::unit::Record>,
        Error<crate::city_production_turn::Error>,
    > {
        self.ordinary_player_action(owner)?;
        let before = self.state().action_stamp();
        let (result, reports) = {
            let mut effects = stock_notifications::city_production::unit::Selection {
                runtime: &mut self.context.notifications.backend,
                definitions,
                current: crate::city_production_turn::unit_selection::Current {
                    build_maps: current.build_maps,
                    fallout_turns: current.fallout_turns,
                    civilization_levels: current.civilization_levels,
                },
                reports: Vec::new(),
            };
            let result = effects.choose(self.context.notifications.state, owner, city, unit_hash);
            (result, effects.reports)
        };
        let partial_failure = self.state().action_stamp() != before;
        self.retain(result.map(|()| reports), partial_failure)
    }
    /// Append an explicit building choice using the same stock runtime and
    /// already-bound current city queries. Legality runs before queue writes;
    /// successful selection returns ordered semantic production publications.
    #[allow(clippy::too_many_arguments)]
    pub fn choose_building(
        &mut self,
        owner: i32,
        city: u32,
        building_hash: u32,
        buildings: &crate::city_production_turn::buildings::Definitions,
        appeal: &crate::city_production_turn::appeal::Definitions,
        choices: &crate::city_production_turn::selection::Definitions,
        current: &crate::city_production_turn::selection::Current<'_>,
        queries: &impl crate::city_production_turn::selection::Queries,
    ) -> Result<
        Vec<stock_notifications::city_production::Record>,
        Error<crate::city_production_turn::Error>,
    > {
        self.ordinary_player_action(owner)?;
        let before = self.state().action_stamp();
        let (result, reports) = {
            let mut effects = stock_notifications::city_production::Effects::new(
                &mut self.context.notifications.backend,
                buildings,
                appeal,
                stock_notifications::city_production::Scope::StockPlainHeadlessNoAddedConsumers,
            );
            let result = effects.choose_building(
                self.context.notifications.state,
                owner,
                city,
                building_hash,
                choices,
                current,
                queries,
            );
            (result, effects.reports)
        };
        let partial_failure = self.state().action_stamp() != before;
        self.retain(result.map(|()| reports), partial_failure)
    }
}
