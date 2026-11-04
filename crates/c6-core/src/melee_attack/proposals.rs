//! Linux CombatProposals ownership and source-qualified ObjectCreated listeners.
//! Registered consumers and their current modifier effects remain mandatory;
//! none of these stages completes melee generation or an applied attack.
use crate::unit::ComponentId;
pub mod alliance;
pub mod alliance_strength;
pub mod collection_modifiers;
pub mod collections;
pub mod player_strength;
pub mod registry;
pub mod subject_predicates;
pub mod subject_sets;

pub trait ComponentCreatedContext {
    type Error;
    /// Native missing Unit member becomes the default invalid ComponentID;
    /// District members are not substituted by this particular listener.
    fn attacker_unit_id(&mut self) -> Result<ComponentId, Self::Error>;
    fn defender_unit_id(&mut self) -> Result<ComponentId, Self::Error>;
    fn has_component_signal(&mut self, identity: ComponentId) -> Result<bool, Self::Error>;
    fn dispatch_component_created(&mut self, identity: ComponentId) -> Result<(), Self::Error>;
}
pub fn component_valid(identity: ComponentId) -> bool {
    identity.owner != -1 && identity.id != u32::MAX
}
/// Stock lambda5652419860. Compare the complete owner/type/id tuple; capture
/// both current IDs before callbacks, but perform each map lookup when reached.
pub fn unit_components_created<C: ComponentCreatedContext>(
    context: &mut C,
) -> Result<(), C::Error> {
    let attacker = context.attacker_unit_id()?;
    let defender = context.defender_unit_id()?;
    if component_valid(attacker) && context.has_component_signal(attacker)? {
        context.dispatch_component_created(attacker)?;
    }
    if component_valid(defender)
        && defender != attacker
        && context.has_component_signal(defender)?
    {
        context.dispatch_component_created(defender)?;
    }
    Ok(())
}

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
    struct Components {
        attacker: ComponentId,
        defender: ComponentId,
        dispatched: Vec<ComponentId>,
    }
    impl ComponentCreatedContext for Components {
        type Error = ();
        fn attacker_unit_id(&mut self) -> Result<ComponentId, ()> {
            Ok(self.attacker)
        }
        fn defender_unit_id(&mut self) -> Result<ComponentId, ()> {
            Ok(self.defender)
        }
        fn has_component_signal(&mut self, _: ComponentId) -> Result<bool, ()> {
            Ok(true)
        }
        fn dispatch_component_created(&mut self, id: ComponentId) -> Result<(), ()> {
            self.dispatched.push(id);
            self.defender.id = 999;
            Ok(())
        }
    }
    #[test]
    fn complete_component_tuple_is_cached_and_type_is_not_ignored() {
        let a = ComponentId {
            owner: 2,
            component_type: 1,
            id: 7,
        };
        let b = ComponentId {
            component_type: 3,
            ..a
        };
        let mut c = Components {
            attacker: a,
            defender: b,
            dispatched: vec![],
        };
        unit_components_created(&mut c).unwrap();
        assert_eq!(c.dispatched, [a, b]);
        assert!(!component_valid(ComponentId { owner: -1, ..a }));
        assert!(!component_valid(ComponentId { id: u32::MAX, ..a }));
        assert!(component_valid(ComponentId {
            component_type: 0,
            ..a
        }));
    }
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
