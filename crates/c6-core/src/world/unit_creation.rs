//! Fresh Unit/Formation storage in the existing canonical owner collections.
//! Native Unit Add2baa4f0 and Formation Add2b053a0 share the verified eight-slot,
//! sixteen-bit recycled allocation contract. CompleteSlots lacks that metadata.
use super::*;
use crate::city_initialization::{self, Allocation};
#[derive(Debug)]
pub enum Error {
    World(MissingInput),
    Allocation(city_initialization::Error),
    Identity,
    ResourceLimit,
}
impl<T> Collection<T> {
    fn reserve_creation(&mut self, missing: MissingInput) -> Result<Option<Allocation>, Error> {
        let CollectionRecords::Wire(records) = &mut self.records else {
            return Err(Error::World(missing));
        };
        // Reserve index storage before changing the native allocator prefix.
        let maximum = (records.capacity as usize).saturating_add(8).min(65536);
        self.by_slot
            .try_reserve(maximum.saturating_sub(self.by_slot.len()))
            .map_err(|_| Error::ResourceLimit)?;
        let allocation = city_initialization::allocate(records).map_err(Error::Allocation)?;
        self.by_slot.resize(records.capacity as usize, None);
        Ok(allocation)
    }
    fn insert_creation(
        &mut self,
        allocation: Allocation,
        value: T,
        missing: MissingInput,
    ) -> Result<(), Error> {
        let CollectionRecords::Wire(records) = &mut self.records else {
            return Err(Error::World(missing));
        };
        allocation
            .insert(records, value)
            .map_err(Error::Allocation)?;
        // Sorted insertion can shift existing logical positions. Rebuild only
        // this small index, never clone/reimport the records or entire World.
        self.by_slot.fill(None);
        for (i, row) in records.entries.iter().enumerate() {
            self.by_slot[row.index as usize] = Some(i);
        }
        Ok(())
    }
}
impl World {
    pub fn reserve_unit_creation(&mut self, owner: i32) -> Result<Option<Allocation>, Error> {
        self.player_mut(owner)
            .map_err(Error::World)?
            .units
            .as_mut()
            .ok_or(Error::World(MissingInput::Units { owner }))?
            .reserve_creation(MissingInput::UnitWireMetadata { owner })
    }
    pub fn insert_created_unit(
        &mut self,
        owner: i32,
        allocation: Allocation,
        unit: Unit,
    ) -> Result<(), Error> {
        if unit.e_owner != owner || unit.id != allocation.id {
            return Err(Error::Identity);
        }
        self.player_mut(owner)
            .map_err(Error::World)?
            .units
            .as_mut()
            .ok_or(Error::World(MissingInput::Units { owner }))?
            .insert_creation(allocation, unit, MissingInput::UnitWireMetadata { owner })
    }
    pub fn reserve_formation_creation(&mut self, owner: i32) -> Result<Option<Allocation>, Error> {
        self.player_mut(owner)
            .map_err(Error::World)?
            .formations
            .as_mut()
            .ok_or(Error::World(MissingInput::Formations { owner }))?
            .reserve_creation(MissingInput::FormationWireMetadata { owner })
    }
    pub fn insert_created_formation(
        &mut self,
        owner: i32,
        allocation: Allocation,
        formation: Formation,
    ) -> Result<(), Error> {
        if formation.owner != owner || formation.id != allocation.id {
            return Err(Error::Identity);
        }
        self.player_mut(owner)
            .map_err(Error::World)?
            .formations
            .as_mut()
            .ok_or(Error::World(MissingInput::Formations { owner }))?
            .insert_creation(
                allocation,
                formation,
                MissingInput::FormationWireMetadata { owner },
            )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn creation_updates_sorted_slot_lookup_without_replacing_existing_records() {
        let records = Recycled {
            capacity: 8,
            entries: vec![Slot {
                index: 3,
                value: 40,
            }],
            current_id_bits: 0x20000,
            free_head: 1,
            free_tail: 7,
            free_links: vec![1, 0, 2, 4, 5, 6, 7],
        };
        let mut c = Collection::new(records).unwrap();
        let a = c
            .reserve_creation(MissingInput::UnitWireMetadata { owner: 0 })
            .unwrap()
            .unwrap();
        assert_eq!((a.slot, a.id), (1, 0x30001));
        c.insert_creation(a, 91, MissingInput::UnitWireMetadata { owner: 0 })
            .unwrap();
        assert_eq!(c.get(3), Some(&40));
        assert_eq!(c.get(0x30001), Some(&91));
        assert_eq!(
            c.records
                .entries()
                .iter()
                .map(|r| r.index)
                .collect::<Vec<_>>(),
            [1, 3]
        );
    }
    #[test]
    fn complete_slots_do_not_invent_free_list_or_generation() {
        let mut c = Collection::from_slots(CompleteSlots::<i32> { entries: vec![] }).unwrap();
        assert!(matches!(
            c.reserve_creation(MissingInput::UnitWireMetadata { owner: 2 }),
            Err(Error::World(MissingInput::UnitWireMetadata { owner: 2 }))
        ));
        assert!(c.records.entries().is_empty());
    }
}
