//! Canonical Player.Districts access for source-derived creation/completion.
use super::{Error, LoadedState, MissingInput};
use crate::player_districts::{District, Districts};
#[derive(Debug)]
pub enum DistrictError {
    State(Error),
    MissingDistrict { owner: i32, id: u32 },
}
impl std::fmt::Display for DistrictError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "canonical District: {self:?}")
    }
}
impl std::error::Error for DistrictError {}
impl LoadedState {
    /// Bind an unsaved District+b0 field from the same initial native state.
    /// This is fixture initialization, not a gameplay mutation; preserve the
    /// pristine revision needed by the other initial runtime projections.
    pub fn initialize_district_marked_for_delete(
        &mut self,
        binding: &crate::core_player::initialization::WireBinding,
        owner: i32,
        id: u32,
        value: bool,
    ) -> Result<(), DistrictError> {
        if binding != &self.binding {
            return Err(DistrictError::State(Error::Binding));
        }
        if !self.is_pristine() {
            return Err(DistrictError::State(Error::NotPristine));
        }
        self.district(owner, id)?;
        let record = self
            .retained
            .players
            .iter_mut()
            .find(|p| p.header.owner == owner)
            .and_then(|p| {
                p.districts
                    .records
                    .entries
                    .iter_mut()
                    .find(|e| e.value.id == id)
            })
            .ok_or(DistrictError::MissingDistrict { owner, id })?;
        if record.value.marked_for_delete.is_some() {
            return Err(DistrictError::State(Error::AlreadyInitialized));
        }
        record.value.marked_for_delete = Some(value);
        Ok(())
    }
    pub fn player_districts_mut(&mut self, owner: i32) -> Result<&mut Districts, Error> {
        self.world.player(owner).map_err(Error::World)?;
        let player = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        Ok(&mut self.retained.players[player].districts)
    }
    /// Full current identity for mutation handles. The native low-slot Find
    /// projection is separate; a stale creation handle cannot edit a new ID.
    pub fn district(&self, owner: i32, id: u32) -> Result<&District, DistrictError> {
        self.world
            .player(owner)
            .map_err(|e| DistrictError::State(Error::World(e)))?;
        self.retained
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .and_then(|p| {
                p.districts
                    .records
                    .entries
                    .iter()
                    .find(|e| e.value.id == id && id != u32::MAX)
            })
            .map(|e| &e.value)
            .ok_or(DistrictError::MissingDistrict { owner, id })
    }
    pub fn district_mut(&mut self, owner: i32, id: u32) -> Result<&mut District, DistrictError> {
        self.district(owner, id)?;
        let player = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(DistrictError::MissingDistrict { owner, id })?;
        let entry = self.retained.players[player]
            .districts
            .records
            .entries
            .iter()
            .position(|e| e.value.id == id)
            .ok_or(DistrictError::MissingDistrict { owner, id })?;
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(DistrictError::State(Error::ResourceLimit))?;
        Ok(&mut self.retained.players[player].districts.records.entries[entry].value)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn district_mutation_borrows_the_canonical_owner_and_refuses_stale_identity() {
        let mut state = super::super::tests::state();
        let owner = state.world().players()[0].owner();
        let mut value = crate::district_initialization::construct(
            crate::district_initialization::Definitions {
                yields: &[11],
                great_person_classes: &[23],
            },
        )
        .unwrap();
        value.id = 0x10000;
        state.player_districts_mut(owner).unwrap().records = crate::map::Recycled {
            capacity: 8,
            entries: vec![crate::map::Slot { index: 0, value }],
            current_id_bits: 0x10000,
            free_head: 1,
            free_tail: 7,
            free_links: (1..8).collect(),
        };
        let before = state.revision();
        assert!(state.district_mut(owner, 0x20000).is_err());
        assert_eq!(state.revision(), before);
        state.district_mut(owner, 0x10000).unwrap().is_complete = true;
        assert!(
            state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == owner)
                .unwrap()
                .districts
                .records
                .entries[0]
                .value
                .is_complete
        );
        assert_eq!(state.revision(), before + 1);
    }
}
