//! Canonical current Player diplomacy modifiers reached by declaration callbacks.
//! This owner holds membership/affected state once; requirement values remain in
//! the action's shared backend Runtime and blackboard trackers remain in Player.
use crate::loaded_actions::stock_notifications::first_contact::opinion_callbacks::CurrentYields;
use crate::{
    loaded_actions::government_actions::requirements::Continuations,
    melee_attack::backend::requirements::Runtime,
};
use c6_core::{
    loaded_state::{ActionStamp, LoadedState},
    melee_attack::proposals::subject_sets as sets,
    unit_moved_signal::modifiers::{self, AffectedModifiers},
};
#[derive(Debug)]
pub enum Error {
    Missing(&'static str),
    State(c6_core::loaded_state::Error),
}
#[derive(Debug)]
pub struct Object {
    pub label: u64,
    pub owner: i32,
    pub first: i32,
    pub second: i32,
    pub affected: AffectedModifiers,
}
#[derive(Debug)]
pub struct Subject {
    pub object: u64,
    pub control: modifiers::TrackedSubject,
}
#[derive(Debug)]
pub struct Modifier {
    pub label: u64,
    pub instance: i32,
    pub owner: i32,
    pub control: modifiers::Modifier,
    pub permanent: bool,
    pub repeatable: bool,
    pub stack_limit: u16,
    pub allow_dead: bool,
    pub tracked: Vec<Subject>,
    pub applied: Vec<u64>,
}
/// Current concrete DiplomacySimpleEffect parameters for a first activation.
/// Values are integer effect parameters; the tracker stores Q8 values.
#[derive(Clone, Debug)]
pub struct Activation {
    pub label: u64,
    pub definition: Vec<u8>,
    pub initial: i32,
    pub maximum: i32,
    pub increment: i32,
    pub increment_turns: i32,
    pub cooldown_turns: i32,
    pub reduction: i32,
    pub name: Vec<u8>,
    pub visibility: i32,
    /// Native DiplomacyBaseEffect+48; the fresh extra-value branch activates
    /// and immediately deactivates its concrete tracker.
    pub remove_immediately: bool,
}
pub struct Modifiers {
    stamp: ActionStamp,
    human_statement_owners: Vec<i32>,
    activations: Vec<Activation>,
    random_effects: Vec<(
        u64,
        crate::loaded_actions::stock_notifications::first_contact::random_modifier::Parameters,
    )>,
    government_effects: Option<(
        Vec<u64>,
        crate::loaded_actions::stock_notifications::first_contact::government_modifier::Definitions,
    )>,
    retirable_has_met: Vec<u64>,
    retired_has_met: Vec<u64>,
    opinion_alive_majors: Option<Vec<i32>>,
    opinion_effects: Vec<(
        u64,
        crate::loaded_actions::stock_notifications::first_contact::opinion_callbacks::Effect,
    )>,
    modifiers: Vec<Modifier>,
    objects: Vec<Object>,
    opinion_graph: Option<crate::game_update_state::activation::opinion_subject_graph::Runtime>,
}
impl Modifiers {
    pub fn from_current(
        state: &LoadedState,
        modifiers: Vec<Modifier>,
        objects: Vec<Object>,
    ) -> Result<Self, Error> {
        if !state.is_pristine() {
            return Err(Error::Missing("initial diplomacy modifier after mutation"));
        }
        if modifiers
            .iter()
            .enumerate()
            .any(|(i, m)| modifiers[..i].iter().any(|p| p.label == m.label))
            || objects
                .iter()
                .enumerate()
                .any(|(i, o)| objects[..i].iter().any(|p| p.label == o.label))
        {
            return Err(Error::Missing("duplicate diplomacy modifier/object"));
        }
        let result = Self {
            stamp: state.action_stamp(),
            human_statement_owners: Vec::new(),
            activations: Vec::new(),
            random_effects: Vec::new(),
            government_effects: None,
            opinion_effects: Vec::new(),
            opinion_alive_majors: None,
            retirable_has_met: Vec::new(),
            retired_has_met: Vec::new(),
            modifiers,
            objects,
            opinion_graph: None,
        };
        for m in &result.modifiers {
            for s in &m.tracked {
                result.object(s.object)?;
            }
            for s in &m.applied {
                result.object(*s)?;
            }
            if m.applied
                .windows(2)
                .any(|w| result.key(w[0]).unwrap() > result.key(w[1]).unwrap())
            {
                return Err(Error::Missing("diplomacy applied comparator order"));
            }
        }
        Ok(result)
    }
    /// Owners whose actual effect statement flags require the current human
    /// CanSendStatement guard. Recheck controls on every concrete activation.
    pub fn with_human_statement_owners(mut self, owners: Vec<i32>) -> Result<Self, Error> {
        if owners.iter().enumerate().any(|(at, owner)| {
            owners[..at].contains(owner) || !self.modifiers.iter().any(|m| m.owner == *owner)
        }) {
            return Err(Error::Missing("human statement owner identity"));
        }
        self.human_statement_owners = owners;
        Ok(self)
    }
    /// Bind parameters for source-qualified simple effects; other positive
    /// effect families still fail explicitly when reached.
    pub fn with_activations(mut self, activations: Vec<Activation>) -> Result<Self, Error> {
        for (at, activation) in activations.iter().enumerate() {
            if activations[..at]
                .iter()
                .any(|a| a.label == activation.label)
                || !self.modifiers.iter().any(|m| m.label == activation.label)
            {
                return Err(Error::Missing("diplomacy activation receiver"));
            }
        }
        self.activations = activations;
        Ok(self)
    }
    /// Bind concrete RandomEffect parameters to this canonical modifier owner.
    /// Their Activation must describe the base effect before SetModifierValue:
    /// explicit RandomEffect removal is distinct from BaseEffect+48.
    pub fn with_random_effects(
        mut self,
        effects: Vec<(
            u64,
            crate::loaded_actions::stock_notifications::first_contact::random_modifier::Parameters,
        )>,
    ) -> Result<Self, Error> {
        for (at, (label, _)) in effects.iter().enumerate() {
            if effects[..at].iter().any(|(previous, _)| previous == label)
                || !self
                    .activations
                    .iter()
                    .any(|a| a.label == *label && !a.remove_immediately)
            {
                return Err(Error::Missing("random diplomacy activation receiver"));
            }
        }
        self.random_effects = effects;
        Ok(self)
    }
    /// Bind the concrete government effect to live Culture and bound rules.
    pub fn with_government_effects(
        mut self,
        labels: Vec<u64>,
        definitions: crate::loaded_actions::stock_notifications::first_contact::government_modifier::Definitions,
    ) -> Result<Self, Error> {
        for (at, label) in labels.iter().enumerate() {
            if labels[..at].contains(label)
                || !self
                    .activations
                    .iter()
                    .any(|a| a.label == *label && !a.remove_immediately)
                || self
                    .random_effects
                    .iter()
                    .any(|(random, _)| random == label)
            {
                return Err(Error::Missing("government diplomacy activation receiver"));
            }
        }
        self.government_effects = Some((labels, definitions));
        Ok(self)
    }
    /// Qualify actual PlayerHasMet external signal receivers imported at BEFORE.
    /// Their external c78 subscriptions are retired only by the exact terminal
    /// nonpersistent requirement release below, never merely by label matching.
    pub fn with_retirable_has_met(mut self, labels: Vec<u64>) -> Result<Self, Error> {
        if !self.retirable_has_met.is_empty()
            || labels
                .iter()
                .enumerate()
                .any(|(at, label)| labels[..at].contains(label))
        {
            return Err(Error::Missing("duplicate HasMet retirement receiver"));
        }
        self.retirable_has_met = labels;
        Ok(self)
    }
    pub fn retired_has_met(&self) -> &[u64] {
        &self.retired_has_met
    }
    /// PlayerHasMet::~ first drops the external subscription; the shared
    /// release kernel then deactivates the retained zero-reference simple row.
    pub fn retire_has_met(
        &mut self,
        requirements: &sets::Runtime,
        label: u64,
    ) -> Result<(), Error> {
        let qualified = self.retirable_has_met.contains(&label)
            && !self.retired_has_met.contains(&label)
            && requirements
                .requirement_definition(label)
                .is_some_and(|definition| definition.id == b"REQUIRES_PLAYERS_HAVE_MET")
            && requirements
                .simple_requirement_for_destruction(label)
                .is_some_and(|requirement| {
                    requirement.active
                        && requirement.state == 2
                        && !requirement.inverse
                        && !requirement.persistent
                        && !requirement.triggered
                });
        if !qualified {
            return Err(Error::Missing("diplomacy requirement destruction"));
        }
        self.retired_has_met.push(label);
        Ok(())
    }
    /// Bind the actual current native Manager::GetAliveMajors vector, in order.
    /// This bounded callback profile rejects a member which ceases to be alive.
    pub fn with_opinion_alive_majors(mut self, owners: Vec<i32>) -> Result<Self, Error> {
        if self.opinion_alive_majors.is_some()
            || owners
                .iter()
                .enumerate()
                .any(|(at, owner)| !(0..64).contains(owner) || owners[..at].contains(owner))
        {
            return Err(Error::Missing("opinion current alive-major roster"));
        }
        self.opinion_alive_majors = Some(owners);
        Ok(self)
    }
    /// Bind reached concrete opinion leaves to the same physical controls.
    pub fn with_opinion_effects(
        mut self,
        effects: Vec<(
            u64,
            crate::loaded_actions::stock_notifications::first_contact::opinion_callbacks::Effect,
        )>,
    ) -> Result<Self, Error> {
        for (at, (label, _)) in effects.iter().enumerate() {
            if effects[..at].iter().any(|(previous, _)| previous == label)
                || !self.activations.iter().any(|a| a.label == *label)
                || self.random_effects.iter().any(|(other, _)| other == label)
                || self
                    .government_effects
                    .as_ref()
                    .is_some_and(|(labels, _)| labels.contains(label))
            {
                return Err(Error::Missing("opinion diplomacy activation receiver"));
            }
        }
        self.opinion_effects = effects;
        Ok(self)
    }
    fn object(&self, label: u64) -> Result<&Object, Error> {
        self.objects
            .iter()
            .find(|o| o.label == label)
            .ok_or(Error::Missing("current diplomacy modifier Player object"))
    }
    fn key(&self, label: u64) -> Result<(i32, i32, i32), Error> {
        let o = self.object(label)?;
        Ok((o.owner, o.first, o.second))
    }
    /// Dispatch a concrete predicate's state-change signal over the canonical
    /// current set rings. No requirement, set or modifier state is copied.
    pub fn requirement_changed(
        &mut self,
        state: &mut LoadedState,
        requirements: &mut sets::Runtime,
        label: u64,
        old: i32,
        new: i32,
    ) -> Result<(), sets::Error<Error>> {
        if requirements
            .simple_requirement(label)
            .is_none_or(|r| r.state != new)
        {
            return Err(sets::Error::Context(Error::Missing(
                "changed diplomacy requirement state",
            )));
        }
        sets::changes::dispatch_requirement_changed(
            &mut RequirementChange {
                state,
                requirements,
                modifiers: self,
            },
            label,
            old,
            new,
        )
    }
    fn modifier_changed_with_sets(
        &mut self,
        state: &mut LoadedState,
        requirements: &mut sets::Runtime,
        listener: sets::SetListener,
        set: u64,
    ) -> Result<(), Error> {
        if !self.stamp.same_instance(state.action_stamp()) {
            return Err(Error::Missing("diplomacy modifier World identity"));
        }
        let sets::SetListener::Subject(label) = listener else {
            return Err(Error::Missing("diplomacy non-subject modifier listener"));
        };
        if self
            .opinion_graph
            .as_ref()
            .is_some_and(|g| g.contains(label))
        {
            return self
                .with_opinion_graph(|graph, canonical| {
                    graph
                        .subject_changed(state, requirements, canonical, label, set)
                        .map(|_| ())
                })
                .map_err(Error::Missing);
        }
        let modifier = self
            .modifiers
            .iter()
            .position(|m| m.label == label)
            .ok_or(Error::Missing("current diplomacy modifier receiver"))?;
        let subject = self.modifiers[modifier]
            .tracked
            .iter()
            .position(|s| s.control.requirement_set == Some(set))
            .ok_or(Error::Missing("current diplomacy tracked requirement set"))?;
        modifiers::check_apply_effects(&mut Apply {
            state,
            requirements,
            owner: self,
            modifier,
            subject,
            opinion_yields: None,
        })
        .map(|_| ())
    }
    /// Bind additional captured opinion leaves without replacing prior owners.
    pub fn bind_additional_opinion_effects(
        &mut self,
        activations: Vec<Activation>,
        effects: Vec<(
            u64,
            crate::loaded_actions::stock_notifications::first_contact::opinion_callbacks::Effect,
        )>,
        alive_majors: Vec<i32>,
    ) -> Result<(), Error> {
        if activations.iter().enumerate().any(|(i, a)| {
            self.activations.iter().any(|old| old.label == a.label)
                || activations[..i].iter().any(|old| old.label == a.label)
                || !self.modifiers.iter().any(|m| m.label == a.label)
        }) || effects.iter().enumerate().any(|(i, (label, _))| {
            self.opinion_effects.iter().any(|(old, _)| old == label)
                || effects[..i].iter().any(|(old, _)| old == label)
                || !self.modifiers.iter().any(|m| m.label == *label)
        }) {
            return Err(Error::Missing(
                "duplicate/missing additional opinion effect binding",
            ));
        }
        if self
            .opinion_alive_majors
            .as_ref()
            .is_some_and(|old| old != &alive_majors)
        {
            return Err(Error::Missing(
                "current opinion alive-major roster mismatch",
            ));
        }
        self.activations.extend(activations);
        self.opinion_effects.extend(effects);
        self.opinion_alive_majors = Some(alive_majors);
        Ok(())
    }
    pub fn bind_opinion_graph(
        &mut self,
        graph: crate::game_update_state::activation::opinion_subject_graph::Runtime,
    ) -> Result<(), Error> {
        if self.opinion_graph.is_some()
            || graph
                .rows()
                .iter()
                .any(|r| !self.modifiers.iter().any(|m| m.label == r.label))
        {
            return Err(Error::Missing("canonical opinion graph binding"));
        }
        self.opinion_graph = Some(graph);
        Ok(())
    }
    pub fn opinion_graph(
        &self,
    ) -> Option<&crate::game_update_state::activation::opinion_subject_graph::Runtime> {
        self.opinion_graph.as_ref()
    }
    /// Loan metadata while retaining the sole mutable physical graph owner.
    /// Restore it on ordinary errors so unsupported transitions stay inspectable.
    pub fn with_opinion_graph<T>(
        &mut self,
        f: impl FnOnce(
            &mut crate::game_update_state::activation::opinion_subject_graph::Runtime,
            &mut Self,
        ) -> Result<T, &'static str>,
    ) -> Result<T, &'static str> {
        let mut graph = self
            .opinion_graph
            .take()
            .ok_or("canonical opinion graph absent")?;
        let result = f(&mut graph, self);
        self.opinion_graph = Some(graph);
        result
    }
    /// Merge additional current receivers into this same physical-object owner.
    /// Shared Player wrappers must describe the identical current affected list.
    pub fn merge_current(
        &mut self,
        state: &LoadedState,
        modifiers: Vec<Modifier>,
        objects: Vec<Object>,
    ) -> Result<(), Error> {
        if !self.stamp.same_instance(state.action_stamp()) || !state.is_pristine() {
            return Err(Error::Missing("modifier import after mutation"));
        }
        let incoming = Self::from_current(state, modifiers, objects)?;
        for object in &incoming.objects {
            if let Some(old) = self.objects.iter().find(|o| o.label == object.label) {
                if (old.owner, old.first, old.second, &old.affected)
                    != (object.owner, object.first, object.second, &object.affected)
                {
                    return Err(Error::Missing("aliased Player modifier object differs"));
                }
            }
        }
        if incoming
            .modifiers
            .iter()
            .any(|m| self.modifiers.iter().any(|old| old.label == m.label))
        {
            return Err(Error::Missing("duplicate canonical modifier receiver"));
        }
        for object in incoming.objects {
            if !self.objects.iter().any(|o| o.label == object.label) {
                self.objects.push(object);
            }
        }
        self.modifiers.extend(incoming.modifiers);
        Ok(())
    }
    /// Execute the concrete leaf after an additional source-qualified dynamic
    /// graph has selected this canonical tracked subject. Membership and applied
    /// flags remain with that graph's CheckApplyEffects invocation.
    pub fn apply_tracked_effect(
        &mut self,
        state: &mut LoadedState,
        requirements: &mut sets::Runtime,
        label: u64,
        subject: usize,
    ) -> Result<(), Error> {
        self.apply_tracked_effect_with_yields(state, requirements, label, subject, None)
    }
    pub fn apply_tracked_effect_with_yields<'s>(
        &'s mut self,
        state: &'s mut LoadedState,
        requirements: &'s mut sets::Runtime,
        label: u64,
        subject: usize,
        opinion_yields: Option<&'s mut dyn CurrentYields>,
    ) -> Result<(), Error> {
        if !self.stamp.same_instance(state.action_stamp()) {
            return Err(Error::Missing("diplomacy modifier World identity"));
        }
        let modifier = self
            .modifiers
            .iter()
            .position(|row| row.label == label)
            .ok_or(Error::Missing("current diplomacy modifier receiver"))?;
        if subject >= self.modifiers[modifier].tracked.len() {
            return Err(Error::Missing("current diplomacy tracked subject"));
        }
        modifiers::apply_effect(&mut Apply {
            state,
            requirements,
            owner: self,
            modifier,
            subject,
            opinion_yields,
        })
        .map_err(|error| match error {
            modifiers::EffectError::Context(error) => error,
            modifiers::EffectError::DefinitionDisappeared => {
                Error::Missing("diplomacy effect definition disappeared")
            }
        })
    }
    /// Borrow live controls for an additional source-qualified modifier family.
    pub fn modifier_mut(&mut self, label: u64) -> Option<&mut Modifier> {
        self.modifiers.iter_mut().find(|m| m.label == label)
    }
    /// Borrow the sole affected-instance list for this physical Player wrapper.
    pub fn object_mut(&mut self, label: u64) -> Option<&mut Object> {
        self.objects.iter_mut().find(|o| o.label == label)
    }
    pub fn activations(&self) -> &[Activation] {
        &self.activations
    }
    pub fn modifiers(&self) -> &[Modifier] {
        &self.modifiers
    }
    pub fn objects(&self) -> &[Object] {
        &self.objects
    }
}
struct RequirementChange<'a> {
    state: &'a mut LoadedState,
    requirements: &'a mut sets::Runtime,
    modifiers: &'a mut Modifiers,
}
impl sets::changes::Context for RequirementChange<'_> {
    type Error = Error;
    fn subject_sets(&self) -> Result<&sets::Runtime, Error> {
        Ok(self.requirements)
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, Error> {
        Ok(self.requirements)
    }
    fn test_requirement(&mut self, _: u64) -> Result<i32, sets::Error<Error>> {
        Err(sets::Error::Context(Error::Missing(
            "diplomacy unexpected predicate retest",
        )))
    }
    fn modifier_changed(
        &mut self,
        listener: sets::SetListener,
        set: u64,
        _: i32,
        _: i32,
    ) -> Result<(), sets::Error<Error>> {
        self.modifiers
            .modifier_changed_with_sets(self.state, self.requirements, listener, set)
            .map_err(sets::Error::Context)
    }
    fn destroy_requirement(&mut self, label: u64) -> Result<(), sets::Error<Error>> {
        self.modifiers
            .retire_has_met(self.requirements, label)
            .map_err(sets::Error::Context)
    }
    fn destroy_set(&mut self, _: u64) -> Result<(), sets::Error<Error>> {
        Err(sets::Error::Context(Error::Missing(
            "diplomacy set destruction",
        )))
    }
    fn reset_trigger(&mut self, set: u64) -> Result<(), sets::Error<Error>> {
        sets::changes::reset_trigger_state(self.requirements, set)
    }
}
struct Apply<'a> {
    opinion_yields: Option<&'a mut dyn CurrentYields>,
    state: &'a mut LoadedState,
    requirements: &'a mut sets::Runtime,
    owner: &'a mut Modifiers,
    modifier: usize,
    subject: usize,
}
impl modifiers::Context for Apply<'_> {
    type Error = Error;
    fn modifier(&self) -> Result<&modifiers::Modifier, Error> {
        Ok(&self.owner.modifiers[self.modifier].control)
    }
    fn tracked(&self) -> Result<&modifiers::TrackedSubject, Error> {
        Ok(&self.owner.modifiers[self.modifier].tracked[self.subject].control)
    }
    fn tracked_mut(&mut self) -> Result<&mut modifiers::TrackedSubject, Error> {
        Ok(&mut self.owner.modifiers[self.modifier].tracked[self.subject].control)
    }
    fn requirement_set_state(&self, id: u64) -> Result<i32, Error> {
        self.requirements
            .state(id)
            .ok_or(Error::Missing("current diplomacy subject set"))
    }
    fn ensure_applied_subject(&mut self) -> Result<(), Error> {
        let object = self.owner.modifiers[self.modifier].tracked[self.subject].object;
        let key = self.owner.key(object)?;
        let at = self.owner.modifiers[self.modifier]
            .applied
            .partition_point(|p| self.owner.key(*p).is_ok_and(|k| k < key));
        if self.owner.modifiers[self.modifier]
            .applied
            .get(at)
            .is_none_or(|p| self.owner.key(*p).ok() != Some(key))
        {
            self.owner.modifiers[self.modifier]
                .applied
                .insert(at, object);
        }
        Ok(())
    }
    fn is_repeatable(&self) -> Result<bool, Error> {
        Ok(self.owner.modifiers[self.modifier].repeatable)
    }
    fn apply_effect(&mut self) -> Result<bool, Error> {
        modifiers::apply_effect(self).map_err(|error| match error {
            modifiers::EffectError::Context(error) => error,
            modifiers::EffectError::DefinitionDisappeared => {
                Error::Missing("diplomacy effect definition disappeared")
            }
        })?;
        Ok(true)
    }
    fn unsubscribe_subject_requirement(&mut self) -> Result<(), Error> {
        Err(Error::Missing("diplomacy subject impossible-set release"))
    }
    fn clear_subject_requirement(&mut self) -> Result<(), Error> {
        Err(Error::Missing("diplomacy subject impossible-set release"))
    }
    fn remove_effects(&mut self) -> Result<(), Error> {
        let m = &self.owner.modifiers[self.modifier];
        if m.control.suppressed || m.permanent || m.tracked[self.subject].control.applied == 0 {
            return Ok(());
        }
        let object = m.tracked[self.subject].object;
        //2334770 sets applied before comparator lookup; map membership survives.
        self.owner.modifiers[self.modifier].tracked[self.subject]
            .control
            .applied = 0;
        let key = self.owner.key(object)?;
        let at = self.owner.modifiers[self.modifier]
            .applied
            .iter()
            .position(|p| self.owner.key(*p).ok() == Some(key));
        if let Some(at) = at {
            self.owner.modifiers[self.modifier].applied.remove(at);
        }
        let instance = self.owner.modifiers[self.modifier].instance;
        self.owner
            .objects
            .iter_mut()
            .find(|o| o.label == object)
            .ok_or(Error::Missing("diplomacy affected object"))?
            .affected
            .clear_instance(instance);
        //2334060 clears affected BEFORE testing the stack limit/concrete leaf.
        let m = &self.owner.modifiers[self.modifier];
        if m.stack_limit != 0 {
            return Err(Error::Missing("diplomacy removal subject stack limit"));
        }
        // RandomEffect::Remove returns true without touching the blackboard.
        if self
            .owner
            .random_effects
            .iter()
            .any(|(label, _)| *label == m.label)
        {
            return Ok(());
        }
        if self
            .owner
            .opinion_effects
            .iter()
            .any(|(label, effect)| *label == m.label && effect.remove_is_inert())
        {
            return Ok(());
        }
        let subject_owner = self.owner.object(object)?.owner;
        if !self
            .state
            .player_alive(subject_owner)
            .map_err(Error::State)?
            && !m.allow_dead
        {
            return Ok(());
        }
        let board = &mut self
            .state
            .war_diplomacy_mut(m.owner)
            .map_err(Error::State)?
            .blackboard;
        let entries = board
            .diplomatic_modifiers
            .get_mut(
                usize::try_from(subject_owner)
                    .map_err(|_| Error::Missing("blackboard subject index"))?,
            )
            .ok_or(Error::Missing("blackboard subject index"))?;
        let key = instance as u32;
        if entries.windows(2).any(|w| w[0].key >= w[1].key) {
            return Err(Error::Missing("canonical blackboard modifier map order"));
        }
        let at = entries.partition_point(|e| e.key < key);
        if entries.get(at).is_none_or(|e| e.key != key) {
            // eastl::map::operator[] constructs this value even when the
            // immediately following IsActive is false (2adaef0).
            entries.insert(
                at,
                c6_core::player_diplomacy_blackboard::ModifierEntry {
                    key,
                    tracker: c6_core::player_diplomacy_blackboard::DiplomaticModifierTracker {
                        is_active: false,
                        current_value: 0,
                        initial_value: 0,
                        max_value: 0,
                        increment: 0,
                        increment_turns: 0,
                        cooldown_turns: 0,
                        accumulation_turn_count: 0,
                        reduction: 0,
                        modifier_name: b"Unknown".to_vec(),
                        visibility: -1,
                    },
                },
            );
        }
        let tracker = &mut entries[at].tracker;
        //Blackboard.Deactivate2adbab0 -> Tracker.Deactivate2adb390.
        if tracker.is_active {
            tracker.is_active = false;
            tracker.accumulation_turn_count = 0;
            if tracker.cooldown_turns <= 0 {
                tracker.current_value = 0;
            }
        }
        // Remaining source path only formats LogModifierDeactivation; no
        // gameplay report, policy call, callback or RNG draw occurs here.
        Ok(())
    }
}
impl modifiers::EffectContext for Apply<'_> {
    type Error = Error;
    fn mark_subject_affected(&mut self) -> Result<(), Error> {
        let m = &self.owner.modifiers[self.modifier];
        let activation = self
            .owner
            .activations
            .iter()
            .find(|a| a.label == m.label)
            .ok_or(Error::Missing("positive diplomacy modifier parameters"))?;
        let object = m.tracked[self.subject].object;
        self.owner
            .objects
            .iter_mut()
            .find(|o| o.label == object)
            .ok_or(Error::Missing("diplomacy affected Player"))?
            .affected
            .mark(&activation.definition, m.instance)
            .map_err(|_| Error::Missing("diplomacy affected order"))
    }
    fn suppressed(&self) -> Result<bool, Error> {
        Ok(self.owner.modifiers[self.modifier].control.suppressed)
    }
    fn subject_stack_limit(&self) -> Result<Option<u16>, Error> {
        Ok(Some(self.owner.modifiers[self.modifier].stack_limit))
    }
    fn affected_count(&self) -> Result<i32, Error> {
        let m = &self.owner.modifiers[self.modifier];
        let a = self
            .owner
            .activations
            .iter()
            .find(|a| a.label == m.label)
            .ok_or(Error::Missing("diplomacy activation parameters"))?;
        Ok(self
            .owner
            .object(m.tracked[self.subject].object)?
            .affected
            .count(&a.definition))
    }
    fn log_applying(&mut self) -> Result<(), Error> {
        Ok(())
    }
    fn log_failed(&mut self) -> Result<(), Error> {
        Ok(())
    }
    fn apply_concrete_effect(&mut self) -> Result<bool, Error> {
        let m = &self.owner.modifiers[self.modifier];
        if self.owner.human_statement_owners.contains(&m.owner)
            && self
                .state
                .player_slot_status(m.owner)
                .map_err(Error::State)?
                != 3
        {
            return Err(Error::Missing("current human diplomacy statement guard"));
        }
        let subject = self.owner.object(m.tracked[self.subject].object)?.owner;
        let random = self
            .owner
            .random_effects
            .iter()
            .find(|(label, _)| *label == m.label)
            .map(|(_, parameters)| *parameters);
        let subject_is_human = if random.is_some() {
            self.state
                .player_slot_status(subject)
                .map_err(Error::State)?
                == 3
        } else {
            false
        };
        if !self.state.player_alive(subject).map_err(Error::State)? && !m.allow_dead {
            return Ok(true);
        }
        let opinion_preparation = self
            .owner
            .opinion_effects
            .iter_mut()
            .find(|(label, _)| *label == m.label)
            .map(|(_, effect)| {
                effect.prepare(
                    self.state,
                    m.owner,
                    subject,
                    m.instance,
                    self.owner.opinion_alive_majors.as_deref(),
                    self.opinion_yields
                        .as_mut()
                        .map(|q| &mut **q as &mut dyn CurrentYields),
                )
            })
            .transpose()
            .map_err(Error::Missing)?;
        use crate::loaded_actions::stock_notifications::first_contact::opinion_callbacks::Preparation;
        if opinion_preparation == Some(Preparation::Skip) {
            return Ok(true);
        }
        let government_value = self
            .owner
            .government_effects
            .as_ref()
            .filter(|(labels, _)| labels.contains(&m.label))
            .map(|(_, definitions)| definitions.value(self.state, m.owner, subject))
            .transpose()
            .map_err(Error::Missing)?;
        let a = self
            .owner
            .activations
            .iter_mut()
            .find(|a| a.label == m.label)
            .ok_or(Error::Missing("diplomacy activation parameters"))?;
        if let Some(value) = government_value.or(match opinion_preparation {
            Some(Preparation::Value(value)) => Some(value),
            _ => None,
        }) {
            a.initial = value;
            a.maximum = value;
        }
        let mut values = a.clone();
        let board = &mut self
            .state
            .war_diplomacy_mut(m.owner)
            .map_err(Error::State)?
            .blackboard;
        if let Some(parameters) = random {
            values.name = crate::loaded_actions::stock_notifications::first_contact::random_modifier::EFFECT_NAME.to_vec();
            if let Some(extra) =
                crate::loaded_actions::stock_notifications::first_contact::random_modifier::prepare(
                    board,
                    parameters,
                    m.instance as u32,
                    subject,
                    subject_is_human,
                )
                .map_err(|_| Error::Missing("random diplomacy value preparation"))?
            {
                // SetModifierValue supplies initial AND maximum. BaseApply
                // clears the temporary extra fields after consuming them.
                values.initial = extra;
                values.maximum = extra;
            }
        }
        let entries = board
            .diplomatic_modifiers
            .get_mut(usize::try_from(subject).map_err(|_| Error::Missing("diplomacy subject"))?)
            .ok_or(Error::Missing("diplomacy subject"))?;
        if entries.windows(2).any(|w| w[0].key >= w[1].key) {
            return Err(Error::Missing("diplomacy map order"));
        }
        let key = m.instance as u32;
        let at = entries.partition_point(|e| e.key < key);
        if entries.get(at).is_some_and(|e| e.key == key) {
            return Err(Error::Missing("existing diplomacy tracker activation"));
        }
        // BaseEffect skips Activate when its integer value already equals the
        // current value (zero for the absent tracker).
        if values.initial == 0 {
            if random.is_some() {
                // Random's explicit SimpleRemove uses map::operator[] even
                // when BaseApply skipped the zero-valued Activate.
                entries.insert(
                    at,
                    c6_core::player_diplomacy_blackboard::ModifierEntry {
                        key,
                        tracker: c6_core::player_diplomacy_blackboard::DiplomaticModifierTracker {
                            is_active: false,
                            current_value: 0,
                            initial_value: 0,
                            max_value: 0,
                            increment: 0,
                            increment_turns: 0,
                            cooldown_turns: 0,
                            accumulation_turn_count: 0,
                            reduction: 0,
                            modifier_name: b"Unknown".to_vec(),
                            visibility: -1,
                        },
                    },
                );
                return Ok(true);
            }
            if a.remove_immediately {
                return Err(Error::Missing("zero-value immediate diplomacy removal"));
            }
            return Ok(true);
        }
        if a.remove_immediately {
            // Fresh tracker GetCurrentValue is zero at both native reads in
            // Apply/AccumulateExtraValue. Existing trackers are rejected above.
            // These sign changes mutate the persistent concrete effect itself.
            if a.initial.wrapping_mul(a.increment) < 0 {
                a.increment = a.increment.wrapping_neg();
            }
            if a.initial.wrapping_mul(a.reduction) > 0 {
                a.reduction = a.reduction.wrapping_neg();
            }
            values.increment = a.increment;
            values.reduction = a.reduction;
            values.maximum = a.initial;
        }
        let mut tracker = first_tracker(&values);
        if a.remove_immediately || random.is_some() {
            tracker.is_active = false;
            tracker.accumulation_turn_count = 0;
            if tracker.cooldown_turns <= 0 {
                tracker.current_value = 0;
            }
        }
        entries.insert(
            at,
            c6_core::player_diplomacy_blackboard::ModifierEntry { key, tracker },
        );
        // SimpleEffect's opinion-table evaluator is omitted at the policy
        // boundary. Inputs must qualify a statement guard (both flags clear,
        // or the human-owner CanSendStatement return) before binding this
        // continuation. No statement, MESSAGE_SENT_TURN update, gameplay
        // callback or RNG follows that guard.
        Ok(true)
    }
}
fn first_tracker(
    a: &Activation,
) -> c6_core::player_diplomacy_blackboard::DiplomaticModifierTracker {
    let initial = a.initial.wrapping_shl(8);
    let maximum = a.maximum.wrapping_shl(8);
    let mut increment = a.increment.wrapping_shl(8);
    let mut reduction = a.reduction.wrapping_shl(8);
    if (((i64::from(increment) * i64::from(maximum)) >> 8) as i32) < 0 {
        increment = increment.wrapping_neg();
    }
    if (((i64::from(reduction) * i64::from(initial)) >> 8) as i32) > 0 {
        reduction = reduction.wrapping_neg();
    }
    c6_core::player_diplomacy_blackboard::DiplomaticModifierTracker {
        is_active: true,
        current_value: initial,
        initial_value: initial,
        max_value: maximum,
        increment,
        increment_turns: a.increment_turns,
        cooldown_turns: a.cooldown_turns,
        accumulation_turn_count: 0,
        reduction,
        modifier_name: a.name.clone(),
        visibility: a.visibility,
    }
}

impl Continuations for Modifiers {
    type Error = Error;
    fn modifier_changed(
        &mut self,
        state: &mut LoadedState,
        requirements: &mut Runtime,
        listener: sets::SetListener,
        set: u64,
        _old: i32,
        _new: i32,
    ) -> Result<(), Error> {
        self.modifier_changed_with_sets(state, requirements.sets_mut(), listener, set)
    }
    fn destroy_requirement(
        &mut self,
        _: &mut LoadedState,
        runtime: &mut Runtime,
        label: u64,
    ) -> Result<(), Error> {
        self.retire_has_met(runtime.sets(), label)
    }
    fn destroy_set(&mut self, _: &mut LoadedState, _: &mut Runtime, _: u64) -> Result<(), Error> {
        Err(Error::Missing("diplomacy set destruction"))
    }
    fn reset_trigger(&mut self, _: &mut LoadedState, _: &mut Runtime, _: u64) -> Result<(), Error> {
        Err(Error::Missing("diplomacy trigger reset"))
    }
}

#[cfg(test)]
mod activation_tests {
    use super::*;
    #[test]
    fn first_negative_penalty_corrects_increment_and_preserves_reduction() {
        let a = Activation {
            label: 1,
            definition: vec![],
            initial: -18,
            maximum: -18,
            increment: 1,
            increment_turns: 0,
            cooldown_turns: 0,
            reduction: 1,
            name: b"occupied".to_vec(),
            visibility: 0,
            remove_immediately: false,
        };
        let t = first_tracker(&a);
        assert_eq!(
            (t.current_value, t.increment, t.reduction),
            (-4608, -256, 256)
        );
        assert!(t.is_active);
    }
}
