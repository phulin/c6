//! Linux Culture.SetCurrentGovernment ordering (2a72100).
//! The ordinary legal-choice gate is separate from this source mutation body.
use super::{Context as PolicyContext, Event as PolicyEvent};

/// BuildSlots2a74b77 visits the original slots backwards, erases each match,
/// then appends a fresh empty slot. `replaces_all=false` stops after the last
/// matching original slot; appended slots are not reconsidered by that rule.
pub fn replace_slots(
    slots: &mut Vec<crate::player_culture::GovernmentSlot>,
    replacements: &[crate::player_culture::GovernmentSlotReplacement],
) {
    for replacement in replacements {
        for index in (0..slots.len()).rev() {
            if slots[index].slot_ordinal != replacement.replaced_slot_ordinal {
                continue;
            }
            slots.remove(index);
            slots.push(crate::player_culture::GovernmentSlot {
                slot_ordinal: replacement.added_slot_ordinal,
                policy_hash: u32::MAX,
            });
            if !replacement.replaces_all {
                break;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    Retracted(i32),
    Enacted(i32),
    Changed(i32),
}

pub trait Context: PolicyContext {
    fn valid_government(&self, government: i32) -> bool;
    fn current_government(&self) -> Result<i32, Self::Error>;
    fn government_event(&mut self, event: Event) -> Result<(), Self::Error>;
    fn store_previous_policies(&mut self) -> Result<(), Self::Error>;
    fn slot_count(&self) -> Result<usize, Self::Error>;
    fn clear_slots(&mut self) -> Result<(), Self::Error>;
    fn reset_policy_change(&mut self) -> Result<(), Self::Error>;
    fn record_government_tier(&mut self, government: i32) -> Result<(), Self::Error>;
    fn anarchy_turns(&self, government: i32) -> Result<i32, Self::Error>;
    fn set_government(&mut self, government: i32) -> Result<(), Self::Error>;
    fn set_post_anarchy_government(&mut self, government: i32) -> Result<(), Self::Error>;
    fn enter_anarchy(&mut self, new: i32, old: i32, turns: i32) -> Result<(), Self::Error>;
    fn mark_government_used(&mut self, government: i32) -> Result<(), Self::Error>;
    fn set_turn_adopted(&mut self) -> Result<(), Self::Error>;
    fn rebuild_slots(&mut self, government: i32) -> Result<(), Self::Error>;
    fn check_government_notifications(&mut self) -> Result<(), Self::Error>;
}

/// ClearPolicies(-1) retains every slot until all retraction callbacks return.
/// Unlike ClearPolicySlot, it clears the complete vector only at the end.
pub fn clear_all<C: Context>(c: &mut C) -> Result<(), C::Error> {
    let count = c.slot_count()?;
    for slot in 0..count {
        if let Some(policy) = c.policy_at(slot as i32)?.filter(|&p| p != -1) {
            c.set_active(policy, false)?;
            c.event(PolicyEvent::Retracted(policy))?;
            c.event(PolicyEvent::PolicyChanged(policy))?;
        }
    }
    c.clear_slots()
}

/// A consumer failure retains the actual completed prefix. Positive anarchy
/// goes through enter_anarchy; an implementation must not silently skip it.
pub fn set_current<C: Context>(c: &mut C, government: i32) -> Result<bool, C::Error> {
    if !c.valid_government(government) {
        return Ok(false);
    }
    let old = c.current_government()?;
    if old != -1 {
        c.government_event(Event::Retracted(old))?;
    }
    c.store_previous_policies()?;
    clear_all(c)?;
    c.reset_policy_change()?;
    c.record_government_tier(government)?;
    let mut anarchy = 0;
    if c.current_government()? != government {
        anarchy = if old == -1 {
            0
        } else {
            c.anarchy_turns(government)?
        };
        if anarchy > 0 {
            c.set_government(-1)?;
            c.set_post_anarchy_government(government)?;
            c.enter_anarchy(government, old, anarchy)?;
        } else {
            c.set_government(government)?;
            c.mark_government_used(government)?;
        }
        c.set_turn_adopted()?;
    }
    if anarchy == 0 {
        c.rebuild_slots(government)?;
        c.government_event(Event::Enacted(c.current_government()?))?;
    }
    c.government_event(Event::Changed(c.current_government()?))?;
    c.check_government_notifications()?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacements_remove_last_match_and_append_before_the_next_rule() {
        use crate::player_culture::{GovernmentSlot, GovernmentSlotReplacement};
        let mut slots = [0, 2, 1, 2]
            .map(|slot_ordinal| GovernmentSlot {
                slot_ordinal,
                policy_hash: u32::MAX,
            })
            .to_vec();
        let mut replacement = GovernmentSlotReplacement {
            wire_version: 45,
            replaced_slot_ordinal: 2,
            added_slot_ordinal: 4,
            replaces_all: false,
        };
        replace_slots(&mut slots, &[replacement.clone()]);
        assert_eq!(
            slots.iter().map(|s| s.slot_ordinal).collect::<Vec<_>>(),
            [0, 2, 1, 4]
        );
        replacement.replaces_all = true;
        replace_slots(&mut slots, &[replacement]);
        assert_eq!(
            slots.iter().map(|s| s.slot_ordinal).collect::<Vec<_>>(),
            [0, 1, 4, 4]
        );
    }
    struct Probe {
        government: i32,
        post: i32,
        slots: Vec<i32>,
        active: [bool; 2],
        previous: Vec<i32>,
        anarchy: i32,
        trace: Vec<(&'static str, Vec<i32>, [bool; 2])>,
    }
    impl Probe {
        fn new(anarchy: i32) -> Self {
            Self {
                government: 0,
                post: -1,
                slots: vec![0, 1],
                active: [true, true],
                previous: vec![],
                anarchy,
                trace: vec![],
            }
        }
        fn log(&mut self, s: &'static str) {
            self.trace.push((s, self.slots.clone(), self.active));
        }
    }
    impl PolicyContext for Probe {
        type Error = &'static str;
        fn can_slot(&mut self, _: i32, _: i32) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn policy_at(&self, s: i32) -> Result<Option<i32>, Self::Error> {
            Ok(self.slots.get(s as usize).copied())
        }
        fn set_enacted(&mut self, _: i32) -> Result<(), Self::Error> {
            Ok(())
        }
        fn set_active(&mut self, p: i32, b: bool) -> Result<(), Self::Error> {
            self.active[p as usize] = b;
            Ok(())
        }
        fn set_slot(&mut self, _: i32, _: i32) -> Result<(), Self::Error> {
            unreachable!()
        }
        fn event(&mut self, e: PolicyEvent) -> Result<(), Self::Error> {
            if matches!(e, PolicyEvent::Retracted(_)) {
                self.log("policy retracted");
            }
            Ok(())
        }
        fn set_change_made(&mut self) -> Result<(), Self::Error> {
            unreachable!()
        }
        fn update_human_processor(&mut self) -> Result<(), Self::Error> {
            unreachable!()
        }
    }
    impl Context for Probe {
        fn valid_government(&self, g: i32) -> bool {
            g == 1
        }
        fn current_government(&self) -> Result<i32, Self::Error> {
            Ok(self.government)
        }
        fn government_event(&mut self, e: Event) -> Result<(), Self::Error> {
            self.log(match e {
                Event::Retracted(_) => "government retracted",
                Event::Enacted(_) => "government enacted",
                Event::Changed(_) => "government report",
            });
            Ok(())
        }
        fn store_previous_policies(&mut self) -> Result<(), Self::Error> {
            self.previous = self.slots.clone();
            Ok(())
        }
        fn slot_count(&self) -> Result<usize, Self::Error> {
            Ok(self.slots.len())
        }
        fn clear_slots(&mut self) -> Result<(), Self::Error> {
            self.slots.clear();
            Ok(())
        }
        fn reset_policy_change(&mut self) -> Result<(), Self::Error> {
            self.log("change reset");
            Ok(())
        }
        fn record_government_tier(&mut self, _: i32) -> Result<(), Self::Error> {
            Ok(())
        }
        fn anarchy_turns(&self, _: i32) -> Result<i32, Self::Error> {
            Ok(self.anarchy)
        }
        fn set_government(&mut self, g: i32) -> Result<(), Self::Error> {
            self.government = g;
            Ok(())
        }
        fn set_post_anarchy_government(&mut self, g: i32) -> Result<(), Self::Error> {
            self.post = g;
            Ok(())
        }
        fn enter_anarchy(&mut self, _: i32, _: i32, _: i32) -> Result<(), Self::Error> {
            Err("anarchy continuation")
        }
        fn mark_government_used(&mut self, _: i32) -> Result<(), Self::Error> {
            Ok(())
        }
        fn set_turn_adopted(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn rebuild_slots(&mut self, _: i32) -> Result<(), Self::Error> {
            self.slots = vec![-1; 4];
            Ok(())
        }
        fn check_government_notifications(&mut self) -> Result<(), Self::Error> {
            self.log("notifications");
            Ok(())
        }
    }
    #[test]
    fn callbacks_observe_old_slots_until_all_policy_retractions_finish() {
        let mut p = Probe::new(0);
        assert_eq!(set_current(&mut p, 1), Ok(true));
        assert_eq!(
            p.trace[0],
            ("government retracted", vec![0, 1], [true, true])
        );
        assert_eq!(p.trace[1], ("policy retracted", vec![0, 1], [false, true]));
        assert_eq!(p.trace[2], ("policy retracted", vec![0, 1], [false, false]));
        assert_eq!(p.trace[3], ("change reset", vec![], [false, false]));
        assert_eq!(p.previous, vec![0, 1]);
        assert_eq!(p.trace[4].0, "government enacted");
        assert_eq!(p.trace.last().unwrap().0, "notifications");
    }
    #[test]
    fn unsupported_anarchy_preserves_prefix_without_claiming_enactment() {
        let mut p = Probe::new(3);
        assert_eq!(set_current(&mut p, 1), Err("anarchy continuation"));
        assert_eq!((p.government, p.post), (-1, 1));
        assert!(p.slots.is_empty());
        assert!(!p.trace.iter().any(|r| r.0 == "government enacted"));
        let mut invalid = Probe::new(0);
        assert_eq!(set_current(&mut invalid, 99), Ok(false));
        assert!(invalid.trace.is_empty());
    }
}
