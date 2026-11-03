//! Bounded post-Map Core wire prefix through the first Player's complete Cities.
//! Stops before unframed Diplomacy; this is not a Player/world/gameplay importer.
use crate::unit::{self, Formation, Limits, PlotCoord};
use crate::{city, map, player_components as pc};

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    ResourceLimit,
    InvalidBool(u8),
    Version { expected: u32, actual: u32 },
    InvalidContainer,
    OwnerMismatch { requested: i32, record: i32 },
    Formation(unit::Error),
    Component(pc::Error),
    City(city::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux post-Map prefix: {self:?}")
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Eq, PartialEq)]
pub struct ReligionData {
    pub religion_hash: u32,
    pub founder: i32,
    pub founded_turn: i32,
    pub holy_city: PlotCoord,
    pub custom_name: Vec<u8>,
    pub belief_hashes: Vec<u32>,
    pub building_hashes: Vec<u32>,
    pub unit_hashes: Vec<u32>,
    pub anyone_condemns_favor: i32,
    pub condemn_prohibited: bool,
    pub auto_spreads_to_new_cities: bool,
    pub awards_envoy: bool,
    pub awards_envoy_religious_pressure: i32,
}
#[derive(Debug, Eq, PartialEq)]
pub struct Religion {
    pub minimum_faith_for_next_pantheon: i32,
    pub next_pantheon_founder: i32,
    pub records: Vec<ReligionData>,
}
#[derive(Debug, Eq, PartialEq)]
pub struct PlayerHeader {
    pub owner: i32,
    pub starting_plot: PlotCoord,
    /// Fifteen original ordered i32/enum words; no inferred semantic names.
    pub ordered_scalars: [i32; 15],
    pub ordered_flags: [bool; 6],
    pub handicap_hash: u32,
    pub district_hash: u32,
    pub project_hash: u32,
    pub final_scalar: i32,
}
#[derive(Debug, Eq, PartialEq)]
pub struct Cities {
    pub records: map::Recycled<city::City>,
    pub fresh_water_housing_bonus: bool,
    pub urban_defenses: i32,
    pub home_continent_hash: u32,
    pub settled_continent_hashes: Vec<u32>,
    pub peak_cities: u32,
    pub extra_starting_population_off_home_continent: i32,
    pub population_after_conquest: i32,
    pub banned_building_hashes: Vec<u32>,
    pub banned_district_hashes: Vec<u32>,
    pub banned_district_building_hashes: Vec<u32>,
    pub banned_project_hashes: Vec<u32>,
    pub building_build_overrides: Vec<[u32; 2]>,
}
#[derive(Debug, Eq, PartialEq)]
pub struct FirstPlayer {
    /// Explicit Manager wire ID, retained separately from the embedded ID.
    pub requested_owner: i32,
    pub header: PlayerHeader,
    pub formations: map::Recycled<Formation>,
    pub goody_hut: pc::GoodyHut,
    pub bonuses: pc::Bonuses,
    pub congress: pc::Congress,
    pub cities: Cities,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Boundary {
    /// Manager has no entries; later Game components remain unsupported.
    AfterEmptyPlayerManager,
    /// The first entry is partial. Remaining includes that entry's unfinished tail.
    PlayerDiplomacy,
}
#[derive(Debug, Eq, PartialEq)]
pub struct Prefix {
    pub religion: Religion,
    pub player_count: u32,
    pub first_player: Option<FirstPlayer>,
    pub boundary: Boundary,
    /// No complete Player entry has been consumed at PlayerDiplomacy.
    pub unfinished_player_entries: u32,
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    items: usize,
    limits: Limits,
}
impl<'a> Reader<'a> {
    fn remaining(&self) -> Limits {
        Limits {
            max_bytes: self.limits.max_bytes - self.pos,
            max_items: self.limits.max_items - self.items,
        }
    }
    fn charge(&mut self, bytes: usize, items: usize) -> Result<(), Error> {
        let pos = self.pos.checked_add(bytes).ok_or(Error::ResourceLimit)?;
        let count = self.items.checked_add(items).ok_or(Error::ResourceLimit)?;
        if pos > self.limits.max_bytes || count > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        self.pos = pos;
        self.items = count;
        Ok(())
    }
    fn raw(&mut self, size: usize) -> Result<&'a [u8], Error> {
        let start = self.pos;
        self.charge(size, 0)?;
        self.bytes.get(start..self.pos).ok_or(Error::Truncated)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.raw(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, Error> {
        Ok(self.u32()? as i32)
    }
    fn boolean(&mut self) -> Result<bool, Error> {
        match self.raw(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            v => Err(Error::InvalidBool(v)),
        }
    }
    fn version(&mut self, expected: u32) -> Result<(), Error> {
        let actual = self.u32()?;
        if actual == expected {
            Ok(())
        } else {
            Err(Error::Version { expected, actual })
        }
    }
    fn coord(&mut self) -> Result<PlotCoord, Error> {
        Ok(PlotCoord {
            x: self.i32()?,
            y: self.i32()?,
        })
    }
    fn vec<T>(&mut self, count: usize, min: usize) -> Result<Vec<T>, Error> {
        self.charge(0, count)?;
        if count > self.bytes.len().saturating_sub(self.pos) / min {
            return Err(Error::Truncated);
        }
        if count
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(Error::ResourceLimit)?
            > self.limits.max_bytes
        {
            return Err(Error::ResourceLimit);
        }
        let mut out = Vec::new();
        out.try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        Ok(out)
    }
    fn list<T>(
        &mut self,
        min: usize,
        mut read: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error> {
        let n = self.u32()? as usize;
        let mut out = self.vec(n, min)?;
        for _ in 0..n {
            out.push(read(self)?);
        }
        Ok(out)
    }
    fn hashes(&mut self) -> Result<Vec<u32>, Error> {
        self.list(4, Self::u32)
    }
    fn component<T, E>(
        &mut self,
        read: impl FnOnce(&[u8], Limits) -> Result<(T, usize, usize), E>,
        error: impl FnOnce(E) -> Error,
    ) -> Result<T, Error> {
        let (v, n, i) = read(&self.bytes[self.pos..], self.remaining()).map_err(error)?;
        self.charge(n, i)?;
        Ok(v)
    }
    fn religion_data(&mut self) -> Result<ReligionData, Error> {
        self.version(7)?;
        Ok(ReligionData {
            religion_hash: self.u32()?,
            founder: self.i32()?,
            founded_turn: self.i32()?,
            holy_city: self.coord()?,
            custom_name: self.list(1, |r| Ok(r.raw(1)?[0]))?,
            belief_hashes: self.hashes()?,
            building_hashes: self.hashes()?,
            unit_hashes: self.hashes()?,
            anyone_condemns_favor: self.i32()?,
            condemn_prohibited: self.boolean()?,
            auto_spreads_to_new_cities: self.boolean()?,
            awards_envoy: self.boolean()?,
            awards_envoy_religious_pressure: self.i32()?,
        })
    }
    fn religion(&mut self) -> Result<Religion, Error> {
        self.version(7)?;
        Ok(Religion {
            minimum_faith_for_next_pantheon: self.i32()?,
            next_pantheon_founder: self.i32()?,
            records: self.list(51, Self::religion_data)?,
        })
    }
    fn header(&mut self) -> Result<PlayerHeader, Error> {
        self.version(47)?;
        let owner = self.i32()?;
        let starting_plot = self.coord()?;
        let mut ordered_scalars = [0; 15];
        for v in &mut ordered_scalars {
            *v = self.i32()?;
        }
        let mut ordered_flags = [false; 6];
        for v in &mut ordered_flags {
            *v = self.boolean()?;
        }
        Ok(PlayerHeader {
            owner,
            starting_plot,
            ordered_scalars,
            ordered_flags,
            handicap_hash: self.u32()?,
            district_hash: self.u32()?,
            project_hash: self.u32()?,
            final_scalar: self.i32()?,
        })
    }
    fn formation(&mut self) -> Result<Formation, Error> {
        // Exact current fixed header, counted PathNode1 records, coordinate, IDs.
        // Strict existing codec validates every field after structural sizing.
        let start = self.pos;
        let limits = self.remaining();
        self.raw(36)?;
        let paths = self.u32()? as usize;
        self.charge(0, paths)?;
        self.raw(paths.checked_mul(20).ok_or(Error::ResourceLimit)?)?;
        self.raw(8)?;
        let units = self.u32()? as usize;
        self.charge(0, units)?;
        self.raw(units.checked_mul(4).ok_or(Error::ResourceLimit)?)?;
        Formation::decode(&self.bytes[start..self.pos], limits).map_err(Error::Formation)
    }
    fn recycled<T>(
        &mut self,
        mut read: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<map::Recycled<T>, Error> {
        self.version(65537)?;
        if self.u32()? != 16 {
            return Err(Error::InvalidContainer);
        }
        let capacity = self.u32()?;
        let n = self.u32()? as usize;
        if n > capacity as usize {
            return Err(Error::InvalidContainer);
        }
        let mut entries = self.vec::<map::Slot<T>>(n, 4)?;
        // A sorted auxiliary slot list is bounded by the same item count. Slots
        // themselves retain their native serialized order in `entries`.
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(n)
            .map_err(|_| Error::ResourceLimit)?;
        for _ in 0..n {
            let index = self.i32()?;
            if index < 0 || index as u32 >= capacity {
                return Err(Error::InvalidContainer);
            }
            slots.push(index);
            entries.push(map::Slot {
                index,
                value: read(self)?,
            });
        }
        slots.sort_unstable();
        if slots.windows(2).any(|s| s[0] == s[1]) {
            return Err(Error::InvalidContainer);
        }
        let current_id_bits = self.u32()?;
        let free_count = self.u32()? as usize;
        let free_head = self.u32()?;
        let free_tail = self.u32()?;
        let mut free_links = self.vec(free_count, 4)?;
        for _ in 0..free_count {
            free_links.push(self.u32()?);
        }
        Ok(map::Recycled {
            capacity,
            entries,
            current_id_bits,
            free_head,
            free_tail,
            free_links,
        })
    }
    fn cities(&mut self) -> Result<Cities, Error> {
        self.version(15)?;
        let records =
            self.recycled(|r| r.component(city::City::decode_prefix_with_items, Error::City))?;
        Ok(Cities {
            records,
            fresh_water_housing_bonus: self.boolean()?,
            urban_defenses: self.i32()?,
            home_continent_hash: self.u32()?,
            settled_continent_hashes: self.hashes()?,
            peak_cities: self.u32()?,
            extra_starting_population_off_home_continent: self.i32()?,
            population_after_conquest: self.i32()?,
            banned_building_hashes: self.hashes()?,
            banned_district_hashes: self.hashes()?,
            banned_district_building_hashes: self.hashes()?,
            banned_project_hashes: self.hashes()?,
            building_build_overrides: self.list(8, |r| Ok([r.u32()?, r.u32()?]))?,
        })
    }
}
impl Prefix {
    /// Input begins immediately after the complete Map wire record. Any suffix
    /// is left untouched at `boundary`; counts are not a complete roster claim.
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        let mut r = Reader {
            bytes,
            pos: 0,
            items: 0,
            limits,
        };
        let religion = r.religion()?;
        r.version(1)?;
        let player_count = r.u32()?;
        // Count is resource-gated even though unsupported later entries are not allocated.
        if player_count as usize > r.remaining().max_items {
            return Err(Error::ResourceLimit);
        }
        let first_player = if player_count == 0 {
            None
        } else {
            r.charge(0, 1)?;
            let requested_owner = r.i32()?;
            let header = r.header()?;
            if requested_owner != header.owner {
                return Err(Error::OwnerMismatch {
                    requested: requested_owner,
                    record: header.owner,
                });
            }
            r.version(1)?;
            let formations = r.recycled(Self::read_formation)?;
            let goody_hut =
                r.component(pc::GoodyHut::decode_prefix_with_items, Error::Component)?;
            let bonuses = r.component(pc::Bonuses::decode_prefix_with_items, Error::Component)?;
            let congress = r.component(pc::Congress::decode_prefix_with_items, Error::Component)?;
            let cities = r.cities()?;
            Some(FirstPlayer {
                requested_owner,
                header,
                formations,
                goody_hut,
                bonuses,
                congress,
                cities,
            })
        };
        let boundary = if first_player.is_some() {
            Boundary::PlayerDiplomacy
        } else {
            Boundary::AfterEmptyPlayerManager
        };
        Ok((
            Self {
                religion,
                player_count,
                first_player,
                boundary,
                unfinished_player_entries: player_count,
            },
            r.pos,
            r.items,
        ))
    }
    fn read_formation(r: &mut Reader<'_>) -> Result<Formation, Error> {
        r.formation()
    }
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        Self::decode_prefix_with_items(bytes, limits).map(|(v, n, _)| (v, n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn words(out: &mut Vec<u8>, values: &[u32]) {
        for value in values {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    fn empty_container(out: &mut Vec<u8>) {
        words(out, &[65537, 16, 8, 0, 0x10000, 2, 7, 3, 7, 3]);
    }
    fn carrier() -> Vec<u8> {
        let mut out = Vec::new();
        words(&mut out, &[7, 101, 102, 1, 7, 0xfeed1234, 4, 5, 6, 7, 3]);
        out.extend_from_slice(b"\xff\0x");
        words(
            &mut out,
            &[2, 0xffffffff, 0x80000000, 0, 1, 0x12345678, 0x80000000],
        );
        out.extend_from_slice(&[1, 0, 1]);
        words(&mut out, &[0x7fffffff]);
        words(&mut out, &[1, 2, 64, 47, 64, 1, 2]);
        words(
            &mut out,
            &[
                0x80000000, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 0xffffffff,
            ],
        );
        out.extend_from_slice(&[0, 1, 0, 1, 0, 1]);
        words(&mut out, &[0xdeadbeef, 7, 8, 9, 1]);
        empty_container(&mut out);
        // Goody, Bonuses, Congress, all complete with empty dynamic vectors.
        words(
            &mut out,
            &[
                1, 0, 4, 0, 12, 64, 1, 2, 3, 4, 5, 0, 0, 0, 0, 0, 6, 7, 8, 9, 0,
            ],
        );
        out.push(1);
        words(&mut out, &[10, 15]);
        empty_container(&mut out);
        out.push(1);
        words(
            &mut out,
            &[0x80000000, 0xdeadbeef, 2, 0x12345678, 0xffffffff, 9, 10, 11],
        );
        words(&mut out, &[1, 123, 0, 1, 456, 0, 2, 111, 222, 111, 333]);
        out
    }
    #[test]
    fn typed_empty_cities_tail_prefix_and_aggregate_budgets() {
        let bytes = carrier();
        let mut trailing = bytes.clone();
        trailing.extend_from_slice(b"untouched Diplomacy");
        let (value, n, items) =
            Prefix::decode_prefix_with_items(&trailing, Limits::default()).unwrap();
        assert_eq!(n, bytes.len());
        assert_eq!(value.boundary, Boundary::PlayerDiplomacy);
        assert_eq!(value.unfinished_player_entries, 2);
        assert_eq!(value.religion.records[0].custom_name, b"\xff\0x");
        let p = value.first_player.unwrap();
        assert_eq!(p.requested_owner, 64);
        assert_eq!(p.header.owner, 64);
        assert_eq!(p.header.ordered_scalars[0], i32::MIN);
        assert_eq!(p.formations.free_links, [7, 3]);
        assert!(p.cities.records.entries.is_empty());
        assert_eq!(p.cities.records.free_links, [7, 3]);
        assert_eq!(p.cities.building_build_overrides, [[111, 222], [111, 333]]);
        let exact = Limits {
            max_bytes: n,
            max_items: items,
        };
        assert!(Prefix::decode_prefix_with_items(&trailing, exact).is_ok());
        assert!(Prefix::decode_prefix_with_items(
            &trailing,
            Limits {
                max_bytes: n - 1,
                ..exact
            }
        )
        .is_err());
        assert!(matches!(
            Prefix::decode_prefix_with_items(
                &trailing,
                Limits {
                    max_items: items - 1,
                    ..exact
                }
            ),
            Err(Error::ResourceLimit)
        ));
        for end in 0..n {
            assert!(
                Prefix::decode_prefix(&bytes[..end], Limits::default()).is_err(),
                "end={end}"
            );
        }
    }
    #[test]
    fn empty_manager_stops_and_preserves_suffix() {
        let mut bytes = Vec::new();
        words(&mut bytes, &[7, 1, 2, 0, 1, 0, 99]);
        let (v, n, i) = Prefix::decode_prefix_with_items(
            &bytes,
            Limits {
                max_bytes: 24,
                max_items: 0,
            },
        )
        .unwrap();
        assert_eq!((n, i), (24, 0));
        assert_eq!(v.boundary, Boundary::AfterEmptyPlayerManager);
        assert_eq!(v.unfinished_player_entries, 0);
        assert!(v.first_player.is_none());
    }
    #[test]
    fn rejects_owner_mismatch_and_noncanonical_flags() {
        let mut bytes = carrier();
        // Independently read Religion framing to obtain the Manager cursor.
        let mut r = Reader {
            bytes: &bytes,
            pos: 0,
            items: 0,
            limits: Limits::default(),
        };
        r.religion().unwrap();
        let owner = r.pos + 8;
        bytes[owner..owner + 4].copy_from_slice(&3u32.to_le_bytes());
        assert!(matches!(
            Prefix::decode_prefix(&bytes, Limits::default()),
            Err(Error::OwnerMismatch {
                requested: 3,
                record: 64
            })
        ));
        let mut bytes = carrier();
        bytes[owner + 4 + 4 + 4 + 8 + 60] = 2;
        assert!(matches!(
            Prefix::decode_prefix(&bytes, Limits::default()),
            Err(Error::InvalidBool(2))
        ));
    }
    #[test]
    fn formation_structural_sizing_delegates_validation_and_charges_counts() {
        let mut bytes = Vec::new();
        words(&mut bytes, &[1, 0x10001, 1, 2, 3, 4, 5]);
        bytes.extend_from_slice(&[0, 1, 0, 1, 0, 1, 0, 1]);
        words(
            &mut bytes,
            &[1, 1, 2, 3, 4, 0xffff8000, 5, 6, 2, 0xffffffff, 0x12345678],
        );
        let mut r = Reader {
            bytes: &bytes,
            pos: 0,
            items: 0,
            limits: Limits::default(),
        };
        let value = r.formation().unwrap();
        assert_eq!(r.pos, bytes.len());
        assert_eq!(r.items, 3);
        assert_eq!(value.last_path[0].x, i16::MIN);
        assert_eq!(value.unit_ids, [u32::MAX, 0x12345678]);
        assert_eq!(value.encode(Limits::default()).unwrap(), bytes);
        bytes[28] = 2;
        let mut r = Reader {
            bytes: &bytes,
            pos: 0,
            items: 0,
            limits: Limits::default(),
        };
        assert!(matches!(
            r.formation(),
            Err(Error::Formation(unit::Error::NoncanonicalBool(2)))
        ));
    }
    #[test]
    fn recycled_metadata_slots_and_duplicate_rejection() {
        let mut bytes = Vec::new();
        words(
            &mut bytes,
            &[65537, 16, 3, 2, 2, 100, 0, 101, 0x70000, 1, 1, 1, 1],
        );
        let mut r = Reader {
            bytes: &bytes,
            pos: 0,
            items: 0,
            limits: Limits::default(),
        };
        let v = r.recycled(Reader::u32).unwrap();
        assert_eq!(v.entries[0].index, 2);
        assert_eq!(v.entries[1].index, 0);
        assert_eq!(v.current_id_bits, 0x70000);
        assert_eq!(v.free_links, [1]);
        assert_eq!(r.pos, bytes.len());
        bytes[24..28].copy_from_slice(&2u32.to_le_bytes());
        let mut r = Reader {
            bytes: &bytes,
            pos: 0,
            items: 0,
            limits: Limits::default(),
        };
        assert!(matches!(
            r.recycled(Reader::u32),
            Err(Error::InvalidContainer)
        ));
    }
}
