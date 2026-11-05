//! Positive City modifier creation in the existing GameEffects owner.
//!
//! Native pointer allocation is not logical identity. Modifier instances use the
//! actual ModifierSystem counter; cached City objects keep their source IDs.
pub mod collection;
pub mod definitions;
pub mod policy;
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
    RequirementState(Box<super::requirements::Error>),
    Requirement(Box<c6_core::melee_attack::proposals::subject_sets::Error<Error>>),
    Effect(super::CityEffectError),
}
struct Object {
    handle: u64,
    affected: AffectedModifiers,
    destroyed: ListenerRing<DestroyedListener>,
}
/// This state is a child of EffectRuntime, not another modifier registry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlayerCitiesSubscription {
    /// Logical collection callback routed through the same DynamicModifier.
    pub receiver: u64,
    pub owner: i32,
}
pub(super) struct Creation {
    player_cities_subscriptions: Vec<PlayerCitiesSubscription>,
    player_capital_subscriptions: Vec<PlayerCitiesSubscription>,
    city_removed_subscriptions: Vec<policy::CitySubscription>,
    modifiers: Vec<Dynamic>,
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
        let loyalty = ListenerRing::from_ordered(&initial.loyalty).map_err(Error::Ring)?;
        self.initialize_modifier_system(
            state,
            initial.next_modifier_id,
            &initial.existing_modifier_ids,
        )
        .map_err(Error::Query)?;
        self.city_creation = Some(Creation {
            player_cities_subscriptions: Vec::new(),
            player_capital_subscriptions: Vec::new(),
            city_removed_subscriptions: Vec::new(),
            modifiers: Vec::new(),
            objects,
            loyalty,
            loyalty_subscriptions: Vec::new(),
        });
        Ok(())
    }
}

struct RequirementInitializer<'a> {
    state: &'a LoadedState,
    effects: &'a mut EffectRuntime,
    requirements: &'a mut super::requirements::Runtime,
    maximum_loyalty: Option<i32>,
    factory: &'a mut dyn collection::RequirementFactory,
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
        context: i32,
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
            return self.factory.initialize(
                self.state,
                self.effects,
                self.requirements,
                label,
                definition,
                context,
                subject,
            );
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
            self.maximum_loyalty,
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
    definition: definitions::Attachment,
    modifier: c6_core::unit_moved_signal::modifiers::Modifier,
    tracked: Vec<collection::Subject>,
    owner_destruction: Option<Subscription>,
    applied_subjects: Vec<u64>,
    activated: bool,
    hooks: collection::Hooks,
}
impl Creation {
    fn object_mut(&mut self, handle: u64) -> Result<&mut Object, Error> {
        self.objects
            .iter_mut()
            .find(|v| v.handle == handle)
            .ok_or(Error::Missing("current City modifier-object lifecycle"))
    }
}

struct Apply<'a, E> {
    state: &'a mut LoadedState,
    runtime: &'a mut EffectRuntime,
    requirements: &'a mut super::requirements::Runtime,
    subject: u64,
    factory: &'a mut dyn collection::RequirementFactory,
    features: &'a [u32],
    maximum_loyalty: Option<i32>,
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
            .iter()
            .find(|r| r.subject == self.subject)
            .map(|r| &r.state)
            .ok_or(Error::Missing("tracked City"))
    }
    fn tracked_mut(
        &mut self,
    ) -> Result<&mut c6_core::unit_moved_signal::modifiers::TrackedSubject, Error> {
        let subject = self.subject;
        self.record_mut()?
            .tracked
            .iter_mut()
            .find(|r| r.subject == subject)
            .map(|r| &mut r.state)
            .ok_or(Error::Missing("tracked City"))
    }
    fn requirement_set_state(&self, id: u64) -> Result<i32, Error> {
        super::Requirements::state(self.requirements, self.state, id)
            .map_err(|e| Error::RequirementState(Box::new(e)))
    }
    fn ensure_applied_subject(&mut self) -> Result<(), Error> {
        let subject = self.subject;
        let key = collection::city_key(self.runtime, subject)?;
        let at = self
            .record()?
            .applied_subjects
            .iter()
            .map(|r| collection::city_key(self.runtime, *r))
            .collect::<Result<Vec<_>, _>>()?
            .partition_point(|r| *r < key);
        if self.record()?.applied_subjects.get(at).copied() != Some(subject) {
            self.record_mut()?.applied_subjects.insert(at, subject);
        }
        Ok(())
    }
    fn is_repeatable(&self) -> Result<bool, Error> {
        Ok(self.record()?.definition.repeatable)
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
        let set = c6_core::unit_moved_signal::modifiers::Context::tracked(self)?
            .requirement_set
            .ok_or(Error::Missing("subject requirement subscription"))?;
        let receiver = self.record()?.identity;
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
        // For the explicitly qualified stock metadata scope, the native pure
        // IsPermanent getter precedes the zero-applied guard without mutation.
        if self.record()?.hooks == collection::Hooks::BaseNoops
            && c6_core::unit_moved_signal::modifiers::Context::tracked(self)?.applied == 0
        {
            return Ok(());
        }
        if self.record()?.modifier.suppressed || self.record()?.definition.permanent {
            return Ok(());
        }
        if c6_core::unit_moved_signal::modifiers::Context::tracked(self)?.applied == 0 {
            return Ok(());
        }
        c6_core::unit_moved_signal::modifiers::Context::tracked_mut(self)?.applied = 0;
        let subject = self.subject;
        self.record_mut()?
            .applied_subjects
            .retain(|r| *r != subject);
        let (instance, id, limit, effect) = {
            let m = self.record()?;
            (
                m.instance,
                m.definition.id.clone(),
                m.definition.subject_stack_limit,
                m.effect,
            )
        };
        let affected = &mut self.creation_mut()?.object_mut(subject)?.affected;
        affected.clear_instance(instance);
        if limit != 0 && affected.count(&id) >= i32::from(limit) {
            return Ok(());
        }
        let (_, effect) = self.runtime.city_effect(effect).map_err(Error::Query)?;
        let super::ContextObject::City(city) =
            self.runtime.object(subject).map_err(Error::Query)?.kind
        else {
            return Err(Error::Missing("ICity removal subject"));
        };
        effect
            .apply(self.state, city, self.features, true, self.effects)
            .map_err(Error::Effect)?;
        Ok(())
    }
}
impl<E: crate::city_production_turn::modifier_effects::Effects>
    c6_core::unit_moved_signal::modifiers::EffectContext for Apply<'_, E>
{
    type Error = Error;
    fn mark_subject_affected(&mut self) -> Result<(), Error> {
        let record = self.record()?;
        let (owner, id, definition) = (self.subject, record.instance, record.definition.id.clone());
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
        Ok(Some(self.record()?.definition.subject_stack_limit))
    }
    fn affected_count(&self) -> Result<i32, Error> {
        let record = self.record()?;
        Ok(self
            .creation()?
            .objects
            .iter()
            .find(|v| v.handle == self.subject)
            .ok_or(Error::Missing("current affected City"))?
            .affected
            .count(&record.definition.id))
    }
    fn log_applying(&mut self) -> Result<(), Error> {
        Ok(())
    }
    fn log_failed(&mut self) -> Result<(), Error> {
        Ok(())
    }
    fn apply_concrete_effect(&mut self) -> Result<bool, Error> {
        let record = self.record()?;
        let (_, effect) = self
            .runtime
            .city_effect(record.effect)
            .map_err(Error::Query)?;
        let super::ContextObject::City(city) = self
            .runtime
            .object(self.subject)
            .map_err(Error::Query)?
            .kind
        else {
            return Err(Error::Missing("ICity effect context"));
        };
        effect
            .apply(self.state, city, self.features, false, self.effects)
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
    fn before_update(&mut self, _: i32) -> Result<(), Error> {
        match self.record()?.hooks {
            collection::Hooks::BaseNoops => Ok(()),
            collection::Hooks::Unsupported => Err(Error::Missing("City effect BeforeUpdate hook")),
        }
    }
    fn after_update(&mut self, _: i32) -> Result<(), Error> {
        match self.record()?.hooks {
            collection::Hooks::BaseNoops => Ok(()),
            collection::Hooks::Unsupported => Err(Error::Missing("City effect AfterUpdate hook")),
        }
    }
    fn subscribe_destroyed(&mut self) -> Result<Subscription, Error> {
        let (owner, instance) = (self.subject, self.record()?.instance);
        self.creation_mut()?
            .object_mut(owner)?
            .destroyed
            .subscribe(DestroyedListener::Modifier(instance))
            .map_err(Error::Ring)
    }
    fn tracked_exists(&self) -> Result<bool, Error> {
        Ok(self
            .record()?
            .tracked
            .iter()
            .any(|r| r.subject == self.subject))
    }
    fn insert_tracked(
        &mut self,
        tracked: c6_core::unit_moved_signal::modifiers::TrackedSubject,
        token: Subscription,
    ) -> Result<(), Error> {
        let subject = self.subject;
        let key = collection::city_key(self.runtime, subject)?;
        let at = self
            .record()?
            .tracked
            .iter()
            .map(|r| collection::city_key(self.runtime, r.subject))
            .collect::<Result<Vec<_>, _>>()?
            .partition_point(|r| *r < key);
        self.record_mut()?.tracked.insert(
            at,
            collection::Subject {
                subject,
                state: tracked,
                destruction: token,
            },
        );
        Ok(())
    }
    fn unsubscribe_temporary(&mut self, token: Subscription) -> Result<(), Error> {
        let owner = self.subject;
        self.creation_mut()?
            .object_mut(owner)?
            .destroyed
            .unsubscribe(token)
            .map_err(Error::Ring)
    }
    fn has_subject_requirement_definition(&self) -> Result<bool, Error> {
        Ok(self.record()?.definition.subject_requirements.is_some())
    }
    fn create_subject_requirement_set(&mut self) -> Result<Option<u64>, Error> {
        let record = self.record()?;
        let (owner, receiver) = (record.owner, record.identity);
        let subject = self.subject;
        let definition = record
            .definition
            .subject_requirements
            .clone()
            .ok_or(Error::Missing("subject definition"))?;
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
                factory: self.factory,
            },
            receiver,
            context,
            c6_core::melee_attack::proposals::subject_sets::Subject::ModifierObject(subject),
            &definition,
        )
        .map_err(|e| Error::Requirement(Box::new(e)))
    }
    fn remove_tracked_subject(&mut self) -> Result<(), Error> {
        let subject = self.subject;
        let Some(at) = self
            .record()?
            .tracked
            .iter()
            .position(|r| r.subject == subject)
        else {
            return Ok(());
        };
        if self.record()?.tracked[at].state.requirement_set.is_some() {
            return Err(Error::Missing(
                "City subject requirement release on removal",
            ));
        }
        c6_core::unit_moved_signal::modifiers::Context::remove_effects(self)?;
        let token = self.record()?.tracked[at].destruction;
        self.creation_mut()?
            .object_mut(subject)?
            .destroyed
            .unsubscribe(token)
            .map_err(Error::Ring)?;
        self.record_mut()?.tracked.remove(at);
        self.record_mut()?
            .applied_subjects
            .retain(|r| *r != subject);
        Ok(())
    }
}

impl EffectRuntime {
    /// Native ordinary MakeObject(City*): reuse the cached normal wrapper or
    /// initialize one for a City created after this owner's initial snapshot.
    /// The same global object store/counter is used by Unit factories.
    pub fn make_city_object(
        &mut self,
        state: &LoadedState,
        city: c6_core::unit::ComponentId,
    ) -> Result<u64, Error> {
        self.current_owner(state).map_err(Error::Query)?;
        if city.component_type != 2 {
            return Err(Error::Missing("City wrapper component type"));
        }
        state
            .city(i32::from(city.owner), city.id)
            .map_err(|_| Error::Missing("current City wrapper target"))?;
        if let Some(row) = self
            .objects
            .iter()
            .find(|r| r.1.kind == super::ContextObject::City(city))
        {
            return Ok(row.0);
        }
        if self
            .allocations
            .has_initial_city(city)
            .map_err(Error::Query)?
        {
            return Err(Error::Missing("current BEFORE City wrapper cache"));
        }
        let creation = self
            .city_creation
            .as_mut()
            .ok_or(Error::Missing("current City creation domain"))?;
        creation
            .objects
            .try_reserve(1)
            .map_err(|_| Error::Missing("City object lifecycle allocation"))?;
        let affected =
            AffectedModifiers::from_sorted(Vec::new()).expect("empty constructor affected list");
        let destroyed = ListenerRing::from_ordered(&[]).map_err(Error::Ring)?;
        let handle = self
            .append_normal_object(
                state,
                super::Object {
                    kind: super::ContextObject::City(city),
                    instance_id: 0,
                    unit_sort_suffix: None,
                    unit_lifecycle: None,
                },
            )
            .map_err(Error::Query)?;
        self.city_creation.as_mut().unwrap().objects.push(Object {
            handle,
            affected,
            destroyed,
        });
        Ok(handle)
    }
    /// Building DynamicModifier factory over a current City context. Owner
    /// selects that object; PlayerCities239eba0 snapshots the owner's ordered
    /// live cities and initializes its future change subscriptions239ece0.
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
        let player_owner = match self.object(owner).map_err(Error::Query)?.kind {
            super::ContextObject::City(context) => {
                state
                    .city(i32::from(context.owner), context.id)
                    .map_err(|_| Error::Missing("current City collection owner"))?
                    .header
                    .owner
            }
            super::ContextObject::Policy { owner, .. } => owner,
            _ => return Err(Error::Missing("City effect attachment owner interface")),
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
        // None is the compiled current parameter's actual absence. Native
        // CityHasFullLoyalty.Test then returns failure, rather than a fake max.
        let maximum = maximum_loyalty;
        let creation = self
            .city_creation
            .as_mut()
            .ok_or(Error::Missing("current City creation domain"))?;
        creation.object_mut(owner)?;
        let mut subjects = match definition.collection {
            definitions::Collection::Owner => vec![owner],
            definitions::Collection::PlayerCities | definitions::Collection::PlayerCapitalCity => {
                let player = state
                    .retained()
                    .players
                    .iter()
                    .find(|p| p.header.owner == player_owner)
                    .ok_or(Error::Missing("current PlayerCities owner"))?;
                let ids = player
                    .cities
                    .records
                    .entries
                    .iter()
                    .filter(|r| {
                        !r.value.header.delayed_death
                            && (definition.collection != definitions::Collection::PlayerCapitalCity
                                || r.value.header.capital)
                    })
                    .map(|r| c6_core::unit::ComponentId {
                        owner: r.value.header.owner as i16,
                        component_type: 2,
                        id: r.value.header.id,
                    })
                    .collect::<Vec<_>>();
                let mut subjects = Vec::with_capacity(ids.len());
                for id in ids {
                    subjects.push(self.make_city_object(state, id)?);
                }
                subjects
            }
            definitions::Collection::AllCities | definitions::Collection::Unsupported(_) => {
                return Err(Error::Missing("new City collection GetItems factory"));
            }
        };
        // TrackSubjectsFromCollection2333f20 sorts GetItems before updates.
        let mut keyed = subjects
            .into_iter()
            .map(|h| collection::city_key(self, h).map(|k| (k, h)))
            .collect::<Result<Vec<_>, _>>()?;
        keyed.sort_by_key(|r| r.0);
        subjects = keyed.into_iter().map(|r| r.1).collect();
        let instance = self.allocate_modifier_id(state).map_err(Error::Query)?;
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
            definition: definition.clone(),
            modifier: c6_core::unit_moved_signal::modifiers::Modifier {
                enabled: true,
                suppressed: false,
                owner_requirement_set: None,
            },
            tracked: Vec::new(),
            owner_destruction: Some(owner_destruction),
            applied_subjects: Vec::new(),
            activated: false,
            hooks: collection::Hooks::BaseNoops,
        });
        if definition.collection == definitions::Collection::PlayerCities {
            self.city_creation
                .as_mut()
                .unwrap()
                .player_cities_subscriptions
                .push(PlayerCitiesSubscription {
                    receiver: (1u64 << 61) | u64::from(instance as u32),
                    owner: player_owner,
                });
        }
        if definition.collection == definitions::Collection::PlayerCapitalCity {
            self.city_creation
                .as_mut()
                .unwrap()
                .player_capital_subscriptions
                .push(PlayerCitiesSubscription {
                    receiver: (1u64 << 61) | u64::from(instance as u32),
                    owner: player_owner,
                });
        }
        if matches!(
            definition.collection,
            definitions::Collection::PlayerCities | definitions::Collection::PlayerCapitalCity
        ) {
            self.city_creation
                .as_mut()
                .unwrap()
                .city_removed_subscriptions
                .push(policy::CitySubscription {
                    receiver: (1u64 << 61) | u64::from(instance as u32),
                    owner: player_owner,
                    capital: definition.collection == definitions::Collection::PlayerCapitalCity,
                });
        }
        // Factory hooks for these classes are BaseEffect single-RET methods.
        // GetItems has already produced the ordered snapshot before Activate.
        let mut factory = collection::MissingRequirements;
        for subject in subjects {
            c6_core::unit_moved_signal::collection_addition::add(&mut Apply {
                state,
                runtime: self,
                requirements,
                subject,
                factory: &mut factory,
                features: &[],
                maximum_loyalty: maximum,
                index,
                effects,
            })?;
        }
        self.city_creation.as_mut().unwrap().modifiers[index].activated = true;
        Ok(instance)
    }
    /// Source subscription order for GameEffects CityAdded/CityRemoved signals
    /// (+288/+2e8). The caller appends these typed callbacks to its SAME signal
    /// owner and dispatches collection::item_changed with this receiver.
    pub fn player_cities_subscriptions(&self) -> Result<&[PlayerCitiesSubscription], Error> {
        Ok(&self
            .city_creation
            .as_ref()
            .ok_or(Error::Missing("current City creation domain"))?
            .player_cities_subscriptions)
    }
    pub fn city_modifier_owner_subscription(&self, instance: i32) -> Result<Subscription, Error> {
        self.city_creation
            .as_ref()
            .ok_or(Error::Missing("current City creation domain"))?
            .modifiers
            .iter()
            .find(|v| v.instance == instance)
            .and_then(|v| v.owner_destruction)
            .ok_or(Error::Missing("created City modifier"))
    }
    /// PlayerCapitalCity listens to CapitalCityChanged+408 and CityRemoved+2e8.
    pub fn player_capital_subscriptions(&self) -> Result<&[PlayerCitiesSubscription], Error> {
        Ok(&self
            .city_creation
            .as_ref()
            .ok_or(Error::Missing("current City creation domain"))?
            .player_capital_subscriptions)
    }
    /// Policy MakeObject uses Objects::Player subtype3 and the shared pool.
    /// Share compares owner/subtype/subvalue while a strong owner remains.
    pub fn make_policy_object(
        &mut self,
        state: &LoadedState,
        owner: i32,
        policy: u32,
    ) -> Result<u64, Error> {
        self.current_owner(state).map_err(Error::Query)?;
        let kind = super::ContextObject::Policy { owner, policy };
        if let Some((handle, _)) = self.objects.iter().find(|(_, o)| o.kind == kind) {
            return Ok(*handle);
        }
        if self.city_creation.is_none() {
            return Err(Error::Missing("current modifier lifecycle domain"));
        }
        let handle = self
            .append_normal_object(
                state,
                super::Object {
                    kind,
                    instance_id: 0,
                    unit_sort_suffix: None,
                    unit_lifecycle: Some(c6_core::unit_kill::objects::Object {
                        pool_ordinal: 0,
                        owned_modifiers: vec![],
                        affected: AffectedModifiers::from_sorted(vec![]).unwrap(),
                        destroyed: ListenerRing::from_ordered(&[]).map_err(Error::Ring)?,
                    }),
                },
            )
            .map_err(Error::Query)?;
        self.city_creation.as_mut().unwrap().objects.push(Object {
            handle,
            affected: AffectedModifiers::from_sorted(vec![]).unwrap(),
            destroyed: ListenerRing::from_ordered(&[]).map_err(Error::Ring)?,
        });
        Ok(handle)
    }
    /// Drop the Policy event's final strong reference after modifier teardown.
    /// ObjectPool.Share/Append23353a0/2335530 retain weak references only;
    /// unlike ordinary City/Unit wrappers, Policies have no gameplay cache.
    pub fn retire_policy_object(&mut self, state: &LoadedState, owner: u64) -> Result<(), Error> {
        self.current_owner(state).map_err(Error::Query)?;
        let object = self.object(owner).map_err(Error::Query)?;
        if !matches!(object.kind, super::ContextObject::Policy { .. }) {
            return Err(Error::Missing("Policy object retirement"));
        }
        if object.unit_lifecycle.as_ref().is_some_and(|r| {
            !matches!(
                r.destroyed.subscription_for(|_| true),
                Err(RegistryError::InvalidSubscription)
            )
        }) {
            return Err(Error::Missing("Policy remaining ObjectDestroyed listener"));
        }
        if object
            .unit_lifecycle
            .as_ref()
            .is_some_and(|r| !r.owned_modifiers.is_empty() || !r.affected.entries().is_empty())
        {
            return Err(Error::Missing("Policy live owned/affected modifiers"));
        }
        if self.unit_modifiers.as_ref().is_some_and(|v| {
            v.iter().any(|m| {
                m.context == owner
                    || m.applied.contains(&owner)
                    || m.tracked.iter().any(|t| t.object == owner)
            })
        }) {
            return Err(Error::Missing("Policy surviving Unit modifier references"));
        }
        let creation = self
            .city_creation
            .as_mut()
            .ok_or(Error::Missing("current City creation domain"))?;
        if creation.modifiers.iter().any(|m| {
            m.owner == owner
                || m.applied_subjects.contains(&owner)
                || m.tracked.iter().any(|t| t.subject == owner)
        }) {
            return Err(Error::Missing("Policy surviving City modifier references"));
        }
        let life = creation.object_mut(owner)?;
        if !matches!(
            life.destroyed.subscription_for(|_| true),
            Err(RegistryError::InvalidSubscription)
        ) {
            return Err(Error::Missing(
                "Policy remaining City ObjectDestroyed listener",
            ));
        }
        creation.objects.retain(|o| o.handle != owner);
        self.objects.retain(|r| r.0 != owner);
        Ok(())
    }
    /// Clear the actual created City modifiers owned by this Policy wrapper.
    /// Source destruction removes registered modifiers in owned-ID order and
    /// runs each leaf through Remove; unsupported requirement release is explicit.
    pub fn remove_policy_city_modifiers(
        &mut self,
        state: &mut LoadedState,
        _requirements: &mut super::requirements::Runtime,
        owner: u64,
        effects: &mut impl crate::city_production_turn::modifier_effects::Effects,
    ) -> Result<(), Error> {
        self.current_owner(state).map_err(Error::Query)?;
        loop {
            let Some(index) = self
                .city_creation
                .as_ref()
                .ok_or(Error::Missing("current City creation domain"))?
                .modifiers
                .iter()
                .position(|m| m.owner == owner)
            else {
                return Ok(());
            };
            // Deactivate23332b0 clears eligible tracked entries first. These
            // admitted policy definitions have no permanent/removed subjects.
            if self.city_creation.as_ref().unwrap().modifiers[index]
                .tracked
                .iter()
                .any(|r| r.state.requirement_set.is_some() || r.state.removed)
            {
                return Err(Error::Missing(
                    "Policy City Deactivate requirement/removed branch",
                ));
            }
            self.city_creation.as_mut().unwrap().modifiers[index]
                .modifier
                .enabled = false;
            let tracked =
                std::mem::take(&mut self.city_creation.as_mut().unwrap().modifiers[index].tracked);
            for row in tracked {
                self.city_creation
                    .as_mut()
                    .unwrap()
                    .object_mut(row.subject)?
                    .destroyed
                    .unsubscribe(row.destruction)
                    .map_err(Error::Ring)?;
            }
            self.city_creation.as_mut().unwrap().modifiers[index]
                .modifier
                .enabled = false;
            let applied = std::mem::take(
                &mut self.city_creation.as_mut().unwrap().modifiers[index].applied_subjects,
            );
            for subject in applied {
                let (instance, id, limit, effect) = {
                    let m = &self.city_creation.as_ref().unwrap().modifiers[index];
                    (
                        m.instance,
                        m.definition.id.clone(),
                        m.definition.subject_stack_limit,
                        m.effect,
                    )
                };
                let affected = &mut self
                    .city_creation
                    .as_mut()
                    .unwrap()
                    .object_mut(subject)?
                    .affected;
                affected.clear_instance(instance);
                if limit != 0 && affected.count(&id) >= i32::from(limit) {
                    continue;
                }
                let (_, effect) = self.city_effect(effect).map_err(Error::Query)?;
                let super::ContextObject::City(city) =
                    self.object(subject).map_err(Error::Query)?.kind
                else {
                    return Err(Error::Missing("Policy Remove ICity"));
                };
                effect
                    .apply(state, city, &[], true, effects)
                    .map_err(Error::Effect)?;
            }
            let creation = self.city_creation.as_mut().unwrap();
            let m = creation.modifiers.remove(index);
            if let Some(token) = m.owner_destruction {
                creation
                    .object_mut(owner)?
                    .destroyed
                    .unsubscribe(token)
                    .map_err(Error::Ring)?;
            }
            creation
                .player_cities_subscriptions
                .retain(|r| r.receiver != m.identity);
            creation
                .player_capital_subscriptions
                .retain(|r| r.receiver != m.identity);
            creation
                .city_removed_subscriptions
                .retain(|r| r.receiver != m.identity);
            self.retire_effect(state, m.effect).map_err(Error::Query)?;
        }
    }
    pub fn next_city_modifier_id(&self) -> Result<i32, Error> {
        self.next_modifier_instance_id().map_err(Error::Query)
    }
}
