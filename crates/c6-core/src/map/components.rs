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
impl Eruption {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(7)?;
        Ok(Self {
            turn: r.i32()?,
            duration: r.i32()?,
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
        r.version(5)?;
        Ok(Self {
            name_hash: r.u32()?,
            plot: r.i32()?,
            discovering_civilization_hash: r.u32()?,
            turn_active: r.i32()?,
            custom_name: r.list(1, Reader::u8)?,
            eruptions: r.list(16, Eruption::read)?,
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
        r.version(5)?;
        Ok(Self {
            feature_hash: r.u32()?,
            plots: r.list(4, Reader::i32)?,
            eruptions: r.list(16, Eruption::read)?,
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
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(7)?;
        Ok(Self {
            extra_yields: SparsePlotData::read(r, read_hashes)?,
            volcanoes: r.list(40, Volcano::read)?,
            natural_wonder_volcanoes: r.list(16, NaturalWonderVolcano::read)?,
            plot_volcanoes: SparsePlotData::read(r, |r| r.list(4, Reader::i32))?,
            ice: r.list(12, Ice::read)?,
        })
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
        r.version(5)?;
        Ok(Self {
            turn: r.i32()?,
            duration: r.i32()?,
            severity: r.i32()?,
            mitigated: r.boolean()?,
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
        r.version(4)?;
        Ok(Self {
            name_hash: r.u32()?,
            id: r.i32()?,
            plots: r.list(4, Reader::i32)?,
            floodplains: r.list(4, Reader::i32)?,
            discovering_civilization_hash: r.u32()?,
            custom_name: r.list(1, Reader::u8)?,
            floods: r.list(17, Flood::read)?,
            edges: r.list(8, |r| Ok([r.i32()?, r.i32()?]))?,
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
        r.version(4)?;
        Ok(Self {
            next_id: r.i32()?,
            rivers: r.list(40, River::read)?,
            plot_rivers: SparsePlotData::read(r, |r| r.list(4, Reader::i32))?,
            floodplain_rivers: SparsePlotData::read(r, |r| r.list(4, Reader::i32))?,
        })
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
        r.version(6)?;
        Ok(Self {
            coastal_lowlands: r.list(33, CoastalLowland::read)?,
            deserts: r.list(20, GeographicalTerritory::read)?,
            mountain_ranges: r.list(20, GeographicalTerritory::read)?,
            named_lakes: r.list(20, NamedTerritory::read)?,
            named_oceans: r.list(20, NamedTerritory::read)?,
            named_seas: r.list(20, NamedTerritory::read)?,
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
