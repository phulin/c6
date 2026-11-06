//! Qualified original signal rings over the Session's ongoing modifier owner.
//! Presentation-only publishers may be omitted by concrete continuations; all
//! positive gameplay consumers and RNG effects must be implemented there.
use super::{Current, Definitions, Event};
use crate::loaded_actions::{session, stock_notifications};
use c6_core::loaded_state::LoadedState;
#[derive(Clone, Copy, Debug)]
pub enum Listener {
    GameEffects,
    Continuation(u64),
}
pub trait Continuations<'a, P> {
    type Error: std::fmt::Debug;
    fn callback(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
        event: Event,
        receiver: u64,
        defs: &Definitions,
    ) -> Result<(), Self::Error>;
    fn report(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
        event: Event,
        defs: &Definitions,
    ) -> Result<(), Self::Error>;
}
pub struct Stock<'s, C> {
    pub current: Current<'s>,
    pub belief_chosen: &'s [Listener],
    pub faith_changed: &'s [Listener],
    pub city_changed: &'s [Listener],
    pub founded: &'s [Listener],
    pub continuations: C,
}
#[derive(Debug)]
pub enum Error<E> {
    Continuation(E),
    Modifiers(crate::melee_attack::backend::pantheon::Error),
    Missing(&'static str),
}
impl<'a, P, C: Continuations<'a, P>> session::pantheon::Services<'a, P> for Stock<'_, C> {
    type Error = Error<C::Error>;
    fn event(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
        event: Event,
        defs: &Definitions,
    ) -> Result<(), Self::Error> {
        let listeners = match event {
            Event::BeliefChosen { .. } => self.belief_chosen,
            Event::FaithChanged { .. } => self.faith_changed,
            Event::CityPantheonChanged { .. } => self.city_changed,
            Event::PantheonFounded { .. } => self.founded,
            Event::ReportFounded { .. } | Event::ReportCity { .. } => {
                return self
                    .continuations
                    .report(state, backend, owner, event, defs)
                    .map_err(Error::Continuation)
            }
        };
        if matches!(event, Event::BeliefChosen { .. })
            && listeners
                .iter()
                .filter(|l| matches!(l, Listener::GameEffects))
                .count()
                != 1
        {
            return Err(Error::Missing(
                "qualified BeliefChosen GameEffects subscriber",
            ));
        }
        for listener in listeners {
            match *listener {
                Listener::GameEffects => {
                    let Event::BeliefChosen { belief } = event else {
                        return Err(Error::Missing("unexpected pantheon GME signal"));
                    };
                    backend
                        .with_modifier_owners(|_, owners| {
                            owners.effects.attach_pantheon(
                                state,
                                owner,
                                belief,
                                defs,
                                &self.current,
                            )
                        })
                        .map_err(|_| Error::Missing("canonical pantheon modifier owner"))?
                        .map_err(Error::Modifiers)?;
                }
                Listener::Continuation(receiver) => self
                    .continuations
                    .callback(state, backend, owner, event, receiver, defs)
                    .map_err(Error::Continuation)?,
            }
        }
        if let Event::CityPantheonChanged { city, .. } = event {
            // PlayerHasPantheon initializes and appends its listener during
            // BeliefChosen. Those listeners follow the captured initial ring.
            backend
                .with_modifier_owners(|_, owners| {
                    owners.effects.pantheon_city_changed(state, owner, city)
                })
                .map_err(|_| Error::Missing("canonical pantheon requirement owner"))?
                .map_err(Error::Modifiers)?;
        }
        Ok(())
    }
}
