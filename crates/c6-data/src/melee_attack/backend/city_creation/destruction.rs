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
            let kind = self.object(row.handle).map_err(Error::Query)?.kind;
            if kind.city_component().is_some()
                && !creation
                    .objects
                    .iter()
                    .any(|object| object.handle == row.handle)
            {
                return Err(Error::Missing("initial City destruction ICity lifecycle"));
            }
        }
        for (handle, _) in self.objects() {
            if !objects.iter().any(|row| row.handle == *handle) {
                return Err(Error::Missing("initial complete ICity destruction domain"));
            }
        }
        let mut order = objects
            .iter()
            .map(|row| (row.ordinal, row.handle))
            .collect::<Vec<_>>();
        order.sort_by_key(|row| row.0);
        self.allocations.pool_order = Some(order);
        let creation = self.city_creation.as_mut().unwrap();
        for row in objects {
            if let Some(object) = creation
                .objects
                .iter_mut()
                .find(|object| object.handle == row.handle)
            {
                object.destruction = Some(Metadata {
                    owned_modifiers: row.owned_modifiers.clone(),
                });
            }
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
    let objects = runtime
        .allocations
        .pool_order
        .as_ref()
        .ok_or(Error::Missing("CityDestroyed complete normal pool order"))?
        .clone();
    for (_, handle) in objects {
        // The copied pool contains WEAK owners. Lock each surviving wrapper
        // only when reached; prior callbacks may have released its last owner.
        // New wrappers are outside the copied pool. The current lock survives
        // its complete ObjectDestroyed dispatch, then releases before the next.
        if !runtime.objects().iter().any(|row| row.0 == handle) {
            continue;
        }
        if current_city(state, runtime, handle)? != Some(city) {
            continue;
        }
        detach_object(
            state,
            runtime,
            requirements,
            handle,
            features,
            factory,
            effects,
        )?;
    }
    // The native subscriber trims expired weak slots after visiting the copied
    // pool. Lifetime retirement remains with the canonical real owner graph;
    // ordinary City cache release belongs to the enclosing City destructor.
    Ok(())
}

/// OnDistrictDestroyed_DetachGameEffects23e2330 always creates/shares the
/// ordinary District object before its weak-pool snapshot. Every visited
/// object's GetPrimaryObject runs, even when it does not match the target.
#[allow(clippy::too_many_arguments)]
pub fn detach_district(
    state: &mut LoadedState,
    runtime: &mut EffectRuntime,
    requirements: &mut requirements::Runtime,
    district: ComponentId,
    logging_priority: i32,
    features: &[u32],
    factory: &mut dyn collection::RequirementFactory,
    effects: &mut impl crate::city_production_turn::modifier_effects::Effects,
) -> Result<(), Error> {
    runtime.current_owner(state).map_err(Error::Query)?;
    let primary = runtime.make_district_object(state, district)?;
    if logging_priority > 1 {
        return Err(Error::Missing(
            "DistrictDestroyed enabled object formatting",
        ));
    }
    let objects = runtime
        .allocations
        .pool_order
        .as_ref()
        .ok_or(Error::Missing(
            "DistrictDestroyed complete normal pool order",
        ))?
        .clone();
    for (_, handle) in objects {
        if !runtime.objects().iter().any(|row| row.0 == handle) {
            continue;
        }
        let current = primary_object(state, runtime, handle)?;
        let matched = current == primary;
        // Native releases the GetPrimaryObject temporary BEFORE dispatching.
        // The target's outer MakeObject owner remains live until this handler
        // returns, so sharing it here cannot expire it.
        if current != primary {
            runtime.release_factory_primary(current)?;
        }
        if matched {
            detach_object(
                state,
                runtime,
                requirements,
                handle,
                features,
                factory,
                effects,
            )?;
        }
    }
    runtime.release_factory_primary(primary)
}
fn primary_object(
    state: &LoadedState,
    runtime: &mut EffectRuntime,
    handle: u64,
) -> Result<u64, Error> {
    let kind = runtime.object(handle).map_err(Error::Query)?.kind;
    match kind {
        ContextObject::City(city) => runtime.make_city_object(state, city),
        ContextObject::District { district, .. } | ContextObject::Building { district, .. } => {
            runtime.make_district_object(state, district)
        }
        ContextObject::Unit(unit) => runtime
            .make_current_unit_object(state, unit, [0, 0])
            .map_err(Error::Query),
        ContextObject::Policy { owner, .. } | ContextObject::Government { owner, .. } => runtime
            .objects()
            .iter()
            .find(|(_, object)| object.kind == ContextObject::Player(owner))
            .map(|row| row.0)
            .ok_or(Error::Missing("DistrictDestroyed normal Player factory")),
        // Actual normal Player/Game factories share the already represented
        // ordinary object; Team/Improvement vtable+20 routes to ModifierObject
        // self-weak GetPrimaryObject2335920 on this target.
        ContextObject::Player(_)
        | ContextObject::Belief { .. }
        | ContextObject::Game
        | ContextObject::Team(_)
        | ContextObject::Improvement { .. } => Ok(handle),
        ContextObject::Plot(_)
        | ContextObject::Alliance { .. }
        | ContextObject::Emergency { .. } => Err(Error::Missing(
            "DistrictDestroyed source-qualified primary object class",
        )),
    }
}

fn current_city(
    state: &LoadedState,
    runtime: &EffectRuntime,
    handle: u64,
) -> Result<Option<ComponentId>, Error> {
    Ok(match runtime.object(handle).map_err(Error::Query)?.kind {
        ContextObject::City(city) => Some(city),
        ContextObject::Building { district, .. } | ContextObject::District { district, .. } => {
            match state.district(i32::from(district.owner), district.id) {
                Ok(record) => Some(ComponentId { owner: district.owner, component_type: 2, id: record.city_id }),
                Err(c6_core::loaded_state::district_initialization::DistrictError::MissingDistrict { .. }) => None,
                Err(_) => return Err(Error::Missing("current ICity District state")),
            }
        }
        ContextObject::Improvement { city, .. } => Some(city),
        _ => None,
    })
}
#[allow(clippy::too_many_arguments)]
fn detach_object(
    state: &mut LoadedState,
    runtime: &mut EffectRuntime,
    requirements: &mut requirements::Runtime,
    handle: u64,
    features: &[u32],
    factory: &mut dyn collection::RequirementFactory,
    effects: &mut impl crate::city_production_turn::modifier_effects::Effects,
) -> Result<(), Error> {
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
        runtime.remove_city_modifier_instance(state, requirements, *instance, features, effects)?;
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
                .remove_city_modifier_instance(state, requirements, instance, features, effects),
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

impl EffectRuntime {
    /// CityD1.SetModifierObject(nullptr) releases the gameplay cache before
    /// validation and CityBuild destruction. Keep wrappers retained by actual
    /// modifier/requirement owners; the pool's weak entry itself retains none.
    pub fn release_destroyed_city_cache(
        &mut self,
        state: &LoadedState,
        requirements: &requirements::Runtime,
        city: ComponentId,
    ) -> Result<(), Error> {
        self.current_owner(state).map_err(Error::Query)?;
        let Some(handle) = self
            .objects
            .iter()
            .find(|(_, r)| r.kind == ContextObject::City(city))
            .map(|r| r.0)
        else {
            // An absent cache is valid only for a source-created City whose
            // first modifier wrapper was never requested.
            if self
                .allocations
                .has_initial_city(city)
                .map_err(Error::Query)?
            {
                return Err(Error::Missing("initial destroyed City modifier cache"));
            }
            return Ok(());
        };
        let creation = self
            .city_creation
            .as_mut()
            .ok_or(Error::Missing("destroyed City lifecycle"))?;
        let object = creation
            .objects
            .iter()
            .find(|r| r.handle == handle)
            .ok_or(Error::Missing("destroyed City object"))?;
        let owned = &object
            .destruction
            .as_ref()
            .ok_or(Error::Missing("destroyed City owned projection"))?
            .owned_modifiers;
        if !owned.is_empty() || !object.affected.entries().is_empty() {
            return Err(Error::Missing(
                "City cache release with surviving owned/affected modifiers",
            ));
        }
        if creation.modifiers.iter().any(|r| {
            r.owner == handle
                || r.tracked.iter().any(|s| s.subject == handle)
                || r.applied_subjects.contains(&handle)
        }) || self.unit_modifiers.as_ref().is_some_and(|rows| {
            rows.iter().any(|r| {
                r.context == handle
                    || r.tracked.iter().any(|s| s.object == handle)
                    || r.applied.contains(&handle)
            })
        }) || requirements.sets().retains_modifier_object(handle)
        {
            return Ok(());
        }
        if object.destroyed.ordered_subscribers().next().is_some() {
            return Err(Error::Missing(
                "City cache release surviving Destroyed listener ownership",
            ));
        }
        creation.objects.retain(|r| r.handle != handle);
        self.objects.retain(|r| r.0 != handle);
        Ok(())
    }
}
