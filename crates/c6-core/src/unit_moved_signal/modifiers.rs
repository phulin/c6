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
