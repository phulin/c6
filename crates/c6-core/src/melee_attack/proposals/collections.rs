//! Source-bound SimpleCollection item notifications used by combat collections.
//! AddItem notifies subscribers; it does not append an object to a stored list.
pub mod destruction;

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
    retained_owners: Option<Vec<(u64, u64)>>,
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
            retained_owners: None,
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
    /// Current shared lifetime owners, needed only when a modifier release can
    /// destroy a collection and unsubscribe its gameplay listener.
    pub fn bind_retained_owners(&mut self, owners: Vec<(u64, u64)>) -> Result<(), RegistryError> {
        if self.retained_owners.is_some()
            || owners.iter().enumerate().any(|(index, row)| {
                row.1 == 0
                    || self.collection(row.0).is_none()
                    || owners[..index].iter().any(|other| other.0 == row.0)
            })
        {
            return Err(RegistryError::DuplicateScope);
        }
        self.retained_owners = Some(owners);
        Ok(())
    }
    /// Returns true exactly when the caller must execute the concrete derived
    /// collection destructor. It does not silently erase that callback owner.
    pub fn release_owner(&mut self, collection: u64) -> Result<bool, RegistryError> {
        let count = self
            .retained_owners
            .as_mut()
            .and_then(|rows| rows.iter_mut().find(|row| row.0 == collection))
            .map(|row| &mut row.1)
            .ok_or(RegistryError::MissingGlobalOrder)?;
        *count = count
            .checked_sub(1)
            .ok_or(RegistryError::InvalidSubscription)?;
        Ok(*count == 0)
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
    pub fn retained_owners(&self, collection: u64) -> Option<u64> {
        self.retained_owners
            .as_ref()?
            .iter()
            .find(|r| r.0 == collection)
            .map(|r| r.1)
    }
    /// Native SimpleCollection destroys +40 before +28. Keep these separate
    /// so a missing second signal retains the already-completed first clear.
    pub fn clear_added_subscriptions(&mut self, collection: u64) -> Result<(), RegistryError> {
        self.ring_mut(collection)
            .ok_or(RegistryError::MissingGlobalOrder)?
            .clear_subscriptions()
    }
    pub fn clear_changed_subscriptions(&mut self, collection: u64) -> Result<(), RegistryError> {
        self.changed
            .as_mut()
            .and_then(|v| v.iter_mut().find(|r| r.0 == collection))
            .ok_or(RegistryError::MissingGlobalOrder)?
            .1
            .clear_subscriptions()
    }
    /// After both signal destructors, drop the collection's logical owner edge.
    /// The caller retains the owner object throughout this destructor.
    pub fn retire_collection(&mut self, collection: u64) -> Result<Kind, RegistryError> {
        if self.retained_owners(collection) != Some(0) {
            return Err(RegistryError::InvalidSubscription);
        }
        let index = self
            .collections
            .iter()
            .position(|r| r.0 == collection)
            .ok_or(RegistryError::MissingGlobalOrder)?;
        let kind = self.collections[index]
            .1
            .kind
            .ok_or(RegistryError::MissingGlobalOrder)?;
        self.collections.remove(index);
        if let Some(rows) = &mut self.changed {
            rows.retain(|r| r.0 != collection)
        }
        if let Some(rows) = &mut self.retained_owners {
            rows.retain(|r| r.0 != collection)
        }
        Ok(kind)
    }
    pub fn collection(&self, id: u64) -> Option<&Collection> {
        self.collections.iter().find(|v| v.0 == id).map(|v| &v.1)
    }
    /// Bind additional existing collections into this same current owner. This
    /// allocates no gameplay IDs and leaves existing held cursors untouched.
    pub fn import_current(&mut self, collections: Vec<(u64, Collection)>) -> Result<(), u64> {
        for (index, row) in collections.iter().enumerate() {
            if self.collection(row.0).is_some()
                || collections[..index].iter().any(|other| other.0 == row.0)
            {
                return Err(row.0);
            }
        }
        self.collections.extend(collections);
        Ok(())
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
    proposal: Item,
) -> Result<(), Error<C::Error>> {
    dispatch_item(c, collection, proposal, true)
}
/// Linux SimpleCollection.RemoveItem (0x2327300) dispatches the same live
/// signal with false. There is no stored collection-item list to erase.
pub fn remove_typed_item<Item, C: Context<Item>>(
    c: &mut C,
    collection: u64,
    item: Item,
) -> Result<(), Error<C::Error>> {
    dispatch_item(c, collection, item, false)
}
fn dispatch_item<Item, C: Context<Item>>(
    c: &mut C,
    collection: u64,
    mut proposal: Item,
    added: bool,
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
            if let Err(error) = c.item_changed(receiver, &mut proposal, added) {
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
        runtime.bind_retained_owners(vec![(5, 3)]).unwrap();
        assert!(!runtime.release_owner(5).unwrap());
        assert!(!runtime.release_owner(5).unwrap());
        assert!(runtime.release_owner(5).unwrap());
        assert!(runtime.collection(5).is_some());
        assert_eq!(
            runtime.release_owner(5),
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
    fn removal_preserves_false_replacement_and_reentrant_append() {
        let (mut f, first) = fixture();
        remove_typed_item(&mut f, 7, first).unwrap();
        assert_eq!(
            f.calls,
            vec![
                (10, first, false),
                (20, f.replacement, false),
                (30, f.replacement, false)
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
