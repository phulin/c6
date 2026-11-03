//! Owned saved PlayerNotifications collections for all 64 Linux Manager slots.
//!
//! Slot position, stored owner and ComponentID are separate identities. This
//! bounded owner retains explicit occurrence identity and supplied runtime data.
//! It does not supply native factories, release/destruction or gameplay callbacks.
use crate::notifications::{self, ComponentId, Notification, PlayerNotifications};

pub const PLAYER_SLOTS: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    CollectionCount(usize),
    Slot(i32),
    Record { slot: i32, index: usize },
    Identity { slot: i32, index: usize },
    ResourceLimit,
    StaleObject(ObjectId),
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
/// The index is an immutable occurrence serial, initially the decoded ordinal.
/// It never compacts or gets reused after removal; use Owner::ordinal_of for the
/// current list ordinal. A pooled logical object can have multiple successive
/// occurrences, each with its own handle.
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

/// Runtime fields omitted by Instance v5. `None` means unavailable, never a
/// constructor default. Providers must establish actual method bindings and
/// current values before invoking gameplay. Live object identity/pool binding
/// are separate from this owner's list-occurrence identity.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Runtime {
    pub live: Option<crate::notification_lifecycle::LiveNotification>,
    pub default_blocking: Option<i32>,
    pub group_hash: Option<u32>,
    pub vtable_addresspoint: Option<u64>,
}

/// Explicit diagnostic storage limits, including released occurrence history.
#[derive(Clone, Copy, Debug)]
pub struct OccurrenceLimits {
    pub max_records: usize,
    pub max_occurrences: usize,
}
impl Default for OccurrenceLimits {
    fn default() -> Self {
        Self {
            max_records: 100_000,
            max_occurrences: 1_000_000,
        }
    }
}
#[derive(Debug, Eq, PartialEq)]
struct Occurrence {
    ordinal: Option<usize>,
    runtime: Runtime,
}
#[derive(Debug, Eq, PartialEq)]
struct SlotOccurrences {
    ordered: Vec<usize>,
    history: Vec<Occurrence>,
}
/// Detached storage, not proof of a native Release, destructor or pool append.
#[derive(Debug, Eq, PartialEq)]
pub struct Removed {
    pub record: Notification,
    pub runtime: Runtime,
}

/// Complete ordered typed storage, admitted into LoadedState only through its
/// one-time pristine source-witness installer. Constructing this value alone
/// does not authenticate a capture or perform native Manager::Read.
#[derive(Debug, Eq, PartialEq)]
pub struct Owner {
    collections: [PlayerNotifications; PLAYER_SLOTS],
    occurrences: [SlotOccurrences; PLAYER_SLOTS],
    record_count: usize,
    occurrence_count: usize,
}
impl Owner {
    pub fn from_decoded(collections: Vec<PlayerNotifications>) -> Result<Self, Error> {
        Self::from_decoded_with_limits(collections, OccurrenceLimits::default())
    }
    pub fn from_decoded_with_limits(
        collections: Vec<PlayerNotifications>,
        limits: OccurrenceLimits,
    ) -> Result<Self, Error> {
        let collections: [PlayerNotifications; PLAYER_SLOTS] = collections
            .try_into()
            .map_err(|values: Vec<PlayerNotifications>| Error::CollectionCount(values.len()))?;
        // Native Manager::Read selects the vector slot independently of the
        // stored owner. Preserve that field and every raw record ID unchanged.
        let total = collections.iter().try_fold(0usize, |total, collection| {
            total
                .checked_add(collection.records.len())
                .ok_or(Error::ResourceLimit)
        })?;
        if total > limits.max_records || total > limits.max_occurrences {
            return Err(Error::ResourceLimit);
        }
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(PLAYER_SLOTS)
            .map_err(|_| Error::ResourceLimit)?;
        let mut record_count = 0usize;
        for collection in &collections {
            let n = collection.records.len();
            record_count = record_count.checked_add(n).ok_or(Error::ResourceLimit)?;
            let mut ordered = Vec::new();
            let mut history = Vec::new();
            ordered
                .try_reserve_exact(n)
                .map_err(|_| Error::ResourceLimit)?;
            history
                .try_reserve_exact(n)
                .map_err(|_| Error::ResourceLimit)?;
            for ordinal in 0..n {
                ordered.push(ordinal);
                history.push(Occurrence {
                    ordinal: Some(ordinal),
                    runtime: Runtime::default(),
                });
            }
            slots.push(SlotOccurrences { ordered, history });
        }
        let occurrences = slots.try_into().map_err(|_| Error::ResourceLimit)?;
        Ok(Self {
            collections,
            occurrences,
            record_count,
            occurrence_count: record_count,
        })
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
            index: self.occurrences[slot as usize].ordered[index],
        })
    }
    pub fn record_by_object(&self, object: ObjectId) -> Result<&Notification, Error> {
        self.record(object.slot(), self.ordinal_of(object)?)
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
                index: self.occurrences[slot as usize].ordered[index],
            }))
    }
    /// O(1) current list lookup; released and never-issued handles reject.
    pub fn ordinal_of(&self, object: ObjectId) -> Result<usize, Error> {
        self.occurrences[object.slot as usize]
            .history
            .get(object.index)
            .and_then(|entry| entry.ordinal)
            .ok_or(Error::StaleObject(object))
    }
    pub fn runtime(&self, object: ObjectId) -> Result<&Runtime, Error> {
        self.ordinal_of(object)?;
        Ok(&self.occurrences[object.slot as usize].history[object.index].runtime)
    }
    /// Explicit raw runtime mutation; assigning a value does not authenticate it.
    pub fn runtime_mut(&mut self, object: ObjectId) -> Result<&mut Runtime, Error> {
        self.ordinal_of(object)?;
        Ok(&mut self.occurrences[object.slot as usize].history[object.index].runtime)
    }
    /// Append supplied initialized storage in list order after fallible capacity
    /// checks. This is NOT native Add: its caller must perform source-ordered
    /// added-turn/ID writes and all required synchronous post-append effects.
    pub fn append_initialized(
        &mut self,
        slot: i32,
        record: Notification,
        runtime: Runtime,
        limits: OccurrenceLimits,
    ) -> Result<ObjectId, Error> {
        let slot_index = Self::slot(slot)?;
        if self.record_count >= limits.max_records
            || self.occurrence_count >= limits.max_occurrences
        {
            return Err(Error::ResourceLimit);
        }
        let records = &mut self.collections[slot_index].records;
        let entries = &mut self.occurrences[slot_index];
        records.try_reserve(1).map_err(|_| Error::ResourceLimit)?;
        entries
            .ordered
            .try_reserve(1)
            .map_err(|_| Error::ResourceLimit)?;
        entries
            .history
            .try_reserve(1)
            .map_err(|_| Error::ResourceLimit)?;
        let index = entries.history.len();
        entries.history.push(Occurrence {
            ordinal: Some(records.len()),
            runtime,
        });
        entries.ordered.push(index);
        records.push(record);
        self.record_count += 1;
        self.occurrence_count += 1;
        Ok(ObjectId {
            slot: slot as u8,
            index,
        })
    }
    /// Erase one occurrence while retaining survivor order and next ID. The
    /// caller must execute actual Release before erasure, retaining its effects
    /// on failure. Returning storage here performs no native destruction.
    pub fn remove(&mut self, object: ObjectId) -> Result<Removed, Error> {
        let ordinal = self.ordinal_of(object)?;
        let slot = object.slot as usize;
        let entries = &mut self.occurrences[slot];
        entries.history[object.index].ordinal = None;
        let runtime = std::mem::take(&mut entries.history[object.index].runtime);
        entries.ordered.remove(ordinal);
        for (position, serial) in entries.ordered.iter().enumerate().skip(ordinal) {
            entries.history[*serial].ordinal = Some(position);
        }
        self.record_count -= 1;
        Ok(Removed {
            record: self.collections[slot].records.remove(ordinal),
            runtime,
        })
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
    fn removal_preserves_duplicates_handles_and_reuse_has_a_fresh_occurrence() {
        let mut players = collections();
        let duplicate = Notification {
            id: ComponentId {
                owner: 9,
                component_type: 38,
                id: 5,
            },
            ..Notification::default()
        };
        players[0].records = vec![duplicate.clone(), duplicate.clone(), duplicate.clone()];
        players[0].next_id = 91;
        let mut owner = Owner::from_decoded(players).unwrap();
        let first = owner.object(0, 0).unwrap();
        let second = owner.object(0, 1).unwrap();
        let third = owner.object(0, 2).unwrap();
        assert_eq!(owner.runtime(second).unwrap(), &Runtime::default());
        let supplied = Runtime {
            default_blocking: Some(-7),
            group_hash: Some(19),
            ..Runtime::default()
        };
        *owner.runtime_mut(first).unwrap() = supplied.clone();
        let detached = owner.remove(first).unwrap();
        assert_eq!(detached.runtime, supplied);
        assert!(matches!(
            owner.record_by_object(first),
            Err(Error::StaleObject(_))
        ));
        assert!(matches!(owner.remove(first), Err(Error::StaleObject(_))));
        assert_eq!(owner.ordinal_of(second), Ok(0));
        assert_eq!(owner.object(0, 1), Ok(third));
        assert_eq!(owner.find_numeric_id(0, 5), Ok(Some(second)));
        let reused = owner
            .append_initialized(
                0,
                detached.record,
                detached.runtime,
                OccurrenceLimits::default(),
            )
            .unwrap();
        assert_ne!(reused, first);
        assert_eq!(owner.ordinal_of(reused), Ok(2));
        assert_eq!(owner.ordinal_of(third), Ok(1));
        assert_eq!(owner.runtime(reused).unwrap().default_blocking, Some(-7));
        assert_eq!(owner.collection(0).unwrap().next_id, 91);
        assert_eq!(
            owner.collection(0).unwrap().records,
            vec![duplicate.clone(), duplicate.clone(), duplicate]
        );
        // Indexed mutation still uses the current list ordinal, never a serial.
        owner
            .record_mut(0, 0, owner.record(0, 0).unwrap().id)
            .unwrap()
            .severity = 4;
        assert_eq!(owner.record_by_object(second).unwrap().severity, 4);
        assert_eq!(owner.record_by_object(third).unwrap().severity, 0);
    }
    #[test]
    fn occurrence_history_and_active_record_limits_are_independent() {
        let mut owner = Owner::from_decoded(collections()).unwrap();
        let limits = OccurrenceLimits {
            max_records: 1,
            max_occurrences: 2,
        };
        let first = owner
            .append_initialized(63, Notification::default(), Runtime::default(), limits)
            .unwrap();
        assert!(matches!(
            owner.append_initialized(63, Notification::default(), Runtime::default(), limits),
            Err(Error::ResourceLimit)
        ));
        assert_eq!(owner.ordinal_of(first), Ok(0));
        owner.remove(first).unwrap();
        let second = owner
            .append_initialized(63, Notification::default(), Runtime::default(), limits)
            .unwrap();
        owner.remove(second).unwrap();
        assert!(matches!(
            owner.append_initialized(0, Notification::default(), Runtime::default(), limits),
            Err(Error::ResourceLimit)
        ));
        assert!(owner.collection(0).unwrap().records.is_empty());
        assert!(owner.collection(63).unwrap().records.is_empty());
        let mut players = collections();
        players[63].records.push(Notification::default());
        assert!(matches!(
            Owner::from_decoded_with_limits(
                players,
                OccurrenceLimits {
                    max_records: 0,
                    max_occurrences: 2
                }
            ),
            Err(Error::ResourceLimit)
        ));
    }
    #[test]
    fn callback_failure_retains_committed_storage_prefix() {
        // A required synchronous callback runs against the same owner after the
        // storage mutation. Its failure must not roll back the committed prefix.
        fn append_then_callback(
            owner: &mut Owner,
            callback: impl FnOnce(&mut Owner, ObjectId) -> Result<(), &'static str>,
        ) -> Result<(), &'static str> {
            let object = owner
                .append_initialized(
                    0,
                    Notification::default(),
                    Runtime::default(),
                    OccurrenceLimits::default(),
                )
                .unwrap();
            callback(owner, object)
        }
        let mut owner = Owner::from_decoded(collections()).unwrap();
        assert_eq!(
            append_then_callback(&mut owner, |owner, object| {
                assert_eq!(owner.find_numeric_id(0, 0), Ok(Some(object)));
                owner.runtime_mut(object).unwrap().default_blocking = Some(22);
                Err("required subscriber failed")
            }),
            Err("required subscriber failed")
        );
        assert_eq!(owner.collection(0).unwrap().records.len(), 1);
        assert_eq!(
            owner
                .runtime(owner.object(0, 0).unwrap())
                .unwrap()
                .default_blocking,
            Some(22)
        );
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
