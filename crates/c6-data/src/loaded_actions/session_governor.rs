//! Explicit governor decisions at the ordinary external-player boundary.
use super::{Error, Session};
use crate::{
    found_city::cultural_identity as identity,
    loaded_actions::governor_actions::{self as action, stock, Decision, Definitions},
};
use c6_core::unit::ComponentId;
impl<P> Session<'_, '_, P> {
    pub fn pending_governor_decision(
        &self,
        owner: i32,
        definitions: &Definitions,
    ) -> Result<Option<Decision>, Error<action::Error>> {
        self.ordinary_player_action(owner)?;
        action::pending(self.state(), definitions, owner).map_err(Error::Action)
    }
    pub fn appoint_governor(
        &mut self,
        owner: i32,
        governor: u32,
        definitions: &Definitions,
        scope: stock::Scope,
    ) -> Result<bool, Error<action::Error>> {
        self.ordinary_player_action(owner)?;
        let result = action::appoint(
            self.context.notifications.state,
            definitions,
            owner,
            governor,
            &mut stock::Current {
                backend: &mut self.context.notifications.backend,
                scope,
            },
        );
        let partial = matches!(
            &result,
            Err(action::Error::Effect(_)) | Err(action::Error::Allocation(_))
        );
        self.retain(result, partial)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn assign_governor(
        &mut self,
        owner: i32,
        governor: u32,
        city: ComponentId,
        definitions: &Definitions,
        identity: &identity::Definitions,
        queries: &mut impl identity::Context,
        scope: stock::Scope,
    ) -> Result<bool, Error<action::Error>> {
        self.ordinary_player_action(owner)?;
        let result = action::assign(
            self.context.notifications.state,
            definitions,
            owner,
            governor,
            city,
            &mut stock::Assignment {
                current: stock::Current {
                    backend: &mut self.context.notifications.backend,
                    scope,
                },
                identity,
                queries,
            },
        );
        let partial = matches!(&result, Err(action::Error::Effect(_)));
        self.retain(result, partial)
    }
    /// Defer an available title explicitly. This never selects a native policy.
    pub fn defer_governor_title(
        &mut self,
        owner: i32,
    ) -> Result<(), Error<c6_core::loaded_state::Error>> {
        self.ordinary_player_action(owner)?;
        self.context
            .notifications
            .state
            .set_governor_title_considered(owner, true)
            .map_err(Error::Action)
    }
}
