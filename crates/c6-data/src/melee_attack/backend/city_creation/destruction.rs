//! CityDestroyed's GameEffects subscriber over the existing canonical owner.
//! Native ObjectPool enumeration supplies ordered live handles and each
//! object's actual owned-modifier vector. These are state, not policy answers.
use super::{collection, Apply, DestroyedListener, EffectRuntime, Error};
use crate::melee_attack::backend::{requirements, ContextObject};
use c6_core::{loaded_state::LoadedState, unit::ComponentId};

/// A live entry selected by the native ICity query from the complete pool.
/// Keep distinct wrappers even if they expose the same underlying City.
#[derive(Clone, Debug)]
pub struct PoolObject {
    pub handle: u64,
    pub ordinal: usize,
    pub owned_modifiers: Vec<i32>,
}

pub(super) struct Metadata {
    pub ordinal: usize,
    pub owned_modifiers: Vec<i32>,
}

impl EffectRuntime {
    /// Import the observed ICity pool domain once, before advancing this state.
    /// Thereafter ownership vectors and factory ordinals live only in this
    /// canonical registry, so later DistrictRemoved cannot replay old lists.
    pub fn initialize_city_destruction_objects(
        &mut self,
        state: &LoadedState,
        objects: &[PoolObject],
    ) -> Result<(), Error> {
        self.current_owner(state).map_err(Error::Query)?;
        if !state.is_pristine() {
            return Err(Error::Missing("initial City destruction object projection"));
        }
        let creation = self
            .city_creation
            .as_ref()
            .ok_or(Error::Missing("City destruction canonical modifier domain"))?;
        if creation.destruction_initialized {
            return Err(Error::Missing(
                "City destruction objects already initialized",
            ));
        }
        for (index, row) in objects.iter().enumerate() {
            if objects[..index]
                .iter()
                .any(|old| old.handle == row.handle || old.ordinal == row.ordinal)
                || row
                    .owned_modifiers
                    .iter()
                    .enumerate()
                    .any(|(at, id)| row.owned_modifiers[..at].contains(id))
            {
                return Err(Error::Missing(
                    "unique initial City destruction pool/owned state",
                ));
            }
            if self
                .object(row.handle)
                .map_err(Error::Query)?
                .kind
                .city_component()
                .is_none()
                || !creation
                    .objects
                    .iter()
                    .any(|object| object.handle == row.handle)
            {
                return Err(Error::Missing("initial City destruction ICity lifecycle"));
            }
        }
        for (handle, object) in self.objects() {
            if object.kind.city_component().is_some()
                && !objects.iter().any(|row| row.handle == *handle)
            {
                return Err(Error::Missing("initial complete ICity destruction domain"));
            }
        }
        let creation = self.city_creation.as_mut().unwrap();
        for row in objects {
            creation.object_mut(row.handle)?.destruction = Some(Metadata {
                ordinal: row.ordinal,
                owned_modifiers: row.owned_modifiers.clone(),
            });
        }
        creation.destruction_initialized = true;
        Ok(())
    }
}

/// Execute OnCityDestroyed_DetachGameEffects23e1d90/ObjectDestroyed2331560
/// using current pool metadata and owned vectors in the SAME canonical owner.
#[allow(clippy::too_many_arguments)]
pub fn detach_city(
    state: &mut LoadedState,
    runtime: &mut EffectRuntime,
    requirements: &mut requirements::Runtime,
    city: ComponentId,
    logging_priority: i32,
    features: &[u32],
    factory: &mut dyn collection::RequirementFactory,
    effects: &mut impl crate::city_production_turn::modifier_effects::Effects,
) -> Result<(), Error> {
    runtime.current_owner(state).map_err(Error::Query)?;
    if logging_priority > 1 {
        return Err(Error::Missing(
            "CityDestroyed enabled logging MakeObject continuation",
        ));
    }
    state
        .city(i32::from(city.owner), city.id)
        .map_err(|_| Error::Missing("CityDestroyed current canonical City"))?;
    if city.component_type != 2 {
        return Err(Error::Missing("CityDestroyed City component type"));
    }
    let creation = runtime
        .city_creation
        .as_ref()
        .ok_or(Error::Missing("CityDestroyed canonical modifier domain"))?;
    if !creation.destruction_initialized {
        return Err(Error::Missing(
            "CityDestroyed initial ICity pool projection",
        ));
    }
    let mut objects = Vec::new();
    for (handle, object) in runtime.objects() {
        if object.kind.city_component() == Some(city) {
            let row = creation
                .objects
                .iter()
                .find(|row| row.handle == *handle)
                .and_then(|row| row.destruction.as_ref())
                .ok_or(Error::Missing(
                    "CityDestroyed current pool/lifecycle metadata",
                ))?;
            objects.push((row.ordinal, *handle));
        }
        if matches!(object.kind, ContextObject::Improvement { city: id, .. } if id == city) {
            return Err(Error::Missing("CityDestroyed Improvement ICity interface"));
        }
    }
    objects.sort_by_key(|row| row.0);
    if objects.windows(2).any(|rows| rows[0].0 == rows[1].0) {
        return Err(Error::Missing("CityDestroyed unique native pool ordering"));
    }
    for (_, handle) in objects {
        // The copied pool contains WEAK owners. Lock each surviving wrapper
        // only when reached; prior callbacks may have released its last owner.
        // New wrappers are outside the copied pool. The current lock survives
        // its complete ObjectDestroyed dispatch, then releases before the next.
        if !runtime.objects().iter().any(|row| row.0 == handle) {
            continue;
        }
        let owned = runtime
            .city_creation
            .as_mut()
            .unwrap()
            .object_mut(handle)?
            .destruction
            .as_ref()
            .ok_or(Error::Missing("CityDestroyed current ownership"))?
            .owned_modifiers
            .clone();
        for instance in &owned {
            let owner = runtime.city_modifier_state(*instance)?.owner;
            if owner != handle {
                return Err(Error::Missing("CityDestroyed owned modifier identity"));
            }
            runtime.remove_city_modifier_instance(
                state,
                requirements,
                *instance,
                features,
                effects,
            )?;
        }
        if runtime
            .city_creation
            .as_ref()
            .unwrap()
            .modifiers
            .iter()
            .any(|record| record.owner == handle)
        {
            return Err(Error::Missing(
                "CityDestroyed omitted current owned modifier",
            ));
        }
        let mut cursor = runtime
            .city_creation
            .as_mut()
            .unwrap()
            .object_mut(handle)?
            .destroyed
            .cursor_start()
            .map_err(Error::Ring)?;
        loop {
            let callback = runtime
                .city_creation
                .as_mut()
                .unwrap()
                .object_mut(handle)?
                .destroyed
                .cursor_route(cursor)
                .map_err(Error::Ring)?;
            let result = match callback {
                Some(DestroyedListener::Existing(_)) => Err(Error::Missing(
                    "CityDestroyed imported ObjectDestroyed listener",
                )),
                Some(DestroyedListener::ModifierOwner(instance)) => runtime
                    .remove_city_modifier_instance(
                        state,
                        requirements,
                        instance,
                        features,
                        effects,
                    ),
                Some(DestroyedListener::Modifier(instance)) => tracked_destroyed(
                    state,
                    runtime,
                    requirements,
                    handle,
                    instance,
                    features,
                    factory,
                    effects,
                ),
                None => Ok(()),
            };
            if let Err(error) = result {
                runtime
                    .city_creation
                    .as_mut()
                    .unwrap()
                    .object_mut(handle)?
                    .destroyed
                    .cursor_release(cursor)
                    .map_err(Error::Ring)?;
                return Err(error);
            }
            match runtime
                .city_creation
                .as_mut()
                .unwrap()
                .object_mut(handle)?
                .destroyed
                .cursor_next(cursor)
                .map_err(Error::Ring)?
            {
                Some(next) => cursor = next,
                None => break,
            }
        }
    }
    // The native subscriber trims expired weak slots after visiting the copied
    // pool. Lifetime retirement remains with the canonical real owner graph;
    // ordinary City cache release belongs to the enclosing City destructor.
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn tracked_destroyed(
    state: &mut LoadedState,
    runtime: &mut EffectRuntime,
    requirements: &mut requirements::Runtime,
    subject: u64,
    instance: i32,
    features: &[u32],
    factory: &mut dyn collection::RequirementFactory,
    effects: &mut impl crate::city_production_turn::modifier_effects::Effects,
) -> Result<(), Error> {
    let index = runtime
        .city_creation
        .as_ref()
        .unwrap()
        .modifiers
        .iter()
        .position(|record| record.instance == instance)
        .ok_or(Error::Missing("CityDestroyed tracked modifier"))?;
    // ObjectDestroyed2334610 removes effects only while enabled, then releases
    // tracking regardless of enabled/suppressed state. It bypasses the
    // collection's owner requirement gate.
    let tracked = runtime.city_creation.as_ref().unwrap().modifiers[index]
        .tracked
        .iter()
        .position(|row| row.subject == subject);
    if let Some(at) = tracked {
        let control = &runtime.city_creation.as_ref().unwrap().modifiers[index].modifier;
        if control.enabled && !control.suppressed {
            c6_core::unit_moved_signal::modifiers::Context::remove_effects(&mut Apply {
                state,
                runtime,
                requirements,
                subject,
                factory,
                features,
                maximum_loyalty: None,
                index,
                effects,
            })?;
        }
        let row = &runtime.city_creation.as_ref().unwrap().modifiers[index].tracked[at];
        let (token, set) = (row.destruction, row.state.requirement_set);
        runtime
            .city_creation
            .as_mut()
            .unwrap()
            .object_mut(subject)?
            .destroyed
            .unsubscribe(token)
            .map_err(Error::Ring)?;
        if let Some(set) = set {
            let receiver = runtime.city_creation.as_ref().unwrap().modifiers[index].identity;
            requirements
                .sets_mut()
                .unsubscribe(receiver, set)
                .map_err(|e| Error::Requirement(Box::new(e)))?;
            runtime.city_creation.as_mut().unwrap().modifiers[index].tracked[at]
                .state
                .requirement_set = None;
            super::government::release(state, runtime, requirements, set, effects)?;
        }
        runtime.city_creation.as_mut().unwrap().modifiers[index]
            .tracked
            .remove(at);
    }
    runtime.city_creation.as_mut().unwrap().modifiers[index]
        .applied_subjects
        .retain(|handle| *handle != subject);
    Ok(())
}
