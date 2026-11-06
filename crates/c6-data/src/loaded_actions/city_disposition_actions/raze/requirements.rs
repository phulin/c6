//! Raze requirement invalidation on the existing canonical requirement owner.
use super::{error, Error, Stage};
use crate::melee_attack::backend::requirements::Runtime;
use c6_core::{
    loaded_state::LoadedState,
    melee_attack::proposals::subject_sets::{self as sets, changes},
    unit::ComponentId,
};
#[derive(Clone, Copy, Debug)]
pub struct Binding {
    pub label: u64,
    pub context: Option<i32>,
    pub subject: Option<i32>,
    pub only_context: bool,
}
fn reached(binding: Binding, previous: i32, owner: i32) -> bool {
    binding.context.is_some()
        && binding.subject.is_some_and(|subject| subject == owner)
        && (!binding.only_context || binding.context == Some(previous))
}
/// CityRazed.OnCityRazed23b44d0. Invalidation retains the same initialized
/// SimpleRequirement, child/set Changed rings and modifier subscribers. A
/// positive modifier callback is mandatory; no unsupported change is swallowed.
#[allow(clippy::too_many_arguments)]
pub fn city_razed<F, D>(
    state: &mut LoadedState,
    runtime: &mut Runtime,
    binding: Binding,
    previous: i32,
    city: ComponentId,
    changed: &mut F,
    destroyed: &mut D,
) -> Result<(), Error>
where
    D: FnMut(u64) -> Result<(), String>,
    F: FnMut(
        &mut LoadedState,
        &mut Runtime,
        sets::SetListener,
        u64,
        i32,
        i32,
    ) -> Result<(), String>,
{
    let owner = state
        .city(i32::from(city.owner), city.id)
        .map_err(|e| error(Stage::CityRazed, e))?
        .header
        .owner;
    if !reached(binding, previous, owner) {
        return Ok(());
    }
    changes::invalidate(
        &mut Current {
            state,
            runtime,
            binding,
            changed,
            destroyed,
        },
        binding.label,
    )
    .map_err(|e| error(Stage::CityRazed, e))
}
struct Current<'s, F, D> {
    state: &'s mut LoadedState,
    runtime: &'s mut Runtime,
    binding: Binding,
    changed: &'s mut F,
    destroyed: &'s mut D,
}
impl<F, D> changes::Context for Current<'_, F, D>
where
    D: FnMut(u64) -> Result<(), String>,
    F: FnMut(
        &mut LoadedState,
        &mut Runtime,
        sets::SetListener,
        u64,
        i32,
        i32,
    ) -> Result<(), String>,
{
    type Error = String;
    fn subject_sets(&self) -> Result<&sets::Runtime, String> {
        Ok(self.runtime.sets())
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, String> {
        Ok(self.runtime.sets_mut())
    }
    fn test_requirement(&mut self, label: u64) -> Result<i32, sets::Error<String>> {
        if label != self.binding.label {
            return Err(sets::Error::Context(
                "Raze requirement Test identity".into(),
            ));
        }
        let value =
            if let (Some(context), Some(subject)) = (self.binding.context, self.binding.subject) {
                let at = usize::try_from(context)
                    .ok()
                    .filter(|i| *i < 64)
                    .ok_or_else(|| sets::Error::Context("Raze requirement context".into()))?;
                let player = self
                    .state
                    .retained()
                    .players
                    .iter()
                    .find(|p| p.header.owner == subject)
                    .ok_or_else(|| sets::Error::Context("Raze requirement subject".into()))?;
                if player.diplomacy.cities_razed[at] > 0 {
                    2
                } else {
                    1
                }
            } else {
                1
            };
        Ok(value)
    }
    fn destroy_requirement(&mut self, label: u64) -> Result<(), sets::Error<String>> {
        (self.destroyed)(label).map_err(sets::Error::Context)
    }
    fn destroy_set(&mut self, label: u64) -> Result<(), sets::Error<String>> {
        Err(sets::Error::Context(format!(
            "Raze set destruction {label:#x}"
        )))
    }
    fn reset_trigger(&mut self, label: u64) -> Result<(), sets::Error<String>> {
        changes::reset_trigger_state(self.runtime.sets_mut(), label)
    }
    fn modifier_changed(
        &mut self,
        listener: sets::SetListener,
        label: u64,
        old: i32,
        new: i32,
    ) -> Result<(), sets::Error<String>> {
        (self.changed)(self.state, self.runtime, listener, label, old, new)
            .map_err(sets::Error::Context)
    }
}
