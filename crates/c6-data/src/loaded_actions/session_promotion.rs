//! Explicit promotion selection at the ordinary external-player decision boundary.
use super::{Error, Session};
use crate::loaded_actions::{
    promotion_actions::{self, stock, Definitions, Outcome},
    stock_notifications::city_production::unit::Factory,
};
impl<'a, P> Session<'_, 'a, P> {
    pub fn unit_can_earn_promotion(
        &self,
        owner: i32,
        unit: u32,
        promotion: u32,
        definitions: &Definitions,
    ) -> Result<bool, Error<promotion_actions::Error<()>>> {
        self.ordinary_player_action(owner)?;
        promotion_actions::can_earn(self.state(), definitions, (owner, unit), promotion)
            .map_err(Error::Action)
    }
    /// The current ordinary player may choose any returned promotion hash.
    /// Eligibility, turn, phase, movement and pending-operation guards apply.
    pub fn unit_promotion_choices(
        &mut self,
        owner: i32,
        unit: u32,
        definitions: &Definitions,
        factory: Factory<'_, '_, '_>,
        inputs: &mut stock::Inputs<'_, '_>,
    ) -> Result<Vec<u32>, Error<promotion_actions::Error<String>>> {
        self.ordinary_player_action(owner)?;
        let effects = stock::Current {
            backend: &mut self.context.notifications.backend,
            factory,
            inputs,
            definitions,
        };
        promotion_actions::choices(
            self.context.notifications.state,
            definitions,
            (owner, unit),
            &effects,
        )
        .map_err(Error::Action)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn promote_unit(
        &mut self,
        owner: i32,
        unit: u32,
        promotion: u32,
        definitions: &Definitions,
        factory: Factory<'_, '_, '_>,
        inputs: &mut stock::Inputs<'_, '_>,
    ) -> Result<Outcome, Error<promotion_actions::Error<String>>> {
        self.ordinary_player_action(owner)?;
        let mut effects = stock::Current {
            backend: &mut self.context.notifications.backend,
            factory,
            inputs,
            definitions,
        };
        let result = promotion_actions::promote(
            self.context.notifications.state,
            definitions,
            (owner, unit),
            promotion,
            &mut effects,
        );
        let partial = matches!(
            &result,
            Err(promotion_actions::Error::Effect(_))
                | Err(promotion_actions::Error::AfterMutation(_))
        );
        self.retain(result, partial)
    }
}
