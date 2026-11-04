//! Revision-checked derived city domains for persistent action providers.
use crate::{
    city_movement::NativeDomainDefinitions,
    movement_placement_state::{self, Definitions},
    resource_binding::ResourceIdentity,
};
use c6_core::{
    city_district::DomainMask,
    loaded_state::{ActionStamp, LoadedState},
};

/// Retains derived masks between queries of the same canonical state revision.
/// Any canonical mutation invalidates them conservatively, including FoundCity,
/// district/building creation, pillage and changes to map ownership. Static rule
/// definitions stay with the caller and are not reconstructed by this cache.
#[derive(Default)]
pub struct Cache {
    stamp: Option<ActionStamp>,
    masks: Vec<Option<DomainMask>>,
}
impl Cache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns no slice for a different state instance or revision. Callers must
    /// refresh before constructing placement queries, rather than reusing an
    /// initial-fixture answer after a later action changes city state.
    pub fn current(&self, state: &LoadedState) -> Option<&[Option<DomainMask>]> {
        (self.stamp == Some(state.action_stamp())).then_some(self.masks.as_slice())
    }

    /// Recomputes only derived city masks, from the current canonical records.
    /// An error leaves the old entry inaccessible for the new revision.
    pub fn refresh(
        &mut self,
        state: &LoadedState,
        placement: &Definitions<'_, '_>,
        domains: &NativeDomainDefinitions,
        resources: &ResourceIdentity,
        max_records: usize,
    ) -> Result<&[Option<DomainMask>], movement_placement_state::Error> {
        let stamp = state.action_stamp();
        if self.stamp != Some(stamp) {
            // Invalidate before building: failure cannot expose old masks as current.
            self.stamp = None;
            let context = movement_placement_state::Context::from_definitions(state, placement)?
                .with_rebuilt_city_domains(
                    domains,
                    resources,
                    max_records,
                    state.world().map().plots.len(),
                )?;
            self.masks = context
                .city_domain_masks()
                .ok_or(movement_placement_state::Error::Definitions)?
                .to_vec();
            self.stamp = Some(stamp);
        }
        Ok(&self.masks)
    }

    /// Optional eager invalidation; revision validation remains mandatory even
    /// when a caller forgets to notify the cache about an action.
    pub fn invalidate(&mut self) {
        self.stamp = None;
    }
}
