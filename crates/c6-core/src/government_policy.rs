//! Linux Culture policy-slot mutations and CivicsCommand ordering.
//! Definition legality and synchronous consumers borrow the caller's live owner.
pub mod government;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    Retracted(i32),
    Enacted(i32),
    PolicyChanged(i32),
    BeginBatch,
    EndBatch,
    BatchChanged,
}

pub enum Command<'a> {
    Slot {
        policy: i32,
        slot: i32,
    },
    Clear {
        slot: i32,
    },
    Batch {
        clear: &'a [i32],
        insert: &'a [(i32, i32)],
    },
}

/// Native indices identify definitions; no policy decision is made here.
pub trait Context {
    type Error;
    fn can_slot(&mut self, policy: i32, slot: i32) -> Result<bool, Self::Error>;
    /// None means an invalid slot, Some(-1) an empty slot.
    fn policy_at(&self, slot: i32) -> Result<Option<i32>, Self::Error>;
    fn set_enacted(&mut self, policy: i32) -> Result<(), Self::Error>;
    fn set_active(&mut self, policy: i32, active: bool) -> Result<(), Self::Error>;
    fn set_slot(&mut self, slot: i32, policy: i32) -> Result<(), Self::Error>;
    fn event(&mut self, event: Event) -> Result<(), Self::Error>;
    fn set_change_made(&mut self) -> Result<(), Self::Error>;
    fn update_human_processor(&mut self) -> Result<(), Self::Error>;
}

/// Culture.SlotPolicy2a760c0. New active/enacted flags precede old removal;
/// the new slot is visible before Enacted subscribers execute.
pub fn slot<C: Context>(c: &mut C, policy: i32, index: i32) -> Result<bool, C::Error> {
    if !c.can_slot(policy, index)? {
        return Ok(false);
    }
    c.set_enacted(policy)?;
    c.set_active(policy, true)?;
    if let Some(old) = c.policy_at(index)?.filter(|&p| p != -1) {
        c.event(Event::Retracted(old))?;
        c.set_active(old, false)?;
    }
    c.set_slot(index, policy)?;
    c.event(Event::Enacted(policy))?;
    c.event(Event::PolicyChanged(policy))?;
    Ok(true)
}

/// Culture.ClearPolicySlot2a76230 has a different order from replacement:
/// both active flag and slot are cleared before Retracted subscribers execute.
pub fn clear<C: Context>(c: &mut C, index: i32) -> Result<bool, C::Error> {
    let Some(old) = c.policy_at(index)?.filter(|&p| p != -1) else {
        return Ok(false);
    };
    c.set_active(old, false)?;
    c.set_slot(index, -1)?;
    c.event(Event::Retracted(old))?;
    c.event(Event::PolicyChanged(old))?;
    Ok(true)
}

/// CivicsCommand.Execute2d21b20. Mutations are ordered and may retain a partial
/// prefix when a consumer fails. Single invalid requests still mark change made;
/// batch clears count as a change even when an individual slot was empty.
pub fn execute<C: Context>(c: &mut C, command: Command<'_>) -> Result<(), C::Error> {
    let changed = match command {
        Command::Slot {
            policy,
            slot: index,
        } => {
            slot(c, policy, index)?;
            true
        }
        Command::Clear { slot } => {
            clear(c, slot)?;
            true
        }
        Command::Batch {
            clear: removals,
            insert,
        } => {
            c.event(Event::BeginBatch)?;
            for &index in removals {
                clear(c, index)?;
            }
            let mut changed = !removals.is_empty();
            for &(policy, index) in insert {
                changed |= slot(c, policy, index)?;
            }
            c.event(Event::EndBatch)?;
            c.event(Event::BatchChanged)?;
            changed
        }
    };
    if changed {
        c.set_change_made()?;
    }
    c.update_human_processor()
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Current {
        slot: i32,
        active: [bool; 2],
        log: Vec<(Event, i32, [bool; 2])>,
        changed: bool,
    }
    impl Context for Current {
        type Error = ();
        fn can_slot(&mut self, p: i32, s: i32) -> Result<bool, ()> {
            Ok(s == 0 && (0..2).contains(&p) && !self.active[p as usize])
        }
        fn policy_at(&self, s: i32) -> Result<Option<i32>, ()> {
            Ok((s == 0).then_some(self.slot))
        }
        fn set_enacted(&mut self, _: i32) -> Result<(), ()> {
            Ok(())
        }
        fn set_active(&mut self, p: i32, b: bool) -> Result<(), ()> {
            self.active[p as usize] = b;
            Ok(())
        }
        fn set_slot(&mut self, _: i32, p: i32) -> Result<(), ()> {
            self.slot = p;
            Ok(())
        }
        fn event(&mut self, e: Event) -> Result<(), ()> {
            self.log.push((e, self.slot, self.active));
            Ok(())
        }
        fn set_change_made(&mut self) -> Result<(), ()> {
            self.changed = true;
            Ok(())
        }
        fn update_human_processor(&mut self) -> Result<(), ()> {
            Ok(())
        }
    }
    #[test]
    fn replacement_and_clear_expose_distinct_live_state_to_consumers() {
        let mut c = Current {
            slot: 0,
            active: [true, false],
            log: vec![],
            changed: false,
        };
        execute(&mut c, Command::Slot { policy: 1, slot: 0 }).unwrap();
        assert_eq!(c.log[0], (Event::Retracted(0), 0, [true, true]));
        assert_eq!(c.log[1], (Event::Enacted(1), 1, [false, true]));
        clear(&mut c, 0).unwrap();
        assert_eq!(c.log[3], (Event::Retracted(1), -1, [false, false]));
    }
    #[test]
    fn empty_clear_batch_still_marks_changed_and_preserves_insert_order() {
        let mut c = Current {
            slot: -1,
            active: [false, false],
            log: vec![],
            changed: false,
        };
        execute(
            &mut c,
            Command::Batch {
                clear: &[0],
                insert: &[(0, 0), (1, 0)],
            },
        )
        .unwrap();
        assert!(c.changed);
        assert_eq!(c.slot, 1);
        assert_eq!(c.log.first().unwrap().0, Event::BeginBatch);
        assert_eq!(c.log.last().unwrap().0, Event::BatchChanged);
    }
}
