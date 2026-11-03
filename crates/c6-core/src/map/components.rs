use super::containers::{read_hashes, write_hashes};
use super::DefinitionValue;
use super::{Error, Reader, Recycled, Writer};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SparsePlotData<T> {
    pub default: T,
    pub indices: Vec<i32>,
    pub values: Vec<T>,
}
impl<T> SparsePlotData<T> {
    pub fn get(&self, plot: i32) -> Result<&T, Error> {
        let index = usize::try_from(plot)
            .ok()
            .and_then(|p| self.indices.get(p))
            .copied();
        match index {
            None | Some(-1) => Ok(&self.default),
            Some(index) => usize::try_from(index)
                .ok()
                .and_then(|i| self.values.get(i))
                .ok_or(Error::InvalidSparseIndex),
        }
    }
    fn read(
        r: &mut Reader<'_>,
        mut element: impl FnMut(&mut Reader<'_>) -> Result<T, Error>,
    ) -> Result<Self, Error> {
        r.version(4)?;
        let default = element(r)?;
        let indices = r.list(4, Reader::i32)?;
        let values = r.list(0, |r| element(r))?;
        let result = Self {
            default,
            indices,
            values,
        };
        result.validate()?;
        Ok(result)
    }
    fn validate(&self) -> Result<(), Error> {
        if self
            .indices
            .iter()
            .any(|i| *i < -1 || (*i >= 0 && *i as usize >= self.values.len()))
        {
            Err(Error::InvalidSparseIndex)
        } else {
            Ok(())
        }
    }
    fn write(
        &self,
        w: &mut Writer,
        mut element: impl FnMut(&mut Writer, &T) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.validate()?;
        w.i32(4)?;
        element(w, &self.default)?;
        w.list(&self.indices, |w, v| w.i32(*v))?;
        w.list(&self.values, |w, v| element(w, v))
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Eruption {
    pub turn: i32,
    pub duration: i32,
    pub severity: i32,
}
// Historical branches are pinned in linux-map-component-versions.md. Unknown
// versions remain errors; writers always emit their current versions.
fn read_version(r: &mut Reader<'_>, minimum: i32, current: i32) -> Result<i32, Error> {
    let actual = r.i32()?;
    if !(minimum..=current).contains(&actual) {
        return Err(Error::UnsupportedVersion {
            expected: current,
            actual,
        });
    }
    Ok(actual)
}
impl Eruption {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let version = read_version(r, 6, 7)?;
        Ok(Self {
            turn: r.i32()?,
            duration: if version >= 7 { r.i32()? } else { 0 },
            severity: r.i32()?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(7)?;
        w.i32(self.turn)?;
        w.i32(self.duration)?;
        w.i32(self.severity)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Volcano {
    pub name_hash: u32,
    pub plot: i32,
    pub discovering_civilization_hash: u32,
    pub turn_active: i32,
    pub custom_name: Vec<u8>,
    pub eruptions: Vec<Eruption>,
    pub affected_plots: Vec<i32>,
    pub periods_raw: Vec<i32>,
    pub chances_raw: Vec<i32>,
}
impl Volcano {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        read_version(r, 3, 5)?;
        Ok(Self {
            name_hash: r.u32()?,
            plot: r.i32()?,
            discovering_civilization_hash: r.u32()?,
            turn_active: r.i32()?,
            custom_name: r.list(1, Reader::u8)?,
            eruptions: r.list(12, Eruption::read)?,
            affected_plots: r.list(4, Reader::i32)?,
            periods_raw: r.list(4, Reader::i32)?,
            chances_raw: r.list(4, Reader::i32)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(5)?;
        w.u32(self.name_hash)?;
        w.i32(self.plot)?;
        w.u32(self.discovering_civilization_hash)?;
        w.i32(self.turn_active)?;
        w.list(&self.custom_name, |w, v| w.u8(*v))?;
        w.list(&self.eruptions, |w, v| v.write(w))?;
        w.list(&self.affected_plots, |w, v| w.i32(*v))?;
        w.list(&self.periods_raw, |w, v| w.i32(*v))?;
        w.list(&self.chances_raw, |w, v| w.i32(*v))
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NaturalWonderVolcano {
    pub feature_hash: u32,
    pub plots: Vec<i32>,
    pub eruptions: Vec<Eruption>,
}
impl NaturalWonderVolcano {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        read_version(r, 4, 5)?;
        Ok(Self {
            feature_hash: r.u32()?,
            plots: r.list(4, Reader::i32)?,
            eruptions: r.list(12, Eruption::read)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(5)?;
        w.u32(self.feature_hash)?;
        w.list(&self.plots, |w, v| w.i32(*v))?;
        w.list(&self.eruptions, |w, v| v.write(w))
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ice {
    pub plot: i32,
    pub random_event_hash: u32,
}
impl Ice {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(2)?;
        Ok(Self {
            plot: r.i32()?,
            random_event_hash: r.u32()?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(2)?;
        w.i32(self.plot)?;
        w.u32(self.random_event_hash)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeatureComponent {
    pub extra_yields: SparsePlotData<Vec<DefinitionValue>>,
    pub volcanoes: Vec<Volcano>,
    pub natural_wonder_volcanoes: Vec<NaturalWonderVolcano>,
    pub plot_volcanoes: SparsePlotData<Vec<i32>>,
    pub ice: Vec<Ice>,
}
impl FeatureComponent {
    /// Decode wire fields and retain the source version, without applying native
    /// map initialization or legacy Ice effects. The caller owns those steps.
    pub fn decode_wire(bytes: &[u8], limits: super::Limits) -> Result<(Self, i32), Error> {
        let mut r = Reader::new(bytes, limits)?;
        let value = Self::read_wire(&mut r)?;
        r.finish()?;
        Ok(value)
    }

    /// Apply the version-6 Plot flag restoration using actual resolved native
    /// RandomEvent collection indices in Ice order (-1 means lookup miss).
    /// None means unavailable definitions, not an empty native collection.
    /// All required input is checked before any Plot mutation. Native runtime
    /// volcano/cache rebuilds are separate and remain outside this helper.
    pub fn apply_legacy_ice_read(
        &self,
        version: i32,
        plots: &mut [super::Plot],
        resolved_events: Option<&[i32]>,
    ) -> Result<(), Error> {
        if !matches!(version, 6 | 7) {
            return Err(Error::UnsupportedVersion {
                expected: 7,
                actual: version,
            });
        }
        if version == 7 || self.ice.is_empty() {
            return Ok(());
        }
        let events = resolved_events.ok_or(Error::UnsupportedComponentBranch)?;
        if events.len() != self.ice.len() {
            return Err(Error::UnsupportedComponentBranch);
        }
        for (ice, event) in self.ice.iter().zip(events) {
            if *event != -1 {
                if let Some(plot) = usize::try_from(ice.plot)
                    .ok()
                    .and_then(|index| plots.get_mut(index))
                {
                    plot.flags.set(super::PlotFlag::FeatureVolatile, true);
                }
            }
        }
        Ok(())
    }

    fn read_wire(r: &mut Reader<'_>) -> Result<(Self, i32), Error> {
        let version = read_version(r, 6, 7)?;
        let value = Self {
            extra_yields: SparsePlotData::read(r, read_hashes)?,
            volcanoes: r.list(40, Volcano::read)?,
            natural_wonder_volcanoes: r.list(16, NaturalWonderVolcano::read)?,
            plot_volcanoes: SparsePlotData::read(r, |r| r.list(4, Reader::i32))?,
            ice: r.list(12, Ice::read)?,
        };
        Ok((value, version))
    }
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let (value, version) = Self::read_wire(r)?;
        // The default Map transport has no active RandomEvent registry input.
        // Empty Ice makes the versioned Plot restoration vacuous.
        if version == 6 && !value.ice.is_empty() {
            return Err(Error::UnsupportedComponentBranch);
        }
        Ok(value)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(7)?;
        self.extra_yields.write(w, |w, v| write_hashes(w, v))?;
        w.list(&self.volcanoes, |w, v| v.write(w))?;
        w.list(&self.natural_wonder_volcanoes, |w, v| v.write(w))?;
        self.plot_volcanoes
            .write(w, |w, v| w.list(v, |w, v| w.i32(*v)))?;
        w.list(&self.ice, |w, v| v.write(w))
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Flood {
    pub turn: i32,
    pub duration: i32,
    pub severity: i32,
    pub mitigated: bool,
}
impl Flood {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let version = read_version(r, 4, 5)?;
        Ok(Self {
            turn: r.i32()?,
            duration: r.i32()?,
            severity: r.i32()?,
            mitigated: if version >= 5 { r.boolean()? } else { false },
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(5)?;
        w.i32(self.turn)?;
        w.i32(self.duration)?;
        w.i32(self.severity)?;
        w.boolean(self.mitigated)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct River {
    pub name_hash: u32,
    pub id: i32,
    pub plots: Vec<i32>,
    pub floodplains: Vec<i32>,
    pub discovering_civilization_hash: u32,
    pub custom_name: Vec<u8>,
    pub floods: Vec<Flood>,
    pub edges: Vec<[i32; 2]>,
    pub periods_raw: Vec<i32>,
    pub chances_raw: Vec<i32>,
}
impl River {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let version = read_version(r, 3, 4)?;
        Ok(Self {
            name_hash: r.u32()?,
            id: r.i32()?,
            plots: r.list(4, Reader::i32)?,
            floodplains: r.list(4, Reader::i32)?,
            discovering_civilization_hash: r.u32()?,
            custom_name: r.list(1, Reader::u8)?,
            floods: r.list(16, Flood::read)?,
            edges: if version >= 4 {
                r.list(8, |r| Ok([r.i32()?, r.i32()?]))?
            } else {
                Vec::new()
            },
            periods_raw: r.list(4, Reader::i32)?,
            chances_raw: r.list(4, Reader::i32)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(4)?;
        w.u32(self.name_hash)?;
        w.i32(self.id)?;
        w.list(&self.plots, |w, v| w.i32(*v))?;
        w.list(&self.floodplains, |w, v| w.i32(*v))?;
        w.u32(self.discovering_civilization_hash)?;
        w.list(&self.custom_name, |w, v| w.u8(*v))?;
        w.list(&self.floods, |w, v| v.write(w))?;
        w.list(&self.edges, |w, v| {
            w.i32(v[0])?;
            w.i32(v[1])
        })?;
        w.list(&self.periods_raw, |w, v| w.i32(*v))?;
        w.list(&self.chances_raw, |w, v| w.i32(*v))
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RiverComponent {
    pub next_id: i32,
    pub rivers: Vec<River>,
    pub plot_rivers: SparsePlotData<Vec<i32>>,
    pub floodplain_rivers: SparsePlotData<Vec<i32>>,
}
impl RiverComponent {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let version = read_version(r, 3, 4)?;
        let mut value = Self {
            next_id: if version >= 4 { r.i32()? } else { 0 },
            rivers: r.list(40, River::read)?,
            plot_rivers: SparsePlotData::read(r, |r| r.list(4, Reader::i32))?,
            floodplain_rivers: SparsePlotData::read(r, |r| r.list(4, Reader::i32))?,
        };
        // Native Read performs this even for current-version zero next_id.
        if value.next_id == 0 {
            value.next_id = value
                .rivers
                .iter()
                .fold(0, |max, river| max.max(river.id))
                .wrapping_add(1);
        }
        Ok(value)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(4)?;
        w.i32(self.next_id)?;
        w.list(&self.rivers, |w, v| v.write(w))?;
        self.plot_rivers
            .write(w, |w, v| w.list(v, |w, v| w.i32(*v)))?;
        self.floodplain_rivers
            .write(w, |w, v| w.list(v, |w, v| w.i32(*v)))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AreaPortal {
    pub entrance_plot_index: i32,
    pub exit_plot_indices: Vec<i32>,
    pub source_plot_indices: Vec<i32>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteComponent {
    pub width: u32,
    pub height: u32,
    pub portals: Vec<AreaPortal>,
}
impl RouteComponent {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(1)?;
        r.version(1)?;
        let width = r.u32()?;
        let height = r.u32()?;
        let count = r.u32()? as usize;
        let mut portals = r.vector::<AreaPortal>(count, 12)?;
        if count != 0 {
            r.version(2)?;
        }
        for _ in 0..count {
            portals.push(AreaPortal {
                entrance_plot_index: r.i32()?,
                exit_plot_indices: r.list(4, Reader::i32)?,
                source_plot_indices: r.list(4, Reader::i32)?,
            });
        }
        Ok(Self {
            width,
            height,
            portals,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(1)?;
        w.i32(1)?;
        w.u32(self.width)?;
        w.u32(self.height)?;
        w.charge::<AreaPortal>(self.portals.len())?;
        w.u32(u32::try_from(self.portals.len()).map_err(|_| Error::ResourceLimit)?)?;
        if !self.portals.is_empty() {
            w.i32(2)?;
        }
        for p in &self.portals {
            w.i32(p.entrance_plot_index)?;
            w.list(&p.exit_plot_indices, |w, v| w.i32(*v))?;
            w.list(&p.source_plot_indices, |w, v| w.i32(*v))?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeographicalTerritory {
    pub type_hash: u32,
    pub discovering_civilization_hash: u32,
    pub territory_id: i32,
    pub custom_name: Vec<u8>,
}
impl GeographicalTerritory {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(6)?;
        Ok(Self {
            type_hash: r.u32()?,
            discovering_civilization_hash: r.u32()?,
            territory_id: r.i32()?,
            custom_name: r.list(1, Reader::u8)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(6)?;
        w.u32(self.type_hash)?;
        w.u32(self.discovering_civilization_hash)?;
        w.i32(self.territory_id)?;
        w.list(&self.custom_name, |w, v| w.u8(*v))
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamedTerritory {
    pub kind: i32,
    pub discovering_civilization_hash: u32,
    pub territory_id: i32,
    pub custom_name: Vec<u8>,
}
impl NamedTerritory {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(1)?;
        Ok(Self {
            kind: r.i32()?,
            discovering_civilization_hash: r.u32()?,
            territory_id: r.i32()?,
            custom_name: r.list(1, Reader::u8)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(1)?;
        w.i32(self.kind)?;
        w.u32(self.discovering_civilization_hash)?;
        w.i32(self.territory_id)?;
        w.list(&self.custom_name, |w, v| w.u8(*v))
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoastalLowland {
    pub lowland_type_hash: u32,
    pub plot_index: i32,
    pub original_terrain_hash: u32,
    pub river_east_flow: i8,
    pub river_south_east_flow: i8,
    pub river_south_west_flow: i8,
    pub river_west_flow: i8,
    pub river_north_west_flow: i8,
    pub river_north_east_flow: i8,
    pub river_crossing_count: i8,
    pub river_crossing_mask: u8,
    pub river_west: bool,
    pub river_north_west: bool,
    pub river_north_east: bool,
    pub river_east: bool,
    pub river_south_east: bool,
    pub river_south_west: bool,
    pub flooded: bool,
    pub submerged: bool,
    pub protected: bool,
}
impl CoastalLowland {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(9)?;
        Ok(Self {
            lowland_type_hash: r.u32()?,
            plot_index: r.i32()?,
            original_terrain_hash: r.u32()?,
            river_east_flow: r.i8()?,
            river_south_east_flow: r.i8()?,
            river_south_west_flow: r.i8()?,
            river_west_flow: r.i8()?,
            river_north_west_flow: r.i8()?,
            river_north_east_flow: r.i8()?,
            river_crossing_count: r.i8()?,
            river_crossing_mask: r.u8()?,
            river_west: r.boolean()?,
            river_north_west: r.boolean()?,
            river_north_east: r.boolean()?,
            river_east: r.boolean()?,
            river_south_east: r.boolean()?,
            river_south_west: r.boolean()?,
            flooded: r.boolean()?,
            submerged: r.boolean()?,
            protected: r.boolean()?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(9)?;
        w.u32(self.lowland_type_hash)?;
        w.i32(self.plot_index)?;
        w.u32(self.original_terrain_hash)?;
        for value in [
            self.river_east_flow,
            self.river_south_east_flow,
            self.river_south_west_flow,
            self.river_west_flow,
            self.river_north_west_flow,
            self.river_north_east_flow,
            self.river_crossing_count,
        ] {
            w.i8(value)?;
        }
        w.u8(self.river_crossing_mask)?;
        for value in [
            self.river_west,
            self.river_north_west,
            self.river_north_east,
            self.river_east,
            self.river_south_east,
            self.river_south_west,
            self.flooded,
            self.submerged,
            self.protected,
        ] {
            w.boolean(value)?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerrainComponent {
    pub coastal_lowlands: Vec<CoastalLowland>,
    pub deserts: Vec<GeographicalTerritory>,
    pub mountain_ranges: Vec<GeographicalTerritory>,
    pub named_lakes: Vec<NamedTerritory>,
    pub named_oceans: Vec<NamedTerritory>,
    pub named_seas: Vec<NamedTerritory>,
}
impl TerrainComponent {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let version = read_version(r, 2, 6)?;
        Ok(Self {
            coastal_lowlands: r.list(33, CoastalLowland::read)?,
            deserts: if version >= 5 {
                r.list(20, GeographicalTerritory::read)?
            } else {
                Vec::new()
            },
            mountain_ranges: if version >= 5 {
                r.list(20, GeographicalTerritory::read)?
            } else {
                Vec::new()
            },
            named_lakes: if version >= 6 {
                r.list(20, NamedTerritory::read)?
            } else {
                Vec::new()
            },
            named_oceans: if version >= 6 {
                r.list(20, NamedTerritory::read)?
            } else {
                Vec::new()
            },
            named_seas: if version >= 6 {
                r.list(20, NamedTerritory::read)?
            } else {
                Vec::new()
            },
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(6)?;
        w.list(&self.coastal_lowlands, |w, v| v.write(w))?;
        w.list(&self.deserts, |w, v| v.write(w))?;
        w.list(&self.mountain_ranges, |w, v| v.write(w))?;
        for values in [&self.named_lakes, &self.named_oceans, &self.named_seas] {
            w.list(values, |w, v| v.write(w))?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Territory {
    pub id: i32,
    pub terrain_class_hash: u32,
    pub plot_count: i32,
    pub coastal_plot_count: i32,
    pub coastal_hill_mountain_count: i32,
    pub river_edge_count: i32,
    pub natural_wonder_count: i32,
    pub feature_count: i32,
    pub is_lake: bool,
    pub is_sea: bool,
    pub east: i32,
    pub north: i32,
    pub south: i32,
    pub west: i32,
    pub centroid_delta: [i32; 2],
    pub feature_counts: Vec<DefinitionValue>,
    pub name: super::LocalizedText,
}
impl Territory {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(2)?;
        Ok(Self {
            id: r.i32()?,
            terrain_class_hash: r.u32()?,
            plot_count: r.i32()?,
            coastal_plot_count: r.i32()?,
            coastal_hill_mountain_count: r.i32()?,
            river_edge_count: r.i32()?,
            natural_wonder_count: r.i32()?,
            feature_count: r.i32()?,
            is_lake: r.boolean()?,
            is_sea: r.boolean()?,
            east: r.i32()?,
            north: r.i32()?,
            south: r.i32()?,
            west: r.i32()?,
            centroid_delta: [r.i32()?, r.i32()?],
            feature_counts: read_hashes(r)?,
            name: super::LocalizedText::read(r, 0)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(2)?;
        w.i32(self.id)?;
        w.u32(self.terrain_class_hash)?;
        for value in [
            self.plot_count,
            self.coastal_plot_count,
            self.coastal_hill_mountain_count,
            self.river_edge_count,
            self.natural_wonder_count,
            self.feature_count,
        ] {
            w.i32(value)?;
        }
        w.boolean(self.is_lake)?;
        w.boolean(self.is_sea)?;
        for value in [
            self.east,
            self.north,
            self.south,
            self.west,
            self.centroid_delta[0],
            self.centroid_delta[1],
        ] {
            w.i32(value)?;
        }
        write_hashes(w, &self.feature_counts)?;
        self.name.write(w, 0)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Component {
    Route(RouteComponent),
    Feature(Box<FeatureComponent>),
    River(Box<RiverComponent>),
    Terrain(TerrainComponent),
    Territories {
        container: Recycled<Territory>,
        plot_territory_ids: Vec<u32>,
    },
    TerritoryBuilder,
}
impl Component {
    pub fn name(&self) -> &'static [u8] {
        match self {
            Self::Route(_) => b"Route",
            Self::Feature(_) => b"Feature",
            Self::River(_) => b"River",
            Self::Terrain(_) => b"Terrain",
            Self::Territories { .. } => b"Territories",
            Self::TerritoryBuilder => b"TerritoryBuilder",
        }
    }
    pub(super) fn read(r: &mut Reader<'_>, name: &[u8]) -> Result<Self, Error> {
        match name {
            b"Route" => Ok(Self::Route(RouteComponent::read(r)?)),
            b"Feature" => {
                r.charge(1, std::mem::size_of::<FeatureComponent>())?;
                Ok(Self::Feature(Box::new(FeatureComponent::read(r)?)))
            }
            b"River" => {
                r.charge(1, std::mem::size_of::<RiverComponent>())?;
                Ok(Self::River(Box::new(RiverComponent::read(r)?)))
            }
            b"Terrain" => Ok(Self::Terrain(TerrainComponent::read(r)?)),
            b"Territories" => {
                r.version(1)?;
                let container = Recycled::read(r, Territory::read)?;
                let plot_territory_ids = r.list(4, Reader::u32)?;
                Ok(Self::Territories {
                    container,
                    plot_territory_ids,
                })
            }
            b"TerritoryBuilder" => Ok(Self::TerritoryBuilder),
            _ => Err(Error::UnsupportedComponent),
        }
    }
    pub(super) fn write(&self, w: &mut Writer) -> Result<(), Error> {
        match self {
            Self::Route(v) => v.write(w),
            Self::Feature(v) => {
                w.charge::<FeatureComponent>(1)?;
                v.write(w)
            }
            Self::River(v) => {
                w.charge::<RiverComponent>(1)?;
                v.write(w)
            }
            Self::Terrain(v) => v.write(w),
            Self::Territories {
                container,
                plot_territory_ids,
            } => {
                w.i32(1)?;
                container.write(w, |w, v| v.write(w))?;
                w.list(plot_territory_ids, |w, v| w.u32(*v))
            }
            Self::TerritoryBuilder => Ok(()),
        }
    }
}

#[cfg(test)]
mod version_tests {
    use super::*;
    use crate::map::Limits;

    fn words(values: &[i32]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }
    fn decode<T>(bytes: &[u8], read: fn(&mut Reader<'_>) -> Result<T, Error>) -> Result<T, Error> {
        let mut r = Reader::new(bytes, Limits::default())?;
        let value = read(&mut r)?;
        r.finish()?;
        Ok(value)
    }
    fn sparse() -> Vec<u8> {
        words(&[4, 0, 0, 0])
    }
    fn feature(version: i32, ice: &[i32]) -> Vec<u8> {
        let mut b = words(&[version]);
        b.extend(sparse());
        b.extend(words(&[0, 0]));
        b.extend(sparse());
        b.extend(words(&[(ice.len() / 3) as i32]));
        b.extend(words(ice));
        b
    }
    fn river(version: i32, id: i32) -> Vec<u8> {
        let mut b = words(&[version, 123, id, 0, 0, 456, 0, 0]);
        if version >= 4 {
            b.extend(words(&[0]));
        }
        b.extend(words(&[0, 0]));
        b
    }
    fn river_manager(version: i32, next: i32, ids: &[i32]) -> Vec<u8> {
        let mut b = words(&[version]);
        if version >= 4 {
            b.extend(words(&[next]));
        }
        b.extend(words(&[ids.len() as i32]));
        for id in ids {
            b.extend(river(3, *id));
        }
        b.extend(sparse());
        b.extend(sparse());
        b
    }
    #[test]
    fn feature6_empty_ice_is_vacuous_and_nonempty_requires_definitions() {
        let old = decode(&feature(6, &[]), FeatureComponent::read).unwrap();
        let current = decode(&feature(7, &[]), FeatureComponent::read).unwrap();
        assert_eq!(old, current);
        let ice = [2, 12, 0x12345678];
        assert_eq!(
            decode(&feature(6, &ice), FeatureComponent::read),
            Err(Error::UnsupportedComponentBranch)
        );
        assert_eq!(
            decode(&feature(7, &ice), FeatureComponent::read)
                .unwrap()
                .ice,
            vec![Ice {
                plot: 12,
                random_event_hash: 0x12345678
            }]
        );
        for version in [-1, 0, 5, 8, i32::MAX] {
            assert!(
                matches!(decode(&feature(version, &[]), FeatureComponent::read), Err(Error::UnsupportedVersion { expected: 7, actual }) if actual == version)
            );
        }
        for version in [6, 7] {
            let bytes = feature(version, &[]);
            for n in 0..bytes.len() {
                assert!(
                    decode(&bytes[..n], FeatureComponent::read).is_err(),
                    "version {version} prefix {n}"
                );
            }
        }
    }
    #[test]
    fn wire_decode_retains_ice_and_version_and_apply_preflights_resolution() {
        let bytes = feature(6, &[2, 0, 123, 2, 1, 456, 2, -1, 123, 2, 99, 123]);
        let (value, version) = FeatureComponent::decode_wire(&bytes, Limits::default()).unwrap();
        assert_eq!(version, 6);
        assert_eq!(value.ice.len(), 4);
        assert_eq!(value.ice[0].random_event_hash, 123);
        let plot = super::super::Plot::decode(&[0; 55], 6, Limits::default()).unwrap();
        let mut plots = vec![plot.clone(), plot];
        let preimage = plots.clone();
        assert_eq!(
            value.apply_legacy_ice_read(6, &mut plots, None),
            Err(Error::UnsupportedComponentBranch)
        );
        assert_eq!(plots, preimage);
        assert_eq!(
            value.apply_legacy_ice_read(6, &mut plots, Some(&[0])),
            Err(Error::UnsupportedComponentBranch)
        );
        assert_eq!(plots, preimage);
        value.apply_legacy_ice_read(7, &mut plots, None).unwrap();
        assert_eq!(plots, preimage);
        value
            .apply_legacy_ice_read(6, &mut plots, Some(&[7, -1, 0, 0]))
            .unwrap();
        assert!(plots[0].flags.get(super::super::PlotFlag::FeatureVolatile));
        assert!(!plots[1].flags.get(super::super::PlotFlag::FeatureVolatile));
        assert_eq!(value.ice[0].random_event_hash, 123);
        assert_eq!(
            value.apply_legacy_ice_read(8, &mut plots, None),
            Err(Error::UnsupportedVersion {
                expected: 7,
                actual: 8
            })
        );
    }
    #[test]
    fn historical_leaf_defaults_and_current_bits() {
        assert_eq!(
            decode(&words(&[6, i32::MIN, -1]), Eruption::read).unwrap(),
            Eruption {
                turn: i32::MIN,
                duration: 0,
                severity: -1
            }
        );
        assert_eq!(
            decode(&words(&[7, -2, i32::MIN, i32::MAX]), Eruption::read).unwrap(),
            Eruption {
                turn: -2,
                duration: i32::MIN,
                severity: i32::MAX
            }
        );
        assert_eq!(
            decode(&words(&[4, -2, i32::MIN, -1]), Flood::read).unwrap(),
            Flood {
                turn: -2,
                duration: i32::MIN,
                severity: -1,
                mitigated: false
            }
        );
        let mut current = words(&[5, -2, 17, -1]);
        current.push(1);
        assert!(decode(&current, Flood::read).unwrap().mitigated);
        current[16] = 2;
        assert_eq!(
            decode(&current, Flood::read),
            Err(Error::NoncanonicalBool(2))
        );
        assert!(matches!(
            decode(&words(&[5]), Eruption::read),
            Err(Error::UnsupportedVersion { .. })
        ));
        assert!(matches!(
            decode(&words(&[3]), Flood::read),
            Err(Error::UnsupportedVersion { .. })
        ));
    }
    #[test]
    fn volcano_compatible_versions_accept_short_old_eruption_records() {
        // Nonempty child arrays catch minimum-wire assumptions at nesting boundaries.
        for version in [3, 4, 5] {
            let b = words(&[
                version,
                123,
                7,
                456,
                -1,
                0,
                1,
                6,
                -99,
                2,
                1,
                7,
                1,
                i32::MIN,
                1,
                i32::MAX,
            ]);
            let v = decode(&b, Volcano::read).unwrap();
            assert_eq!(v.eruptions[0].duration, 0);
            assert_eq!(v.affected_plots, vec![7]);
            assert_eq!(v.periods_raw, vec![i32::MIN]);
            assert_eq!(v.chances_raw, vec![i32::MAX]);
            for n in 0..b.len() {
                assert!(decode(&b[..n], Volcano::read).is_err());
            }
        }
        for version in [4, 5] {
            let b = words(&[version, 123, 1, 7, 1, 6, -99, 2]);
            let v = decode(&b, NaturalWonderVolcano::read).unwrap();
            assert_eq!(v.plots, vec![7]);
            assert_eq!(v.eruptions[0].duration, 0);
        }
        assert!(matches!(
            decode(&words(&[2]), Volcano::read),
            Err(Error::UnsupportedVersion { .. })
        ));
        assert!(matches!(
            decode(&words(&[6]), Volcano::read),
            Err(Error::UnsupportedVersion { .. })
        ));
    }
    #[test]
    fn river3_omitted_edges_and_native_next_id_conversion() {
        assert!(decode(&river(3, 17), River::read).unwrap().edges.is_empty());
        for version in [3, 4] {
            for (ids, expected) in [
                (vec![], 1),
                (vec![-5, -1], 1),
                (vec![5, 2, 17], 18),
                (vec![i32::MAX], i32::MIN),
            ] {
                let b = river_manager(version, 0, &ids);
                assert_eq!(decode(&b, RiverComponent::read).unwrap().next_id, expected);
                for n in 0..b.len() {
                    assert!(decode(&b[..n], RiverComponent::read).is_err());
                }
            }
        }
        assert_eq!(
            decode(&river_manager(4, -7, &[17]), RiverComponent::read)
                .unwrap()
                .next_id,
            -7
        );
        assert_eq!(
            decode(&river_manager(4, 9, &[17]), RiverComponent::read)
                .unwrap()
                .next_id,
            9
        );
        assert!(matches!(
            decode(&words(&[2]), RiverComponent::read),
            Err(Error::UnsupportedVersion { .. })
        ));
    }
    #[test]
    fn terrain5_constructor_empty_named_vectors_and_current6() {
        for version in [2, 3, 4] {
            let old = words(&[version, 0]);
            let value = decode(&old, TerrainComponent::read).unwrap();
            assert!(value.deserts.is_empty() && value.mountain_ranges.is_empty());
            assert!(
                value.named_lakes.is_empty()
                    && value.named_oceans.is_empty()
                    && value.named_seas.is_empty()
            );
            for n in 0..old.len() {
                assert!(decode(&old[..n], TerrainComponent::read).is_err());
            }
        }
        let old = words(&[5, 0, 0, 0]);
        let current = words(&[6, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            decode(&old, TerrainComponent::read).unwrap(),
            decode(&current, TerrainComponent::read).unwrap()
        );
        assert_eq!(
            decode(&current[..16], TerrainComponent::read),
            Err(Error::Truncated)
        );
        assert_eq!(
            decode(&[old.clone(), words(&[0])].concat(), TerrainComponent::read),
            Err(Error::TrailingBytes)
        );
        for n in 0..old.len() {
            assert!(decode(&old[..n], TerrainComponent::read).is_err());
        }
    }
    #[test]
    fn historical_component_budgets_remain_enforced() {
        let bytes = river_manager(3, 0, &[7]);
        for limits in [
            Limits {
                max_items: 0,
                ..Limits::default()
            },
            Limits {
                max_allocation: 0,
                ..Limits::default()
            },
        ] {
            let mut r = Reader::new(&bytes, limits).unwrap();
            assert_eq!(RiverComponent::read(&mut r), Err(Error::ResourceLimit));
        }
        let mut bytes = feature(6, &[]);
        // First sparse data's indices count requests an impossible vector.
        bytes[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            decode(&bytes, FeatureComponent::read),
            Err(Error::Truncated)
        );
    }
}
