//! Explicit City gold purchase on the same persistent Session owners.
use super::{Error, Session};
use crate::{
    city_production_turn::unit_selection,
    loaded_actions::{
        purchase_actions::{self, stock, Definitions, Outcome},
        stock_notifications::city_production::unit::Factory,
    },
};
impl<'a, P> Session<'_, 'a, P> {
    /// Query current gold cost without spending gold or creating a Unit.
    pub fn unit_purchase_gold_cost(
        &self,
        owner: i32,
        city: u32,
        unit: u32,
        formation: i32,
        definitions: &Definitions,
    ) -> Result<i32, Error<purchase_actions::Error<()>>> {
        self.ordinary_player_action(owner)?;
        purchase_actions::unit_gold_cost(
            self.context.notifications.state,
            definitions,
            owner,
            city,
            unit,
            formation,
        )
        .map_err(Error::Action)
    }
    /// Apply one explicit gold request. Illegal requests return Rejected without
    /// changing Treasury/rosters. Callback failures retain their actual prefix.
    #[allow(clippy::too_many_arguments)]
    pub fn purchase_unit_gold(
        &mut self,
        owner: i32,
        city: u32,
        unit: u32,
        formation: i32,
        definitions: &Definitions,
        current: &unit_selection::Current<'_>,
        factory: Factory<'_, '_, '_>,
        reports: &mut Vec<stock::Report>,
    ) -> Result<Outcome, Error<purchase_actions::Error<String>>> {
        self.ordinary_player_action(owner)?;
        let mut effects = stock::Current {
            backend: &mut self.context.notifications.backend,
            factory,
            reports,
        };
        let result = purchase_actions::purchase_unit(
            self.context.notifications.state,
            definitions,
            current,
            owner,
            city,
            unit,
            formation,
            &mut effects,
        );
        let partial = matches!(&result, Err(purchase_actions::Error::Effect(_)));
        self.retain(result, partial)
    }
}
