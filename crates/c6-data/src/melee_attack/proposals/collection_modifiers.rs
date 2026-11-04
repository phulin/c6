//! Live DynamicModifier collection additions over the canonical proposal owner.
use c6_core::{
    loaded_state::{combat_proposals::AccessError, LoadedState},
    melee_attack::proposals::{collection_modifiers as kernel, registry},
};
#[derive(Debug)]
pub enum Error<E> {
    Access(AccessError),
    Services(E),
}
/// Real reached RequirementSystem, strength-effect and log services. Every
/// method borrows the current LoadedState; native predicate answers are not
/// parameters. Unsupported branches must return errors at the reached call.
pub trait Services {
    type Error;
    fn proposal_order(
        &self,
        state: &LoadedState,
        first: registry::Handle,
        second: registry::Handle,
    ) -> Result<std::cmp::Ordering, Self::Error>;
    fn requirement_set_state(&self, state: &LoadedState, id: u64) -> Result<i32, Self::Error>;
    fn create_subject_requirements(
        &mut self,
        state: &mut LoadedState,
        modifier: u64,
        proposal: registry::Handle,
    ) -> Result<Option<u64>, Self::Error>;
    fn unsubscribe_subject_requirements(
        &mut self,
        state: &mut LoadedState,
        receiver: u64,
        proposal: registry::Handle,
        set: u64,
    ) -> Result<(), Self::Error>;
    fn release_subject_requirements(
        &mut self,
        state: &mut LoadedState,
        set: u64,
    ) -> Result<(), Self::Error>;
    fn apply_strength_effect(
        &mut self,
        state: &mut LoadedState,
        effect: u64,
        proposal: registry::Handle,
    ) -> Result<bool, Self::Error>;
    fn log_applying(
        &mut self,
        state: &mut LoadedState,
        modifier: u64,
        proposal: registry::Handle,
    ) -> Result<(), Self::Error>;
    fn log_failed(
        &mut self,
        state: &mut LoadedState,
        modifier: u64,
        proposal: registry::Handle,
    ) -> Result<(), Self::Error>;
}
struct Context<'a, S> {
    state: &'a mut LoadedState,
    services: &'a mut S,
}
impl<S: Services> kernel::Context for Context<'_, S> {
    type Error = Error<S::Error>;
    fn proposals(&self) -> Result<&registry::Runtime, Self::Error> {
        self.state.combat_proposals().map_err(Error::Access)
    }
    fn proposals_mut(&mut self) -> Result<&mut registry::Runtime, Self::Error> {
        self.state.combat_proposals_mut().map_err(Error::Access)
    }
    fn proposal_order(
        &self,
        first: registry::Handle,
        second: registry::Handle,
    ) -> Result<std::cmp::Ordering, Self::Error> {
        self.services
            .proposal_order(self.state, first, second)
            .map_err(Error::Services)
    }
    fn requirement_set_state(&self, id: u64) -> Result<i32, Self::Error> {
        self.services
            .requirement_set_state(self.state, id)
            .map_err(Error::Services)
    }
    fn create_subject_requirements(
        &mut self,
        modifier: u64,
        proposal: registry::Handle,
    ) -> Result<Option<u64>, Self::Error> {
        self.services
            .create_subject_requirements(self.state, modifier, proposal)
            .map_err(Error::Services)
    }
    fn unsubscribe_subject_requirements(
        &mut self,
        receiver: u64,
        proposal: registry::Handle,
        set: u64,
    ) -> Result<(), Self::Error> {
        self.services
            .unsubscribe_subject_requirements(self.state, receiver, proposal, set)
            .map_err(Error::Services)
    }
    fn release_subject_requirements(&mut self, set: u64) -> Result<(), Self::Error> {
        self.services
            .release_subject_requirements(self.state, set)
            .map_err(Error::Services)
    }
    fn apply_strength_effect(
        &mut self,
        effect: u64,
        proposal: registry::Handle,
    ) -> Result<bool, Self::Error> {
        self.services
            .apply_strength_effect(self.state, effect, proposal)
            .map_err(Error::Services)
    }
    fn log_applying(
        &mut self,
        modifier: u64,
        proposal: registry::Handle,
    ) -> Result<(), Self::Error> {
        self.services
            .log_applying(self.state, modifier, proposal)
            .map_err(Error::Services)
    }
    fn log_failed(&mut self, modifier: u64, proposal: registry::Handle) -> Result<(), Self::Error> {
        self.services
            .log_failed(self.state, modifier, proposal)
            .map_err(Error::Services)
    }
}
/// Diagnostic execution retains native-prefix writes if a genuinely reached
/// service is unsupported. This internal stage is not a complete combat action.
pub fn item_changed<S: Services>(
    state: &mut LoadedState,
    services: &mut S,
    receiver: u64,
    proposal: registry::Handle,
    added: bool,
) -> Result<(), kernel::Error<Error<S::Error>>> {
    kernel::item_changed(&mut Context { state, services }, receiver, proposal, added)
}
