//! Construct a Session from caller-initialized typed owners.
//!
//! This is an assembly boundary, not a game initializer or save loader. Bind
//! the required stock gameplay services on `backend` before moving it here.
//! Absent services remain absent and fail at their existing reached boundary.
use super::{stock_notifications, Session};
use crate::{
    game_update_state::deactivation::VisibilityHistory, loaded_actions::notifications::processing,
};
use c6_core::{
    loaded_state::{game_update, LoadedState},
    player_notifications_owner::OccurrenceLimits,
};

/// All mutable runtime owners are moved exactly once. Definitions and query
/// providers borrow the caller's rules and initialized services; the canonical
/// state remains exclusively borrowed for this Session's lifetime.
pub struct Initialized<'s, 'a, P = c6_core::unit::OperationParameters> {
    pub backend: stock_notifications::Runtime<'a, P>,
    pub notifications: processing::Inputs<'s>,
    pub occurrence_limits: OccurrenceLimits,
    pub update: game_update::Runtime,
    pub research: crate::research::Runtime,
    pub visibility: VisibilityHistory,
}

#[derive(Debug)]
pub enum Error {
    ResearchIdentity,
    UpdateIdentity,
    Backend(processing::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Session initialization: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Failed assembly returns every supplied owner and the original mutable state
/// borrow. In particular, a binding error never drops the script/RNG runtime.
pub struct Rejected<'s, 'a, P> {
    pub error: Error,
    pub state: &'s mut LoadedState,
    pub initialized: Initialized<'s, 'a, P>,
}
impl<'s, 'a, P> Session<'s, 'a, P> {
    /// Join the existing notification provider, aftermath and persistent Session
    /// without snapshots, defaults or repeated action-time construction.
    pub fn from_initialized(
        state: &'s mut LoadedState,
        initialized: Initialized<'s, 'a, P>,
    ) -> Result<Self, Box<Rejected<'s, 'a, P>>> {
        let error = if !initialized.update.matches(state) {
            Some(Error::UpdateIdentity)
        } else if &initialized.research.binding != state.binding() {
            Some(Error::ResearchIdentity)
        } else {
            initialized.backend.check(state).err().map(Error::Backend)
        };
        if let Some(error) = error {
            return Err(Box::new(Rejected {
                error,
                state,
                initialized,
            }));
        }
        let Initialized {
            backend,
            notifications,
            occurrence_limits,
            update,
            research,
            visibility,
        } = initialized;
        // The check above is the same identity check used by into_current. Build
        // its existing provider directly so no owned runtime is lost on error.
        let notifications = processing::Provider::new(
            state,
            notifications,
            backend,
            occurrence_limits,
            processing::Scope::SourceInitializedStockXp2,
        );
        Ok(Self::new(
            super::current_aftermath::Context::new(notifications),
            update,
            research,
            visibility,
        ))
    }
}
