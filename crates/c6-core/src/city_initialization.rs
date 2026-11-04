//! Native recycled City/District allocation used by live creation.
//! This preserves slot/free order and wrapping generated IDs. It does not
//! initialize a City or dispatch its creation effects by itself.
use crate::map::{Recycled, Slot};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Allocator,
    ResourceLimit,
}
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
