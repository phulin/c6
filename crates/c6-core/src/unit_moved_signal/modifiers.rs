//! Linux DynamicModifier.CheckApplyEffects (0x2334a90).
//! These fields are live modifier/subject state; effect dispatch remains concrete
//! and mandatory. No legal-action, predicate, or native effect answers are inputs.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Modifier {
    pub enabled: bool,
    pub suppressed: bool,
    pub owner_requirement_set: Option<u64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrackedSubject {
    pub requirement_set: Option<u64>,
    /// Native compares this byte with exactly1, not merely nonzero.
    pub applied: u8,
    pub removed: bool,
}
pub trait Context {
    type Error;
    fn modifier(&self) -> Result<&Modifier, Self::Error>;
    fn tracked(&self) -> Result<&TrackedSubject, Self::Error>;
    fn tracked_mut(&mut self) -> Result<&mut TrackedSubject, Self::Error>;
    fn requirement_set_state(&self, id: u64) -> Result<i32, Self::Error>;
    /// Insert the current subject only when absent from the live applied vector,
    /// using the actual DynamicModifierComparator order/equivalence.
    fn ensure_applied_subject(&mut self) -> Result<(), Self::Error>;
    fn is_repeatable(&self) -> Result<bool, Self::Error>;
    fn apply_effect(&mut self) -> Result<bool, Self::Error>;
    fn unsubscribe_subject_requirement(&mut self) -> Result<(), Self::Error>;
    /// Swap the live subject-set shared reference to empty and release the old
    /// reference. Any reached destruction effects must complete before return.
    fn clear_subject_requirement(&mut self) -> Result<(), Self::Error>;
    fn remove_effects(&mut self) -> Result<(), Self::Error>;
}

/// Returns the native CheckApplyEffects boolean. A reached ApplyEffect false
/// result is deliberately ignored by this caller, which still returns true.
pub fn check_apply_effects<C: Context>(c: &mut C) -> Result<bool, C::Error> {
    if c.modifier()?.suppressed || !c.modifier()?.enabled || c.tracked()?.removed {
        return Ok(false);
    }
    let owner_triggered = match c.modifier()?.owner_requirement_set {
        Some(id) => c.requirement_set_state(id)? == 4,
        None => false,
    };
    let mut subject_triggered = false;
    let eligible = match c.tracked()?.requirement_set {
        None => true,
        Some(id) => {
            let met = (c.requirement_set_state(id)? | 2) == 2;
            subject_triggered = match c.tracked()?.requirement_set {
                Some(current) => c.requirement_set_state(current)? == 4,
                None => false,
            };
            met || subject_triggered
        }
    };
    if eligible {
        if owner_triggered || c.tracked()?.applied != 1 || subject_triggered {
            c.ensure_applied_subject()?;
            if !c.is_repeatable()? {
                c.tracked_mut()?.applied = 1;
            }
            let _ = c.apply_effect()?;
            return Ok(true);
        }
        return Ok(false);
    }
    // Native rereads the subject set after the preceding getters.
    if let Some(id) = c.tracked()?.requirement_set {
        if c.requirement_set_state(id)? == 3 {
            c.unsubscribe_subject_requirement()?;
            c.clear_subject_requirement()?;
            c.tracked_mut()?.removed = true;
        }
    }
    c.remove_effects()?;
    Ok(false)
}

/// Current concrete DynamicModifier.ApplyEffect continuation. Logging methods
/// must route the actual stock output path or a source-qualified inert profile.
pub trait EffectContext {
    type Error;
    fn mark_subject_affected(&mut self) -> Result<(), Self::Error>;
    fn suppressed(&self) -> Result<bool, Self::Error>;
    fn subject_stack_limit(&self) -> Result<Option<u16>, Self::Error>;
    fn affected_count(&self) -> Result<i32, Self::Error>;
    fn log_applying(&mut self) -> Result<(), Self::Error>;
    fn apply_concrete_effect(&mut self) -> Result<bool, Self::Error>;
    fn log_failed(&mut self) -> Result<(), Self::Error>;
}
#[derive(Debug, PartialEq, Eq)]
pub enum EffectError<E> {
    Context(E),
    DefinitionDisappeared,
}
/// 0x23348b0: MarkAsAffected happens even when suppression/stack-limit subsequently
/// skips the kernel. A count equal to the u16 limit still permits application.
pub fn apply_effect<C: EffectContext>(c: &mut C) -> Result<(), EffectError<C::Error>> {
    c.mark_subject_affected().map_err(EffectError::Context)?;
    if c.suppressed().map_err(EffectError::Context)? {
        return Ok(());
    }
    if let Some(limit) = c.subject_stack_limit().map_err(EffectError::Context)? {
        if limit != 0 {
            let count = c.affected_count().map_err(EffectError::Context)?;
            let limit = c
                .subject_stack_limit()
                .map_err(EffectError::Context)?
                .ok_or(EffectError::DefinitionDisappeared)?;
            if count > i32::from(limit) {
                return Ok(());
            }
        }
    }
    c.log_applying().map_err(EffectError::Context)?;
    if !c.apply_concrete_effect().map_err(EffectError::Context)? {
        c.log_failed().map_err(EffectError::Context)?;
    }
    Ok(())
}

/// Canonical ModifierObject affected-instance list. Native entries are ordered
/// by definition bytes; distinct instances of one definition insert first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AffectedEntry {
    pub definition: Vec<u8>,
    pub instance: i32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AffectedModifiers {
    entries: Vec<AffectedEntry>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AffectedError {
    Order,
    Allocation,
}
impl AffectedModifiers {
    pub fn from_sorted(entries: Vec<AffectedEntry>) -> Result<Self, AffectedError> {
        if entries
            .windows(2)
            .any(|r| r[0].definition > r[1].definition)
        {
            return Err(AffectedError::Order);
        }
        Ok(Self { entries })
    }
    pub fn entries(&self) -> &[AffectedEntry] {
        &self.entries
    }
    /// ModifierObject.MarkAsAffected2335a30: duplicate same definition/instance
    /// does nothing; a different instance inserts at the LOWER bound.
    pub fn mark(&mut self, definition: &[u8], instance: i32) -> Result<(), AffectedError> {
        let lower = self
            .entries
            .partition_point(|r| r.definition.as_slice() < definition);
        let upper = lower
            + self.entries[lower..].partition_point(|r| r.definition.as_slice() == definition);
        if self.entries[lower..upper]
            .iter()
            .any(|r| r.instance == instance)
        {
            return Ok(());
        }
        let mut owned = Vec::new();
        owned
            .try_reserve(definition.len())
            .map_err(|_| AffectedError::Allocation)?;
        owned.extend_from_slice(definition);
        self.entries
            .try_reserve(1)
            .map_err(|_| AffectedError::Allocation)?;
        self.entries.insert(
            lower,
            AffectedEntry {
                definition: owned,
                instance,
            },
        );
        Ok(())
    }
    /// GetModifierAffectedByCount2331500 counts the current matching instances.
    pub fn count(&self, definition: &[u8]) -> i32 {
        let lower = self
            .entries
            .partition_point(|r| r.definition.as_slice() < definition);
        self.entries[lower..].partition_point(|r| r.definition.as_slice() == definition) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Test {
        modifier: Modifier,
        tracked: TrackedSubject,
        states: [i32; 2],
        repeat: bool,
        events: Vec<&'static str>,
    }
    impl Context for Test {
        type Error = ();
        fn modifier(&self) -> Result<&Modifier, ()> {
            Ok(&self.modifier)
        }
        fn tracked(&self) -> Result<&TrackedSubject, ()> {
            Ok(&self.tracked)
        }
        fn tracked_mut(&mut self) -> Result<&mut TrackedSubject, ()> {
            Ok(&mut self.tracked)
        }
        fn requirement_set_state(&self, id: u64) -> Result<i32, ()> {
            Ok(self.states[id as usize])
        }
        fn ensure_applied_subject(&mut self) -> Result<(), ()> {
            self.events.push("insert");
            Ok(())
        }
        fn is_repeatable(&self) -> Result<bool, ()> {
            Ok(self.repeat)
        }
        fn apply_effect(&mut self) -> Result<bool, ()> {
            assert!(self.repeat || self.tracked.applied == 1);
            self.events.push("apply");
            Ok(false)
        }
        fn unsubscribe_subject_requirement(&mut self) -> Result<(), ()> {
            self.events.push("unsubscribe");
            Ok(())
        }
        fn clear_subject_requirement(&mut self) -> Result<(), ()> {
            self.tracked.requirement_set = None;
            self.events.push("clear");
            Ok(())
        }
        fn remove_effects(&mut self) -> Result<(), ()> {
            self.events.push("remove");
            Ok(())
        }
    }
    fn fixture() -> Test {
        Test {
            modifier: Modifier {
                enabled: true,
                suppressed: false,
                owner_requirement_set: None,
            },
            tracked: TrackedSubject {
                requirement_set: Some(0),
                applied: 0,
                removed: false,
            },
            states: [0, 0],
            repeat: false,
            events: vec![],
        }
    }
    struct ApplyTest {
        count: i32,
        limit: Option<u16>,
        suppressed: bool,
        events: Vec<&'static str>,
    }
    impl EffectContext for ApplyTest {
        type Error = ();
        fn mark_subject_affected(&mut self) -> Result<(), ()> {
            self.count += 1;
            self.events.push("mark");
            Ok(())
        }
        fn suppressed(&self) -> Result<bool, ()> {
            Ok(self.suppressed)
        }
        fn subject_stack_limit(&self) -> Result<Option<u16>, ()> {
            Ok(self.limit)
        }
        fn affected_count(&self) -> Result<i32, ()> {
            assert_ne!(self.limit, Some(0));
            Ok(self.count)
        }
        fn log_applying(&mut self) -> Result<(), ()> {
            self.events.push("log");
            Ok(())
        }
        fn apply_concrete_effect(&mut self) -> Result<bool, ()> {
            self.events.push("effect");
            Ok(false)
        }
        fn log_failed(&mut self) -> Result<(), ()> {
            self.events.push("failed");
            Ok(())
        }
    }
    #[test]
    fn affected_mark_survives_stack_rejection_and_equal_limit_runs_effect() {
        let mut c = ApplyTest {
            count: 0,
            limit: Some(1),
            suppressed: false,
            events: vec![],
        };
        apply_effect(&mut c).unwrap();
        assert_eq!(c.events, ["mark", "log", "effect", "failed"]);
        c.events.clear();
        apply_effect(&mut c).unwrap();
        assert_eq!(c.events, ["mark"]);
        assert_eq!(c.count, 2);
        c.events.clear();
        c.limit = Some(0);
        apply_effect(&mut c).unwrap();
        assert_eq!(c.events, ["mark", "log", "effect", "failed"]);
        c.events.clear();
        c.suppressed = true;
        apply_effect(&mut c).unwrap();
        assert_eq!(c.events, ["mark"]);
    }
    #[test]
    fn affected_instances_preserve_definition_order_and_native_lower_bound_insertion() {
        let mut list = AffectedModifiers::from_sorted(vec![]).unwrap();
        list.mark(b"Z", 7).unwrap();
        list.mark(b"A", 2).unwrap();
        list.mark(b"A", 1).unwrap();
        list.mark(b"A", 2).unwrap();
        assert_eq!(
            list.entries()
                .iter()
                .map(|r| r.instance)
                .collect::<Vec<_>>(),
            [1, 2, 7]
        );
        assert_eq!(list.count(b"A"), 2);
        assert_eq!(list.count(b"B"), 0);
    }
    #[test]
    fn inserts_and_marks_before_real_effect_even_if_apply_returns_false() {
        let mut c = fixture();
        assert_eq!(check_apply_effects(&mut c), Ok(true));
        assert_eq!(c.events, ["insert", "apply"]);
        assert_eq!(c.tracked.applied, 1);
        c.events.clear();
        assert_eq!(check_apply_effects(&mut c), Ok(false));
        assert!(c.events.is_empty());
    }
    #[test]
    fn subject_trigger_repeats_and_repeatable_keeps_applied_byte() {
        let mut c = fixture();
        c.states[0] = 4;
        c.tracked.applied = 1;
        assert_eq!(check_apply_effects(&mut c), Ok(true));
        c = fixture();
        c.repeat = true;
        assert_eq!(check_apply_effects(&mut c), Ok(true));
        assert_eq!(c.tracked.applied, 0);
        c = fixture();
        c.tracked.applied = 2;
        assert_eq!(check_apply_effects(&mut c), Ok(true));
    }
    #[test]
    fn failed_and_terminal_sets_remove_in_native_order() {
        let mut c = fixture();
        c.states[0] = 1;
        assert_eq!(check_apply_effects(&mut c), Ok(false));
        assert_eq!(c.events, ["remove"]);
        c = fixture();
        c.states[0] = 3;
        assert_eq!(check_apply_effects(&mut c), Ok(false));
        assert_eq!(c.events, ["unsubscribe", "clear", "remove"]);
        assert!(c.tracked.removed);
        c.events.clear();
        assert_eq!(check_apply_effects(&mut c), Ok(false));
        assert!(c.events.is_empty());
    }
}
