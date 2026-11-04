//! Fresh constant combat-subject requirement sets. Native definition lookup,
//! child initialization order, inverse, TestAll pruning and weak cache lifetime
//! are preserved. This is not the general dynamic RequirementSystem.
use super::registry::Handle;
use crate::unit_moved_signal::requirement_sets as all;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Predicate {
    AllianceWar,
    CombatVersus { expected: u32 },
    AllianceReligion,
    OpponentPromotionClass { expected: Vec<u8> },
    PlayerIsAttacking,
    Unsupported(u32),
}
#[derive(Clone, Debug)]
pub struct RequirementDefinition {
    pub id: Vec<u8>,
    pub predicate: Predicate,
    pub inverse: bool,
    pub persistent: bool,
}
#[derive(Clone, Debug)]
pub struct SetDefinition {
    pub id: Vec<u8>,
    pub requirements: Vec<Vec<u8>>,
}
#[derive(Debug)]
pub enum Error<E> {
    Context(E),
    Allocation,
    MissingDefinitionInput,
    MissingRequirementDefinition,
    UnsupportedPredicate(u32),
    MissingSet,
    MissingSubscription,
    CounterExhausted,
}
#[derive(Debug)]
struct Requirement {
    label: u64,
    native_id: u32,
    context: i32,
    subject: Handle,
    definition: usize,
    state: i32,
    references: u32,
}
#[derive(Debug)]
struct Set {
    label: u64,
    native_id: u32,
    context: i32,
    subject: Handle,
    definition: usize,
    state: i32,
    children: Vec<u64>,
    subscribers: Vec<u64>,
    alive: bool,
    references: u32,
}
#[derive(Debug)]
pub struct Runtime {
    next_native_id: u32,
    next_label: u64,
    // None means proven absent in the complete current definition collection.
    set_definitions: Vec<(Vec<u8>, Option<SetDefinition>)>,
    requirement_definitions: Vec<RequirementDefinition>,
    requirements: Vec<Requirement>,
    sets: Vec<Set>,
}
impl Runtime {
    pub fn from_initial(
        next_native_id: u32,
        set_definitions: Vec<(Vec<u8>, Option<SetDefinition>)>,
        requirement_definitions: Vec<RequirementDefinition>,
    ) -> Result<Self, &'static str> {
        for (i, (id, d)) in set_definitions.iter().enumerate() {
            if set_definitions[..i].iter().any(|x| x.0 == *id)
                || d.as_ref().is_some_and(|d| d.id != *id)
            {
                return Err("duplicate/mismatched set definition");
            }
        }
        for (i, d) in requirement_definitions.iter().enumerate() {
            if requirement_definitions[..i].iter().any(|x| x.id == d.id) {
                return Err("duplicate requirement definition");
            }
        }
        Ok(Self {
            next_native_id,
            next_label: 1 << 63,
            set_definitions,
            requirement_definitions,
            requirements: vec![],
            sets: vec![],
        })
    }
    pub fn next_native_id(&self) -> u32 {
        self.next_native_id
    }
    pub fn state(&self, id: u64) -> Option<i32> {
        self.sets
            .iter()
            .find(|s| s.label == id && s.alive)
            .map(|s| s.state)
    }
    pub fn native_set_id(&self, id: u64) -> Option<u32> {
        self.sets
            .iter()
            .find(|s| s.label == id && s.alive)
            .map(|s| s.native_id)
    }
    pub fn native_requirement_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.requirements.iter().map(|r| r.native_id)
    }
    pub fn children(&self, id: u64) -> Option<&[u64]> {
        self.sets
            .iter()
            .find(|s| s.label == id && s.alive)
            .map(|s| s.children.as_slice())
    }
    fn label<E>(&mut self) -> Result<u64, Error<E>> {
        let id = self.next_label;
        self.next_label = self
            .next_label
            .checked_add(1)
            .ok_or(Error::CounterExhausted)?;
        Ok(id)
    }
    fn native_id(&mut self) -> u32 {
        let id = self.next_native_id;
        self.next_native_id = self.next_native_id.wrapping_add(1);
        id
    }
    fn release_child<E>(&mut self, id: u64) -> Result<(), Error<E>> {
        let r = self
            .requirements
            .iter_mut()
            .find(|r| r.label == id)
            .ok_or(Error::MissingSet)?;
        r.references = r.references.checked_sub(1).ok_or(Error::MissingSet)?;
        Ok(())
    }
    /// Unsubscribe one actual modifier binding then release its set reference.
    /// Constant children have no state-change dispatch after their initialization.
    pub fn unsubscribe<E>(&mut self, receiver: u64, id: u64) -> Result<(), Error<E>> {
        let s = self
            .sets
            .iter_mut()
            .find(|s| s.label == id && s.alive)
            .ok_or(Error::MissingSet)?;
        let i = s
            .subscribers
            .iter()
            .position(|r| *r == receiver)
            .ok_or(Error::MissingSubscription)?;
        s.subscribers.remove(i);
        Ok(())
    }
    pub fn release<E>(&mut self, id: u64) -> Result<(), Error<E>> {
        let s = self
            .sets
            .iter_mut()
            .find(|s| s.label == id && s.alive)
            .ok_or(Error::MissingSet)?;
        s.references = s.references.checked_sub(1).ok_or(Error::MissingSet)?;
        if s.references == 0 {
            s.alive = false;
            let children = std::mem::take(&mut s.children);
            for id in children {
                self.release_child(id)?;
            }
        }
        Ok(())
    }
}
pub trait Context {
    type Error;
    fn subject_sets(&self) -> Result<&Runtime, Self::Error>;
    fn subject_sets_mut(&mut self) -> Result<&mut Runtime, Self::Error>;
    /// Executes the concrete current-state ConstRequirement.OnInitialize leaf,
    /// before inverse and before assigning its native requirement instance ID.
    fn evaluate_constant(
        &mut self,
        predicate: &Predicate,
        context: i32,
        subject: Handle,
    ) -> Result<i32, Self::Error>;
    fn log_missing_set(&mut self, id: &[u8]) -> Result<(), Self::Error>;
    fn log_created_requirement(&mut self, id: u32) -> Result<(), Self::Error>;
    fn log_created_set(&mut self, id: u32) -> Result<(), Self::Error>;
}
/// Exact ConstRequirement.Inverse233a9f0, including out-of-range fallback.
pub fn inverse(state: i32) -> i32 {
    match state {
        0 => 1,
        1 => 0,
        2 => 3,
        3 => 2,
        _ => 1,
    }
}

/// The source subject cache keys by definition bytes and CONTEXT INSTANCE ID,
/// not Unit ComponentID: two context wrappers for one Unit remain distinct.
/// Returned labels are internal identities; native integer IDs are retained
/// separately and allocated only after each successful object initialization.
pub fn get_or_create<C: Context>(
    c: &mut C,
    receiver: u64,
    context: i32,
    subject: Handle,
    id: &[u8],
) -> Result<Option<u64>, Error<C::Error>> {
    use Error::Context as Cx;
    let definition = c
        .subject_sets()
        .map_err(Cx)?
        .set_definitions
        .iter()
        .position(|d| d.0 == id)
        .ok_or(Error::MissingDefinitionInput)?;
    if let Some(s) = c.subject_sets_mut().map_err(Cx)?.sets.iter_mut().find(|s| {
        s.alive && s.context == context && s.subject == subject && s.definition == definition
    }) {
        s.subscribers
            .try_reserve(1)
            .map_err(|_| Error::Allocation)?;
        s.subscribers.push(receiver);
        s.references = s.references.checked_add(1).ok_or(Error::CounterExhausted)?;
        return Ok(Some(s.label));
    }
    if c.subject_sets().map_err(Cx)?.set_definitions[definition]
        .1
        .is_none()
    {
        c.log_missing_set(id).map_err(Cx)?;
        return Ok(None);
    }
    let count = c.subject_sets().map_err(Cx)?.set_definitions[definition]
        .1
        .as_ref()
        .unwrap()
        .requirements
        .len();
    let mut children = Vec::new();
    children.try_reserve(count).map_err(|_| Error::Allocation)?;
    // Native creates EVERY child before Attach/TestAll, even if the first child
    // already fails. Short-circuiting predicate initialization changes native IDs.
    for index in 0..count {
        let requirement = {
            let r = c.subject_sets().map_err(Cx)?;
            let name = &r.set_definitions[definition]
                .1
                .as_ref()
                .unwrap()
                .requirements[index];
            r.requirement_definitions
                .iter()
                .position(|d| d.id == *name)
                .ok_or(Error::MissingRequirementDefinition)?
        };
        if let Some(r) = c
            .subject_sets_mut()
            .map_err(Cx)?
            .requirements
            .iter_mut()
            .find(|r| {
                r.references != 0
                    && r.context == context
                    && r.subject == subject
                    && r.definition == requirement
            })
        {
            r.references = r.references.checked_add(1).ok_or(Error::CounterExhausted)?;
            children.push(r.label);
            continue;
        }
        let (predicate, do_inverse) = {
            let d = &c.subject_sets().map_err(Cx)?.requirement_definitions[requirement];
            (d.predicate.clone(), d.inverse)
        };
        if let Predicate::Unsupported(t) = predicate {
            return Err(Error::UnsupportedPredicate(t));
        }
        let state = c
            .evaluate_constant(&predicate, context, subject)
            .map_err(Cx)?;
        let state = if do_inverse { inverse(state) } else { state };
        let r = c.subject_sets_mut().map_err(Cx)?;
        r.requirements
            .try_reserve(1)
            .map_err(|_| Error::Allocation)?;
        let label = r.label()?;
        let native_id = r.native_id();
        r.requirements.push(Requirement {
            label,
            native_id,
            context,
            subject,
            definition: requirement,
            state,
            references: 1,
        });
        children.push(label);
        c.log_created_requirement(native_id).map_err(Cx)?;
    }
    let r = c.subject_sets_mut().map_err(Cx)?;
    r.sets.try_reserve(1).map_err(|_| Error::Allocation)?;
    let label = r.label()?;
    r.sets.push(Set {
        label,
        native_id: u32::MAX,
        context,
        subject,
        definition,
        state: 1,
        children,
        subscribers: vec![],
        alive: true,
        references: 1,
    });
    let mut initial = Initial::<C::Error> {
        runtime: r,
        label,
        _error: std::marker::PhantomData,
    };
    let state = all::test_all(&mut initial).map_err(|e| match e {
        all::Error::Context(e) => e,
        all::Error::Allocation => Error::Allocation,
    })?;
    all::set_state(&mut initial, state, true).map_err(|e| match e {
        all::Error::Context(e) => e,
        all::Error::Allocation => Error::Allocation,
    })?;
    let r = c.subject_sets_mut().map_err(Cx)?;
    let native_id = r.native_id();
    let s = r.sets.iter_mut().find(|s| s.label == label).unwrap();
    s.native_id = native_id;
    c.log_created_set(native_id).map_err(Cx)?;
    let r = c.subject_sets_mut().map_err(Cx)?;
    let s = r.sets.iter_mut().find(|s| s.label == label).unwrap();
    s.subscribers
        .try_reserve(1)
        .map_err(|_| Error::Allocation)?;
    s.subscribers.push(receiver);
    Ok(Some(label))
}
struct Initial<'a, E> {
    runtime: &'a mut Runtime,
    label: u64,
    _error: std::marker::PhantomData<E>,
}
impl<E> Initial<'_, E> {
    fn set(&self) -> &Set {
        self.runtime
            .sets
            .iter()
            .find(|s| s.label == self.label)
            .unwrap()
    }
    fn set_mut(&mut self) -> &mut Set {
        self.runtime
            .sets
            .iter_mut()
            .find(|s| s.label == self.label)
            .unwrap()
    }
}
impl<E> all::Context for Initial<'_, E> {
    type Error = Error<E>;
    fn children(&self) -> Result<&[u64], Self::Error> {
        Ok(&self.set().children)
    }
    fn child_state(&self, id: u64) -> Result<i32, Self::Error> {
        self.runtime
            .requirements
            .iter()
            .find(|r| r.label == id && r.references != 0)
            .map(|r| r.state)
            .ok_or(Error::MissingSet)
    }
    fn child_persistent(&self, id: u64) -> Result<bool, Self::Error> {
        let r = self
            .runtime
            .requirements
            .iter()
            .find(|r| r.label == id && r.references != 0)
            .ok_or(Error::MissingSet)?;
        Ok(self.runtime.requirement_definitions[r.definition].persistent)
    }
    fn remove_child(&mut self, id: u64) -> Result<(), Self::Error> {
        let s = self.set_mut();
        let i = s
            .children
            .iter()
            .position(|x| *x == id)
            .ok_or(Error::MissingSet)?;
        s.children.remove(i);
        self.runtime.release_child(id)
    }
    fn remove_all_children(&mut self) -> Result<(), Self::Error> {
        let children = std::mem::take(&mut self.set_mut().children);
        for id in children {
            self.runtime.release_child(id)?;
        }
        Ok(())
    }
    fn state(&self) -> Result<i32, Self::Error> {
        Ok(self.set().state)
    }
    fn store_state(&mut self, state: i32) -> Result<(), Self::Error> {
        self.set_mut().state = state;
        Ok(())
    }
    fn changed(&mut self, _: i32, _: i32) -> Result<(), Self::Error> {
        if self.set().subscribers.is_empty() {
            Ok(())
        } else {
            Err(Error::MissingSubscription)
        }
    }
    fn reset_trigger(&mut self) -> Result<(), Self::Error> {
        Err(Error::MissingSet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        map::UnitReference,
        melee_attack::proposals::registry::{Proposal, Runtime as Proposals},
    };
    struct Fixture {
        runtime: Runtime,
        calls: Vec<Predicate>,
    }
    impl Context for Fixture {
        type Error = &'static str;
        fn subject_sets(&self) -> Result<&Runtime, Self::Error> {
            Ok(&self.runtime)
        }
        fn subject_sets_mut(&mut self) -> Result<&mut Runtime, Self::Error> {
            Ok(&mut self.runtime)
        }
        fn evaluate_constant(
            &mut self,
            p: &Predicate,
            _: i32,
            _: Handle,
        ) -> Result<i32, Self::Error> {
            self.calls.push(p.clone());
            Ok(match p {
                Predicate::AllianceWar => 3,
                Predicate::CombatVersus { .. } => 2,
                Predicate::AllianceReligion => 0,
                _ => return Err("unsupported concrete predicate"),
            })
        }
        fn log_missing_set(&mut self, _: &[u8]) -> Result<(), Self::Error> {
            Ok(())
        }
        fn log_created_requirement(&mut self, _: u32) -> Result<(), Self::Error> {
            Ok(())
        }
        fn log_created_set(&mut self, _: u32) -> Result<(), Self::Error> {
            Ok(())
        }
    }
    fn subject() -> Handle {
        let mut p = Proposals::from_initial(0, vec![], None, None, None).unwrap();
        let u = UnitReference {
            owner: 0x1000c,
            id: 1,
        };
        p.register(Proposal::melee_units(u, u, None)).unwrap().1
    }
    fn fixture(ids: &[&[u8]], inverse: bool) -> Fixture {
        let defs = vec![
            RequirementDefinition {
                id: b"war".to_vec(),
                predicate: Predicate::AllianceWar,
                inverse: false,
                persistent: false,
            },
            RequirementDefinition {
                id: b"versus".to_vec(),
                predicate: Predicate::CombatVersus { expected: 1 },
                inverse: false,
                persistent: false,
            },
            RequirementDefinition {
                id: b"religion".to_vec(),
                predicate: Predicate::AllianceReligion,
                inverse,
                persistent: false,
            },
        ];
        let sets = vec![
            (
                b"set".to_vec(),
                Some(SetDefinition {
                    id: b"set".to_vec(),
                    requirements: ids.iter().map(|s| s.to_vec()).collect(),
                }),
            ),
            (b"absent".to_vec(), None),
        ];
        Fixture {
            runtime: Runtime::from_initial(100, sets, defs).unwrap(),
            calls: vec![],
        }
    }
    #[test]
    fn all_children_initialize_before_failing_test_and_ids_preserve_order() {
        let p = subject();
        let mut f = fixture(&[b"war", b"versus"], false);
        let id = get_or_create(&mut f, 7, 9, p, b"set").unwrap().unwrap();
        assert_eq!(
            f.calls,
            vec![
                Predicate::AllianceWar,
                Predicate::CombatVersus { expected: 1 }
            ]
        );
        assert_eq!(
            f.runtime.native_requirement_ids().collect::<Vec<_>>(),
            vec![100, 101]
        );
        assert_eq!(f.runtime.native_set_id(id), Some(102));
        assert_eq!(f.runtime.next_native_id(), 103);
        assert_eq!(f.runtime.state(id), Some(3));
        assert_eq!(f.runtime.children(id), Some([].as_slice()));
    }
    #[test]
    fn source_cache_reuses_live_set_but_context_wrappers_remain_distinct() {
        let p = subject();
        let mut f = fixture(&[b"religion"], false);
        let a = get_or_create(&mut f, 7, 9, p, b"set").unwrap().unwrap();
        assert_eq!(get_or_create(&mut f, 8, 9, p, b"set").unwrap(), Some(a));
        let b = get_or_create(&mut f, 9, 10, p, b"set").unwrap().unwrap();
        assert_ne!(a, b);
        assert_eq!(f.calls.len(), 2);
        assert_eq!(f.runtime.next_native_id(), 104);
        f.runtime.unsubscribe::<()>(7, a).unwrap();
        f.runtime.release::<()>(a).unwrap();
        assert_eq!(f.runtime.state(a), Some(0));
        f.runtime.unsubscribe::<()>(8, a).unwrap();
        f.runtime.release::<()>(a).unwrap();
        assert_eq!(f.runtime.state(a), None);
        let c = get_or_create(&mut f, 10, 9, p, b"set").unwrap().unwrap();
        assert_ne!(c, a);
        assert_eq!(f.calls.len(), 3);
    }
    #[test]
    fn inverse_is_const_mapping_without_trigger_promotion() {
        let p = subject();
        let mut f = fixture(&[b"religion"], true);
        let id = get_or_create(&mut f, 7, 9, p, b"set").unwrap().unwrap();
        assert_eq!(f.runtime.state(id), Some(1));
        assert_eq!(
            [
                inverse(0),
                inverse(1),
                inverse(2),
                inverse(3),
                inverse(4),
                inverse(-1)
            ],
            [1, 0, 3, 2, 1, 1]
        );
    }
    #[test]
    fn proven_absence_is_distinct_from_an_unprojected_definition() {
        let p = subject();
        let mut f = fixture(&[], false);
        assert_eq!(get_or_create(&mut f, 7, 9, p, b"absent").unwrap(), None);
        assert!(matches!(
            get_or_create(&mut f, 7, 9, p, b"unknown"),
            Err(Error::MissingDefinitionInput)
        ));
        assert!(f.calls.is_empty());
        assert_eq!(f.runtime.next_native_id(), 100);
    }
    #[test]
    fn true_nonpersistent_const_children_are_pruned_but_set_stays_met() {
        let p = subject();
        let mut f = fixture(&[b"versus"], false);
        let id = get_or_create(&mut f, 7, 9, p, b"set").unwrap().unwrap();
        assert_eq!(f.runtime.state(id), Some(2));
        assert!(f.runtime.children(id).unwrap().is_empty());
        assert_eq!(f.runtime.requirements[0].references, 0);
    }
    #[test]
    fn native_counter_wrap_does_not_reuse_logical_identity() {
        let p = subject();
        let mut f = fixture(&[b"religion"], false);
        f.runtime.next_native_id = u32::MAX;
        let id = get_or_create(&mut f, 7, 9, p, b"set").unwrap().unwrap();
        assert_eq!(
            f.runtime.native_requirement_ids().collect::<Vec<_>>(),
            vec![u32::MAX]
        );
        assert_eq!(f.runtime.native_set_id(id), Some(0));
        assert_eq!(f.runtime.next_native_id(), 1);
    }
}
