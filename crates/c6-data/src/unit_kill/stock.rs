//! Stock Unit death continuations over the same live object, requirement and
//! collection owners. Callers supply typed initialized listener bindings.
//! Unsupported positive consumers stop at their actual position in dispatch.
pub mod default_defeat;
use crate::unit_placement_state as normal;
use crate::{
    melee_attack::backend::{EffectRuntime, QueryError},
    unit_initialization_definitions::Definitions,
    unit_kill::{self as kill, objects, Continuations, Providers},
    unit_sight_state as sight,
};
use c6_core::melee_attack::proposals::subject_sets::{self as shared, changes};
use c6_core::{loaded_state::LoadedState, unit::ComponentId};

type EffectError<P> = <P as sight::Effects>::Error;

#[derive(Debug)]
pub enum Error {
    Input(&'static str),
    Influence(String),
    Requirements(shared::Error<&'static str>),
    Collection(crate::melee_attack::proposals::collection_modifiers::DestroyCollectionError),
    ItemSignal(Box<c6_core::melee_attack::proposals::collections::Error<Error>>),
    UnknownSubscriber(u64),
}

/// Source-qualified stock listener kinds in initialized registration order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Listener {
    GameEffectsDetach,
    TradeNonTraderGuard,
    Influence {
        owner: i32,
    },
    MilitaryOwnerGuard {
        owner: i32,
    },
    PlayerUnits {
        owner: i32,
        collection: u64,
    },
    AllUnits {
        collection: u64,
    },
    DefaultDefeatFilter {
        receiver: u64,
        owner: i32,
        unit_type: i32,
    },
    Unsupported(u64),
}
/// Logical destructor handles; these are not serialized/native byte offsets.
#[derive(Clone, Debug)]
pub struct ModifierCleanup {
    pub instance: i32,
    pub receiver: u64,
    pub owner_requirement: Option<u64>,
    pub requirement_subscription: bool,
    pub collection: Option<u64>,
    pub collection_changed_subscription: bool,
    pub collection_added_subscription: bool,
    pub collection_lifecycle_ret: bool,
}
#[derive(Clone, Debug)]
pub struct RequirementCleanup {
    pub label: u64,
    pub provider_receiver: u64,
    pub global_unit_moved: bool,
    pub unit_scopes: Vec<ComponentId>,
}
#[derive(Clone, Debug)]
pub struct Metadata {
    pub death_listeners: Vec<Listener>,
    pub ability_listeners: Vec<Listener>,
    pub modifiers: Vec<ModifierCleanup>,
    pub requirements: Vec<RequirementCleanup>,
    /// Actual current log threshold. Missing input is not a disabled logger.
    pub logging_priority: Option<i32>,
}
/// Visit the initialized sequence once. In particular, DefaultDefeat may precede
/// AllUnits after fresh registrations; subscriber classes must never be grouped.
fn owner_of(listener: Listener) -> Option<i32> {
    match listener {
        Listener::PlayerUnits { owner, .. } => Some(owner),
        _ => None,
    }
}
fn ordered<E>(
    listeners: &[Listener],
    mut invoke: impl FnMut(Listener) -> Result<(), E>,
) -> Result<(), E> {
    for listener in listeners {
        invoke(*listener)?;
    }
    Ok(())
}
pub trait RequirementOwner {
    fn requirements(&self) -> &shared::Runtime;
    fn requirements_mut(&mut self) -> &mut shared::Runtime;
    /// Invalidate through the same requirement/modifier owner. Implementations
    /// query current canonical Cities/Units; snapshot city counts are not inputs.
    fn invalidate_default_defeat(
        &mut self,
        _state: &mut LoadedState,
        _effects: &mut EffectRuntime,
        _receiver: u64,
    ) -> Result<(), Error> {
        Err(Error::Input(
            "matching default defeat requirement invalidation",
        ))
    }
}
impl RequirementOwner for crate::melee_attack::backend::requirements::Runtime {
    fn requirements(&self) -> &shared::Runtime {
        self.sets()
    }
    fn requirements_mut(&mut self) -> &mut shared::Runtime {
        self.sets_mut()
    }
}
struct Release<'a, R, P> {
    requirements: &'a mut R,
    providers: &'a mut P,
    input: &'a Metadata,
}
impl<R: RequirementOwner, P: normal::Effects> changes::Context for Release<'_, R, P> {
    type Error = &'static str;
    fn subject_sets(&self) -> Result<&shared::Runtime, Self::Error> {
        Ok(self.requirements.requirements())
    }
    fn subject_sets_mut(&mut self) -> Result<&mut shared::Runtime, Self::Error> {
        Ok(self.requirements.requirements_mut())
    }
    fn test_requirement(&mut self, _: u64) -> Result<i32, shared::Error<Self::Error>> {
        Err(shared::Error::Context(
            "unexpected Test during current-set destruction",
        ))
    }
    fn modifier_changed(
        &mut self,
        _: shared::SetListener,
        _: u64,
        _: i32,
        _: i32,
    ) -> Result<(), shared::Error<Self::Error>> {
        Err(shared::Error::Context(
            "unexpected state change during current-set destruction",
        ))
    }
    fn reset_trigger(&mut self, _: u64) -> Result<(), shared::Error<Self::Error>> {
        Err(shared::Error::Context(
            "unexpected trigger reset during destruction",
        ))
    }
    fn destroy_set(&mut self, set: u64) -> Result<(), shared::Error<Self::Error>> {
        if self.requirements.requirements().set_has_subscribers(set) != Some(false) {
            return Err(shared::Error::Context(
                "current set retains unknown destruction listeners",
            ));
        }
        // Native stock SimpleRequirementSet destructor has no changed dispatch.
        // Child vectors/handles have already been removed by the shared kernel;
        // definition and stock Unit context/subject releases add no gameplay call.
        Ok(())
    }
    fn destroy_requirement(&mut self, label: u64) -> Result<(), shared::Error<Self::Error>> {
        use c6_core::unit_moved_signal::Subscriber;
        let row = self
            .input
            .requirements
            .iter()
            .find(|r| r.label == label)
            .ok_or(shared::Error::Context("current child destructor binding"))?;
        let receiver = row.provider_receiver;
        if row.global_unit_moved {
            let ring = self
                .providers
                .unit_moved_registrations_mut()
                .map_err(|_| shared::Error::Context("current UnitMoved registration owner"))?
                .global_mut()
                .map_err(shared::Error::Ring)?;
            let token = ring
                .subscription_for(|route| route == Subscriber::Requirement { receiver })
                .map_err(shared::Error::Ring)?;
            ring.unsubscribe(token).map_err(shared::Error::Ring)?;
        }
        for cid in &row.unit_scopes {
            let cid = *cid;
            let ring = self
                .providers
                .unit_moved_registrations_mut()
                .map_err(|_| shared::Error::Context("current UnitMoved registration owner"))?
                .unit_mut(cid)
                .map_err(shared::Error::Ring)?
                .ok_or(shared::Error::Context("current UnitMoved scoped signal"))?;
            let token = ring
                .subscription_for(|route| route == Subscriber::Requirement { receiver })
                .map_err(shared::Error::Ring)?;
            ring.unsubscribe(token).map_err(shared::Error::Ring)?;
        }
        Ok(())
    }
}
// Creation and combat registrations can coexist, but each collection identity
// has one mutable owner. An override does not replace unrelated combat owners.
fn collection_owner<'s>(
    state: &'s mut LoadedState,
    current: Option<&'s mut c6_core::melee_attack::proposals::collections::Runtime>,
    collection: u64,
) -> Result<&'s mut c6_core::melee_attack::proposals::collections::Runtime, &'static str> {
    let current_has = current
        .as_ref()
        .is_some_and(|owner| owner.collection(collection).is_some());
    let combat_has = state
        .combat_proposals()
        .ok()
        .and_then(|owner| owner.collections().ok())
        .is_some_and(|owner| owner.collection(collection).is_some());
    match (current_has, combat_has) {
        (true, true) => return Err("duplicate current collection owners"),
        (true, false) => return Ok(current.expect("current collection found")),
        (false, false) => return Err("missing current collection identity"),
        (false, true) => {}
    }
    state
        .combat_proposals_mut()
        .map_err(|_| "current proposal owner")?
        .collections_mut()
        .map_err(|_| "current collection owner")
}
struct UnitItemRemoval<'a> {
    collections: &'a mut c6_core::melee_attack::proposals::collections::Runtime,
    effects: &'a EffectRuntime,
    input: &'a Metadata,
}
impl c6_core::melee_attack::proposals::collections::Context<u64> for UnitItemRemoval<'_> {
    type Error = Error;
    fn collections(
        &self,
    ) -> Result<&c6_core::melee_attack::proposals::collections::Runtime, Error> {
        Ok(self.collections)
    }
    fn collections_mut(
        &mut self,
    ) -> Result<&mut c6_core::melee_attack::proposals::collections::Runtime, Error> {
        Ok(self.collections)
    }
    fn item_changed(&mut self, receiver: u64, object: &mut u64, added: bool) -> Result<(), Error> {
        if added {
            return Err(Error::Input("Unit death collection addition"));
        }
        let row = self
            .input
            .modifiers
            .iter()
            .find(|row| row.receiver == receiver)
            .ok_or(Error::Input("current collection modifier binding"))?;
        if !row.collection_lifecycle_ret {
            return Err(Error::Input("current collection effect lifecycle"));
        }
        let modifier = self
            .effects
            .unit_modifier(receiver)
            .map_err(|_| Error::Input("current Unit modifier owner"))?;
        let object_key = |handle| -> Result<_, Error> {
            let object = self
                .effects
                .object(handle)
                .map_err(|_| Error::Input("current collection object"))?;
            let crate::melee_attack::backend::ContextObject::Unit(unit) = object.kind else {
                return Err(Error::Input("collection Unit interface"));
            };
            let suffix = object
                .unit_sort_suffix
                .ok_or(Error::Input("collection Unit sort suffix"))?;
            Ok(c6_core::unit_moved_signal::modifiers::UnitObjectKey {
                component: unit.component_id(),
                first: suffix[0],
                second: suffix[1],
            })
        };
        let key = object_key(*object)?;
        for entry in &modifier.tracked {
            if object_key(entry.object)?.compare(key) == std::cmp::Ordering::Equal {
                return Err(Error::Input("current collection tracked Unit removal"));
            }
        }
        // A failed tracked lookup returns. Stock Before/After hooks are inert;
        // earlier modifier guards only read current control/requirement state.
        Ok(())
    }
}
pub type Influence<'a> = dyn FnMut(
        &mut LoadedState,
        &sight::Runtime,
        &crate::unit_placement_services::Runtime,
        i32,
    ) -> Result<(), Error>
    + 'a;
pub struct Stock<'a, R> {
    effects: &'a mut EffectRuntime,
    _requirements: &'a mut R,
    definitions: &'a Definitions,
    input: &'a Metadata,
    unit_definitions: Option<
        &'a c6_core::movement_world::DefinitionTable<
            'a,
            Option<c6_core::skip_turn::placement::UnitProperties>,
        >,
    >,
    influence: Option<&'a mut Influence<'a>>,
    collections: Option<&'a mut c6_core::melee_attack::proposals::collections::Runtime>,
}
impl<'a, R> Stock<'a, R> {
    pub fn new(
        effects: &'a mut EffectRuntime,
        requirements: &'a mut R,
        definitions: &'a Definitions,
        input: &'a Metadata,
    ) -> Result<Self, Error> {
        if input.death_listeners.first() != Some(&Listener::GameEffectsDetach)
            || input.ability_listeners != [Listener::GameEffectsDetach]
        {
            return Err(Error::Input("stock GameEffects death/ability registration"));
        }
        Ok(Self {
            effects,
            _requirements: requirements,
            definitions,
            input,
            unit_definitions: None,
            influence: None,
            collections: None,
        })
    }
    /// Borrow an existing generic collection owner when it is not hosted by
    /// combat proposals. This is the same live owner used by City/Unit callbacks.
    pub fn with_collections(
        mut self,
        collections: &'a mut c6_core::melee_attack::proposals::collections::Runtime,
    ) -> Self {
        self.collections = Some(collections);
        self
    }
    pub fn with_influence(mut self, callback: &'a mut Influence<'a>) -> Self {
        self.influence = Some(callback);
        self
    }
    pub fn with_unit_definitions(
        mut self,
        definitions: &'a c6_core::movement_world::DefinitionTable<
            'a,
            Option<c6_core::skip_turn::placement::UnitProperties>,
        >,
    ) -> Self {
        self.unit_definitions = Some(definitions);
        self
    }
}
impl<P: Providers + normal::Effects, R: RequirementOwner> Continuations<P> for Stock<'_, R> {
    type Error = Error;
    fn removed(
        &mut self,
        state: &mut LoadedState,
        sight: &mut sight::Runtime,
        providers: &mut P,
        actor: (i32, u32),
    ) -> Result<(), kill::Error<Error, EffectError<P>>> {
        let definitions = self.definitions;
        objects::detach_unit(
            state,
            sight,
            providers,
            self,
            definitions,
            ComponentId {
                owner: actor
                    .0
                    .try_into()
                    .map_err(|_| kill::Error::Missing("Unit owner range"))?,
                component_type: 1,
                id: actor.1,
            },
        )?;
        ordered(&self.input.death_listeners[1..], |route| {
            match route {
                Listener::TradeNonTraderGuard => {
                    let definitions = self
                        .unit_definitions
                        .ok_or(kill::Error::Missing("current Trade Unit definitions"))?;
                    let runtime = state
                        .unit_runtime(actor.0, actor.1)
                        .map_err(kill::Error::State)?;
                    let definition = definitions
                        .at(runtime.definition_index)
                        .and_then(Option::as_ref)
                        .ok_or(kill::Error::Missing("current Trade Unit definition"))?;
                    if definition.make_trade_route {
                        return Err(kill::Error::Missing("Trade route destruction"));
                    }
                }
                Listener::Influence { owner } => {
                    if state
                        .player_slot_status(owner)
                        .map_err(kill::Error::State)?
                        == 3
                        && state.turn_active(owner).map_err(kill::Error::State)?
                    {
                        let callback = self
                            .influence
                            .as_mut()
                            .ok_or(kill::Error::Missing("current Influence callback provider"))?;
                        let services = providers
                            .placement_service_runtime()
                            .map_err(kill::Error::Services)?;
                        callback(state, sight, services, owner).map_err(kill::Error::Effect)?;
                    }
                }
                Listener::MilitaryOwnerGuard { owner } => {
                    if owner == actor.0 {
                        return Err(kill::Error::Missing(
                            "matching military unit requirement invalidation",
                        ));
                    }
                }
                Listener::PlayerUnits { collection, .. } | Listener::AllUnits { collection }
                    if matches!(route, Listener::AllUnits { .. })
                        || owner_of(route) == Some(actor.0) =>
                {
                    let object = self
                        .effects
                        .make_current_unit_object(
                            state,
                            c6_core::map::UnitReference::from_component_id(ComponentId {
                                owner: actor
                                    .0
                                    .try_into()
                                    .map_err(|_| kill::Error::Missing("Unit owner range"))?,
                                component_type: 1,
                                id: actor.1,
                            }),
                            [0, 0],
                        )
                        .map_err(kill::Error::CleanupQuery)?;
                    c6_core::melee_attack::proposals::collections::remove_typed_item(
                        &mut UnitItemRemoval {
                            collections: collection_owner(
                                state,
                                self.collections.as_deref_mut(),
                                collection,
                            )
                            .map_err(kill::Error::Missing)?,
                            effects: self.effects,
                            input: self.input,
                        },
                        collection,
                        object,
                    )
                    .map_err(|e| kill::Error::Effect(Error::ItemSignal(Box::new(e))))?;
                }
                Listener::PlayerUnits { .. } => {}
                Listener::DefaultDefeatFilter {
                    receiver,
                    owner,
                    unit_type,
                } => {
                    if owner == actor.0
                        && (unit_type == -1
                            || state
                                .unit_runtime(actor.0, actor.1)
                                .map_err(kill::Error::State)?
                                .definition_index
                                == unit_type)
                    {
                        self._requirements
                            .invalidate_default_defeat(state, self.effects, receiver)
                            .map_err(kill::Error::Effect)?;
                    }
                }
                Listener::Unsupported(label) => {
                    return Err(kill::Error::Effect(Error::UnknownSubscriber(label)))
                }
                _ => return Err(kill::Error::Missing("unexpected stock death registration")),
            }
            Ok(())
        })?;
        Ok(())
    }
    fn ability_lost(
        &mut self,
        state: &mut LoadedState,
        sight: &mut sight::Runtime,
        providers: &mut P,
        actor: (i32, u32),
        ability: i32,
    ) -> Result<(), kill::Error<Error, EffectError<P>>> {
        let definitions = self.definitions;
        objects::revoke_ability(
            state,
            sight,
            providers,
            self,
            definitions,
            ComponentId {
                owner: actor
                    .0
                    .try_into()
                    .map_err(|_| kill::Error::Missing("Unit owner range"))?,
                component_type: 1,
                id: actor.1,
            },
            u32::try_from(ability).map_err(|_| kill::Error::Missing("ability index range"))?,
        )
    }
}
impl<P: Providers + normal::Effects, R: RequirementOwner> objects::Services<P> for Stock<'_, R> {
    fn clear_owned_unit_modifier(
        &mut self,
        state: &mut LoadedState,
        _sight: &mut sight::Runtime,
        _providers: &mut P,
        instance: i32,
    ) -> Result<(), kill::Error<Error, EffectError<P>>> {
        let row = self
            .input
            .modifiers
            .iter()
            .find(|r| r.instance == instance)
            .ok_or(kill::Error::Missing("owned modifier current instance"))?;
        let receiver = row.receiver;
        // Keep the removed registration alive through its source destructor.
        enum Retained {
            Combat(c6_core::melee_attack::proposals::collection_modifiers::Modifier),
            Unit(c6_core::unit_kill::objects::Modifier),
        }
        let combat = state
            .combat_proposals()
            .ok()
            .and_then(|p| p.collection_modifiers().ok())
            .and_then(|m| m.modifier(receiver));
        let retained = if let Some(current) = combat {
            if current.instance != instance
                || !current.tracked.is_empty()
                || !current.applied.is_empty()
            {
                return Err(kill::Error::Missing(
                    "owned combat modifier current destructor inputs",
                ));
            }
            Retained::Combat(
                state
                    .combat_proposals_mut()
                    .map_err(|_| kill::Error::Missing("current proposal owner"))?
                    .collection_modifiers_mut()
                    .map_err(|_| kill::Error::Missing("current proposal modifiers"))?
                    .remove_registration(receiver)
                    .ok_or(kill::Error::Missing("owned combat modifier removal"))?,
            )
        } else {
            let current = self
                .effects
                .unit_modifier(receiver)
                .map_err(kill::Error::CleanupQuery)?;
            if current.instance != instance
                || current.enabled
                || !current.tracked.is_empty()
                || !current.applied.is_empty()
            {
                return Err(kill::Error::Missing(
                    "active owned noncombat modifier destructor",
                ));
            }
            Retained::Unit(
                self.effects
                    .remove_unit_modifier_registration(receiver)
                    .map_err(kill::Error::CleanupQuery)?,
            )
        };
        let owner_ptr = match &retained {
            Retained::Combat(m) => m.context,
            Retained::Unit(m) => m.context,
        };
        let owner_requirement = row.owner_requirement;
        // Native destructor first unsubscribes owner+218. This ring is already
        // the canonical live Unit wrapper, including nested dispatch cursors.
        let owner = self
            .effects
            .object_mut(owner_ptr)
            .map_err(kill::Error::CleanupQuery)?;
        let ring = &mut owner
            .unit_lifecycle
            .as_mut()
            .ok_or(kill::Error::Missing("owned modifier Unit lifecycle"))?
            .destroyed;
        let token = ring
            .subscription_for(|route| {
                route == c6_core::unit_kill::objects::Callback::OwnerDestroyed(receiver)
            })
            .map_err(|_| kill::Error::Missing("actual owner destruction subscription"))?;
        ring.unsubscribe(token)
            .map_err(|_| kill::Error::Missing("owner destruction unsubscribe"))?;
        // Release subscriptions through the SAME requirements/collection owners.
        if row.requirement_subscription {
            let set = owner_requirement
                .ok_or(kill::Error::Missing("owned requirement handle identity"))?;
            self._requirements
                .requirements_mut()
                .unsubscribe::<&'static str>(receiver, set)
                .map_err(|e| kill::Error::Effect(Error::Requirements(e)))?;
        }
        let collection = row.collection;
        if row.collection_changed_subscription || row.collection_added_subscription {
            let collection =
                collection.ok_or(kill::Error::Missing("modifier collection handle"))?;
            let collections = collection_owner(state, self.collections.as_deref_mut(), collection)
                .map_err(kill::Error::Missing)?;
            if row.collection_changed_subscription {
                collections
                    .unsubscribe_changed_receiver(collection, receiver)
                    .map_err(|_| kill::Error::Missing("owned collection changed unsubscribe"))?;
            }
            if row.collection_added_subscription {
                collections
                    .unsubscribe_added_receiver(collection, receiver)
                    .map_err(|_| kill::Error::Missing("owned collection item unsubscribe"))?;
            }
        }
        if self
            .input
            .logging_priority
            .is_none_or(|priority| priority >= 2)
        {
            return Err(kill::Error::Missing(
                "enabled owned modifier detach logging",
            ));
        }
        // Stock Unit::ToString and DynamicModifier::ToString use only current
        // scalar getters and formatting-owned strings (the latter caches only
        // its rendered output). There are no gameplay/RNG callbacks in them.
        // Deactivate has no tracked/applied subjects here; its concrete effect
        // Before/After slots are the already-qualified PlayerStrength RETs.
        let owner = self
            .effects
            .object_mut(owner_ptr)
            .map_err(kill::Error::CleanupQuery)?;
        owner
            .unit_lifecycle
            .as_mut()
            .ok_or(kill::Error::Missing("current modifier owner"))?
            .owned_modifiers
            .retain(|id| *id != instance);
        if let Some(owner_requirement) = owner_requirement {
            changes::release(
                &mut Release {
                    requirements: self._requirements,
                    providers: _providers,
                    input: self.input,
                },
                owner_requirement,
            )
            .map_err(|e| kill::Error::Effect(Error::Requirements(e)))?;
        }
        if let Some(collection) = collection {
            let factory_owned = self
                .collections
                .as_ref()
                .is_some_and(|owner| owner.collection(collection).is_some());
            let destroyed = collection_owner(state, self.collections.as_deref_mut(), collection)
                .map_err(kill::Error::Missing)?
                .release_owner(collection)
                .map_err(|_| kill::Error::Missing("current collection lifetime"))?;
            if destroyed {
                if factory_owned {
                    return Err(kill::Error::Missing(
                        "generic Unit collection destruction continuation",
                    ));
                }
                crate::melee_attack::proposals::collection_modifiers::destroy_unit_combat_collection(state,collection)
                    .map_err(|e|kill::Error::Effect(Error::Collection(e)))?;
            }
        }
        Ok(())
    }
    fn unsubscribe_unit_requirement(
        &mut self,
        _state: &mut LoadedState,
        _sight: &mut sight::Runtime,
        _providers: &mut P,
        receiver: u64,
        _object: u64,
        set: u64,
    ) -> Result<(), kill::Error<Error, EffectError<P>>> {
        self._requirements
            .requirements_mut()
            .unsubscribe::<&'static str>(receiver, set)
            .map_err(|e| kill::Error::Effect(Error::Requirements(e)))
    }
    fn release_unit_requirement(
        &mut self,
        _state: &mut LoadedState,
        _sight: &mut sight::Runtime,
        providers: &mut P,
        set: u64,
    ) -> Result<(), kill::Error<Error, EffectError<P>>> {
        changes::release(
            &mut Release {
                requirements: self._requirements,
                providers,
                input: self.input,
            },
            set,
        )
        .map_err(|e| kill::Error::Effect(Error::Requirements(e)))
    }
    fn unit_cleanup_log(
        &mut self,
        _state: &LoadedState,
        _receiver: u64,
        _object: u64,
        _failed: bool,
    ) -> Result<(), kill::Error<Error, EffectError<P>>> {
        match self.input.logging_priority {
            Some(priority) if priority < 3 => Ok(()),
            _ => Err(kill::Error::Missing("enabled Unit Remove effect logging")),
        }
    }
    fn unit_cleanup_runtime(&self) -> Result<&EffectRuntime, QueryError> {
        Ok(self.effects)
    }
    fn unit_cleanup_runtime_mut(&mut self) -> Result<&mut EffectRuntime, QueryError> {
        Ok(self.effects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interleaved_defeat_precedes_late_all_units_and_failure_stops_there() {
        let routes = [
            Listener::PlayerUnits {
                owner: 0,
                collection: 10,
            },
            Listener::DefaultDefeatFilter {
                receiver: 40,
                owner: 0,
                unit_type: 7,
            },
            Listener::PlayerUnits {
                owner: 1,
                collection: 20,
            },
            Listener::AllUnits { collection: 30 },
        ];
        let mut seen = Vec::new();
        ordered::<()>(&routes, |r| {
            seen.push(r);
            Ok(())
        })
        .unwrap();
        assert_eq!(seen, routes);
        seen.clear();
        let failed = ordered(&routes, |r| {
            seen.push(r);
            if matches!(r, Listener::DefaultDefeatFilter { .. }) {
                Err("required defeat continuation")
            } else {
                Ok(())
            }
        });
        assert_eq!(failed, Err("required defeat continuation"));
        assert_eq!(seen, routes[..2]);
    }
}
