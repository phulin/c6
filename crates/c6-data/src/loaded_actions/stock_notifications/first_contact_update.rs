//! The mandatory Game.Update following delivery of a StatementResult action.
//!
//! This uses the ordinary turn loop's single-update implementation and the
//! caller's retained owners. It neither completes a turn nor selects policy.
use super::{deactivation, turn_loop, Runtime};
use crate::{
    game_update_state::{self as data, activation::upkeep::PolicyChoice},
    loaded_actions::{current_aftermath, session::advance::TurnServices},
};
use c6_core::loaded_state::{game_update as loaded, LoadedState};
use std::cell::RefCell;

/// Retained on the same backend, including when Session owners are extracted.
pub(crate) struct Continuation {
    pub result: c6_core::diplomacy_first_meeting::ResultActionContinuation,
    pub choice: Option<PolicyChoice>,
}

#[derive(Debug)]
pub enum Outcome {
    Diplomacy(c6_core::diplomacy_first_meeting::ChoiceOutcome),
    Choice(PolicyChoice),
}

#[derive(Debug)]
pub enum Error<E> {
    Response(super::movement::Error),
    Update(turn_loop::Error<E>),
}

impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "first-contact result update: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}

/// Qualify BroadcastMessage's required TurnUnready ingress before mutation.
/// CanUnreadyTurn returns false immediately for the genuine active local
/// player. Other profiles require the phase/timer/host message continuation.
pub fn preflight_ingress(state: &LoadedState) -> Result<(), super::movement::Error> {
    let local = state.retained().core.local_player;
    if local < 0
        || !state
            .turn_active(local)
            .map_err(super::movement::Error::State)?
    {
        return Err(super::movement::Error::Missing(
            "first-contact result-action local-player TurnUnready continuation",
        ));
    }
    Ok(())
}

/// Execute exactly the required update, or resume its suspended callback.
/// `Some` is a genuine external decision: retain the response continuation and
/// resume this same update before processing the remaining diplomacy effects.
/// `None` means this update returned, so the statement close may continue.
/// Missing provider support is an error; no successful default is supplied.
/// The caller must retain a failed prefix rather than retry it as a fresh update.
#[allow(clippy::too_many_arguments)]
pub fn execute<'a, P, S: TurnServices<'a, P>>(
    context: &mut current_aftermath::Context<'_, Runtime<'a, P>>,
    runtime: &mut loaded::Runtime,
    research: &mut crate::research::Runtime,
    inputs: &deactivation::Inputs<'_, '_, '_>,
    visibility: &mut data::deactivation::VisibilityHistory,
    services: &mut S,
    budget: &mut usize,
    resuming: bool,
) -> Result<Option<PolicyChoice>, turn_loop::Error<S::Error>> {
    let services = RefCell::new(services);
    let result = turn_loop::update(
        context,
        runtime,
        research,
        inputs,
        visibility,
        budget,
        resuming,
        &mut |s, b, u, r, v, p, f| services.borrow_mut().activate(s, b, u, r, v, p, f),
        &mut |s, b, p| services.borrow_mut().inactive(s, b, p),
        &mut |s, b, u, p| services.borrow_mut().active(s, b, u, p),
    );
    if result.is_err() {
        runtime.fail_player_callback();
    }
    result
}
