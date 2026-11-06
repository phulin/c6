//! External pantheon policy on the persistent state and callback owners.
use super::{stock_notifications, Error, Session};
use crate::loaded_actions::pantheon_actions as action;
use c6_core::loaded_state::LoadedState;
pub trait Services<'a, P> {
    type Error: std::fmt::Debug;
    #[allow(clippy::too_many_arguments)]
    fn event(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        research: &mut crate::research::Runtime,
        owner: i32,
        event: action::Event,
        defs: &action::Definitions,
    ) -> Result<(), Self::Error>;
}
struct Effects<'s, 'a, P, S> {
    backend: &'s mut stock_notifications::Runtime<'a, P>,
    services: &'s mut S,
    research: &'s mut crate::research::Runtime,
}
impl<'a, P, S: Services<'a, P>> action::Effects for Effects<'_, 'a, P, S> {
    type Error = S::Error;
    fn event(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        event: action::Event,
        defs: &action::Definitions,
    ) -> Result<(), Self::Error> {
        self.services
            .event(state, self.backend, self.research, owner, event, defs)
    }
}
impl<'a, P> Session<'_, 'a, P> {
    fn pantheon_action_boundary<E>(&self, owner: i32) -> Result<(), Error<E>> {
        self.check()?;
        if self.pending_choice().is_some_and(|choice| choice.player == owner
            && choice.kind == crate::game_update_state::activation::upkeep::PolicyKind::Pantheon)
            && matches!(self.update.pending_player_effect(), Some(c6_core::game_update::PlayerEffect::Activation {player, ..}) if player == owner)
            && c6_core::loaded_state::game_update::npc_activation::pending(&self.update)
                .is_some_and(|decision| decision.player == owner && decision.kind == c6_core::loaded_state::game_update::npc_activation::DecisionKind::Pantheon)
        { return Ok(()); }
        self.ordinary_player_action(owner)
    }
    pub fn available_pantheons(
        &self,
        owner: i32,
        defs: &action::Definitions,
        current: &action::Current<'_>,
    ) -> Result<Vec<action::Choice>, Error<action::Error>> {
        self.pantheon_action_boundary(owner)?;
        action::available(self.state(), defs, current, owner).map_err(Error::Action)
    }
    /// An invalid choice is rejected without mutation. A reached callback
    /// failure retains the incomplete prefix on the same Session.
    pub fn request_pantheon<S: Services<'a, P>>(
        &mut self,
        owner: i32,
        belief: i32,
        defs: &action::Definitions,
        current: &action::Current<'_>,
        services: &mut S,
    ) -> Result<bool, Error<action::Error<S::Error>>> {
        self.pantheon_action_boundary(owner)?;
        let stamp = self.state().action_stamp();
        let mut effects = Effects {
            backend: &mut self.context.notifications.backend,
            services,
            research: &mut self.research,
        };
        let result = action::found(
            self.context.notifications.state,
            defs,
            current,
            owner,
            belief,
            &mut effects,
        );
        let partial = self.state().action_stamp() != stamp;
        self.retain(result, partial)
    }
}
