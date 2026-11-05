//! Persistent turn handoff and source-positioned external policy continuations.
use super::{current_aftermath, deactivation, Runtime};
use crate::game_update_state::{
    self as data,
    activation::upkeep::{ActivationOutcome, PolicyChoice, PolicyKind},
};
use c6_core::{
    loaded_state::{game_update as loaded, turn_completion, ActionStamp, LoadedState},
    turn_completion as completion,
};
#[derive(Debug)]
pub enum Outcome {
    InvalidEncodedPlayer,
    PausedHuman,
    AlreadyComplete,
    DecisionReady {
        player: i32,
        updates: usize,
    },
    /// The current callback is suspended before a genuine player choice.
    Choice(PolicyChoice),
}
#[derive(Debug)]
pub enum Error<E> {
    Completion(completion::Error),
    Activation(E),
    InactiveControl(E),
    ActiveControl(E),
    Notifications(c6_core::unit_operation_aftermath::Error),
    Deactivation(deactivation::Error),
    Policy(&'static str),
    PolicyAction(String),
    Budget,
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stock end turn: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}
#[derive(Clone, Copy)]
enum Phase {
    MandatoryFirst,
    ConditionalSecond,
    Ordinary,
}
pub(crate) struct Progress {
    stamp: ActionStamp,
    issuer: i32,
    phase: Phase,
    updates: usize,
    in_progress: bool,
    choice: Option<PolicyChoice>,
    queued_completion: Option<(i32, u32)>,
}
impl<P> Runtime<'_, P> {
    pub fn pending_policy_choice(&self) -> Option<&PolicyChoice> {
        self.turn_progress.as_ref().and_then(|p| p.choice.as_ref())
    }
}
#[allow(clippy::too_many_arguments)]
fn update<'b, P, E>(
    context: &mut current_aftermath::Context<'_, Runtime<'b, P>>,
    runtime: &mut loaded::Runtime,
    research_runtime: &mut crate::research::Runtime,
    inputs: &deactivation::Inputs<'_, '_, '_>,
    visibility: &mut data::deactivation::VisibilityHistory,
    budget: &mut usize,
    resuming: bool,
    activation: &mut impl FnMut(
        &mut LoadedState,
        &mut Runtime<'b, P>,
        &mut loaded::Runtime,
        &mut crate::research::Runtime,
        i32,
        bool,
    ) -> Result<ActivationOutcome, E>,
    inactive: &mut impl FnMut(&mut LoadedState, &mut Runtime<'b, P>, i32) -> Result<(), E>,
    active: &mut impl FnMut(
        &mut LoadedState,
        &mut Runtime<'b, P>,
        &mut loaded::Runtime,
        i32,
    ) -> Result<ActivationOutcome, E>,
) -> Result<Option<PolicyChoice>, Error<E>> {
    if !resuming {
        if *budget == 0 {
            return Err(Error::Budget);
        }
        *budget -= 1;
        data::context(
            context.notifications.state,
            runtime,
            inputs.readiness,
            &mut context.notifications.backend,
        )
        .map_err(Error::Completion)?
        .prepare_prefix()
        .map_err(Error::Completion)?;
    }
    loop {
        loop {
            let next = match runtime.pending_player_effect() {
                Some(effect) => Some(effect),
                None => loaded::prepare_current_player(context.notifications.state, runtime)
                    .map_err(Error::Completion)?,
            };
            let Some(next) = next else { break };
            match next {
                c6_core::game_update::PlayerEffect::Activation { player, first } => {
                    match activation(
                        context.notifications.state,
                        &mut context.notifications.backend,
                        runtime,
                        research_runtime,
                        player,
                        first,
                    )
                    .map_err(Error::Activation)?
                    {
                        ActivationOutcome::Choice(choice) => return Ok(Some(choice)),
                        ActivationOutcome::Complete => {}
                    }
                    loaded::complete_activation(context.notifications.state, runtime)
                        .map_err(Error::Completion)?;
                }
                c6_core::game_update::PlayerEffect::ActiveControl { player } => {
                    match active(
                        context.notifications.state,
                        &mut context.notifications.backend,
                        runtime,
                        player,
                    )
                    .map_err(Error::ActiveControl)?
                    {
                        ActivationOutcome::Choice(choice) => return Ok(Some(choice)),
                        ActivationOutcome::Complete => {}
                    }
                    loaded::complete_active_control(context.notifications.state, runtime)
                        .map_err(Error::Completion)?;
                }
                c6_core::game_update::PlayerEffect::InactiveControl { player } => {
                    inactive(
                        context.notifications.state,
                        &mut context.notifications.backend,
                        player,
                    )
                    .map_err(Error::InactiveControl)?;
                    loaded::complete_inactive_player(context.notifications.state, runtime)
                        .map_err(Error::Completion)?;
                }
                c6_core::game_update::PlayerEffect::Blocking { player } => {
                    context
                        .notifications
                        .update(player, budget)
                        .map_err(Error::Notifications)?;
                    loaded::complete_current_player(context.notifications.state, runtime)
                        .map_err(Error::Completion)?;
                }
            }
        }
        match deactivation::finish_completed_human_pass(
            context, runtime, inputs, visibility, budget,
        )
        .map_err(Error::Deactivation)?
        {
            deactivation::Outcome::Continue(_)
            | deactivation::Outcome::ContinueWithoutDeactivation => continue,
            deactivation::Outcome::Deactivated(_) | deactivation::Outcome::NoDeactivation => {
                return Ok(None)
            }
        }
    }
}
#[allow(clippy::too_many_arguments)]
fn drive<'b, P, E>(
    context: &mut current_aftermath::Context<'_, Runtime<'b, P>>,
    runtime: &mut loaded::Runtime,
    research_runtime: &mut crate::research::Runtime,
    inputs: &deactivation::Inputs<'_, '_, '_>,
    visibility: &mut data::deactivation::VisibilityHistory,
    budget: &mut usize,
    progress: &mut Progress,
    activation: &mut impl FnMut(
        &mut LoadedState,
        &mut Runtime<'b, P>,
        &mut loaded::Runtime,
        &mut crate::research::Runtime,
        i32,
        bool,
    ) -> Result<ActivationOutcome, E>,
    inactive: &mut impl FnMut(&mut LoadedState, &mut Runtime<'b, P>, i32) -> Result<(), E>,
    active: &mut impl FnMut(
        &mut LoadedState,
        &mut Runtime<'b, P>,
        &mut loaded::Runtime,
        i32,
    ) -> Result<ActivationOutcome, E>,
) -> Result<Outcome, Error<E>> {
    loop {
        if !progress.in_progress {
            match progress.phase {
                Phase::ConditionalSecond => {
                    if !context
                        .notifications
                        .state
                        .turn_active(progress.issuer)
                        .map_err(|_| Error::Policy("current completion player"))?
                    {
                        progress.phase = Phase::Ordinary;
                        continue;
                    }
                }
                Phase::Ordinary => {
                    // An entered completion remains in flight through all its
                    // nested updates, until Execute returns at this boundary.
                    context.notifications.backend.completion_inflight = None;
                    // Ordinary begins after the parent Complete.Execute has
                    // returned. Only now may ProcessMessages drain a command
                    // queued by its suspended active-control callback.
                    if let Some((player, reason)) = progress.queued_completion.take() {
                        let prefix = (turn_completion::Context {
                            state: context.notifications.state,
                            effects: &mut context.notifications.backend,
                        })
                        .execute(player, reason)
                        .map_err(Error::Completion)?;
                        match prefix {
                            completion::Prefix::RequiresGameUpdate(_) => {
                                context.notifications.backend.completion_inflight = Some(player);
                                progress.issuer = player;
                                progress.phase = Phase::MandatoryFirst;
                                continue;
                            }
                            completion::Prefix::AlreadyComplete => {}
                            completion::Prefix::InvalidEncodedPlayer => {
                                return Err(Error::Policy("queued completion player is invalid"))
                            }
                            completion::Prefix::PausedHuman => {
                                return Err(Error::Policy("queued completion player is paused"))
                            }
                        }
                    }
                    let state = &context.notifications.state;
                    let owner = state.retained().core.local_player;
                    let index = usize::try_from(owner)
                        .ok()
                        .filter(|i| *i < 64)
                        .ok_or(Error::Policy("current next player slot"))?;
                    if state
                        .player_slot_status(owner)
                        .map_err(|_| Error::Policy("current next player role"))?
                        == 3
                        && state
                            .turn_active(owner)
                            .map_err(|_| Error::Policy("current next player activity"))?
                        && !state.retained().core.player_processor.activation_event[index]
                        && runtime.pending().is_none()
                    {
                        return Ok(Outcome::DecisionReady {
                            player: owner,
                            updates: progress.updates,
                        });
                    }
                }
                Phase::MandatoryFirst => {}
            }
        }
        if let Some(choice) = update(
            context,
            runtime,
            research_runtime,
            inputs,
            visibility,
            budget,
            progress.in_progress,
            activation,
            inactive,
            active,
        )? {
            if runtime.pending_player_effect().is_none() {
                return Err(Error::Policy("policy choice lost its current callback"));
            }
            progress.in_progress = true;
            progress.choice = Some(choice.clone());
            return Ok(Outcome::Choice(choice));
        }
        progress.updates += 1;
        progress.in_progress = false;
        progress.choice = None;
        progress.phase = match progress.phase {
            Phase::MandatoryFirst => Phase::ConditionalSecond,
            Phase::ConditionalSecond | Phase::Ordinary => Phase::Ordinary,
        };
    }
}
fn retain<P, E>(
    context: &mut current_aftermath::Context<'_, Runtime<'_, P>>,
    runtime: &mut loaded::Runtime,
    progress: Progress,
    result: Result<Outcome, Error<E>>,
) -> Result<Outcome, Error<E>> {
    if matches!(result, Ok(Outcome::Choice(_))) {
        context.notifications.backend.turn_progress = Some(progress);
    } else {
        context.notifications.backend.turn_progress = None;
        if result.is_err() {
            runtime.fail_player_callback();
        }
    }
    result
}
/// Execute completion and mandatory ordinary updates. A Choice retains the
/// current callback and outer update position on the SAME backend; it is not an
/// error or a completed update. Call resume_research after supplying a research choice.
#[allow(clippy::too_many_arguments)]
pub fn execute<'b, P, E>(
    context: &mut current_aftermath::Context<'_, Runtime<'b, P>>,
    runtime: &mut loaded::Runtime,
    research_runtime: &mut crate::research::Runtime,
    inputs: &deactivation::Inputs<'_, '_, '_>,
    visibility: &mut data::deactivation::VisibilityHistory,
    player: i32,
    reason: u32,
    budget: &mut usize,
    mut activation: impl FnMut(
        &mut LoadedState,
        &mut Runtime<'b, P>,
        &mut loaded::Runtime,
        &mut crate::research::Runtime,
        i32,
        bool,
    ) -> Result<ActivationOutcome, E>,
    mut inactive: impl FnMut(&mut LoadedState, &mut Runtime<'b, P>, i32) -> Result<(), E>,
    mut active: impl FnMut(
        &mut LoadedState,
        &mut Runtime<'b, P>,
        &mut loaded::Runtime,
        i32,
    ) -> Result<ActivationOutcome, E>,
) -> Result<Outcome, Error<E>> {
    if context.notifications.backend.turn_progress.is_some() {
        return Err(Error::Policy("resume the pending external choice"));
    }
    let prefix = match (turn_completion::Context {
        state: context.notifications.state,
        effects: &mut context.notifications.backend,
    })
    .execute(player, reason)
    {
        Ok(prefix) => prefix,
        Err(error) => {
            runtime.fail_player_callback();
            return Err(Error::Completion(error));
        }
    };
    match prefix {
        completion::Prefix::InvalidEncodedPlayer => Ok(Outcome::InvalidEncodedPlayer),
        completion::Prefix::PausedHuman => Ok(Outcome::PausedHuman),
        completion::Prefix::AlreadyComplete => Ok(Outcome::AlreadyComplete),
        completion::Prefix::RequiresGameUpdate(_) => {
            context.notifications.backend.completion_inflight = Some(player);
            let mut progress = Progress {
                stamp: context.notifications.state.action_stamp(),
                issuer: player,
                phase: Phase::MandatoryFirst,
                updates: 0,
                in_progress: false,
                choice: None,
                queued_completion: None,
            };
            let result = drive(
                context,
                runtime,
                research_runtime,
                inputs,
                visibility,
                budget,
                &mut progress,
                &mut activation,
                &mut inactive,
                &mut active,
            );
            retain(context, runtime, progress, result)
        }
    }
}
/// Queue an ordinary completion message from a suspended player-action site.
/// The required `return_active` callback executes only the current control
/// frame's return/wait suffix. Completion is drained after the parent message
/// returns, preserving the original non-reentrant QueueMessage ordering.
#[allow(clippy::too_many_arguments)]
pub fn resume_end_turn<'b, P, E>(
    context: &mut current_aftermath::Context<'_, Runtime<'b, P>>,
    runtime: &mut loaded::Runtime,
    research_runtime: &mut crate::research::Runtime,
    inputs: &deactivation::Inputs<'_, '_, '_>,
    visibility: &mut data::deactivation::VisibilityHistory,
    budget: &mut usize,
    mut activation: impl FnMut(
        &mut LoadedState,
        &mut Runtime<'b, P>,
        &mut loaded::Runtime,
        &mut crate::research::Runtime,
        i32,
        bool,
    ) -> Result<ActivationOutcome, E>,
    mut inactive: impl FnMut(&mut LoadedState, &mut Runtime<'b, P>, i32) -> Result<(), E>,
    mut active: impl FnMut(
        &mut LoadedState,
        &mut Runtime<'b, P>,
        &mut loaded::Runtime,
        i32,
    ) -> Result<ActivationOutcome, E>,
    mut return_active: impl FnMut(
        &mut LoadedState,
        &mut Runtime<'b, P>,
        &mut loaded::Runtime,
        i32,
    ) -> Result<ActivationOutcome, E>,
) -> Result<Outcome, Error<E>> {
    let progress = context
        .notifications
        .backend
        .turn_progress
        .as_ref()
        .ok_or(Error::Policy("no suspended player action"))?;
    let choice = progress
        .choice
        .as_ref()
        .ok_or(Error::Policy("no suspended player action"))?;
    let owner = choice.player;
    if !progress
        .stamp
        .same_instance(context.notifications.state.action_stamp())
        || choice.kind != PolicyKind::PlayerActions
        || progress.queued_completion.is_some()
        || runtime.pending_player_effect()
            != Some(c6_core::game_update::PlayerEffect::ActiveControl { player: owner })
        || !context
            .notifications
            .state
            .turn_active(owner)
            .map_err(|_| Error::Policy("current action player"))?
    {
        return Err(Error::Policy("player action continuation differs"));
    }
    let mut progress = context
        .notifications
        .backend
        .turn_progress
        .take()
        .expect("checked");
    progress.choice = None;
    progress.queued_completion = Some((owner, completion::NONE));
    let mut resumed_active = |state: &mut LoadedState,
                              backend: &mut Runtime<'b, P>,
                              runtime: &mut loaded::Runtime,
                              player: i32| {
        // The queued command cannot enter Execute until the suspended parent
        // Update returns. Its internal NPC pass loop can select this actor
        // again meanwhile: retain the submitted decision and execute the
        // ordinary control return suffix, rather than requesting it twice.
        if player == owner {
            return_active(state, backend, runtime, player)
        } else {
            active(state, backend, runtime, player)
        }
    };
    let result = drive(
        context,
        runtime,
        research_runtime,
        inputs,
        visibility,
        budget,
        &mut progress,
        &mut activation,
        &mut inactive,
        &mut resumed_active,
    );
    retain(context, runtime, progress, result)
}

/// Validate and apply the actual external technology request, then resume at
/// the suspended callback. No completion command, update prefix or activation
/// upkeep is replayed. Invalid choices preserve the pending continuation.
#[allow(clippy::too_many_arguments)]
pub fn resume_research<'b, P, E>(
    context: &mut current_aftermath::Context<'_, Runtime<'b, P>>,
    runtime: &mut loaded::Runtime,
    inputs: &deactivation::Inputs<'_, '_, '_>,
    visibility: &mut data::deactivation::VisibilityHistory,
    budget: &mut usize,
    definitions: &crate::research::Definitions,
    research_runtime: &mut crate::research::Runtime,
    choices: &crate::loaded_actions::choose_tech::Definitions,
    culture: Option<&super::research::CultureInputs<'_>>,
    index: i32,
    mut activation: impl FnMut(
        &mut LoadedState,
        &mut Runtime<'b, P>,
        &mut loaded::Runtime,
        &mut crate::research::Runtime,
        i32,
        bool,
    ) -> Result<ActivationOutcome, E>,
    mut inactive: impl FnMut(&mut LoadedState, &mut Runtime<'b, P>, i32) -> Result<(), E>,
    mut active: impl FnMut(
        &mut LoadedState,
        &mut Runtime<'b, P>,
        &mut loaded::Runtime,
        i32,
    ) -> Result<ActivationOutcome, E>,
) -> Result<Outcome, Error<E>> {
    let progress = context
        .notifications
        .backend
        .turn_progress
        .as_ref()
        .ok_or(Error::Policy("no suspended policy choice"))?;
    if !progress
        .stamp
        .same_instance(context.notifications.state.action_stamp())
    {
        return Err(Error::Policy(
            "policy continuation belongs to another world",
        ));
    }
    let choice = progress
        .choice
        .as_ref()
        .ok_or(Error::Policy("no suspended policy choice"))?
        .clone();
    let kind = match choice.kind {
        PolicyKind::Research(kind) => kind,
        PolicyKind::PlayerActions => {
            return Err(Error::Policy("current choice requires a player action"))
        }
    };
    if !choice.options.contains(&index) {
        return Err(Error::Policy("illegal external research choice"));
    }
    match runtime.pending_player_effect() {
        Some(
            c6_core::game_update::PlayerEffect::Activation { player, .. }
            | c6_core::game_update::PlayerEffect::ActiveControl { player },
        ) if player == choice.player => {}
        _ => {
            return Err(Error::Policy(
                "policy continuation current callback differs",
            ))
        }
    }
    {
        let mut effects = super::research::Effects {
            backend: &mut context.notifications.backend,
            runtime: research_runtime,
            choices,
            choice_graph: None,
            sight: None,
            culture,
        };
        let mut research = crate::research::Context {
            state: context.notifications.state,
            definitions,
            effects: &mut effects,
        };
        let legal = match kind {
            crate::research::Kind::Technology => {
                research.can_research_technology(choice.player, index)
            }
            crate::research::Kind::Civic => research.can_progress_civic(choice.player, index),
        }
        .map_err(|e| Error::PolicyAction(format!("{e:?}")))?;
        if !legal {
            return Err(Error::Policy("research choice is no longer legal"));
        }
        let request = match kind {
            crate::research::Kind::Technology => research.request_technology(
                choice.player,
                index,
                crate::research::InsertMode::Replace,
            ),
            crate::research::Kind::Civic => {
                research.request_civic(choice.player, index, crate::research::InsertMode::Replace)
            }
        };
        if let Err(error) = request {
            runtime.fail_player_callback();
            return Err(Error::PolicyAction(format!("{error:?}")));
        }
        let selected = c6_core::research_turn::current(&research, choice.player, kind)
            .map_err(|e| Error::PolicyAction(format!("{e:?}")))?;
        if selected != index {
            runtime.fail_player_callback();
            return Err(Error::Policy("research request did not select the choice"));
        }
    }
    let mut progress = context
        .notifications
        .backend
        .turn_progress
        .take()
        .expect("checked policy continuation");
    progress.choice = None;
    let result = drive(
        context,
        runtime,
        research_runtime,
        inputs,
        visibility,
        budget,
        &mut progress,
        &mut activation,
        &mut inactive,
        &mut active,
    );
    retain(context, runtime, progress, result)
}
