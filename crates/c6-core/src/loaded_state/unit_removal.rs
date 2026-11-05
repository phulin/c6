//! Canonical storage/metadata release AFTER reached destruction effects.
//! This does not replace UnitRemoved, placement, modifier or notification work.
use super::*;
use crate::world::unit_creation as storage;
#[derive(Debug)]
pub enum RemovalError {
    State(Error),
    Storage(storage::Error),
    Identity,
}
impl LoadedState {
    /// Formation.RemoveUnit's singleton path: null the current Unit link and
    /// erase its sole membership before the immediately following empty destroy.
    /// No nonempty Formation update/signals are silently omitted here.
    pub fn detach_destroyed_singleton(
        &mut self,
        owner: i32,
        id: u32,
        formation: u32,
    ) -> Result<(), Error> {
        let record = self
            .world
            .player(owner)
            .map_err(Error::World)?
            .formation(formation)
            .map_err(Error::World)?
            .ok_or(Error::Missing(MissingInput::Formation {
                owner,
                id: formation,
            }))?
            .record;
        if record.id != formation || record.unit_ids.as_slice() != [id] {
            return Err(Error::RuntimeIdentity);
        }
        let (unit, runtime) = self.unit_with_runtime_mut(owner, id)?;
        if runtime.formation.is_none_or(|r| r.resolved_id != formation) {
            return Err(Error::RuntimeIdentity);
        }
        unit.formation_id = u32::MAX;
        runtime.formation = None;
        self.refresh_unit_dependencies(owner, id)?;
        self.formation_mut(owner, formation)?.unit_ids.clear();
        Ok(())
    }
    pub fn release_destroyed_unit_storage(
        &mut self,
        owner: i32,
        id: u32,
    ) -> Result<(), RemovalError> {
        let current = self
            .world
            .player(owner)
            .map_err(|e| RemovalError::State(Error::World(e)))?
            .unit(id)
            .map_err(|e| RemovalError::State(Error::World(e)))?
            .ok_or(RemovalError::State(Error::Missing(MissingInput::Unit {
                owner,
                id,
            })))?
            .record;
        if current.id != id {
            return Err(RemovalError::Identity);
        }
        let slot = (id & 0xffff) as usize;
        let runtime_at = if let Some(rows) = &self.runtime {
            let row = rows
                .iter()
                .find(|r| r.owner == owner)
                .ok_or(RemovalError::State(Error::Missing(MissingInput::Player(
                    owner,
                ))))?;
            let at = row
                .unit_index
                .get(slot)
                .copied()
                .flatten()
                .ok_or(RemovalError::State(Error::Missing(MissingInput::Unit {
                    owner,
                    id,
                })))?;
            if row
                .units
                .get(at)
                .is_none_or(|u| u.value.archive.identity().id != id)
            {
                return Err(RemovalError::Identity);
            }
            Some(at)
        } else {
            None
        };
        let next = self
            .revision
            .checked_add(1)
            .ok_or(RemovalError::State(Error::ResourceLimit))?;
        self.world
            .release_unit_allocation(owner, id)
            .map_err(RemovalError::Storage)?
            .ok_or(RemovalError::Identity)?;
        if let Some(at) = runtime_at {
            let row = self
                .runtime
                .as_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r.owner == owner)
                .unwrap();
            row.units.remove(at);
            row.unit_index.fill(None);
            for (at, u) in row.units.iter().enumerate() {
                row.unit_index[(u.value.archive.identity().id & 0xffff) as usize] = Some(at);
            }
        }
        self.revision = next;
        Ok(())
    }
    /// Empty Formation destructor has no surviving member links to clear.
    pub fn release_destroyed_empty_formation(
        &mut self,
        owner: i32,
        id: u32,
    ) -> Result<(), RemovalError> {
        let current = self
            .world
            .player(owner)
            .map_err(|e| RemovalError::State(Error::World(e)))?
            .formation(id)
            .map_err(|e| RemovalError::State(Error::World(e)))?
            .ok_or(RemovalError::State(Error::Missing(
                MissingInput::Formation { owner, id },
            )))?
            .record;
        if current.id != id || !current.unit_ids.is_empty() {
            return Err(RemovalError::Identity);
        }
        let slot = (id & 0xffff) as usize;
        let runtime_at = if let Some(rows) = &self.formation_runtime {
            let row = rows
                .iter()
                .find(|r| r.owner == owner)
                .ok_or(RemovalError::State(Error::Missing(MissingInput::Player(
                    owner,
                ))))?;
            let at =
                row.formation_index
                    .get(slot)
                    .copied()
                    .flatten()
                    .ok_or(RemovalError::State(Error::Missing(
                        MissingInput::Formation { owner, id },
                    )))?;
            if row
                .formations
                .get(at)
                .is_none_or(|f| f.archive.identity().id != id)
            {
                return Err(RemovalError::Identity);
            }
            Some(at)
        } else {
            None
        };
        let next = self
            .revision
            .checked_add(1)
            .ok_or(RemovalError::State(Error::ResourceLimit))?;
        self.world
            .release_formation_allocation(owner, id)
            .map_err(RemovalError::Storage)?
            .ok_or(RemovalError::Identity)?;
        if let Some(at) = runtime_at {
            let row = self
                .formation_runtime
                .as_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r.owner == owner)
                .unwrap();
            row.formations.remove(at);
            row.formation_index.fill(None);
            for (at, f) in row.formations.iter().enumerate() {
                row.formation_index[(f.archive.identity().id & 0xffff) as usize] = Some(at);
            }
        }
        self.revision = next;
        Ok(())
    }
}
