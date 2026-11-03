//! Owned saved PlayerNotifications collections for all 64 Linux Manager slots.
//!
//! Slot position, stored owner and ComponentID are separate identities. This
//! bounded owner provides no native locks, pools, subtype/vtable bindings,
//! default blocking, notification creation, removal or gameplay callbacks.
use crate::notifications::{self, ComponentId, Notification, PlayerNotifications};

pub const PLAYER_SLOTS: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    CollectionCount(usize),
    Slot(i32),
    Record { slot: i32, index: usize },
    Identity { slot: i32, index: usize },
    ResourceLimit,
    Codec(notifications::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "owned PlayerNotifications: {self:?}")
    }
}
impl std::error::Error for Error {}

/// An owner-relative logical list occurrence, not a native pointer or a
/// ComponentID. Duplicate ComponentIDs still designate different occurrences.
/// No structural mutation API exists here, so these ordinals do not compact.
/// Handles must not be transferred to a different Owner.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct ObjectId {
    slot: u8,
    index: usize,
}
impl ObjectId {
    pub fn slot(self) -> i32 {
        i32::from(self.slot)
    }
    pub fn index(self) -> usize {
        self.index
    }
}

/// Complete ordered typed storage, admitted into LoadedState only through its
/// one-time pristine source-witness installer. Constructing this value alone
/// does not authenticate a capture or perform native Manager::Read.
#[derive(Debug, Eq, PartialEq)]
pub struct Owner {
    collections: [PlayerNotifications; PLAYER_SLOTS],
}
impl Owner {
    pub fn from_decoded(collections: Vec<PlayerNotifications>) -> Result<Self, Error> {
        let collections = collections
            .try_into()
            .map_err(|values: Vec<PlayerNotifications>| Error::CollectionCount(values.len()))?;
        // Native Manager::Read selects the vector slot independently of the
        // stored owner. Preserve that field and every raw record ID unchanged.
        Ok(Self { collections })
    }
    pub fn collections(&self) -> &[PlayerNotifications; PLAYER_SLOTS] {
        &self.collections
    }
    fn slot(slot: i32) -> Result<usize, Error> {
        usize::try_from(slot)
            .ok()
            .filter(|slot| *slot < PLAYER_SLOTS)
            .ok_or(Error::Slot(slot))
    }
    pub fn collection(&self, slot: i32) -> Result<&PlayerNotifications, Error> {
        Ok(&self.collections[Self::slot(slot)?])
    }
    pub fn record(&self, slot: i32, index: usize) -> Result<&Notification, Error> {
        self.collection(slot)?
            .records
            .get(index)
            .ok_or(Error::Record { slot, index })
    }
    pub fn object(&self, slot: i32, index: usize) -> Result<ObjectId, Error> {
        self.record(slot, index)?;
        Ok(ObjectId {
            slot: slot as u8,
            index,
        })
    }
    pub fn record_by_object(&self, object: ObjectId) -> Result<&Notification, Error> {
        self.record(object.slot(), object.index)
    }
    /// Source PlayerNotifications::Find(uint), 0x2d30940: scan list order and
    /// compare only the numeric ID word, retaining the first duplicate match.
    /// This is not Manager::Find's separate ComponentID validity/type routing.
    pub fn find_numeric_id(&self, slot: i32, id: u32) -> Result<Option<ObjectId>, Error> {
        Ok(self
            .collection(slot)?
            .records
            .iter()
            .position(|r| r.id.id == id)
            .map(|index| ObjectId {
                slot: slot as u8,
                index,
            }))
    }
    /// Validate an exact current ID before exposing mutable record storage.
    /// LoadedState reserves its revision before delegating to this accessor.
    /// No owner, type or ID word is normalized, inferred or deduplicated.
    pub fn record_mut(
        &mut self,
        slot: i32,
        index: usize,
        expected_id: ComponentId,
    ) -> Result<&mut Notification, Error> {
        if self.record(slot, index)?.id != expected_id {
            return Err(Error::Identity { slot, index });
        }
        Ok(&mut self.collections[Self::slot(slot)?].records[index])
    }
    /// Raw owned counter mutation only. Native Add's assignment, wrapping ID
    /// increment, list insertion and callbacks are not implemented by this setter.
    pub fn set_next_id(&mut self, slot: i32, value: u32) -> Result<(), Error> {
        self.collections[Self::slot(slot)?].next_id = value;
        Ok(())
    }
    /// Validate this supplied typed source against one attributed current
    /// Manager span. Reuses the existing collection encoder solely for exact
    /// preimage comparison; no Core/game save is produced or rewritten.
    ///
    /// Total source bytes and records are bounded before any encoded buffer.
    /// Item/depth accounting retains the existing per-collection codec policy;
    /// this does not claim a new aggregate Manager item/allocation budget.
    pub fn matches_source(
        &self,
        bytes: &[u8],
        limits: notifications::Limits,
    ) -> Result<bool, Error> {
        let records = self.collections.iter().try_fold(0usize, |total, player| {
            total
                .checked_add(player.records.len())
                .ok_or(Error::ResourceLimit)
        })?;
        if bytes.len() > limits.max_bytes || records > limits.max_records {
            return Err(Error::ResourceLimit);
        }
        let mut header = [0u8; 8];
        header[..4].copy_from_slice(&1u32.to_le_bytes());
        header[4..].copy_from_slice(&(PLAYER_SLOTS as u32).to_le_bytes());
        if bytes.get(..8) != Some(header.as_slice()) {
            return Ok(false);
        }
        let mut cursor = 8usize;
        for collection in &self.collections {
            let encoded = collection
                .encode(notifications::Limits {
                    max_bytes: limits
                        .max_bytes
                        .checked_sub(cursor)
                        .ok_or(Error::ResourceLimit)?,
                    ..limits
                })
                .map_err(Error::Codec)?;
            let end = cursor
                .checked_add(encoded.len())
                .ok_or(Error::ResourceLimit)?;
            if bytes.get(cursor..end) != Some(encoded.as_slice()) {
                return Ok(false);
            }
            cursor = end;
        }
        Ok(cursor == bytes.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn collections() -> Vec<PlayerNotifications> {
        (0..PLAYER_SLOTS)
            .map(|slot| PlayerNotifications {
                owner: slot as i32,
                next_id: 0,
                records: vec![],
            })
            .collect()
    }
    fn source(owner: &Owner) -> Vec<u8> {
        let mut bytes = [1u32.to_le_bytes(), 64u32.to_le_bytes()].concat();
        for player in owner.collections() {
            bytes.extend(player.encode(notifications::Limits::default()).unwrap());
        }
        bytes
    }
    #[test]
    fn slots_stored_owners_ids_and_duplicate_occurrences_remain_distinct() {
        let mut players = collections();
        players[0].owner = 63;
        let id = ComponentId {
            owner: 7,
            component_type: 38,
            id: u32::MAX,
        };
        players[0].records = vec![
            Notification {
                id,
                severity: -1,
                ..Notification::default()
            },
            Notification {
                id,
                severity: 3,
                ..Notification::default()
            },
        ];
        let mut owner = Owner::from_decoded(players).unwrap();
        assert_eq!(owner.collection(0).unwrap().owner, 63);
        let first = owner.object(0, 0).unwrap();
        let second = owner.object(0, 1).unwrap();
        assert_ne!(first, second);
        assert_eq!(owner.find_numeric_id(0, u32::MAX), Ok(Some(first)));
        owner.record_mut(0, 1, id).unwrap().has_been_activated = true;
        assert!(!owner.record_by_object(first).unwrap().has_been_activated);
        assert!(owner.record_by_object(second).unwrap().has_been_activated);
        assert_eq!(owner.record(0, 1).unwrap().id, id);
        assert!(matches!(owner.collection(-1), Err(Error::Slot(-1))));
        assert!(matches!(owner.collection(64), Err(Error::Slot(64))));
        assert!(matches!(owner.record(63, 0), Err(Error::Record { .. })));
    }
    #[test]
    fn exact_identity_failure_precedes_mutation_and_counter_is_owned() {
        let mut players = collections();
        let record = Notification {
            id: ComponentId {
                owner: 0,
                component_type: 38,
                id: 5,
            },
            ..Notification::default()
        };
        players[0].records.push(record.clone());
        let mut owner = Owner::from_decoded(players).unwrap();
        for wrong in [
            ComponentId {
                owner: 1,
                ..record.id
            },
            ComponentId {
                component_type: 1,
                ..record.id
            },
            ComponentId { id: 6, ..record.id },
        ] {
            assert!(matches!(
                owner.record_mut(0, 0, wrong),
                Err(Error::Identity { .. })
            ));
            assert_eq!(owner.record(0, 0).unwrap(), &record);
        }
        owner.set_next_id(0, u32::MAX).unwrap();
        assert_eq!(owner.collection(0).unwrap().next_id, u32::MAX);
        assert!(owner.set_next_id(64, 0).is_err());
        assert_eq!(owner.collection(0).unwrap().next_id, u32::MAX);
        assert!(matches!(
            Owner::from_decoded(collections().into_iter().take(63).collect()),
            Err(Error::CollectionCount(63))
        ));
    }
    #[test]
    fn source_match_rejects_wrong_shape_trailing_bytes_and_total_budgets() {
        let mut players = collections();
        players[0].records.push(Notification::default());
        players[63].records.push(Notification::default());
        let owner = Owner::from_decoded(players).unwrap();
        let bytes = source(&owner);
        assert!(owner
            .matches_source(&bytes, notifications::Limits::default())
            .unwrap());
        for index in [0, 4, 12] {
            let mut wrong = bytes.clone();
            wrong[index] ^= 1;
            assert!(!owner
                .matches_source(&wrong, notifications::Limits::default())
                .unwrap());
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(!owner
            .matches_source(&trailing, notifications::Limits::default())
            .unwrap());
        assert!(matches!(
            owner.matches_source(
                &bytes,
                notifications::Limits {
                    max_records: 1,
                    ..notifications::Limits::default()
                }
            ),
            Err(Error::ResourceLimit)
        ));
        assert!(matches!(
            owner.matches_source(
                &bytes,
                notifications::Limits {
                    max_bytes: bytes.len() - 1,
                    ..notifications::Limits::default()
                }
            ),
            Err(Error::ResourceLimit)
        ));
    }
}
