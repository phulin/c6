//! Shared subject requirement sets. Native definition lookup,
//! child initialization order, inverse, TestAll pruning and weak cache lifetime
//! are preserved. Only the explicitly supplied factories/predicates are supported.
use super::registry::Handle;
use crate::unit_moved_signal::requirement_sets as all;
use crate::unit_moved_signal::{ListenerRing, RegistryError, Subscription};

pub mod changes;
pub mod player_has_district;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SetListener {
    Subject(u64),
    Owner(u64),
    /// A different current synchronous set listener. Its concrete continuation
    /// remains mandatory at dispatch; this token carries no native address.
    External(u64),
}
impl SetListener {
    fn receiver(self) -> u64 {
        match self {
            Self::Subject(id) | Self::Owner(id) | Self::External(id) => id,
        }
    }
}

/// Actual modifier-object identity. A City wrapper never masquerades as a
/// ProposedCombat handle, even if native integer bits happen to coincide.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Subject {
    Proposal(Handle),
    ModifierObject(u64),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InitializedRequirement {
    Const(i32),
    Simple(crate::simple_requirement::Requirement),
}
impl InitializedRequirement {
    pub fn state(&self) -> i32 {
        match self {
            Self::Const(state) => *state,
            Self::Simple(value) => value.state,
        }
    }
}
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
    UnsupportedSubject,
    DynamicReleaseRequired(u64),
    Ring(RegistryError),
    CurrentInput(&'static str),
}
#[derive(Debug)]
struct Requirement {
    label: u64,
    native_id: u32,
    context: i32,
    subject: Subject,
    definition: usize,
    value: InitializedRequirement,
    references: u32,
    changed: ListenerRing<u64>,
}
#[derive(Debug)]
struct Set {
    label: u64,
    native_id: u32,
    context: i32,
    subject: Subject,
    definition: usize,
    state: i32,
    children: Vec<u64>,
    subscribers: Vec<(SetListener, Subscription)>,
    changed: ListenerRing<SetListener>,
    child_subscriptions: Vec<Subscription>,
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
    /// Immutable identity metadata remains available during the final derived
    /// destructor after its last shared reference has been released.
    pub fn requirement_definition(&self, label: u64) -> Option<&RequirementDefinition> {
        let row = self.requirements.iter().find(|r| r.label == label)?;
        self.requirement_definitions.get(row.definition)
    }
    pub fn requirement_state(&self, label: u64) -> Option<i32> {
        self.requirements
            .iter()
            .find(|r| r.label == label && r.references != 0)
            .map(|r| r.value.state())
    }
    pub fn simple_requirement(
        &self,
        label: u64,
    ) -> Option<&crate::simple_requirement::Requirement> {
        match &self
            .requirements
            .iter()
            .find(|r| r.label == label && r.references != 0)?
            .value
        {
            InitializedRequirement::Simple(v) => Some(v),
            _ => None,
        }
    }
    pub fn simple_requirement_mut(
        &mut self,
        label: u64,
    ) -> Option<&mut crate::simple_requirement::Requirement> {
        match &mut self
            .requirements
            .iter_mut()
            .find(|r| r.label == label && r.references != 0)?
            .value
        {
            InitializedRequirement::Simple(v) => Some(v),
            _ => None,
        }
    }
    pub fn requirement_subject(&self, label: u64) -> Option<Subject> {
        self.requirements
            .iter()
            .find(|r| r.label == label && r.references != 0)
            .map(|r| r.subject)
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
        if r.references == 1 && matches!(r.value, InitializedRequirement::Simple(_)) {
            return Err(Error::DynamicReleaseRequired(id));
        }
        r.references = r.references.checked_sub(1).ok_or(Error::MissingSet)?;
        Ok(())
    }
    fn detach_child<E>(&mut self, set: u64, child: u64) -> Result<(), Error<E>> {
        let set = self
            .sets
            .iter_mut()
            .find(|row| row.label == set && row.alive)
            .ok_or(Error::MissingSet)?;
        let index = set
            .children
            .iter()
            .position(|id| *id == child)
            .ok_or(Error::MissingSet)?;
        if index >= set.child_subscriptions.len() {
            return Err(Error::MissingSubscription);
        }
        set.children.remove(index);
        let token = set.child_subscriptions.remove(index);
        self.requirements
            .iter_mut()
            .find(|row| row.label == child)
            .ok_or(Error::MissingSet)?
            .changed
            .unsubscribe(token)
            .map_err(Error::Ring)
    }
    fn detach_all_children<E>(&mut self, set: u64) -> Result<Vec<u64>, Error<E>> {
        let set = self
            .sets
            .iter_mut()
            .find(|row| row.label == set)
            .ok_or(Error::MissingSet)?;
        if set.children.len() != set.child_subscriptions.len() {
            return Err(Error::MissingSubscription);
        }
        let children = std::mem::take(&mut set.children);
        let tokens = std::mem::take(&mut set.child_subscriptions);
        for (child, token) in children.iter().zip(tokens) {
            self.requirements
                .iter_mut()
                .find(|row| row.label == *child)
                .ok_or(Error::MissingSet)?
                .changed
                .unsubscribe(token)
                .map_err(Error::Ring)?;
        }
        Ok(children)
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
            .position(|r| r.0.receiver() == receiver)
            .ok_or(Error::MissingSubscription)?;
        let (_, token) = s.subscribers.remove(i);
        s.changed.unsubscribe(token).map_err(Error::Ring)?;
        Ok(())
    }
    pub fn release<E>(&mut self, id: u64) -> Result<(), Error<E>> {
        if let Some(s) = self.sets.iter().find(|s| s.label == id && s.alive) {
            if s.references == 1 {
                for child in &s.children {
                    if self.requirements.iter().any(|r| {
                        r.label == *child
                            && r.references
                                <= s.children.iter().filter(|id| *id == child).count() as u32
                            && matches!(r.value, InitializedRequirement::Simple(_))
                    }) {
                        return Err(Error::DynamicReleaseRequired(*child));
                    }
                }
            }
        }
        let s = self
            .sets
            .iter_mut()
            .find(|s| s.label == id && s.alive)
            .ok_or(Error::MissingSet)?;
        s.references = s.references.checked_sub(1).ok_or(Error::MissingSet)?;
        if s.references == 0 {
            s.alive = false;
            let children = self.detach_all_children(id)?;
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
    /// Source-specific Simple factories may initialize over the same store.
    /// None retains the existing Const factory; this never imports a Test answer.
    fn initialize_simple(
        &mut self,
        _definition: &RequirementDefinition,
        _context: i32,
        _subject: Handle,
    ) -> Result<Option<crate::simple_requirement::Requirement>, Self::Error> {
        Ok(None)
    }
    fn log_missing_set(&mut self, id: &[u8]) -> Result<(), Self::Error>;
    fn log_created_requirement(&mut self, id: u32) -> Result<(), Self::Error>;
    fn log_created_set(&mut self, id: u32) -> Result<(), Self::Error>;
}
/// Generic initialization over the SAME store/cache/counter. Source-specific
/// dynamic factories register their actual listeners before returning a Simple
/// control; a Test answer alone is not an initialized requirement.
pub trait SubjectContext {
    type Error;
    fn subject_sets(&self) -> Result<&Runtime, Self::Error>;
    fn subject_sets_mut(&mut self) -> Result<&mut Runtime, Self::Error>;
    fn initialize_requirement(
        &mut self,
        label: u64,
        definition: &RequirementDefinition,
        context: i32,
        subject: Subject,
    ) -> Result<InitializedRequirement, Error<Self::Error>>;
    fn log_missing_set(&mut self, id: &[u8]) -> Result<(), Self::Error>;
    fn log_created_requirement(&mut self, id: u32) -> Result<(), Self::Error>;
    fn log_created_set(&mut self, id: u32) -> Result<(), Self::Error>;
}
struct ProposalContext<'a, C>(&'a mut C);
impl<C: Context> SubjectContext for ProposalContext<'_, C> {
    type Error = C::Error;
    fn subject_sets(&self) -> Result<&Runtime, Self::Error> {
        self.0.subject_sets()
    }
    fn subject_sets_mut(&mut self) -> Result<&mut Runtime, Self::Error> {
        self.0.subject_sets_mut()
    }
    fn initialize_requirement(
        &mut self,
        _: u64,
        d: &RequirementDefinition,
        context: i32,
        subject: Subject,
    ) -> Result<InitializedRequirement, Error<Self::Error>> {
        let Subject::Proposal(subject) = subject else {
            return Err(Error::UnsupportedSubject);
        };
        if let Some(value) = self
            .0
            .initialize_simple(d, context, subject)
            .map_err(Error::Context)?
        {
            return Ok(InitializedRequirement::Simple(value));
        }
        if let Predicate::Unsupported(kind) = d.predicate {
            return Err(Error::UnsupportedPredicate(kind));
        }
        let state = self
            .0
            .evaluate_constant(&d.predicate, context, subject)
            .map_err(Error::Context)?;
        Ok(InitializedRequirement::Const(if d.inverse {
            inverse(state)
        } else {
            state
        }))
    }
    fn log_missing_set(&mut self, id: &[u8]) -> Result<(), Self::Error> {
        self.0.log_missing_set(id)
    }
    fn log_created_requirement(&mut self, id: u32) -> Result<(), Self::Error> {
        self.0.log_created_requirement(id)
    }
    fn log_created_set(&mut self, id: u32) -> Result<(), Self::Error> {
        self.0.log_created_set(id)
    }
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
    get_or_create_subject(
        &mut ProposalContext(c),
        receiver,
        context,
        Subject::Proposal(subject),
        id,
    )
}
pub fn get_or_create_subject<C: SubjectContext>(
    c: &mut C,
    receiver: u64,
    context: i32,
    subject: Subject,
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
        let listener = SetListener::Subject(receiver);
        let token = s.changed.subscribe(listener).map_err(Error::Ring)?;
        s.subscribers.push((listener, token));
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
        let definition_data =
            c.subject_sets().map_err(Cx)?.requirement_definitions[requirement].clone();
        // Logical object identity exists during OnInitialize subscriptions;
        // the native numeric ID is assigned only AFTER initialization returns.
        let label = c.subject_sets_mut().map_err(Cx)?.label()?;
        let value = c.initialize_requirement(label, &definition_data, context, subject)?;
        let r = c.subject_sets_mut().map_err(Cx)?;
        r.requirements
            .try_reserve(1)
            .map_err(|_| Error::Allocation)?;
        let native_id = r.native_id();
        r.requirements.push(Requirement {
            label,
            native_id,
            context,
            subject,
            definition: requirement,
            value,
            references: 1,
            changed: ListenerRing::from_ordered(&[]).map_err(Error::Ring)?,
        });
        children.push(label);
        c.log_created_requirement(native_id).map_err(Cx)?;
    }
    let r = c.subject_sets_mut().map_err(Cx)?;
    r.sets.try_reserve(1).map_err(|_| Error::Allocation)?;
    let label = r.label()?;
    let mut child_subscriptions = Vec::new();
    child_subscriptions
        .try_reserve(children.len())
        .map_err(|_| Error::Allocation)?;
    for child in &children {
        let requirement = r
            .requirements
            .iter_mut()
            .find(|row| row.label == *child && row.references != 0)
            .ok_or(Error::MissingSet)?;
        child_subscriptions.push(requirement.changed.subscribe(label).map_err(Error::Ring)?);
    }
    r.sets.push(Set {
        label,
        native_id: u32::MAX,
        context,
        subject,
        definition,
        state: 1,
        children,
        subscribers: vec![],
        changed: ListenerRing::from_ordered(&[]).map_err(Error::Ring)?,
        child_subscriptions,
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
    let listener = SetListener::Subject(receiver);
    let token = s.changed.subscribe(listener).map_err(Error::Ring)?;
    s.subscribers.push((listener, token));
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
            .map(|r| r.value.state())
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
        self.runtime.detach_child(self.label, id)?;
        self.runtime.release_child(id)
    }
    fn remove_all_children(&mut self) -> Result<(), Self::Error> {
        let children = self.runtime.detach_all_children(self.label)?;
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
    #[test]
    fn dynamic_city_subject_uses_same_counter_cache_and_preserves_simple_state() {
        struct Dynamic {
            runtime: Runtime,
            initializations: Vec<(u64, Subject)>,
        }
        impl SubjectContext for Dynamic {
            type Error = &'static str;
            fn subject_sets(&self) -> Result<&Runtime, Self::Error> {
                Ok(&self.runtime)
            }
            fn subject_sets_mut(&mut self) -> Result<&mut Runtime, Self::Error> {
                Ok(&mut self.runtime)
            }
            fn initialize_requirement(
                &mut self,
                label: u64,
                _: &RequirementDefinition,
                _: i32,
                subject: Subject,
            ) -> Result<InitializedRequirement, Error<Self::Error>> {
                self.initializations.push((label, subject));
                // The real initializer owns its source callback registrations;
                // this test demonstrates retained Simple flags, not a native Test.
                Ok(InitializedRequirement::Simple(
                    crate::simple_requirement::Requirement {
                        active: true,
                        state: 1,
                        inverse: false,
                        persistent: false,
                        triggered: true,
                    },
                ))
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
        let mut c = Dynamic {
            runtime: Runtime::from_initial(
                18921,
                vec![(
                    b"S".to_vec(),
                    Some(SetDefinition {
                        id: b"S".to_vec(),
                        requirements: vec![b"R".to_vec()],
                    }),
                )],
                vec![RequirementDefinition {
                    id: b"R".to_vec(),
                    predicate: Predicate::Unsupported(0xc88bd7ea),
                    inverse: false,
                    persistent: false,
                }],
            )
            .unwrap(),
            initializations: vec![],
        };
        let a = get_or_create_subject(&mut c, 7, 172, Subject::ModifierObject(123), b"S")
            .unwrap()
            .unwrap();
        assert_eq!(c.runtime.state(a), Some(1));
        assert_eq!(c.runtime.next_native_id(), 18923);
        let child = c.runtime.children(a).unwrap()[0];
        assert!(c.runtime.simple_requirement(child).unwrap().triggered);
        assert_eq!(
            c.runtime.requirement_subject(child),
            Some(Subject::ModifierObject(123))
        );
        let same = get_or_create_subject(&mut c, 8, 172, Subject::ModifierObject(123), b"S")
            .unwrap()
            .unwrap();
        assert_eq!(a, same);
        assert_eq!(c.initializations.len(), 1);
        let other = get_or_create_subject(&mut c, 9, 173, Subject::ModifierObject(123), b"S")
            .unwrap()
            .unwrap();
        assert_ne!(a, other);
        assert_eq!(c.runtime.next_native_id(), 18925);
        c.runtime.unsubscribe::<()>(7, a).unwrap();
        c.runtime.release::<()>(a).unwrap();
        c.runtime.unsubscribe::<()>(8, a).unwrap();
        assert!(
            matches!(c.runtime.release::<()>(a),Err(Error::DynamicReleaseRequired(id))if id==child)
        );
        assert_eq!(c.runtime.state(a), Some(1));
    }
}
