//! Serialized Linux movement identity projection. Not full District state or
//! domain reconstruction: native definition order and player validity are inputs.
use c6_core::city_district::{self, CityRecord, DistrictRecord};
use c6_core::city_district::{DomainMask, DomainRebuildInputs, RebuildDefinitions, Registry};
use c6_core::city_wire::Header;
use c6_core::map::{Map, PlotFlag};
use c6_core::movement_world::{DefinitionLookup, DefinitionTable, TerrainProperties};
use c6_core::unit::PlotCoord;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Version(u32),
    InvalidBool(u8),
    Location,
    Attributes,
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
