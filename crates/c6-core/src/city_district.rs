//! Verified Linux XP2 movement-relevant City/District state, not full instances.
//!
//! Slot tables borrow an authoritative export. Missing tables are unavailable,
//! whereas an empty complete table proves absence. No SQL ordinal assumptions,
//! native heap addresses, allocator reconstruction, or complete city codec.
use crate::map::{CityAttributes, Map, PlotFlag};
use crate::movement_world::{
    DefinitionLookup, DefinitionTable, DistrictInstance, MissingInput, RuntimePlotInputs,
    TerrainProperties,
};
use crate::unit::PlotCoord;

pub const PLAYER_COUNT: usize = 65;
pub const MAX_SLOTS: usize = 1 << 16;
pub const CITY_CENTER_HASH: u32 = 0x8a32_007d;

/// The original eight-byte union's scalar value (not a live native object).
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComponentId {
    pub owner: i16,
    pub tag: i16,
    pub id: u32,
}
impl ComponentId {
    pub fn city(owner: i32, id: u32) -> Self {
        Self {
            owner: owner as i16,
            tag: 2,
            id,
        }
    }
    pub fn district(owner: i32, id: u32) -> Self {
        Self {
            owner: owner as i16,
            tag: 3,
            id,
        }
    }
    /// Generic native validity does not test the tag or player range.
    pub fn is_valid(self) -> bool {
        self.owner != -1 && self.id != u32::MAX
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    InvalidSlotTable,
    UnavailablePlayer { player: usize },
    UnavailableCities { player: usize },
    UnavailableDistricts { player: usize },
    DefinitionMapping,
    PlotAttributes { plot: usize },
    PlotOutsideMap { plot: usize },
    MissingWater { plot: usize },
    MissingCoastalLand { plot: usize },
    MissingWonderCompletion { plot: usize },
    MissingAdjacency { plot: usize, direction: u8 },
    InvalidAdjacentPlot { plot: usize },
    ResourceLimit,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "missing or inconsistent Linux City/District input: {self:?}"
        )
    }
}
impl std::error::Error for Error {}

/// Movement-relevant projection only. Later City fields remain unsupported.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CityRecord {
    pub id: u32,
    pub location: PlotCoord,
    pub owner: i32,
}
/// Type is the actual runtime definition index, not an instance ID or SQL rowid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DistrictRecord {
    pub id: u32,
    /// Actual District.GetPlot().GetOwner(), not inferred from requested owner.
    pub owner: i32,
    pub location: PlotCoord,
    pub parent_city: u32,
    pub type_index: i32,
    pub complete: bool,
}
impl DistrictRecord {
    /// Original DistrictTypes hash-to-index overlay. Unknown hashes become -1;
    /// an inconsistent provider result remains an error, not a missing district.
    pub fn from_saved_type(
        id: u32,
        owner: i32,
        location: PlotCoord,
        parent_city: u32,
        definition_hash: u32,
        complete: bool,
        definitions: &dyn DefinitionLookup,
    ) -> Result<Self, Error> {
        let type_index = remap_type(definition_hash, definitions)?;
        Ok(Self {
            id,
            owner,
            location,
            parent_city,
            type_index,
            complete,
        })
    }
}
fn remap_type(key: u32, definitions: &dyn DefinitionLookup) -> Result<i32, Error> {
    if definitions.count() > i32::MAX as usize {
        return Err(Error::DefinitionMapping);
    }
    match definitions.find_index(key) {
        Some(index) if index < definitions.count() => Ok(index as i32),
        Some(_) => Err(Error::DefinitionMapping),
        None => Ok(-1),
    }
}

pub trait InstanceRecord {
    fn instance_id(&self) -> u32;
}
impl InstanceRecord for CityRecord {
    fn instance_id(&self) -> u32 {
        self.id
    }
}
impl InstanceRecord for DistrictRecord {
    fn instance_id(&self) -> u32 {
        self.id
    }
}
/// Complete nonnull slots, sorted by low sixteen ID bits. Null/unallocated
/// slots need no storage. At most 65536 records; lookup performs no allocation.
#[derive(Debug)]
pub struct SlotTable<'a, T> {
    records: &'a [T],
}
impl<T> Copy for SlotTable<'_, T> {}
impl<T> Clone for SlotTable<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<'a, T: InstanceRecord> SlotTable<'a, T> {
    pub fn new(records: &'a [T]) -> Result<Self, Error> {
        if records.len() > MAX_SLOTS {
            return Err(Error::InvalidSlotTable);
        }
        let mut previous = None;
        for record in records {
            let id = record.instance_id();
            let slot = id as u16;
            if id == u32::MAX || previous.is_some_and(|p| p >= slot) {
                return Err(Error::InvalidSlotTable);
            }
            previous = Some(slot);
        }
        Ok(Self { records })
    }
    pub fn find(&self, requested_id: u32) -> Option<&'a T> {
        if requested_id == u32::MAX {
            return None;
        }
        let slot = requested_id as u16;
        self.records
            .binary_search_by_key(&slot, |r| r.instance_id() as u16)
            .ok()
            .map(|index| &self.records[index])
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PlayerRegistry<'a> {
    /// The native valid-player mapping was not supplied.
    Unavailable,
    /// The native valid-player mapping proves this player absent.
    Invalid,
    Valid {
        /// None means unavailable, not an empty container.
        cities: Option<SlotTable<'a, CityRecord>>,
        districts: Option<SlotTable<'a, DistrictRecord>>,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstanceMatch<'a, T> {
    pub requested: ComponentId,
    pub resolved: ComponentId,
    pub record: &'a T,
}
pub type CityMatch<'a> = InstanceMatch<'a, CityRecord>;
pub type DistrictMatch<'a> = InstanceMatch<'a, DistrictRecord>;

pub struct Registry<'a> {
    players: [PlayerRegistry<'a>; PLAYER_COUNT],
}
impl<'a> Registry<'a> {
    pub fn new(players: [PlayerRegistry<'a>; PLAYER_COUNT]) -> Self {
        Self { players }
    }
    fn player(&self, id: ComponentId) -> Result<Option<PlayerRegistry<'a>>, Error> {
        if !id.is_valid() || !(0..PLAYER_COUNT as i16).contains(&id.owner) {
            return Ok(None);
        }
        let player = id.owner as usize;
        match self.players[player] {
            PlayerRegistry::Unavailable => Err(Error::UnavailablePlayer { player }),
            PlayerRegistry::Invalid => Ok(None),
            entry => Ok(Some(entry)),
        }
    }
    /// Native generic CManager lookup ignores the supplied component tag.
    pub fn find_city(&self, requested: ComponentId) -> Result<Option<CityMatch<'a>>, Error> {
        let Some(PlayerRegistry::Valid { cities, .. }) = self.player(requested)? else {
            return Ok(None);
        };
        let table = cities.ok_or(Error::UnavailableCities {
            player: requested.owner as usize,
        })?;
        Ok(table.find(requested.id).map(|record| InstanceMatch {
            requested,
            resolved: ComponentId::city(record.owner, record.id),
            record,
        }))
    }
    pub fn find_district(
        &self,
        requested: ComponentId,
    ) -> Result<Option<DistrictMatch<'a>>, Error> {
        let Some(PlayerRegistry::Valid { districts, .. }) = self.player(requested)? else {
            return Ok(None);
        };
        let table = districts.ok_or(Error::UnavailableDistricts {
            player: requested.owner as usize,
        })?;
        Ok(table.find(requested.id).map(|record| InstanceMatch {
            requested,
            resolved: ComponentId::district(record.owner, record.id),
            record,
        }))
    }
    pub fn district(&self, city: &CityAttributes) -> Result<Option<DistrictMatch<'a>>, Error> {
        self.find_district(ComponentId::district(i32::from(city.player), city.district))
    }
    pub fn owning_city(&self, city: &CityAttributes) -> Result<Option<CityMatch<'a>>, Error> {
        self.find_city(ComponentId::city(i32::from(city.player), city.owning_city))
    }
    pub fn working_city(&self, city: &CityAttributes) -> Result<Option<CityMatch<'a>>, Error> {
        self.find_city(ComponentId::city(i32::from(city.player), city.working_city))
    }
    /// District.GetCity uses the District's actual plot owner and parent ID.
    pub fn district_city(&self, district: &DistrictRecord) -> Result<Option<CityMatch<'a>>, Error> {
        self.find_city(ComponentId::city(district.owner, district.parent_city))
    }
    pub fn is_city(
        &self,
        city: &CityAttributes,
        definitions: &dyn DefinitionLookup,
    ) -> Result<bool, Error> {
        let index = self.district(city)?.map_or(-1, |d| d.record.type_index);
        if index == -1 {
            return Ok(false);
        }
        Ok(definitions
            .type_hash(index as u32 as usize)
            .ok_or(Error::DefinitionMapping)?
            == CITY_CENTER_HASH)
    }
}

/// Native i16 mask, including bits not named by the five current domains.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DomainMask(pub i16);
impl DomainMask {
    pub fn is_valid(self, domain: i32) -> bool {
        let bit = match domain {
            0 => 1,
            1 => 2,
            2 => 4,
            4 => 16,
            _ => 8,
        };
        i32::from(self.0) & bit != 0
    }
}
/// Ready-to-use movement adapter. Masks are genuine live exports or results of
/// the initialization rebuild below. None means unavailable, never mask zero.
pub struct MovementRuntime<'a, 'r> {
    pub registry: &'r Registry<'a>,
    pub domain_masks: &'r [Option<DomainMask>],
}
impl RuntimePlotInputs for MovementRuntime<'_, '_> {
    fn district(&self, city: &CityAttributes) -> Result<Option<DistrictInstance>, MissingInput> {
        self.registry
            .district(city)
            .map(|d| {
                d.map(|d| DistrictInstance {
                    type_index: d.record.type_index,
                    complete: d.record.complete,
                })
            })
            .map_err(|_| MissingInput::DistrictLookup {
                player: city.player,
                district: city.district,
            })
    }
    fn domain_valid(&self, plot: usize, domain: i32) -> Result<bool, MissingInput> {
        self.domain_masks
            .get(plot)
            .copied()
            .flatten()
            .map(|m| m.is_valid(domain))
            .ok_or(MissingInput::CityDomainValidity { plot, domain })
    }
}

/// Actual getter properties in the same order as the native District registry.
/// `canal` is false for a proven absent XP2 definition, not an unknown export.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DistrictDomainProperties {
    pub canal: bool,
    pub internal_only: bool,
}
/// Actual XP2 Building getter results; proven absent XP2 definition means false.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BuildingDomainProperties {
    pub canal_wonder: bool,
    pub bridge: bool,
}
pub struct RebuildDefinitions<'a> {
    pub districts: DefinitionTable<'a, DistrictDomainProperties>,
    pub buildings: DefinitionTable<'a, BuildingDomainProperties>,
}
/// Source-closed original Plot.IsWater query from typed Map and actual native
/// Terrain collection. Built-in indexes 15/16 precede the optional definition.
pub fn map_is_water(
    map: &Map,
    plot: usize,
    terrains: &DefinitionTable<'_, TerrainProperties>,
) -> Result<bool, Error> {
    let plot_data = map.plots.get(plot).ok_or(Error::PlotOutsideMap { plot })?;
    let index = terrains
        .plot_index(plot_data.terrain_hash)
        .map_err(|_| Error::DefinitionMapping)?;
    Ok(index == 15 || index == 16 || terrains.at(index).is_some_and(|t| t.is_water))
}

/// Dependencies not yet reconstructible from Map + this bounded registry alone.
/// Supply exact, stable native/rules queries, or retain the named missing error.
/// `map_is_water` supplies the closed water query. In particular the coastal
/// query is IsCoastalLand(1), not an approximate adjacent-water test.
pub trait DomainRebuildInputs {
    fn is_water(&self, plot: usize) -> Result<bool, Error> {
        Err(Error::MissingWater { plot })
    }
    fn is_coastal_land_one(&self, plot: usize) -> Result<bool, Error> {
        Err(Error::MissingCoastalLand { plot })
    }
    fn is_wonder_complete(&self, plot: usize) -> Result<bool, Error> {
        Err(Error::MissingWonderCompletion { plot })
    }
    /// Original direction order 0..5. None proves no plot in that direction.
    fn adjacent_plot(&self, plot: usize, direction: u8) -> Result<Option<usize>, Error> {
        Err(Error::MissingAdjacency { plot, direction })
    }
}

fn attributes(map: &Map, plot: usize) -> Result<Option<&CityAttributes>, Error> {
    let p = &map.plots[plot];
    if p.flags.get(PlotFlag::HasCityAttributes) != p.city_attributes.is_some() {
        return Err(Error::PlotAttributes { plot });
    }
    Ok(p.city_attributes.as_ref())
}
fn mark_adjacent_cities(
    map: &Map,
    registry: &Registry<'_>,
    definitions: &dyn DefinitionLookup,
    inputs: &impl DomainRebuildInputs,
    masks: &mut [Option<DomainMask>],
    plot: usize,
) -> Result<(), Error> {
    for direction in 0..6 {
        if let Some(adjacent) = inputs.adjacent_plot(plot, direction)? {
            if adjacent >= map.plots.len() {
                return Err(Error::InvalidAdjacentPlot { plot: adjacent });
            }
            if let Some(city) = attributes(map, adjacent)? {
                if registry.is_city(city, definitions)? {
                    let mask = masks[adjacent]
                        .as_mut()
                        .ok_or(Error::PlotAttributes { plot: adjacent })?;
                    mask.0 |= 1;
                }
            }
        }
    }
    Ok(())
}

/// Original post-load base pass followed by XP2 augmentation in plot-index order.
/// Bounded O(plots + actual adjacency queries), one mask vector, no Map clone.
/// On any missing input returns no partially usable vector. This does not apply
/// construction/pillage/destruction/transfer updates after initialization.
pub fn rebuild_valid_domains(
    map: &Map,
    registry: &Registry<'_>,
    definitions: &RebuildDefinitions<'_>,
    inputs: &impl DomainRebuildInputs,
    max_plots: usize,
) -> Result<Vec<Option<DomainMask>>, Error> {
    if map.plots.len() > max_plots {
        return Err(Error::ResourceLimit);
    }
    let mut masks = Vec::new();
    masks
        .try_reserve_exact(map.plots.len())
        .map_err(|_| Error::ResourceLimit)?;
    for plot in 0..map.plots.len() {
        let mask = if let Some(city) = attributes(map, plot)? {
            let water = inputs.is_water(plot)?;
            let sea = water
                || (registry.is_city(city, definitions.districts.lookup)?
                    && inputs.is_coastal_land_one(plot)?);
            // Native clears then sets SEA, LAND, HOVER, AIR; IMMOBILE stays clear.
            Some(DomainMask(
                i16::from(sea) | (i16::from(!water) << 2) | 16 | 2,
            ))
        } else {
            None
        };
        masks.push(mask);
    }
    for plot in 0..map.plots.len() {
        let Some(city) = attributes(map, plot)? else {
            continue;
        };
        let Some(district) = registry.district(city)? else {
            continue;
        };
        let index = district.record.type_index;
        if index == -1 || !district.record.complete {
            continue;
        }
        let district_properties = definitions
            .districts
            .at(index)
            .ok_or(Error::DefinitionMapping)?;
        if district_properties.canal {
            masks[plot]
                .as_mut()
                .ok_or(Error::PlotAttributes { plot })?
                .0 |= 1;
            mark_adjacent_cities(
                map,
                registry,
                definitions.districts.lookup,
                inputs,
                &mut masks,
                plot,
            )?;
        }
        if district_properties.internal_only {
            // Packed BuildingTypes is a signed short after hash remapping.
            let wonder_index =
                remap_type(city.wonder_building_hash, definitions.buildings.lookup)? as i16 as i32;
            let wonder = if wonder_index == -1 {
                None
            } else {
                Some(
                    definitions
                        .buildings
                        .at(wonder_index)
                        .ok_or(Error::DefinitionMapping)?,
                )
            };
            // Canal Wonder does not require IsWonderComplete in the original.
            if wonder.is_some_and(|w| w.canal_wonder) {
                masks[plot]
                    .as_mut()
                    .ok_or(Error::PlotAttributes { plot })?
                    .0 |= 1;
                mark_adjacent_cities(
                    map,
                    registry,
                    definitions.districts.lookup,
                    inputs,
                    &mut masks,
                    plot,
                )?;
            }
            if inputs.is_wonder_complete(plot)? {
                let wonder = wonder.ok_or(Error::DefinitionMapping)?;
                if wonder.bridge {
                    masks[plot]
                        .as_mut()
                        .ok_or(Error::PlotAttributes { plot })?
                        .0 |= 4;
                }
            }
        }
    }
    Ok(masks)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Definitions(&'static [u32]);
    impl DefinitionLookup for Definitions {
        fn count(&self) -> usize {
            self.0.len()
        }
        fn find_index(&self, key: u32) -> Option<usize> {
            if (key as usize) < self.0.len() {
                Some(key as usize)
            } else {
                self.0.iter().rposition(|&hash| hash == key)
            }
        }
        fn type_hash(&self, index: usize) -> Option<u32> {
            self.0.get(index).copied()
        }
    }
    fn city_attributes(id: u32) -> CityAttributes {
        CityAttributes {
            working_city: 9,
            owning_city: 9,
            district: id,
            player: 0,
            wonder_building_hash: u32::MAX,
        }
    }
    fn district(id: u32, index: i32, complete: bool) -> DistrictRecord {
        DistrictRecord {
            id,
            owner: 0,
            location: PlotCoord { x: 0, y: 0 },
            parent_city: 9,
            type_index: index,
            complete,
        }
    }
    fn registry(records: &[DistrictRecord]) -> Registry<'_> {
        let mut players = [PlayerRegistry::Unavailable; PLAYER_COUNT];
        players[0] = PlayerRegistry::Valid {
            cities: Some(SlotTable::new(&[]).unwrap()),
            districts: Some(SlotTable::new(records).unwrap()),
        };
        Registry::new(players)
    }
    #[test]
    fn native_identity_and_slot_alias_preserve_requested_and_actual_owners() {
        assert_eq!(std::mem::size_of::<ComponentId>(), 8);
        assert_eq!(std::mem::align_of::<ComponentId>(), 8);
        let mut record = district(0x1234_0005, 0, false);
        record.owner = 3;
        let records = [record];
        let mut players = [PlayerRegistry::Unavailable; PLAYER_COUNT];
        players[64] = PlayerRegistry::Valid {
            cities: None,
            districts: Some(SlotTable::new(&records).unwrap()),
        };
        let registry = Registry::new(players);
        let requested = ComponentId {
            owner: 64,
            tag: -7,
            id: 0xbeef_0005,
        };
        let found = registry.find_district(requested).unwrap().unwrap();
        assert_eq!(found.requested, requested); // Native lookup ignores tag and high counter.
        assert_eq!(found.resolved, ComponentId::district(3, record.id));
        assert_eq!(
            registry
                .find_district(ComponentId::district(64, 6))
                .unwrap(),
            None
        );
        assert_eq!(
            registry
                .find_district(ComponentId::district(65, 5))
                .unwrap(),
            None
        );
        assert_eq!(
            registry
                .find_district(ComponentId::district(0, u32::MAX))
                .unwrap(),
            None
        );
        assert_eq!(
            registry.find_district(ComponentId::district(0, 5)),
            Err(Error::UnavailablePlayer { player: 0 })
        );
        assert_eq!(
            registry.find_city(ComponentId::city(64, 5)),
            Err(Error::UnavailableCities { player: 64 })
        );
        assert_eq!(ComponentId::city(0x1_0040, 5).owner, 64);
    }
    #[test]
    fn complete_absence_and_missing_registry_are_distinct() {
        let mut players = [PlayerRegistry::Invalid; PLAYER_COUNT];
        players[1] = PlayerRegistry::Valid {
            cities: None,
            districts: None,
        };
        let registry = Registry::new(players);
        assert_eq!(
            registry.find_district(ComponentId::district(0, 9)).unwrap(),
            None
        );
        assert_eq!(
            registry.find_district(ComponentId::district(1, 9)),
            Err(Error::UnavailableDistricts { player: 1 })
        );
        let duplicate = [district(0x10002, 0, true), district(0x20002, 0, true)];
        assert!(matches!(
            SlotTable::new(&duplicate),
            Err(Error::InvalidSlotTable)
        ));
        assert!(matches!(
            SlotTable::new(&[district(u32::MAX, 0, true)]),
            Err(Error::InvalidSlotTable)
        ));
    }
    #[test]
    fn city_classification_uses_definition_hash_without_completion() {
        let definitions = Definitions(&[55, CITY_CENTER_HASH]);
        let record = DistrictRecord::from_saved_type(
            5,
            0,
            PlotCoord { x: 2, y: 3 },
            9,
            CITY_CENTER_HASH,
            false,
            &definitions,
        )
        .unwrap();
        let records = [record];
        let registry = registry(&records);
        assert!(registry.is_city(&city_attributes(5), &definitions).unwrap());
        assert!(!registry.is_city(&city_attributes(6), &definitions).unwrap());
        assert_eq!(
            registry.is_city(&city_attributes(5), &Definitions(&[])),
            Err(Error::DefinitionMapping)
        );
        let missing =
            DistrictRecord::from_saved_type(6, 0, record.location, 9, 999, true, &definitions)
                .unwrap();
        assert_eq!(missing.type_index, -1);
        let masks = [Some(DomainMask(0x8008u16 as i16))];
        let runtime = MovementRuntime {
            registry: &registry,
            domain_masks: &masks,
        };
        assert_eq!(
            runtime.district(&city_attributes(5)).unwrap(),
            Some(DistrictInstance {
                type_index: 1,
                complete: false
            })
        );
        assert!(runtime.domain_valid(0, -1).unwrap());
        assert!(!runtime.domain_valid(0, 0).unwrap());
        assert!(runtime.domain_valid(1, 0).is_err());
    }
    fn empty<T>() -> crate::map::Recycled<T> {
        crate::map::Recycled {
            capacity: 0,
            entries: vec![],
            current_id_bits: 0,
            free_head: 0,
            free_tail: 0,
            free_links: vec![],
        }
    }
    fn map_fixture() -> Map {
        let mut plot =
            crate::map::Plot::decode(&[0; 55], 6, crate::map::Limits::default()).unwrap();
        plot.flags.set(PlotFlag::HasCityAttributes, true);
        let mut plots = vec![plot; 4];
        for (id, p) in plots.iter_mut().enumerate() {
            p.city_attributes = Some(city_attributes(id as u32));
        }
        plots[2]
            .city_attributes
            .as_mut()
            .unwrap()
            .wonder_building_hash = 100;
        plots[3]
            .city_attributes
            .as_mut()
            .unwrap()
            .wonder_building_hash = 200;
        Map {
            header: crate::map::Header {
                width: 4,
                height: 1,
                top_latitude: 90,
                bottom_latitude: -90,
                wrap_x: false,
                wrap_y: false,
                map_size_hash: 0,
                land_plots: 3,
                owned_plots: 0,
                natural_wonders: 0,
                resource_counts: vec![],
                land_resource_counts: vec![],
            },
            visibility: vec![],
            areas: empty(),
            landmasses: empty(),
            regions: empty(),
            chokepoints: empty(),
            continents: vec![],
            feature_layouts: empty(),
            plots,
            units: crate::map::UnitContainer {
                width: 4,
                height: 1,
                layers: vec![],
            },
            components: vec![],
        }
    }
    #[test]
    fn map_water_uses_actual_terrain_indexes_and_properties() {
        let mut map = map_fixture();
        let lookup = Definitions(&[
            100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116,
        ]);
        let mut properties = [TerrainProperties {
            movement_cost: 1,
            is_water: false,
        }; 17];
        properties[0].is_water = true;
        let terrains = DefinitionTable::new(&lookup, &properties).unwrap();
        map.plots[0].terrain_hash = 100;
        map.plots[1].terrain_hash = 15;
        map.plots[2].terrain_hash = 999;
        assert!(map_is_water(&map, 0, &terrains).unwrap());
        assert!(map_is_water(&map, 1, &terrains).unwrap());
        assert!(!map_is_water(&map, 2, &terrains).unwrap());
        assert_eq!(
            map_is_water(&map, 4, &terrains),
            Err(Error::PlotOutsideMap { plot: 4 })
        );
    }
    struct Inputs;
    impl DomainRebuildInputs for Inputs {
        fn is_water(&self, plot: usize) -> Result<bool, Error> {
            Ok(plot == 3)
        }
        fn is_coastal_land_one(&self, _: usize) -> Result<bool, Error> {
            Ok(false)
        }
        fn is_wonder_complete(&self, plot: usize) -> Result<bool, Error> {
            Ok(plot == 3)
        }
        fn adjacent_plot(&self, plot: usize, direction: u8) -> Result<Option<usize>, Error> {
            Ok((plot == 1 && direction == 0).then_some(0))
        }
    }
    #[test]
    fn post_load_two_pass_rebuild_keeps_incomplete_city_canal_wonder_and_bridge_rules() {
        let map = map_fixture();
        let records = [
            district(0, 0, false),
            district(1, 1, true),
            district(2, 2, true),
            district(3, 2, true),
        ];
        let registry = registry(&records);
        let districts = Definitions(&[CITY_CENTER_HASH, 77, 88]);
        let buildings = Definitions(&[100, 200]);
        let district_properties = [
            DistrictDomainProperties {
                canal: false,
                internal_only: false,
            },
            DistrictDomainProperties {
                canal: true,
                internal_only: false,
            },
            DistrictDomainProperties {
                canal: false,
                internal_only: true,
            },
        ];
        let building_properties = [
            BuildingDomainProperties {
                canal_wonder: true,
                bridge: false,
            },
            BuildingDomainProperties {
                canal_wonder: false,
                bridge: true,
            },
        ];
        let definitions = RebuildDefinitions {
            districts: DefinitionTable::new(&districts, &district_properties).unwrap(),
            buildings: DefinitionTable::new(&buildings, &building_properties).unwrap(),
        };
        // Plot0 is an incomplete City Center: base22, then adjacent Canal adds Sea.
        // Plot2's Canal Wonder adds Sea despite IsWonderComplete=false.
        // Plot3's complete bridge adds Land to its water mask.
        assert_eq!(
            rebuild_valid_domains(&map, &registry, &definitions, &Inputs, 4).unwrap(),
            vec![Some(DomainMask(23)); 4]
        );
        assert_eq!(
            rebuild_valid_domains(&map, &registry, &definitions, &Inputs, 3),
            Err(Error::ResourceLimit)
        );
        struct Missing;
        impl DomainRebuildInputs for Missing {}
        assert_eq!(
            rebuild_valid_domains(&map, &registry, &definitions, &Missing, 4),
            Err(Error::MissingWater { plot: 0 })
        );
        struct NoCoastal;
        impl DomainRebuildInputs for NoCoastal {
            fn is_water(&self, _: usize) -> Result<bool, Error> {
                Ok(false)
            }
        }
        assert_eq!(
            rebuild_valid_domains(&map, &registry, &definitions, &NoCoastal, 4),
            Err(Error::MissingCoastalLand { plot: 0 })
        );
        let mut broken = map_fixture();
        broken.plots[0]
            .flags
            .set(PlotFlag::HasCityAttributes, false);
        assert_eq!(
            rebuild_valid_domains(&broken, &registry, &definitions, &Inputs, 4),
            Err(Error::PlotAttributes { plot: 0 })
        );
    }
}
