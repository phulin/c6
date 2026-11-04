//! Fresh creation registration beside the same canonical World records.
use super::*;
use crate::{city_initialization::Allocation, world::unit_creation as storage};
#[derive(Debug)]
pub enum CreationError {
    State(Error),
    Storage(storage::Error),
    MissingConstructed(u32),
}
impl LoadedState {
    /// Stats.IncrementNumUnitsConstructed2b81c50: one wrapping i32 increment.
    /// Missing remapped storage is not treated as a captured zero.
    pub fn increment_units_constructed(
        &mut self,
        owner: i32,
        hash: u32,
    ) -> Result<(), CreationError> {
        let player = self
            .retained
            .players
            .iter_mut()
            .find(|p| p.header.owner == owner)
            .ok_or(CreationError::State(Error::Missing(MissingInput::Player(
                owner,
            ))))?;
        let at = player
            .stats
            .num_units_constructed
            .iter()
            .rposition(|entry| entry.definition_hash == hash)
            .ok_or(CreationError::MissingConstructed(hash))?;
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(CreationError::State(Error::ResourceLimit))?;
        player.stats.num_units_constructed[at].value =
            player.stats.num_units_constructed[at].value.wrapping_add(1);
        Ok(())
    }
    /// Player.Units.Create updates peak after successful initialization, before
    /// Formation attachment and synchronous Unit-created signals.
    pub fn record_created_unit_peak(&mut self, owner: i32) -> Result<(), Error> {
        let count = self
            .world
            .player(owner)
            .map_err(Error::World)?
            .unit_records()
            .map_err(Error::World)?
            .len();
        let count = u32::try_from(count).map_err(|_| Error::ResourceLimit)?;
        let player = self
            .retained
            .players
            .iter_mut()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        player.units.peak_units = player.units.peak_units.max(count);
        Ok(())
    }
    pub fn reserve_unit_creation(
        &mut self,
        owner: i32,
    ) -> Result<Option<Allocation>, CreationError> {
        let next = self
            .revision
            .checked_add(1)
            .ok_or(CreationError::State(Error::ResourceLimit))?;
        let allocation = self
            .world
            .reserve_unit_creation(owner)
            .map_err(CreationError::Storage)?;
        self.revision = next;
        Ok(allocation)
    }
    pub fn insert_created_unit(
        &mut self,
        owner: i32,
        allocation: Allocation,
        unit: unit::Unit,
    ) -> Result<(), CreationError> {
        let next = self
            .revision
            .checked_add(1)
            .ok_or(CreationError::State(Error::ResourceLimit))?;
        self.world
            .insert_created_unit(owner, allocation, unit)
            .map_err(CreationError::Storage)?;
        self.revision = next;
        Ok(())
    }
    pub fn reserve_formation_creation(
        &mut self,
        owner: i32,
    ) -> Result<Option<Allocation>, CreationError> {
        let next = self
            .revision
            .checked_add(1)
            .ok_or(CreationError::State(Error::ResourceLimit))?;
        let allocation = self
            .world
            .reserve_formation_creation(owner)
            .map_err(CreationError::Storage)?;
        self.revision = next;
        Ok(allocation)
    }
    pub fn insert_created_formation(
        &mut self,
        owner: i32,
        allocation: Allocation,
        formation: unit::Formation,
    ) -> Result<(), CreationError> {
        let next = self
            .revision
            .checked_add(1)
            .ok_or(CreationError::State(Error::ResourceLimit))?;
        self.world
            .insert_created_formation(owner, allocation, formation)
            .map_err(CreationError::Storage)?;
        self.revision = next;
        Ok(())
    }
    /// Source Unit.Initialize has resolved its definition/domain and reset
    /// indexed arrays, but Player.Units.Create has not attached a Formation yet.
    pub fn register_created_unit_prefix(
        &mut self,
        owner: i32,
        id: u32,
        inputs: &InitializationInputs<'_>,
    ) -> Result<(), Error> {
        if inputs.identities != &self.binding.inputs {
            return Err(Error::Binding);
        }
        let player = self.world.player(owner).map_err(Error::World)?;
        let unit = player
            .unit(id)
            .map_err(Error::World)?
            .ok_or(Error::Missing(MissingInput::Unit { owner, id }))?
            .record;
        if unit.id != id {
            return Err(Error::RuntimeIdentity);
        }
        let value = UnitRuntime::from_created_prefix(unit, &self.binding.inputs, inputs)
            .map_err(Error::UnitInitialization)?;
        let dependencies = UnitDependencies::capture(unit)?;
        let slot = (id & 0xffff) as usize;
        let runtime_items = |runtime: &UnitRuntime| -> Option<usize> {
            [
                runtime.abilities.len(),
                runtime.invalid_terrain.len(),
                runtime.ignore_terrain.len(),
                runtime.ignore_feature.len(),
                runtime.bonus_points_per_kill.len(),
                runtime.bonus_points_per_kill_strength.len(),
                runtime.operations_disabled.len(),
            ]
            .into_iter()
            .try_fold(0usize, usize::checked_add)
        };
        let mut items = runtime_items(&value).ok_or(Error::ResourceLimit)?;
        for player in self
            .runtime
            .as_ref()
            .ok_or(Error::Missing(MissingInput::UnitRuntime))?
        {
            let replaced = (player.owner == owner)
                .then(|| player.unit_index.get(slot).copied().flatten())
                .flatten();
            for (index, entry) in player.units.iter().enumerate() {
                if Some(index) != replaced {
                    items = items
                        .checked_add(runtime_items(&entry.value).ok_or(Error::ResourceLimit)?)
                        .ok_or(Error::ResourceLimit)?;
                }
            }
        }
        for player in self.world.players() {
            for entry in player.unit_records().map_err(Error::World)? {
                items = items
                    .checked_add(UnitDependencies::items(&entry.value).ok_or(Error::ResourceLimit)?)
                    .ok_or(Error::ResourceLimit)?;
            }
        }
        if items > self.limits.max_runtime_array_items {
            return Err(Error::ResourceLimit);
        }
        let current = self
            .runtime
            .as_mut()
            .ok_or(Error::Missing(MissingInput::UnitRuntime))?
            .iter_mut()
            .find(|r| r.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        current
            .unit_index
            .try_reserve(
                slot.saturating_add(1)
                    .saturating_sub(current.unit_index.len()),
            )
            .map_err(|_| Error::ResourceLimit)?;
        current
            .units
            .try_reserve(1)
            .map_err(|_| Error::ResourceLimit)?;
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        current
            .unit_index
            .resize(current.unit_index.len().max(slot + 1), None);
        let entry = RuntimeUnit {
            value,
            dependencies,
        };
        if let Some(at) = current.unit_index[slot] {
            current.units[at] = entry;
        } else {
            let at = current.units.len();
            current.units.push(entry);
            current.unit_index[slot] = Some(at);
        }
        Ok(())
    }
    pub fn register_created_formation(&mut self, owner: i32, id: u32) -> Result<(), Error> {
        let formation = self
            .world
            .player(owner)
            .map_err(Error::World)?
            .formation(id)
            .map_err(Error::World)?
            .ok_or(Error::Missing(MissingInput::Formation { owner, id }))?
            .record;
        if formation.id != id {
            return Err(Error::RuntimeIdentity);
        }
        let value = FormationRuntime::from_created(formation);
        let current = self
            .formation_runtime
            .as_mut()
            .ok_or(Error::Missing(MissingInput::FormationRuntime))?
            .iter_mut()
            .find(|r| r.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        let slot = (id & 0xffff) as usize;
        current
            .formation_index
            .try_reserve(
                slot.saturating_add(1)
                    .saturating_sub(current.formation_index.len()),
            )
            .map_err(|_| Error::ResourceLimit)?;
        current
            .formations
            .try_reserve(1)
            .map_err(|_| Error::ResourceLimit)?;
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        current
            .formation_index
            .resize(current.formation_index.len().max(slot + 1), None);
        if let Some(at) = current.formation_index[slot] {
            current.formations[at] = value;
        } else {
            let at = current.formations.len();
            current.formations.push(value);
            current.formation_index[slot] = Some(at);
        }
        Ok(())
    }
    /// Source SetFormation/OnFormationIDChanged updates both the saved ID and
    /// the unsaved resolved pointer. Never infer attachment from slot zero.
    pub fn attach_created_unit(
        &mut self,
        owner: i32,
        id: u32,
        formation_id: u32,
    ) -> Result<(), Error> {
        let formation = self
            .world
            .player(owner)
            .map_err(Error::World)?
            .formation(formation_id)
            .map_err(Error::World)?
            .ok_or(Error::Missing(MissingInput::Formation {
                owner,
                id: formation_id,
            }))?;
        if formation.record.id != formation_id {
            return Err(Error::RuntimeIdentity);
        }
        let reference = crate::unit_initialization::FormationReference {
            owner,
            requested_id: formation_id,
            resolved_id: formation_id,
        };
        let (unit, runtime) = self.unit_with_runtime_mut(owner, id)?;
        unit.formation_id = formation_id;
        runtime.formation = Some(reference);
        self.refresh_unit_dependencies(owner, id)
    }
    /// Commit the actual leaf's canonical array/formation updates before its
    /// synchronous signal. This never resets live arrays or invokes Read.
    pub fn refresh_unit_dependencies(&mut self, owner: i32, id: u32) -> Result<(), Error> {
        let unit = self
            .world
            .player(owner)
            .map_err(Error::World)?
            .unit(id)
            .map_err(Error::World)?
            .ok_or(Error::Missing(MissingInput::Unit { owner, id }))?
            .record;
        if unit.id != id {
            return Err(Error::RuntimeIdentity);
        }
        let dependencies = UnitDependencies::capture(unit)?;
        let row = self
            .runtime
            .as_mut()
            .ok_or(Error::Missing(MissingInput::UnitRuntime))?
            .iter_mut()
            .find(|r| r.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        let at = row
            .unit_index
            .get((id & 0xffff) as usize)
            .copied()
            .flatten()
            .ok_or(Error::Missing(MissingInput::Unit { owner, id }))?;
        let identity = row.units[at].value.archive.identity();
        if identity.id != id || i32::from(identity.owner) != owner {
            return Err(Error::RuntimeIdentity);
        }
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        row.units[at].dependencies = dependencies;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement_world::{DefinitionLookup, DefinitionTable};
    struct Lookup(usize);
    impl DefinitionLookup for Lookup {
        fn count(&self) -> usize {
            self.0
        }
        fn find_index(&self, key: u32) -> Option<usize> {
            (key == 0 && self.0 == 1).then_some(0)
        }
        fn type_hash(&self, index: usize) -> Option<u32> {
            (index < self.0).then_some(index as u32)
        }
    }
    #[test]
    fn freshly_unattached_pointer_stays_null_despite_existing_formation_slot_zero() {
        let mut state = super::super::tests::state();
        let owner = state.world.players()[0].owner();
        let mut forms = state.world.players()[0].formations().unwrap().clone();
        forms.capacity = 8;
        forms.entries[0].index = 0;
        forms.entries[0].value.id = 0x10000;
        forms.current_id_bits = 0x10000;
        forms.free_head = 1;
        forms.free_tail = 7;
        forms.free_links = (1..8).collect();
        let mut unit = super::super::tests::commit_fixture_unit();
        unit.e_owner = owner;
        unit.e_type_index = 0;
        unit.formation_id = 0;
        let units = map::Recycled {
            capacity: 0,
            entries: vec![],
            current_id_bits: 0,
            free_head: u32::MAX,
            free_tail: u32::MAX,
            free_links: vec![],
        };
        let player = world::Player::new(owner, Some(units), Some(forms)).unwrap();
        state.world = world::World::new(state.world.map().clone(), vec![player]).unwrap();
        state.runtime = Some(vec![RuntimePlayer {
            owner,
            unit_index: vec![],
            units: vec![],
        }]);
        let allocation = state.reserve_unit_creation(owner).unwrap().unwrap();
        unit.id = allocation.id;
        state.insert_created_unit(owner, allocation, unit).unwrap();
        let unit_definitions = [crate::unit_initialization::UnitDefinition {
            domain_hash: 0xb6e1baac,
        }];
        let one = Lookup(1);
        let empty = Lookup(0);
        let identities = state.binding.inputs.clone();
        let inputs = InitializationInputs {
            identities: &identities,
            units: DefinitionTable::new(&one, &unit_definitions).unwrap(),
            abilities: &empty,
            terrains: &empty,
            features: &empty,
            great_person_classes: &empty,
            operations: &empty,
        };
        state
            .register_created_unit_prefix(owner, allocation.id, &inputs)
            .unwrap();
        assert!(state
            .world
            .player(owner)
            .unwrap()
            .formation(0)
            .unwrap()
            .is_some());
        let runtime = state.unit_runtime(owner, allocation.id).unwrap();
        assert!(runtime.formation.is_none());
        assert_eq!(runtime.domain, 2);
        state.unit_mut(owner, allocation.id).unwrap().formation_id = u32::MAX;
        assert!(matches!(
            state.unit_runtime(owner, allocation.id),
            Err(Error::RuntimeDependenciesChanged)
        ));
        state
            .refresh_unit_dependencies(owner, allocation.id)
            .unwrap();
        assert!(state
            .unit_runtime(owner, allocation.id)
            .unwrap()
            .formation
            .is_none());
    }
}
