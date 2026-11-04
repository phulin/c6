//! Persistent stock SkipTurn request composition through headless aftermath.
use crate::loaded_actions::{
    self as actions, current_aftermath, handlers, stock_notifications, transition, Inputs,
};
use c6_core::map::UnitReference;

#[derive(Debug)]
pub enum Error {
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Unsupported(&'static str),
    Manager(actions::Failure),
    Aftermath(current_aftermath::Error<actions::notifications::processing::Error>),
}
#[derive(Debug)]
pub struct Outcome {
    /// No events for an invalid addressed player or a missing Unit.
    pub events: Vec<actions::Event>,
    pub boundary: Option<transition::ExecutionBoundary>,
}
/// Apply one explicit player order using the SAME state, script and report
/// provider retained by `context`. Callers may invoke this repeatedly without
/// rebuilding rules or importing state. Failure retains its completed prefix.
/// Inactive-player unready scheduling remains explicitly unsupported.
pub fn execute(
    context: &mut current_aftermath::Context<'_, stock_notifications::Runtime<'_, ()>>,
    issuer: i32,
    actor: UnitReference,
    inputs: &Inputs<'_, '_>,
    handlers: &handlers::BoundHandlers<'_>,
    budget: &mut usize,
) -> Result<Outcome, Error> {
    if let Some(stage) = context.failed_at() {
        return Err(Error::Aftermath(
            current_aftermath::Error::FailedContinuation(stage),
        ));
    }
    let state = &mut context.notifications.state;
    if issuer != actor.player() {
        return Err(Error::Unsupported("issuer does not own requested Unit"));
    }
    if issuer as u32 >= 64
        || !state
            .world()
            .player_is_valid(issuer)
            .map_err(Error::World)?
    {
        return Ok(Outcome {
            events: Vec::new(),
            boundary: None,
        });
    }
    if !state.turn_active(issuer).map_err(Error::State)? {
        return Err(Error::Unsupported("inactive requested player continuation"));
    }
    let local = state.retained().core.local_player;
    if local < 0 || !state.turn_active(local).map_err(Error::State)? {
        return Err(Error::Unsupported("local player unready continuation"));
    }
    let exists = state
        .world()
        .player(issuer)
        .map_err(Error::World)?
        .unit(actor.id)
        .map_err(Error::World)?
        .is_some();
    let events = if exists {
        let backend = &mut context.notifications.backend;
        let events = actions::skip_turn_state_phase_observed(
            state,
            actor,
            inputs,
            handlers,
            &mut backend.script,
            &mut backend.reporting,
        )
        .map_err(Error::Manager)?;
        context
            .after_manager(issuer, budget)
            .map_err(Error::Aftermath)?;
        events
    } else {
        Vec::new()
    };
    context.messages_processed().map_err(Error::Aftermath)?;
    Ok(Outcome {
        events,
        boundary: Some(transition::ExecutionBoundary::HeadlessMessagesProcessed),
    })
}
