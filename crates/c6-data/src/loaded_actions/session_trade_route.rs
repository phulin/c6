//! Explicit domestic Trader order on the persistent Session state/RNG owners.
use super::{Error, Session};
use crate::{
    game_update_state::{round_quests, round_trade},
    loaded_actions::{
        self as actions, handlers, move_to, stock_notifications,
        trade_route_actions::{self as trade, current, stock},
    },
    research,
};
use c6_core::{
    pathfinder::TwoLayer,
    unit::{OperationParameters, PlotCoord},
};

pub type TradeRouteError = move_to::ActionError<
    stock_notifications::movement::Error,
    current::Failure<stock_notifications::movement::Error>,
    actions::notifications::processing::Error,
>;
impl<'a> Session<'_, 'a, OperationParameters> {
    /// The supplied destination is an external player decision. Queue changes,
    /// route IDs, path search, callbacks and the reached Trader step operate on
    /// the same current owners. Errors retain their prefix and block reuse.
    #[allow(clippy::too_many_arguments)]
    pub fn domestic_trade_route(
        &mut self,
        issuer: i32,
        actor: (i32, u32),
        destination: PlotCoord,
        inputs: &actions::Inputs<'_, '_>,
        handlers: &handlers::BoundHandlers<'_>,
        queued: &mut move_to::Runtime<'_>,
        movement: &mut move_to::ComputedMovement<'_, '_>,
        definitions: &trade::Definitions,
        path_definitions: &trade::path::Definitions,
        search: &mut TwoLayer,
        manager: &mut round_trade::Runtime,
        callbacks: &stock::Inputs<'_>,
        research_definitions: &research::Definitions,
        choices: &actions::choose_tech::Definitions,
        culture: Option<&stock_notifications::research::CultureInputs<'_>>,
        quests: &std::cell::RefCell<round_quests::Runtime>,
        reports: &mut dyn stock::Reports<OperationParameters>,
        budget: &mut usize,
    ) -> Result<move_to::ActionOutcome, Error<TradeRouteError>> {
        self.ordinary_player_action(issuer)?;
        let mut callbacks = stock::Current {
            inputs: callbacks,
            research: &mut self.research,
            research_definitions,
            choices,
            culture,
            quests,
            reports,
        };
        let mut movement = current::Current {
            movement,
            definitions,
            path_definitions,
            search,
            manager,
            continuations: &mut callbacks,
        };
        let result = move_to::execute_operation_active_request(
            &mut self.context,
            issuer,
            actor,
            move_to::OperationRequest::MakeTradeRoute { destination },
            inputs,
            handlers,
            queued,
            &mut movement,
            budget,
        );
        self.retain(result, true)
    }
}
