//! Reuse one caller-owned stock turn composition across Session requests.
//!
//! The required methods are the existing activation and controller boundaries.
//! Implementations call the public upkeep/control compositions with their
//! initialized owners. None has a successful default or selects player policy.
use super::{
    stock_notifications::{self, deactivation, turn_loop},
    Error, Session,
};
use crate::game_update_state::activation::upkeep::{ActivationOutcome, PolicyChoice};
use c6_core::loaded_state::{game_update, LoadedState};
use std::cell::RefCell;

pub trait TurnServices<'a, P = c6_core::unit::OperationParameters> {
    type Error: std::fmt::Debug;
    fn activate(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        update: &mut game_update::Runtime,
        research: &mut crate::research::Runtime,
        owner: i32,
        first: bool,
    ) -> Result<ActivationOutcome, Self::Error>;
    fn inactive(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
    ) -> Result<(), Self::Error>;
    fn active(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        update: &mut game_update::Runtime,
        owner: i32,
    ) -> Result<ActivationOutcome, Self::Error>;
    fn return_active(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        update: &mut game_update::Runtime,
        owner: i32,
    ) -> Result<ActivationOutcome, Self::Error>;
}

/// Adapt an already assembled stock composition without changing its callbacks.
/// Captured mutable owners stay in these closures between all advances.
pub struct Callbacks<A, I, C, R> {
    pub activation: A,
    pub inactive: I,
    pub active: C,
    pub return_active: R,
}
impl<'a, P, E, A, I, C, R> TurnServices<'a, P> for Callbacks<A, I, C, R>
where
    E: std::fmt::Debug,
    A: FnMut(
        &mut LoadedState,
        &mut stock_notifications::Runtime<'a, P>,
        &mut game_update::Runtime,
        &mut crate::research::Runtime,
        i32,
        bool,
    ) -> Result<ActivationOutcome, E>,
    I: FnMut(&mut LoadedState, &mut stock_notifications::Runtime<'a, P>, i32) -> Result<(), E>,
    C: FnMut(
        &mut LoadedState,
        &mut stock_notifications::Runtime<'a, P>,
        &mut game_update::Runtime,
        i32,
    ) -> Result<ActivationOutcome, E>,
    R: FnMut(
        &mut LoadedState,
        &mut stock_notifications::Runtime<'a, P>,
        &mut game_update::Runtime,
        i32,
    ) -> Result<ActivationOutcome, E>,
{
    type Error = E;
    fn activate(
        &mut self,
        s: &mut LoadedState,
        b: &mut stock_notifications::Runtime<'a, P>,
        u: &mut game_update::Runtime,
        r: &mut crate::research::Runtime,
        p: i32,
        f: bool,
    ) -> Result<ActivationOutcome, E> {
        (self.activation)(s, b, u, r, p, f)
    }
    fn inactive(
        &mut self,
        s: &mut LoadedState,
        b: &mut stock_notifications::Runtime<'a, P>,
        p: i32,
    ) -> Result<(), E> {
        (self.inactive)(s, b, p)
    }
    fn active(
        &mut self,
        s: &mut LoadedState,
        b: &mut stock_notifications::Runtime<'a, P>,
        u: &mut game_update::Runtime,
        p: i32,
    ) -> Result<ActivationOutcome, E> {
        (self.active)(s, b, u, p)
    }
    fn return_active(
        &mut self,
        s: &mut LoadedState,
        b: &mut stock_notifications::Runtime<'a, P>,
        u: &mut game_update::Runtime,
        p: i32,
    ) -> Result<ActivationOutcome, E> {
        (self.return_active)(s, b, u, p)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Request {
    /// An explicit player decision, never issued automatically at a choice.
    EndTurn { player: i32, reason: u32 },
    /// Continue an existing suspended turn after the caller resolves its choice.
    Resume,
}

/// Borrowed decisions from the same live backend. These are semantic pending
/// requests, not native notification pool objects or a complete legal-action list.
pub struct Decisions<'s> {
    pub policy: Option<&'s PolicyChoice>,
    pub notifications: &'s [stock_notifications::Decision],
}
impl<'a, P> Session<'_, 'a, P> {
    pub fn decisions(&self) -> Decisions<'_> {
        Decisions {
            policy: self.pending_choice(),
            notifications: &self.context.notifications.backend.decisions,
        }
    }
    /// Advance using one retained services owner. TurnLoop calls the callbacks
    /// synchronously in native order; the short loans below never clone owners.
    /// Errors use the existing partial-prefix retention rules on this Session.
    pub fn advance<S: TurnServices<'a, P>>(
        &mut self,
        request: Request,
        inputs: &deactivation::Inputs<'_, '_, '_>,
        services: &mut S,
        budget: &mut usize,
    ) -> Result<turn_loop::Outcome, Error<turn_loop::Error<S::Error>>> {
        let services = RefCell::new(services);
        let activation =
            |s: &mut LoadedState,
             b: &mut stock_notifications::Runtime<'a, P>,
             u: &mut game_update::Runtime,
             r: &mut crate::research::Runtime,
             p,
             f| services.borrow_mut().activate(s, b, u, r, p, f);
        let inactive = |s: &mut LoadedState, b: &mut stock_notifications::Runtime<'a, P>, p| {
            services.borrow_mut().inactive(s, b, p)
        };
        let active = |s: &mut LoadedState,
                      b: &mut stock_notifications::Runtime<'a, P>,
                      u: &mut game_update::Runtime,
                      p| services.borrow_mut().active(s, b, u, p);
        match request {
            Request::EndTurn { player, reason } => {
                self.end_turn(inputs, player, reason, budget, activation, inactive, active)
            }
            Request::Resume => self.resume_end_turn(
                inputs,
                budget,
                activation,
                inactive,
                active,
                |s, b, u, p| services.borrow_mut().return_active(s, b, u, p),
            ),
        }
    }
    /// Resolve the currently suspended research choice, then continue with the
    /// same turn services. Illegal choices keep the pending decision intact.
    #[allow(clippy::too_many_arguments)]
    pub fn advance_research<S: TurnServices<'a, P>>(
        &mut self,
        inputs: &deactivation::Inputs<'_, '_, '_>,
        services: &mut S,
        budget: &mut usize,
        definitions: &crate::research::Definitions,
        choices: &crate::loaded_actions::choose_tech::Definitions,
        culture: Option<&stock_notifications::research::CultureInputs<'_>>,
        index: i32,
    ) -> Result<turn_loop::Outcome, Error<turn_loop::Error<S::Error>>> {
        let services = RefCell::new(services);
        self.resume_research(
            inputs,
            budget,
            definitions,
            choices,
            culture,
            index,
            |s, b, u, r, p, f| services.borrow_mut().activate(s, b, u, r, p, f),
            |s, b, p| services.borrow_mut().inactive(s, b, p),
            |s, b, u, p| services.borrow_mut().active(s, b, u, p),
        )
    }
}
