//! Source-qualified DynamicModifier subject updates for TradeRelations.
//! Controls, tracked/applied membership and affected instances stay in the same
//! canonical diplomacy modifier owner used by action callbacks. Requirement
//! construction borrows the existing shared cache/counter and concrete factories.
use super::relationship_requirements as relationship;
use super::trade_relations_opinion::{self as leaf, Effect};
use crate::loaded_actions::declare_war::requirement_modifiers::{Modifiers, Subject};
use c6_core::unit_moved_signal::modifiers::Context as _;
use c6_core::{
    loaded_state::LoadedState,
    melee_attack::proposals::subject_sets as sets,
    unit_moved_signal::modifiers::{self, TrackedSubject},
};

#[derive(Debug)]
pub struct Metadata {
    pub label: u64,
    pub definition: Vec<u8>,
    pub context_instance: i32,
    pub subject_requirements: Vec<u8>,
    pub run_once: bool,
    pub has_activated: bool,
    pub effect: Effect,
    pub statement_flags: [bool; 2],
    /// Other concrete leaves are owned by the SAME canonical modifier owner.
    pub canonical_leaf: bool,
}
#[derive(Debug)]
pub struct Runtime {
    rows: Vec<Metadata>,
    known_major: Option<KnownMajor>,
}
/// Concrete collection/factory continuations. A factory must execute native
/// predicate initialization and subscriptions, rather than import an answer.
pub trait Services {
    fn collection(
        &mut self,
        state: &mut LoadedState,
        canonical: &mut Modifiers,
        receiver: u64,
    ) -> Result<Vec<u64>, &'static str>;
    fn create_subject_set(
        &mut self,
        state: &mut LoadedState,
        sets: &mut sets::Runtime,
        receiver: u64,
        context: i32,
        subject: u64,
        definition: &[u8],
    ) -> Result<Option<u64>, &'static str>;
    fn release_subject_set(
        &mut self,
        _state: &mut LoadedState,
        sets: &mut sets::Runtime,
        receiver: u64,
        set: u64,
    ) -> Result<(), &'static str> {
        sets.unsubscribe::<()>(receiver, set)
            .map_err(|_| "opinion set unsubscribe")?;
        sets.release::<()>(set)
            .map_err(|_| "opinion dynamic set destruction")
    }
}
impl Runtime {
    pub fn from_current(rows: Vec<Metadata>) -> Result<Self, &'static str> {
        if rows
            .iter()
            .enumerate()
            .any(|(i, r)| rows[..i].iter().any(|p| p.label == r.label))
        {
            return Err("duplicate opinion graph metadata");
        }
        Ok(Self {
            rows,
            known_major: None,
        })
    }
    pub fn bind_known_major(&mut self, profile: KnownMajor) -> Result<(), &'static str> {
        if self.known_major.is_some() {
            return Err("duplicate opinion collection profile");
        }
        self.known_major = Some(profile);
        Ok(())
    }
    pub fn known_major(&self) -> Option<&KnownMajor> {
        self.known_major.as_ref()
    }
    pub fn subject_changed(
        &mut self,
        state: &mut LoadedState,
        sets: &mut sets::Runtime,
        canonical: &mut Modifiers,
        receiver: u64,
        set: u64,
    ) -> Result<bool, &'static str> {
        let Some(row) = self.rows.iter_mut().find(|r| r.label == receiver) else {
            return Ok(false);
        };
        let subject = modifier(canonical, receiver)?
            .tracked
            .iter()
            .position(|s| s.control.requirement_set == Some(set))
            .ok_or("opinion subject callback set")?;
        modifiers::check_apply_effects(&mut Apply {
            state,
            sets,
            canonical,
            row,
            subject,
        })?;
        Ok(true)
    }
    pub fn owner_changed_current_with_relationships(
        &mut self,
        state: &mut LoadedState,
        sets: &mut sets::Runtime,
        canonical: &mut Modifiers,
        receiver: u64,
        definitions: &relationship::Definitions,
    ) -> Result<bool, &'static str> {
        let mut profile = self
            .known_major
            .take()
            .ok_or("opinion collection profile absent")?;
        let result = self.owner_changed(
            state,
            sets,
            canonical,
            receiver,
            &mut RelationshipServices {
                profile: &mut profile,
                definitions,
            },
        );
        self.known_major = Some(profile);
        result
    }
    pub fn destroy_set(&mut self, set: u64) -> Result<bool, &'static str> {
        let profile = self
            .known_major
            .as_mut()
            .ok_or("opinion collection profile absent")?;
        let Some(at) = profile.managed_sets.iter().position(|id| *id == set) else {
            return Ok(false);
        };
        profile.managed_sets.remove(at);
        Ok(true)
    }
    pub fn retire_requirement(&mut self, label: u64) -> Result<bool, &'static str> {
        let profile = self
            .known_major
            .as_mut()
            .ok_or("opinion collection profile absent")?;
        if !profile.subscriptions.iter().any(|r| r.0 == label) {
            return Ok(false);
        }
        profile.destroy(label)?;
        Ok(true)
    }
    pub fn signal_cursor_start(&mut self, signal: u32) -> Result<usize, &'static str> {
        self.known_major
            .as_mut()
            .ok_or("opinion collection profile absent")?
            .ring_mut(signal)?
            .cursor_start()
            .map_err(|_| "opinion signal cursor start")
    }
    pub fn signal_cursor_next(
        &mut self,
        signal: u32,
        current: usize,
    ) -> Result<Option<usize>, &'static str> {
        self.known_major
            .as_mut()
            .ok_or("opinion collection profile absent")?
            .ring_mut(signal)?
            .cursor_next(current)
            .map_err(|_| "opinion signal cursor next")
    }
    pub fn signal_cursor_route(
        &mut self,
        signal: u32,
        current: usize,
    ) -> Result<Option<SignalRegistration>, &'static str> {
        self.known_major
            .as_mut()
            .ok_or("opinion collection profile absent")?
            .ring_mut(signal)?
            .cursor_route(current)
            .map_err(|_| "opinion signal cursor route")
    }
    pub fn signal_cursor_release(
        &mut self,
        signal: u32,
        current: usize,
    ) -> Result<(), &'static str> {
        self.known_major
            .as_mut()
            .ok_or("opinion collection profile absent")?
            .ring_mut(signal)?
            .cursor_release(current)
            .map_err(|_| "opinion signal cursor release")
    }
    pub fn owner_changed_current(
        &mut self,
        state: &mut LoadedState,
        sets: &mut sets::Runtime,
        canonical: &mut Modifiers,
        receiver: u64,
    ) -> Result<bool, &'static str> {
        let mut profile = self
            .known_major
            .take()
            .ok_or("opinion collection profile absent")?;
        let result = self.owner_changed(state, sets, canonical, receiver, &mut profile);
        self.known_major = Some(profile);
        result
    }
    pub fn rows(&self) -> &[Metadata] {
        &self.rows
    }
    pub fn contains(&self, receiver: u64) -> bool {
        self.rows.iter().any(|r| r.label == receiver)
    }
    /// 2332b90 / 2333b50: Attach and AfterUpdate for this concrete family are
    /// inert. BeforeUpdate stores instance before collection or predicate work.
    pub fn owner_changed<S: Services>(
        &mut self,
        state: &mut LoadedState,
        sets: &mut sets::Runtime,
        canonical: &mut Modifiers,
        receiver: u64,
        services: &mut S,
    ) -> Result<bool, &'static str> {
        let Some(row) = self.rows.iter_mut().find(|r| r.label == receiver) else {
            return Ok(false);
        };
        let m = canonical
            .modifier_mut(receiver)
            .ok_or("canonical opinion modifier")?;
        if m.control.suppressed {
            return Ok(true);
        }
        let met = match m.control.owner_requirement_set {
            None => true,
            Some(set) => matches!(
                sets.state(set).ok_or("opinion owner requirement set")?,
                0 | 2 | 4
            ),
        };
        if !met {
            // Deactivate23332b0 removes effects but retains tracked subjects.
            if m.control.enabled {
                m.control.enabled = false;
                let count = m.tracked.len();
                for subject in 0..count {
                    Apply {
                        state,
                        sets,
                        canonical,
                        row,
                        subject,
                    }
                    .remove_effects()?;
                }
            }
            return Ok(true);
        }
        let activating = !m.control.enabled;
        // RunOnce first-activation ignored-subject initialization is a distinct
        // source branch and must not silently become an ordinary collection.
        if activating && !row.has_activated && row.run_once {
            return Err("opinion RunOnce ignored-subject initialization");
        }
        m.control.enabled = true;
        row.effect.current_update_instance = m.instance;
        let mut collection = services.collection(state, canonical, receiver)?;
        for &object in &collection {
            key(canonical, object)?;
        }
        collection.sort_by_key(|o| key(canonical, *o).unwrap());
        // 2333df0 visits the tracked tree in comparator order. Removed and
        // permanent-applied nodes survive collection absence.
        let mut subject = 0;
        while subject < modifier(canonical, receiver)?.tracked.len() {
            let m = modifier(canonical, receiver)?;
            let tracked = &m.tracked[subject];
            let keep = tracked.control.removed
                || (m.permanent && tracked.control.applied != 0)
                || collection
                    .iter()
                    .any(|o| key(canonical, *o).ok() == key(canonical, tracked.object).ok());
            if keep {
                subject += 1;
                continue;
            }
            Apply {
                state,
                sets,
                canonical,
                row,
                subject,
            }
            .remove_effects()?;
            if let Some(set) = modifier(canonical, receiver)?.tracked[subject]
                .control
                .requirement_set
            {
                services.release_subject_set(state, sets, receiver, set)?;
            }
            canonical
                .modifier_mut(receiver)
                .unwrap()
                .tracked
                .remove(subject);
        }
        for object in collection {
            let object_key = key(canonical, object)?;
            let subject = modifier(canonical, receiver)?
                .tracked
                .partition_point(|s| key(canonical, s.object).unwrap() < object_key);
            if modifier(canonical, receiver)?
                .tracked
                .get(subject)
                .is_none_or(|s| key(canonical, s.object).ok() != Some(object_key))
            {
                canonical.modifier_mut(receiver).unwrap().tracked.insert(
                    subject,
                    Subject {
                        object,
                        control: TrackedSubject {
                            requirement_set: None,
                            applied: 0,
                            removed: false,
                        },
                    },
                );
            }
            let tracked = &modifier(canonical, receiver)?.tracked[subject].control;
            if !tracked.removed
                && tracked.requirement_set.is_none()
                && !row.subject_requirements.is_empty()
            {
                let set = services.create_subject_set(
                    state,
                    sets,
                    receiver,
                    row.context_instance,
                    object,
                    &row.subject_requirements,
                )?;
                let tracked =
                    &mut canonical.modifier_mut(receiver).unwrap().tracked[subject].control;
                tracked.requirement_set = set;
                if set.is_none() {
                    tracked.removed = true;
                }
            }
            modifiers::check_apply_effects(&mut Apply {
                state,
                sets,
                canonical,
                row,
                subject,
            })?;
        }
        if activating {
            row.has_activated = true;
        }
        Ok(true)
    }
}
fn modifier(
    canonical: &Modifiers,
    label: u64,
) -> Result<&crate::loaded_actions::declare_war::requirement_modifiers::Modifier, &'static str> {
    canonical
        .modifiers()
        .iter()
        .find(|m| m.label == label)
        .ok_or("canonical opinion modifier")
}
fn key(canonical: &Modifiers, label: u64) -> Result<(i32, i32, i32), &'static str> {
    canonical
        .objects()
        .iter()
        .find(|o| o.label == label)
        .map(|o| (o.owner, o.first, o.second))
        .ok_or("canonical opinion Player object")
}
struct Apply<'a> {
    state: &'a mut LoadedState,
    sets: &'a mut sets::Runtime,
    canonical: &'a mut Modifiers,
    row: &'a mut Metadata,
    subject: usize,
}
impl modifiers::Context for Apply<'_> {
    type Error = &'static str;
    fn modifier(&self) -> Result<&modifiers::Modifier, Self::Error> {
        Ok(&modifier(self.canonical, self.row.label)?.control)
    }
    fn tracked(&self) -> Result<&TrackedSubject, Self::Error> {
        Ok(&modifier(self.canonical, self.row.label)?.tracked[self.subject].control)
    }
    fn tracked_mut(&mut self) -> Result<&mut TrackedSubject, Self::Error> {
        Ok(&mut self
            .canonical
            .modifier_mut(self.row.label)
            .ok_or("opinion modifier")?
            .tracked[self.subject]
            .control)
    }
    fn requirement_set_state(&self, id: u64) -> Result<i32, Self::Error> {
        self.sets.state(id).ok_or("opinion subject set")
    }
    fn ensure_applied_subject(&mut self) -> Result<(), Self::Error> {
        let object = modifier(self.canonical, self.row.label)?.tracked[self.subject].object;
        let k = key(self.canonical, object)?;
        let at = modifier(self.canonical, self.row.label)?
            .applied
            .partition_point(|p| key(self.canonical, *p).unwrap() < k);
        if modifier(self.canonical, self.row.label)?
            .applied
            .get(at)
            .is_none_or(|p| key(self.canonical, *p).ok() != Some(k))
        {
            self.canonical
                .modifier_mut(self.row.label)
                .unwrap()
                .applied
                .insert(at, object);
        }
        Ok(())
    }
    fn is_repeatable(&self) -> Result<bool, Self::Error> {
        Ok(modifier(self.canonical, self.row.label)?.repeatable)
    }
    fn apply_effect(&mut self) -> Result<bool, Self::Error> {
        if self.row.canonical_leaf {
            let owner = modifier(self.canonical, self.row.label)?.owner;
            if self.row.statement_flags != [false, false]
                && self
                    .state
                    .player_slot_status(owner)
                    .map_err(|_| "opinion statement owner controls")?
                    != 3
            {
                return Err("opinion nonhuman statement concrete guard");
            }
            self.canonical
                .apply_tracked_effect(self.state, self.sets, self.row.label, self.subject)
                .map_err(|e| match e {
                    crate::loaded_actions::declare_war::requirement_modifiers::Error::Missing(
                        reason,
                    ) => reason,
                    crate::loaded_actions::declare_war::requirement_modifiers::Error::State(_) => {
                        "opinion canonical effect state"
                    }
                })?;
            return Ok(true);
        }
        modifiers::apply_effect(self).map_err(|e| match e {
            modifiers::EffectError::Context(reason) => reason,
            modifiers::EffectError::DefinitionDisappeared => {
                "opinion effect definition disappeared"
            }
        })?;
        Ok(true)
    }
    fn unsubscribe_subject_requirement(&mut self) -> Result<(), Self::Error> {
        // The shared reference is released by clear_subject_requirement below.
        self.sets
            .unsubscribe::<()>(
                self.row.label,
                self.tracked()?
                    .requirement_set
                    .ok_or("opinion terminal set")?,
            )
            .map_err(|_| "opinion terminal set unsubscribe")
    }
    fn clear_subject_requirement(&mut self) -> Result<(), Self::Error> {
        let set = self
            .tracked_mut()?
            .requirement_set
            .take()
            .ok_or("opinion terminal set")?;
        self.sets
            .release::<()>(set)
            .map_err(|_| "opinion terminal set release")
    }
    fn remove_effects(&mut self) -> Result<(), Self::Error> {
        let m = modifier(self.canonical, self.row.label)?;
        if m.control.suppressed || m.permanent || m.tracked[self.subject].control.applied == 0 {
            return Ok(());
        }
        let object = m.tracked[self.subject].object;
        let (owner, instance, allow_dead, limit) =
            (m.owner, m.instance, m.allow_dead, m.stack_limit);
        self.tracked_mut()?.applied = 0;
        let k = key(self.canonical, object)?;
        let at = modifier(self.canonical, self.row.label)?
            .applied
            .iter()
            .position(|o| key(self.canonical, *o).ok() == Some(k));
        if let Some(at) = at {
            self.canonical
                .modifier_mut(self.row.label)
                .unwrap()
                .applied
                .remove(at);
        }
        self.canonical
            .object_mut(object)
            .ok_or("opinion affected object")?
            .affected
            .clear_instance(instance);
        if limit != 0 {
            return Err("opinion removal stack limit");
        }
        if !self
            .state
            .player_alive(k.0)
            .map_err(|_| "opinion subject alive")?
            && !allow_dead
        {
            return Ok(());
        }
        leaf::remove(self.state, owner, k.0, instance)
    }
}
impl modifiers::EffectContext for Apply<'_> {
    type Error = &'static str;
    fn mark_subject_affected(&mut self) -> Result<(), Self::Error> {
        let m = modifier(self.canonical, self.row.label)?;
        let object = m.tracked[self.subject].object;
        let instance = m.instance;
        self.canonical
            .object_mut(object)
            .ok_or("opinion affected object")?
            .affected
            .mark(&self.row.definition, instance)
            .map_err(|_| "opinion affected order")
    }
    fn suppressed(&self) -> Result<bool, Self::Error> {
        Ok(modifier(self.canonical, self.row.label)?.control.suppressed)
    }
    fn subject_stack_limit(&self) -> Result<Option<u16>, Self::Error> {
        Ok(Some(modifier(self.canonical, self.row.label)?.stack_limit))
    }
    fn affected_count(&self) -> Result<i32, Self::Error> {
        let object = modifier(self.canonical, self.row.label)?.tracked[self.subject].object;
        Ok(self
            .canonical
            .objects()
            .iter()
            .find(|o| o.label == object)
            .ok_or("opinion affected object")?
            .affected
            .count(&self.row.definition))
    }
    fn log_applying(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn log_failed(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn apply_concrete_effect(&mut self) -> Result<bool, Self::Error> {
        let m = modifier(self.canonical, self.row.label)?;
        let subject = key(self.canonical, m.tracked[self.subject].object)?.0;
        if !self
            .state
            .player_alive(subject)
            .map_err(|_| "opinion subject alive")?
            && !m.allow_dead
        {
            return Ok(true);
        }
        leaf::apply(
            self.state,
            &mut self.row.effect,
            m.owner,
            subject,
            m.instance,
            self.row.statement_flags,
        )?;
        Ok(true)
    }
}

/// Current manager MajorPlayers collection and supported KnownMajor factories.
/// Entries are logical references into the canonical owner, not object copies.
#[derive(Debug)]
pub struct KnownMajor {
    pub players: Vec<(i32, u64)>,
    pub contexts: Vec<(i32, i32)>,
    pub relationship_factories: Vec<(Vec<u8>, relationship::Kind)>,
    signals: Vec<(
        u32,
        c6_core::unit_moved_signal::ListenerRing<SignalRegistration>,
    )>,
    subscriptions: Vec<(u64, u32, c6_core::unit_moved_signal::Subscription)>,
    managed_sets: Vec<u64>,
    pub meeting_receivers:
        Vec<crate::loaded_actions::stock_notifications::first_contact::requirements::Receiver>,
}
#[derive(Clone, Copy, Debug)]
pub enum SignalReceiver {
    Relationship(relationship::Receiver),
    Meeting(crate::loaded_actions::stock_notifications::first_contact::requirements::Receiver),
}
#[derive(Clone, Copy, Debug)]
pub struct SignalRegistration {
    pub label: u64,
    pub receiver: SignalReceiver,
}
impl KnownMajor {
    pub fn new(players: Vec<(i32, u64)>, contexts: Vec<(i32, i32)>) -> Self {
        Self {
            players,
            contexts,
            meeting_receivers: Vec::new(),
            relationship_factories: Vec::new(),
            signals: Vec::new(),
            subscriptions: Vec::new(),
            managed_sets: Vec::new(),
        }
    }
    pub fn bind_current_sets(&mut self, labels: impl IntoIterator<Item = u64>) {
        for label in labels {
            if !self.managed_sets.contains(&label) {
                self.managed_sets.push(label)
            }
        }
    }
    pub fn signal_registrations(&self) -> Vec<(u32, Vec<SignalRegistration>)> {
        let mut result = self
            .signals
            .iter()
            .map(|(signal, ring)| (*signal, ring.ordered_subscribers().collect()))
            .collect::<Vec<_>>();
        result.sort_by_key(|r| r.0);
        result
    }
    fn ring_mut(
        &mut self,
        signal: u32,
    ) -> Result<&mut c6_core::unit_moved_signal::ListenerRing<SignalRegistration>, &'static str>
    {
        if !self.signals.iter().any(|r| r.0 == signal) {
            self.signals.push((
                signal,
                c6_core::unit_moved_signal::ListenerRing::from_ordered(&[])
                    .map_err(|_| "opinion signal allocation")?,
            ));
        }
        Ok(&mut self.signals.iter_mut().find(|r| r.0 == signal).unwrap().1)
    }
    fn subscribe(
        &mut self,
        label: u64,
        signal: u32,
        receiver: SignalReceiver,
    ) -> Result<(), &'static str> {
        let token = self
            .ring_mut(signal)?
            .subscribe(SignalRegistration { label, receiver })
            .map_err(|_| "opinion signal subscribe")?;
        self.subscriptions.push((label, signal, token));
        Ok(())
    }
    fn destroy(&mut self, label: u64) -> Result<(), &'static str> {
        while let Some(index) = self.subscriptions.iter().position(|r| r.0 == label) {
            let (_, signal, token) = self.subscriptions.remove(index);
            self.ring_mut(signal)?
                .unsubscribe(token)
                .map_err(|_| "opinion signal unsubscribe")?;
        }
        self.meeting_receivers.retain(|r| r.label != label);
        Ok(())
    }
}
struct RelationshipServices<'a> {
    profile: &'a mut KnownMajor,
    definitions: &'a relationship::Definitions,
}
impl Services for RelationshipServices<'_> {
    fn collection(
        &mut self,
        state: &mut LoadedState,
        canonical: &mut Modifiers,
        receiver: u64,
    ) -> Result<Vec<u64>, &'static str> {
        self.profile.collection(state, canonical, receiver)
    }
    fn create_subject_set(
        &mut self,
        state: &mut LoadedState,
        sets: &mut sets::Runtime,
        receiver: u64,
        context: i32,
        subject: u64,
        definition: &[u8],
    ) -> Result<Option<u64>, &'static str> {
        let result = sets::get_or_create_subject(
            &mut KnownMajorFactory {
                state,
                sets,
                profile: self.profile,
                definitions: Some(self.definitions),
            },
            receiver,
            context,
            sets::Subject::ModifierObject(subject),
            definition,
        )
        .map_err(initialization_error)?;
        if let Some(id) = result {
            if !self.profile.managed_sets.contains(&id) {
                self.profile.managed_sets.push(id);
            }
        }
        Ok(result)
    }
    fn release_subject_set(
        &mut self,
        state: &mut LoadedState,
        sets: &mut sets::Runtime,
        receiver: u64,
        set: u64,
    ) -> Result<(), &'static str> {
        self.profile.release_subject_set(state, sets, receiver, set)
    }
}
impl Services for KnownMajor {
    fn release_subject_set(
        &mut self,
        state: &mut LoadedState,
        sets: &mut sets::Runtime,
        receiver: u64,
        set: u64,
    ) -> Result<(), &'static str> {
        sets.unsubscribe::<()>(receiver, set)
            .map_err(|_| "opinion set unsubscribe")?;
        sets::changes::release(
            &mut KnownMajorFactory {
                state,
                sets,
                profile: self,
                definitions: None,
            },
            set,
        )
        .map_err(|_| "opinion KnownMajor final release")
    }
    fn collection(
        &mut self,
        state: &mut LoadedState,
        canonical: &mut Modifiers,
        _: u64,
    ) -> Result<Vec<u64>, &'static str> {
        let mut result = Vec::new();
        for &(owner, object) in &self.players {
            if state
                .player_alive(owner)
                .map_err(|_| "opinion MajorPlayers alive")?
            {
                if key(canonical, object)?.0 != owner {
                    return Err("opinion MajorPlayers object identity");
                }
                result.push(object);
            }
        }
        Ok(result)
    }
    fn create_subject_set(
        &mut self,
        state: &mut LoadedState,
        sets: &mut sets::Runtime,
        receiver: u64,
        context: i32,
        subject: u64,
        definition: &[u8],
    ) -> Result<Option<u64>, &'static str> {
        if definition != b"PLAYER_IS_KNOWN_MAJOR_CIV" {
            return Err("unsupported opinion subject definition");
        }
        let result = sets::get_or_create_subject(
            &mut KnownMajorFactory {
                state,
                sets,
                profile: self,
                definitions: None,
            },
            receiver,
            context,
            sets::Subject::ModifierObject(subject),
            definition,
        )
        .map_err(initialization_error)?;
        if let Some(id) = result {
            if !self.managed_sets.contains(&id) {
                self.managed_sets.push(id);
            }
        }
        Ok(result)
    }
}
struct KnownMajorFactory<'a> {
    state: &'a mut LoadedState,
    sets: &'a mut sets::Runtime,
    profile: &'a mut KnownMajor,
    definitions: Option<&'a relationship::Definitions>,
}
impl sets::SubjectContext for KnownMajorFactory<'_> {
    type Error = &'static str;
    fn subject_sets(&self) -> Result<&sets::Runtime, Self::Error> {
        Ok(self.sets)
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, Self::Error> {
        Ok(self.sets)
    }
    fn initialize_requirement(
        &mut self,
        label: u64,
        definition: &sets::RequirementDefinition,
        context: i32,
        subject: sets::Subject,
    ) -> Result<sets::InitializedRequirement, sets::Error<Self::Error>> {
        use crate::loaded_actions::stock_notifications::first_contact::requirements::{
            self as meeting, Kind, Receiver,
        };
        let sets::Subject::ModifierObject(object) = subject else {
            return Err(sets::Error::UnsupportedSubject);
        };
        let owner = self
            .profile
            .contexts
            .iter()
            .find(|p| p.0 == context)
            .map(|p| p.1)
            .ok_or(sets::Error::Context("opinion context Player"))?;
        let other = self
            .profile
            .players
            .iter()
            .find(|p| p.1 == object)
            .map(|p| p.0)
            .ok_or(sets::Error::Context("opinion subject Player"))?;
        let value = if definition.id == b"REQUIRES_MAJOR_CIV_OPPONENT" {
            // CivilizationLevel2377c60 compares the configured SubjectType;
            // this profile's collection consists solely of actual full civs.
            2
        } else if definition.id == b"REQUIRES_PLAYERS_HAVE_MET" {
            let receiver = Receiver {
                label,
                kind: Kind::HasMet,
                context: Some(owner),
                subject: Some(other),
                minimum_turns: 0,
            };
            // OnInitialize23835a0 subscribes BEFORE the first virtual Test.
            self.profile.meeting_receivers.push(receiver);
            self.profile
                .subscribe(label, 0xc78, SignalReceiver::Meeting(receiver))
                .map_err(sets::Error::Context)?;
            meeting::test::<()>(self.state, receiver)
                .map_err(|_| sets::Error::Context("opinion HasMet Test"))?
        } else if let Some(kind) = self
            .profile
            .relationship_factories
            .iter()
            .find(|r| r.0 == definition.id)
            .map(|r| r.1)
        {
            let descriptor = relationship::Receiver {
                kind,
                context: owner,
                subject: other,
            };
            let signal_receiver = SignalReceiver::Relationship(descriptor);
            match kind {
                relationship::Kind::MetXTurns { turns } => {
                    let receiver = Receiver {
                        label,
                        kind: Kind::MetXTurnsAgo,
                        context: Some(owner),
                        subject: Some(other),
                        minimum_turns: turns,
                    };
                    self.profile.meeting_receivers.push(receiver);
                    self.profile
                        .subscribe(label, 0xc78, SignalReceiver::Meeting(receiver))
                        .map_err(sets::Error::Context)?;
                    self.profile
                        .subscribe(label, 0x990, signal_receiver)
                        .map_err(sets::Error::Context)?;
                }
                relationship::Kind::AtPeace { turns } => {
                    for signal in [0xcc0, 0xcd8, 0xc78] {
                        self.profile
                            .subscribe(label, signal, signal_receiver)
                            .map_err(sets::Error::Context)?;
                    }
                    if turns > 0 {
                        self.profile
                            .subscribe(label, 0x990, signal_receiver)
                            .map_err(sets::Error::Context)?;
                    }
                }
                relationship::Kind::OpenBorders { grace_turns } => {
                    for signal in [0xdc8, 0xde0] {
                        self.profile
                            .subscribe(label, signal, signal_receiver)
                            .map_err(sets::Error::Context)?;
                    }
                    if grace_turns > 0 {
                        self.profile
                            .subscribe(label, 0x990, signal_receiver)
                            .map_err(sets::Error::Context)?;
                    }
                }
                relationship::Kind::NearCultureBorder { .. } => {
                    self.profile
                        .subscribe(label, 0x990, signal_receiver)
                        .map_err(sets::Error::Context)?;
                }
                _ => {
                    return Err(sets::Error::Context(
                        "unsupported opinion relationship factory",
                    ))
                }
            }
            let defs = self.definitions.ok_or(sets::Error::Context(
                "opinion current relationship definitions",
            ))?;
            let alive_majors = self.profile.players.iter().map(|p| p.0).collect::<Vec<_>>();
            relationship::test(self.state, defs, &alive_majors, descriptor)
                .map_err(|_| sets::Error::Context("opinion initial relationship Test"))?
        } else {
            return Err(sets::Error::Context("unsupported opinion subject child"));
        };
        Ok(sets::InitializedRequirement::Simple(
            c6_core::simple_requirement::Requirement {
                active: true,
                state: if definition.inverse {
                    sets::inverse(value)
                } else {
                    value
                },
                inverse: definition.inverse,
                persistent: definition.persistent,
                triggered: false,
            },
        ))
    }
    fn destroy_initial_simple_requirement(
        &mut self,
        label: u64,
    ) -> Result<(), sets::Error<Self::Error>> {
        // CivilizationLevel has no signal registration. HasMet destructor
        // 2383650 unsubscribes its actual meeting handle.
        self.profile.destroy(label).map_err(sets::Error::Context)
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

impl sets::changes::Context for KnownMajorFactory<'_> {
    type Error = &'static str;
    fn subject_sets(&self) -> Result<&sets::Runtime, Self::Error> {
        Ok(self.sets)
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, Self::Error> {
        Ok(self.sets)
    }
    fn test_requirement(&mut self, _: u64) -> Result<i32, sets::Error<Self::Error>> {
        Err(sets::Error::Context("unexpected KnownMajor release Test"))
    }
    fn modifier_changed(
        &mut self,
        _: sets::SetListener,
        _: u64,
        _: i32,
        _: i32,
    ) -> Result<(), sets::Error<Self::Error>> {
        Err(sets::Error::Context(
            "unexpected KnownMajor release callback",
        ))
    }
    fn destroy_requirement(&mut self, label: u64) -> Result<(), sets::Error<Self::Error>> {
        let definition = self
            .sets
            .requirement_definition(label)
            .ok_or(sets::Error::MissingRequirementDefinition)?;
        if definition.id != b"REQUIRES_MAJOR_CIV_OPPONENT"
            && definition.id != b"REQUIRES_PLAYERS_HAVE_MET"
            && !self
                .profile
                .relationship_factories
                .iter()
                .any(|r| r.0 == definition.id)
        {
            return Err(sets::Error::Context(
                "unsupported opinion requirement destruction",
            ));
        }
        self.profile.destroy(label).map_err(sets::Error::Context)
    }
    fn destroy_set(&mut self, set: u64) -> Result<(), sets::Error<Self::Error>> {
        let at = self
            .profile
            .managed_sets
            .iter()
            .position(|id| *id == set)
            .ok_or(sets::Error::Context("unknown opinion base set destruction"))?;
        self.profile.managed_sets.remove(at);
        Ok(())
    }
    fn reset_trigger(&mut self, set: u64) -> Result<(), sets::Error<Self::Error>> {
        sets::changes::reset_trigger_state(self.sets, set)
    }
}

fn initialization_error(error: sets::Error<&'static str>) -> &'static str {
    match error {
        sets::Error::Context(reason) | sets::Error::CurrentInput(reason) => reason,
        sets::Error::MissingDefinitionInput => {
            "opinion subject set definition absent from immutable catalog"
        }
        sets::Error::MissingRequirementDefinition => {
            "opinion subject child definition absent from immutable catalog"
        }
        sets::Error::MissingSet => "opinion subject initialization missing live set",
        sets::Error::MissingSubscription => "opinion subject initialization subscription missing",
        sets::Error::DynamicReleaseRequired(_) => {
            "opinion initial child destruction requires concrete callback"
        }
        sets::Error::UnsupportedPredicate(_) => "opinion unsupported subject predicate factory",
        sets::Error::UnsupportedSubject => "opinion unsupported subject object",
        sets::Error::Allocation => "opinion subject initialization allocation",
        sets::Error::CounterExhausted => "opinion subject logical identity exhausted",
        sets::Error::Ring(_) => "opinion subject initialization listener ring",
    }
}
