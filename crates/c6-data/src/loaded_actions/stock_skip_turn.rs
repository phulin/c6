//! Persistent stock SkipTurn request composition through headless aftermath.
//!
//! After installing typed state and constructing one stock aftermath context,
//! callers issue successive explicit orders without importing the world again:
//!
//! ```no_run
//! use c6_data::loaded_actions::{current_aftermath, handlers, stock_notifications, Inputs};
//! use c6_core::{map::UnitReference, unit::ComponentId};
//!
//! fn order_units(
//!     session: &mut current_aftermath::Context<'_, stock_notifications::Runtime<'_>>,
//!     inputs: &Inputs<'_, '_>,
//!     handlers: &handlers::BoundHandlers<'_>,
//!     owner: i32,
//!     ids: &[u32],
//! ) -> Result<(), stock_notifications::skip_turn::Error> {
//!     let mut continuation_budget = 64;
//!     for &id in ids {
//!         let actor = UnitReference::from_component_id(ComponentId {
//!             owner: owner as i16, component_type: 1, id,
//!         });
//!         stock_notifications::skip_turn::execute(
//!             session, owner, actor, inputs, handlers, &mut continuation_budget,
//!         )?;
//!     }
//!     // session retains the same Script RNG, reporting and semantic decisions.
//!     Ok(())
//! }
//! ```
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
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stock SkipTurn request: {self:?}")
    }
}
impl std::error::Error for Error {}
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
pub fn execute<P>(
    context: &mut current_aftermath::Context<'_, stock_notifications::Runtime<'_, P>>,
    issuer: i32,
    actor: UnitReference,
    inputs: &Inputs<'_, '_>,
    handlers: &handlers::BoundHandlers<'_>,
    budget: &mut usize,
) -> Result<Outcome, Error> {
    execute_with_delivery(
        context,
        issuer,
        actor,
        inputs,
        handlers,
        budget,
        Delivery::Request,
    )
}
/// Execute an already delivered UnitOperation SkipTurn message for an active
/// owner. The native Execute body has no Broadcast/RequestTurnUnready phase.
/// Manager, synchronous reports, diplomacy and notification aftermath use the
/// same ongoing owners as ordinary requests. Inactive owners remain unsupported.
pub fn execute_message<P>(
    context: &mut current_aftermath::Context<'_, stock_notifications::Runtime<'_, P>>,
    issuer: i32,
    actor: UnitReference,
    inputs: &Inputs<'_, '_>,
    handlers: &handlers::BoundHandlers<'_>,
    budget: &mut usize,
) -> Result<Outcome, Error> {
    execute_with_delivery(
        context,
        issuer,
        actor,
        inputs,
        handlers,
        budget,
        Delivery::Message,
    )
}
#[derive(Clone, Copy)]
enum Delivery {
    Request,
    Message,
}
fn check_local_request(
    delivery: Delivery,
    local: i32,
    active: impl FnOnce(i32) -> Result<bool, Error>,
) -> Result<(), Error> {
    if matches!(delivery, Delivery::Request) && (local < 0 || !active(local)?) {
        return Err(Error::Unsupported("local player unready continuation"));
    }
    Ok(())
}
fn execute_with_delivery<P>(
    context: &mut current_aftermath::Context<'_, stock_notifications::Runtime<'_, P>>,
    issuer: i32,
    actor: UnitReference,
    inputs: &Inputs<'_, '_>,
    handlers: &handlers::BoundHandlers<'_>,
    budget: &mut usize,
    delivery: Delivery,
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
    check_local_request(delivery, local, |owner| {
        state.turn_active(owner).map_err(Error::State)
    })?;
    let exists = state
        .world()
        .player(issuer)
        .map_err(Error::World)?
        .unit(actor.id)
        .map_err(Error::World)?
        .is_some();
    let events = if exists {
        let backend = &mut context.notifications.backend;
        let mut queries = backend.script.queries::<()>();
        let events = actions::skip_turn_state_phase_observed(
            state,
            actor,
            inputs,
            handlers,
            &mut queries,
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delivered_npc_order_does_not_enter_local_broadcast_unready() {
        // The local human has handed off; a different active owner is acting.
        let mut local_reads = Vec::new();
        assert!(matches!(
            check_local_request(Delivery::Request, 1, |owner| {
                local_reads.push(owner);
                Ok(false)
            }),
            Err(Error::Unsupported("local player unready continuation"))
        ));
        assert_eq!(local_reads, [1]);
        check_local_request(Delivery::Message, 1, |_| {
            panic!("delivered Execute must not enter Broadcast")
        })
        .unwrap();
        check_local_request(Delivery::Request, 1, |_| Ok(true)).unwrap();
    }
}
