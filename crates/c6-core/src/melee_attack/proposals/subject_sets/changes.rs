//! Synchronous changes and final releases over the SAME subject-set owner.
//! Concrete predicate, modifier and native subscription leaves are required.
use super::{
    all, Error, InitializedRequirement, RequirementDefinition, Runtime, SetDefinition, SetListener,
    Subject,
};
use crate::simple_requirement as simple;
use crate::unit_moved_signal::ListenerRing;

/// Already-initialized live objects from an admitted current snapshot. These
/// are state/registrations, not native Test results; importing assigns no IDs.
pub struct CurrentRequirement {
    pub label: u64,
    pub native_id: u32,
    pub context: i32,
    pub subject: Subject,
    pub definition: RequirementDefinition,
    pub value: InitializedRequirement,
    pub references: u32,
    /// Actual state-change subscribers, in native linked-list order.
    pub subscribers: Vec<u64>,
}
pub struct CurrentSet {
    pub label: u64,
    pub native_id: u32,
    pub context: i32,
    pub subject: Subject,
    pub definition: SetDefinition,
    pub state: i32,
    pub references: u32,
    pub children: Vec<u64>,
    pub subscribers: Vec<SetListener>,
}
impl Runtime {
    pub fn import_current<E>(
        &mut self,
        requirements: Vec<CurrentRequirement>,
        sets: Vec<CurrentSet>,
    ) -> Result<(), Error<E>> {
        for (index, row) in requirements.iter().enumerate() {
            if row.label == 0
                || row.label >= 1 << 63
                || row.references == 0
                || self.requirements.iter().any(|r| r.label == row.label)
                || requirements[..index].iter().any(|r| r.label == row.label)
            {
                return Err(Error::CurrentInput("duplicate/invalid current requirement"));
            }
        }
        for (index, row) in sets.iter().enumerate() {
            if row.label == 0
                || row.label >= 1 << 63
                || row.references == 0
                || self.sets.iter().any(|r| r.label == row.label)
                || sets[..index].iter().any(|r| r.label == row.label)
            {
                return Err(Error::CurrentInput("duplicate/invalid current set"));
            }
        }
        self.requirements
            .try_reserve(requirements.len())
            .map_err(|_| Error::Allocation)?;
        self.sets
            .try_reserve(sets.len())
            .map_err(|_| Error::Allocation)?;
        for row in requirements {
            let definition = if let Some(index) = self
                .requirement_definitions
                .iter()
                .position(|d| d.id == row.definition.id)
            {
                let previous = &self.requirement_definitions[index];
                if previous.inverse != row.definition.inverse
                    || previous.persistent != row.definition.persistent
                {
                    return Err(Error::CurrentInput(
                        "current requirement definition mismatch",
                    ));
                }
                index
            } else {
                self.requirement_definitions
                    .try_reserve(1)
                    .map_err(|_| Error::Allocation)?;
                let index = self.requirement_definitions.len();
                self.requirement_definitions.push(row.definition);
                index
            };
            self.requirements.push(super::Requirement {
                label: row.label,
                native_id: row.native_id,
                context: row.context,
                subject: row.subject,
                definition,
                value: row.value,
                references: row.references,
                changed: ListenerRing::from_ordered(&row.subscribers).map_err(Error::Ring)?,
            });
        }
        for row in sets {
            let definition = if let Some(index) = self
                .set_definitions
                .iter()
                .position(|d| d.0 == row.definition.id)
            {
                if self.set_definitions[index]
                    .1
                    .as_ref()
                    .is_none_or(|d| d.requirements != row.definition.requirements)
                {
                    return Err(Error::CurrentInput("current set definition mismatch"));
                }
                index
            } else {
                self.set_definitions
                    .try_reserve(1)
                    .map_err(|_| Error::Allocation)?;
                let index = self.set_definitions.len();
                self.set_definitions
                    .push((row.definition.id.clone(), Some(row.definition)));
                index
            };
            let mut tokens = Vec::new();
            tokens
                .try_reserve(row.children.len())
                .map_err(|_| Error::Allocation)?;
            for child in &row.children {
                let requirement = self
                    .requirements
                    .iter()
                    .find(|r| r.label == *child && r.references != 0)
                    .ok_or(Error::MissingSet)?;
                tokens.push(
                    requirement
                        .changed
                        .subscription_for(|set| set == row.label)
                        .map_err(Error::Ring)?,
                );
            }
            let mut changed = ListenerRing::from_ordered(&[]).map_err(Error::Ring)?;
            let mut subscribers = Vec::new();
            subscribers
                .try_reserve(row.subscribers.len())
                .map_err(|_| Error::Allocation)?;
            for listener in row.subscribers {
                let token = changed.subscribe(listener).map_err(Error::Ring)?;
                subscribers.push((listener, token));
            }
            self.sets.push(super::Set {
                label: row.label,
                native_id: row.native_id,
                context: row.context,
                subject: row.subject,
                definition,
                state: row.state,
                children: row.children,
                subscribers,
                changed,
                child_subscriptions: tokens,
                alive: true,
                references: row.references,
            });
        }
        Ok(())
    }
    pub fn requirement_context(&self, label: u64) -> Option<(i32, Subject)> {
        self.requirements
            .iter()
            .find(|r| r.label == label)
            .map(|r| (r.context, r.subject))
    }
    pub fn set_has_subscribers(&self, label: u64) -> Option<bool> {
        self.sets
            .iter()
            .find(|s| s.label == label)
            .map(|s| !s.subscribers.is_empty())
    }
}

pub trait Context {
    type Error;
    fn subject_sets(&self) -> Result<&Runtime, Self::Error>;
    fn subject_sets_mut(&mut self) -> Result<&mut Runtime, Self::Error>;
    fn test_requirement(&mut self, requirement: u64) -> Result<i32, Error<Self::Error>>;
    fn modifier_changed(
        &mut self,
        listener: SetListener,
        set: u64,
        old: i32,
        new: i32,
    ) -> Result<(), Error<Self::Error>>;
    /// Last shared owner has gone. Execute the concrete derived requirement's
    /// subscription destruction using this same owner and current game state.
    fn destroy_requirement(&mut self, requirement: u64) -> Result<(), Error<Self::Error>>;
    fn destroy_set(&mut self, set: u64) -> Result<(), Error<Self::Error>>;
    fn reset_trigger(&mut self, set: u64) -> Result<(), Error<Self::Error>>;
}
fn requirement_mut<E>(
    runtime: &mut Runtime,
    label: u64,
) -> Result<&mut super::Requirement, Error<E>> {
    runtime
        .requirements
        .iter_mut()
        .find(|r| r.label == label)
        .ok_or(Error::MissingSet)
}
fn set_mut<E>(runtime: &mut Runtime, label: u64) -> Result<&mut super::Set, Error<E>> {
    runtime
        .sets
        .iter_mut()
        .find(|r| r.label == label)
        .ok_or(Error::MissingSet)
}
/// SimpleRequirementSet.ResetTrigger: only state4 resets to0; no signal.
pub fn reset_trigger_state<E>(runtime: &mut Runtime, label: u64) -> Result<(), Error<E>> {
    let set = set_mut(runtime, label)?;
    if set.state == 4 {
        set.state = 0;
    }
    Ok(())
}
fn release_child<C: Context>(c: &mut C, label: u64) -> Result<(), Error<C::Error>> {
    let destroy = {
        let r = requirement_mut(c.subject_sets_mut().map_err(Error::Context)?, label)?;
        r.references = r.references.checked_sub(1).ok_or(Error::MissingSet)?;
        r.references == 0 && matches!(r.value, InitializedRequirement::Simple(_))
    };
    if destroy {
        c.destroy_requirement(label)?;
        // Derived destructor first unsubscribes its external listeners;
        // SimpleRequirement::~ then calls Deactivate (active=false, no signal).
        if let InitializedRequirement::Simple(value) =
            &mut requirement_mut(c.subject_sets_mut().map_err(Error::Context)?, label)?.value
        {
            value.active = false;
        }
    }
    Ok(())
}
fn remove_child<C: Context>(c: &mut C, set: u64, child: u64) -> Result<(), Error<C::Error>> {
    c.subject_sets_mut()
        .map_err(Error::Context)?
        .detach_child(set, child)?;
    release_child(c, child)
}
fn remove_all<C: Context>(c: &mut C, set: u64) -> Result<(), Error<C::Error>> {
    let children = c
        .subject_sets_mut()
        .map_err(Error::Context)?
        .detach_all_children(set)?;
    // RemoveAllRequirements moves the two vectors out, unsubscribes all
    // handles, then vector destruction releases child shared_ptrs from the end.
    for child in children.into_iter().rev() {
        release_child(c, child)?;
    }
    Ok(())
}
pub fn release<C: Context>(c: &mut C, label: u64) -> Result<(), Error<C::Error>> {
    let final_owner = {
        let set = set_mut(c.subject_sets_mut().map_err(Error::Context)?, label)?;
        if !set.alive {
            return Err(Error::MissingSet);
        }
        set.references = set.references.checked_sub(1).ok_or(Error::MissingSet)?;
        if set.references == 0 {
            set.alive = false;
            true
        } else {
            false
        }
    };
    if final_owner {
        remove_all(c, label)?;
        c.destroy_set(label)?;
    }
    Ok(())
}

struct RequirementContext<'a, C> {
    outer: &'a mut C,
    label: u64,
}
impl<C: Context> simple::Context for RequirementContext<'_, C> {
    type Error = Error<C::Error>;
    fn active(&mut self) -> Result<bool, Self::Error> {
        let r = self
            .outer
            .subject_sets()
            .map_err(Error::Context)?
            .simple_requirement(self.label)
            .ok_or(Error::MissingSet)?;
        Ok(r.active)
    }
    fn requirement(&mut self) -> Result<&mut simple::Requirement, Self::Error> {
        self.outer
            .subject_sets_mut()
            .map_err(Error::Context)?
            .simple_requirement_mut(self.label)
            .ok_or(Error::MissingSet)
    }
    fn changed(&mut self, old: i32, new: i32) -> Result<(), Self::Error> {
        dispatch_requirement_changed(self.outer, self.label, old, new)
    }
}
impl<C: Context> simple::TestContext for RequirementContext<'_, C> {
    fn test(&mut self) -> Result<i32, Self::Error> {
        self.outer.test_requirement(self.label)
    }
}
pub fn invalidate<C: Context>(c: &mut C, label: u64) -> Result<(), Error<C::Error>> {
    simple::invalidate(&mut RequirementContext { outer: c, label })
}
pub fn set_requirement_state<C: Context>(
    c: &mut C,
    label: u64,
    tested: i32,
    notify: bool,
) -> Result<(), Error<C::Error>> {
    simple::set_state(&mut RequirementContext { outer: c, label }, tested, notify)
}
/// Dispatch the current requirement's existing listener ring after an external
/// concrete predicate has already assigned its state. No second state store.
pub fn dispatch_requirement_changed<C: Context>(
    c: &mut C,
    label: u64,
    _old: i32,
    new: i32,
) -> Result<(), Error<C::Error>> {
    let mut cursor = requirement_mut(c.subject_sets_mut().map_err(Error::Context)?, label)?
        .changed
        .cursor_start()
        .map_err(Error::Ring)?;
    loop {
        let route = requirement_mut(c.subject_sets_mut().map_err(Error::Context)?, label)?
            .changed
            .cursor_route(cursor)
            .map_err(Error::Ring)?;
        if let Some(set) = route {
            if let Err(error) = requirement_changed(c, set, new) {
                requirement_mut(c.subject_sets_mut().map_err(Error::Context)?, label)?
                    .changed
                    .cursor_release(cursor)
                    .map_err(Error::Ring)?;
                return Err(error);
            }
        }
        match requirement_mut(c.subject_sets_mut().map_err(Error::Context)?, label)?
            .changed
            .cursor_next(cursor)
            .map_err(Error::Ring)?
        {
            Some(next) => cursor = next,
            None => return Ok(()),
        }
    }
}
struct SetContext<'a, C> {
    outer: &'a mut C,
    label: u64,
}
impl<C: Context> all::Context for SetContext<'_, C> {
    type Error = Error<C::Error>;
    fn children(&self) -> Result<&[u64], Self::Error> {
        self.outer
            .subject_sets()
            .map_err(Error::Context)?
            .children(self.label)
            .ok_or(Error::MissingSet)
    }
    fn child_state(&self, label: u64) -> Result<i32, Self::Error> {
        self.outer
            .subject_sets()
            .map_err(Error::Context)?
            .requirement_state(label)
            .ok_or(Error::MissingSet)
    }
    fn child_persistent(&self, label: u64) -> Result<bool, Self::Error> {
        let r = self.outer.subject_sets().map_err(Error::Context)?;
        let child = r
            .requirements
            .iter()
            .find(|r| r.label == label && r.references != 0)
            .ok_or(Error::MissingSet)?;
        Ok(r.requirement_definitions[child.definition].persistent)
    }
    fn remove_child(&mut self, child: u64) -> Result<(), Self::Error> {
        remove_child(self.outer, self.label, child)
    }
    fn remove_all_children(&mut self) -> Result<(), Self::Error> {
        remove_all(self.outer, self.label)
    }
    fn state(&self) -> Result<i32, Self::Error> {
        self.outer
            .subject_sets()
            .map_err(Error::Context)?
            .state(self.label)
            .ok_or(Error::MissingSet)
    }
    fn store_state(&mut self, value: i32) -> Result<(), Self::Error> {
        set_mut(
            self.outer.subject_sets_mut().map_err(Error::Context)?,
            self.label,
        )?
        .state = value;
        Ok(())
    }
    fn changed(&mut self, old: i32, new: i32) -> Result<(), Self::Error> {
        set_changed_dispatch(self.outer, self.label, old, new)
    }
    fn reset_trigger(&mut self) -> Result<(), Self::Error> {
        self.outer.reset_trigger(self.label)
    }
}
pub fn requirement_changed<C: Context>(
    c: &mut C,
    set: u64,
    incoming: i32,
) -> Result<(), Error<C::Error>> {
    let any = c
        .subject_sets()
        .map_err(Error::Context)?
        .is_any(set)
        .ok_or(Error::MissingSet)?;
    all::requirement_changed_kind(
        &mut SetContext {
            outer: c,
            label: set,
        },
        incoming,
        any,
    )
    .map_err(|e| match e {
        all::Error::Context(e) => e,
        all::Error::Allocation => Error::Allocation,
    })
}
fn set_changed_dispatch<C: Context>(
    c: &mut C,
    label: u64,
    old: i32,
    new: i32,
) -> Result<(), Error<C::Error>> {
    let mut cursor = set_mut(c.subject_sets_mut().map_err(Error::Context)?, label)?
        .changed
        .cursor_start()
        .map_err(Error::Ring)?;
    loop {
        let route = set_mut(c.subject_sets_mut().map_err(Error::Context)?, label)?
            .changed
            .cursor_route(cursor)
            .map_err(Error::Ring)?;
        if let Some(listener) = route {
            if let Err(error) = c.modifier_changed(listener, label, old, new) {
                set_mut(c.subject_sets_mut().map_err(Error::Context)?, label)?
                    .changed
                    .cursor_release(cursor)
                    .map_err(Error::Ring)?;
                return Err(error);
            }
        }
        match set_mut(c.subject_sets_mut().map_err(Error::Context)?, label)?
            .changed
            .cursor_next(cursor)
            .map_err(Error::Ring)?
        {
            Some(next) => cursor = next,
            None => return Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        melee_attack::proposals::subject_sets::Predicate, simple_requirement::Requirement,
    };
    const CHILD: u64 = 0x1000;
    const A: u64 = 0x2000;
    const B: u64 = 0x3000;
    struct Fixture {
        runtime: Runtime,
        tested: i32,
        release_next: bool,
        calls: Vec<(u64, u64)>,
    }
    impl Fixture {
        fn new(two: bool, persistent: bool) -> Self {
            let mut runtime = Runtime::from_initial(99, vec![], vec![]).unwrap();
            let definition = RequirementDefinition {
                id: b"child".to_vec(),
                predicate: Predicate::Unsupported(7),
                inverse: false,
                persistent,
            };
            let child = CurrentRequirement {
                label: CHILD,
                native_id: 7,
                context: 1,
                subject: Subject::ModifierObject(42),
                definition,
                value: InitializedRequirement::Simple(Requirement {
                    active: true,
                    state: 1,
                    inverse: false,
                    persistent,
                    triggered: false,
                }),
                references: if two { 2 } else { 1 },
                subscribers: if two { vec![B, A] } else { vec![A] },
            };
            let set = |label, native_id, receiver, id: &[u8]| CurrentSet {
                label,
                native_id,
                context: 1,
                subject: Subject::ModifierObject(42),
                definition: SetDefinition {
                    id: id.to_vec(),
                    requirements: vec![b"child".to_vec()],
                },
                state: 1,
                references: 1,
                children: vec![CHILD],
                subscribers: vec![SetListener::Subject(receiver)],
            };
            let mut sets = vec![set(A, 8, 10, b"a")];
            if two {
                sets.push(set(B, 9, 20, b"b"));
            }
            runtime.import_current::<&str>(vec![child], sets).unwrap();
            assert_eq!(runtime.next_native_id(), 99);
            Self {
                runtime,
                tested: 0,
                release_next: false,
                calls: vec![],
            }
        }
    }
    impl Context for Fixture {
        type Error = &'static str;
        fn subject_sets(&self) -> Result<&Runtime, Self::Error> {
            Ok(&self.runtime)
        }
        fn subject_sets_mut(&mut self) -> Result<&mut Runtime, Self::Error> {
            Ok(&mut self.runtime)
        }
        fn test_requirement(&mut self, _: u64) -> Result<i32, Error<Self::Error>> {
            Ok(self.tested)
        }
        fn modifier_changed(
            &mut self,
            listener: SetListener,
            set: u64,
            _: i32,
            _: i32,
        ) -> Result<(), Error<Self::Error>> {
            self.calls.push((listener.receiver(), set));
            if self.release_next && set == B {
                self.runtime.unsubscribe::<&str>(10, A)?;
                release(self, A)?;
            }
            Ok(())
        }
        fn destroy_requirement(&mut self, child: u64) -> Result<(), Error<Self::Error>> {
            let r = self
                .runtime
                .requirements
                .iter()
                .find(|r| r.label == child)
                .unwrap();
            assert_eq!(r.references, 0);
            assert!(matches!(
                r.value,
                InitializedRequirement::Simple(Requirement { active: true, .. })
            ));
            self.calls.push((0, child));
            Ok(())
        }
        fn destroy_set(&mut self, set: u64) -> Result<(), Error<Self::Error>> {
            assert_eq!(self.runtime.set_has_subscribers(set), Some(false));
            self.calls.push((1, set));
            Ok(())
        }
        fn reset_trigger(&mut self, set: u64) -> Result<(), Error<Self::Error>> {
            reset_trigger_state(&mut self.runtime, set)
        }
    }
    #[test]
    fn native_order_survives_reentrant_removal_of_next_set() {
        let mut f = Fixture::new(true, true);
        f.release_next = true;
        invalidate(&mut f, CHILD).unwrap();
        assert_eq!(f.calls, vec![(20, B), (1, A)]);
        assert_eq!(f.runtime.state(A), None);
        assert_eq!(f.runtime.state(B), Some(0));
        f.runtime.unsubscribe::<&str>(20, B).unwrap();
        release(&mut f, B).unwrap();
        assert_eq!(&f.calls[2..], &[(0, CHILD), (1, B)]);
        assert_eq!(f.runtime.requirement_state(CHILD), None);
    }
    #[test]
    fn satisfied_nonpersistent_child_can_die_during_its_own_dispatch() {
        let mut f = Fixture::new(false, false);
        f.tested = 2;
        invalidate(&mut f, CHILD).unwrap();
        assert_eq!(f.calls, vec![(0, CHILD), (10, A)]);
        assert_eq!(f.runtime.state(A), Some(2));
        assert_eq!(f.runtime.children(A), Some(&[][..]));
        let value = &f
            .runtime
            .requirements
            .iter()
            .find(|r| r.label == CHILD)
            .unwrap()
            .value;
        assert!(matches!(
            value,
            InitializedRequirement::Simple(Requirement {
                active: false,
                state: 2,
                ..
            })
        ));
    }
    #[test]
    fn external_store_dispatches_trigger_then_resets_only_the_set() {
        let mut f = Fixture::new(false, false);
        f.runtime.simple_requirement_mut(CHILD).unwrap().state = 4;
        dispatch_requirement_changed(&mut f, CHILD, 1, 4).unwrap();
        assert_eq!(f.calls, [(10, A)]);
        assert_eq!(f.runtime.state(A), Some(0));
        assert_eq!(f.runtime.requirement_state(CHILD), Some(4));
        dispatch_requirement_changed(&mut f, CHILD, 4, 4).unwrap();
        assert_eq!(f.calls, [(10, A), (10, A)]);
        assert_eq!(f.runtime.state(A), Some(0));
        set_mut::<&str>(&mut f.runtime, A).unwrap().state = 1;
        reset_trigger_state::<&str>(&mut f.runtime, A).unwrap();
        assert_eq!(f.runtime.state(A), Some(1));
    }
}
