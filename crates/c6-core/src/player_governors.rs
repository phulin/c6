//! Linux Player.Governors v5/v6 and transitive Governor instance saved wire only.
//! Decisions remain external; native allocation, remapping and archive effects are not run.
use crate::unit::Limits;
pub const VERSION: u32 = 6;
pub const INSTANCE_VERSION: u32 = 16;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    UnsupportedLegacyWrite,
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
    NoncanonicalBool(u8),
    InvalidContainerMetadata,
    NegativeCount,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player.Governors wire: {self:?}")
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CityId {
    pub owner: i16,
    pub component_type: i16,
    pub id: u32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EstablishedCity {
    pub city: CityId,
    pub value: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Promotion {
    pub definition_hash: u32,
    pub earned: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Grievance {
    pub score: i32,
    pub turns: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Governor {
    pub id: i32,
    /// Raw definition hash, never a SQL ordinal.
    pub definition_hash: u32,
    /// Encoding-0 bytes, excluding a terminator; no Unicode normalization.
    pub name: Vec<u8>,
    pub assigned_city: CityId,
    /// Signed PlayerTypes ordinal.
    pub owning_player: i32,
    pub turns_on_site: i32,
    pub cultural_pressure: i32,
    pub domestic_identity_pressure: i32,
    pub foreign_identity_pressure: i32,
    pub domestic_identity_pressure_once_per_city: i32,
    pub foreign_identity_pressure_once_per_city: i32,
    pub established: bool,
    pub neutralized_turns: i32,
    pub neutralized_indefinitely: i32,
    pub identity_per_title: i32,
    pub alliance_points: i32,
    pub ignore_identity_pressure: i32,
    pub feature_no_improvement_appeal_modifier: i32,
    pub prevent_structural_damage: i32,
    pub assign_to_major: bool,
    pub resource_power_provided_change: i32,
    pub faith_purchase_districts: i32,
    pub extra_heal: i32,
    /// Ordered saved map entries, retaining duplicates rather than inserting into a map.
    pub established_cities: Vec<EstablishedCity>,
    pub promotions: Vec<Promotion>,
    pub govern_city_state: bool,
    pub grievances: Vec<Grievance>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Slot {
    pub index: i32,
    pub governor: Governor,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Governors {
    pub wire_version: u32,
    pub governor_points: i32,
    pub capacity: u32,
    pub entries: Vec<Slot>,
    pub current_id_bits: u32,
    pub free_head: u32,
    pub free_tail: u32,
    pub free_links: Vec<u32>,
    pub neutralized_indefinitely: i32,
    pub title_considered: bool,
    pub favored_governor_hash: u32,
    pub diplomatic_favor: i32,
}
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    items: usize,
    storage: usize,
    limits: Limits,
    collect: bool,
}
impl<'a> Reader<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let b = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(b)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, Error> {
        Ok(self.u32()? as i32)
    }
    fn bool(&mut self) -> Result<bool, Error> {
        match self.bytes(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            b => Err(Error::NoncanonicalBool(b)),
        }
    }
    fn version(&mut self, expected: u32) -> Result<(), Error> {
        let actual = self.u32()?;
        if actual != expected {
            return Err(Error::Version { expected, actual });
        }
        Ok(())
    }
    fn list<T>(
        &mut self,
        min: usize,
        signed: bool,
        read: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error> {
        let raw = self.u32()?;
        if signed && raw > i32::MAX as u32 {
            return Err(Error::NegativeCount);
        }
        let n = raw as usize;
        self.list_n(n, min, read)
    }
    fn list_n<T>(
        &mut self,
        n: usize,
        min: usize,
        mut read: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error> {
        self.items = self
            .items
            .checked_add(n)
            .filter(|x| *x <= self.limits.max_items)
            .ok_or(Error::ResourceLimit)?;
        self.storage = self
            .storage
            .checked_add(
                n.checked_mul(std::mem::size_of::<T>())
                    .ok_or(Error::ResourceLimit)?,
            )
            .filter(|x| *x <= isize::MAX as usize)
            .ok_or(Error::ResourceLimit)?;
        let bytes = n.checked_mul(min).ok_or(Error::ResourceLimit)?;
        let end = self.pos.checked_add(bytes).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        if end > self.data.len() {
            return Err(Error::Truncated);
        }
        let mut out = Vec::new();
        if self.collect {
            out.try_reserve_exact(n).map_err(|_| Error::ResourceLimit)?;
        }
        for _ in 0..n {
            let value = read(self)?;
            if self.collect {
                out.push(value)
            }
        }
        Ok(out)
    }
    fn city(&mut self) -> Result<CityId, Error> {
        Ok(CityId {
            owner: i16::from_le_bytes(self.bytes(2)?.try_into().unwrap()),
            component_type: i16::from_le_bytes(self.bytes(2)?.try_into().unwrap()),
            id: self.u32()?,
        })
    }
    fn governor(&mut self) -> Result<Governor, Error> {
        self.version(INSTANCE_VERSION)?;
        Ok(Governor {
            id: self.i32()?,
            definition_hash: self.u32()?,
            name: self.list(1, false, |r| Ok(r.bytes(1)?[0]))?,
            assigned_city: self.city()?,
            owning_player: self.i32()?,
            turns_on_site: self.i32()?,
            cultural_pressure: self.i32()?,
            domestic_identity_pressure: self.i32()?,
            foreign_identity_pressure: self.i32()?,
            domestic_identity_pressure_once_per_city: self.i32()?,
            foreign_identity_pressure_once_per_city: self.i32()?,
            established: self.bool()?,
            neutralized_turns: self.i32()?,
            neutralized_indefinitely: self.i32()?,
            identity_per_title: self.i32()?,
            alliance_points: self.i32()?,
            ignore_identity_pressure: self.i32()?,
            feature_no_improvement_appeal_modifier: self.i32()?,
            prevent_structural_damage: self.i32()?,
            assign_to_major: self.bool()?,
            resource_power_provided_change: self.i32()?,
            faith_purchase_districts: self.i32()?,
            extra_heal: self.i32()?,
            established_cities: self.list(12, true, |r| {
                Ok(EstablishedCity {
                    city: r.city()?,
                    value: r.i32()?,
                })
            })?,
            promotions: self.list(5, false, |r| {
                Ok(Promotion {
                    definition_hash: r.u32()?,
                    earned: r.bool()?,
                })
            })?,
            govern_city_state: self.bool()?,
            grievances: self.list(12, false, |r| {
                r.version(1)?;
                Ok(Grievance {
                    score: r.i32()?,
                    turns: r.i32()?,
                })
            })?,
        })
    }
    fn record(&mut self) -> Result<Governors, Error> {
        let wire_version = self.u32()?;
        if wire_version != 5 && wire_version != VERSION {
            return Err(Error::Version {
                expected: VERSION,
                actual: wire_version,
            });
        }
        let governor_points = self.i32()?;
        self.version(65537)?;
        if self.u32()? != 16 {
            return Err(Error::InvalidContainerMetadata);
        }
        let capacity = self.u32()?;
        let count = self.u32()? as usize;
        if count > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        if count > capacity as usize {
            return Err(Error::InvalidContainerMetadata);
        }
        let entries = self.list_n(count, 111, |r| {
            Ok(Slot {
                index: r.i32()?,
                governor: r.governor()?,
            })
        })?;
        let current_id_bits = self.u32()?;
        let n = self.u32()?;
        let free_head = self.u32()?;
        let free_tail = self.u32()?;
        let free_links = self.list_n(n as usize, 4, |r| r.u32())?;
        Ok(Governors {
            wire_version,
            governor_points,
            capacity,
            entries,
            current_id_bits,
            free_head,
            free_tail,
            free_links,
            neutralized_indefinitely: if wire_version == 5 { 0 } else { self.i32()? },
            title_considered: self.bool()?,
            favored_governor_hash: self.u32()?,
            diplomatic_favor: self.i32()?,
        })
    }
}
struct Writer {
    data: Vec<u8>,
    pos: usize,
    items: usize,
    storage: usize,
    limits: Limits,
    collect: bool,
}
impl Writer {
    fn bytes(&mut self, b: &[u8]) -> Result<(), Error> {
        self.pos = self
            .pos
            .checked_add(b.len())
            .filter(|x| *x <= self.limits.max_bytes && *x <= isize::MAX as usize)
            .ok_or(Error::ResourceLimit)?;
        if self.collect {
            self.data.extend_from_slice(b)
        }
        Ok(())
    }
    fn u32(&mut self, n: u32) -> Result<(), Error> {
        self.bytes(&n.to_le_bytes())
    }
    fn i32(&mut self, n: i32) -> Result<(), Error> {
        self.u32(n as u32)
    }
    fn bool(&mut self, b: bool) -> Result<(), Error> {
        self.bytes(&[u8::from(b)])
    }
    fn charge<T>(&mut self, n: usize) -> Result<(), Error> {
        self.items = self
            .items
            .checked_add(n)
            .filter(|x| *x <= self.limits.max_items)
            .ok_or(Error::ResourceLimit)?;
        self.storage = self
            .storage
            .checked_add(
                n.checked_mul(std::mem::size_of::<T>())
                    .ok_or(Error::ResourceLimit)?,
            )
            .filter(|x| *x <= isize::MAX as usize)
            .ok_or(Error::ResourceLimit)?;
        Ok(())
    }
    fn count<T>(&mut self, n: usize) -> Result<(), Error> {
        self.charge::<T>(n)?;
        self.u32(u32::try_from(n).map_err(|_| Error::ResourceLimit)?)
    }
    fn city(&mut self, c: &CityId) -> Result<(), Error> {
        self.bytes(&c.owner.to_le_bytes())?;
        self.bytes(&c.component_type.to_le_bytes())?;
        self.u32(c.id)
    }
    fn governor(&mut self, g: &Governor) -> Result<(), Error> {
        self.u32(INSTANCE_VERSION)?;
        self.i32(g.id)?;
        self.u32(g.definition_hash)?;
        self.count::<u8>(g.name.len())?;
        self.bytes(&g.name)?;
        self.city(&g.assigned_city)?;
        self.i32(g.owning_player)?;
        self.i32(g.turns_on_site)?;
        self.i32(g.cultural_pressure)?;
        self.i32(g.domestic_identity_pressure)?;
        self.i32(g.foreign_identity_pressure)?;
        self.i32(g.domestic_identity_pressure_once_per_city)?;
        self.i32(g.foreign_identity_pressure_once_per_city)?;
        self.bool(g.established)?;
        self.i32(g.neutralized_turns)?;
        self.i32(g.neutralized_indefinitely)?;
        self.i32(g.identity_per_title)?;
        self.i32(g.alliance_points)?;
        self.i32(g.ignore_identity_pressure)?;
        self.i32(g.feature_no_improvement_appeal_modifier)?;
        self.i32(g.prevent_structural_damage)?;
        self.bool(g.assign_to_major)?;
        self.i32(g.resource_power_provided_change)?;
        self.i32(g.faith_purchase_districts)?;
        self.i32(g.extra_heal)?;
        if g.established_cities.len() > i32::MAX as usize {
            return Err(Error::NegativeCount);
        }
        self.count::<EstablishedCity>(g.established_cities.len())?;
        for x in &g.established_cities {
            self.city(&x.city)?;
            self.i32(x.value)?;
        }
        self.count::<Promotion>(g.promotions.len())?;
        for x in &g.promotions {
            self.u32(x.definition_hash)?;
            self.bool(x.earned)?;
        }
        self.bool(g.govern_city_state)?;
        self.count::<Grievance>(g.grievances.len())?;
        for x in &g.grievances {
            self.u32(1)?;
            self.i32(x.score)?;
            self.i32(x.turns)?;
        }
        Ok(())
    }
    fn record(&mut self, g: &Governors) -> Result<(), Error> {
        if g.wire_version != VERSION {
            return Err(Error::UnsupportedLegacyWrite);
        }
        if g.entries.len() > g.capacity as usize {
            return Err(Error::InvalidContainerMetadata);
        }
        self.u32(VERSION)?;
        self.i32(g.governor_points)?;
        self.u32(65537)?;
        self.u32(16)?;
        self.u32(g.capacity)?;
        self.count::<Slot>(g.entries.len())?;
        for x in &g.entries {
            self.i32(x.index)?;
            self.governor(&x.governor)?;
        }
        self.u32(g.current_id_bits)?;
        self.count::<u32>(g.free_links.len())?;
        self.u32(g.free_head)?;
        self.u32(g.free_tail)?;
        for x in &g.free_links {
            self.u32(*x)?;
        }
        self.i32(g.neutralized_indefinitely)?;
        self.bool(g.title_considered)?;
        self.u32(g.favored_governor_hash)?;
        self.i32(g.diplomatic_favor)
    }
}
impl Governors {
    pub fn decode(data: &[u8], limits: Limits) -> Result<Self, Error> {
        let (v, n, _) = Self::decode_prefix_with_items(data, limits)?;
        if n != data.len() {
            return Err(Error::Trailing);
        }
        Ok(v)
    }
    pub fn decode_prefix(data: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        Self::decode_prefix_with_items(data, limits).map(|(v, n, _)| (v, n))
    }
    pub fn decode_prefix_with_items(
        data: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        let mut r = Reader {
            data,
            pos: 0,
            items: 0,
            storage: 0,
            limits,
            collect: false,
        };
        r.record()?;
        let n = r.pos;
        let items = r.items;
        let mut r = Reader {
            data: &data[..n],
            pos: 0,
            items: 0,
            storage: 0,
            limits,
            collect: true,
        };
        let value = r.record()?;
        Ok((value, n, items))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        self.encode_with_items(limits).map(|(b, _)| b)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        let mut w = Writer {
            data: Vec::new(),
            pos: 0,
            items: 0,
            storage: 0,
            limits,
            collect: false,
        };
        w.record(self)?;
        let size = w.pos;
        let mut data = Vec::new();
        data.try_reserve_exact(size)
            .map_err(|_| Error::ResourceLimit)?;
        let mut w = Writer {
            data,
            pos: 0,
            items: 0,
            storage: 0,
            limits,
            collect: true,
        };
        w.record(self)?;
        Ok((w.data, w.items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Governors {
        Governors {
            wire_version: VERSION,
            governor_points: i32::MIN,
            capacity: 12,
            entries: vec![Slot {
                index: -7,
                governor: Governor {
                    id: i32::MIN,
                    definition_hash: u32::MAX,
                    name: vec![0, 255, 0xc3, 0xa9],
                    assigned_city: CityId {
                        owner: -1,
                        component_type: i16::MIN,
                        id: u32::MAX,
                    },
                    owning_player: i32::MAX,
                    turns_on_site: -10,
                    cultural_pressure: -9,
                    domestic_identity_pressure: -8,
                    foreign_identity_pressure: -7,
                    domestic_identity_pressure_once_per_city: -6,
                    foreign_identity_pressure_once_per_city: -5,
                    established: true,
                    neutralized_turns: 30,
                    neutralized_indefinitely: 31,
                    identity_per_title: 32,
                    alliance_points: 33,
                    ignore_identity_pressure: 34,
                    feature_no_improvement_appeal_modifier: 35,
                    prevent_structural_damage: 36,
                    assign_to_major: false,
                    resource_power_provided_change: -80,
                    faith_purchase_districts: -79,
                    extra_heal: -78,
                    established_cities: vec![
                        EstablishedCity {
                            city: CityId {
                                owner: 4,
                                component_type: 3,
                                id: 0x80000000
                            },
                            value: i32::MIN
                        };
                        2
                    ],
                    promotions: vec![
                        Promotion {
                            definition_hash: 0xdeadbeef,
                            earned: true,
                        },
                        Promotion {
                            definition_hash: 0xdeadbeef,
                            earned: false,
                        },
                    ],
                    govern_city_state: true,
                    grievances: vec![
                        Grievance {
                            score: i32::MIN,
                            turns: i32::MAX
                        };
                        2
                    ],
                },
            }],
            current_id_bits: u32::MAX,
            free_head: 0x80000000,
            free_tail: 12,
            free_links: vec![9, 9, u32::MAX],
            neutralized_indefinitely: i32::MIN,
            title_considered: true,
            favored_governor_hash: 0x80000000,
            diplomatic_favor: i32::MAX,
        }
    }
    #[test]
    fn nonempty_all_fields_caps_and_prefixes() {
        let v = sample();
        let generous = Limits {
            max_bytes: 10000,
            max_items: 10000,
        };
        let (b, items) = v.encode_with_items(generous).unwrap();
        assert_eq!(items, 14);
        let exact = Limits {
            max_bytes: b.len(),
            max_items: items,
        };
        assert_eq!(Governors::decode(&b, exact).unwrap(), v);
        assert_eq!(
            Governors::decode_prefix_with_items(&b, exact).unwrap(),
            (v.clone(), b.len(), items)
        );
        for end in 0..b.len() {
            assert!(Governors::decode(&b[..end], exact).is_err());
        }
        for lim in [
            Limits {
                max_bytes: b.len() - 1,
                ..exact
            },
            Limits {
                max_items: items - 1,
                ..exact
            },
        ] {
            assert_eq!(Governors::decode(&b, lim), Err(Error::ResourceLimit));
            assert_eq!(v.encode(lim), Err(Error::ResourceLimit));
        }
        let mut trailing = b.clone();
        trailing.push(1);
        assert_eq!(Governors::decode(&trailing, exact), Err(Error::Trailing));
        assert_eq!(
            Governors::decode_prefix(&trailing, exact).unwrap(),
            (v, b.len())
        );
    }
    #[test]
    fn malformed_versions_bool_and_counts() {
        let lim = Limits {
            max_bytes: 10000,
            max_items: 10000,
        };
        let b = sample().encode(lim).unwrap();
        for offset in [0, 8, 28] {
            for version in [0u32, 2, u32::MAX] {
                let mut bad = b.clone();
                bad[offset..offset + 4].copy_from_slice(&version.to_le_bytes());
                assert!(matches!(
                    Governors::decode(&bad, lim),
                    Err(Error::Version { .. })
                ));
            }
        }
        let mut bad = b.clone();
        let n = bad.len();
        bad[n - 9] = 2;
        assert_eq!(
            Governors::decode(&bad, lim),
            Err(Error::NoncanonicalBool(2))
        );
        let mut bad = b;
        bad[20..24].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(Governors::decode(&bad, lim), Err(Error::ResourceLimit));
    }
    #[test]
    fn empty_exact_and_metadata() {
        let mut v = sample();
        v.entries.clear();
        v.free_links.clear();
        let lim = Limits {
            max_bytes: 53,
            max_items: 0,
        };
        let b = v.encode(lim).unwrap();
        assert_eq!(b.len(), 53);
        assert_eq!(Governors::decode(&b, lim).unwrap(), v);
        let mut bad = b;
        bad[12..16].copy_from_slice(&8u32.to_le_bytes());
        assert_eq!(
            Governors::decode(&bad, lim),
            Err(Error::InvalidContainerMetadata)
        );
    }
    #[test]
    fn legacy5_neutralized_default_preserves_instance_and_container() {
        let mut expected = sample();
        let (current, items) = expected.encode_with_items(Limits::default()).unwrap();
        // The13-byte current tail begins with the since6 neutralization scalar.
        let split = current.len() - 13;
        let mut bytes = current[..split].to_vec();
        bytes.extend_from_slice(&current[split + 4..]);
        bytes[..4].copy_from_slice(&5u32.to_le_bytes());
        expected.wire_version = 5;
        expected.neutralized_indefinitely = 0;
        let exact = Limits {
            max_bytes: bytes.len(),
            max_items: items,
        };
        assert_eq!(
            Governors::decode_prefix_with_items(&bytes, exact).unwrap(),
            (expected.clone(), bytes.len(), items)
        );
        assert_eq!(Governors::decode(&bytes, exact).unwrap(), expected);
        assert_eq!(expected.encode(exact), Err(Error::UnsupportedLegacyWrite));
        for n in 0..bytes.len() {
            assert!(Governors::decode(&bytes[..n], exact).is_err(), "prefix {n}");
        }
        assert_eq!(
            Governors::decode(
                &bytes,
                Limits {
                    max_items: items - 1,
                    ..exact
                }
            ),
            Err(Error::ResourceLimit)
        );
        assert_eq!(
            Governors::decode(
                &bytes,
                Limits {
                    max_bytes: bytes.len() - 1,
                    ..exact
                }
            ),
            Err(Error::ResourceLimit)
        );
        let mut bad = bytes.clone();
        bad[28..32].copy_from_slice(&15u32.to_le_bytes());
        assert_eq!(
            Governors::decode(&bad, exact),
            Err(Error::Version {
                expected: 16,
                actual: 15
            })
        );
        bad = bytes.clone();
        bad[split] = 2;
        assert_eq!(
            Governors::decode(&bad, exact),
            Err(Error::NoncanonicalBool(2))
        );
        bytes[..4].copy_from_slice(&4u32.to_le_bytes());
        assert_eq!(
            Governors::decode(&bytes, exact),
            Err(Error::Version {
                expected: 6,
                actual: 4
            })
        );
    }
}
