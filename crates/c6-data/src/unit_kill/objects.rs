//! Unit-wrapper cleanup over the SAME persistent combat effect/object owner.
//! These functions execute the concrete detach/Remove state machines; source
//! registration routing and reached requirement/owned-modifier services remain
//! explicit and must use that same owner, not per-callback reconstructed state.
use super::{EffectError, Error};
use crate::{
    melee_attack::backend::{ContextObject, EffectRuntime, EffectState},
    unit_initialization_definitions::Definitions,
    unit_sight_state as sight,
};
use c6_core::movement_world::DefinitionLookup;
use c6_core::{
    loaded_state::LoadedState, unit::ComponentId, unit_kill::objects as kernel,
    unit_moved_signal::modifiers::UnitObjectKey,
};

pub trait Services<P: super::Providers>: super::Continuations<P> {
    fn unit_cleanup_runtime(
        &self,
    ) -> Result<&EffectRuntime, crate::melee_attack::backend::QueryError>;
    fn unit_cleanup_runtime_mut(
        &mut self,
    ) -> Result<&mut EffectRuntime, crate::melee_attack::backend::QueryError>;
    fn clear_owned_unit_modifier(
        &mut self,
        _state: &mut LoadedState,
        _sight: &mut sight::Runtime,
        _providers: &mut P,
        _instance: i32,
    ) -> Result<(), Error<Self::Error, EffectError<P>>> {
        Err(Error::Missing("current owned Unit modifier destruction"))
    }
    fn unit_modifier_owner_destroyed(
        &mut self,
        _state: &mut LoadedState,
        _sight: &mut sight::Runtime,
        _providers: &mut P,
        _receiver: u64,
    ) -> Result<(), Error<Self::Error, EffectError<P>>> {
        Err(Error::Missing(
            "unexpected Unit modifier owner-destruction fallback",
        ))
    }
    fn unsubscribe_unit_requirement(
        &mut self,
        _state: &mut LoadedState,
        _sight: &mut sight::Runtime,
        _providers: &mut P,
        _receiver: u64,
        _object: u64,
        _set: u64,
    ) -> Result<(), Error<Self::Error, EffectError<P>>> {
        Err(Error::Missing(
            "current Unit requirement change unsubscribe",
        ))
    }
    fn release_unit_requirement(
        &mut self,
        _state: &mut LoadedState,
        _sight: &mut sight::Runtime,
        _providers: &mut P,
        _set: u64,
    ) -> Result<(), Error<Self::Error, EffectError<P>>> {
        Err(Error::Missing("current Unit requirement reference release"))
    }
    /// Stock modifier/Unit ToString are discarded output on the source-qualified
    /// log-disabled profile. Enabled/custom output paths need their real service.
    fn unit_cleanup_log(
        &mut self,
        _state: &LoadedState,
        _receiver: u64,
        _object: u64,
        _failed: bool,
    ) -> Result<(), Error<Self::Error, EffectError<P>>> {
        Err(Error::Missing("Unit modifier Remove logging profile"))
    }
}
struct Context<'a, P, C> {
    state: &'a mut LoadedState,
    sight: &'a mut sight::Runtime,
    providers: &'a mut P,
    effects: &'a mut C,
    definitions: &'a Definitions,
}
fn object_key<E, P>(
    object: &crate::melee_attack::backend::Object,
) -> Result<UnitObjectKey, Error<E, P>> {
    let ContextObject::Unit(reference) = object.kind else {
        return Err(Error::Missing("Unit object comparator class"));
    };
    let suffix = object
        .unit_sort_suffix
        .ok_or(Error::Missing("Unit object comparator suffix"))?;
    Ok(UnitObjectKey {
        component: reference.component_id(),
        first: suffix[0],
        second: suffix[1],
    })
}
impl<P: super::Providers, C: Services<P>> kernel::Context for Context<'_, P, C> {
    type Error = Error<C::Error, EffectError<P>>;
    fn object(&self, id: u64) -> Result<&kernel::Object, Self::Error> {
        self.effects
            .unit_cleanup_runtime()
            .map_err(Error::CleanupQuery)?
            .object(id)
            .map_err(Error::CleanupQuery)?
            .unit_lifecycle
            .as_ref()
            .ok_or(Error::Missing("current Unit wrapper lifecycle"))
    }
    fn object_mut(&mut self, id: u64) -> Result<&mut kernel::Object, Self::Error> {
        self.effects
            .unit_cleanup_runtime_mut()
            .map_err(Error::CleanupQuery)?
            .object_mut(id)
            .map_err(Error::CleanupQuery)?
            .unit_lifecycle
            .as_mut()
            .ok_or(Error::Missing("current Unit wrapper lifecycle"))
    }
    fn modifier(&self, id: u64) -> Result<&kernel::Modifier, Self::Error> {
        self.effects
            .unit_cleanup_runtime()
            .map_err(Error::CleanupQuery)?
            .unit_modifier(id)
            .map_err(Error::CleanupQuery)
    }
    fn modifier_mut(&mut self, id: u64) -> Result<&mut kernel::Modifier, Self::Error> {
        self.effects
            .unit_cleanup_runtime_mut()
            .map_err(Error::CleanupQuery)?
            .unit_modifier_mut(id)
            .map_err(Error::CleanupQuery)
    }
    fn object_order(&self, a: u64, b: u64) -> Result<std::cmp::Ordering, Self::Error> {
        let runtime = self
            .effects
            .unit_cleanup_runtime()
            .map_err(Error::CleanupQuery)?;
        Ok(object_key(runtime.object(a).map_err(Error::CleanupQuery)?)?
            .compare(object_key(runtime.object(b).map_err(Error::CleanupQuery)?)?))
    }
    fn clear_owned_modifier(&mut self, id: i32) -> Result<(), Self::Error> {
        self.effects
            .clear_owned_unit_modifier(self.state, self.sight, self.providers, id)
    }
    fn owner_destroyed(&mut self, receiver: u64) -> Result<(), Self::Error> {
        self.effects
            .unit_modifier_owner_destroyed(self.state, self.sight, self.providers, receiver)
    }
    fn unsubscribe_requirement(
        &mut self,
        receiver: u64,
        object: u64,
        set: u64,
    ) -> Result<(), Self::Error> {
        self.effects
            .unsubscribe_unit_requirement(
                self.state,
                self.sight,
                self.providers,
                receiver,
                object,
                set,
            )
    }
    fn release_requirement(&mut self, set: u64) -> Result<(), Self::Error> {
        self.effects
            .release_unit_requirement(self.state, self.sight, self.providers, set)
    }
    fn remove_effect(&mut self, effect: u64, object: u64) -> Result<bool, Self::Error> {
        let runtime = self
            .effects
            .unit_cleanup_runtime()
            .map_err(Error::CleanupQuery)?;
        let ContextObject::Unit(reference) =
            runtime.object(object).map_err(Error::CleanupQuery)?.kind
        else {
            return Err(Error::Missing("Unit effect subject interface class"));
        };
        let EffectState::Unit(effect) = runtime.effect(effect).map_err(Error::CleanupQuery)? else {
            return Err(Error::Missing("non-Unit effect in Unit cleanup"));
        };
        let effect = *effect;
        let Some(unit) = self.state.world().unit(reference).map_err(Error::World)? else {
            return Ok(false);
        };
        let actor = (unit.record.e_owner, unit.record.id);
        super::effects::remove_with_continuations(
            self.state,
            self.sight,
            self.providers,
            self.effects,
            self.definitions,
            actor,
            effect,
        )
    }
    fn log_removing(&mut self, receiver: u64, object: u64) -> Result<(), Self::Error> {
        self.effects
            .unit_cleanup_log(self.state, receiver, object, false)
    }
    fn log_failed_remove(&mut self, receiver: u64, object: u64) -> Result<(), Self::Error> {
        self.effects
            .unit_cleanup_log(self.state, receiver, object, true)
    }
}
fn dispatch<P: super::Providers, C: Services<P>>(
    state: &mut LoadedState,
    sight: &mut sight::Runtime,
    providers: &mut P,
    effects: &mut C,
    definitions: &Definitions,
    objects: &[u64],
) -> Result<(), Error<C::Error, EffectError<P>>> {
    kernel::detach(
        &mut Context {
            state,
            sight,
            providers,
            effects,
            definitions,
        },
        objects,
    )
    .map_err(|e| Error::ObjectCleanup(Box::new(e)))
}
/// GameEventHandler.OnUnitDestroyed_DetachGameEffects: all distinct matching
/// current Unit wrappers, in captured/current pool order. Relevant ownership
/// state must have been initialized in the canonical object map.
pub fn detach_unit<P: super::Providers, C: Services<P>>(
    state: &mut LoadedState,
    sight: &mut sight::Runtime,
    providers: &mut P,
    effects: &mut C,
    definitions: &Definitions,
    actor: ComponentId,
) -> Result<(), Error<C::Error, EffectError<P>>> {
    let runtime = effects
        .unit_cleanup_runtime()
        .map_err(Error::CleanupQuery)?;
    let mut objects = Vec::new();
    for (handle, object) in runtime.objects() {
        if let ContextObject::Unit(reference) = object.kind {
            if reference.component_id() == actor {
                let life = object
                    .unit_lifecycle
                    .as_ref()
                    .ok_or(Error::Missing("matching Unit wrapper lifecycle"))?;
                objects.push((life.pool_ordinal, *handle));
            }
        }
    }
    if objects.is_empty() {
        return Err(Error::Missing("Unit.MakeObject creation/cache binding"));
    }
    objects.sort_by_key(|r| r.0);
    let handles: Vec<_> = objects.into_iter().map(|r| r.1).collect();
    dispatch(state, sight, providers, effects, definitions, &handles)
}
/// Source-only ability-revoked GameEffects leaf. Resolves the existing wrapper
/// by CURRENT Unit CID and configured ability hash. An empty-owned wrapper may
/// have expired; that factory/liveness case is explicit rather than guessed.
pub fn revoke_ability<P: super::Providers, C: Services<P>>(
    state: &mut LoadedState,
    sight: &mut sight::Runtime,
    providers: &mut P,
    effects: &mut C,
    definitions: &Definitions,
    actor: ComponentId,
    ability: u32,
) -> Result<(), Error<C::Error, EffectError<P>>> {
    let index = definitions
        .abilities()
        .find_index(ability)
        .ok_or(Error::Missing("revoked ability definition index"))?;
    let hash = definitions
        .abilities()
        .type_hash(index)
        .ok_or(Error::Missing("revoked ability definition hash"))?;
    let runtime = effects
        .unit_cleanup_runtime()
        .map_err(Error::CleanupQuery)?;
    let mut found = None;
    for (handle, object) in runtime.objects() {
        if matches!(object.kind,ContextObject::Unit(reference) if reference.component_id()==actor)
            && object.unit_sort_suffix == Some([1, hash as i32])
        {
            if found.replace(*handle).is_some() {
                return Err(Error::Missing("duplicate ability wrapper key"));
            }
            let life = object
                .unit_lifecycle
                .as_ref()
                .ok_or(Error::Missing("ability wrapper lifecycle"))?;
            if life.owned_modifiers.is_empty() {
                return Err(Error::Missing("revoked ability wrapper factory/liveness"));
            }
        }
    }
    let handle = found.ok_or(Error::Missing("revoked ability Unit.MakeObject creation"))?;
    dispatch(state, sight, providers, effects, definitions, &[handle])
}
