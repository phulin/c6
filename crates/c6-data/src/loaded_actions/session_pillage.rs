//! Explicit Pillage request on persistent Session state and callback owners.
use super::{Error, Session};
use crate::loaded_actions::{
    self as actions, handlers, move_to, pillage::current, stock_notifications,
};
use c6_core::unit::OperationParameters;
pub type PillageError = move_to::ActionError<
    stock_notifications::movement::Error,
    current::Error,
    actions::notifications::processing::Error,
>;
impl Session<'_, '_, OperationParameters> {
    /// Submit one externally selected improvement Pillage order through the
    /// existing queue, callbacks, diplomacy and MessagesProcessed boundary.
    /// The current Session's visibility history is the Gossip owner. Missing
    /// services fail explicitly and retain any already applied prefix.
    #[allow(clippy::too_many_arguments)]
    pub fn pillage(
        &mut self,
        issuer: i32,
        actor: (i32, u32),
        inputs: &actions::Inputs<'_, '_>,
        handlers: &handlers::BoundHandlers<'_>,
        queued: &mut move_to::Runtime<'_>,
        dispatcher: &mut current::Dispatcher<'_, '_, '_>,
        budget: &mut usize,
    ) -> Result<move_to::ActionOutcome, Error<PillageError>> {
        self.ordinary_player_action(issuer)?;
        let mut current = current::Dispatcher {
            inputs: dispatcher.inputs,
            gossip_history: &self.visibility.history,
            reports: &mut *dispatcher.reports,
        };
        let result = move_to::execute_operation_active_request(
            &mut self.context,
            issuer,
            actor,
            move_to::OperationRequest::Pillage,
            inputs,
            handlers,
            queued,
            &mut current,
            budget,
        );
        self.retain(result, true)
    }
}
