//! Serialized Linux movement identity projection. Not full District state or
//! domain reconstruction: native definition order and player validity are inputs.
use crate::registry::LinuxDefinitionHashRegistry;
use c6_core::city::City;
use c6_core::city_district::{self, CityRecord, DistrictRecord};
use c6_core::city_district::{
    BuildingDomainProperties, DistrictDomainProperties, PlayerRegistry, SlotTable, PLAYER_COUNT,
};
use c6_core::city_district::{DomainMask, DomainRebuildInputs, RebuildDefinitions, Registry};
use c6_core::city_production::Buildings;
use c6_core::city_wire::Header;
use c6_core::map::{CityAttributes, Landmass, Map, PlotFlag};
use c6_core::movement_world::{DefinitionLookup, DefinitionTable, TerrainProperties};
use c6_core::unit::PlotCoord;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Version(u32),
    InvalidBool(u8),
    Location,
    Attributes,
    MapGeometry,
    LandmassTable,
    CityTable,
    ResourceLimit,
    MetadataRegistry,
    MetadataPlayer,
    Membership,
    UnmatchedSerializedInstance,
    Registry(city_district::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux serialized movement projection: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Genuine definition metadata only. Native plot_expected fields have no type
/// here and cannot populate any prerequisite of the calculation.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainRegistryExport {
    pub kind: String,
    pub name: String,
    pub count: u32,
    pub complete: bool,
    pub entries: Vec<DomainDefinitionExport>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainDefinitionExport {
    pub query: u32,
    pub index: u32,
    pub hash: u32,
    pub find_index: Option<u32>,
    pub direct_index: Option<u32>,
    pub internal_only: Option<bool>,
    pub xp2: Xp2Export,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Xp2Export {
    Absent {},
    Present {
        is_canal: Option<bool>,
        canal_wonder: Option<bool>,
        bridge: Option<bool>,
    },
}
pub struct NativeDomainDefinitions {
    districts: LinuxDefinitionHashRegistry,
    district_properties: Vec<DistrictDomainProperties>,
    buildings: LinuxDefinitionHashRegistry,
    building_properties: Vec<BuildingDomainProperties>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainRegistryExport {
    pub kind: String,
    pub name: String,
    pub count: u32,
    pub complete: bool,
    pub entries: Vec<TerrainDefinitionExport>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainDefinitionExport {
    pub query: u32,
    pub index: u32,
    pub hash: u32,
    pub find_index: Option<u32>,
    pub direct_index: Option<u32>,
    pub is_water: bool,
    pub movement_cost: i32,
}
pub struct NativeTerrainDefinitions {
    lookup: LinuxDefinitionHashRegistry,
    properties: Vec<TerrainProperties>,
}
impl NativeTerrainDefinitions {
    pub fn from_export(export: &TerrainRegistryExport) -> Result<Self, Error> {
        if export.kind != "registry"
            || export.name != "terrain"
            || !export.complete
            || export.count > 65536
            || export.count as usize != export.entries.len()
        {
            return Err(Error::MetadataRegistry);
        }
        let lookup = LinuxDefinitionHashRegistry::from_native_order(
            export.entries.iter().map(|entry| entry.hash).collect(),
        );
        let mut properties = Vec::new();
        properties
            .try_reserve_exact(export.entries.len())
            .map_err(|_| Error::ResourceLimit)?;
        for (position, entry) in export.entries.iter().enumerate() {
            if entry.query as usize != position
                || entry.index != entry.query
                || entry.direct_index != Some(entry.index)
                || entry.find_index.map(|index| index as usize) != lookup.find_index(entry.hash)
            {
                return Err(Error::MetadataRegistry);
            }
            properties.push(TerrainProperties {
                is_water: entry.is_water,
                movement_cost: entry.movement_cost,
            });
        }
        Ok(Self { lookup, properties })
    }
    pub fn as_core(&self) -> Result<DefinitionTable<'_, TerrainProperties>, Error> {
        DefinitionTable::new(&self.lookup, &self.properties).map_err(|_| Error::MetadataRegistry)
    }
}
impl NativeDomainDefinitions {
    /// Count/At, collection-index and original Find observations must agree with
    /// the currently supported collection contract. Exotic layouts are errors.
    /// Source/module/profile authentication remains the enclosing import gate.
    pub fn from_exports(exports: &[DomainRegistryExport]) -> Result<Self, Error> {
        if exports.len() != 2 {
            return Err(Error::MetadataRegistry);
        }
        let find = |name| {
            let mut matches = exports.iter().filter(|export| export.name == name);
            let export = matches.next().ok_or(Error::MetadataRegistry)?;
            if matches.next().is_some() {
                return Err(Error::MetadataRegistry);
            }
            Ok(export)
        };
        let district = find("district")?;
        let building = find("building")?;
        fn lookup(export: &DomainRegistryExport) -> Result<LinuxDefinitionHashRegistry, Error> {
            if export.kind != "registry"
                || !export.complete
                || export.count > 65536
                || export.count as usize != export.entries.len()
            {
                return Err(Error::MetadataRegistry);
            }
            let lookup = LinuxDefinitionHashRegistry::from_native_order(
                export.entries.iter().map(|entry| entry.hash).collect(),
            );
            for (position, entry) in export.entries.iter().enumerate() {
                if entry.query as usize != position
                    || entry.index != entry.query
                    || entry.direct_index != Some(entry.index)
                    || entry.find_index.map(|index| index as usize) != lookup.find_index(entry.hash)
                {
                    return Err(Error::MetadataRegistry);
                }
            }
            Ok(lookup)
        }
        let districts = lookup(district)?;
        let buildings = lookup(building)?;
        let mut district_properties = Vec::new();
        district_properties
            .try_reserve_exact(district.entries.len())
            .map_err(|_| Error::ResourceLimit)?;
        for entry in &district.entries {
            let internal_only = entry.internal_only.ok_or(Error::MetadataRegistry)?;
            let canal = match entry.xp2 {
                Xp2Export::Absent {} => false,
                Xp2Export::Present {
                    is_canal: Some(canal),
                    canal_wonder: None,
                    bridge: None,
                } => canal,
                _ => return Err(Error::MetadataRegistry),
            };
            district_properties.push(DistrictDomainProperties {
                canal,
                internal_only,
            });
        }
        let mut building_properties = Vec::new();
        building_properties
            .try_reserve_exact(building.entries.len())
            .map_err(|_| Error::ResourceLimit)?;
        for entry in &building.entries {
            if entry.internal_only.is_some() {
                return Err(Error::MetadataRegistry);
            }
            let properties = match entry.xp2 {
                Xp2Export::Absent {} => BuildingDomainProperties {
                    canal_wonder: false,
                    bridge: false,
                },
                Xp2Export::Present {
                    is_canal: None,
                    canal_wonder: Some(canal_wonder),
                    bridge: Some(bridge),
                } => BuildingDomainProperties {
                    canal_wonder,
                    bridge,
                },
                _ => return Err(Error::MetadataRegistry),
            };
            building_properties.push(properties);
        }
        Ok(Self {
            districts,
            district_properties,
            buildings,
            building_properties,
        })
    }
    pub fn district_lookup(&self) -> &dyn DefinitionLookup {
        &self.districts
    }
    pub fn building_lookup(&self) -> &dyn DefinitionLookup {
        &self.buildings
    }
    pub fn as_core(&self) -> Result<RebuildDefinitions<'_>, Error> {
        Ok(RebuildDefinitions {
            districts: DefinitionTable::new(&self.districts, &self.district_properties)
                .map_err(|_| Error::MetadataRegistry)?,
            buildings: DefinitionTable::new(&self.buildings, &self.building_properties)
                .map_err(|_| Error::MetadataRegistry)?,
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerMetadata {
    pub kind: String,
    pub query: u32,
    pub mapped_index: i32,
    pub is_valid: bool,
    pub resolved: bool,
    pub actual_owner: Option<i32>,
    pub cities: Option<MembershipExport>,
    pub districts: Option<MembershipExport>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MembershipExport {
    pub count: u32,
    pub entries: Vec<MembershipEntry>,
    pub complete: bool,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MembershipEntry {
    pub ordinal: u32,
    pub id: u32,
    pub slot: u32,
    pub actual_owner: i32,
}
impl MembershipExport {
    fn validate(&self) -> Result<(), Error> {
        if !self.complete || self.count > 65536 || self.count as usize != self.entries.len() {
            return Err(Error::Membership);
        }
        for (ordinal, entry) in self.entries.iter().enumerate() {
            if entry.ordinal as usize != ordinal
                || entry.id == u32::MAX
                || entry.slot != entry.id & 65535
            {
                return Err(Error::Membership);
            }
        }
        Ok(())
    }
}
#[derive(Default)]
struct PlayerRecords {
    cities: Vec<CityRecord>,
    districts: Vec<DistrictRecord>,
}
/// Owns only lookup projections. Native container membership chooses the player
/// table, independently of each object's actual owner. No traversal is discarded
/// from the source metadata; these tables sort by slot solely for Find queries.
pub struct WorldInstances {
    records: [PlayerRecords; PLAYER_COUNT],
    resolved: [bool; PLAYER_COUNT],
}
impl WorldInstances {
    pub fn from_metadata(
        map: &Map,
        cities: &[City],
        prefixes: &[DistrictPrefix],
        players: &[PlayerMetadata],
        definitions: &NativeDomainDefinitions,
        max_records: usize,
    ) -> Result<Self, Error> {
        if players.len() != PLAYER_COUNT {
            return Err(Error::MetadataPlayer);
        }
        if cities
            .len()
            .checked_add(prefixes.len())
            .is_none_or(|count| count > max_records)
        {
            return Err(Error::ResourceLimit);
        }
        let mut city_sources: Vec<_> = cities
            .iter()
            .map(|city| city_record(&city.header))
            .collect();
        let mut district_sources = Vec::new();
        district_sources
            .try_reserve_exact(prefixes.len())
            .map_err(|_| Error::ResourceLimit)?;
        for prefix in prefixes {
            district_sources.push(prefix.movement_record(map, definitions.district_lookup())?);
        }
        city_sources.sort_unstable_by_key(|record| (record.owner, record.id));
        district_sources.sort_unstable_by_key(|record| (record.owner, record.id));
        if city_sources
            .windows(2)
            .any(|pair| (pair[0].owner, pair[0].id) == (pair[1].owner, pair[1].id))
            || district_sources
                .windows(2)
                .any(|pair| (pair[0].owner, pair[0].id) == (pair[1].owner, pair[1].id))
        {
            return Err(Error::UnmatchedSerializedInstance);
        }
        let mut seen_cities = vec![false; city_sources.len()];
        let mut seen_districts = vec![false; district_sources.len()];
        let mut records: [PlayerRecords; PLAYER_COUNT] =
            std::array::from_fn(|_| PlayerRecords::default());
        let mut resolved = [false; PLAYER_COUNT];
        for (query, player) in players.iter().enumerate() {
            if player.kind != "player"
                || player.query as usize != query
                || player.is_valid != (player.mapped_index != -1)
                || player.resolved != player.is_valid
            {
                return Err(Error::MetadataPlayer);
            }
            if !player.resolved {
                if player.actual_owner.is_some()
                    || player.cities.is_some()
                    || player.districts.is_some()
                {
                    return Err(Error::MetadataPlayer);
                }
                continue;
            }
            if player.actual_owner.is_none() {
                return Err(Error::MetadataPlayer);
            }
            resolved[query] = true;
            let city_members = player.cities.as_ref().ok_or(Error::Membership)?;
            let district_members = player.districts.as_ref().ok_or(Error::Membership)?;
            city_members.validate()?;
            district_members.validate()?;
            records[query]
                .cities
                .try_reserve_exact(city_members.entries.len())
                .map_err(|_| Error::ResourceLimit)?;
            records[query]
                .districts
                .try_reserve_exact(district_members.entries.len())
                .map_err(|_| Error::ResourceLimit)?;
            for member in &city_members.entries {
                let index = city_sources
                    .binary_search_by_key(&(member.actual_owner, member.id), |record| {
                        (record.owner, record.id)
                    })
                    .map_err(|_| Error::UnmatchedSerializedInstance)?;
                if std::mem::replace(&mut seen_cities[index], true) {
                    return Err(Error::Membership);
                }
                records[query].cities.push(city_sources[index]);
            }
            for member in &district_members.entries {
                let index = district_sources
                    .binary_search_by_key(&(member.actual_owner, member.id), |record| {
                        (record.owner, record.id)
                    })
                    .map_err(|_| Error::UnmatchedSerializedInstance)?;
                if std::mem::replace(&mut seen_districts[index], true) {
                    return Err(Error::Membership);
                }
                records[query].districts.push(district_sources[index]);
            }
            records[query]
                .cities
                .sort_unstable_by_key(|record| record.id as u16);
            records[query]
                .districts
                .sort_unstable_by_key(|record| record.id as u16);
            SlotTable::new(&records[query].cities).map_err(Error::Registry)?;
            SlotTable::new(&records[query].districts).map_err(Error::Registry)?;
        }
        if seen_cities.contains(&false) || seen_districts.contains(&false) {
            return Err(Error::UnmatchedSerializedInstance);
        }
        Ok(Self { records, resolved })
    }
    pub fn registry(&self) -> Result<Registry<'_>, Error> {
        let mut players = [PlayerRegistry::Invalid; PLAYER_COUNT];
        for (query, records) in self.records.iter().enumerate() {
            if self.resolved[query] {
                players[query] = PlayerRegistry::Valid {
                    cities: Some(SlotTable::new(&records.cities).map_err(Error::Registry)?),
                    districts: Some(SlotTable::new(&records.districts).map_err(Error::Registry)?),
                };
            }
        }
        Ok(Registry::new(players))
    }
}

/// First 45 bytes of an independently owner-bounded District v15 record.
/// Later bytes are deliberately unconsumed; no scanning for plausible IDs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DistrictPrefix {
    pub id: u32,
    pub location: PlotCoord,
    pub parent_city: u32,
    pub type_hash: u32,
    pub complete: bool,
}
impl DistrictPrefix {
    pub const BYTES: usize = 45;
    pub fn decode_prefix(bytes: &[u8]) -> Result<Self, Error> {
        let bytes = bytes.get(..Self::BYTES).ok_or(Error::Truncated)?;
        let word = |offset| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        if word(0) != 15 {
            return Err(Error::Version(word(0)));
        }
        let complete = match bytes[44] {
            0 => false,
            1 => true,
            byte => return Err(Error::InvalidBool(byte)),
        };
        Ok(Self {
            id: word(4),
            location: PlotCoord {
                x: word(8) as i32,
                y: word(12) as i32,
            },
            parent_city: word(16),
            type_hash: word(20),
            complete,
        })
    }
    /// Original owner comes from the actual District plot's CityAttributes,
    /// not the requested owner or the City/container holding this record.
    pub fn movement_record(
        self,
        map: &Map,
        definitions: &dyn DefinitionLookup,
    ) -> Result<DistrictRecord, Error> {
        let plot = map
            .plot_at(self.location.x, self.location.y)
            .ok_or(Error::Location)?;
        if plot.flags.get(PlotFlag::HasCityAttributes) != plot.city_attributes.is_some() {
            return Err(Error::Attributes);
        }
        let owner = plot
            .city_attributes
            .as_ref()
            .map_or(-1, |a| i32::from(a.player));
        DistrictRecord::from_saved_type(
            self.id,
            owner,
            self.location,
            self.parent_city,
            self.type_hash,
            self.complete,
            definitions,
        )
        .map_err(Error::Registry)
    }
}

pub fn city_record(header: &Header) -> CityRecord {
    CityRecord {
        id: header.id,
        location: PlotCoord {
            x: header.location_x,
            y: header.location_y,
        },
        owner: header.owner,
    }
}

/// Complete typed Map dependencies of native IsCoastalLand(1) and neighbors.
/// Landmass lookup uses the saved container slot, never Area or object ID equality.
pub struct MapDomainInputs<'a, 'd> {
    map: &'a Map,
    terrains: &'a DefinitionTable<'d, TerrainProperties>,
    landmasses: Vec<(u16, &'a Landmass)>,
}
impl<'a, 'd> MapDomainInputs<'a, 'd> {
    pub fn new(
        map: &'a Map,
        terrains: &'a DefinitionTable<'d, TerrainProperties>,
    ) -> Result<Self, Error> {
        // The original Plot stores each coordinate in a signed short. This
        // bounded adapter admits canonical coordinates representable there.
        if map.header.width <= 0
            || map.header.height <= 0
            || map.header.width > i32::from(i16::MAX) + 1
            || map.header.height > i32::from(i16::MAX) + 1
            || (map.header.width as usize).checked_mul(map.header.height as usize)
                != Some(map.plots.len())
        {
            return Err(Error::MapGeometry);
        }
        let table = &map.landmasses;
        if table.capacity > 65536 || table.entries.len() > table.capacity as usize {
            return Err(Error::LandmassTable);
        }
        let mut landmasses = Vec::new();
        landmasses
            .try_reserve_exact(table.entries.len())
            .map_err(|_| Error::ResourceLimit)?;
        for entry in &table.entries {
            if entry.index < 0 || entry.index as u32 >= table.capacity {
                return Err(Error::LandmassTable);
            }
            landmasses.push((entry.index as u16, &entry.value));
        }
        landmasses.sort_unstable_by_key(|(slot, _)| *slot);
        if landmasses.windows(2).any(|pair| pair[0].0 == pair[1].0) {
            return Err(Error::LandmassTable);
        }
        Ok(Self {
            map,
            terrains,
            landmasses,
        })
    }
    fn landmass(&self, id: i32) -> Option<&Landmass> {
        if id == -1 {
            return None;
        }
        let slot = id as u16;
        self.landmasses
            .binary_search_by_key(&slot, |(slot, _)| *slot)
            .ok()
            .map(|i| self.landmasses[i].1)
    }
}
// Original PlotXToX/XToPlotX negative-row division and CoordRange. In particular
// negative multiples of dimension wrap to dimension, not zero.
fn row_half(y: i32) -> i32 {
    if y >= 0 {
        y >> 1
    } else {
        let minus_one = y.wrapping_sub(1);
        minus_one.wrapping_add(((minus_one as u32) >> 31) as i32) >> 1
    }
}
fn coord_range(value: i32, dimension: i32, wrap: bool) -> i32 {
    if !wrap {
        value
    } else if value < 0 {
        (value % dimension).wrapping_add(dimension)
    } else if value >= dimension {
        value % dimension
    } else {
        value
    }
}
impl DomainRebuildInputs for MapDomainInputs<'_, '_> {
    fn is_water(&self, plot: usize) -> Result<bool, city_district::Error> {
        city_district::map_is_water(self.map, plot, self.terrains)
    }
    fn adjacent_plot(
        &self,
        plot: usize,
        direction: u8,
    ) -> Result<Option<usize>, city_district::Error> {
        if plot >= self.map.plots.len() {
            return Err(city_district::Error::PlotOutsideMap { plot });
        }
        let direction = usize::from(direction);
        const X: [i32; 6] = [0, 1, 1, 0, -1, -1];
        const Y: [i32; 6] = [1, 0, -1, -1, 0, 1];
        if direction >= X.len() {
            return Err(city_district::Error::MissingAdjacency {
                plot,
                direction: direction as u8,
            });
        }
        let width = self.map.header.width;
        let from_x = (plot % width as usize) as i32;
        let from_y = (plot / width as usize) as i32;
        let y = from_y.wrapping_add(Y[direction]);
        let x = from_x
            .wrapping_sub(row_half(from_y))
            .wrapping_add(X[direction])
            .wrapping_add(row_half(y));
        let x = coord_range(x, width, self.map.header.wrap_x);
        let y = coord_range(y, self.map.header.height, self.map.header.wrap_y);
        Ok(
            if x >= 0 && y >= 0 && x < width && y < self.map.header.height {
                Some(y as usize * width as usize + x as usize)
            } else {
                None
            },
        )
    }
    fn is_coastal_land_one(&self, plot: usize) -> Result<bool, city_district::Error> {
        if self.is_water(plot)? {
            return Ok(false);
        }
        for direction in 0..6 {
            let Some(adjacent) = self.adjacent_plot(plot, direction)? else {
                continue;
            };
            let neighbor = &self.map.plots[adjacent];
            if self.is_water(adjacent)?
                && !neighbor.flags.get(PlotFlag::Impassable)
                && self
                    .landmass(neighbor.landmass_id)
                    .is_some_and(|landmass| landmass.plot_count >= 1)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// Borrowed complete typed City objects; does not infer native player validity.
/// The separate core Registry controls actual player/container availability.
pub struct CityBuildings<'a> {
    cities: Vec<(CityRecord, &'a Buildings)>,
}
impl<'a> CityBuildings<'a> {
    pub fn new(cities: &'a [City], max_cities: usize) -> Result<Self, Error> {
        if cities.len() > max_cities {
            return Err(Error::ResourceLimit);
        }
        let mut records = Vec::new();
        records
            .try_reserve_exact(cities.len())
            .map_err(|_| Error::ResourceLimit)?;
        records.extend(
            cities
                .iter()
                .map(|city| (city_record(&city.header), &city.buildings)),
        );
        records.sort_unstable_by_key(|(city, _)| (city.owner, city.id as u16));
        if records.iter().any(|(city, _)| city.id == u32::MAX)
            || records.windows(2).any(|pair| {
                (pair[0].0.owner, pair[0].0.id as u16) == (pair[1].0.owner, pair[1].0.id as u16)
            })
        {
            return Err(Error::CityTable);
        }
        Ok(Self { cities: records })
    }
    fn find(&self, resolved: &CityRecord) -> Option<&Buildings> {
        let key = (resolved.owner, resolved.id as u16);
        let index = self
            .cities
            .binary_search_by_key(&key, |(city, _)| (city.owner, city.id as u16))
            .ok()?;
        let (city, buildings) = self.cities[index];
        (city == *resolved).then_some(buildings)
    }
    /// Exact CityAttributes.IsWonderComplete: signed-short type, owning City,
    /// then HasBuilding. Pillage and equality of building location to this plot
    /// are not tested by the original method.
    pub fn is_wonder_complete(
        &self,
        attributes: &CityAttributes,
        registry: &Registry<'_>,
        definitions: &dyn DefinitionLookup,
        plot: usize,
    ) -> Result<bool, city_district::Error> {
        let index = saved_index(attributes.wonder_building_hash, definitions)? as i16 as i32;
        if index == -1 {
            return Ok(false);
        }
        let Some(found) = registry.owning_city(attributes)? else {
            return Ok(false);
        };
        let city = self
            .find(found.record)
            .ok_or(city_district::Error::MissingWonderCompletion { plot })?;
        Ok(building_location(city, index, definitions)? >= 0)
    }
}
fn saved_index(hash: u32, definitions: &dyn DefinitionLookup) -> Result<i32, city_district::Error> {
    if definitions.count() > i32::MAX as usize {
        return Err(city_district::Error::DefinitionMapping);
    }
    match definitions.find_index(hash) {
        Some(index) if index < definitions.count() => Ok(index as i32),
        Some(_) => Err(city_district::Error::DefinitionMapping),
        None => Ok(-1),
    }
}
/// Original Buildings.Read resets all locations to -1 at actual collection
/// count, then applies hash entries in wire order. Unknown hashes are consumed
/// and discarded; repeated aliases overwrite. Querying an out-of-range unsigned
/// index returns -1. This calculates the queried slot without allocating a vector.
pub fn building_location(
    buildings: &Buildings,
    index: i32,
    definitions: &dyn DefinitionLookup,
) -> Result<i16, city_district::Error> {
    if definitions.count() > i32::MAX as usize {
        return Err(city_district::Error::DefinitionMapping);
    }
    if index as u32 as usize >= definitions.count() {
        return Ok(-1);
    }
    let mut location = -1;
    for entry in &buildings.building_locations {
        if saved_index(entry.definition_hash, definitions)? == index {
            location = entry.value;
        }
    }
    Ok(location)
}

/// Complete source-derived rebuild queries once exact registry metadata exists.
pub struct TypedDomainInputs<'a, 'd, 'r> {
    pub map: MapDomainInputs<'a, 'd>,
    pub registry: &'r Registry<'a>,
    pub cities: &'r CityBuildings<'a>,
    pub building_definitions: &'r dyn DefinitionLookup,
}
impl DomainRebuildInputs for TypedDomainInputs<'_, '_, '_> {
    fn is_water(&self, plot: usize) -> Result<bool, city_district::Error> {
        self.map.is_water(plot)
    }
    fn is_coastal_land_one(&self, plot: usize) -> Result<bool, city_district::Error> {
        self.map.is_coastal_land_one(plot)
    }
    fn adjacent_plot(
        &self,
        plot: usize,
        direction: u8,
    ) -> Result<Option<usize>, city_district::Error> {
        self.map.adjacent_plot(plot, direction)
    }
    fn is_wonder_complete(&self, plot: usize) -> Result<bool, city_district::Error> {
        let record = self
            .map
            .map
            .plots
            .get(plot)
            .ok_or(city_district::Error::PlotOutsideMap { plot })?;
        if record.flags.get(PlotFlag::HasCityAttributes) != record.city_attributes.is_some() {
            return Err(city_district::Error::PlotAttributes { plot });
        }
        let Some(attributes) = &record.city_attributes else {
            return Err(city_district::Error::MissingWonderCompletion { plot });
        };
        self.cities
            .is_wonder_complete(attributes, self.registry, self.building_definitions, plot)
    }
}

/// Initialization-time adapter: water is calculated from typed Map and actual
/// Terrain definitions; remaining queries must be exact providers. Supplying no
/// coastal/Wonder/neighbor provider leaves named core errors, never defaults.
/// This preserves the core's two passes and Canal/Bridge ordering unchanged.
pub fn initialization_domains(
    map: &Map,
    registry: &Registry<'_>,
    definitions: &RebuildDefinitions<'_>,
    terrains: &DefinitionTable<'_, TerrainProperties>,
    remaining: &impl DomainRebuildInputs,
    max_plots: usize,
) -> Result<Vec<Option<DomainMask>>, city_district::Error> {
    struct Inputs<'a, 'b, T> {
        map: &'a Map,
        terrains: &'a DefinitionTable<'b, TerrainProperties>,
        remaining: &'a T,
    }
    impl<T: DomainRebuildInputs> DomainRebuildInputs for Inputs<'_, '_, T> {
        fn is_water(&self, plot: usize) -> Result<bool, city_district::Error> {
            city_district::map_is_water(self.map, plot, self.terrains)
        }
        fn is_coastal_land_one(&self, plot: usize) -> Result<bool, city_district::Error> {
            self.remaining.is_coastal_land_one(plot)
        }
        fn is_wonder_complete(&self, plot: usize) -> Result<bool, city_district::Error> {
            self.remaining.is_wonder_complete(plot)
        }
        fn adjacent_plot(
            &self,
            plot: usize,
            direction: u8,
        ) -> Result<Option<usize>, city_district::Error> {
            self.remaining.adjacent_plot(plot, direction)
        }
    }
    city_district::rebuild_valid_domains(
        map,
        registry,
        definitions,
        &Inputs {
            map,
            terrains,
            remaining,
        },
        max_plots,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::LinuxDefinitionHashRegistry;
    use c6_core::city_district::{PlayerRegistry, SlotTable, PLAYER_COUNT};
    use c6_core::map::{Header, Plot, Recycled, Slot, UnitContainer};
    use c6_core::unit::HashValue;
    fn empty<T>() -> Recycled<T> {
        Recycled {
            capacity: 0,
            entries: vec![],
            current_id_bits: 0,
            free_head: 0,
            free_tail: 0,
            free_links: vec![],
        }
    }
    fn map() -> Map {
        Map {
            header: Header {
                width: 3,
                height: 3,
                top_latitude: 90,
                bottom_latitude: -90,
                wrap_x: false,
                wrap_y: false,
                map_size_hash: 0,
                land_plots: 9,
                owned_plots: 0,
                natural_wonders: 0,
                resource_counts: vec![],
                land_resource_counts: vec![],
            },
            visibility: vec![],
            areas: empty(),
            landmasses: Recycled {
                capacity: 8,
                entries: vec![Slot {
                    index: 1,
                    value: Landmass {
                        id: 0x30002,
                        plot_count: 1,
                        water: false,
                        centroid_delta: [0, 0],
                    },
                }],
                ..empty()
            },
            regions: empty(),
            chokepoints: empty(),
            continents: vec![],
            feature_layouts: empty(),
            plots: vec![Plot::decode(&[0; 55], 6, c6_core::map::Limits::default()).unwrap(); 9],
            units: UnitContainer {
                width: 3,
                height: 3,
                layers: vec![],
            },
            components: vec![],
        }
    }
    fn buildings() -> Buildings {
        Buildings {
            building_locations: vec![],
            pillaged: vec![],
            era_created: vec![],
            num_units_supported: vec![],
            yield_modifiers: vec![],
            yield_changes: vec![],
            yield_changes_city_states: vec![],
            wonder_yield_changes: vec![],
            great_work_buildings: vec![],
            extra_great_works: vec![],
            yield_change_from_features: vec![],
            building_name_overrides: vec![],
        }
    }
    fn source_exports() -> Vec<DomainRegistryExport> {
        ["district", "building"]
            .into_iter()
            .map(|name| DomainRegistryExport {
                kind: "registry".into(),
                name: name.into(),
                count: 1,
                complete: true,
                entries: vec![DomainDefinitionExport {
                    query: 0,
                    index: 0,
                    hash: if name == "district" {
                        city_district::CITY_CENTER_HASH
                    } else {
                        100
                    },
                    find_index: Some(0),
                    direct_index: Some(0),
                    internal_only: (name == "district").then_some(false),
                    xp2: Xp2Export::Absent {},
                }],
            })
            .collect()
    }
    fn source_players() -> Vec<PlayerMetadata> {
        (0..PLAYER_COUNT)
            .map(|query| PlayerMetadata {
                kind: "player".into(),
                query: query as u32,
                mapped_index: -1,
                is_valid: false,
                resolved: false,
                actual_owner: None,
                cities: None,
                districts: None,
            })
            .collect()
    }
    fn membership(entries: Vec<MembershipEntry>) -> MembershipExport {
        MembershipExport {
            count: entries.len() as u32,
            entries,
            complete: true,
        }
    }
    #[test]
    fn source_metadata_world_uses_container_query_and_all_serialized_records() {
        let definitions = NativeDomainDefinitions::from_exports(&source_exports()).unwrap();
        let lookup = LinuxDefinitionHashRegistry::from_native_order(vec![100, 200]);
        let properties = [
            TerrainProperties {
                is_water: false,
                movement_cost: 1,
            },
            TerrainProperties {
                is_water: true,
                movement_cost: 1,
            },
        ];
        let terrains = DefinitionTable::new(&lookup, &properties).unwrap();
        let mut map = map();
        for (plot, owner, district) in [(4, 1, 0xdead0005), (5, 2, u32::MAX)] {
            map.plots[plot].flags.set(PlotFlag::HasCityAttributes, true);
            map.plots[plot].city_attributes = Some(CityAttributes {
                working_city: u32::MAX,
                owning_city: u32::MAX,
                district,
                player: owner,
                wonder_building_hash: u32::MAX,
            });
        }
        map.plots[5].terrain_hash = 200;
        map.plots[5].landmass_id = 1;
        let prefixes = [DistrictPrefix {
            id: 0x10005,
            location: PlotCoord { x: 2, y: 1 },
            parent_city: u32::MAX,
            type_hash: city_district::CITY_CENTER_HASH,
            complete: true,
        }];
        let mut players = source_players();
        players[1] = PlayerMetadata {
            kind: "player".into(),
            query: 1,
            mapped_index: 0,
            is_valid: true,
            resolved: true,
            actual_owner: Some(41),
            cities: Some(membership(vec![])),
            districts: Some(membership(vec![MembershipEntry {
                ordinal: 0,
                id: 0x10005,
                slot: 5,
                actual_owner: 2,
            }])),
        };
        let instances =
            WorldInstances::from_metadata(&map, &[], &prefixes, &players, &definitions, 1).unwrap();
        let registry = instances.registry().unwrap();
        let found = registry
            .find_district(city_district::ComponentId::district(1, 0xabcd0005))
            .unwrap()
            .unwrap();
        assert_eq!(found.record.owner, 2); // Actual owner is not the container query1.
        assert_eq!(found.record.id, 0x10005); // Upper requested counter is ignored.
        assert!(registry
            .find_district(city_district::ComponentId::district(2, 0x10005))
            .unwrap()
            .is_none());
        let cities = CityBuildings::new(&[], 0).unwrap();
        let inputs = TypedDomainInputs {
            map: MapDomainInputs::new(&map, &terrains).unwrap(),
            registry: &registry,
            cities: &cities,
            building_definitions: definitions.building_lookup(),
        };
        assert_eq!(
            inputs.is_wonder_complete(0),
            Err(city_district::Error::MissingWonderCompletion { plot: 0 })
        );
        let masks = city_district::rebuild_valid_domains(
            &map,
            &registry,
            &definitions.as_core().unwrap(),
            &inputs,
            9,
        )
        .unwrap();
        assert_eq!(masks[4], Some(DomainMask(23)));
        assert_eq!(masks[5], Some(DomainMask(19)));
        assert_eq!(masks.iter().filter(|mask| mask.is_some()).count(), 2);
        players[1].districts = Some(membership(vec![]));
        assert!(matches!(
            WorldInstances::from_metadata(&map, &[], &prefixes, &players, &definitions, 1),
            Err(Error::UnmatchedSerializedInstance)
        ));
        assert!(matches!(
            WorldInstances::from_metadata(&map, &[], &prefixes, &players[..64], &definitions, 1),
            Err(Error::MetadataPlayer)
        ));
        assert!(matches!(
            WorldInstances::from_metadata(&map, &[], &prefixes, &players, &definitions, 0),
            Err(Error::ResourceLimit)
        ));
    }
    #[test]
    fn source_metadata_rejects_unknown_properties_and_lookup_disagreement() {
        let mut exports = source_exports();
        exports[0].entries[0].xp2 = Xp2Export::Present {
            is_canal: None,
            canal_wonder: None,
            bridge: None,
        };
        assert!(matches!(
            NativeDomainDefinitions::from_exports(&exports),
            Err(Error::MetadataRegistry)
        ));
        exports = source_exports();
        exports[1].entries[0].find_index = None;
        assert!(matches!(
            NativeDomainDefinitions::from_exports(&exports),
            Err(Error::MetadataRegistry)
        ));
        exports = source_exports();
        exports[0].complete = false;
        assert!(matches!(
            NativeDomainDefinitions::from_exports(&exports),
            Err(Error::MetadataRegistry)
        ));
        assert!(serde_json::from_value::<Xp2Export>(
            serde_json::json!({"status":"absent","is_canal":false})
        )
        .is_err());
        assert!(serde_json::from_value::<DomainRegistryExport>(serde_json::json!({"kind":"registry","name":"district","count":0,"complete":true,"entries":[],"valid_domains_mask":23})).is_err());
        let mut terrain = TerrainRegistryExport {
            kind: "registry".into(),
            name: "terrain".into(),
            count: 1,
            complete: true,
            entries: vec![TerrainDefinitionExport {
                query: 0,
                index: 0,
                hash: 100,
                find_index: Some(0),
                direct_index: Some(0),
                is_water: true,
                movement_cost: 7,
            }],
        };
        let definitions = NativeTerrainDefinitions::from_export(&terrain).unwrap();
        assert_eq!(
            definitions.as_core().unwrap().at(0).unwrap().movement_cost,
            7
        );
        terrain.entries[0].direct_index = Some(1);
        assert!(matches!(
            NativeTerrainDefinitions::from_export(&terrain),
            Err(Error::MetadataRegistry)
        ));
    }
    #[test]
    fn source_wonder_uses_owning_city_not_building_plot_or_pillage() {
        let lookup = LinuxDefinitionHashRegistry::from_native_order(vec![100]);
        let records = [CityRecord {
            id: 9,
            owner: 0,
            location: PlotCoord { x: 1, y: 1 },
        }];
        let mut state = buildings();
        state.building_locations.push(HashValue {
            definition_hash: 100,
            value: 123,
        });
        state.pillaged.push(HashValue {
            definition_hash: 100,
            value: true,
        });
        let cities = CityBuildings {
            cities: vec![(records[0], &state)],
        };
        let mut players = [PlayerRegistry::Invalid; PLAYER_COUNT];
        players[0] = PlayerRegistry::Valid {
            cities: Some(SlotTable::new(&records).unwrap()),
            districts: None,
        };
        let registry = Registry::new(players);
        let attributes = CityAttributes {
            working_city: u32::MAX,
            owning_city: 0xabcd0009,
            district: u32::MAX,
            player: 0,
            wonder_building_hash: 100,
        };
        assert!(cities
            .is_wonder_complete(&attributes, &registry, &lookup, 4)
            .unwrap());
    }
    #[test]
    fn source_coastal_requires_passable_water_nonnull_landmass_and_positive_count() {
        let lookup = LinuxDefinitionHashRegistry::from_native_order(vec![100, 200]);
        let properties = [
            TerrainProperties {
                is_water: false,
                movement_cost: 1,
            },
            TerrainProperties {
                is_water: true,
                movement_cost: 1,
            },
        ];
        let terrains = DefinitionTable::new(&lookup, &properties).unwrap();
        let mut map = map();
        map.plots[5].terrain_hash = 200;
        map.plots[5].landmass_id = 0x12340001; // Low16 container slot; object's ID differs.
        assert!(MapDomainInputs::new(&map, &terrains)
            .unwrap()
            .is_coastal_land_one(4)
            .unwrap());
        assert!(!MapDomainInputs::new(&map, &terrains)
            .unwrap()
            .is_coastal_land_one(5)
            .unwrap());
        map.plots[5].flags.set(PlotFlag::Impassable, true);
        assert!(!MapDomainInputs::new(&map, &terrains)
            .unwrap()
            .is_coastal_land_one(4)
            .unwrap());
        map.plots[5].flags.set(PlotFlag::Impassable, false);
        for count in [0, -1] {
            map.landmasses.entries[0].value.plot_count = count;
            assert!(!MapDomainInputs::new(&map, &terrains)
                .unwrap()
                .is_coastal_land_one(4)
                .unwrap());
        }
        map.landmasses.entries[0].value.plot_count = 1;
        for id in [-1, 2] {
            map.plots[5].landmass_id = id;
            assert!(!MapDomainInputs::new(&map, &terrains)
                .unwrap()
                .is_coastal_land_one(4)
                .unwrap());
        }
    }
    #[test]
    fn source_direction_order_and_negative_multiple_wrap_are_original() {
        let lookup = LinuxDefinitionHashRegistry::from_native_order(vec![100]);
        let properties = [TerrainProperties {
            is_water: false,
            movement_cost: 1,
        }];
        let terrains = DefinitionTable::new(&lookup, &properties).unwrap();
        let mut map = map();
        let inputs = MapDomainInputs::new(&map, &terrains).unwrap();
        for (direction, expected) in [8, 5, 2, 1, 3, 7].into_iter().enumerate() {
            assert_eq!(
                inputs.adjacent_plot(4, direction as u8).unwrap(),
                Some(expected)
            );
        }
        assert_eq!(inputs.adjacent_plot(0, 4).unwrap(), None);
        map.header.width = 1;
        map.header.wrap_x = true;
        map.plots.truncate(3);
        let inputs = MapDomainInputs::new(&map, &terrains).unwrap();
        assert_eq!(inputs.adjacent_plot(1, 4).unwrap(), None); // CoordRange(-1,1,true)==1.
        assert_eq!(inputs.adjacent_plot(1, 1).unwrap(), Some(1));
    }
    #[test]
    fn source_building_locations_remap_filter_and_overwrite_without_pillage_test() {
        let lookup = LinuxDefinitionHashRegistry::from_native_order(vec![100, 200]);
        let mut buildings = buildings();
        buildings.building_locations = vec![
            HashValue {
                definition_hash: 100,
                value: 12,
            },
            HashValue {
                definition_hash: u32::MAX,
                value: 99,
            },
            HashValue {
                definition_hash: 0,
                value: -2,
            }, // Numeric query aliases index0.
            HashValue {
                definition_hash: 200,
                value: 0,
            },
        ];
        buildings.pillaged.push(HashValue {
            definition_hash: 200,
            value: true,
        });
        assert_eq!(building_location(&buildings, 0, &lookup).unwrap(), -2);
        assert_eq!(building_location(&buildings, 1, &lookup).unwrap(), 0);
        for index in [-32768, -1, 2] {
            assert_eq!(building_location(&buildings, index, &lookup).unwrap(), -1);
        }
        buildings.building_locations.clear();
        assert_eq!(building_location(&buildings, 0, &lookup).unwrap(), -1);
    }
    #[test]
    fn source_wonder_guards_and_missing_typed_city_are_explicit() {
        let lookup = LinuxDefinitionHashRegistry::from_native_order(vec![100]);
        let cities = CityBuildings::new(&[], 0).unwrap();
        let registry = Registry::new([PlayerRegistry::Unavailable; PLAYER_COUNT]);
        let mut attributes = CityAttributes {
            working_city: 9,
            owning_city: 9,
            district: 3,
            player: 0,
            wonder_building_hash: u32::MAX,
        };
        assert!(!cities
            .is_wonder_complete(&attributes, &registry, &lookup, 4)
            .unwrap());
        attributes.wonder_building_hash = 100;
        assert_eq!(
            cities.is_wonder_complete(&attributes, &registry, &lookup, 4),
            Err(city_district::Error::UnavailablePlayer { player: 0 })
        );
        let records = [CityRecord {
            id: 9,
            owner: 0,
            location: PlotCoord { x: 1, y: 1 },
        }];
        let mut players = [PlayerRegistry::Unavailable; PLAYER_COUNT];
        players[0] = PlayerRegistry::Valid {
            cities: Some(SlotTable::new(&records).unwrap()),
            districts: None,
        };
        let registry = Registry::new(players);
        assert_eq!(
            cities.is_wonder_complete(&attributes, &registry, &lookup, 4),
            Err(city_district::Error::MissingWonderCompletion { plot: 4 })
        );
    }
    #[test]
    fn prefix_has_strict_version_bool_and_boundary() {
        let mut bytes = [0; 46];
        bytes[..4].copy_from_slice(&15u32.to_le_bytes());
        bytes[4..8].copy_from_slice(&0x12340001u32.to_le_bytes());
        bytes[44] = 1;
        bytes[45] = 255; // Not consumed as part of this projection.
        let prefix = DistrictPrefix::decode_prefix(&bytes).unwrap();
        assert_eq!(prefix.id, 0x12340001);
        assert!(prefix.complete);
        assert_eq!(
            DistrictPrefix::decode_prefix(&bytes[..44]),
            Err(Error::Truncated)
        );
        bytes[44] = 2;
        assert_eq!(
            DistrictPrefix::decode_prefix(&bytes),
            Err(Error::InvalidBool(2))
        );
        bytes[0] = 14;
        assert_eq!(
            DistrictPrefix::decode_prefix(&bytes),
            Err(Error::Version(14))
        );
    }
}
