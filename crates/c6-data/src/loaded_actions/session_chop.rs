//! Explicit externally selected ordinary forest removal request on persistent Session.
use super::{Error, Session};
use crate::loaded_actions::{self as actions, handlers, move_to, stock_notifications};
pub type ChopError = move_to::ActionError<
    stock_notifications::movement::Error,
    crate::builder_chop::stock::Error,
    actions::notifications::processing::Error,
>;
impl Session<'_, '_, c6_core::unit::OperationParameters> {
    /// Execute Chop through the same operation queue and stock aftermath.
    /// Errors preserve reached effects; the selected Builder charge is consumed.
    #[allow(clippy::too_many_arguments)]
    pub fn chop(
        &mut self,
        issuer: i32,
        actor: (i32, u32),
        inputs: &actions::Inputs<'_, '_>,
        handlers: &handlers::BoundHandlers<'_>,
        queued: &mut move_to::Runtime<'_>,
        dispatcher: &mut crate::builder_chop::stock::Dispatcher<'_, '_>,
        budget: &mut usize,
    ) -> Result<move_to::ActionOutcome, Error<ChopError>> {
        self.ordinary_player_action(issuer)?;
        let result = move_to::execute_operation_active_request(
            &mut self.context,
            issuer,
            actor,
            move_to::OperationRequest::RemoveFeature,
            inputs,
            handlers,
            queued,
            dispatcher,
            budget,
        );
        self.retain(result, true)
    }
}
