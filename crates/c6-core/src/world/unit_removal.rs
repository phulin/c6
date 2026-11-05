//! Storage step AFTER native-equivalent Unit/Formation destructor effects.
//! No removal callbacks are implied by these methods. Recycled metadata remains
//! authoritative, including LIFO free order and unchanged generation bits.
use super::*;
use crate::{city_initialization, world::unit_creation::Error};
impl<T> Collection<T> {
    fn release_recycled_slot(
        &mut self,
        id: u32,
        missing: MissingInput,
    ) -> Result<Option<T>, Error> {
        let CollectionRecords::Wire(records) = &mut self.records else {
            return Err(Error::World(missing));
        };
        let removed =
            city_initialization::release_slot(records, id & 0xffff).map_err(Error::Allocation)?;
        if removed.is_some() {
            self.by_slot.fill(None);
            for (at, row) in records.entries.iter().enumerate() {
                self.by_slot[row.index as usize] = Some(at);
            }
        }
        Ok(removed)
    }
}
impl World {
    /// Recycled Remove(unsigned) masks to low16 exactly. Callers retain the
    /// actual Unit pointer/identity across all preceding destructor effects.
    pub fn release_unit_allocation(&mut self, owner: i32, id: u32) -> Result<Option<Unit>, Error> {
        self.player_mut(owner)
            .map_err(Error::World)?
            .units
            .as_mut()
            .ok_or(Error::World(MissingInput::Units { owner }))?
            .release_recycled_slot(id, MissingInput::UnitWireMetadata { owner })
    }
    pub fn release_formation_allocation(
        &mut self,
        owner: i32,
        id: u32,
    ) -> Result<Option<Formation>, Error> {
        self.player_mut(owner)
            .map_err(Error::World)?
            .formations
            .as_mut()
            .ok_or(Error::World(MissingInput::Formations { owner }))?
            .release_recycled_slot(id, MissingInput::FormationWireMetadata { owner })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_reindexes_shifted_live_slots_and_retains_native_reuse_order() {
        let records = Recycled {
            capacity: 8,
            entries: vec![
                Slot {
                    index: 0,
                    value: 10,
                },
                Slot {
                    index: 3,
                    value: 30,
                },
            ],
            current_id_bits: 0x20000,
            free_head: 1,
            free_tail: 7,
            free_links: vec![1, 2, 4, 5, 6, 7],
        };
        let mut c = Collection::new(records).unwrap();
        assert_eq!(
            c.release_recycled_slot(0x10000, MissingInput::UnitWireMetadata { owner: 0 })
                .unwrap(),
            Some(10)
        );
        assert_eq!(c.get(0), None);
        assert_eq!(c.get(3), Some(&30));
        let CollectionRecords::Wire(records) = &c.records else {
            unreachable!()
        };
        assert_eq!(records.free_links, vec![0, 1, 2, 4, 5, 6, 7]);
        assert_eq!(records.current_id_bits, 0x20000);
    }
}
