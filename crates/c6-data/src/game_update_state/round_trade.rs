//! Linux XP2 Trade.Manager.DoTurn2c05960.
//!
//! Temporary routes are distinct from the canonical LoadedState trade graph.
//! DoTurn clears only this manager vector, then dispatches Game.Signals+0x13b0 when
//! its old count is nonzero. It never invalidates or rebuilds Graph paths.
use c6_core::{
    loaded_state::{ActionStamp, LoadedState},
    trade::Route,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Missing(&'static str),
    Requires(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "round trade: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Current temporary-route owner, initialized from the same BEFORE as state.
/// These are not active Graph routes or a second copy of its edges/paths.
/// Route fields omitted by the wire projection must be supplied by any future
/// operation that observes them; this operation destroys the entire records.
#[derive(Debug)]
pub struct Runtime {
    instance: ActionStamp,
    temporary_routes: Vec<Route>,
    next_route_id: Option<i32>,
}
impl Runtime {
    pub fn from_initial(state: &LoadedState, temporary_routes: Vec<Route>) -> Self {
        Self {
            instance: state.action_stamp(),
            temporary_routes,
            next_route_id: None,
        }
    }
    /// Bind the actual unsaved manager route counter from the same initial
    /// state. It is independent of Graph.total_edges_created and Route IDs.
    pub fn from_initial_with_route_id(
        state: &LoadedState,
        temporary_routes: Vec<Route>,
        next_route_id: i32,
    ) -> Self {
        let mut runtime = Self::from_initial(state, temporary_routes);
        runtime.next_route_id = Some(next_route_id);
        runtime
    }
    pub fn next_route_id(&self) -> Option<i32> {
        self.next_route_id
    }
    pub(crate) fn allocate_route_id(&mut self) -> Result<i32, Error> {
        let value = self
            .next_route_id
            .as_mut()
            .ok_or(Error::Missing("Trade next route ID"))?;
        let id = *value;
        *value = value.wrapping_add(1);
        Ok(id)
    }
    pub fn temporary_routes(&self) -> &[Route] {
        &self.temporary_routes
    }
    pub fn validate(&self, state: &LoadedState) -> Result<(), Error> {
        if !self.instance.same_instance(state.action_stamp()) {
            return Err(Error::Missing("Trade manager instance"));
        }
        Ok(())
    }
}

/// Game.Signals+0x13b0 dispatch after clearing nonempty temporary routes.
/// Implementations must execute the actual ordered subscriber effects and RNG,
/// or return Requires. There is no successful default callback.
pub trait Effects {
    fn temporary_routes_cleared(&mut self, state: &mut LoadedState) -> Result<(), Error>;
}

/// ClearTempRoutes2c05990 destroys records in reverse order and returns count.
/// Route::~Route2c117a0 destroys its two yield vectors; no gameplay/RNG call is
/// made before the conditional signal. An effect failure retains the cleared
/// manager prefix, just as other incomplete round transitions retain writes.
pub fn run<E: Effects>(
    state: &mut LoadedState,
    runtime: &mut Runtime,
    effects: &mut E,
) -> Result<(), Error> {
    runtime.validate(state)?;
    // The graph is a separate, shared owner; require its presence without
    // taking an initial copy or treating active-route count as temp-route count.
    state
        .trade_graph()
        .map_err(|_| Error::Missing("canonical Trade graph"))?;
    let count = runtime.temporary_routes.len();
    while runtime.temporary_routes.pop().is_some() {}
    if count != 0 {
        effects.temporary_routes_cleared(state)?;
    }
    Ok(())
}
