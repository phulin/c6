//! Concrete CityDestroyed requirement filters and Tests over the SAME live
//! SimpleRequirement/set owner used by modifier creation and destruction.
use crate::{
    found_city::city_created_callbacks as queries,
    game_update_state::activation::victory_requirements as victory,
    melee_attack::backend::requirements::Runtime, unit_kill::stock::default_defeat,
};
use c6_core::{
    loaded_state::LoadedState,
    melee_attack::proposals::subject_sets::{self as sets, changes},
    unit::ComponentId,
};

#[derive(Clone, Copy, Debug)]
pub enum Binding {
    /// Destroyed23b3f00 filters SUBJECT owner; Created23b3ea0 instead filters
    /// context. Preserve the initialized semantic interfaces, not Test answers.
    CityOccupied {
        label: u64,
        context: Option<i32>,
        subject: Option<i32>,
        only_context: bool,
    },
    TeamDomination {
        label: u64,
        team: i32,
    },
    DefaultDefeat(default_defeat::Binding),
}
impl Binding {
    fn label(self) -> u64 {
        match self {
            Self::CityOccupied { label, .. } | Self::TeamDomination { label, .. } => label,
            Self::DefaultDefeat(binding) => binding.receiver,
        }
    }
}
#[derive(Debug)]
pub enum Error {
    State(c6_core::loaded_state::Error),
    CityQuery(queries::Error),
    DefaultQuery(victory::Error),
    Requirement(Box<sets::Error<Error>>),
    Missing(&'static str),
}

/// Source callbacks invalidate their existing SimpleRequirement in registration
/// order. Active, inverse, persistent/triggered flags, state assignment and all
/// child/set Changed effects remain in the shared runtime. Positive unsupported
/// consumers fail at their position after retaining the actual applied prefix.
pub fn city_destroyed(
    state: &mut LoadedState,
    runtime: &mut Runtime,
    current: &queries::Current<'_>,
    binding: Binding,
    city: ComponentId,
) -> Result<(), Error> {
    if city.component_type != 2 {
        return Err(Error::Missing("CityDestroyed requirement City type"));
    }
    let owner = state
        .city(i32::from(city.owner), city.id)
        .map_err(Error::State)?
        .header
        .owner;
    match binding {
        Binding::CityOccupied {
            subject, context, ..
        } => {
            if subject != Some(owner) {
                return Ok(());
            }
            if context.is_none() {
                return Err(Error::Missing("CityOccupied destruction context IPlayer"));
            }
        }
        Binding::DefaultDefeat(binding) if binding.initialized_owner != owner => return Ok(()),
        _ => {}
    }
    changes::invalidate(
        &mut Current {
            state,
            runtime,
            current,
            binding,
        },
        binding.label(),
    )
    .map_err(|error| Error::Requirement(Box::new(error)))
}
struct Current<'a, 'b> {
    state: &'a LoadedState,
    runtime: &'a mut Runtime,
    current: &'a queries::Current<'b>,
    binding: Binding,
}
impl changes::Context for Current<'_, '_> {
    type Error = Error;
    fn subject_sets(&self) -> Result<&sets::Runtime, Error> {
        Ok(self.runtime.sets())
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, Error> {
        Ok(self.runtime.sets_mut())
    }
    fn test_requirement(&mut self, label: u64) -> Result<i32, sets::Error<Error>> {
        if self.binding.label() != label {
            return Err(sets::Error::Context(Error::Missing(
                "CityDestroyed requirement Test identity",
            )));
        }
        match self.binding {
            Binding::CityOccupied {
                context,
                subject,
                only_context,
                ..
            } => queries::occupied_test(self.state, self.current, context, subject, only_context)
                .map_err(|error| sets::Error::Context(Error::CityQuery(error))),
            Binding::TeamDomination { team, .. } => {
                queries::domination_test(self.state, self.current, team)
                    .map_err(|error| sets::Error::Context(Error::CityQuery(error)))
            }
            Binding::DefaultDefeat(binding) => victory::test_default_defeat(
                self.state,
                binding.initialized_owner,
                binding.subject,
                binding.no_units_raw,
                binding.no_cities_raw,
            )
            .map_err(|error| sets::Error::Context(Error::DefaultQuery(error))),
        }
    }
    fn modifier_changed(
        &mut self,
        _: sets::SetListener,
        _: u64,
        _: i32,
        _: i32,
    ) -> Result<(), sets::Error<Error>> {
        Err(sets::Error::Context(Error::Missing(
            "CityDestroyed changed requirement gameplay continuation",
        )))
    }
    fn destroy_requirement(&mut self, _: u64) -> Result<(), sets::Error<Error>> {
        Err(sets::Error::Context(Error::Missing(
            "CityDestroyed derived requirement release",
        )))
    }
    fn destroy_set(&mut self, _: u64) -> Result<(), sets::Error<Error>> {
        Err(sets::Error::Context(Error::Missing(
            "CityDestroyed derived set release",
        )))
    }
    fn reset_trigger(&mut self, set: u64) -> Result<(), sets::Error<Error>> {
        changes::reset_trigger_state(self.runtime.sets_mut(), set)
    }
}
