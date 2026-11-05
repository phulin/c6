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

impl<'a, P> Session<'_, 'a, P> {
    #[allow(clippy::too_many_arguments)]
    pub fn building_purchase_gold_cost(
        &self,
        owner: i32,
        city: u32,
        building: u32,
        definitions: &Definitions,
        construction: &crate::city_production_turn::buildings::Definitions,
    ) -> Result<i32, Error<purchase_actions::Error<()>>> {
        self.ordinary_player_action(owner)?;
        purchase_actions::buildings::gold_cost(
            self.state(),
            definitions,
            construction,
            owner,
            city,
            building,
        )
        .map_err(Error::Action)
    }
    /// Ordinary empty-queue gold purchase. Building callbacks use the same
    /// mutable backend and canonical requirement/modifier owner as city turns.
    #[allow(clippy::too_many_arguments)]
    pub fn purchase_building_gold<Q: crate::city_production_turn::selection::Queries>(
        &mut self,
        owner: i32,
        city: u32,
        building: u32,
        definitions: &Definitions,
        construction: &crate::city_production_turn::buildings::Definitions,
        choices: &crate::city_production_turn::selection::Definitions,
        current: &crate::city_production_turn::selection::Current<'_>,
        queries: &Q,
        appeal: &crate::city_production_turn::appeal::Definitions,
        completion:crate::loaded_actions::stock_notifications::city_production::completion::Services<'_>,
        production_reports: &mut Vec<
            crate::loaded_actions::stock_notifications::city_production::Record,
        >,
        reports: &mut Vec<stock::Report>,
    ) -> Result<purchase_actions::buildings::Outcome, Error<purchase_actions::Error<String>>> {
        self.ordinary_player_action(owner)?;
        // This is the Session's sole live Gossip history owner, including
        // records written by prior turns/actions on the same Session.
        let completion =
            crate::loaded_actions::stock_notifications::city_production::completion::Services {
                gossip_history: &self.visibility.history,
                ..completion
            };
        let mut effects = stock::BuildingCurrent {
            backend: &mut self.context.notifications.backend,
            definitions: construction,
            appeal,
            completion,
            production_reports,
            reports,
        };
        let result = purchase_actions::buildings::purchase(
            self.context.notifications.state,
            definitions,
            construction,
            choices,
            current,
            queries,
            owner,
            city,
            building,
            &mut effects,
        );
        let partial = matches!(&result, Err(purchase_actions::Error::Effect(_)));
        self.retain(result, partial)
    }
}
