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

/// Execute OnCityDestroyed_DetachGameEffects23e1d90/ObjectDestroyed2331560.
/// The caller supplies the current complete matching pool projection from the
/// same initial snapshot and updates it for subsequent source allocations.
/// Missing lifecycle state is an error; no wrapper/factory is invented here.
#[allow(clippy::too_many_arguments)]
pub fn detach_city(
    state: &mut LoadedState,
    runtime: &mut EffectRuntime,
    requirements: &mut requirements::Runtime,
    city: ComponentId,
    objects: &[PoolObject],
    logging_priority: i32,
    features: &[u32],
    factory: &mut dyn collection::RequirementFactory,
    effects: &mut impl crate::city_production_turn::modifier_effects::Effects,
) -> Result<(), Error> {
    runtime.current_owner(state).map_err(Error::Query)?;
    // Enabled diagnostics call MakeObject before pool enumeration and can
    // allocate a wrapper. Preserve that gameplay identity effect when added.
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
    // Check the represented interface domain, preserving every distinct wrapper
    // and source pool order. An unimported owner is not an empty native pool.
    if objects.is_empty() {
        return Err(Error::Missing(
            "CityDestroyed current matching ObjectPool projection",
        ));
    }
    for (index, row) in objects.iter().enumerate() {
        if objects[..index].iter().any(|old| old.handle == row.handle)
            || index > 0 && objects[index - 1].ordinal >= row.ordinal
        {
            return Err(Error::Missing("CityDestroyed unique native pool ordering"));
        }
        if runtime
            .object(row.handle)
            .map_err(Error::Query)?
            .kind
            .city_component()
            != Some(city)
        {
            return Err(Error::Missing("CityDestroyed current ICity pool identity"));
        }
        runtime
            .city_creation
            .as_mut()
            .ok_or(Error::Missing("CityDestroyed canonical modifier domain"))?
            .object_mut(row.handle)?;
    }
    for (handle, object) in runtime.objects() {
        if object.kind.city_component() == Some(city)
            && !objects.iter().any(|row| row.handle == *handle)
        {
            return Err(Error::Missing(
                "CityDestroyed omitted current ICity wrapper",
            ));
        }
        // Improvement interface support must be established before this class
        // can silently disappear from the complete native ICity projection.
        if matches!(object.kind, ContextObject::Improvement { city: id, .. } if id == city) {
            return Err(Error::Missing("CityDestroyed Improvement ICity interface"));
        }
    }
    for row in objects {
        // ClearModifiers copies the owned vector before invoking destruction.
        // The projection is retained for the whole batch, like native shared_ptrs.
        for instance in &row.owned_modifiers {
            let owner = runtime.city_modifier_state(*instance)?.owner;
            if owner != row.handle {
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
            .any(|record| record.owner == row.handle)
        {
            return Err(Error::Missing(
                "CityDestroyed omitted current owned modifier",
            ));
        }
        let mut cursor = runtime
            .city_creation
            .as_mut()
            .unwrap()
            .object_mut(row.handle)?
            .destroyed
            .cursor_start()
            .map_err(Error::Ring)?;
        loop {
            let callback = runtime
                .city_creation
                .as_mut()
                .unwrap()
                .object_mut(row.handle)?
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
                    row.handle,
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
                    .object_mut(row.handle)?
                    .destroyed
                    .cursor_release(cursor)
                    .map_err(Error::Ring)?;
                return Err(error);
            }
            match runtime
                .city_creation
                .as_mut()
                .unwrap()
                .object_mut(row.handle)?
                .destroyed
                .cursor_next(cursor)
                .map_err(Error::Ring)?
            {
                Some(next) => cursor = next,
                None => break,
            }
        }
    }
    // The native subscriber trims expired weak pool slots after releasing its
    // temporary strong owners. Canonical gameplay wrappers remain until the
    // enclosing City/District destruction releases their real owners.
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
