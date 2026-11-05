//! Explicit Unit upgrade requests on the same persistent Session owners.
use super::{Error, Session};
use crate::loaded_actions::{
    stock_notifications::city_production::unit::Factory,
    upgrade_actions::{self, stock, Definitions, Outcome},
};

impl<'a, P> Session<'_, 'a, P> {
    pub fn unit_upgrade_target(
        &self,
        owner: i32,
        unit: u32,
        definitions: &Definitions,
        maps: &crate::unit_creation::signals::BuildMaps,
    ) -> Result<Option<u32>, Error<upgrade_actions::Error<()>>> {
        self.ordinary_player_action(owner)?;
        upgrade_actions::target(self.state(), definitions, maps, (owner, unit))
            .map_err(Error::Action)
    }
    pub fn unit_upgrade_gold_cost(
        &self,
        owner: i32,
        unit: u32,
        target: u32,
        definitions: &Definitions,
    ) -> Result<i32, Error<upgrade_actions::Error<()>>> {
        self.ordinary_player_action(owner)?;
        upgrade_actions::gold_cost(self.state(), definitions, (owner, unit), target)
            .map_err(Error::Action)
    }
    pub fn unit_upgrade_resource_cost(
        &self,
        owner: i32,
        unit: u32,
        target: u32,
        definitions: &Definitions,
    ) -> Result<i32, Error<upgrade_actions::Error<()>>> {
        self.ordinary_player_action(owner)?;
        upgrade_actions::resource_cost(self.state(), definitions, (owner, unit), target)
            .map_err(Error::Action)
    }
    /// Apply one explicit upgrade using the same state, Script, RNG, modifier
    /// and requirement owners as previous actions. A reached callback failure
    /// retains its prefix and prevents issuing another action on this Session.
    #[allow(clippy::too_many_arguments)]
    pub fn upgrade_unit(
        &mut self,
        owner: i32,
        unit: u32,
        definitions: &Definitions,
        maps: &crate::unit_creation::signals::BuildMaps,
        factory: Factory<'_, '_, '_>,
        death: &crate::builder_improvement::death::Inputs<'_, '_>,
        transfer: &mut upgrade_actions::transfer::Inputs<'_>,
        resources: &upgrade_actions::resources::Inputs<'_>,
        reports: &mut Vec<stock::Report>,
    ) -> Result<Outcome, Error<upgrade_actions::Error<String>>> {
        self.ordinary_player_action(owner)?;
        let mut effects = stock::Current {
            backend: &mut self.context.notifications.backend,
            factory,
            death,
            transfer,
            resources,
            reports,
        };
        let result = upgrade_actions::upgrade(
            self.context.notifications.state,
            definitions,
            maps,
            (owner, unit),
            &mut effects,
        );
        let partial = matches!(
            &result,
            Err(upgrade_actions::Error::Effect(_))
                | Err(upgrade_actions::Error::Incomplete(_))
                | Err(upgrade_actions::Error::AfterMutation(_))
        );
        self.retain(result, partial)
    }
}
