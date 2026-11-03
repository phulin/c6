use super::{Error, Limits, Reader, Writer};
use crate::unit::variants::TypedVariantMap;

/// Actual packed writer bit positions for current Plot vector version 6.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PlotFlag {
    ImprovementPillaged = 0,
    RoutePillaged,
    RouteBridgey,
    StartingPlot,
    Hills,
    NorthEastRiver,
    WestRiver,
    NorthWestRiver,
    NorthEastCliff,
    WestCliff,
    NorthWestCliff,
    RoughFeature,
    AdjacentLand,
    Impassable,
    HasCityAttributes,
    FeatureAdded,
    FeatureVolatile,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlotFlags(u32);
impl PlotFlags {
    pub fn from_bits(bits: u32) -> Result<Self, Error> {
        if bits & !0x1ffff != 0 {
            Err(Error::UnsupportedPlotFlags(bits))
        } else {
            Ok(Self(bits))
        }
    }
    pub fn bits(self) -> u32 {
        self.0
    }
    pub fn get(self, flag: PlotFlag) -> bool {
        self.0 & (1 << (flag as u8)) != 0
    }
    pub fn set(&mut self, flag: PlotFlag, value: bool) {
        let bit = 1 << (flag as u8);
        if value {
            self.0 |= bit;
        } else {
            self.0 &= !bit;
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CityAttributes {
    pub working_city: u32,
    pub owning_city: u32,
    pub district: u32,
    pub player: i8,
    pub wonder_building_hash: u32,
}
/// Complete current Plot writer fields. Coordinates are derived by the Map
/// reader; live validDomains/yieldEffect/found-location fields are not serialized.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Plot {
    pub area_id: i32,
    pub landmass_id: i32,
    pub region_id: u32,
    pub terrain_hash: u32,
    pub feature_hash: u32,
    pub feature_layout: u16,
    pub continent_hash: u32,
    pub unit_count: i8,
    pub resource_hash: u32,
    pub resource_count: u16,
    pub improvement_hash: u32,
    pub improvement_owner: i8,
    pub route_type: i8,
    pub route_era: i8,
    pub appeal: i16,
    pub river_east_flow: i8,
    pub river_south_east_flow: i8,
    pub river_south_west_flow: i8,
    pub river_crossing_count: i8,
    pub river_crossing_mask: u8,
    pub cliff_crossing_mask: u8,
    pub flags: PlotFlags,
    /// None represents the wire's zero count, not native pointer absence.
    pub properties: Option<TypedVariantMap>,
    pub city_attributes: Option<CityAttributes>,
}
impl Plot {
    /// Plot records do not contain their own version word. The enclosing Plot
    /// vector supplies version 6; older forms explicitly reject here.
    pub fn decode(bytes: &[u8], plot_version: i32, limits: Limits) -> Result<Self, Error> {
        if plot_version != 6 {
            return Err(Error::UnsupportedVersion {
                expected: 6,
                actual: plot_version,
            });
        }
        let mut reader = Reader::new(bytes, limits)?;
        let value = Self::read(&mut reader)?;
        reader.finish()?;
        Ok(value)
    }
    pub fn encode(&self, plot_version: i32, limits: Limits) -> Result<Vec<u8>, Error> {
        if plot_version != 6 {
            return Err(Error::UnsupportedVersion {
                expected: 6,
                actual: plot_version,
            });
        }
        let mut writer = Writer::new(limits);
        self.write(&mut writer)?;
        Ok(writer.bytes)
    }
    pub(super) fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let area_id = r.i32()?;
        let landmass_id = r.i32()?;
        let region_id = r.u32()?;
        let terrain_hash = r.u32()?;
        let feature_hash = r.u32()?;
        let feature_layout = r.u16()?;
        let continent_hash = r.u32()?;
        let unit_count = r.i8()?;
        let resource_hash = r.u32()?;
        let resource_count = r.u16()?;
        let improvement_hash = r.u32()?;
        let improvement_owner = r.i8()?;
        let route_type = r.i8()?;
        let route_era = r.i8()?;
        let appeal = r.i16()?;
        let river_east_flow = r.i8()?;
        let river_south_east_flow = r.i8()?;
        let river_south_west_flow = r.i8()?;
        let river_crossing_count = r.i8()?;
        let river_crossing_mask = r.u8()?;
        let cliff_crossing_mask = r.u8()?;
        let bytes = r.take(3)?;
        let flags = PlotFlags::from_bits(
            u32::from(bytes[0]) | (u32::from(bytes[1]) << 8) | (u32::from(bytes[2]) << 16),
        )?;
        let count = r.u32()? as usize;
        let properties = r.properties(count)?;
        let city_attributes = if flags.get(PlotFlag::HasCityAttributes) {
            Some(CityAttributes {
                working_city: r.u32()?,
                owning_city: r.u32()?,
                district: r.u32()?,
                player: r.i8()?,
                wonder_building_hash: r.u32()?,
            })
        } else {
            None
        };
        Ok(Self {
            area_id,
            landmass_id,
            region_id,
            terrain_hash,
            feature_hash,
            feature_layout,
            continent_hash,
            unit_count,
            resource_hash,
            resource_count,
            improvement_hash,
            improvement_owner,
            route_type,
            route_era,
            appeal,
            river_east_flow,
            river_south_east_flow,
            river_south_west_flow,
            river_crossing_count,
            river_crossing_mask,
            cliff_crossing_mask,
            flags,
            properties,
            city_attributes,
        })
    }
    pub(super) fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.flags.get(PlotFlag::HasCityAttributes) != self.city_attributes.is_some() {
            return Err(Error::CityAttributesFlagMismatch);
        }
        w.i32(self.area_id)?;
        w.i32(self.landmass_id)?;
        w.u32(self.region_id)?;
        w.u32(self.terrain_hash)?;
        w.u32(self.feature_hash)?;
        w.u16(self.feature_layout)?;
        w.u32(self.continent_hash)?;
        w.i8(self.unit_count)?;
        w.u32(self.resource_hash)?;
        w.u16(self.resource_count)?;
        w.u32(self.improvement_hash)?;
        w.i8(self.improvement_owner)?;
        w.i8(self.route_type)?;
        w.i8(self.route_era)?;
        w.i16(self.appeal)?;
        for value in [
            self.river_east_flow,
            self.river_south_east_flow,
            self.river_south_west_flow,
            self.river_crossing_count,
        ] {
            w.i8(value)?;
        }
        w.u8(self.river_crossing_mask)?;
        w.u8(self.cliff_crossing_mask)?;
        let bits = self.flags.bits().to_le_bytes();
        w.put(&bits[..3])?;
        w.properties(&self.properties)?;
        if let Some(city) = &self.city_attributes {
            w.u32(city.working_city)?;
            w.u32(city.owning_city)?;
            w.u32(city.district)?;
            w.i8(city.player)?;
            w.u32(city.wonder_building_hash)?;
        }
        Ok(())
    }
}
