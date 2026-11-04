//! Live source-qualified TestAll -> DynamicModifier -> unit scalar effect chain.
//! Current listeners/objects/fields are explicit runtime state. Other requirement
//! set kinds and reached removal/trigger continuations remain unsupported.
use crate::unit_modifier_effect::{self as effect, Effect};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    map::UnitReference,
    unit_moved_signal::{modifiers as m, requirement_sets as sets},
};

#[derive(Debug)]
pub enum Error<E> {
    Context(E),
    Missing(&'static str),
    Binding,
    Allocation,
    Effect(effect::Error),
}
#[derive(Debug)]
pub struct Object {
    pub id: u64,
    pub key: m::UnitObjectKey,
    pub affected: m::AffectedModifiers,
}
#[derive(Debug)]
pub struct Set {
    pub id: u64,
    pub state: i32,
    pub children: Vec<u64>,
    pub subject: u64,
    pub modifiers: Vec<u64>,
}
#[derive(Debug)]
pub struct Tracked {
    pub id: u64,
    pub object: u64,
    pub value: m::TrackedSubject,
}
#[derive(Debug)]
pub struct Modifier {
    pub id: u64,
    pub value: m::Modifier,
    pub definition: Vec<u8>,
    pub instance: i32,
    pub repeatable: bool,
    pub stack_limit: u16,
    pub effect: Effect,
    pub tracked: Vec<Tracked>,
    pub tracked_complete: bool,
    pub applied_subjects: Vec<m::UnitObjectKey>,
}
#[derive(Debug)]
pub struct Runtime {
    binding: WireBinding,
    routes: Vec<(u64, Vec<u64>)>,
    sets: Vec<Set>,
    modifiers: Vec<Modifier>,
    objects: Vec<Object>,
    logging_priority: i32,
}
impl Runtime {
    pub fn from_initial(
        binding: WireBinding,
        routes: Vec<(u64, Vec<u64>)>,
        sets: Vec<Set>,
        modifiers: Vec<Modifier>,
        objects: Vec<Object>,
        logging_priority: i32,
    ) -> Result<Self, &'static str> {
        fn unique(ids: impl Iterator<Item = u64>) -> bool {
            let mut seen = std::collections::BTreeSet::new();
            ids.into_iter().all(|id| seen.insert(id))
        }
        if !unique(routes.iter().map(|r| r.0))
            || !unique(sets.iter().map(|r| r.id))
            || !unique(modifiers.iter().map(|r| r.id))
            || !unique(objects.iter().map(|r| r.id))
        {
            return Err("duplicate current runtime identity");
        }
        for m in &modifiers {
            if !unique(m.tracked.iter().map(|r| r.id)) {
                return Err("duplicate tracked identity");
            }
            if m.applied_subjects
                .windows(2)
                .any(|r| r[0].compare(r[1]).is_gt())
            {
                return Err("applied subjects not in native comparator order");
            }
        }
        Ok(Self {
            binding,
            routes,
            sets,
            modifiers,
            objects,
            logging_priority,
        })
    }
    pub fn sets(&self) -> &[Set] {
        &self.sets
    }
    pub fn modifiers(&self) -> &[Modifier] {
        &self.modifiers
    }
    pub fn objects(&self) -> &[Object] {
        &self.objects
    }
}
pub trait Services {
    type Error;
    fn requirement_changes(&self) -> Result<&Runtime, Self::Error>;
    fn requirement_changes_mut(&mut self) -> Result<&mut Runtime, Self::Error>;
    /// Current stored state from the SAME concrete requirement owner.
    fn current_requirement_state(&self, receiver: u64) -> Result<i32, Self::Error>;
    fn current_requirement_persistent(&self, receiver: u64) -> Result<bool, Self::Error>;
}
fn set<E>(r: &Runtime, id: u64) -> Result<&Set, Error<E>> {
    r.sets
        .iter()
        .find(|s| s.id == id)
        .ok_or(Error::Missing("current requirement set"))
}
fn modifier<E>(r: &Runtime, id: u64) -> Result<&Modifier, Error<E>> {
    r.modifiers
        .iter()
        .find(|s| s.id == id)
        .ok_or(Error::Missing("current dynamic modifier"))
}
fn object<E>(r: &Runtime, id: u64) -> Result<&Object, Error<E>> {
    r.objects
        .iter()
        .find(|s| s.id == id)
        .ok_or(Error::Missing("current unit modifier object"))
}
fn tracked<E>(r: &Runtime, id: u64, node: u64) -> Result<&Tracked, Error<E>> {
    modifier(r, id)?
        .tracked
        .iter()
        .find(|s| s.id == node)
        .ok_or(Error::Missing("current tracked subject"))
}

/// Called only AFTER SimpleRequirement has stored its new state. Supported
/// concrete kernels have no registration callbacks; unknown routes never skip.
pub fn changed<S: Services>(
    state: &mut LoadedState,
    services: &mut S,
    receiver: u64,
    _old: i32,
    new: i32,
) -> Result<(), Error<S::Error>> {
    let r = services.requirement_changes().map_err(Error::Context)?;
    if &r.binding != state.binding() {
        return Err(Error::Binding);
    }
    let routes = r
        .routes
        .iter()
        .find(|r| r.0 == receiver)
        .ok_or(Error::Missing("actual requirement changed listeners"))?
        .1
        .clone();
    for id in routes {
        sets::requirement_changed(
            &mut SetContext {
                state,
                services,
                id,
            },
            new,
        )
        .map_err(|e| match e {
            sets::Error::Context(e) => e,
            sets::Error::Allocation => Error::Allocation,
        })?;
    }
    Ok(())
}
struct SetContext<'a, S> {
    state: &'a mut LoadedState,
    services: &'a mut S,
    id: u64,
}
impl<S: Services> sets::Context for SetContext<'_, S> {
    type Error = Error<S::Error>;
    fn children(&self) -> Result<&[u64], Self::Error> {
        Ok(&set(
            self.services
                .requirement_changes()
                .map_err(Error::Context)?,
            self.id,
        )?
        .children)
    }
    fn child_state(&self, id: u64) -> Result<i32, Self::Error> {
        self.services
            .current_requirement_state(id)
            .map_err(Error::Context)
    }
    fn child_persistent(&self, id: u64) -> Result<bool, Self::Error> {
        self.services
            .current_requirement_persistent(id)
            .map_err(Error::Context)
    }
    fn remove_child(&mut self, _id: u64) -> Result<(), Self::Error> {
        Err(Error::Missing(
            "RemoveRequirement unsubscribe/lifetime continuation",
        ))
    }
    fn remove_all_children(&mut self) -> Result<(), Self::Error> {
        Err(Error::Missing(
            "RemoveAllRequirements unsubscribe/lifetime continuation",
        ))
    }
    fn state(&self) -> Result<i32, Self::Error> {
        Ok(set(
            self.services
                .requirement_changes()
                .map_err(Error::Context)?,
            self.id,
        )?
        .state)
    }
    fn store_state(&mut self, state: i32) -> Result<(), Self::Error> {
        self.services
            .requirement_changes_mut()
            .map_err(Error::Context)?
            .sets
            .iter_mut()
            .find(|s| s.id == self.id)
            .ok_or(Error::Missing("current requirement set"))?
            .state = state;
        Ok(())
    }
    fn changed(&mut self, _old: i32, _new: i32) -> Result<(), Self::Error> {
        let r = self
            .services
            .requirement_changes()
            .map_err(Error::Context)?;
        let s = set(r, self.id)?;
        let key = object(r, s.subject)?.key;
        let ids = s.modifiers.clone();
        for id in ids {
            let r = self
                .services
                .requirement_changes()
                .map_err(Error::Context)?;
            // Source comparator equivalence, including both signed suffixes.
            let Some(node) = modifier(r, id)?
                .tracked
                .iter()
                .find_map(|t| match object::<S::Error>(r, t.object) {
                    Ok(o) if o.key.compare(key).is_eq() => Some(Ok(t.id)),
                    Ok(_) => None,
                    Err(e) => Some(Err(e)),
                })
                .transpose()?
            else {
                if !modifier(r, id)?.tracked_complete {
                    return Err(Error::Missing("unqueried tracked subject"));
                }
                continue;
            };
            let _ = m::check_apply_effects(&mut ModifierContext {
                state: self.state,
                services: self.services,
                id,
                node,
            })?;
        }
        Ok(())
    }
    fn reset_trigger(&mut self) -> Result<(), Self::Error> {
        Err(Error::Missing("SimpleRequirementSet.ResetTrigger"))
    }
}
struct ModifierContext<'a, S> {
    state: &'a mut LoadedState,
    services: &'a mut S,
    id: u64,
    node: u64,
}
impl<S: Services> ModifierContext<'_, S> {
    fn object_id(&self) -> Result<u64, Error<S::Error>> {
        Ok(tracked(
            self.services
                .requirement_changes()
                .map_err(Error::Context)?,
            self.id,
            self.node,
        )?
        .object)
    }
}
impl<S: Services> m::Context for ModifierContext<'_, S> {
    type Error = Error<S::Error>;
    fn modifier(&self) -> Result<&m::Modifier, Self::Error> {
        Ok(&modifier(
            self.services
                .requirement_changes()
                .map_err(Error::Context)?,
            self.id,
        )?
        .value)
    }
    fn tracked(&self) -> Result<&m::TrackedSubject, Self::Error> {
        Ok(&tracked(
            self.services
                .requirement_changes()
                .map_err(Error::Context)?,
            self.id,
            self.node,
        )?
        .value)
    }
    fn tracked_mut(&mut self) -> Result<&mut m::TrackedSubject, Self::Error> {
        Ok(&mut self
            .services
            .requirement_changes_mut()
            .map_err(Error::Context)?
            .modifiers
            .iter_mut()
            .find(|m| m.id == self.id)
            .ok_or(Error::Missing("current modifier"))?
            .tracked
            .iter_mut()
            .find(|t| t.id == self.node)
            .ok_or(Error::Missing("current tracked subject"))?
            .value)
    }
    fn requirement_set_state(&self, id: u64) -> Result<i32, Self::Error> {
        Ok(set(
            self.services
                .requirement_changes()
                .map_err(Error::Context)?,
            id,
        )?
        .state)
    }
    fn ensure_applied_subject(&mut self) -> Result<(), Self::Error> {
        let r = self
            .services
            .requirement_changes()
            .map_err(Error::Context)?;
        let key = object(r, self.object_id()?)?.key;
        let list = &mut self
            .services
            .requirement_changes_mut()
            .map_err(Error::Context)?
            .modifiers
            .iter_mut()
            .find(|m| m.id == self.id)
            .ok_or(Error::Missing("current modifier"))?
            .applied_subjects;
        let lower = list.partition_point(|v| v.compare(key).is_lt());
        if list.get(lower).is_none_or(|v| !v.compare(key).is_eq()) {
            list.try_reserve(1).map_err(|_| Error::Allocation)?;
            list.insert(lower, key);
        }
        Ok(())
    }
    fn is_repeatable(&self) -> Result<bool, Self::Error> {
        Ok(modifier(
            self.services
                .requirement_changes()
                .map_err(Error::Context)?,
            self.id,
        )?
        .repeatable)
    }
    fn apply_effect(&mut self) -> Result<bool, Self::Error> {
        m::apply_effect(self).map_err(|e| match e {
            m::EffectError::Context(e) => e,
            m::EffectError::DefinitionDisappeared => {
                Error::Missing("modifier definition disappeared")
            }
        })?;
        Ok(true)
    }
    fn unsubscribe_subject_requirement(&mut self) -> Result<(), Self::Error> {
        Err(Error::Missing("modifier subject subscription removal"))
    }
    fn clear_subject_requirement(&mut self) -> Result<(), Self::Error> {
        Err(Error::Missing("modifier subject reference release"))
    }
    fn remove_effects(&mut self) -> Result<(), Self::Error> {
        Err(Error::Missing("DynamicModifier.RemoveEffects"))
    }
}
impl<S: Services> m::EffectContext for ModifierContext<'_, S> {
    type Error = Error<S::Error>;
    fn mark_subject_affected(&mut self) -> Result<(), Self::Error> {
        let object_id = self.object_id()?;
        let r = self
            .services
            .requirement_changes_mut()
            .map_err(Error::Context)?;
        let m = r
            .modifiers
            .iter()
            .find(|m| m.id == self.id)
            .ok_or(Error::Missing("current modifier"))?;
        r.objects
            .iter_mut()
            .find(|o| o.id == object_id)
            .ok_or(Error::Missing("current modifier object"))?
            .affected
            .mark(&m.definition, m.instance)
            .map_err(|_| Error::Allocation)
    }

    fn suppressed(&self) -> Result<bool, Self::Error> {
        Ok(modifier(
            self.services
                .requirement_changes()
                .map_err(Error::Context)?,
            self.id,
        )?
        .value
        .suppressed)
    }
    fn subject_stack_limit(&self) -> Result<Option<u16>, Self::Error> {
        Ok(Some(
            modifier(
                self.services
                    .requirement_changes()
                    .map_err(Error::Context)?,
                self.id,
            )?
            .stack_limit,
        ))
    }
    fn affected_count(&self) -> Result<i32, Self::Error> {
        let r = self
            .services
            .requirement_changes()
            .map_err(Error::Context)?;
        Ok(object(r, self.object_id()?)?
            .affected
            .count(&modifier(r, self.id)?.definition))
    }
    fn log_applying(&mut self) -> Result<(), Self::Error> {
        if self
            .services
            .requirement_changes()
            .map_err(Error::Context)?
            .logging_priority
            < 3
        {
            Ok(())
        } else {
            Err(Error::Missing("active stock modifier output sink"))
        }
    }
    fn apply_concrete_effect(&mut self) -> Result<bool, Self::Error> {
        let r = self
            .services
            .requirement_changes()
            .map_err(Error::Context)?;
        let m = modifier(r, self.id)?;
        let o = object(r, self.object_id()?)?;
        let unit = UnitReference {
            owner: i32::from(o.key.component.owner),
            id: o.key.component.id,
        };
        effect::apply(self.state, Some(unit), m.effect).map_err(Error::Effect)
    }
    fn log_failed(&mut self) -> Result<(), Self::Error> {
        self.log_applying()
    }
}
