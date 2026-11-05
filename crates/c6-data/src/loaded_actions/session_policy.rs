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
    ) -> Result<(), Self::Error>;
    fn processor_update(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        update: &mut game_update::Runtime,
        research: &mut crate::research::Runtime,
        owner: i32,
    ) -> Result<(), Self::Error>;
}
struct Effects<'s, 'a, P, S> {
    backend: &'s mut stock_notifications::Runtime<'a, P>,
    update: &'s mut game_update::Runtime,
    research: &'s mut crate::research::Runtime,
    services: &'s mut S,
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
            .policy_event(state, self.backend, owner, event, defs)
    }
    fn update_human_processor(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
    ) -> Result<(), Self::Error> {
        self.services
            .processor_update(state, self.backend, self.update, self.research, owner)
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
    ) -> Result<(), Error<policy_actions::Error<S::Error>>> {
        self.ordinary_player_action(owner)?;
        let mut effects = Effects {
            backend: &mut self.context.notifications.backend,
            update: &mut self.update,
            research: &mut self.research,
            services,
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
        self.retain(result, true)
    }
}
