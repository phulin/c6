//! One stock CombatProposals ObjectCreated listener, not the entire signal.
//! The proposal registry must register the object before invoking this listener.
//! Other registered consumers and their modifier effects remain mandatory.

/// Typed Unit/Unit proposal receiver. Both owners are read from the retained
/// current units before any signal dispatch. Signal registrations themselves
/// are looked up when reached, so earlier callbacks can change later listeners.
pub trait OwnerCreatedContext {
    type Error;
    fn attacker_owner(&mut self) -> Result<i32, Self::Error>;
    fn defender_owner(&mut self) -> Result<i32, Self::Error>;
    fn has_owner_signal(&mut self, owner: i32) -> Result<bool, Self::Error>;
    fn dispatch_owner_created(&mut self, owner: i32) -> Result<(), Self::Error>;
}

/// CombatProposals ctor lambda564, 2419690..2419781, after successful typed
/// QueryInterface/IProposedCombatInterface lookup. This never creates an empty
/// registry or assumes a missing imported callback list means no subscribers.
pub fn unit_owners_created<C: OwnerCreatedContext>(context: &mut C) -> Result<(), C::Error> {
    let attacker = context.attacker_owner()?;
    let defender = context.defender_owner()?;
    if attacker != -1 && context.has_owner_signal(attacker)? {
        context.dispatch_owner_created(attacker)?;
    }
    if defender != -1 && defender != attacker && context.has_owner_signal(defender)? {
        context.dispatch_owner_created(defender)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct C {
        attacker: i32,
        defender: i32,
        registered: Vec<i32>,
        events: Vec<(&'static str, i32)>,
        fail_dispatch: bool,
    }
    impl OwnerCreatedContext for C {
        type Error = &'static str;
        fn attacker_owner(&mut self) -> Result<i32, Self::Error> {
            self.events.push(("attacker", self.attacker));
            Ok(self.attacker)
        }
        fn defender_owner(&mut self) -> Result<i32, Self::Error> {
            self.events.push(("defender", self.defender));
            Ok(self.defender)
        }
        fn has_owner_signal(&mut self, owner: i32) -> Result<bool, Self::Error> {
            self.events.push(("find", owner));
            Ok(self.registered.contains(&owner))
        }
        fn dispatch_owner_created(&mut self, owner: i32) -> Result<(), Self::Error> {
            self.events.push(("dispatch", owner));
            // Same-state callback changes a current owner and registers the
            // already-cached second owner's signal before its lookup.
            self.defender = 9;
            self.registered.push(3);
            if self.fail_dispatch {
                Err("callback")
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn callback_preserves_cached_owners_but_second_lookup_is_current() {
        let mut c = C {
            attacker: 1,
            defender: 3,
            registered: vec![1],
            events: vec![],
            fail_dispatch: false,
        };
        unit_owners_created(&mut c).unwrap();
        assert_eq!(
            c.events,
            [
                ("attacker", 1),
                ("defender", 3),
                ("find", 1),
                ("dispatch", 1),
                ("find", 3),
                ("dispatch", 3)
            ]
        );
    }
    #[test]
    fn equal_owners_dispatch_once_and_minus_one_is_skipped() {
        let mut c = C {
            attacker: 1,
            defender: 1,
            registered: vec![1],
            events: vec![],
            fail_dispatch: false,
        };
        unit_owners_created(&mut c).unwrap();
        assert_eq!(
            c.events
                .iter()
                .filter(|(name, _)| *name == "dispatch")
                .count(),
            1
        );
        c.attacker = -1;
        c.defender = -1;
        c.events.clear();
        unit_owners_created(&mut c).unwrap();
        assert_eq!(c.events, [("attacker", -1), ("defender", -1)]);
    }
    #[test]
    fn callback_error_retains_effects_and_stops_before_second_lookup() {
        let mut c = C {
            attacker: 1,
            defender: 3,
            registered: vec![1],
            events: vec![],
            fail_dispatch: true,
        };
        assert_eq!(unit_owners_created(&mut c), Err("callback"));
        assert_eq!(c.defender, 9);
        assert!(c.registered.contains(&3));
        assert_eq!(
            c.events,
            [
                ("attacker", 1),
                ("defender", 3),
                ("find", 1),
                ("dispatch", 1)
            ]
        );
    }
}
