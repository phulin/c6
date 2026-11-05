//! Explicit externally selected ordinary Repair request on persistent Session.
use super::{Error, Session};
use crate::loaded_actions::{self as actions, handlers, move_to, stock_notifications};
pub type RepairError = move_to::ActionError<
    stock_notifications::movement::Error,
    crate::builder_repair::stock::Error,
    actions::notifications::processing::Error,
>;
impl Session<'_, '_, c6_core::unit::OperationParameters> {
    /// Execute Repair through the same operation queue and stock aftermath.
    /// Errors preserve reached effects; one remaining charge is not consumed.
    #[allow(clippy::too_many_arguments)]
    pub fn repair(
        &mut self,
        issuer: i32,
        actor: (i32, u32),
        inputs: &actions::Inputs<'_, '_>,
        handlers: &handlers::BoundHandlers<'_>,
        queued: &mut move_to::Runtime<'_>,
        dispatcher: &mut crate::builder_repair::stock::Dispatcher<'_, '_>,
        budget: &mut usize,
    ) -> Result<move_to::ActionOutcome, Error<RepairError>> {
        self.ordinary_player_action(issuer)?;
        let result = move_to::execute_operation_active_request(
            &mut self.context,
            issuer,
            actor,
            move_to::OperationRequest::Repair,
            inputs,
            handlers,
            queued,
            dispatcher,
            budget,
        );
        self.retain(result, true)
    }
}
