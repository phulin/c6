//! Bounded current Linux XP2 Map/Plot wire state, not movement or a save loader.
//! Complete live map reconstruction, definition-index resolution and gameplay
//! remain outside the codec. Exact target contracts are in the Map evidence.
mod components;
mod localization;
pub use localization::*;
mod containers;
pub use components::*;
mod plot;
pub mod appeal;

use crate::unit::variants::{self, DecodeBudget, TypedVariantMap};
pub use containers::*;
pub use plot::{CityAttributes, Plot, PlotFlag, PlotFlags};

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_items: usize,
    /// Logical owned-vector storage plus conservative typed-variant allocation.
    /// Allocator bookkeeping/rounding is not included in this caller policy.
    pub max_allocation: usize,
    pub max_depth: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: 64 * 1024 * 1024,
            max_items: 4_000_000,
            max_allocation: 128 * 1024 * 1024,
            max_depth: 64,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    TrailingBytes,
    ResourceLimit,
    UnsupportedVersion { expected: i32, actual: i32 },
    NoncanonicalBool(u8),
    UnsupportedPlotFlags(u32),
    CityAttributesFlagMismatch,
    EmptyPropertiesBody,
    PropertyCountMismatch,
    InvalidContainerMetadata,
    PlayerArrayWidth,
    LayerCountMismatch,
    InvalidSparseIndex,
    UnsupportedComponent,
    UnsupportedComponentBranch,
    InvalidDimensions,
    PlotCountDimensions,
    UnitDimensions,
    InvalidPlotReference,
    UnsupportedLocalizationFormat,
    UnsupportedLocalizationArgument(u8),
    UnsupportedLocalizationNameCount,
    Variant(variants::VariantError),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
impl From<variants::VariantError> for Error {
    fn from(value: variants::VariantError) -> Self {
        Self::Variant(value)
    }
}

const VARIANT_ALLOCATION_PER_ITEM: usize = 256;
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    items: usize,
    allocation: usize,
    limits: Limits,
    collect_ranges: bool,
    ranges: Vec<FieldRange>,
    base: usize,
}
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        if bytes.len() > limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        Ok(Self {
            bytes,
            pos: 0,
            items: 0,
            allocation: 0,
            limits,
            collect_ranges: false,
            ranges: Vec::new(),
            base: 0,
        })
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::ResourceLimit)?;
        let value = self.bytes.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(value)
    }
    fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }
    fn i8(&mut self) -> Result<i8, Error> {
        Ok(self.u8()? as i8)
    }
    fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn i16(&mut self) -> Result<i16, Error> {
        Ok(self.u16()? as i16)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, Error> {
        Ok(self.u32()? as i32)
    }
    fn version(&mut self, expected: i32) -> Result<(), Error> {
        let actual = self.i32()?;
        if actual == expected {
            Ok(())
        } else {
            Err(Error::UnsupportedVersion { expected, actual })
        }
    }
    fn boolean(&mut self) -> Result<bool, Error> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            b => Err(Error::NoncanonicalBool(b)),
        }
    }
    fn charge(&mut self, count: usize, allocation: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(count).ok_or(Error::ResourceLimit)?;
        self.allocation = self
            .allocation
            .checked_add(allocation)
            .ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items || self.allocation > self.limits.max_allocation {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
    fn vector<T>(&mut self, count: usize, min_wire: usize) -> Result<Vec<T>, Error> {
        if min_wire != 0 && count > self.bytes.len().saturating_sub(self.pos) / min_wire {
            return Err(Error::Truncated);
        }
        self.charge(
            count,
            count
                .checked_mul(std::mem::size_of::<T>())
                .ok_or(Error::ResourceLimit)?,
        )?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        Ok(result)
    }
    fn list<T>(
        &mut self,
        min_wire: usize,
        mut read: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error> {
        let count = self.u32()? as usize;
        let mut result = self.vector::<T>(count, min_wire)?;
        for _ in 0..count {
            result.push(read(self)?);
        }
        Ok(result)
    }
    fn record<T>(
        &mut self,
        name: &'static str,
        mut read: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let start = self.pos;
        let value = read(self)?;
        if self.collect_ranges {
            self.charge(1, std::mem::size_of::<FieldRange>())?;
            self.ranges
                .try_reserve(1)
                .map_err(|_| Error::ResourceLimit)?;
            self.ranges.push(FieldRange {
                name,
                start: self.base + start,
                end: self.base + self.pos,
            });
        }
        Ok(value)
    }
    fn properties(&mut self, count: usize) -> Result<Option<TypedVariantMap>, Error> {
        if count == 0 {
            return Ok(None);
        }
        let max_items = (self.limits.max_items - self.items)
            .min((self.limits.max_allocation - self.allocation) / VARIANT_ALLOCATION_PER_ITEM);
        let limits = variants::Limits {
            max_bytes: self.bytes.len() - self.pos,
            max_items,
            max_depth: self.limits.max_depth,
        };
        let mut budget = DecodeBudget::new(limits);
        let (map, bytes) = TypedVariantMap::decode_prefix(&self.bytes[self.pos..], &mut budget)?;
        if map.entries.len() != count {
            return Err(Error::PropertyCountMismatch);
        }
        self.charge(
            budget.items_used(),
            budget
                .items_used()
                .checked_mul(VARIANT_ALLOCATION_PER_ITEM)
                .ok_or(Error::ResourceLimit)?,
        )?;
        self.take(bytes)?;
        Ok(Some(map))
    }
    fn finish(self) -> Result<(), Error> {
        if self.pos == self.bytes.len() {
            Ok(())
        } else {
            Err(Error::TrailingBytes)
        }
    }
}
struct Writer {
    bytes: Vec<u8>,
    limits: Limits,
    items: usize,
    allocation: usize,
}
impl Writer {
    fn new(limits: Limits) -> Self {
        Self {
            bytes: vec![],
            limits,
            items: 0,
            allocation: 0,
        }
    }
    fn put(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let end = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        self.bytes
            .try_reserve(bytes.len())
            .map_err(|_| Error::ResourceLimit)?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn u8(&mut self, v: u8) -> Result<(), Error> {
        self.put(&[v])
    }
    fn i8(&mut self, v: i8) -> Result<(), Error> {
        self.u8(v as u8)
    }
    fn u16(&mut self, v: u16) -> Result<(), Error> {
        self.put(&v.to_le_bytes())
    }
    fn i16(&mut self, v: i16) -> Result<(), Error> {
        self.u16(v as u16)
    }
    fn u32(&mut self, v: u32) -> Result<(), Error> {
        self.put(&v.to_le_bytes())
    }
    fn u64(&mut self, v: u64) -> Result<(), Error> {
        self.put(&v.to_le_bytes())
    }
    fn i32(&mut self, v: i32) -> Result<(), Error> {
        self.u32(v as u32)
    }
    fn boolean(&mut self, v: bool) -> Result<(), Error> {
        self.u8(u8::from(v))
    }
    fn charge<T>(&mut self, count: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(count).ok_or(Error::ResourceLimit)?;
        self.allocation = self
            .allocation
            .checked_add(
                count
                    .checked_mul(std::mem::size_of::<T>())
                    .ok_or(Error::ResourceLimit)?,
            )
            .ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items || self.allocation > self.limits.max_allocation {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
    fn list<T>(
        &mut self,
        values: &[T],
        mut write: impl FnMut(&mut Self, &T) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.charge::<T>(values.len())?;
        self.u32(u32::try_from(values.len()).map_err(|_| Error::ResourceLimit)?)?;
        for value in values {
            write(self, value)?;
        }
        Ok(())
    }
    fn properties(&mut self, map: &Option<TypedVariantMap>) -> Result<(), Error> {
        let Some(map) = map else {
            return self.u32(0);
        };
        if map.entries.is_empty() {
            return Err(Error::EmptyPropertiesBody);
        }
        let count = u32::try_from(map.entries.len()).map_err(|_| Error::ResourceLimit)?;
        self.u32(count)?;
        let max_items = (self.limits.max_items - self.items)
            .min((self.limits.max_allocation - self.allocation) / VARIANT_ALLOCATION_PER_ITEM);
        let (bytes, items) = map.encode(variants::Limits {
            max_bytes: self.limits.max_bytes - self.bytes.len(),
            max_items,
            max_depth: self.limits.max_depth,
        })?;
        self.items += items;
        self.allocation += items * VARIANT_ALLOCATION_PER_ITEM;
        self.put(&bytes)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FieldRange {
    pub name: &'static str,
    pub start: usize,
    pub end: usize,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Header {
    pub width: i32,
    pub height: i32,
    pub top_latitude: i32,
    pub bottom_latitude: i32,
    pub wrap_x: bool,
    pub wrap_y: bool,
    pub map_size_hash: u32,
    pub land_plots: i32,
    pub owned_plots: i32,
    pub natural_wonders: i32,
    pub resource_counts: Vec<DefinitionValue>,
    pub land_resource_counts: Vec<DefinitionValue>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Map {
    pub header: Header,
    pub visibility: Vec<PlayerVisibility>,
    pub areas: Recycled<Area>,
    pub landmasses: Recycled<Landmass>,
    pub regions: Recycled<Region>,
    pub chokepoints: Recycled<Chokepoint>,
    pub continents: Vec<Continent>,
    pub feature_layouts: Recycled<FeatureLayout>,
    /// Actual native x-inner/y-outer order. Plot index is y*width+x.
    pub plots: Vec<Plot>,
    pub units: UnitContainer,
    pub components: Vec<Component>,
}
type DecodedMap = (Map, Vec<FieldRange>, usize, usize, usize, i32);

impl Map {
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        Self::decode_inner(bytes, limits, false, false).map(|v| v.0)
    }
    pub fn decode_with_ranges(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, Vec<FieldRange>), Error> {
        Self::decode_inner(bytes, limits, true, false)
            .map(|(map, ranges, _, _, _, _)| (map, ranges))
    }
    /// Decode the complete supported Map at a known structural cursor. The
    /// untouched caller tail is neither parsed nor charged against max_bytes.
    /// Returns the dynamic item charge for a shared enclosing Core budget.
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        Self::decode_prefix_with_usage(bytes, limits)
            .map(|(map, consumed, items, _)| (map, consumed, items))
    }
    /// Returns consumed bytes, item charge and logical allocation charge for
    /// aggregate readers which continue after Map. Charges use this parser's
    /// existing conservative allocation policy, not native heap measurements.
    pub fn decode_prefix_with_usage(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize, usize), Error> {
        Self::decode_prefix_with_version_and_usage(bytes, limits)
            .map(|(map, consumed, items, allocation, _)| (map, consumed, items, allocation))
    }
    /// Also retains the enclosing Plot record version. Encoding remains the
    /// current native v6 format; this is a reader provenance value.
    pub fn decode_prefix_with_version_and_usage(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize, usize, i32), Error> {
        let clipped = bytes.len() > limits.max_bytes;
        let input = &bytes[..bytes.len().min(limits.max_bytes)];
        Self::decode_inner(input, limits, false, true)
            .map(|(map, _, consumed, items, allocation, version)| {
                (map, consumed, items, allocation, version)
            })
            .map_err(|error| {
                if clipped && error == Error::Truncated {
                    Error::ResourceLimit
                } else {
                    error
                }
            })
    }
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        Self::decode_prefix_with_items(bytes, limits).map(|(map, consumed, _)| (map, consumed))
    }
    fn decode_inner(
        bytes: &[u8],
        limits: Limits,
        ranges: bool,
        allow_trailing: bool,
    ) -> Result<DecodedMap, Error> {
        let mut r = Reader::new(bytes, limits)?;
        r.collect_ranges = ranges;
        let header = r.record("header", |r| {
            r.version(4)?;
            let width = r.i32()?;
            let height = r.i32()?;
            let top_latitude = r.i32()?;
            let bottom_latitude = r.i32()?;
            let wrap_x = r.boolean()?;
            let wrap_y = r.boolean()?;
            let map_size_hash = r.u32()?;
            let land_plots = r.i32()?;
            let owned_plots = r.i32()?;
            let natural_wonders = r.i32()?;
            let resource_counts = containers::read_hashes(r)?;
            let land_resource_counts = containers::read_hashes(r)?;
            if width < 0 || height < 0 {
                return Err(Error::InvalidDimensions);
            }
            Ok(Header {
                width,
                height,
                top_latitude,
                bottom_latitude,
                wrap_x,
                wrap_y,
                map_size_hash,
                land_plots,
                owned_plots,
                natural_wonders,
                resource_counts,
                land_resource_counts,
            })
        })?;
        let visibility = r.record("visibility", |r| {
            r.version(1)?;
            r.list(8, |r| {
                let player = r.i32()?;
                let visibility = r.record("visibility_instance", Visibility::read)?;
                Ok(PlayerVisibility { player, visibility })
            })
        })?;
        let areas = r.record("areas", |r| {
            r.version(1)?;
            Recycled::read(r, |r| r.record("area", Area::read))
        })?;
        let landmasses = r.record("landmasses", |r| {
            r.version(1)?;
            Recycled::read(r, |r| r.record("landmass", Landmass::read))
        })?;
        let (regions, chokepoints) = r.record("regions", |r| {
            r.version(1)?;
            Ok((
                Recycled::read(r, |r| r.record("region", Region::read))?,
                Recycled::read(r, |r| r.record("chokepoint", Chokepoint::read))?,
            ))
        })?;
        let continents = r.record("continents", |r| {
            r.list(20, |r| r.record("continent", Continent::read))
        })?;
        let feature_layouts = r.record("features", |r| {
            r.version(1)?;
            Recycled::read(r, FeatureLayout::read)
        })?;
        let plot_count = (header.width as usize)
            .checked_mul(header.height as usize)
            .ok_or(Error::ResourceLimit)?;
        let (plots, plot_version) = r.record("plots", |r| {
            let version = r.i32()?;
            plot::check_version(version)?;
            let count = r.u32()? as usize;
            if count != plot_count {
                return Err(Error::PlotCountDimensions);
            }
            let mut plots = r.vector::<Plot>(count, 55)?;
            for _ in 0..count {
                plots.push(r.record("plot", |r| Plot::read_versioned(r, version))?);
            }
            Ok((plots, version))
        })?;
        let units = r.record("units", UnitContainer::read)?;
        let components = r.record("components", |r| {
            r.version(1)?;
            let count = r.u32()? as usize;
            let mut values = r.vector::<Component>(count, 8)?;
            for _ in 0..count {
                let n = r.u32()? as usize;
                let name = r.take(n)?;
                let n = r.u32()? as usize;
                let start = r.pos;
                let payload = r.take(n)?;
                let mut child = Reader::new(
                    payload,
                    Limits {
                        max_bytes: n,
                        max_items: r.limits.max_items - r.items,
                        max_allocation: r.limits.max_allocation - r.allocation,
                        max_depth: r.limits.max_depth,
                    },
                )?;
                child.collect_ranges = r.collect_ranges;
                child.base = r.base + start;
                let value = Component::read(&mut child, name)?;
                if child.pos != payload.len() {
                    return Err(Error::TrailingBytes);
                }
                r.charge(child.items, child.allocation)?;
                r.ranges.append(&mut child.ranges);
                values.push(value);
            }
            Ok(values)
        })?;
        let result = Self {
            header,
            visibility,
            areas,
            landmasses,
            regions,
            chokepoints,
            continents,
            feature_layouts,
            plots,
            units,
            components,
        };
        result.validate_dimensions()?;
        if !allow_trailing && r.pos != bytes.len() {
            return Err(Error::TrailingBytes);
        }
        Ok((result, r.ranges, r.pos, r.items, r.allocation, plot_version))
    }
    fn validate_dimensions(&self) -> Result<(), Error> {
        if self.header.width < 0 || self.header.height < 0 {
            return Err(Error::InvalidDimensions);
        }
        let count = (self.header.width as usize)
            .checked_mul(self.header.height as usize)
            .ok_or(Error::ResourceLimit)?;
        if count != self.plots.len() {
            return Err(Error::PlotCountDimensions);
        }
        if self.units.width != self.header.width as u32
            || self.units.height != self.header.height as u32
        {
            return Err(Error::UnitDimensions);
        }
        for layer in &self.units.layers {
            if layer.cells.iter().any(|c| c.plot_index as usize >= count) {
                return Err(Error::InvalidPlotReference);
            }
        }
        Ok(())
    }
    /// In-bounds canonical coordinate addressing only. This does not emulate
    /// native wrapping or movement; negative/wrapped coordinates return None.
    pub fn plot_at(&self, x: i32, y: i32) -> Option<&Plot> {
        if x < 0 || y < 0 || x >= self.header.width || y >= self.header.height {
            return None;
        }
        let index = (y as usize)
            .checked_mul(self.header.width as usize)?
            .checked_add(x as usize)?;
        self.plots.get(index)
    }
    pub fn plot_by_index(&self, index: u32) -> Option<&Plot> {
        self.plots.get(index as usize)
    }
    pub fn coordinates(&self, index: u32) -> Option<(i32, i32)> {
        if self.header.width <= 0 || index as usize >= self.plots.len() {
            return None;
        }
        Some((
            (index % self.header.width as u32) as i32,
            (index / self.header.width as u32) as i32,
        ))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        self.validate_dimensions()?;
        let mut w = Writer::new(limits);
        let h = &self.header;
        w.i32(4)?;
        for v in [h.width, h.height, h.top_latitude, h.bottom_latitude] {
            w.i32(v)?;
        }
        w.boolean(h.wrap_x)?;
        w.boolean(h.wrap_y)?;
        w.u32(h.map_size_hash)?;
        for v in [h.land_plots, h.owned_plots, h.natural_wonders] {
            w.i32(v)?;
        }
        containers::write_hashes(&mut w, &h.resource_counts)?;
        containers::write_hashes(&mut w, &h.land_resource_counts)?;
        w.i32(1)?;
        w.list(&self.visibility, |w, v| {
            w.i32(v.player)?;
            v.visibility.write(w)
        })?;
        w.i32(1)?;
        self.areas.write(&mut w, |w, v| v.write(w))?;
        w.i32(1)?;
        self.landmasses.write(&mut w, |w, v| v.write(w))?;
        w.i32(1)?;
        self.regions.write(&mut w, |w, v| v.write(w))?;
        self.chokepoints.write(&mut w, |w, v| v.write(w))?;
        w.list(&self.continents, |w, v| v.write(w))?;
        w.i32(1)?;
        self.feature_layouts.write(&mut w, |w, v| v.write(w))?;
        w.i32(6)?;
        w.list(&self.plots, |w, p| p.write(w))?;
        self.units.write(&mut w)?;
        w.i32(1)?;
        w.charge::<Component>(self.components.len())?;
        w.u32(u32::try_from(self.components.len()).map_err(|_| Error::ResourceLimit)?)?;
        for component in &self.components {
            let mut child = Writer::new(Limits {
                max_bytes: w.limits.max_bytes - w.bytes.len(),
                max_items: w.limits.max_items - w.items,
                max_allocation: w.limits.max_allocation - w.allocation,
                max_depth: w.limits.max_depth,
            });
            component.write(&mut child)?;
            w.items += child.items;
            w.allocation += child.allocation;
            let name = component.name();
            w.u32(name.len() as u32)?;
            w.put(name)?;
            w.u32(u32::try_from(child.bytes.len()).map_err(|_| Error::ResourceLimit)?)?;
            w.put(&child.bytes)?;
        }
        Ok(w.bytes)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod prefix_tests {
    use super::*;
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
    #[test]
    fn prefix_consumes_only_map_and_reports_shared_budget_without_changing_strict_decode() {
        let map = Map {
            header: Header {
                width: 0,
                height: 0,
                top_latitude: 90,
                bottom_latitude: -90,
                wrap_x: false,
                wrap_y: false,
                map_size_hash: 0,
                land_plots: 0,
                owned_plots: 0,
                natural_wonders: 0,
                resource_counts: vec![DefinitionValue { hash: 7, value: -3 }],
                land_resource_counts: vec![],
            },
            visibility: vec![],
            areas: empty(),
            landmasses: empty(),
            regions: empty(),
            chokepoints: empty(),
            continents: vec![],
            feature_layouts: empty(),
            plots: vec![],
            units: UnitContainer {
                width: 0,
                height: 0,
                layers: vec![],
            },
            components: vec![],
        };
        let limits = Limits::default();
        let wire = map.encode(limits).unwrap();
        let (_, ranges) = Map::decode_with_ranges(&wire, limits).unwrap();
        let plots_start = ranges.iter().find(|r| r.name == "plots").unwrap().start;
        let mut v5 = wire.clone();
        v5[plots_start..plots_start + 4].copy_from_slice(&5i32.to_le_bytes());
        let (legacy, used, _, _, version) =
            Map::decode_prefix_with_version_and_usage(&v5, limits).unwrap();
        assert_eq!(legacy, map);
        assert_eq!((used, version), (wire.len(), 5));
        assert_eq!(legacy.encode(limits).unwrap(), wire);
        let (_, _, _, _, current_version) =
            Map::decode_prefix_with_version_and_usage(&wire, limits).unwrap();
        assert_eq!(current_version, 6);
        for unsupported in [4i32, 7] {
            v5[plots_start..plots_start + 4].copy_from_slice(&unsupported.to_le_bytes());
            assert!(
                matches!(Map::decode(&v5, limits), Err(Error::UnsupportedVersion { actual, .. }) if actual == unsupported)
            );
        }
        // Exercise version dispatch through a nonempty Map collection, not
        // only the standalone record decoder or an empty collection.
        let mut one = map.clone();
        one.header.width = 1;
        one.header.height = 1;
        one.units.width = 1;
        one.units.height = 1;
        let mut record = [0u8; 55];
        record[50] = 1; // FeatureVolatile source bit, cleared only by v5 Read.
        one.plots.push(Plot::decode(&record, 6, limits).unwrap());
        let mut one_wire = one.encode(limits).unwrap();
        let (_, one_ranges) = Map::decode_with_ranges(&one_wire, limits).unwrap();
        let plot_start = one_ranges.iter().find(|r| r.name == "plots").unwrap().start;
        one_wire[plot_start..plot_start + 4].copy_from_slice(&5i32.to_le_bytes());
        let (legacy_one, _, _, _, version) =
            Map::decode_prefix_with_version_and_usage(&one_wire, limits).unwrap();
        assert_eq!(version, 5);
        assert!(!legacy_one.plots[0].flags.get(PlotFlag::FeatureVolatile));
        assert!(one.plots[0].flags.get(PlotFlag::FeatureVolatile));
        assert_eq!(one_wire[plot_start + 8 + 50], 1);
        assert_eq!(
            Map::decode_prefix(
                &one_wire,
                Limits {
                    max_bytes: one_wire.len() - 1,
                    ..limits
                }
            ),
            Err(Error::ResourceLimit)
        );
        let mut carrier = wire.clone();
        carrier.extend_from_slice(b"untouched-Core-tail");
        let exact = Limits {
            max_bytes: wire.len(),
            ..limits
        };
        let (decoded, consumed, items) = Map::decode_prefix_with_items(&carrier, exact).unwrap();
        assert_eq!(decoded, map);
        assert_eq!(consumed, wire.len());
        assert_eq!(items, 1);
        let (usage_map, usage_consumed, usage_items, allocation) =
            Map::decode_prefix_with_usage(&carrier, exact).unwrap();
        assert_eq!(
            (usage_map, usage_consumed, usage_items),
            (map.clone(), consumed, items)
        );
        assert!(allocation > 0);
        assert!(Map::decode_prefix_with_usage(
            &carrier,
            Limits {
                max_allocation: allocation,
                ..exact
            }
        )
        .is_ok());
        assert_eq!(
            Map::decode_prefix_with_usage(
                &carrier,
                Limits {
                    max_allocation: allocation - 1,
                    ..exact
                }
            ),
            Err(Error::ResourceLimit)
        );
        assert_eq!(Map::decode(&wire, limits).unwrap(), map);
        assert_eq!(Map::decode(&carrier, limits), Err(Error::TrailingBytes));
        assert_eq!(Map::decode(&carrier, exact), Err(Error::ResourceLimit));
        assert_eq!(
            Map::decode_prefix_with_items(
                &carrier,
                Limits {
                    max_items: items - 1,
                    ..limits
                }
            ),
            Err(Error::ResourceLimit)
        );
        assert!(Map::decode_prefix(
            &carrier,
            Limits {
                max_bytes: wire.len() - 1,
                ..limits
            }
        )
        .is_err());
    }
}
