//! Serialized Linux movement identity projection. Not full District state or
//! domain reconstruction: native definition order and player validity are inputs.
use c6_core::city::City;
use c6_core::city_district::{self, CityRecord, DistrictRecord};
use c6_core::city_district::{DomainMask, DomainRebuildInputs, RebuildDefinitions, Registry};
use c6_core::city_production::Buildings;
use c6_core::city_wire::Header;
use c6_core::map::{CityAttributes, Landmass, Map, PlotFlag};
use c6_core::movement_world::{DefinitionLookup, DefinitionTable, TerrainProperties};
use c6_core::unit::PlotCoord;

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
    Registry(city_district::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux serialized movement projection: {self:?}")
    }
}
impl std::error::Error for Error {}

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
    cities: Vec<&'a City>,
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
        records.extend(cities);
        records.sort_unstable_by_key(|city| (city.header.owner, city.header.id as u16));
        if records.iter().any(|city| city.header.id == u32::MAX)
            || records.windows(2).any(|pair| {
                (pair[0].header.owner, pair[0].header.id as u16)
                    == (pair[1].header.owner, pair[1].header.id as u16)
            })
        {
            return Err(Error::CityTable);
        }
        Ok(Self { cities: records })
    }
    fn find(&self, resolved: &CityRecord) -> Option<&City> {
        let key = (resolved.owner, resolved.id as u16);
        let index = self
            .cities
            .binary_search_by_key(&key, |city| (city.header.owner, city.header.id as u16))
            .ok()?;
        let city = self.cities[index];
        (city_record(&city.header) == *resolved).then_some(city)
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
        Ok(building_location(&city.buildings, index, definitions)? >= 0)
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
            return Ok(false);
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
