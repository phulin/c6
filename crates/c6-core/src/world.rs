//! Owned bounded Linux world records, not complete live game reconstruction.
//!
//! Collections must be complete typed exports in their native iteration order.
//! Explicit recycled slots, never generated IDs, supply lookup storage. Rules,
//! module and configuration identity gates belong to the importer. See
//! `docs/linux-world-state.md` for the evidence and unsupported live state.
use crate::map::{Map, Plot, Recycled, Slot, UnitReference};
use crate::unit::{Formation, Unit};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MissingInput {
    Player { owner: i32 },
    PlayerRegistry,
    Units { owner: i32 },
    Formations { owner: i32 },
    UnitWireMetadata { owner: i32 },
    FormationWireMetadata { owner: i32 },
    InvalidDimensions,
    DuplicatePlayer { owner: i32 },
    InvalidSlot,
    DuplicateSlot,
    RecordOwner { owner: i32 },
    DuplicateLayer,
    InvalidCell,
}
impl std::fmt::Display for MissingInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "missing or inconsistent Linux world input: {self:?}")
    }
}
impl std::error::Error for MissingInput {}

/// Requested generation bits are retained even when native Find resolves a
/// different generation at the same slot. The borrowed record is authoritative.
#[derive(Debug)]
pub struct Found<'a, T> {
    pub requested_id: u32,
    pub resolved_id: u32,
    pub record: &'a T,
}

/// Complete logical slot export in native iteration order. This establishes
/// Find/iteration contents, not native capacity, free-list or allocator metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompleteSlots<T> {
    pub entries: Vec<Slot<T>>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
enum CollectionRecords<T> {
    Wire(Recycled<T>),
    Slots(CompleteSlots<T>),
}
impl<T> CollectionRecords<T> {
    fn entries(&self) -> &[Slot<T>] {
        match self {
            Self::Wire(v) => &v.entries,
            Self::Slots(v) => &v.entries,
        }
    }
    fn entries_mut(&mut self) -> &mut [Slot<T>] {
        match self {
            Self::Wire(v) => &mut v.entries,
            Self::Slots(v) => &mut v.entries,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct Collection<T> {
    records: CollectionRecords<T>,
    by_slot: Vec<Option<usize>>,
}
impl<T> Collection<T> {
    fn new(records: Recycled<T>) -> Result<Self, MissingInput> {
        if records.capacity > 65536 {
            return Err(MissingInput::InvalidSlot);
        }
        let capacity = records.capacity as usize;
        Self::assemble(CollectionRecords::Wire(records), capacity)
    }
    fn from_slots(records: CompleteSlots<T>) -> Result<Self, MissingInput> {
        let mut capacity = 0;
        for entry in &records.entries {
            let slot = usize::try_from(entry.index).map_err(|_| MissingInput::InvalidSlot)?;
            if slot >= 65536 {
                return Err(MissingInput::InvalidSlot);
            }
            capacity = capacity.max(slot + 1);
        }
        Self::assemble(CollectionRecords::Slots(records), capacity)
    }
    fn assemble(records: CollectionRecords<T>, capacity: usize) -> Result<Self, MissingInput> {
        let mut by_slot = vec![None; capacity];
        for (i, entry) in records.entries().iter().enumerate() {
            let slot = usize::try_from(entry.index).map_err(|_| MissingInput::InvalidSlot)?;
            let target = by_slot.get_mut(slot).ok_or(MissingInput::InvalidSlot)?;
            if target.replace(i).is_some() {
                return Err(MissingInput::DuplicateSlot);
            }
        }
        Ok(Self { records, by_slot })
    }
    fn index(&self, id: u32) -> Option<usize> {
        if id == u32::MAX {
            return None;
        }
        self.by_slot.get((id & 0xffff) as usize).copied().flatten()
    }
    fn get(&self, id: u32) -> Option<&T> {
        Some(&self.records.entries().get(self.index(id)?)?.value)
    }
    fn get_mut(&mut self, id: u32) -> Option<&mut T> {
        let index = self.index(id)?;
        Some(&mut self.records.entries_mut().get_mut(index)?.value)
    }
}

/// Actual owner identity and complete collections, or explicit unavailable
/// collections. `Some(empty)` proves absence; `None` cannot answer Find.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Player {
    owner: i32,
    units: Option<Collection<Unit>>,
    formations: Option<Collection<Formation>>,
    /// Actual Configuration::Player slot status; SS_TAKEN is 3.
    pub slot_status: Option<i32>,
    /// Actual Configuration::Player team; -1 is NO_TEAM.
    pub team: Option<i32>,
    /// Actual Core Player::IsAlive field, distinct from configuration lifecycle.
    pub is_alive: Option<bool>,
    pub technologies: Option<Vec<bool>>,
    pub civics: Option<Vec<bool>>,
}
impl Player {
    pub fn new(
        owner: i32,
        units: Option<Recycled<Unit>>,
        formations: Option<Recycled<Formation>>,
    ) -> Result<Self, MissingInput> {
        if units
            .as_ref()
            .is_some_and(|c| c.entries.iter().any(|s| s.value.e_owner != owner))
            || formations
                .as_ref()
                .is_some_and(|c| c.entries.iter().any(|s| s.value.owner != owner))
        {
            return Err(MissingInput::RecordOwner { owner });
        }
        Ok(Self {
            owner,
            units: units.map(Collection::new).transpose()?,
            formations: formations.map(Collection::new).transpose()?,
            slot_status: None,
            team: None,
            is_alive: None,
            technologies: None,
            civics: None,
        })
    }
    pub fn owner(&self) -> i32 {
        self.owner
    }
    /// Construct from verified complete Begin/End/GetCount/Find exports, without
    /// fabricating unused allocator metadata. Absent collections remain unknown.
    pub fn from_lookup_slots(
        owner: i32,
        units: Option<CompleteSlots<Unit>>,
        formations: Option<CompleteSlots<Formation>>,
    ) -> Result<Self, MissingInput> {
        if units
            .as_ref()
            .is_some_and(|c| c.entries.iter().any(|s| s.value.e_owner != owner))
            || formations
                .as_ref()
                .is_some_and(|c| c.entries.iter().any(|s| s.value.owner != owner))
        {
            return Err(MissingInput::RecordOwner { owner });
        }
        Ok(Self {
            owner,
            units: units.map(Collection::from_slots).transpose()?,
            formations: formations.map(Collection::from_slots).transpose()?,
            slot_status: None,
            team: None,
            is_alive: None,
            technologies: None,
            civics: None,
        })
    }
    pub fn unit_records(&self) -> Result<&[Slot<Unit>], MissingInput> {
        Ok(self
            .units
            .as_ref()
            .ok_or(MissingInput::Units { owner: self.owner })?
            .records
            .entries())
    }
    pub fn formation_records(&self) -> Result<&[Slot<Formation>], MissingInput> {
        Ok(self
            .formations
            .as_ref()
            .ok_or(MissingInput::Formations { owner: self.owner })?
            .records
            .entries())
    }
    pub fn units(&self) -> Result<&Recycled<Unit>, MissingInput> {
        match &self
            .units
            .as_ref()
            .ok_or(MissingInput::Units { owner: self.owner })?
            .records
        {
            CollectionRecords::Wire(v) => Ok(v),
            CollectionRecords::Slots(_) => {
                Err(MissingInput::UnitWireMetadata { owner: self.owner })
            }
        }
    }
    pub fn formations(&self) -> Result<&Recycled<Formation>, MissingInput> {
        match &self
            .formations
            .as_ref()
            .ok_or(MissingInput::Formations { owner: self.owner })?
            .records
        {
            CollectionRecords::Wire(v) => Ok(v),
            CollectionRecords::Slots(_) => {
                Err(MissingInput::FormationWireMetadata { owner: self.owner })
            }
        }
    }
    pub fn unit(&self, id: u32) -> Result<Option<Found<'_, Unit>>, MissingInput> {
        let collection = self
            .units
            .as_ref()
            .ok_or(MissingInput::Units { owner: self.owner })?;
        let Some(record) = collection.get(id) else {
            return Ok(None);
        };
        if record.e_owner != self.owner {
            return Err(MissingInput::RecordOwner { owner: self.owner });
        }
        Ok(Some(Found {
            requested_id: id,
            resolved_id: record.id,
            record,
        }))
    }
    pub fn formation(&self, id: u32) -> Result<Option<Found<'_, Formation>>, MissingInput> {
        let collection = self
            .formations
            .as_ref()
            .ok_or(MissingInput::Formations { owner: self.owner })?;
        let Some(record) = collection.get(id) else {
            return Ok(None);
        };
        if record.owner != self.owner {
            return Err(MissingInput::RecordOwner { owner: self.owner });
        }
        Ok(Some(Found {
            requested_id: id,
            resolved_id: record.id,
            record,
        }))
    }
    /// Mutates the actual owned record. Changing identity, placement or ownership
    /// does not update other containers: a future action must maintain them too.
    pub fn unit_mut(&mut self, id: u32) -> Result<Option<&mut Unit>, MissingInput> {
        self.unit(id)?;
        Ok(self.units.as_mut().expect("checked collection").get_mut(id))
    }
    pub fn formation_mut(&mut self, id: u32) -> Result<Option<&mut Formation>, MissingInput> {
        self.formation(id)?;
        Ok(self
            .formations
            .as_mut()
            .expect("checked collection")
            .get_mut(id))
    }
    pub fn unit_formation(&self, id: u32) -> Result<Option<Found<'_, Formation>>, MissingInput> {
        match self.unit(id)? {
            Some(unit) => self.formation(unit.record.formation_id),
            None => Ok(None),
        }
    }
    /// One result per serialized member, including duplicates and absent slots.
    /// No membership is synthesized from Unit.formation_id or reverse links.
    pub fn members<'a>(
        &'a self,
        formation: &'a Formation,
    ) -> impl Iterator<Item = Result<Option<Found<'a, Unit>>, MissingInput>> + 'a {
        formation.unit_ids.iter().map(|&id| {
            if formation.owner != self.owner {
                Err(MissingInput::RecordOwner { owner: self.owner })
            } else {
                self.unit(id)
            }
        })
    }
}

/// Owns bounded typed state and builds stable indices once at assembly time.
/// The player vector is the complete supplied native player iteration order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct World {
    map: Map,
    players: Vec<Player>,
    by_owner: Vec<(i32, usize)>,
    cells: Vec<Vec<Option<usize>>>,
    by_layer: Vec<(u32, usize)>,
    /// Observed current global node availability, in native definition order.
    /// Unknown is `None`; false must be an actual observed absent/inactive node.
    pub technology_availability: Option<Vec<bool>>,
    /// Native Manager player-ID to player-container-index mapping. -1 is absent;
    /// all other values are valid for IsValid, independent of imported records.
    pub player_mapping: Option<[i32; 65]>,
    /// Actual manager Player* slot non-null facts, independent of validity.
    /// Unknown remains None; false is a captured null pointer, not absent import.
    pub player_slot_present: Option<[bool; 65]>,
    pub civic_availability: Option<Vec<bool>>,
}
impl World {
    pub fn new(map: Map, players: Vec<Player>) -> Result<Self, MissingInput> {
        let width =
            usize::try_from(map.header.width).map_err(|_| MissingInput::InvalidDimensions)?;
        let height =
            usize::try_from(map.header.height).map_err(|_| MissingInput::InvalidDimensions)?;
        let count = width
            .checked_mul(height)
            .filter(|_| width > 0 && height > 0)
            .ok_or(MissingInput::InvalidDimensions)?;
        if count != map.plots.len()
            || map.units.width as usize != width
            || map.units.height as usize != height
        {
            return Err(MissingInput::InvalidDimensions);
        }
        let mut by_owner: Vec<_> = players
            .iter()
            .enumerate()
            .map(|(i, p)| (p.owner, i))
            .collect();
        by_owner.sort_unstable_by_key(|&(owner, _)| owner);
        if let Some(pair) = by_owner.windows(2).find(|p| p[0].0 == p[1].0) {
            return Err(MissingInput::DuplicatePlayer { owner: pair[0].0 });
        }
        let mut cells = Vec::with_capacity(map.units.layers.len());
        for (layer_index, layer) in map.units.layers.iter().enumerate() {
            if map.units.layers[..layer_index]
                .iter()
                .any(|l| l.layer_id == layer.layer_id)
            {
                return Err(MissingInput::DuplicateLayer);
            }
            let mut indices = vec![None; count];
            for (index, cell) in layer.cells.iter().enumerate() {
                let entry = indices
                    .get_mut(cell.plot_index as usize)
                    .ok_or(MissingInput::InvalidCell)?;
                if entry.replace(index).is_some() {
                    return Err(MissingInput::InvalidCell);
                }
            }
            cells.push(indices);
        }
        let mut by_layer: Vec<_> = map
            .units
            .layers
            .iter()
            .enumerate()
            .map(|(i, l)| (l.layer_id, i))
            .collect();
        by_layer.sort_unstable_by_key(|&(id, _)| id);
        Ok(Self {
            map,
            players,
            by_owner,
            cells,
            by_layer,
            technology_availability: None,
            civic_availability: None,
            player_mapping: None,
            player_slot_present: None,
        })
    }
    pub fn map(&self) -> &Map {
        &self.map
    }
    /// Serialized own visibility layers and the common revealed-state array.
    /// Shared visibility counts are separate live state, absent from Map wire.
    pub fn visibility(&self, owner: i32) -> Option<&crate::map::Visibility> {
        self.map
            .visibility
            .iter()
            .find(|v| v.player == owner)
            .map(|v| &v.visibility)
    }
    pub fn visibility_mut(&mut self, owner: i32) -> Option<&mut crate::map::Visibility> {
        self.map
            .visibility
            .iter_mut()
            .find(|v| v.player == owner)
            .map(|v| &mut v.visibility)
    }
    pub fn players(&self) -> &[Player] {
        &self.players
    }
    pub fn player(&self, owner: i32) -> Result<&Player, MissingInput> {
        let i = self
            .by_owner
            .binary_search_by_key(&owner, |&(key, _)| key)
            .map_err(|_| MissingInput::Player { owner })?;
        let player = &self.players[self.by_owner[i].1];
        if player.owner != owner {
            return Err(MissingInput::RecordOwner { owner });
        }
        Ok(player)
    }
    pub fn player_mut(&mut self, owner: i32) -> Result<&mut Player, MissingInput> {
        let i = self
            .by_owner
            .binary_search_by_key(&owner, |&(key, _)| key)
            .map_err(|_| MissingInput::Player { owner })?;
        let player = &mut self.players[self.by_owner[i].1];
        if player.owner != owner {
            return Err(MissingInput::RecordOwner { owner });
        }
        Ok(player)
    }
    pub fn plot_mut(&mut self, index: usize) -> Option<&mut Plot> {
        self.map.plots.get_mut(index)
    }
    /// Native UnitContainer::AddUnit only. Placement owns Plot counters, sight,
    /// Unit/Formation location and callbacks in their original order.
    pub fn add_unit_occupancy(
        &mut self,
        identity: crate::unit::ComponentId,
        location: crate::unit::PlotCoord,
        layer_id: u32,
    ) -> Result<bool, crate::unit_occupancy::OccupancyError> {
        use crate::unit_occupancy::OccupancyError;
        let count = self.map.plots.len();
        let current = self
            .map
            .units
            .layers
            .iter()
            .position(|l| l.layer_id == layer_id);
        if current.is_some_and(|i| self.map.units.layers[i].maximum as usize != count) {
            return Err(OccupancyError::InvalidWorldGrid);
        }
        let mut new_index = Vec::new();
        if current.is_none() {
            self.cells
                .try_reserve(1)
                .map_err(|_| OccupancyError::ResourceLimit)?;
            self.by_layer
                .try_reserve(1)
                .map_err(|_| OccupancyError::ResourceLimit)?;
            new_index
                .try_reserve_exact(count)
                .map_err(|_| OccupancyError::ResourceLimit)?;
            new_index.resize(count, None);
        }
        let result =
            crate::unit_occupancy::add_unit(&mut self.map.units, identity, location, layer_id)?;
        let layer = current.unwrap_or_else(|| self.map.units.layers.len() - 1);
        if current.is_none() {
            self.cells.push(new_index);
            let insertion = self
                .by_layer
                .binary_search_by_key(&layer_id, |&(id, _)| id)
                .unwrap_err();
            self.by_layer.insert(insertion, (layer_id, layer));
        }
        self.cells[layer].fill(None);
        for (i, cell) in self.map.units.layers[layer].cells.iter().enumerate() {
            self.cells[layer][cell.plot_index as usize] = Some(i);
        }
        Ok(result)
    }

    /// Native UnitContainer::RemoveUnit, preserving native all-layer traversal
    /// and its last-successful-layer return. Existing lookup indexes stay live.
    pub fn remove_unit_occupancy(
        &mut self,
        identity: crate::unit::ComponentId,
        location: crate::unit::PlotCoord,
        layer_id: u32,
    ) -> Result<u32, crate::unit_occupancy::OccupancyError> {
        let result =
            crate::unit_occupancy::remove_unit(&mut self.map.units, identity, location, layer_id)?;
        for (layer, indices) in self.map.units.layers.iter().zip(&mut self.cells) {
            indices.fill(None);
            for (i, cell) in layer.cells.iter().enumerate() {
                indices[cell.plot_index as usize] = Some(i);
            }
        }
        Ok(result)
    }

    /// The original layer/cell UnitReference vector, with its exact candidate
    /// order. An absent layer/cell has the native empty-vector query result.
    pub fn plot_units(
        &self,
        layer_id: u32,
        plot_index: usize,
    ) -> Result<&[UnitReference], MissingInput> {
        if plot_index >= self.map.plots.len() {
            return Err(MissingInput::InvalidCell);
        }
        let Ok(index) = self.by_layer.binary_search_by_key(&layer_id, |&(id, _)| id) else {
            return Ok(&[]);
        };
        let layer = self.by_layer[index].1;
        let Some(cell) = self.cells[layer][plot_index] else {
            return Ok(&[]);
        };
        Ok(&self.map.units.layers[layer].cells[cell].units)
    }
    /// Original Manager validity, independent of imported record presence.
    pub fn player_is_valid(&self, owner: i32) -> Result<bool, MissingInput> {
        if owner as u32 >= 65 {
            return Ok(false);
        }
        let mapping = self
            .player_mapping
            .as_ref()
            .ok_or(MissingInput::PlayerRegistry)?;
        Ok(mapping[owner as usize] != -1)
    }
    /// Resolve an actual GetValidPlayer slot. A valid mapping with missing
    /// imported state remains an error; a separately captured null slot is absent.
    pub fn valid_player(&self, owner: i32) -> Result<Option<&Player>, MissingInput> {
        if !self.player_is_valid(owner)? {
            return Ok(None);
        }
        if self
            .player_slot_present
            .as_ref()
            .is_some_and(|v| !v[owner as usize])
        {
            return Ok(None);
        }
        self.player(owner).map(Some)
    }
    /// Original CManager::GetUnit: invalid identity/player returns absent. Type
    /// bits do not gate this lookup; layer membership equality retains them.
    pub fn unit(&self, reference: UnitReference) -> Result<Option<Found<'_, Unit>>, MissingInput> {
        let owner = reference.player();
        if owner == -1 || reference.id == u32::MAX {
            return Ok(None);
        }
        match self.valid_player(owner)? {
            Some(player) => player.unit(reference.id),
            None => Ok(None),
        }
    }
}

/// CityAttributes omits district instance state and validDomains. Owned wire
/// records alone cannot fulfill these native queries, even with known players.
impl crate::movement_world::RuntimePlotInputs for World {
    fn district(
        &self,
        city: &crate::map::CityAttributes,
    ) -> Result<Option<crate::movement_world::DistrictInstance>, crate::movement_world::MissingInput>
    {
        Err(crate::movement_world::MissingInput::DistrictLookup {
            player: city.player,
            district: city.district,
        })
    }
    fn domain_valid(
        &self,
        plot: usize,
        domain: i32,
    ) -> Result<bool, crate::movement_world::MissingInput> {
        Err(crate::movement_world::MissingInput::CityDomainValidity { plot, domain })
    }
}

impl crate::yields::PrerequisiteState for World {
    fn is_valid_player(&self, player: i32) -> Result<bool, crate::yields::MissingInput> {
        // Original unsigned IsValidRange excludes negatives and IDs >= 65.
        if player as u32 >= 65 {
            return Ok(false);
        }
        let mapping = self
            .player_mapping
            .as_ref()
            .ok_or(crate::yields::MissingInput::PlayerRegistry)?;
        Ok(mapping[player as usize] != -1)
    }
    fn acquired_mask(
        &self,
        player: i32,
        kind: crate::yields::PrerequisiteKind,
    ) -> Result<&[bool], crate::yields::MissingInput> {
        let p = self
            .player(player)
            .map_err(|_| crate::yields::MissingInput::PlayerMask { player, kind })?;
        let mask = match kind {
            crate::yields::PrerequisiteKind::Technology => &p.technologies,
            crate::yields::PrerequisiteKind::Civic => &p.civics,
        };
        mask.as_deref()
            .ok_or(crate::yields::MissingInput::PlayerMask { player, kind })
    }
    fn globally_available(
        &self,
        kind: crate::yields::PrerequisiteKind,
        index: i32,
    ) -> Result<bool, crate::yields::MissingInput> {
        let mask = match kind {
            crate::yields::PrerequisiteKind::Technology => &self.technology_availability,
            crate::yields::PrerequisiteKind::Civic => &self.civic_availability,
        };
        mask.as_ref()
            .and_then(|v| v.get(index as u32 as usize))
            .copied()
            .ok_or(crate::yields::MissingInput::GlobalAvailability { kind, index })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::map::{Header, Slot, UnitCell, UnitContainer, UnitLayer};
    fn recycled<T>(capacity: u32, entries: Vec<(i32, T)>) -> Recycled<T> {
        Recycled {
            capacity,
            entries: entries
                .into_iter()
                .map(|(index, value)| Slot { index, value })
                .collect(),
            current_id_bits: 0x10000,
            free_head: 0,
            free_tail: 0,
            free_links: vec![],
        }
    }
    fn formation(id: u32, owner: i32, ids: Vec<u32>) -> Formation {
        Formation {
            id,
            owner,
            location: crate::unit::PlotCoord { x: 0, y: 0 },
            moves_remaining_raw: 0,
            domain: 0,
            is_embarked: false,
            can_embark: false,
            can_attack: false,
            is_ranged_combat: false,
            can_capture: false,
            has_moved: false,
            has_moved_into_zoc: false,
            ignores_zoc: false,
            last_path: vec![],
            last_path_cache_location: crate::unit::PlotCoord { x: 0, y: 0 },
            unit_ids: ids,
        }
    }
    pub(crate) fn map() -> Map {
        Map {
            header: Header {
                width: 2,
                height: 1,
                top_latitude: 90,
                bottom_latitude: -90,
                wrap_x: false,
                wrap_y: false,
                map_size_hash: 0,
                land_plots: 2,
                owned_plots: 0,
                natural_wonders: 0,
                resource_counts: vec![],
                land_resource_counts: vec![],
            },
            visibility: vec![],
            areas: recycled(0, vec![]),
            landmasses: recycled(0, vec![]),
            regions: recycled(0, vec![]),
            chokepoints: recycled(0, vec![]),
            continents: vec![],
            feature_layouts: recycled(0, vec![]),
            plots: vec![Plot::decode(&[0; 55], 6, crate::map::Limits::default()).unwrap(); 2],
            units: UnitContainer {
                width: 2,
                height: 1,
                layers: vec![
                    UnitLayer {
                        layer_id: 3,
                        maximum: 1,
                        cells: vec![UnitCell {
                            plot_index: 1,
                            units: vec![
                                UnitReference {
                                    owner: 7,
                                    id: 0x20009,
                                },
                                UnitReference {
                                    owner: 2,
                                    id: 0x10004,
                                },
                                UnitReference {
                                    owner: 7,
                                    id: 0x20009,
                                },
                            ],
                        }],
                    },
                    UnitLayer {
                        layer_id: 1,
                        maximum: 0,
                        cells: vec![],
                    },
                ],
            },
            components: vec![],
        }
    }
    #[test]
    fn occupancy_updates_keep_current_layer_cell_indexes_live() {
        let mut m = map();
        m.units.layers.clear();
        let mut w = World::new(m, vec![]).unwrap();
        let a = crate::unit::ComponentId {
            owner: 0,
            component_type: 1,
            id: 12,
        };
        let b = crate::unit::ComponentId {
            owner: 0,
            component_type: 1,
            id: 13,
        };
        let first = crate::unit::PlotCoord { x: 0, y: 0 };
        let second = crate::unit::PlotCoord { x: 1, y: 0 };
        w.add_unit_occupancy(a, second, 7).unwrap();
        w.add_unit_occupancy(b, first, 7).unwrap();
        w.add_unit_occupancy(a, second, 0).unwrap();
        assert_eq!(w.plot_units(7, 1).unwrap()[0].id, 12);
        assert_eq!(w.plot_units(7, 0).unwrap()[0].id, 13);
        assert_eq!(w.remove_unit_occupancy(b, first, 7).unwrap(), 7);
        assert!(w.plot_units(7, 0).unwrap().is_empty());
        assert_eq!(w.plot_units(7, 1).unwrap()[0].id, 12);
        assert_eq!(w.remove_unit_occupancy(a, second, u32::MAX).unwrap(), 0);
        assert!(w.plot_units(7, 1).unwrap().is_empty());
        assert!(w.plot_units(0, 1).unwrap().is_empty());
        // Plot counters are exclusively Placement's ordered responsibility.
        assert_eq!(w.map().plots[0].unit_count, 0);
    }

    #[test]
    fn recycled_lookup_uses_explicit_slot_and_keeps_requested_generation() {
        // Generated IDs deliberately differ from their slots: no guessed ID→slot
        // assembly is allowed. The indexed storage is authoritative for Find.
        let p = Player::new(
            7,
            None,
            Some(recycled(
                16,
                vec![
                    (9, formation(0x10004, 7, vec![])),
                    (2, formation(0x20007, 7, vec![])),
                ],
            )),
        )
        .unwrap();
        let found = p.formation(0x90009).unwrap().unwrap();
        assert_eq!((found.requested_id, found.resolved_id), (0x90009, 0x10004));
        assert!(p.formation(u32::MAX).unwrap().is_none());
        assert!(p.formation(8).unwrap().is_none());
        assert!(p.formation(16).unwrap().is_none());
        assert_eq!(
            p.formations()
                .unwrap()
                .entries
                .iter()
                .map(|s| s.index)
                .collect::<Vec<_>>(),
            [9, 2]
        );
        assert_eq!(p.unit(0).unwrap_err(), MissingInput::Units { owner: 7 });
    }
    #[test]
    fn owned_mutation_is_borrowed_in_place_and_associations_preserve_missing_members() {
        let f = formation(0x10001, 7, vec![3, 3, u32::MAX, 2]);
        let mut p = Player::new(
            7,
            Some(recycled(0, vec![])),
            Some(recycled(8, vec![(1, f)])),
        )
        .unwrap();
        p.formation_mut(0x20001)
            .unwrap()
            .unwrap()
            .moves_remaining_raw = -17;
        assert_eq!(
            p.formations().unwrap().entries[0].value.moves_remaining_raw,
            -17
        );
        let f = p.formation(1).unwrap().unwrap();
        let members = p.members(f.record).collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(members.len(), 4);
        assert!(members.iter().all(Option::is_none));
        assert!(p.unit_formation(3).unwrap().is_none());
        p.formation_mut(1).unwrap().unwrap().owner = 2;
        assert_eq!(
            p.formation(1).unwrap_err(),
            MissingInput::RecordOwner { owner: 7 }
        );
    }
    #[test]
    fn world_preserves_owner_layer_and_candidate_order_without_per_query_rebuild() {
        let world = World::new(
            map(),
            vec![
                Player::new(7, None, None).unwrap(),
                Player::new(2, None, None).unwrap(),
            ],
        )
        .unwrap();
        assert_eq!(
            world
                .players()
                .iter()
                .map(Player::owner)
                .collect::<Vec<_>>(),
            [7, 2]
        );
        assert_eq!(world.player(2).unwrap().owner(), 2);
        assert_eq!(
            world.player(0).unwrap_err(),
            MissingInput::Player { owner: 0 }
        );
        assert_eq!(
            world
                .map()
                .units
                .layers
                .iter()
                .map(|l| l.layer_id)
                .collect::<Vec<_>>(),
            [3, 1]
        );
        let mut world = world;
        let mut mapping = [-1; 65];
        mapping[7] = 0;
        mapping[2] = 1;
        world.player_mapping = Some(mapping);
        let candidates = world.plot_units(3, 1).unwrap();
        assert_eq!(
            candidates.iter().map(|r| r.owner).collect::<Vec<_>>(),
            [7, 2, 7]
        );
        assert!(std::ptr::eq(
            candidates.as_ptr(),
            world.map().units.layers[0].cells[0].units.as_ptr()
        ));
        assert!(world.plot_units(3, 0).unwrap().is_empty());
        assert!(world.plot_units(0, 1).unwrap().is_empty());
        assert_eq!(world.plot_units(3, 2), Err(MissingInput::InvalidCell));
        assert_eq!(
            world.unit(candidates[0]).unwrap_err(),
            MissingInput::Units { owner: 7 }
        );
    }
    #[test]
    fn team_query_resolves_actual_players_before_same_owner_shortcut() {
        use crate::skip_turn::placement::{Error, Inputs};
        let mut world = World::new(map(), vec![]).unwrap();
        let mut mapping = [-1; 65];
        world.player_mapping = Some(mapping);
        fn query(world: &World, a: i32, b: i32) -> Result<bool, Error> {
            Inputs {
                world,
                definitions: None,
                relations: None,
                plot_unit_limit: None,
            }
            .same_team(a, b)
        }
        assert_eq!(query(&world, 5, 5), Ok(false)); // Known invalid, not missing.
        mapping[5] = 0;
        world.player_mapping = Some(mapping);
        assert_eq!(
            query(&world, 5, 5),
            Err(Error::World(MissingInput::Player { owner: 5 }))
        );
        let player = Player::new(5, None, None).unwrap();
        let mut world = World::new(map(), vec![player]).unwrap();
        world.player_mapping = Some(mapping);
        assert_eq!(query(&world, 5, 5), Ok(true)); // Resolved identity needs no Team.
        assert_eq!(query(&world, 0, 5), Ok(false));
        mapping[6] = 1;
        world.player_mapping = Some(mapping);
        assert_eq!(
            query(&world, 5, 6),
            Err(Error::World(MissingInput::Player { owner: 6 }))
        );
        assert_eq!(
            query(&world, 0, 6),
            Err(Error::World(MissingInput::Player { owner: 6 }))
        );
    }
    #[test]
    fn complete_lookup_slots_never_invent_allocator_metadata() {
        let f = formation(0x10004, 7, vec![3]);
        let p = Player::from_lookup_slots(
            7,
            Some(CompleteSlots { entries: vec![] }),
            Some(CompleteSlots {
                entries: vec![Slot { index: 4, value: f }],
            }),
        )
        .unwrap();
        assert!(p.unit_records().unwrap().is_empty());
        assert_eq!(p.units(), Err(MissingInput::UnitWireMetadata { owner: 7 }));
        assert_eq!(
            p.formations(),
            Err(MissingInput::FormationWireMetadata { owner: 7 })
        );
        assert_eq!(p.formation(0x90004).unwrap().unwrap().resolved_id, 0x10004);
        assert!(p.formation(5).unwrap().is_none());
        assert!(p.formation(u32::MAX).unwrap().is_none());
        let unknown = Player::from_lookup_slots(7, None, None).unwrap();
        assert_eq!(
            unknown.unit_records(),
            Err(MissingInput::Units { owner: 7 })
        );
        assert_eq!(
            Collection::from_slots(CompleteSlots {
                entries: vec![Slot {
                    index: 65536,
                    value: 0
                }]
            })
            .unwrap_err(),
            MissingInput::InvalidSlot
        );
        assert_eq!(
            Collection::from_slots(CompleteSlots {
                entries: vec![Slot { index: 1, value: 0 }, Slot { index: 1, value: 1 }]
            })
            .unwrap_err(),
            MissingInput::DuplicateSlot
        );
    }
    #[test]
    fn actual_null_player_slot_is_distinct_from_unavailable_record() {
        let mut w = World::new(map(), vec![]).unwrap();
        let mut mapping = [-1; 65];
        mapping[5] = 0;
        w.player_mapping = Some(mapping);
        assert_eq!(w.valid_player(5), Err(MissingInput::Player { owner: 5 }));
        w.player_slot_present = Some([false; 65]);
        assert_eq!(w.valid_player(5), Ok(None));
        assert!(w
            .unit(UnitReference::from_component_id(crate::unit::ComponentId {
                owner: 5,
                component_type: 1,
                id: 1
            }))
            .unwrap()
            .is_none());
        let input = crate::skip_turn::placement::Inputs {
            world: &w,
            definitions: None,
            relations: None,
            plot_unit_limit: None,
        };
        assert_eq!(input.same_team(5, 5), Ok(false));
        assert_eq!(input.at_war_with(0, 5), Ok(false));
    }
    #[test]
    fn formation_layer_uses_first_raw_member_and_neighbors_keep_nulls() {
        use crate::skip_turn::placement::{formation_layer, query_plots};
        let mut m = map();
        m.units.layers[0].maximum = 2;
        m.units.layers[0].cells[0].units =
            vec![UnitReference::from_component_id(crate::unit::ComponentId {
                owner: 7,
                component_type: 1,
                id: 0x10001,
            })];
        let w = World::new(m, vec![]).unwrap();
        let mut f = formation(2, 7, vec![0x90001, 0x10001]);
        f.location = crate::unit::PlotCoord { x: 1, y: 0 };
        assert_eq!(formation_layer(&w, &f), Ok(None));
        f.unit_ids.remove(0);
        assert_eq!(formation_layer(&w, &f), Ok(Some(3)));
        let anchor = crate::unit::PlotCoord { x: 0, y: 0 };
        let q = query_plots(w.map(), anchor).unwrap();
        assert_eq!(q.iter().filter(|v| v.is_some()).count(), 2);
        assert_eq!(q[2], Some(crate::unit::PlotCoord { x: 1, y: 0 }));
        let mut wrapped = w.map().clone();
        wrapped.header.wrap_x = true;
        wrapped.header.wrap_y = true;
        let q = query_plots(&wrapped, anchor).unwrap();
        assert_eq!(q[0], q[1]);
        assert_eq!(q[5], q[2]);
        assert_eq!(
            query_plots(&wrapped, crate::unit::PlotCoord { x: -2, y: 0 }).unwrap()[0],
            None
        );
        assert_eq!(
            query_plots(&wrapped, crate::unit::PlotCoord { x: -9999, y: 0 }).unwrap(),
            [None; 7]
        );
    }
    #[test]
    fn invalid_assembly_rejects_duplicates_bounds_and_inconsistent_owners() {
        assert_eq!(
            Collection::new(recycled(2, vec![(2, 0)])).unwrap_err(),
            MissingInput::InvalidSlot
        );
        assert_eq!(
            Collection::new(recycled(2, vec![(-1, 0)])).unwrap_err(),
            MissingInput::InvalidSlot
        );
        assert_eq!(
            Collection::new(recycled(2, vec![(0, 0), (0, 1)])).unwrap_err(),
            MissingInput::DuplicateSlot
        );
        assert_eq!(
            Player::new(
                7,
                None,
                Some(recycled(8, vec![(1, formation(1, 2, vec![]))]))
            )
            .unwrap_err(),
            MissingInput::RecordOwner { owner: 7 }
        );
        assert_eq!(
            World::new(
                map(),
                vec![
                    Player::new(2, None, None).unwrap(),
                    Player::new(2, None, None).unwrap()
                ]
            )
            .unwrap_err(),
            MissingInput::DuplicatePlayer { owner: 2 }
        );
        let mut invalid = map();
        invalid.plots.pop();
        assert_eq!(
            World::new(invalid, vec![]).unwrap_err(),
            MissingInput::InvalidDimensions
        );
        let mut invalid = map();
        let duplicate = invalid.units.layers[0].cells[0].clone();
        invalid.units.layers[0].cells.push(duplicate);
        assert_eq!(
            World::new(invalid, vec![]).unwrap_err(),
            MissingInput::InvalidCell
        );
    }
    #[test]
    fn replacing_player_with_another_owner_cannot_silently_rebind_the_index() {
        let mut world = World::new(map(), vec![Player::new(7, None, None).unwrap()]).unwrap();
        *world.player_mut(7).unwrap() = Player::new(2, None, None).unwrap();
        assert_eq!(
            world.player(7).unwrap_err(),
            MissingInput::RecordOwner { owner: 7 }
        );
        assert_eq!(
            world.player_mut(7).unwrap_err(),
            MissingInput::RecordOwner { owner: 7 }
        );
    }
    #[test]
    fn prerequisite_adapter_preserves_native_validity_and_rejects_missing_masks() {
        use crate::yields::{MissingInput as YieldMissing, PrerequisiteKind, PrerequisiteState};
        let mut world = World::new(map(), vec![Player::new(7, None, None).unwrap()]).unwrap();
        assert_eq!(world.is_valid_player(-1), Ok(false));
        assert_eq!(world.is_valid_player(65), Ok(false));
        assert_eq!(world.is_valid_player(7), Err(YieldMissing::PlayerRegistry));
        let mut mapping = [-1; 65];
        mapping[7] = 0;
        mapping[2] = 1;
        world.player_mapping = Some(mapping);
        assert_eq!(world.is_valid_player(7), Ok(true));
        assert_eq!(world.is_valid_player(2), Ok(true));
        assert_eq!(world.is_valid_player(3), Ok(false));
        let tech = PrerequisiteKind::Technology;
        assert_eq!(
            world.acquired_mask(2, tech),
            Err(YieldMissing::PlayerMask {
                player: 2,
                kind: tech
            })
        );
        assert_eq!(
            world.acquired_mask(7, tech),
            Err(YieldMissing::PlayerMask {
                player: 7,
                kind: tech
            })
        );
        world.player_mut(7).unwrap().technologies = Some(vec![true, false]);
        assert_eq!(world.acquired_mask(7, tech).unwrap(), [true, false]);
        assert_eq!(
            world.globally_available(tech, 0),
            Err(YieldMissing::GlobalAvailability {
                kind: tech,
                index: 0
            })
        );
        world.technology_availability = Some(vec![false, true]);
        assert_eq!(world.globally_available(tech, 0), Ok(false));
        assert_eq!(world.globally_available(tech, 1), Ok(true));
        assert_eq!(
            world.globally_available(tech, 2),
            Err(YieldMissing::GlobalAvailability {
                kind: tech,
                index: 2
            })
        );
    }
    #[test]
    fn owned_world_drives_numeric_technology_and_civic_bonus_helpers() {
        use crate::movement_world::DefinitionLookup;
        use crate::yields::{
            improvement_bonus_civic_yield, improvement_bonus_tech_yield, BonusYieldChange,
            ImprovementDefinition, MissingInput as YieldMissing, NativeDefinitionIndex,
            NativeFixedPoint8, OrderedImprovementDefinitions, PrerequisiteKind,
        };
        // Synthetic native-order registry and references, not an original-game
        // fixture. The production adapter receives its actual DefinitionLookup.
        struct Registry;
        impl DefinitionLookup for Registry {
            fn count(&self) -> usize {
                1
            }
            fn find_index(&self, value: u32) -> Option<usize> {
                (value == 0 || value == 0xabcdef01).then_some(0)
            }
            fn type_hash(&self, index: usize) -> Option<u32> {
                (index == 0).then_some(0xabcdef01)
            }
        }
        let index = |value| Some(NativeDefinitionIndex::new(value, 2).unwrap());
        let references = [
            Some(BonusYieldChange {
                yield_index: index(0),
                prerequisite_technology: index(0),
                prerequisite_civic: index(0),
                bonus_yield_change: 3,
            }),
            Some(BonusYieldChange {
                yield_index: index(0),
                prerequisite_technology: index(1),
                prerequisite_civic: index(1),
                bonus_yield_change: 7,
            }),
        ];
        let definitions = [ImprovementDefinition {
            type_hash: 0xabcdef01,
            bonus_yield_changes: Some(&references),
        }];
        let rules = OrderedImprovementDefinitions::new(&Registry, &definitions).unwrap();
        let mut world = World::new(map(), vec![Player::new(7, None, None).unwrap()]).unwrap();
        let mut mapping = [-1; 65];
        mapping[7] = 0;
        world.player_mapping = Some(mapping);
        world.player_mut(7).unwrap().technologies = Some(vec![true, false]);
        world.technology_availability = Some(vec![true, true]);
        assert_eq!(
            improvement_bonus_tech_yield(rules, &world, 7, 0xabcdef01, 0),
            Ok(NativeFixedPoint8(3 * 256))
        );
        world.player_mut(7).unwrap().technologies.as_mut().unwrap()[1] = true;
        assert_eq!(
            improvement_bonus_tech_yield(rules, &world, 7, 0xabcdef01, 0),
            Ok(NativeFixedPoint8(10 * 256))
        );
        world.technology_availability.as_mut().unwrap()[0] = false;
        assert_eq!(
            improvement_bonus_tech_yield(rules, &world, 7, 0xabcdef01, 0),
            Ok(NativeFixedPoint8(7 * 256))
        );
        world.technology_availability = None;
        assert_eq!(
            improvement_bonus_tech_yield(rules, &world, 7, 0xabcdef01, 0),
            Err(YieldMissing::GlobalAvailability {
                kind: PrerequisiteKind::Technology,
                index: 0
            })
        );
        world.player_mut(7).unwrap().civics = Some(vec![false, true]);
        world.civic_availability = Some(vec![true, true]);
        assert_eq!(
            improvement_bonus_civic_yield(rules, &world, 7, 0xabcdef01, 0),
            Ok(NativeFixedPoint8(7 * 256))
        );
        world.player_mut(7).unwrap().civics = None;
        assert_eq!(
            improvement_bonus_civic_yield(rules, &world, 7, 0xabcdef01, 0),
            Err(YieldMissing::PlayerMask {
                player: 7,
                kind: PrerequisiteKind::Civic
            })
        );
    }
}

pub mod unit_creation;
