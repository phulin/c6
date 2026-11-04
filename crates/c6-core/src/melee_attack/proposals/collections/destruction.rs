//! Source-ordered final UnitCombat collection release. The caller holds its
//! owner object across this destructor; no native allocation is emulated.
use super::Kind;
use crate::{
    melee_attack::proposals::registry::{self, Signal, Subscriber},
    unit_moved_signal::RegistryError,
};
#[derive(Debug)]
pub enum Error {
    Registry(registry::Error),
    Ring(RegistryError),
    MissingCollection(u64),
    UnsupportedKind,
    OwnersRemain,
}
/// UnitCombat2343340 unsubscribes its full-component ObjectCreated binding,
/// then SimpleCollection24309e0 clears +40, +28 and releases its owner. The map
/// entry remains even when its listener list becomes empty.
pub fn destroy_unit_combat(
    runtime: &mut registry::Runtime,
    collection: u64,
) -> Result<Kind, Error> {
    let collections = runtime.collections().map_err(Error::Registry)?;
    if collections.retained_owners(collection) != Some(0) {
        return Err(Error::OwnersRemain);
    }
    let kind = collections
        .collection(collection)
        .ok_or(Error::MissingCollection(collection))?
        .kind
        .ok_or(Error::UnsupportedKind)?;
    let Kind::Unit { unit } = kind else {
        return Err(Error::UnsupportedKind);
    };
    let signal = Signal::Unit(unit.component_id());
    let ring = runtime
        .ring_mut(signal)
        .map_err(Error::Registry)?
        .ok_or(Error::Registry(registry::Error::MissingSignal(signal)))?;
    let token = ring
        .subscription_for(|route| route == Subscriber::Callback(collection))
        .map_err(Error::Ring)?;
    ring.unsubscribe(token).map_err(Error::Ring)?;
    let collections = runtime.collections_mut().map_err(Error::Registry)?;
    collections
        .clear_added_subscriptions(collection)
        .map_err(Error::Ring)?;
    collections
        .clear_changed_subscriptions(collection)
        .map_err(Error::Ring)?;
    collections
        .retire_collection(collection)
        .map_err(Error::Ring)
}

#[cfg(test)]
mod tests {
    use super::super::{Collection, Runtime};
    use super::*;
    use crate::{map::UnitReference, unit::ComponentId, unit_moved_signal::ListenerRing};
    fn id(owner: i16) -> ComponentId {
        ComponentId {
            owner,
            component_type: 1,
            id: 7,
        }
    }
    fn fixture(changed: bool) -> registry::Runtime {
        let unit = UnitReference::from_component_id(id(12));
        let mut collections = Runtime::from_initial(vec![(
            77,
            Collection {
                kind: Some(Kind::Unit { unit }),
                added: ListenerRing::from_ordered(&[10, 11]).unwrap(),
            },
        )])
        .unwrap();
        if changed {
            collections
                .bind_changed_signals(vec![(77, ListenerRing::from_ordered(&[12]).unwrap())])
                .unwrap()
        }
        collections.bind_retained_owners(vec![(77, 1)]).unwrap();
        assert!(collections.release_owner(77).unwrap());
        registry::Runtime::from_initial(
            0,
            vec![],
            None,
            None,
            Some(vec![
                (
                    id(12),
                    ListenerRing::from_ordered(&[Subscriber::Callback(77)]).unwrap(),
                ),
                (
                    id(62),
                    ListenerRing::from_ordered(&[Subscriber::Callback(77)]).unwrap(),
                ),
            ]),
        )
        .unwrap()
        .with_collections(collections)
        .unwrap()
    }
    #[test]
    fn last_release_unsubscribes_only_exact_component_and_retires_collection() {
        let mut r = fixture(true);
        assert_eq!(
            destroy_unit_combat(&mut r, 77).unwrap(),
            Kind::Unit {
                unit: UnitReference::from_component_id(id(12))
            }
        );
        assert!(r.collections().unwrap().collection(77).is_none());
        assert!(r.ring(Signal::Unit(id(12))).unwrap().is_some());
        assert!(r
            .ring(Signal::Unit(id(12)))
            .unwrap()
            .unwrap()
            .subscription_for(|_| true)
            .is_err());
        assert!(r
            .ring(Signal::Unit(id(62)))
            .unwrap()
            .unwrap()
            .subscription_for(|_| true)
            .is_ok());
    }
    #[test]
    fn missing_changed_signal_keeps_unsubscribe_and_added_clear_prefix() {
        let mut r = fixture(false);
        assert!(matches!(
            destroy_unit_combat(&mut r, 77),
            Err(Error::Ring(RegistryError::MissingGlobalOrder))
        ));
        assert!(r
            .ring(Signal::Unit(id(12)))
            .unwrap()
            .unwrap()
            .subscription_for(|_| true)
            .is_err());
        assert!(r
            .collections()
            .unwrap()
            .ring(77)
            .unwrap()
            .subscription_for(|_| true)
            .is_err());
        assert!(r.collections().unwrap().collection(77).is_some());
    }
}
