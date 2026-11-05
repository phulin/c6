//! Explicit Pillage request on the persistent Session state and callback owners.
use super::{Error, Session};
use crate::loaded_actions::{self as actions, handlers, move_to, stock_notifications};
use c6_core::unit::OperationParameters;

impl<'a> Session<'_, 'a, OperationParameters> {
    /// Submit one externally selected improvement Pillage order through the
    /// existing queue, callbacks, diplomacy and MessagesProcessed boundary.
    /// The dispatcher must provide the source implementation of CanStart/Start;
    /// missing services fail explicitly and retain any already applied prefix.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub fn pillage<M>(
        &mut self,
        issuer: i32,
        actor: (i32, u32),
        inputs: &actions::Inputs<'_, '_>,
        handlers: &handlers::BoundHandlers<'_>,
        queued: &mut move_to::Runtime<'_>,
        dispatcher: &mut M,
        budget: &mut usize,
    ) -> Result<
        move_to::ActionOutcome,
        Error<
            move_to::ActionError<
                stock_notifications::movement::Error,
                M::Error,
                actions::notifications::processing::Error,
            >,
        >,
    >
    where
        M: move_to::Movement<stock_notifications::Runtime<'a, OperationParameters>>,
        M::Error: std::fmt::Debug,
    {
        self.ordinary_player_action(issuer)?;
        let result = move_to::execute_operation_active_request(
            &mut self.context,
            issuer,
            actor,
            move_to::OperationRequest::Pillage,
            inputs,
            handlers,
            queued,
            dispatcher,
            budget,
        );
        self.retain(result, true)
    }
}
