//! Owned bounded Linux world records, not complete live game reconstruction.
//!
//! Collections must be complete typed exports in their native iteration order.
//! Explicit recycled slots, never generated IDs, supply lookup storage. Rules,
//! module and configuration identity gates belong to the importer. See
//! `docs/linux-world-state.md` for the evidence and unsupported live state.
use crate::map::{Map, Plot, Recycled, UnitReference};
use crate::unit::{Formation, Unit};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MissingInput {
    Player { owner: i32 },
    PlayerRegistry,
    Units { owner: i32 },
    Formations { owner: i32 },
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct Collection<T> {
    records: Recycled<T>,
    by_slot: Vec<Option<usize>>,
}
impl<T> Collection<T> {
    fn new(records: Recycled<T>) -> Result<Self, MissingInput> {
        if records.capacity > 65536 {
            return Err(MissingInput::InvalidSlot);
        }
        let mut by_slot = vec![None; records.capacity as usize];
        for (i, entry) in records.entries.iter().enumerate() {
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
        Some(&self.records.entries.get(self.index(id)?)?.value)
    }
    fn get_mut(&mut self, id: u32) -> Option<&mut T> {
        let index = self.index(id)?;
        Some(&mut self.records.entries.get_mut(index)?.value)
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
    pub fn units(&self) -> Result<&Recycled<Unit>, MissingInput> {
        Ok(&self
            .units
            .as_ref()
            .ok_or(MissingInput::Units { owner: self.owner })?
            .records)
    }
    pub fn formations(&self) -> Result<&Recycled<Formation>, MissingInput> {
        Ok(&self
            .formations
            .as_ref()
            .ok_or(MissingInput::Formations { owner: self.owner })?
            .records)
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
        })
    }
    pub fn map(&self) -> &Map {
        &self.map
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
    /// Original CManager::GetUnit: invalid identity/player returns absent. Type
    /// bits do not gate this lookup; layer membership equality retains them.
    pub fn unit(&self, reference: UnitReference) -> Result<Option<Found<'_, Unit>>, MissingInput> {
        let owner = reference.player();
        if owner == -1 || reference.id == u32::MAX || !self.player_is_valid(owner)? {
            return Ok(None);
        }
        self.player(owner)?.unit(reference.id)
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
mod tests {
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
    fn map() -> Map {
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
