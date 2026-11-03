use super::{Error, Reader, Writer};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DefinitionValue {
    pub hash: u32,
    pub value: i32,
}
fn hashes_read(r: &mut Reader<'_>) -> Result<Vec<DefinitionValue>, Error> {
    r.list(8, |r| {
        Ok(DefinitionValue {
            hash: r.u32()?,
            value: r.i32()?,
        })
    })
}
fn hashes_write(w: &mut Writer, v: &[DefinitionValue]) -> Result<(), Error> {
    w.list(v, |w, v| {
        w.u32(v.hash)?;
        w.i32(v.value)
    })
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Slot<T> {
    pub index: i32,
    pub value: T,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Recycled<T> {
    pub capacity: u32,
    pub entries: Vec<Slot<T>>,
    pub current_id_bits: u32,
    pub free_head: u32,
    pub free_tail: u32,
    pub free_links: Vec<u32>,
}
impl<T> Recycled<T> {
    pub(super) fn read(
        r: &mut Reader<'_>,
        mut element: impl FnMut(&mut Reader<'_>) -> Result<T, Error>,
    ) -> Result<Self, Error> {
        r.version(65537)?;
        if r.u32()? != 16 {
            return Err(Error::InvalidContainerMetadata);
        }
        let capacity = r.u32()?;
        let count = r.u32()? as usize;
        if count > capacity as usize {
            return Err(Error::InvalidContainerMetadata);
        }
        let mut entries = r.vector::<Slot<T>>(count, 4)?;
        for _ in 0..count {
            entries.push(Slot {
                index: r.i32()?,
                value: element(r)?,
            });
        }
        let current_id_bits = r.u32()?;
        let free_count = r.u32()? as usize;
        let free_head = r.u32()?;
        let free_tail = r.u32()?;
        let mut free_links = r.vector::<u32>(free_count, 4)?;
        for _ in 0..free_count {
            free_links.push(r.u32()?);
        }
        Ok(Self {
            capacity,
            entries,
            current_id_bits,
            free_head,
            free_tail,
            free_links,
        })
    }
    pub(super) fn write(
        &self,
        w: &mut Writer,
        mut element: impl FnMut(&mut Writer, &T) -> Result<(), Error>,
    ) -> Result<(), Error> {
        if self.entries.len() > self.capacity as usize {
            return Err(Error::InvalidContainerMetadata);
        }
        w.i32(65537)?;
        w.u32(16)?;
        w.u32(self.capacity)?;
        w.charge::<Slot<T>>(self.entries.len())?;
        w.u32(u32::try_from(self.entries.len()).map_err(|_| Error::ResourceLimit)?)?;
        for entry in &self.entries {
            w.i32(entry.index)?;
            element(w, &entry.value)?;
        }
        w.u32(self.current_id_bits)?;
        w.charge::<u32>(self.free_links.len())?;
        w.u32(u32::try_from(self.free_links.len()).map_err(|_| Error::ResourceLimit)?)?;
        w.u32(self.free_head)?;
        w.u32(self.free_tail)?;
        for value in &self.free_links {
            w.u32(*value)?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Area {
    pub id: i32,
    pub plot_count: i32,
    pub coastal_plot_count: i32,
    pub coastal_hill_mountain_count: i32,
    pub owned_plot_count: i32,
    pub river_edge_count: i32,
    pub unit_count: i32,
    pub city_count: i32,
    pub total_population: i32,
    pub starting_plot_count: i32,
    pub natural_wonder_count: i32,
    pub units_per_player: Vec<i32>,
    pub cities_per_player: Vec<i32>,
    pub population_per_player: Vec<i32>,
    pub free_specialist: Vec<i32>,
    pub revealed_plot_count: Vec<i32>,
    pub yields_per_player: [Vec<DefinitionValue>; 64],
    pub resource_counts: Vec<DefinitionValue>,
    pub improvement_counts: Vec<DefinitionValue>,
    pub east: i32,
    pub north: i32,
    pub south: i32,
    pub west: i32,
    pub water: bool,
    pub mountains: bool,
    pub canyons: bool,
}
impl Area {
    pub(super) fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(2)?;
        let id = r.i32()?;
        let plot_count = r.i32()?;
        let coastal_plot_count = r.i32()?;
        let coastal_hill_mountain_count = r.i32()?;
        let owned_plot_count = r.i32()?;
        let river_edge_count = r.i32()?;
        let unit_count = r.i32()?;
        let city_count = r.i32()?;
        let total_population = r.i32()?;
        let starting_plot_count = r.i32()?;
        let natural_wonder_count = r.i32()?;
        let mut player_array = || -> Result<Vec<i32>, Error> {
            let values = r.list(4, Reader::i32)?;
            if values.len() != 64 {
                return Err(Error::PlayerArrayWidth);
            }
            Ok(values)
        };
        let units_per_player = player_array()?;
        let cities_per_player = player_array()?;
        let population_per_player = player_array()?;
        let free_specialist = player_array()?;
        let revealed_plot_count = player_array()?;
        let mut yields_per_player = std::array::from_fn(|_| Vec::new());
        for values in &mut yields_per_player {
            *values = hashes_read(r)?;
        }
        let resource_counts = hashes_read(r)?;
        let improvement_counts = hashes_read(r)?;
        let east = r.i32()?;
        let north = r.i32()?;
        let south = r.i32()?;
        let west = r.i32()?;
        let water = r.boolean()?;
        let mountains = r.boolean()?;
        let canyons = r.boolean()?;
        Ok(Self {
            id,
            plot_count,
            coastal_plot_count,
            coastal_hill_mountain_count,
            owned_plot_count,
            river_edge_count,
            unit_count,
            city_count,
            total_population,
            starting_plot_count,
            natural_wonder_count,
            units_per_player,
            cities_per_player,
            population_per_player,
            free_specialist,
            revealed_plot_count,
            yields_per_player,
            resource_counts,
            improvement_counts,
            east,
            north,
            south,
            west,
            water,
            mountains,
            canyons,
        })
    }
    pub(super) fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(2)?;
        for value in [
            self.id,
            self.plot_count,
            self.coastal_plot_count,
            self.coastal_hill_mountain_count,
            self.owned_plot_count,
            self.river_edge_count,
            self.unit_count,
            self.city_count,
            self.total_population,
            self.starting_plot_count,
            self.natural_wonder_count,
        ] {
            w.i32(value)?;
        }
        for values in [
            &self.units_per_player,
            &self.cities_per_player,
            &self.population_per_player,
            &self.free_specialist,
            &self.revealed_plot_count,
        ] {
            if values.len() != 64 {
                return Err(Error::PlayerArrayWidth);
            }
            w.list(values, |w, v| w.i32(*v))?;
        }
        for values in &self.yields_per_player {
            hashes_write(w, values)?;
        }
        hashes_write(w, &self.resource_counts)?;
        hashes_write(w, &self.improvement_counts)?;
        for value in [self.east, self.north, self.south, self.west] {
            w.i32(value)?;
        }
        for value in [self.water, self.mountains, self.canyons] {
            w.boolean(value)?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Landmass {
    pub id: i32,
    pub plot_count: i32,
    pub water: bool,
    pub centroid_delta: [i32; 2],
}
impl Landmass {
    pub(super) fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(1)?;
        Ok(Self {
            id: r.i32()?,
            plot_count: r.i32()?,
            water: r.boolean()?,
            centroid_delta: [r.i32()?, r.i32()?],
        })
    }
    pub(super) fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(1)?;
        w.i32(self.id)?;
        w.i32(self.plot_count)?;
        w.boolean(self.water)?;
        w.i32(self.centroid_delta[0])?;
        w.i32(self.centroid_delta[1])
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Region {
    pub id: u32,
    pub plot_count: i32,
    pub chokepoint: bool,
    pub area_id: u32,
    pub joined_region_ids: Vec<u32>,
}
impl Region {
    pub(super) fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(1)?;
        Ok(Self {
            id: r.u32()?,
            plot_count: r.i32()?,
            chokepoint: r.boolean()?,
            area_id: r.u32()?,
            joined_region_ids: r.list(4, Reader::u32)?,
        })
    }
    pub(super) fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(1)?;
        w.u32(self.id)?;
        w.i32(self.plot_count)?;
        w.boolean(self.chokepoint)?;
        w.u32(self.area_id)?;
        w.list(&self.joined_region_ids, |w, v| w.u32(*v))
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Chokepoint {
    pub id: u32,
    pub initialized: bool,
    pub endpoints: Vec<[i32; 2]>,
    pub points: Vec<[i32; 2]>,
    pub joined_area_ids: Vec<u32>,
}
impl Chokepoint {
    pub(super) fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(1)?;
        Ok(Self {
            id: r.u32()?,
            initialized: r.boolean()?,
            endpoints: r.list(8, |r| Ok([r.i32()?, r.i32()?]))?,
            points: r.list(8, |r| Ok([r.i32()?, r.i32()?]))?,
            joined_area_ids: r.list(4, Reader::u32)?,
        })
    }
    pub(super) fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(1)?;
        w.u32(self.id)?;
        w.boolean(self.initialized)?;
        w.list(&self.endpoints, |w, v| {
            w.i32(v[0])?;
            w.i32(v[1])
        })?;
        w.list(&self.points, |w, v| {
            w.i32(v[0])?;
            w.i32(v[1])
        })?;
        w.list(&self.joined_area_ids, |w, v| w.u32(*v))
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Continent {
    pub id: i32,
    pub type_hash: u32,
    pub plot_count: i32,
    pub fertility: u32,
}
impl Continent {
    pub(super) fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(1)?;
        Ok(Self {
            id: r.i32()?,
            type_hash: r.u32()?,
            plot_count: r.i32()?,
            fertility: r.u32()?,
        })
    }
    pub(super) fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(1)?;
        w.i32(self.id)?;
        w.u32(self.type_hash)?;
        w.i32(self.plot_count)?;
        w.u32(self.fertility)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeatureLayout {
    pub lowest_plot: u16,
    pub plot_indices: Vec<u16>,
}
impl FeatureLayout {
    pub(super) fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            lowest_plot: r.u16()?,
            plot_indices: r.list(2, Reader::u16)?,
        })
    }
    pub(super) fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.u16(self.lowest_plot)?;
        w.list(&self.plot_indices, |w, v| w.u16(*v))
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VisibilityLayer {
    pub type_hash: u32,
    pub index: u32,
    pub count: u32,
    pub grid_width: u32,
    pub values: Vec<i16>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutgoingVisibility {
    pub player_id: i32,
    pub count: u32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Visibility {
    pub player_id: i32,
    pub cached_num_revealed_hexes: u32,
    pub cached_old: bool,
    pub revealed: Vec<u8>,
    pub layers: Vec<VisibilityLayer>,
    pub outgoing_visibility: Vec<OutgoingVisibility>,
    pub outgoing_visibility_blockers: Vec<OutgoingVisibility>,
}
impl Visibility {
    pub(super) fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(6)?;
        let player_id = r.i32()?;
        let cached_num_revealed_hexes = r.u32()?;
        let cached_old = r.boolean()?;
        let revealed = r.list(1, Reader::u8)?;
        let layers = r.list(20, |r| {
            let type_hash = r.u32()?;
            let index = r.u32()?;
            let count = r.u32()?;
            let grid_width = r.u32()?;
            let values = r.list(2, Reader::i16)?;
            if values.len() != count as usize {
                return Err(Error::LayerCountMismatch);
            }
            Ok(VisibilityLayer {
                type_hash,
                index,
                count,
                grid_width,
                values,
            })
        })?;
        let mut outgoing = || {
            r.list(12, |r| {
                r.version(6)?;
                Ok(OutgoingVisibility {
                    player_id: r.i32()?,
                    count: r.u32()?,
                })
            })
        };
        let outgoing_visibility = outgoing()?;
        let outgoing_visibility_blockers = outgoing()?;
        Ok(Self {
            player_id,
            cached_num_revealed_hexes,
            cached_old,
            revealed,
            layers,
            outgoing_visibility,
            outgoing_visibility_blockers,
        })
    }
    pub(super) fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(6)?;
        w.i32(self.player_id)?;
        w.u32(self.cached_num_revealed_hexes)?;
        w.boolean(self.cached_old)?;
        w.list(&self.revealed, |w, v| w.u8(*v))?;
        w.list(&self.layers, |w, l| {
            if l.values.len() != l.count as usize {
                return Err(Error::LayerCountMismatch);
            }
            w.u32(l.type_hash)?;
            w.u32(l.index)?;
            w.u32(l.count)?;
            w.u32(l.grid_width)?;
            w.list(&l.values, |w, v| w.i16(*v))
        })?;
        for values in [
            &self.outgoing_visibility,
            &self.outgoing_visibility_blockers,
        ] {
            w.list(values, |w, v| {
                w.i32(6)?;
                w.i32(v.player_id)?;
                w.u32(v.count)
            })?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerVisibility {
    pub player: i32,
    pub visibility: Visibility,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnitReference {
    /// Raw first ComponentID wire word: signed owner low16, component type high16.
    /// Retained for wire compatibility; this is not a plain PlayerTypes value.
    pub owner: i32,
    pub id: u32,
}
impl UnitReference {
    pub fn component_id(self) -> crate::unit::ComponentId {
        crate::unit::ComponentId {
            owner: self.owner as i16,
            component_type: (self.owner as u32 >> 16) as i16,
            id: self.id,
        }
    }
    pub fn player(self) -> i32 {
        i32::from(self.owner as i16)
    }
    pub fn from_component_id(id: crate::unit::ComponentId) -> Self {
        Self {
            owner: (u32::from(id.owner as u16) | (u32::from(id.component_type as u16) << 16))
                as i32,
            id: id.id,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitCell {
    pub plot_index: u32,
    pub units: Vec<UnitReference>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitLayer {
    pub layer_id: u32,
    pub maximum: u32,
    pub cells: Vec<UnitCell>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitContainer {
    pub width: u32,
    pub height: u32,
    pub layers: Vec<UnitLayer>,
}
impl UnitContainer {
    pub(super) fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(1)?;
        let width = r.u32()?;
        let height = r.u32()?;
        let layers = r.list(16, |r| {
            let layer_id = r.u32()?;
            r.version(1)?;
            let count = r.u32()? as usize;
            let maximum = r.u32()?;
            let mut cells = r.vector::<UnitCell>(count, 8)?;
            for _ in 0..count {
                cells.push(UnitCell {
                    plot_index: r.u32()?,
                    units: r.list(8, |r| {
                        Ok(UnitReference {
                            owner: r.i32()?,
                            id: r.u32()?,
                        })
                    })?,
                });
            }
            Ok(UnitLayer {
                layer_id,
                maximum,
                cells,
            })
        })?;
        Ok(Self {
            width,
            height,
            layers,
        })
    }
    pub(super) fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(1)?;
        w.u32(self.width)?;
        w.u32(self.height)?;
        w.list(&self.layers, |w, l| {
            w.u32(l.layer_id)?;
            w.i32(1)?;
            w.charge::<UnitCell>(l.cells.len())?;
            w.u32(u32::try_from(l.cells.len()).map_err(|_| Error::ResourceLimit)?)?;
            w.u32(l.maximum)?;
            for cell in &l.cells {
                w.u32(cell.plot_index)?;
                w.list(&cell.units, |w, u| {
                    w.i32(u.owner)?;
                    w.u32(u.id)
                })?;
            }
            Ok(())
        })
    }
}
pub(super) fn read_hashes(r: &mut Reader<'_>) -> Result<Vec<DefinitionValue>, Error> {
    hashes_read(r)
}
pub(super) fn write_hashes(w: &mut Writer, v: &[DefinitionValue]) -> Result<(), Error> {
    hashes_write(w, v)
}
