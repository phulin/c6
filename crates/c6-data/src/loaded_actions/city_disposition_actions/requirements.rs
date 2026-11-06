//! Keep's CityOccupied subscriber uses its distinct subject/previous-owner filter.
use super::{error, Error, Stage};
use crate::{
    found_city::city_created_callbacks as queries, melee_attack::backend::requirements::Runtime,
};
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
        && binding
            .subject
            .is_some_and(|subject| subject == previous || subject == owner)
        && (!binding.only_context || binding.context == Some(previous))
}
/// CityOccupied.OnCityOccupied23b3d80. Invalidation retains the same initialized
/// SimpleRequirement, child/set Changed rings and modifier subscribers. A
/// positive modifier callback is mandatory; no unsupported change is swallowed.
#[allow(clippy::too_many_arguments)]
pub fn city_occupied<F>(
    state: &mut LoadedState,
    runtime: &mut Runtime,
    current: &queries::Current<'_>,
    binding: Binding,
    previous: i32,
    city: ComponentId,
    changed: &mut F,
) -> Result<(), Error>
where
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
        .map_err(|e| error(Stage::CityOccupied, e))?
        .header
        .owner;
    if !reached(binding, previous, owner) {
        return Ok(());
    }
    changes::invalidate(
        &mut Current {
            state,
            runtime,
            current,
            binding,
            changed,
        },
        binding.label,
    )
    .map_err(|e| error(Stage::CityOccupied, e))
}
struct Current<'s, 'a, F> {
    state: &'s mut LoadedState,
    runtime: &'s mut Runtime,
    current: &'s queries::Current<'a>,
    binding: Binding,
    changed: &'s mut F,
}
impl<F> changes::Context for Current<'_, '_, F>
where
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
                "Keep requirement Test identity".into(),
            ));
        }
        queries::occupied_test(
            self.state,
            self.current,
            self.binding.context,
            self.binding.subject,
            self.binding.only_context,
        )
        .map_err(|e| sets::Error::Context(format!("{e:?}")))
    }
    fn destroy_requirement(&mut self, label: u64) -> Result<(), sets::Error<String>> {
        Err(sets::Error::Context(format!(
            "Keep requirement destruction {label:#x}"
        )))
    }
    fn destroy_set(&mut self, label: u64) -> Result<(), sets::Error<String>> {
        Err(sets::Error::Context(format!(
            "Keep set destruction {label:#x}"
        )))
    }
    fn reset_trigger(&mut self, label: u64) -> Result<(), sets::Error<String>> {
        Err(sets::Error::Context(format!(
            "Keep requirement trigger reset {label:#x}"
        )))
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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn occupation_event_filters_subject_and_only_context_against_previous_owner() {
        let mut b = Binding {
            label: 1,
            context: Some(2),
            subject: Some(0),
            only_context: false,
        };
        assert!(reached(b, 1, 0));
        b.only_context = true;
        assert!(!reached(b, 1, 0));
        b.context = Some(1);
        assert!(reached(b, 1, 0));
        b.subject = Some(1);
        assert!(reached(b, 1, 0));
        b.subject = Some(2);
        assert!(!reached(b, 1, 0));
        b.subject = None;
        assert!(!reached(b, 1, 0));
    }
}
