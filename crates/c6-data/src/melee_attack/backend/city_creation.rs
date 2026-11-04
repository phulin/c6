//! Positive City modifier creation in the existing GameEffects owner.
//!
//! Native pointer allocation is not logical identity. Modifier instances use the
//! actual ModifierSystem counter; cached City objects keep their source IDs.
pub mod definitions;
use super::{EffectRuntime, QueryError};
use c6_core::{
    loaded_state::LoadedState,
    unit_moved_signal::{modifiers::AffectedModifiers, ListenerRing, RegistryError, Subscription},
};

#[derive(Clone, Copy, Debug)]
pub enum DestroyedListener {
    Existing(u64),
    ModifierOwner(i32),
    Modifier(i32),
}
#[derive(Clone, Copy, Debug)]
pub enum LoyaltyListener {
    Existing(u64),
    Requirement(u64),
}
/// Observed initial object domain, including all existing native subscriptions.
/// Empty vectors must come from actual empty source rings, not missing inputs.
pub struct InitialObject {
    pub handle: u64,
    pub affected: AffectedModifiers,
    pub destroyed: Vec<DestroyedListener>,
}
pub struct Initial {
    pub next_modifier_id: i32,
    pub existing_modifier_ids: Vec<i32>,
    pub objects: Vec<InitialObject>,
    pub loyalty: Vec<LoyaltyListener>,
}
#[derive(Debug)]
pub enum Error {
    Query(QueryError),
    Ring(RegistryError),
    Missing(&'static str),
    DuplicateModifier(i32),
    Requirement(Box<c6_core::melee_attack::proposals::subject_sets::Error<Error>>),
    Effect(super::CityEffectError),
}
struct Object {
    handle: u64,
    affected: AffectedModifiers,
    destroyed: ListenerRing<DestroyedListener>,
}
/// This state is a child of EffectRuntime, not another modifier registry.
pub(super) struct Creation {
    next_modifier_id: i32,
    modifiers: Vec<Dynamic>,
    existing_modifier_ids: Vec<i32>,
    objects: Vec<Object>,
    loyalty: ListenerRing<LoyaltyListener>,
    loyalty_subscriptions: Vec<(u64, Subscription)>,
}
impl EffectRuntime {
    /// Import only the source-backed creation domains needed by reached City
    /// factories. Other modifier fields remain unavailable in their owner.
    pub fn install_city_creation(
        &mut self,
        state: &LoadedState,
        initial: Initial,
    ) -> Result<(), Error> {
        self.current_owner(state).map_err(Error::Query)?;
        if self.city_creation.is_some() {
            return Err(Error::Missing("City creation already installed"));
        }
        for (index, id) in initial.existing_modifier_ids.iter().enumerate() {
            if initial.existing_modifier_ids[..index].contains(id) {
                return Err(Error::DuplicateModifier(*id));
            }
        }
        let mut objects = Vec::new();
        for object in initial.objects {
            self.object(object.handle).map_err(Error::Query)?;
            if objects
                .iter()
                .any(|old: &Object| old.handle == object.handle)
            {
                return Err(Error::Missing("duplicate City creation object"));
            }
            objects.push(Object {
                handle: object.handle,
                affected: object.affected,
                destroyed: ListenerRing::from_ordered(&object.destroyed).map_err(Error::Ring)?,
            });
        }
        self.city_creation = Some(Creation {
            next_modifier_id: initial.next_modifier_id,
            modifiers: Vec::new(),
            existing_modifier_ids: initial.existing_modifier_ids,
            objects,
            loyalty: ListenerRing::from_ordered(&initial.loyalty).map_err(Error::Ring)?,
            loyalty_subscriptions: Vec::new(),
        });
        Ok(())
    }
}

struct RequirementInitializer<'a> {
    state: &'a LoadedState,
    effects: &'a mut EffectRuntime,
    requirements: &'a mut super::requirements::Runtime,
    maximum_loyalty: i32,
}
impl c6_core::melee_attack::proposals::subject_sets::SubjectContext for RequirementInitializer<'_> {
    type Error = Error;
    fn subject_sets(
        &self,
    ) -> Result<&c6_core::melee_attack::proposals::subject_sets::Runtime, Error> {
        Ok(self.requirements.sets())
    }
    fn subject_sets_mut(
        &mut self,
    ) -> Result<&mut c6_core::melee_attack::proposals::subject_sets::Runtime, Error> {
        Ok(self.requirements.sets_mut())
    }
    fn initialize_requirement(
        &mut self,
        label: u64,
        definition: &c6_core::melee_attack::proposals::subject_sets::RequirementDefinition,
        _: i32,
        subject: c6_core::melee_attack::proposals::subject_sets::Subject,
    ) -> Result<
        c6_core::melee_attack::proposals::subject_sets::InitializedRequirement,
        c6_core::melee_attack::proposals::subject_sets::Error<Error>,
    > {
        use c6_core::melee_attack::proposals::subject_sets::{self as sets, Error as E};
        if definition.predicate
            != sets::Predicate::Unsupported(crate::registry::linux_type_hash(
                b"REQUIREMENT_CITY_HAS_FULL_LOYALTY",
            ))
        {
            return Err(E::Context(Error::Missing(
                "unsupported City requirement factory",
            )));
        }
        let sets::Subject::ModifierObject(handle) = subject else {
            return Err(E::UnsupportedSubject);
        };
        let object = self
            .effects
            .object(handle)
            .map_err(|e| E::Context(Error::Query(e)))?;
        let super::ContextObject::City(component) = object.kind else {
            return Err(E::UnsupportedSubject);
        };
        // OnInitialize23b0970 appends to current GameSignals+348 BEFORE the
        // SimpleRequirement initial Test. The shared set child subscribes later.
        let creation = self
            .effects
            .city_creation
            .as_mut()
            .ok_or(E::Context(Error::Missing("current City creation domain")))?;
        let token = creation
            .loyalty
            .subscribe(LoyaltyListener::Requirement(label))
            .map_err(|e| E::Context(Error::Ring(e)))?;
        creation.loyalty_subscriptions.push((label, token));
        let city = self
            .state
            .city(i32::from(component.owner), component.id)
            .map_err(|_| E::Context(Error::Missing("current loyalty City")))?;
        let tested = crate::city_production_turn::modifier_effects::test_full_loyalty(
            Some(city),
            Some(self.maximum_loyalty),
        );
        Ok(sets::InitializedRequirement::Simple(
            c6_core::simple_requirement::Requirement {
                active: true,
                state: if definition.inverse {
                    sets::inverse(tested)
                } else {
                    tested
                },
                inverse: definition.inverse,
                persistent: definition.persistent,
                // This supported factory's static definition compiler rejects the
                // separate Triggered flag; persistent and triggered are distinct.
                triggered: false,
            },
        ))
    }
    fn log_missing_set(&mut self, _: &[u8]) -> Result<(), Error> {
        Ok(())
    }
    fn log_created_requirement(&mut self, _: u32) -> Result<(), Error> {
        Ok(())
    }
    fn log_created_set(&mut self, _: u32) -> Result<(), Error> {
        Ok(())
    }
}

struct Dynamic {
    identity: u64,
    instance: i32,
    owner: u64,
    effect: u64,
    definition: Vec<u8>,
    modifier: c6_core::unit_moved_signal::modifiers::Modifier,
    tracked: Option<c6_core::unit_moved_signal::modifiers::TrackedSubject>,
    owner_destruction: Subscription,
    destruction: Option<Subscription>,
    applied_subjects: Vec<u64>,
    activated: bool,
}
impl Creation {
    fn object_mut(&mut self, handle: u64) -> Result<&mut Object, Error> {
        self.objects
            .iter_mut()
            .find(|v| v.handle == handle)
            .ok_or(Error::Missing("current City modifier-object lifecycle"))
    }
    fn assign_instance(&mut self) -> Result<i32, Error> {
        let id = self.next_modifier_id;
        if self.existing_modifier_ids.contains(&id)
            || self.modifiers.iter().any(|v| v.instance == id)
        {
            return Err(Error::DuplicateModifier(id));
        }
        // 232f440: post-increment the source i32 counter immediately before
        // SetInstanceId, insertion into the manager map and optional Activate.
        self.next_modifier_id = id.wrapping_add(1);
        Ok(id)
    }
}

struct Apply<'a, E> {
    state: &'a mut LoadedState,
    runtime: &'a mut EffectRuntime,
    requirements: &'a mut super::requirements::Runtime,
    definition: &'a definitions::Attachment,
    maximum_loyalty: i32,
    index: usize,
    effects: &'a mut E,
}
impl<E> Apply<'_, E> {
    fn creation(&self) -> Result<&Creation, Error> {
        self.runtime
            .city_creation
            .as_ref()
            .ok_or(Error::Missing("current City creation domain"))
    }
    fn creation_mut(&mut self) -> Result<&mut Creation, Error> {
        self.runtime
            .city_creation
            .as_mut()
            .ok_or(Error::Missing("current City creation domain"))
    }
    fn record(&self) -> Result<&Dynamic, Error> {
        self.creation()?
            .modifiers
            .get(self.index)
            .ok_or(Error::Missing("current created City modifier"))
    }
    fn record_mut(&mut self) -> Result<&mut Dynamic, Error> {
        let index = self.index;
        self.creation_mut()?
            .modifiers
            .get_mut(index)
            .ok_or(Error::Missing("current created City modifier"))
    }
}
impl<E: crate::city_production_turn::modifier_effects::Effects>
    c6_core::unit_moved_signal::modifiers::Context for Apply<'_, E>
{
    type Error = Error;
    fn modifier(&self) -> Result<&c6_core::unit_moved_signal::modifiers::Modifier, Error> {
        Ok(&self.record()?.modifier)
    }
    fn tracked(&self) -> Result<&c6_core::unit_moved_signal::modifiers::TrackedSubject, Error> {
        self.record()?
            .tracked
            .as_ref()
            .ok_or(Error::Missing("tracked City"))
    }
    fn tracked_mut(
        &mut self,
    ) -> Result<&mut c6_core::unit_moved_signal::modifiers::TrackedSubject, Error> {
        self.record_mut()?
            .tracked
            .as_mut()
            .ok_or(Error::Missing("tracked City"))
    }
    fn requirement_set_state(&self, id: u64) -> Result<i32, Error> {
        self.requirements
            .sets()
            .state(id)
            .ok_or(Error::Missing("current City requirement set"))
    }
    fn ensure_applied_subject(&mut self) -> Result<(), Error> {
        let owner = self.record()?.owner;
        if !self.record()?.applied_subjects.contains(&owner) {
            self.record_mut()?.applied_subjects.push(owner);
        }
        Ok(())
    }
    fn is_repeatable(&self) -> Result<bool, Error> {
        Ok(self.definition.repeatable)
    }
    fn apply_effect(&mut self) -> Result<bool, Error> {
        c6_core::unit_moved_signal::modifiers::apply_effect(self).map_err(|error| match error {
            c6_core::unit_moved_signal::modifiers::EffectError::Context(error) => error,
            c6_core::unit_moved_signal::modifiers::EffectError::DefinitionDisappeared => {
                Error::Missing("City modifier definition")
            }
        })?;
        Ok(true)
    }
    fn unsubscribe_subject_requirement(&mut self) -> Result<(), Error> {
        let record = self.record()?;
        let set = record
            .tracked
            .as_ref()
            .and_then(|v| v.requirement_set)
            .ok_or(Error::Missing("subject requirement subscription"))?;
        let receiver = record.identity;
        self.requirements
            .sets_mut()
            .unsubscribe(receiver, set)
            .map_err(|e| Error::Requirement(Box::new(e)))
    }
    fn clear_subject_requirement(&mut self) -> Result<(), Error> {
        Err(Error::Missing(
            "City dynamic requirement destruction continuation",
        ))
    }
    fn remove_effects(&mut self) -> Result<(), Error> {
        Err(Error::Missing("City modifier removal continuation"))
    }
}
impl<E: crate::city_production_turn::modifier_effects::Effects>
    c6_core::unit_moved_signal::modifiers::EffectContext for Apply<'_, E>
{
    type Error = Error;
    fn mark_subject_affected(&mut self) -> Result<(), Error> {
        let record = self.record()?;
        let (owner, id, definition) = (record.owner, record.instance, record.definition.clone());
        self.creation_mut()?
            .object_mut(owner)?
            .affected
            .mark(&definition, id)
            .map_err(|_| Error::Missing("affected City modifier ordering"))
    }
    fn suppressed(&self) -> Result<bool, Error> {
        Ok(self.record()?.modifier.suppressed)
    }
    fn subject_stack_limit(&self) -> Result<Option<u16>, Error> {
        Ok(Some(self.definition.subject_stack_limit))
    }
    fn affected_count(&self) -> Result<i32, Error> {
        let record = self.record()?;
        Ok(self
            .creation()?
            .objects
            .iter()
            .find(|v| v.handle == record.owner)
            .ok_or(Error::Missing("current affected City"))?
            .affected
            .count(&record.definition))
    }
    fn log_applying(&mut self) -> Result<(), Error> {
        Ok(())
    }
    fn log_failed(&mut self) -> Result<(), Error> {
        Ok(())
    }
    fn apply_concrete_effect(&mut self) -> Result<bool, Error> {
        let record = self.record()?;
        let (context, effect) = self
            .runtime
            .city_effect(record.effect)
            .map_err(Error::Query)?;
        let super::ContextObject::City(city) =
            self.runtime.object(context).map_err(Error::Query)?.kind
        else {
            return Err(Error::Missing("ICity effect context"));
        };
        effect
            .apply(self.state, city, &[], false, self.effects)
            .map_err(Error::Effect)
    }
}
impl<E: crate::city_production_turn::modifier_effects::Effects>
    c6_core::unit_moved_signal::collection_addition::Context for Apply<'_, E>
{
    type DestructionSubscription = Subscription;
    fn instance_id(&self) -> Result<i32, Error> {
        Ok(self.record()?.instance)
    }
    // Both concrete effect classes inherit the single-RET BaseEffect hooks.
    fn before_update(&mut self, _: i32) -> Result<(), Error> {
        Ok(())
    }
    fn after_update(&mut self, _: i32) -> Result<(), Error> {
        Ok(())
    }
    fn subscribe_destroyed(&mut self) -> Result<Subscription, Error> {
        let (owner, instance) = (self.record()?.owner, self.record()?.instance);
        self.creation_mut()?
            .object_mut(owner)?
            .destroyed
            .subscribe(DestroyedListener::Modifier(instance))
            .map_err(Error::Ring)
    }
    fn tracked_exists(&self) -> Result<bool, Error> {
        Ok(self.record()?.tracked.is_some())
    }
    fn insert_tracked(
        &mut self,
        tracked: c6_core::unit_moved_signal::modifiers::TrackedSubject,
        token: Subscription,
    ) -> Result<(), Error> {
        let record = self.record_mut()?;
        record.tracked = Some(tracked);
        record.destruction = Some(token);
        Ok(())
    }
    fn unsubscribe_temporary(&mut self, token: Subscription) -> Result<(), Error> {
        let owner = self.record()?.owner;
        self.creation_mut()?
            .object_mut(owner)?
            .destroyed
            .unsubscribe(token)
            .map_err(Error::Ring)
    }
    fn has_subject_requirement_definition(&self) -> Result<bool, Error> {
        Ok(self.definition.subject_requirements.is_some())
    }
    fn create_subject_requirement_set(&mut self) -> Result<Option<u64>, Error> {
        let record = self.record()?;
        let (owner, receiver) = (record.owner, record.identity);
        let context = self
            .runtime
            .object(owner)
            .map_err(Error::Query)?
            .instance_id;
        c6_core::melee_attack::proposals::subject_sets::get_or_create_subject(
            &mut RequirementInitializer {
                state: self.state,
                effects: self.runtime,
                requirements: self.requirements,
                maximum_loyalty: self.maximum_loyalty,
            },
            receiver,
            context,
            c6_core::melee_attack::proposals::subject_sets::Subject::ModifierObject(owner),
            self.definition
                .subject_requirements
                .as_deref()
                .ok_or(Error::Missing("subject definition"))?,
        )
        .map_err(|e| Error::Requirement(Box::new(e)))
    }
    fn remove_tracked_subject(&mut self) -> Result<(), Error> {
        Err(Error::Missing("City tracked removal continuation"))
    }
}

impl EffectRuntime {
    /// Supported positive DynamicModifier factory: COLLECTION_OWNER over an
    /// already cached City wrapper, with the two concrete City effect classes.
    /// The modifier is inserted before Activate; callback failure retains its
    /// real mutation prefix and never counts as successful attachment.
    #[allow(clippy::too_many_arguments)] // Native factory plus same-state owners.
    pub fn attach_city_owner(
        &mut self,
        state: &mut LoadedState,
        requirements: &mut super::requirements::Runtime,
        owner: u64,
        definition: &definitions::Attachment,
        maximum_loyalty: Option<i32>,
        effects: &mut impl crate::city_production_turn::modifier_effects::Effects,
    ) -> Result<i32, Error> {
        self.current_owner(state).map_err(Error::Query)?;
        let super::ContextObject::City(_) = self.object(owner).map_err(Error::Query)?.kind else {
            return Err(Error::Missing("City attachment owner"));
        };
        if definition.owner_requirements.is_some() {
            return Err(Error::Missing("City modifier owner requirement factory"));
        }
        if definition.owner_stack_limit != 0 {
            return Err(Error::Missing(
                "current modifier owned-by definition counts",
            ));
        }
        if definition.run_once || definition.permanent || definition.new_only {
            return Err(Error::Missing(
                "City modifier one-shot/permanent/new-only activation",
            ));
        }
        let maximum = if definition.subject_requirements.is_some() {
            maximum_loyalty.ok_or(Error::Missing("current maximum loyalty parameter"))?
        } else {
            0
        };
        let creation = self
            .city_creation
            .as_mut()
            .ok_or(Error::Missing("current City creation domain"))?;
        creation.object_mut(owner)?;
        let instance = creation.assign_instance()?;
        // Logical effect handles occupy their own namespace. Native heap
        // addresses/allocator state are not fabricated or gameplay counters.
        let effect_handle = (1u64 << 62) | u64::from(instance as u32);
        self.register_city_effect(state, effect_handle, owner, definition.effect.clone())
            .map_err(Error::Query)?;
        let creation = self.city_creation.as_mut().unwrap();
        let index = creation.modifiers.len();
        // DynamicModifier's constructor subscribes owner destruction before
        // Activate creates the distinct TrackedSubject destruction handle.
        let owner_destruction = creation
            .object_mut(owner)?
            .destroyed
            .subscribe(DestroyedListener::ModifierOwner(instance))
            .map_err(Error::Ring)?;
        creation.modifiers.push(Dynamic {
            identity: (1u64 << 61) | u64::from(instance as u32),
            instance,
            owner,
            effect: effect_handle,
            definition: definition.id.clone(),
            modifier: c6_core::unit_moved_signal::modifiers::Modifier {
                enabled: true,
                suppressed: false,
                owner_requirement_set: None,
            },
            tracked: None,
            owner_destruction,
            destruction: None,
            applied_subjects: Vec::new(),
            activated: false,
        });
        // Factory hooks for these classes are BaseEffect single-RET methods.
        // Owner.GetItems yields exactly the same registered owner object.
        c6_core::unit_moved_signal::collection_addition::add(&mut Apply {
            state,
            runtime: self,
            requirements,
            definition,
            maximum_loyalty: maximum,
            index,
            effects,
        })?;
        self.city_creation.as_mut().unwrap().modifiers[index].activated = true;
        Ok(instance)
    }
    pub fn city_modifier_owner_subscription(&self, instance: i32) -> Result<Subscription, Error> {
        self.city_creation
            .as_ref()
            .ok_or(Error::Missing("current City creation domain"))?
            .modifiers
            .iter()
            .find(|v| v.instance == instance)
            .map(|v| v.owner_destruction)
            .ok_or(Error::Missing("created City modifier"))
    }
    pub fn next_city_modifier_id(&self) -> Result<i32, Error> {
        Ok(self
            .city_creation
            .as_ref()
            .ok_or(Error::Missing("current City creation domain"))?
            .next_modifier_id)
    }
}
