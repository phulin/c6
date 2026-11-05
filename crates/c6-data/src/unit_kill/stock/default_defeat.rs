//! Live default-defeat invalidation through the existing shared requirement owner.
use super::{EffectRuntime, Error, LoadedState, RequirementOwner};
use crate::{
    game_update_state::activation::victory_requirements as query,
    melee_attack::backend::requirements,
};
use c6_core::melee_attack::proposals::subject_sets::{self as sets, changes};

#[derive(Clone, Copy, Debug)]
pub struct Binding {
    pub receiver: u64,
    pub initialized_owner: i32,
    pub subject: query::Object,
    pub no_units_raw: u8,
    pub no_cities_raw: u8,
}
/// Borrows the same initialized Simple requirements; no stored predicate answer
/// or secondary requirement factory is maintained here.
pub struct Owner<'a> {
    pub runtime: &'a mut requirements::Runtime,
    pub bindings: &'a [Binding],
}
impl RequirementOwner for Owner<'_> {
    fn requirements(&self) -> &sets::Runtime {
        self.runtime.sets()
    }
    fn requirements_mut(&mut self) -> &mut sets::Runtime {
        self.runtime.sets_mut()
    }
    fn invalidate_default_defeat(
        &mut self,
        state: &mut LoadedState,
        _effects: &mut EffectRuntime,
        receiver: u64,
    ) -> Result<(), Error> {
        let binding = *self
            .bindings
            .iter()
            .find(|b| b.receiver == receiver)
            .ok_or(Error::Input("current DefaultDefeat binding"))?;
        changes::invalidate(
            &mut Current {
                state,
                runtime: self.runtime,
                binding,
            },
            receiver,
        )
        .map_err(Error::Requirements)
    }
}
struct Current<'a> {
    state: &'a LoadedState,
    runtime: &'a mut requirements::Runtime,
    binding: Binding,
}
impl changes::Context for Current<'_> {
    type Error = &'static str;
    fn subject_sets(&self) -> Result<&sets::Runtime, Self::Error> {
        Ok(self.runtime.sets())
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, Self::Error> {
        Ok(self.runtime.sets_mut())
    }
    fn test_requirement(&mut self, label: u64) -> Result<i32, sets::Error<Self::Error>> {
        if label != self.binding.receiver {
            return Err(sets::Error::Context("DefaultDefeat requirement identity"));
        }
        query::test_default_defeat(
            self.state,
            self.binding.initialized_owner,
            self.binding.subject,
            self.binding.no_units_raw,
            self.binding.no_cities_raw,
        )
        .map_err(|_| sets::Error::Context("DefaultDefeat live query unavailable"))
    }
    fn modifier_changed(
        &mut self,
        _: sets::SetListener,
        _: u64,
        _: i32,
        _: i32,
    ) -> Result<(), sets::Error<Self::Error>> {
        Err(sets::Error::Context(
            "DefaultDefeat changed gameplay continuation",
        ))
    }
    fn destroy_requirement(&mut self, _: u64) -> Result<(), sets::Error<Self::Error>> {
        Err(sets::Error::Context("DefaultDefeat derived destructor"))
    }
    fn destroy_set(&mut self, _: u64) -> Result<(), sets::Error<Self::Error>> {
        Err(sets::Error::Context("DefaultDefeat set destructor"))
    }
    fn reset_trigger(&mut self, _: u64) -> Result<(), sets::Error<Self::Error>> {
        Err(sets::Error::Context("DefaultDefeat trigger reset"))
    }
}
