//! Whole current-state human turn handoff on the persistent stock backend.
//! New-round and active nonhuman policy continuations remain unsupported.
use super::{current_aftermath, deactivation, Runtime};
use crate::game_update_state as data;
use c6_core::{
    loaded_state::{game_update as loaded, turn_completion, LoadedState},
    turn_completion as completion,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    InvalidEncodedPlayer,
    PausedHuman,
    AlreadyComplete,
    /// Complete ordinary updates returned before exposing this player's input.
    DecisionReady {
        player: i32,
        updates: usize,
    },
}
#[derive(Debug)]
pub enum Error<E> {
    Completion(completion::Error),
    Activation(E),
    InactiveControl(E),
    Notifications(c6_core::unit_operation_aftermath::Error),
    Deactivation(deactivation::Error),
    Budget,
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stock end turn: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}

#[allow(clippy::too_many_arguments)]
fn update<'b, P, E>(
    context: &mut current_aftermath::Context<'_, Runtime<'b, P>>,
    runtime: &mut loaded::Runtime,
    inputs: &deactivation::Inputs<'_, '_, '_>,
    visibility: &mut data::deactivation::VisibilityHistory,
    budget: &mut usize,
    activation: &mut impl FnMut(&mut LoadedState, &mut Runtime<'b, P>, i32, bool) -> Result<(), E>,
    inactive: &mut impl FnMut(&mut LoadedState, &mut Runtime<'b, P>, i32) -> Result<(), E>,
) -> Result<(), Error<E>> {
    if *budget == 0 {
        return Err(Error::Budget);
    }
    *budget -= 1;
    {
        let mut c = data::context(
            context.notifications.state,
            runtime,
            inputs.readiness,
            &mut context.notifications.backend,
        )
        .map_err(Error::Completion)?;
        c.prepare_prefix().map_err(Error::Completion)?;
    }
    loop {
        let next = loaded::prepare_current_player(context.notifications.state, runtime)
            .map_err(Error::Completion)?;
        let Some(next) = next else { break };
        let owner = match next {
            c6_core::game_update::PlayerEffect::Blocking { player } => player,
            c6_core::game_update::PlayerEffect::Activation { player, first } => {
                activation(
                    context.notifications.state,
                    &mut context.notifications.backend,
                    player,
                    first,
                )
                .map_err(Error::Activation)?;
                loaded::complete_activation(context.notifications.state, runtime)
                    .map_err(Error::Completion)?;
                player
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
                continue;
            }
            c6_core::game_update::PlayerEffect::ActiveControl { .. } => {
                return Err(Error::Completion(completion::Error::Unsupported(
                    "current mixed nonhuman/off-turn controller gameplay",
                )));
            }
        };
        context
            .notifications
            .update(owner, budget)
            .map_err(Error::Notifications)?;
        loaded::complete_current_player(context.notifications.state, runtime)
            .map_err(Error::Completion)?;
    }
    deactivation::finish_completed_human_pass(context, runtime, inputs, visibility, budget)
        .map_err(Error::Deactivation)?;
    Ok(())
}

/// Execute PlayerTurnComplete, its mandatory first Update and conditional
/// second Update, then ordinary engine updates until the active human has
/// finished activation. `activation` MUST execute the full TurnManager body and
/// Processor activation report; the public upkeep::Context::activate supplies
/// that implementation. `inactive` executes the selected inactive controller
/// branch (stock inactive_control::run), retaining its pending work state. Both
/// receive the same canonical state and backend.
#[allow(clippy::too_many_arguments)]
pub fn execute<'b, P, E>(
    context: &mut current_aftermath::Context<'_, Runtime<'b, P>>,
    runtime: &mut loaded::Runtime,
    inputs: &deactivation::Inputs<'_, '_, '_>,
    visibility: &mut data::deactivation::VisibilityHistory,
    player: i32,
    reason: u32,
    budget: &mut usize,
    mut activation: impl FnMut(&mut LoadedState, &mut Runtime<'b, P>, i32, bool) -> Result<(), E>,
    mut inactive: impl FnMut(&mut LoadedState, &mut Runtime<'b, P>, i32) -> Result<(), E>,
) -> Result<Outcome, Error<E>> {
    let result = (|| {
        let prefix = turn_completion::Context {
            state: context.notifications.state,
            effects: &mut context.notifications.backend,
        }
        .execute(player, reason)
        .map_err(Error::Completion)?;
        match prefix {
            completion::Prefix::InvalidEncodedPlayer => Ok(Outcome::InvalidEncodedPlayer),
            completion::Prefix::PausedHuman => Ok(Outcome::PausedHuman),
            completion::Prefix::AlreadyComplete => Ok(Outcome::AlreadyComplete),
            completion::Prefix::RequiresGameUpdate(_mandatory) => {
                // Each call freshly resolves this exclusive logical Game through
                // a current data Context, as the native singleton calls do.
                update(
                    context,
                    runtime,
                    inputs,
                    visibility,
                    budget,
                    &mut activation,
                    &mut inactive,
                )?;
                let mut updates = 1;
                if context
                    .notifications
                    .state
                    .turn_active(player)
                    .map_err(|_| {
                        Error::Completion(completion::Error::Unsupported(
                            "current completion player",
                        ))
                    })?
                {
                    update(
                        context,
                        runtime,
                        inputs,
                        visibility,
                        budget,
                        &mut activation,
                        &mut inactive,
                    )?;
                    updates += 1;
                }
                loop {
                    let state = &context.notifications.state;
                    let owner = state.retained().core.local_player;
                    if state.player_slot_status(owner).map_err(|_| {
                        Error::Completion(completion::Error::Unsupported(
                            "current next player control role",
                        ))
                    })? != 3
                    {
                        return Err(Error::Completion(completion::Error::Unsupported(
                            "next nonhuman external decision gameplay",
                        )));
                    }
                    let index = usize::try_from(owner).ok().filter(|i| *i < 64).ok_or(
                        Error::Completion(completion::Error::Unsupported(
                            "current next player slot",
                        )),
                    )?;
                    if state.turn_active(owner).map_err(|_| {
                        Error::Completion(completion::Error::Unsupported(
                            "current next player activity",
                        ))
                    })? && !state.retained().core.player_processor.activation_event[index]
                        && runtime.pending().is_none()
                    {
                        return Ok(Outcome::DecisionReady {
                            player: owner,
                            updates,
                        });
                    }
                    update(
                        context,
                        runtime,
                        inputs,
                        visibility,
                        budget,
                        &mut activation,
                        &mut inactive,
                    )?;
                    updates += 1;
                }
            }
        }
    })();
    if result.is_err() {
        runtime.fail_player_callback();
    }
    result
}
