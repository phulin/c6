//! Ordinary explicit policy commands on the persistent Session owners.
use super::{stock_notifications, Error, Session};
use crate::loaded_actions::policy_actions::{self, Current, Definitions};
use c6_core::{
    government_policy::{self as kernel, Command, Event},
    loaded_state::{game_update, LoadedState},
};

/// Synchronous policy signals and the existing Player.Processor.Update tail.
/// Providers borrow the Session's real owners; no runtime is reconstructed.
pub trait Services<'a, P> {
    type Error: std::fmt::Debug;
    fn policy_event(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
        event: Event,
        definitions: &Definitions,
        history: &crate::game_update_state::deactivation::VisibilityHistory,
    ) -> Result<(), Self::Error>;
}
struct Effects<'s, 'a, P, S> {
    backend: &'s mut stock_notifications::Runtime<'a, P>,
    services: &'s mut S,
    refresh_human: bool,
    history: &'s crate::game_update_state::deactivation::VisibilityHistory,
}
impl<'a, P, S: Services<'a, P>> policy_actions::Effects for Effects<'_, 'a, P, S> {
    type Error = S::Error;
    fn event(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        event: Event,
        defs: &Definitions,
    ) -> Result<(), Self::Error> {
        self.services
            .policy_event(state, self.backend, owner, event, defs, self.history)
    }
    fn update_human_processor(&mut self, _: &mut LoadedState, _: i32) -> Result<(), Self::Error> {
        self.refresh_human = true;
        Ok(())
    }
}
impl<'a, P> Session<'_, 'a, P> {
    /// Execute the externally supplied clear/insert order. Invalid single
    /// commands retain native SetPolicyChangeMade behavior; legal-action
    /// callers can query policy_actions::can_slot before issuing a command.
    /// Every consumer error preserves the actual partial prefix on this Session.
    pub fn request_policies<S: Services<'a, P>>(
        &mut self,
        owner: i32,
        command: Command<'_>,
        definitions: &Definitions,
        current: &Current<'_>,
        services: &mut S,
        budget: &mut usize,
    ) -> Result<(), Error<policy_actions::Error<S::Error>>> {
        self.ordinary_player_action(owner)?;
        let mut effects = Effects {
            backend: &mut self.context.notifications.backend,
            services,
            refresh_human: false,
            history: &self.visibility,
        };
        let result = kernel::execute(
            &mut policy_actions::Context {
                state: self.context.notifications.state,
                owner,
                definitions,
                current,
                effects: &mut effects,
            },
            command,
        );
        let refresh = effects.refresh_human;
        self.retain(result, true)?;
        if refresh {
            let result =
                game_update::command_update::prepare_human(self.context.notifications.state, owner)
                    .map_err(policy_actions::Error::Processor);
            self.retain(result, true)?;
            let result = self
                .context
                .notifications
                .update(owner, budget)
                .map_err(policy_actions::Error::Notifications);
            self.retain(result, true)?;
        }
        let result = self
            .context
            .messages_processed()
            .map(|_| ())
            .map_err(policy_actions::Error::Aftermath);
        self.retain(result, true)
    }
}
