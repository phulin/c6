//! Ordered stock GovernmentEnacted/GovernmentRetracted routing.
//! Initializers qualify native callbacks once and supply their typed captures.
use super::{history, modifiers, requirements, Definitions};
use crate::loaded_actions::{policy_actions, session, stock_notifications};
use c6_core::{government_policy::government::Event, loaded_state::LoadedState};

#[derive(Clone, Copy, Debug)]
pub enum Listener {
    GameEffects,
    Gossip,
    History(history::Tiers),
    Boost { row_id: i32 },
    PlayerEvent { owner: i32 },
    Requirement(requirements::Receiver),
    Achievement,
}
/// Concrete continuations borrow the same research, history, requirements and
/// report services as the enclosing Session. Positive branches may not be
/// replaced with a successful default implementation.
pub trait Continuations<'a, P> {
    type Error: std::fmt::Debug;
    fn gossip(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
        government: i32,
        history: &crate::game_update_state::deactivation::VisibilityHistory,
    ) -> Result<(), Self::Error>;
    fn history(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
        government: i32,
        tiers: history::Tiers,
    ) -> Result<(), Self::Error>;
    fn boost(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
        government: i32,
        row_id: i32,
    ) -> Result<(), Self::Error>;
    fn player_event(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        receiver_owner: i32,
        owner: i32,
        government: i32,
        enacted: bool,
    ) -> Result<(), Self::Error>;
    fn requirement(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        receiver: requirements::Receiver,
        owner: i32,
    ) -> Result<(), Self::Error>;
    fn slots_rebuilt(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
    ) -> Result<(), Self::Error>;
    fn notifications(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
    ) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub enum Error<E> {
    Policy(policy_actions::stock::Error),
    Modifiers(modifiers::EventError),
    MissingGovernment(i32),
    Continuation(E),
}
pub struct Stock<'s, C> {
    pub policy: policy_actions::stock::Stock<'s>,
    pub enacted: &'s [Listener],
    pub retracted: &'s [Listener],
    pub modifiers: &'s modifiers::Definitions,
    pub continuations: C,
}
struct YieldReports;
impl crate::city_production_turn::modifier_effects::Effects for YieldReports {
    fn report(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
        _: crate::city_production_turn::modifier_effects::Report,
    ) -> Result<(), crate::city_production_turn::Error> {
        // Stock ef1e2348 is a numeric/read-only cache publication. Current
        // gameplay queries read the mutated canonical City yield changes.
        Ok(())
    }
}
impl<'a, P, C: Continuations<'a, P>> session::policy::Services<'a, P> for Stock<'_, C> {
    type Error = Error<C::Error>;
    fn policy_event(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
        event: c6_core::government_policy::Event,
        definitions: &policy_actions::Definitions,
        history: &crate::game_update_state::deactivation::VisibilityHistory,
    ) -> Result<(), Self::Error> {
        self.policy
            .policy_event(state, backend, owner, event, definitions, history)
            .map_err(Error::Policy)
    }
}
impl<'a, P, C: Continuations<'a, P>> session::government::Services<'a, P> for Stock<'_, C> {
    fn government_event(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
        event: Event,
        definitions: &Definitions,
        history: &crate::game_update_state::deactivation::VisibilityHistory,
    ) -> Result<(), Self::Error> {
        let (government, enacted, listeners) = match event {
            Event::Retracted(g) => (g, false, self.retracted),
            Event::Enacted(g) => (g, true, self.enacted),
            // 90fb33e4 only publishes the already mutated Player Culture cache.
            Event::Changed(_) => return Ok(()),
        };
        for listener in listeners {
            match *listener {
                Listener::GameEffects => backend
                    .government_modifier_event(
                        state,
                        owner,
                        government,
                        definitions
                            .government_hash(government)
                            .ok_or(Error::MissingGovernment(government))?,
                        enacted,
                        self.modifiers,
                        &mut YieldReports,
                    )
                    .map_err(Error::Modifiers)?,
                Listener::Gossip => self
                    .continuations
                    .gossip(state, backend, owner, government, history)
                    .map_err(Error::Continuation)?,
                Listener::History(tiers) => self
                    .continuations
                    .history(state, backend, owner, government, tiers)
                    .map_err(Error::Continuation)?,
                Listener::Boost { row_id } => self
                    .continuations
                    .boost(state, backend, owner, government, row_id)
                    .map_err(Error::Continuation)?,
                Listener::PlayerEvent {
                    owner: receiver_owner,
                } => self
                    .continuations
                    .player_event(state, backend, receiver_owner, owner, government, enacted)
                    .map_err(Error::Continuation)?,
                Listener::Requirement(receiver) => self
                    .continuations
                    .requirement(state, backend, receiver, owner)
                    .map_err(Error::Continuation)?,
                // 28f9190 submits external achievements after static checks;
                // it does not mutate rules state or consume Game RNG.
                Listener::Achievement => {}
            }
        }
        Ok(())
    }
    fn slots_rebuilt(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
    ) -> Result<(), Self::Error> {
        self.continuations
            .slots_rebuilt(state, backend, owner)
            .map_err(Error::Continuation)
    }
    fn government_notifications(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
    ) -> Result<(), Self::Error> {
        self.continuations
            .notifications(state, backend, owner)
            .map_err(Error::Continuation)
    }
}
