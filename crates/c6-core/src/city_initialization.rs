//! Native recycled City/District allocation used by live creation.
//! This preserves slot/free order and wrapping generated IDs. It does not
//! initialize a City or dispatch its creation effects by itself.
use crate::map::{Recycled, Slot};
pub mod constructor;
pub use constructor::{BuildingSupport, Definitions, Parameters};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Allocator,
    ResourceLimit,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "City initialization: {self:?}")
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Allocation {
    pub slot: u32,
    pub id: u32,
}

/// Native City Add2a5c640/AddBlock2a5d560 (eight slots, sixteen ID bits).
/// The wire free_links vector contains free indices in linked-list order,
/// as written by City serializer2a5b840, not a capacity-indexed next array.
/// Insert the constructed record before another allocation/query of this
/// collection. On a constructor failure the caller must expose the prefix.
pub fn allocate<T>(records: &mut Recycled<T>) -> Result<Option<Allocation>, Error> {
    validate(records)?;
    if records.free_links.is_empty() {
        if records.capacity == 65536 {
            return Ok(None);
        }
        records
            .free_links
            .try_reserve(8)
            .map_err(|_| Error::ResourceLimit)?;
        records
            .entries
            .try_reserve(1)
            .map_err(|_| Error::ResourceLimit)?;
        let base = records.capacity;
        records.free_links.extend(base..base + 8);
        records.capacity += 8;
        records.free_head = base;
        records.free_tail = base + 7;
    } else {
        records
            .entries
            .try_reserve(1)
            .map_err(|_| Error::ResourceLimit)?;
    }
    let slot = records.free_links.remove(0);
    records.free_head = records.free_links.first().copied().unwrap_or(u32::MAX);
    if records.free_links.is_empty() {
        records.free_tail = u32::MAX;
    }
    records.current_id_bits = records.current_id_bits.wrapping_add(0x10000);
    Ok(Some(Allocation {
        slot,
        id: records.current_id_bits | slot,
    }))
}

/// Native Unit Remove2baa8e0/Formation Remove: destructors run BEFORE this
/// storage step. Freed indices are PREPENDED to the free list; generation
/// bits remain unchanged until the next Add. Callers preserve destructor order.
pub fn release_slot<T>(records: &mut Recycled<T>, slot: u32) -> Result<Option<T>, Error> {
    validate(records)?;
    let Some(at) = records
        .entries
        .iter()
        .position(|row| row.index as u32 == slot)
    else {
        return Ok(None);
    };
    records
        .free_links
        .try_reserve(1)
        .map_err(|_| Error::ResourceLimit)?;
    let value = records.entries.remove(at).value;
    records.free_links.insert(0, slot);
    records.free_head = slot;
    if records.free_tail == u32::MAX {
        records.free_tail = slot;
    }
    Ok(Some(value))
}

impl Allocation {
    pub fn insert<T>(self, records: &mut Recycled<T>, value: T) -> Result<(), Error> {
        if self.slot >= records.capacity
            || self.id != records.current_id_bits | self.slot
            || records.free_links.contains(&self.slot)
            || records
                .entries
                .iter()
                .any(|entry| entry.index as u32 == self.slot)
        {
            return Err(Error::Allocator);
        }
        let at = records
            .entries
            .partition_point(|entry| entry.index < self.slot as i32);
        records.entries.insert(
            at,
            Slot {
                index: self.slot as i32,
                value,
            },
        );
        Ok(())
    }
}

fn validate<T>(records: &Recycled<T>) -> Result<(), Error> {
    let capacity = records.capacity as usize;
    if capacity > 65536
        || !capacity.is_multiple_of(8)
        || records.current_id_bits & 0xffff != 0
        || records.entries.len() + records.free_links.len() != capacity
        || records.free_head != records.free_links.first().copied().unwrap_or(u32::MAX)
        || records.free_tail != records.free_links.last().copied().unwrap_or(u32::MAX)
    {
        return Err(Error::Allocator);
    }
    let mut occupied = vec![false; capacity];
    for (n, entry) in records.entries.iter().enumerate() {
        if entry.index < 0
            || entry.index as usize >= capacity
            || n > 0 && records.entries[n - 1].index >= entry.index
        {
            return Err(Error::Allocator);
        }
        occupied[entry.index as usize] = true;
    }
    for slot in &records.free_links {
        let used = occupied.get_mut(*slot as usize).ok_or(Error::Allocator)?;
        if *used {
            return Err(Error::Allocator);
        }
        *used = true;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn empty() -> Recycled<u32> {
        Recycled {
            capacity: 0,
            entries: vec![],
            current_id_bits: 0,
            free_head: u32::MAX,
            free_tail: u32::MAX,
            free_links: vec![],
        }
    }
    #[test]
    fn first_block_and_next_block_keep_native_generated_ids() {
        let mut records = empty();
        for n in 0..9u32 {
            let value = allocate(&mut records).unwrap().unwrap();
            assert_eq!(
                value,
                Allocation {
                    slot: n,
                    id: ((n + 1) << 16) | n
                }
            );
            value.insert(&mut records, n).unwrap();
        }
        assert_eq!(records.capacity, 16);
        assert_eq!(records.free_links, (9..16).collect::<Vec<_>>());
    }
    #[test]
    fn recycled_order_and_generation_wrap_are_preserved() {
        let mut records = Recycled {
            capacity: 8,
            entries: vec![Slot { index: 0, value: 0 }],
            current_id_bits: 0xffff0000,
            free_head: 5,
            free_tail: 7,
            free_links: vec![5, 3, 1, 6, 2, 4, 7],
        };
        let first = allocate(&mut records).unwrap().unwrap();
        assert_eq!(first, Allocation { slot: 5, id: 5 });
        first.insert(&mut records, 5).unwrap();
        let next = allocate(&mut records).unwrap().unwrap();
        assert_eq!(
            next,
            Allocation {
                slot: 3,
                id: 0x10003
            }
        );
        next.insert(&mut records, 3).unwrap();
        assert_eq!(
            records.entries.iter().map(|v| v.index).collect::<Vec<_>>(),
            [0, 3, 5]
        );
        assert_eq!(records.free_links, [1, 6, 2, 4, 7]);
    }
    #[test]
    fn invalid_metadata_cannot_silently_repair_or_assign_an_id() {
        let mut records = empty();
        records.free_head = 0;
        assert_eq!(allocate(&mut records), Err(Error::Allocator));
        assert_eq!(records.capacity, 0);
        assert_eq!(records.current_id_bits, 0);
    }
}

/// Current creation effects reached by the native constructor/Initialize.
/// Implementations register the actual logical subscribers and call the
/// concrete CityBuild initializer on this same exclusively owned state.
pub trait Context {
    type Error;
    fn state(&self) -> &crate::loaded_state::LoadedState;
    fn state_mut(&mut self) -> &mut crate::loaded_state::LoadedState;
    fn subscribe_city_signals(&mut self, city: crate::unit::ComponentId)
        -> Result<(), Self::Error>;
    fn initialize_city_build(
        &mut self,
        city: crate::unit::ComponentId,
    ) -> Result<Option<u32>, Self::Error>;
}
#[derive(Debug)]
pub enum CreationError<E> {
    State(crate::loaded_state::Error),
    Initialization(Error),
    Effect(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for CreationError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "City creation prefix: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for CreationError<E> {}
/// Player.Cities.Create2a58e10, ending before placement/name/CityCreated.
/// Signal registration does not invoke the registered callbacks. The canonical
/// state assignments preceding CityBuild are installed together, then all
/// synchronous initialization effects observe that same owned City.
pub fn create<C: Context>(
    context: &mut C,
    definitions: &Definitions<'_>,
    owner: i32,
    parameters: Parameters,
) -> Result<Option<crate::unit::ComponentId>, CreationError<C::Error>> {
    let Some(city) = context
        .state_mut()
        .begin_city_creation(definitions, owner, parameters)
        .map_err(|error| match error {
            CreationError::State(error) => CreationError::State(error),
            CreationError::Initialization(error) => CreationError::Initialization(error),
            CreationError::Effect(never) => match never {},
        })?
    else {
        return Ok(None);
    };
    context
        .subscribe_city_signals(city)
        .map_err(CreationError::Effect)?;
    let city_ai_id = context
        .initialize_city_build(city)
        .map_err(CreationError::Effect)?;
    context
        .state_mut()
        .city_mut(owner, city.id)
        .map_err(CreationError::State)?
        .header
        .city_ai_id = city_ai_id.unwrap_or(u32::MAX);
    // The original current-turn getter is lazy and follows CityBuild effects.
    let founded_turn = if parameters.founded_turn >= 0 {
        parameters.founded_turn
    } else {
        context.state().retained().core.current_turn
    };
    let current = context
        .state_mut()
        .city_mut(owner, city.id)
        .map_err(CreationError::State)?;
    current.turn_city_founded = founded_turn;
    current.header.last_warmonger_penalty.fill(0);
    Ok(Some(city))
}

#[cfg(test)]
mod release_tests {
    use super::*;
    #[test]
    fn release_prepends_free_slots_and_preserves_generation_until_add() {
        let mut records = Recycled {
            capacity: 8,
            entries: vec![
                Slot {
                    index: 0,
                    value: 10,
                },
                Slot {
                    index: 1,
                    value: 11,
                },
            ],
            current_id_bits: 0x20000,
            free_head: 2,
            free_tail: 7,
            free_links: (2..8).collect(),
        };
        assert_eq!(release_slot(&mut records, 0).unwrap(), Some(10));
        assert_eq!(release_slot(&mut records, 1).unwrap(), Some(11));
        assert_eq!(records.free_links, vec![1, 0, 2, 3, 4, 5, 6, 7]);
        assert_eq!(
            (
                records.free_head,
                records.free_tail,
                records.current_id_bits
            ),
            (1, 7, 0x20000)
        );
        let next = allocate(&mut records).unwrap().unwrap();
        assert_eq!((next.slot, next.id), (1, 0x30001));
    }
    #[test]
    fn releasing_last_allocated_slot_into_empty_free_list_sets_both_ends() {
        let mut records = Recycled {
            capacity: 8,
            entries: (0..8).map(|i| Slot { index: i, value: i }).collect(),
            current_id_bits: 0x80000,
            free_head: u32::MAX,
            free_tail: u32::MAX,
            free_links: vec![],
        };
        assert_eq!(release_slot(&mut records, 3).unwrap(), Some(3));
        assert_eq!((records.free_head, records.free_tail), (3, 3));
        let before = records.clone();
        assert_eq!(release_slot(&mut records, 3).unwrap(), None);
        assert_eq!(records, before);
    }
}
