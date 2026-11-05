//! Ordinary melee and ranged operations through the persistent stock action boundary.
use super::{Error, Session};
use crate::loaded_actions::{self, handlers, move_to, stock_notifications};
pub type RangedError<M> = move_to::ActionError<
    stock_notifications::movement::Error,
    M,
    loaded_actions::notifications::processing::Error,
>;
pub type MeleeError<M> = RangedError<M>;
impl<'s, 'a> Session<'s, 'a> {
    /// Execute qualified ordinary ranged Unit/Unit combat and its mandatory
    /// stock aftermath using current state, script, sight and modifier owners.
    /// Unsupported branches retain their reached effects and freeze the session.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub fn range_attack<M>(
        &mut self,
        issuer: i32,
        actor: (i32, u32),
        request: move_to::Request,
        inputs: &loaded_actions::Inputs<'_, '_>,
        handlers: &handlers::BoundHandlers<'_>,
        runtime: &mut move_to::Runtime<'_>,
        movement: &mut M,
        budget: &mut usize,
    ) -> Result<move_to::ActionOutcome, Error<RangedError<M::Error>>>
    where
        M: move_to::Movement<stock_notifications::Runtime<'a>>,
        M::Error: std::fmt::Debug,
    {
        self.ordinary_player_action(issuer)?;
        let result = move_to::execute_range_active_request(
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
    /// Execute qualified ordinary melee Unit/Unit combat and its mandatory
    /// stock aftermath using current state, script, sight and modifier owners.
    /// Unsupported branches retain their reached effects and freeze the session.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub fn melee_attack<M>(
        &mut self,
        issuer: i32,
        actor: (i32, u32),
        request: move_to::Request,
        inputs: &loaded_actions::Inputs<'_, '_>,
        handlers: &handlers::BoundHandlers<'_>,
        runtime: &mut move_to::Runtime<'_>,
        movement: &mut M,
        budget: &mut usize,
    ) -> Result<move_to::ActionOutcome, Error<MeleeError<M::Error>>>
    where
        M: move_to::Movement<stock_notifications::Runtime<'a>>,
        M::Error: std::fmt::Debug,
    {
        self.ordinary_player_action(issuer)?;
        let result = move_to::execute_active_request(
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
}
