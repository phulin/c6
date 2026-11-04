//! Source-bound SimpleCollection item notifications used by combat collections.
//! AddItem notifies subscribers; it does not append an object to a stored list.
use super::registry::Handle;
use crate::unit_moved_signal::{ListenerRing, RegistryError};

#[derive(Debug)]
pub enum Error<E> {
    Context(E),
    MissingCollection(u64),
    Ring(RegistryError),
}

/// Current collection+0x40 signals. Collections remain allocated during callbacks
/// (native AddItem retains shared_from_this); logical identities are not reused.
#[derive(Debug)]
pub struct Runtime {
    collections: Vec<(u64, Collection)>,
    changed: Option<Vec<(u64, ListenerRing<u64>)>>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Player {
        owner: i32,
    },
    Unit {
        unit: crate::map::UnitReference,
    },
    Alliance {
        first_player: i32,
        second_player: i32,
    },
}
#[derive(Debug)]
pub struct Collection {
    /// Captured source-defined receiver fields, unknown until supplied.
    pub kind: Option<Kind>,
    pub added: ListenerRing<u64>,
}
impl Runtime {
    pub fn from_initial(collections: Vec<(u64, Collection)>) -> Result<Self, u64> {
        for (i, entry) in collections.iter().enumerate() {
            if collections[..i].iter().any(|other| other.0 == entry.0) {
                return Err(entry.0);
            }
        }
        Ok(Self {
            collections,
            changed: None,
        })
    }
    /// The separate SimpleCollection.GetOnChanged signal (+0x28), supplied
    /// from the same current collection owner when a destructor reaches it.
    pub fn with_changed_signals(
        mut self,
        changed: Vec<(u64, ListenerRing<u64>)>,
    ) -> Result<Self, RegistryError> {
        self.bind_changed_signals(changed)?;
        Ok(self)
    }
    pub fn bind_changed_signals(
        &mut self,
        changed: Vec<(u64, ListenerRing<u64>)>,
    ) -> Result<(), RegistryError> {
        if self.changed.is_some()
            || changed.iter().enumerate().any(|(index, row)| {
                self.collection(row.0).is_none()
                    || changed[..index].iter().any(|other| other.0 == row.0)
            })
        {
            return Err(RegistryError::DuplicateScope);
        }
        self.changed = Some(changed);
        Ok(())
    }
    pub fn unsubscribe_changed_receiver(
        &mut self,
        collection: u64,
        receiver: u64,
    ) -> Result<(), RegistryError> {
        let ring = self
            .changed
            .as_mut()
            .and_then(|rows| rows.iter_mut().find(|row| row.0 == collection))
            .map(|row| &mut row.1)
            .ok_or(RegistryError::MissingGlobalOrder)?;
        let subscription = ring.subscription_for(|route| route == receiver)?;
        ring.unsubscribe(subscription)
    }
    pub fn collection(&self, id: u64) -> Option<&Collection> {
        self.collections.iter().find(|v| v.0 == id).map(|v| &v.1)
    }
    pub fn collection_mut(&mut self, id: u64) -> Option<&mut Collection> {
        self.collections
            .iter_mut()
            .find(|v| v.0 == id)
            .map(|v| &mut v.1)
    }
    pub fn ring(&self, collection: u64) -> Option<&ListenerRing<u64>> {
        self.collections
            .iter()
            .find(|v| v.0 == collection)
            .map(|v| &v.1.added)
    }
    pub fn ring_mut(&mut self, collection: u64) -> Option<&mut ListenerRing<u64>> {
        self.collections
            .iter_mut()
            .find(|v| v.0 == collection)
            .map(|v| &mut v.1.added)
    }
    /// Unsubscribe the actual unique DynamicModifier item-change binding. This
    /// uses the same ring as AddItem, including an in-progress held cursor.
    pub fn unsubscribe_added_receiver(
        &mut self,
        collection: u64,
        receiver: u64,
    ) -> Result<(), RegistryError> {
        let ring = self
            .ring_mut(collection)
            .ok_or(RegistryError::MissingGlobalOrder)?;
        let subscription = ring.subscription_for(|current| current == receiver)?;
        ring.unsubscribe(subscription)
    }
}

pub trait Context<Item = Handle> {
    type Error;
    fn collections(&self) -> Result<&Runtime, Self::Error>;
    fn collections_mut(&mut self) -> Result<&mut Runtime, Self::Error>;
    /// The native signal passes a shared_ptr by mutable reference. Replacement
    /// affects following listeners of this AddItem, not the caller's original
    /// shared_ptr copy. Payload mutations affect the one canonical proposal.
    fn item_changed(
        &mut self,
        receiver: u64,
        proposal: &mut Item,
        added: bool,
    ) -> Result<(), Self::Error>;
}

/// PlayerCombat::OnCombatProposed and UnitCombat::OnCombatProposed call this
/// unconditionally. ObjectRelevant is not another filter on that callback path.
pub fn add_item<C: Context>(
    c: &mut C,
    collection: u64,
    proposal: Handle,
) -> Result<(), Error<C::Error>> {
    add_typed_item(c, collection, proposal)
}
/// Same SimpleCollection signal for an actual typed subject (for example a
/// City ModifierObject handle). This does not register or reinterpret an item.
pub fn add_typed_item<Item, C: Context<Item>>(
    c: &mut C,
    collection: u64,
    mut proposal: Item,
) -> Result<(), Error<C::Error>> {
    use Error::{Context as Cx, MissingCollection as Missing, Ring};
    let mut cursor = c
        .collections_mut()
        .map_err(Cx)?
        .ring_mut(collection)
        .ok_or(Missing(collection))?
        .cursor_start()
        .map_err(Ring)?;
    loop {
        let route = c
            .collections()
            .map_err(Cx)?
            .ring(collection)
            .ok_or(Missing(collection))?
            .cursor_route(cursor)
            .map_err(Ring)?;
        if let Some(receiver) = route {
            if let Err(error) = c.item_changed(receiver, &mut proposal, true) {
                c.collections_mut()
                    .map_err(Cx)?
                    .ring_mut(collection)
                    .ok_or(Missing(collection))?
                    .cursor_release(cursor)
                    .map_err(Ring)?;
                return Err(Cx(error));
            }
        }
        match c
            .collections_mut()
            .map_err(Cx)?
            .ring_mut(collection)
            .ok_or(Missing(collection))?
            .cursor_next(cursor)
            .map_err(Ring)?
        {
            Some(next) => cursor = next,
            None => return Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn destructor_unsubscribes_distinct_changed_and_item_signals() {
        let mut runtime = Runtime::from_initial(vec![(
            5,
            Collection {
                kind: None,
                added: ListenerRing::from_ordered(&[7]).unwrap(),
            },
        )])
        .unwrap();
        assert_eq!(
            runtime.unsubscribe_changed_receiver(5, 7),
            Err(RegistryError::MissingGlobalOrder)
        );
        runtime
            .bind_changed_signals(vec![(5, ListenerRing::from_ordered(&[7]).unwrap())])
            .unwrap();
        runtime.unsubscribe_changed_receiver(5, 7).unwrap();
        runtime.unsubscribe_added_receiver(5, 7).unwrap();
        assert_eq!(
            runtime.unsubscribe_changed_receiver(5, 7),
            Err(RegistryError::InvalidSubscription)
        );
    }
    use crate::{
        map::UnitReference,
        melee_attack::proposals::registry::{Proposal, Runtime as Proposals},
    };
    struct Fixture {
        runtime: Runtime,
        calls: Vec<(u64, Handle, bool)>,
        replacement: Handle,
        fail: bool,
    }
    impl Context for Fixture {
        type Error = &'static str;
        fn collections(&self) -> Result<&Runtime, Self::Error> {
            Ok(&self.runtime)
        }
        fn collections_mut(&mut self) -> Result<&mut Runtime, Self::Error> {
            Ok(&mut self.runtime)
        }
        fn item_changed(
            &mut self,
            receiver: u64,
            proposal: &mut Handle,
            added: bool,
        ) -> Result<(), Self::Error> {
            self.calls.push((receiver, *proposal, added));
            if receiver == 10 {
                *proposal = self.replacement;
                self.runtime.ring_mut(7).unwrap().subscribe(30).unwrap();
            }
            if self.fail {
                Err("real consumer unsupported")
            } else {
                Ok(())
            }
        }
    }
    fn fixture() -> (Fixture, Handle) {
        let mut proposals = Proposals::from_initial(0, vec![], None, None, None).unwrap();
        let unit = UnitReference { owner: 12, id: 1 };
        let (_, first) = proposals
            .register(Proposal::melee_units(unit, unit, None))
            .unwrap();
        let (_, replacement) = proposals
            .register(Proposal::melee_units(unit, unit, None))
            .unwrap();
        (
            Fixture {
                runtime: Runtime::from_initial(vec![(
                    7,
                    Collection {
                        kind: None,
                        added: ListenerRing::from_ordered(&[10, 20]).unwrap(),
                    },
                )])
                .unwrap(),
                calls: vec![],
                replacement,
                fail: false,
            },
            first,
        )
    }
    #[test]
    fn replacement_and_reentrant_append_reach_later_listeners() {
        let (mut f, first) = fixture();
        add_item(&mut f, 7, first).unwrap();
        assert_eq!(
            f.calls,
            vec![
                (10, first, true),
                (20, f.replacement, true),
                (30, f.replacement, true)
            ]
        );
    }
    #[test]
    fn missing_collection_is_not_an_empty_signal() {
        let (mut f, first) = fixture();
        assert!(matches!(
            add_item(&mut f, 8, first),
            Err(Error::MissingCollection(8))
        ));
        assert!(f.calls.is_empty());
    }
    #[test]
    fn unsupported_consumer_retains_prior_effects_and_stops_dispatch() {
        let (mut f, first) = fixture();
        f.fail = true;
        assert!(matches!(add_item(&mut f, 7, first), Err(Error::Context(_))));
        assert_eq!(f.calls, vec![(10, first, true)]);
        // The appended listener remains present; dispatch failure does not undo it.
        f.fail = false;
        f.calls.clear();
        let replacement = f.replacement;
        add_item(&mut f, 7, replacement).unwrap();
        assert_eq!(f.calls.len(), 4);
    }
}
