//! Parameterless founding through the persistent Session and stock creation.
use super::{Error, Session};
use crate::loaded_actions::{
    self, handlers, move_to,
    stock_notifications::{self, founding},
};

pub type FoundCityError = move_to::ActionError<
    stock_notifications::movement::Error,
    String,
    loaded_actions::notifications::processing::Error,
>;
impl<'s, 'a> Session<'s, 'a> {
    /// Execute the current Settler's real parameterless FoundCity operation.
    /// Naming, placement, CityCreated, capital/Palace, policies and settler
    /// consumption all retain their existing owners and partial failures.
    #[allow(clippy::too_many_arguments)]
    pub fn found_city<S: founding::Domains<'a>, M>(
        &mut self,
        issuer: i32,
        actor: (i32, u32),
        inputs: &loaded_actions::Inputs<'_, '_>,
        handlers: &handlers::BoundHandlers<'_>,
        runtime: &mut move_to::Runtime<'_>,
        founding: &mut founding::Movement<'_, 'a, S, M>,
        budget: &mut usize,
    ) -> Result<move_to::ActionOutcome, Error<FoundCityError>>
    where
        M: move_to::Movement<stock_notifications::Runtime<'a>>,
        M::Error: std::fmt::Debug,
    {
        self.ordinary_player_action(issuer)?;
        let result = move_to::execute_found_city_active_request(
            &mut self.context,
            issuer,
            actor,
            inputs,
            handlers,
            runtime,
            founding,
            budget,
        );
        if result.is_ok() {
            // Creation appends a City to the live Player roster. Research owns
            // its traversal order across requests, so propagate this successful
            // change before the next research choice or ordinary turn upkeep.
            // Reusing the initialized pre-founding empty list would fail the
            // research yield extent check when the first city activates.
            if let Some(player) = self
                .context
                .notifications
                .state
                .retained()
                .players
                .iter()
                .find(|player| player.header.owner == actor.0)
            {
                let order = player
                    .cities
                    .records
                    .entries
                    .iter()
                    .map(|entry| c6_core::unit::ComponentId {
                        owner: entry.value.header.owner as i16,
                        component_type: 2,
                        id: entry.value.header.id,
                    })
                    .collect();
                if let Some((_, current)) = self
                    .research
                    .as_mut()
                    .ok_or(Error::DecisionBoundary("research owner temporarily loaned"))?
                    .city_order
                    .iter_mut()
                    .find(|(owner, _)| *owner == actor.0)
                {
                    *current = order;
                } else {
                    self.research
                        .as_mut()
                        .ok_or(Error::DecisionBoundary("research owner temporarily loaned"))?
                        .city_order
                        .push((actor.0, order));
                }
            }
        }
        // Reports/script/RNG can advance before a retained-state stamp changes.
        // Keep the same failed-prefix rule as the ordinary operation entry.
        self.retain(result, true)
    }
}
