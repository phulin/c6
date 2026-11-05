//! Source-ordered visibility-loss publication for an unoccupied ground plot.
//! Discovery and occupied/city/structure propagation remain explicit boundaries.
use c6_core::{loaded_state::LoadedState, visibility_change::Propagation, visibility_propagation};

pub trait Effects {
    type Error;
    fn fallout(&self, plot: u32) -> Result<bool, Self::Error>;
    fn park_at(&self, state: &LoadedState, plot: u32) -> Result<bool, Self::Error>;
    fn revealed_resource(
        &self,
        state: &LoadedState,
        owner: i32,
        hash: u32,
    ) -> Result<bool, Self::Error>;
    /// Execute synchronous gameplay consumers; presentation payloads may be omitted
    /// only by a separately qualified stock reporting provider.
    fn report(
        &mut self,
        state: &LoadedState,
        event: Propagation,
        kind: u32,
    ) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub enum Error<E> {
    Effect(E),
    Missing(&'static str),
    World(c6_core::world::MissingInput),
}

/// Linux PropagateVisibilityChanges 2bc1b20, mask ff, firstReveal=false.
/// The sole non-report gameplay call in the feature branch is firstReveal-gated
/// CompleteMovement; this path cannot reach it. All-layer occupancy is checked
/// before omitting the final unit visibility iteration.
pub fn propagate<E: Effects>(
    state: &LoadedState,
    event: Propagation,
    effects: &mut E,
) -> Result<(), Error<E::Error>> {
    if event.mask != 0xff || event.first_reveal || event.revealed_state != 1 {
        return Err(Error::Missing("non-loss visibility propagation"));
    }
    let plot = state
        .world()
        .map()
        .plots
        .get(event.plot as usize)
        .ok_or(Error::Missing("visibility Plot"))?;
    if !visibility_propagation::hidden_propagation_is_empty(state.world(), event.plot)
        .map_err(Error::World)?
    {
        return Err(Error::Missing("occupied visibility-loss plot"));
    }
    if plot.improvement_hash != u32::MAX || plot.route_type != -1 || plot.city_attributes.is_some()
    {
        return Err(Error::Missing("structure visibility-loss plot"));
    }
    if effects.park_at(state, event.plot).map_err(Error::Effect)? {
        return Err(Error::Missing("park visibility-loss plot"));
    }
    effects
        .report(state, event, 0x827289e6)
        .map_err(Error::Effect)?;
    if effects.fallout(event.plot).map_err(Error::Effect)? {
        return Err(Error::Missing("fallout visibility-loss report"));
    }
    if plot.feature_hash != u32::MAX {
        effects
            .report(state, event, 0x878cd252)
            .map_err(Error::Effect)?;
    }
    if effects
        .revealed_resource(state, event.player, plot.resource_hash)
        .map_err(Error::Effect)?
    {
        effects
            .report(state, event, 0x9aea8529)
            .map_err(Error::Effect)?;
    }
    Ok(())
}
