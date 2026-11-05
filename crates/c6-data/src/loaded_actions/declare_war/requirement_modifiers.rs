//! Canonical current Player diplomacy modifiers reached by declaration callbacks.
//! This owner holds membership/affected state once; requirement values remain in
//! the action's shared backend Runtime and blackboard trackers remain in Player.
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
pub struct Modifiers {
    stamp: ActionStamp,
    modifiers: Vec<Modifier>,
    objects: Vec<Object>,
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
            modifiers,
            objects,
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
    pub fn modifiers(&self) -> &[Modifier] {
        &self.modifiers
    }
    pub fn objects(&self) -> &[Object] {
        &self.objects
    }
}
struct Apply<'a> {
    state: &'a mut LoadedState,
    requirements: &'a mut Runtime,
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
            .sets()
            .state(id)
            .ok_or(Error::Missing("current diplomacy subject set"))
    }
    fn ensure_applied_subject(&mut self) -> Result<(), Error> {
        Err(Error::Missing("positive diplomacy modifier application"))
    }
    fn is_repeatable(&self) -> Result<bool, Error> {
        Ok(self.owner.modifiers[self.modifier].repeatable)
    }
    fn apply_effect(&mut self) -> Result<bool, Error> {
        Err(Error::Missing("positive diplomacy modifier application"))
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
        if !self.stamp.same_instance(state.action_stamp()) {
            return Err(Error::Missing("diplomacy modifier World identity"));
        }
        let sets::SetListener::Subject(label) = listener else {
            return Err(Error::Missing("diplomacy non-subject modifier listener"));
        };
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
        })
        .map(|_| ())
    }
    fn destroy_requirement(
        &mut self,
        _: &mut LoadedState,
        _: &mut Runtime,
        _: u64,
    ) -> Result<(), Error> {
        Err(Error::Missing("diplomacy requirement destruction"))
    }
    fn destroy_set(&mut self, _: &mut LoadedState, _: &mut Runtime, _: u64) -> Result<(), Error> {
        Err(Error::Missing("diplomacy set destruction"))
    }
    fn reset_trigger(&mut self, _: &mut LoadedState, _: &mut Runtime, _: u64) -> Result<(), Error> {
        Err(Error::Missing("diplomacy trigger reset"))
    }
}
