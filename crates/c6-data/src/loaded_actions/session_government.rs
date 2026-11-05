//! Explicit legal government choices on the persistent Session.
use super::{stock_notifications, Error, Session};
use crate::loaded_actions::{government_actions as action, policy_actions};
use c6_core::{
    government_policy::{self, government},
    loaded_state::{game_update, LoadedState},
};

/// Source-positioned synchronous consumers borrow the Session's existing owners.
pub trait Services<'a, P>: super::policy::Services<'a, P> {
    fn government_event(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
        event: government::Event,
        definitions: &action::Definitions,
        history: &crate::game_update_state::deactivation::VisibilityHistory,
    ) -> Result<(), Self::Error>;
    fn slots_rebuilt(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
    ) -> Result<(), Self::Error>;
    fn government_notifications(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
    ) -> Result<(), Self::Error>;
}
struct Effects<'s, 'a, P, S> {
    backend: &'s mut stock_notifications::Runtime<'a, P>,
    services: &'s mut S,
    history: &'s crate::game_update_state::deactivation::VisibilityHistory,
}
impl<'a, P, S: Services<'a, P>> policy_actions::Effects for Effects<'_, 'a, P, S> {
    type Error = S::Error;
    fn event(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        event: government_policy::Event,
        defs: &policy_actions::Definitions,
    ) -> Result<(), Self::Error> {
        self.services
            .policy_event(state, self.backend, owner, event, defs, self.history)
    }
    fn update_human_processor(&mut self, _: &mut LoadedState, _: i32) -> Result<(), Self::Error> {
        Ok(())
    }
}
impl<'a, P, S: Services<'a, P>> action::Effects for Effects<'_, 'a, P, S> {
    fn government_event(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        event: government::Event,
        defs: &action::Definitions,
    ) -> Result<(), Self::Error> {
        self.services
            .government_event(state, self.backend, owner, event, defs, self.history)
    }
    fn slots_rebuilt(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Self::Error> {
        self.services.slots_rebuilt(state, self.backend, owner)
    }
    fn government_notifications(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
    ) -> Result<(), Self::Error> {
        self.services
            .government_notifications(state, self.backend, owner)
    }
}
impl<'a, P> Session<'_, 'a, P> {
    /// Return current legal choices without selecting a government.
    pub fn available_governments(
        &self,
        owner: i32,
        definitions: &action::Definitions,
        current: &action::Current<'_>,
    ) -> Result<Vec<i32>, Error<action::Error>> {
        self.ordinary_player_action(owner)?;
        action::available(
            self.context.notifications.state,
            definitions,
            current,
            owner,
        )
        .map_err(Error::Action)
    }
    /// Reject an unavailable choice without mutation. Consumer failures retain
    /// the actual source prefix; positive anarchy remains explicitly unsupported.
    #[allow(clippy::too_many_arguments)]
    pub fn request_government<S: Services<'a, P>>(
        &mut self,
        owner: i32,
        government: i32,
        definitions: &action::Definitions,
        current: &action::Current<'_>,
        services: &mut S,
        budget: &mut usize,
    ) -> Result<bool, Error<action::Error<S::Error>>> {
        self.ordinary_player_action(owner)?;
        if !action::can_change(
            self.context.notifications.state,
            definitions,
            current,
            owner,
            government,
        )
        .map_err(Error::Action)?
        {
            return Ok(false);
        }
        let mut effects = Effects {
            backend: &mut self.context.notifications.backend,
            services,
            history: &self.visibility,
        };
        let result = government::set_current(
            &mut action::Context {
                state: self.context.notifications.state,
                owner,
                definitions,
                current,
                effects: &mut effects,
            },
            government,
        );
        let changed = self.retain(result, true)?;
        let human = self
            .context
            .notifications
            .state
            .player_slot_status(owner)
            .map_err(|e| Error::Action(action::Error::State(e)))?;
        if human == 3 {
            let result =
                game_update::command_update::prepare_human(self.context.notifications.state, owner)
                    .map_err(|e| action::Error::Policy(policy_actions::Error::Processor(e)));
            self.retain(result, true)?;
            let result = self
                .context
                .notifications
                .update(owner, budget)
                .map_err(|e| action::Error::Policy(policy_actions::Error::Notifications(e)));
            self.retain(result, true)?;
        }
        let result = self
            .context
            .messages_processed()
            .map(|_| ())
            .map_err(|e| action::Error::Policy(policy_actions::Error::Aftermath(e)));
        self.retain(result, true)?;
        Ok(changed)
    }
}
