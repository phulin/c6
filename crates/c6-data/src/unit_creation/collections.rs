//! UnitCreated PlayerUnits/AllUnits notifications over the shared live
//! modifier, object, collection and requirement owners.
use crate::melee_attack::backend::{self, ContextObject, EffectRuntime};
use c6_core::{
    loaded_state::LoadedState,
    map::UnitReference,
    melee_attack::proposals::{collections, subject_sets as sets},
    unit_kill::objects::{self as records, Callback, Tracked},
    unit_moved_signal::{collection_addition as addition, modifiers as common, Subscription},
};
pub mod ability;
pub mod see_hidden;
pub use ability::{on_ability_earned, on_home_moved};
#[derive(Debug)]
pub enum Error {
    Query(backend::QueryError),
    State(c6_core::loaded_state::Error),
    Requirements(backend::requirements::Error),
    Factory(Box<sets::Error<Error>>),
    Ring(c6_core::unit_moved_signal::RegistryError),
    Collection(Box<collections::Error<Error>>),
    Missing(&'static str),
}
#[derive(Clone, Copy, Debug)]
pub enum CreatedCollection {
    PlayerUnits { collection: u64, owner: i32 },
    AllUnits { collection: u64 },
}
/// Immutable definition inputs admitted for the same configuration. Mutable
/// control/tracked/applied fields live only in EffectRuntime.unit_modifier.
pub struct Definition {
    pub receiver: u64,
    pub subject_requirements: Vec<u8>,
    pub repeatable: bool,
}
pub struct Runtime {
    definitions: Vec<Definition>,
    logging_priority: Option<i32>,
    abilities: Vec<ability::Ability>,
    homes: Vec<ability::Home>,
    requirement_flags: Vec<(Vec<u8>, bool)>,
}
impl Runtime {
    pub fn from_initial(
        definitions: Vec<Definition>,
        logging_priority: Option<i32>,
    ) -> Result<Self, Error> {
        if definitions
            .iter()
            .enumerate()
            .any(|(i, d)| definitions[..i].iter().any(|p| p.receiver == d.receiver))
        {
            return Err(Error::Missing("duplicate Unit modifier definition"));
        }
        Ok(Self {
            definitions,
            logging_priority,
            abilities: Vec::new(),
            homes: Vec::new(),
            requirement_flags: Vec::new(),
        })
    }
    fn definition(&self, id: u64) -> Result<&Definition, Error> {
        self.definitions
            .iter()
            .find(|d| d.receiver == id)
            .ok_or(Error::Missing("current Unit modifier definition"))
    }
    fn log_disabled(&self, threshold: i32) -> Result<(), Error> {
        if self
            .logging_priority
            .ok_or(Error::Missing("current GameEffects logging priority"))?
            < threshold
        {
            Ok(())
        } else {
            Err(Error::Missing("enabled Unit modifier logging"))
        }
    }
}
pub struct Owners<'a> {
    pub state: &'a mut LoadedState,
    pub effects: &'a mut EffectRuntime,
    pub requirements: &'a mut backend::requirements::Runtime,
    pub collections: &'a mut collections::Runtime,
}
/// Concrete synchronous lower leaves. All methods must use these same owners;
/// there is no captured Test result or success-default callback.
pub trait Effects {
    /// ChangeAbilityCount queries current eligibility before storing the first
    /// positive count, using the same configured Unit tags and definitions.
    fn can_have_ability(
        &mut self,
        owners: &mut Owners<'_>,
        unit: UnitReference,
        index: usize,
    ) -> Result<bool, Error>;
    /// Append to the SAME current UnitMoved dispatcher. The returned position
    /// must later call on_home_moved, with no early execution during initialize.
    fn subscribe_home_movement(
        &mut self,
        owners: &mut Owners<'_>,
        requirement: u64,
    ) -> Result<(), Error>;
    fn ability_changed(
        &mut self,
        owners: &mut Owners<'_>,
        runtime: &mut Runtime,
        unit: UnitReference,
        index: usize,
        added: bool,
    ) -> Result<(), Error>;
    fn ability_report(
        &mut self,
        owners: &mut Owners<'_>,
        unit: UnitReference,
        index: usize,
        added: bool,
    ) -> Result<(), Error>;
}
fn unit_key(effects: &EffectRuntime, handle: u64) -> Result<common::UnitObjectKey, Error> {
    let object = effects.object(handle).map_err(Error::Query)?;
    let ContextObject::Unit(unit) = object.kind else {
        return Err(Error::Missing("collection subject IUnit"));
    };
    let suffix = object
        .unit_sort_suffix
        .ok_or(Error::Missing("Unit object comparator suffix"))?;
    Ok(common::UnitObjectKey {
        component: unit.component_id(),
        first: suffix[0],
        second: suffix[1],
    })
}
fn tracked(owners: &Owners<'_>, receiver: u64, subject: u64) -> Result<Option<usize>, Error> {
    let key = unit_key(owners.effects, subject)?;
    for (index, row) in owners
        .effects
        .unit_modifier(receiver)
        .map_err(Error::Query)?
        .tracked
        .iter()
        .enumerate()
    {
        if unit_key(owners.effects, row.object)?.compare(key) == std::cmp::Ordering::Equal {
            return Ok(Some(index));
        }
    }
    Ok(None)
}
struct Apply<'a, 's, E> {
    owners: &'a mut Owners<'s>,
    runtime: &'a mut Runtime,
    effects: &'a mut E,
    definitions: &'a super::Definitions,
    receiver: u64,
    subject: u64,
}
impl<E: Effects> Apply<'_, '_, E> {
    fn record(&self) -> Result<&records::Modifier, Error> {
        self.owners
            .effects
            .unit_modifier(self.receiver)
            .map_err(Error::Query)
    }
    fn record_mut(&mut self) -> Result<&mut records::Modifier, Error> {
        self.owners
            .effects
            .unit_modifier_mut(self.receiver)
            .map_err(Error::Query)
    }
    fn index(&self) -> Result<usize, Error> {
        tracked(self.owners, self.receiver, self.subject)?
            .ok_or(Error::Missing("current tracked Unit subject"))
    }
    fn object_mut(&mut self) -> Result<&mut records::Object, Error> {
        self.owners
            .effects
            .object_mut(self.subject)
            .map_err(Error::Query)?
            .unit_lifecycle
            .as_mut()
            .ok_or(Error::Missing("current Unit object lifecycle"))
    }
    fn unit(&self) -> Result<UnitReference, Error> {
        match self
            .owners
            .effects
            .object(self.subject)
            .map_err(Error::Query)?
            .kind
        {
            ContextObject::Unit(unit) => Ok(unit),
            _ => Err(Error::Missing("current effect subject IUnit")),
        }
    }
}
impl<E: Effects> common::Context for Apply<'_, '_, E> {
    type Error = Error;
    fn modifier(&self) -> Result<&common::Modifier, Error> {
        Ok(&self.record()?.control)
    }
    fn tracked(&self) -> Result<&common::TrackedSubject, Error> {
        Ok(&self.record()?.tracked[self.index()?].state)
    }
    fn tracked_mut(&mut self) -> Result<&mut common::TrackedSubject, Error> {
        let i = self.index()?;
        Ok(&mut self.record_mut()?.tracked[i].state)
    }
    fn requirement_set_state(&self, id: u64) -> Result<i32, Error> {
        backend::Requirements::state(self.owners.requirements, self.owners.state, id)
            .map_err(Error::Requirements)
    }
    fn ensure_applied_subject(&mut self) -> Result<(), Error> {
        let key = unit_key(self.owners.effects, self.subject)?;
        let mut i = 0;
        while i < self.record()?.applied.len() {
            match unit_key(self.owners.effects, self.record()?.applied[i])?.compare(key) {
                std::cmp::Ordering::Equal => return Ok(()),
                std::cmp::Ordering::Greater => break,
                _ => i += 1,
            }
        }
        let subject = self.subject;
        self.record_mut()?.applied.insert(i, subject);
        Ok(())
    }
    fn is_repeatable(&self) -> Result<bool, Error> {
        Ok(self.runtime.definition(self.receiver)?.repeatable)
    }
    fn apply_effect(&mut self) -> Result<bool, Error> {
        common::apply_effect(self).map_err(|e| match e {
            common::EffectError::Context(e) => e,
            common::EffectError::DefinitionDisappeared => {
                Error::Missing("Unit modifier definition")
            }
        })?;
        Ok(true)
    }
    fn unsubscribe_subject_requirement(&mut self) -> Result<(), Error> {
        let i = self.index()?;
        let set = self.record()?.tracked[i]
            .requirement_set
            .ok_or(Error::Missing("Unit subject requirement subscription"))?;
        self.owners
            .requirements
            .sets_mut()
            .unsubscribe(self.receiver, set)
            .map_err(|e| Error::Factory(Box::new(e)))
    }
    fn clear_subject_requirement(&mut self) -> Result<(), Error> {
        Err(Error::Missing("Unit subject requirement final release"))
    }
    fn remove_effects(&mut self) -> Result<(), Error> {
        Err(Error::Missing("Unit collection effect removal"))
    }
}
impl<E: Effects> common::EffectContext for Apply<'_, '_, E> {
    type Error = Error;
    fn mark_subject_affected(&mut self) -> Result<(), Error> {
        let (definition, instance) = (self.record()?.definition.clone(), self.record()?.instance);
        self.object_mut()?
            .affected
            .mark(&definition, instance)
            .map_err(|_| Error::Missing("Unit affected modifier order"))
    }
    fn suppressed(&self) -> Result<bool, Error> {
        Ok(self.record()?.suppressed)
    }
    fn subject_stack_limit(&self) -> Result<Option<u16>, Error> {
        Ok(Some(self.record()?.stack_limit))
    }
    fn affected_count(&self) -> Result<i32, Error> {
        let definition = &self.record()?.definition;
        Ok(self
            .owners
            .effects
            .object(self.subject)
            .map_err(Error::Query)?
            .unit_lifecycle
            .as_ref()
            .ok_or(Error::Missing("Unit affected modifier owner"))?
            .affected
            .count(definition))
    }
    fn log_applying(&mut self) -> Result<(), Error> {
        self.runtime.log_disabled(3)
    }
    fn log_failed(&mut self) -> Result<(), Error> {
        self.runtime.log_disabled(1)
    }
    fn apply_concrete_effect(&mut self) -> Result<bool, Error> {
        let effect = *match self
            .owners
            .effects
            .effect(self.record()?.effect)
            .map_err(Error::Query)?
        {
            backend::EffectState::Unit(effect) => effect,
            _ => return Err(Error::Missing("Unit effect class")),
        };
        let reference = self.unit()?;
        let unit = super::actor::<Error>(self.owners.state, reference.player(), reference.id)
            .map_err(|_| Error::Missing("current Unit effect target"))?;
        let actor = (unit.e_owner, unit.id);
        use crate::unit_kill::effects::Effect;
        match effect {
            Effect::SeeHidden => see_hidden::apply(self.owners.state, actor, false),
            Effect::CombatStrength { amount, kind } => {
                crate::unit_kill::effects::adjust_combat_strength(
                    self.owners
                        .state
                        .unit_mut(actor.0, actor.1)
                        .map_err(Error::State)?,
                    kind,
                    amount,
                );
                Ok(true)
            }
            Effect::GrantAbility(value) => {
                use c6_core::movement_world::DefinitionLookup;
                let registry = self.definitions.initialization.abilities();
                let Some(index) = registry.find_index(value) else {
                    return Ok(true);
                };
                let Some(previous) = self
                    .owners
                    .state
                    .unit_runtime(actor.0, actor.1)
                    .map_err(Error::State)?
                    .abilities
                    .get(index)
                    .copied()
                else {
                    return Ok(true);
                };
                let hash = registry
                    .type_hash(index)
                    .ok_or(Error::Missing("GrantAbility type hash"))?;
                let next = previous.wrapping_add(1).max(0);
                if previous == 0
                    && next > 0
                    && !self
                        .effects
                        .can_have_ability(self.owners, reference, index)?
                {
                    return Ok(true);
                }
                self.owners
                    .state
                    .set_unit_ability_count(actor.0, actor.1, index, hash, next)
                    .map_err(Error::State)?;
                if (previous > 0) != (next > 0) {
                    self.effects.ability_changed(
                        self.owners,
                        self.runtime,
                        reference,
                        index,
                        next > 0,
                    )?;
                    self.effects
                        .ability_report(self.owners, reference, index, next > 0)?;
                }
                Ok(true)
            }
            Effect::DiploVisibilityCombat { amount, higher } => {
                let unit = self
                    .owners
                    .state
                    .unit_mut(actor.0, actor.1)
                    .map_err(Error::State)?;
                let field = if higher {
                    &mut unit.k_combat_modifers.i_higher_diplo_viz_combat_mod
                } else {
                    &mut unit.k_combat_modifers.i_each_diplo_viz_combat_mod
                };
                *field = field.wrapping_add(amount).clamp(0, 100);
                Ok(true)
            }
            Effect::Common(effect) => {
                crate::unit_modifier_effect::apply(self.owners.state, Some(reference), effect)
                    .map_err(|_| Error::Missing("Unit common effect"))
            }
            _ => Err(Error::Missing("current Unit effect Apply class")),
        }
    }
}
impl<E: Effects> addition::Context for Apply<'_, '_, E> {
    type DestructionSubscription = Subscription;
    fn instance_id(&self) -> Result<i32, Error> {
        Ok(self.record()?.instance)
    }
    fn before_update(&mut self, _: i32) -> Result<(), Error> {
        match self
            .owners
            .effects
            .effect(self.record()?.effect)
            .map_err(Error::Query)?
        {
            backend::EffectState::Unit(crate::unit_kill::effects::Effect::Unsupported(_)) => {
                Err(Error::Missing("Unit effect BeforeUpdate"))
            }
            backend::EffectState::Unit(_) => Ok(()),
            _ => Err(Error::Missing("Unit effect lifecycle class")),
        }
    }
    fn after_update(&mut self, instance: i32) -> Result<(), Error> {
        self.before_update(instance)
    }
    fn subscribe_destroyed(&mut self) -> Result<Subscription, Error> {
        let receiver = self.receiver;
        self.object_mut()?
            .destroyed
            .subscribe(Callback::TrackedDestroyed(receiver))
            .map_err(Error::Ring)
    }
    fn tracked_exists(&self) -> Result<bool, Error> {
        Ok(tracked(self.owners, self.receiver, self.subject)?.is_some())
    }
    fn insert_tracked(
        &mut self,
        state: common::TrackedSubject,
        token: Subscription,
    ) -> Result<(), Error> {
        let key = unit_key(self.owners.effects, self.subject)?;
        let mut index = 0;
        while index < self.record()?.tracked.len()
            && unit_key(self.owners.effects, self.record()?.tracked[index].object)?.compare(key)
                == std::cmp::Ordering::Less
        {
            index += 1;
        }
        let subject = self.subject;
        self.record_mut()?.tracked.insert(
            index,
            Tracked {
                object: subject,
                state,
                destruction_subscription: Some(token),
            },
        );
        Ok(())
    }
    fn unsubscribe_temporary(&mut self, token: Subscription) -> Result<(), Error> {
        self.object_mut()?
            .destroyed
            .unsubscribe(token)
            .map_err(Error::Ring)
    }
    fn has_subject_requirement_definition(&self) -> Result<bool, Error> {
        Ok(!self
            .runtime
            .definition(self.receiver)?
            .subject_requirements
            .is_empty())
    }
    fn create_subject_requirement_set(&mut self) -> Result<Option<u64>, Error> {
        ability::create(self)
    }
    fn remove_tracked_subject(&mut self) -> Result<(), Error> {
        Err(Error::Missing("Unit collection RemoveTrackedSubject"))
    }
}
struct Fanout<'a, 's, E> {
    owners: &'a mut Owners<'s>,
    runtime: &'a mut Runtime,
    effects: &'a mut E,
    definitions: &'a super::Definitions,
}
impl<E: Effects> collections::Context<u64> for Fanout<'_, '_, E> {
    type Error = Error;
    fn collections(&self) -> Result<&collections::Runtime, Error> {
        Ok(self.owners.collections)
    }
    fn collections_mut(&mut self) -> Result<&mut collections::Runtime, Error> {
        Ok(self.owners.collections)
    }
    fn item_changed(&mut self, receiver: u64, subject: &mut u64, added: bool) -> Result<(), Error> {
        addition::item_changed(
            &mut Apply {
                owners: self.owners,
                runtime: self.runtime,
                effects: self.effects,
                definitions: self.definitions,
                receiver,
                subject: *subject,
            },
            added,
        )
    }
}
/// Native PlayerUnits23fc1c0 filters the captured Player owner; AllUnits always
/// makes the current normal Unit object. All listeners share the one live ring.
pub fn on_unit_created<E: Effects>(
    owners: &mut Owners<'_>,
    runtime: &mut Runtime,
    effects: &mut E,
    definitions: &super::Definitions,
    route: CreatedCollection,
    unit: UnitReference,
) -> Result<(), Error> {
    let actor = super::actor::<Error>(owners.state, unit.player(), unit.id)
        .map_err(|_| Error::Missing("UnitCreated current Unit"))?;
    let collection = match route {
        CreatedCollection::PlayerUnits { collection, owner } => {
            if actor.e_owner != owner {
                return Ok(());
            }
            collection
        }
        CreatedCollection::AllUnits { collection } => collection,
    };
    let object = owners
        .effects
        .make_current_unit_object(owners.state, unit, [0, 0])
        .map_err(Error::Query)?;
    if unit_key(owners.effects, object)?.component != unit.component_id() {
        return Err(Error::Missing("UnitCreated object full identity"));
    }
    collections::add_typed_item(
        &mut Fanout {
            owners,
            runtime,
            effects,
            definitions,
        },
        collection,
        object,
    )
    .map_err(|e| Error::Collection(Box::new(e)))
}
