//! Linux Player38/47 inline saved tail. Native remaps/listeners and gameplay do not run.
use crate::unit::variants::{DecodeBudget, Limits as VariantLimits, TypedVariantMap, VariantError};
use crate::unit::{HashValue, Limits};
pub const PLAYER_VERSION: u32 = 47;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Context {
    pub player_version: u32,
    pub was_ever_alive: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
    NoncanonicalBool(u8),
    ContextMismatch,
    UnsupportedLegacyWrite,
    Variant(VariantError),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player tail wire: {self:?}")
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChopFeatureBehavior {
    /// The writer emits the native FeatureTypes integer directly, unlike its reader.
    pub feature_wire: i32,
    pub ban: bool,
    pub yield_type: i32,
    pub amount: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdditionalPillaging {
    pub plunder_type: i32,
    pub improvement_hash: u32,
    pub district_hash: u32,
    pub amount: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AliveTail {
    pub agendas: Vec<u32>,
    pub agenda_visibilities: Vec<i32>,
    /// None before v40: no wire payload; prior initialized native value is unknown.
    pub foreign_continent_city_unit_class_hashes: Option<Vec<u32>>,
    /// None before v40: no wire payload; prior initialized native value is unknown.
    pub district_naval_unit_hashes: Option<Vec<u32>>,
    pub improvement_hashes: Vec<u32>,
    pub special_allowed_projects: Vec<HashValue<i32>>,
    pub chop_behaviors: Vec<ChopFeatureBehavior>,
    pub additional_pillaging: Vec<AdditionalPillaging>,
    pub co2_generation_reductions: Vec<i32>,
}
impl Default for AliveTail {
    fn default() -> Self {
        Self {
            agendas: Vec::new(),
            agenda_visibilities: Vec::new(),
            foreign_continent_city_unit_class_hashes: Some(Vec::new()),
            district_naval_unit_hashes: Some(Vec::new()),
            improvement_hashes: Vec::new(),
            special_allowed_projects: Vec::new(),
            chop_behaviors: Vec::new(),
            additional_pillaging: Vec::new(),
            co2_generation_reductions: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tail {
    /// Context metadata; no extra word is serialized here.
    pub player_version: u32,
    /// Present exactly when the enclosing Player header says m_bWasEverAlive.
    pub alive: Option<AliveTail>,
    pub properties: TypedVariantMap,
    /// Paired table since v46; empty at v38 means no serialized pairs, not an empty native map.
    pub improvement_replacements: Vec<HashValue<u32>>,
    /// Older listed improvement hashes. Native Read maps each listed improvement to GOODY_HUT;
    /// that definition lookup/assignment is deliberately not executed here.
    pub legacy_improvement_replacement_hashes: Option<Vec<u32>>,
    pub random_event_hashes: Vec<u32>,
    pub terrain_work_impassable: Vec<i32>,
    pub random_event_no_unit_damage: Vec<i32>,
    pub random_event_modified_damage_opposing_player: Vec<i32>,
}
impl Default for Tail {
    fn default() -> Self {
        Self {
            player_version: 47,
            alive: None,
            properties: TypedVariantMap::default(),
            improvement_replacements: Vec::new(),
            legacy_improvement_replacement_hashes: None,
            random_event_hashes: Vec::new(),
            terrain_work_impassable: Vec::new(),
            random_event_no_unit_damage: Vec::new(),
            random_event_modified_damage_opposing_player: Vec::new(),
        }
    }
}

fn context(c: Context) -> Result<(), Error> {
    if !matches!(c.player_version, 38 | PLAYER_VERSION) {
        return Err(Error::Version {
            expected: PLAYER_VERSION,
            actual: c.player_version,
        });
    }
    Ok(())
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
    fn list<T>(
        &mut self,
        min: usize,
        read: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error> {
        let raw = self.u32()?;
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
    fn tail(&mut self, c: Context) -> Result<Tail, Error> {
        let alive = if c.was_ever_alive {
            Some(AliveTail {
                agendas: self.list(4, Self::u32)?,
                agenda_visibilities: self.list(4, Self::i32)?,
                foreign_continent_city_unit_class_hashes: if c.player_version == 47 {
                    Some(self.list(4, Self::u32)?)
                } else {
                    None
                },
                district_naval_unit_hashes: if c.player_version == 47 {
                    Some(self.list(4, Self::u32)?)
                } else {
                    None
                },
                improvement_hashes: self.list(4, Self::u32)?,
                special_allowed_projects: self.list(8, |r| {
                    Ok(HashValue {
                        definition_hash: r.u32()?,
                        value: r.i32()?,
                    })
                })?,
                chop_behaviors: if c.player_version == 47 {
                    self.list(13, |r| {
                        Ok(ChopFeatureBehavior {
                            feature_wire: r.i32()?,
                            ban: r.bool()?,
                            yield_type: r.i32()?,
                            amount: r.i32()?,
                        })
                    })?
                } else {
                    Vec::new()
                },
                additional_pillaging: if c.player_version == 47 {
                    self.list(16, |r| {
                        Ok(AdditionalPillaging {
                            plunder_type: r.i32()?,
                            improvement_hash: r.u32()?,
                            district_hash: r.u32()?,
                            amount: r.i32()?,
                        })
                    })?
                } else {
                    Vec::new()
                },
                co2_generation_reductions: if c.player_version == 47 {
                    self.list(4, Self::i32)?
                } else if self.collect {
                    vec![0; 4]
                } else {
                    Vec::new()
                },
            })
        } else {
            None
        };
        let mut budget = DecodeBudget::new(VariantLimits {
            max_bytes: self.limits.max_bytes - self.pos,
            max_items: self.limits.max_items - self.items,
            max_depth: 64,
        });
        let (properties, n) = TypedVariantMap::decode_prefix(&self.data[self.pos..], &mut budget)
            .map_err(Error::Variant)?;
        self.items = self
            .items
            .checked_add(budget.items_used())
            .ok_or(Error::ResourceLimit)?;
        self.bytes(n)?;
        Ok(Tail {
            player_version: c.player_version,
            alive,
            properties,
            improvement_replacements: if c.player_version == 47 {
                self.list(8, |r| {
                    Ok(HashValue {
                        definition_hash: r.u32()?,
                        value: r.u32()?,
                    })
                })?
            } else {
                Vec::new()
            },
            legacy_improvement_replacement_hashes: if c.player_version == 38 {
                Some(self.list(4, Self::u32)?)
            } else {
                None
            },
            random_event_hashes: self.list(4, Self::u32)?,
            terrain_work_impassable: self.list(4, Self::i32)?,
            random_event_no_unit_damage: self.list(4, Self::i32)?,
            random_event_modified_damage_opposing_player: self.list(4, Self::i32)?,
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
    fn list<T>(
        &mut self,
        v: &[T],
        mut f: impl FnMut(&mut Self, &T) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.count::<T>(v.len())?;
        for x in v {
            f(self, x)?;
        }
        Ok(())
    }
    fn tail(&mut self, t: &Tail, c: Context) -> Result<(), Error> {
        if t.player_version != c.player_version
            || t.legacy_improvement_replacement_hashes.is_some()
            || t.alive.is_some() != c.was_ever_alive
        {
            return Err(Error::ContextMismatch);
        }
        if let Some(a) = &t.alive {
            self.list(&a.agendas, |w, x| w.u32(*x))?;
            self.list(&a.agenda_visibilities, |w, x| w.i32(*x))?;
            self.list(
                a.foreign_continent_city_unit_class_hashes
                    .as_ref()
                    .ok_or(Error::ContextMismatch)?,
                |w, x| w.u32(*x),
            )?;
            self.list(
                a.district_naval_unit_hashes
                    .as_ref()
                    .ok_or(Error::ContextMismatch)?,
                |w, x| w.u32(*x),
            )?;
            self.list(&a.improvement_hashes, |w, x| w.u32(*x))?;
            self.list(&a.special_allowed_projects, |w, x| {
                w.u32(x.definition_hash)?;
                w.i32(x.value)
            })?;
            self.list(&a.chop_behaviors, |w, x| {
                w.i32(x.feature_wire)?;
                w.bool(x.ban)?;
                w.i32(x.yield_type)?;
                w.i32(x.amount)
            })?;
            self.list(&a.additional_pillaging, |w, x| {
                w.i32(x.plunder_type)?;
                w.u32(x.improvement_hash)?;
                w.u32(x.district_hash)?;
                w.i32(x.amount)
            })?;
            self.list(&a.co2_generation_reductions, |w, x| w.i32(*x))?;
        }
        let (b, n) = t
            .properties
            .encode(VariantLimits {
                max_bytes: self.limits.max_bytes - self.pos,
                max_items: self.limits.max_items - self.items,
                max_depth: 64,
            })
            .map_err(Error::Variant)?;
        self.items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        self.bytes(&b)?;
        self.list(&t.improvement_replacements, |w, x| {
            w.u32(x.definition_hash)?;
            w.u32(x.value)
        })?;
        self.list(&t.random_event_hashes, |w, x| w.u32(*x))?;
        self.list(&t.terrain_work_impassable, |w, x| w.i32(*x))?;
        self.list(&t.random_event_no_unit_damage, |w, x| w.i32(*x))?;
        self.list(&t.random_event_modified_damage_opposing_player, |w, x| {
            w.i32(*x)
        })?;
        Ok(())
    }
}
impl Tail {
    pub fn decode(data: &[u8], c: Context, limits: Limits) -> Result<Self, Error> {
        let (v, n, _) = Self::decode_prefix_with_items(data, c, limits)?;
        if n != data.len() {
            return Err(Error::Trailing);
        }
        Ok(v)
    }
    pub fn decode_prefix(data: &[u8], c: Context, limits: Limits) -> Result<(Self, usize), Error> {
        Self::decode_prefix_with_items(data, c, limits).map(|(v, n, _)| (v, n))
    }
    pub fn decode_prefix_with_items(
        data: &[u8],
        c: Context,
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        context(c)?;
        let mut r = Reader {
            data,
            pos: 0,
            items: 0,
            storage: 0,
            limits,
            collect: false,
        };
        r.tail(c)?;
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
        let value = r.tail(c)?;
        Ok((value, n, items))
    }
    pub fn encode(&self, c: Context, limits: Limits) -> Result<Vec<u8>, Error> {
        self.encode_with_items(c, limits).map(|(b, _)| b)
    }
    pub fn encode_with_items(&self, c: Context, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        context(c)?;
        if c.player_version != PLAYER_VERSION {
            return Err(Error::UnsupportedLegacyWrite);
        }
        let mut w = Writer {
            data: Vec::new(),
            pos: 0,
            items: 0,
            storage: 0,
            limits,
            collect: false,
        };
        w.tail(self, c)?;
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
        w.tail(self, c)?;
        Ok((w.data, w.items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn context(alive: bool) -> Context {
        Context {
            player_version: 47,
            was_ever_alive: alive,
        }
    }
    fn sample(alive: bool) -> Tail {
        Tail {
            alive: alive.then(|| AliveTail {
                agendas: vec![u32::MAX, 0x81234567, 0x81234567],
                agenda_visibilities: vec![-1, i32::MIN, i32::MAX],
                foreign_continent_city_unit_class_hashes: Some(vec![u32::MAX]),
                district_naval_unit_hashes: Some(vec![0]),
                improvement_hashes: vec![0x87654321],
                special_allowed_projects: vec![
                    HashValue {
                        definition_hash: 7,
                        value: i32::MIN,
                    },
                    HashValue {
                        definition_hash: 7,
                        value: -1,
                    },
                ],
                chop_behaviors: vec![ChopFeatureBehavior {
                    feature_wire: -123,
                    ban: true,
                    yield_type: i32::MAX,
                    amount: i32::MAX,
                }],
                additional_pillaging: vec![AdditionalPillaging {
                    plunder_type: i32::MIN,
                    improvement_hash: u32::MAX,
                    district_hash: 9,
                    amount: -1,
                }],
                co2_generation_reductions: vec![i32::MIN, -1, 0, i32::MAX],
            }),
            improvement_replacements: vec![
                HashValue {
                    definition_hash: 5,
                    value: u32::MAX,
                },
                HashValue {
                    definition_hash: 5,
                    value: 0,
                },
            ],
            random_event_hashes: vec![u32::MAX, 1],
            terrain_work_impassable: vec![-1],
            random_event_no_unit_damage: vec![i32::MIN],
            random_event_modified_damage_opposing_player: vec![i32::MAX],
            ..Tail::default()
        }
    }
    #[test]
    fn legacy38_omissions_defaults_and_legacy_replacement_keys() {
        fn words(raw: &mut Vec<u8>, xs: &[u32]) {
            for v in xs {
                raw.extend_from_slice(&v.to_le_bytes());
            }
        }
        for alive in [false, true] {
            let mut raw = Vec::new();
            if alive {
                words(
                    &mut raw,
                    &[
                        2,
                        u32::MAX,
                        0x80000001,
                        1,
                        i32::MIN as u32,
                        1,
                        0x12345678,
                        1,
                        u32::MAX,
                        (-257i32) as u32,
                    ],
                );
            }
            words(
                &mut raw,
                &[
                    0,
                    2,
                    0x80000001,
                    u32::MAX,
                    1,
                    0x87654321,
                    1,
                    i32::MIN as u32,
                    1,
                    u32::MAX,
                    1,
                    i32::MAX as u32,
                ],
            );
            let (bytes, items) = if alive { (88, 11) } else { (48, 6) };
            assert_eq!(raw.len(), bytes);
            let c = Context {
                player_version: 38,
                was_ever_alive: alive,
            };
            let cap = Limits {
                max_bytes: bytes,
                max_items: items,
            };
            let (v, n, i) = Tail::decode_prefix_with_items(&raw, c, cap).unwrap();
            assert_eq!((n, i), (bytes, items));
            assert_eq!(v.player_version, 38);
            assert_eq!(
                v.legacy_improvement_replacement_hashes,
                Some(vec![0x80000001, u32::MAX])
            );
            assert!(v.improvement_replacements.is_empty());
            assert_eq!(v.random_event_hashes, [0x87654321]);
            assert_eq!(v.terrain_work_impassable, [i32::MIN]);
            assert_eq!(v.random_event_no_unit_damage, [-1]);
            assert_eq!(v.random_event_modified_damage_opposing_player, [i32::MAX]);
            if let Some(a) = &v.alive {
                assert_eq!(a.foreign_continent_city_unit_class_hashes, None);
                assert_eq!(a.district_naval_unit_hashes, None);
                assert!(a.chop_behaviors.is_empty());
                assert!(a.additional_pillaging.is_empty());
                assert_eq!(a.co2_generation_reductions, [0; 4]);
                assert_eq!(a.special_allowed_projects[0].value, -257);
            }
            assert_eq!(v.encode(c, cap), Err(Error::UnsupportedLegacyWrite));
            assert_eq!(v.encode(context(alive), cap), Err(Error::ContextMismatch));
            for n in 0..raw.len() {
                assert!(Tail::decode(&raw[..n], c, cap).is_err());
            }
            for short in [
                Limits {
                    max_bytes: bytes - 1,
                    ..cap
                },
                Limits {
                    max_items: items - 1,
                    ..cap
                },
            ] {
                assert!(Tail::decode(&raw, c, short).is_err());
            }
            raw.extend_from_slice(&0xdeadbeefu32.to_le_bytes());
            assert_eq!(
                Tail::decode_prefix_with_items(&raw, c, cap),
                Ok((v, bytes, items))
            );
            assert_eq!(Tail::decode(&raw, c, cap), Err(Error::Trailing));
        }
        let mut current = sample(true);
        current.alive.as_mut().unwrap().district_naval_unit_hashes = None;
        assert_eq!(
            current.encode(context(true), Limits::default()),
            Err(Error::ContextMismatch)
        );
    }
    #[test]
    fn both_branches_roundtrip_caps_prefix_and_every_truncation() {
        for alive in [false, true] {
            let v = sample(alive);
            let c = context(alive);
            let (b, items) = v.encode_with_items(c, Limits::default()).unwrap();
            let exact = Limits {
                max_bytes: b.len(),
                max_items: items,
            };
            assert_eq!(
                Tail::decode_prefix_with_items(&b, c, exact).unwrap(),
                (v.clone(), b.len(), items)
            );
            assert_eq!(v.encode(c, exact).unwrap(), b);
            for n in 0..b.len() {
                assert!(Tail::decode(&b[..n], c, exact).is_err());
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
                assert!(Tail::decode(&b, c, lim).is_err());
                assert!(v.encode(c, lim).is_err());
            }
            let mut suffix = b.clone();
            suffix.push(42);
            assert_eq!(Tail::decode(&suffix, c, exact), Err(Error::Trailing));
            assert_eq!(
                Tail::decode_prefix(&suffix, c, exact).unwrap(),
                (v.clone(), b.len())
            );
            assert_eq!(
                v.encode(context(!alive), exact),
                Err(Error::ContextMismatch)
            );
        }
    }
    #[test]
    fn hostile_counts_versions_and_chop_bool() {
        let c = context(true);
        let v = sample(true);
        let b = v.encode(c, Limits::default()).unwrap();
        let mut offset = 0;
        for width in [4, 4, 4, 4, 4, 8, 13, 16, 4] {
            let count = u32::from_le_bytes(b[offset..offset + 4].try_into().unwrap()) as usize;
            let mut bad = b.clone();
            bad[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert!(Tail::decode(
                &bad,
                c,
                Limits {
                    max_bytes: b.len(),
                    max_items: 1000
                }
            )
            .is_err());
            if width == 13 {
                let mut bad = b.clone();
                bad[offset + 8] = 2;
                assert_eq!(
                    Tail::decode(&bad, c, Limits::default()),
                    Err(Error::NoncanonicalBool(2))
                );
            }
            offset += 4 + width * count;
        }
        for version in [0, 46, 48, u32::MAX] {
            assert_eq!(
                Tail::decode(
                    &b,
                    Context {
                        player_version: version,
                        ..c
                    },
                    Limits::default()
                ),
                Err(Error::Version {
                    expected: 47,
                    actual: version
                })
            );
        }
    }
    #[test]
    fn nested_properties_share_cumulative_budget() {
        let wire = [
            1, 0, 0, 128, 1, 0, 0, 0, 7, 0, 0, 0, 2, 0, b'n', b'm', 5, 0, 0, 0, 3, 0, 0, 33, 1, 0,
            0, 0, b'x', b'y', 0,
        ];
        let mut budget = DecodeBudget::new(VariantLimits::default());
        let (properties, n) = TypedVariantMap::decode_prefix(&wire, &mut budget).unwrap();
        assert_eq!(n, wire.len());
        for alive in [false, true] {
            let mut value = sample(alive);
            value.properties = properties.clone();
            let c = context(alive);
            let (bytes, items) = value.encode_with_items(c, Limits::default()).unwrap();
            let lim = Limits {
                max_bytes: bytes.len(),
                max_items: items,
            };
            assert_eq!(
                Tail::decode_prefix_with_items(&bytes, c, lim).unwrap(),
                (value.clone(), bytes.len(), items)
            );
            for cap in [
                Limits {
                    max_bytes: bytes.len() - 1,
                    ..lim
                },
                Limits {
                    max_items: items - 1,
                    ..lim
                },
            ] {
                assert!(Tail::decode(&bytes, c, cap).is_err());
                assert!(value.encode(c, cap).is_err());
            }
            value.properties.entries[0].value.modifier_bits = 1;
            assert!(matches!(
                value.encode(c, Limits::default()),
                Err(Error::Variant(_))
            ));
        }
    }
    #[test]
    fn empty_branch_size_and_zero_items() {
        for alive in [false, true] {
            let v = Tail {
                alive: alive.then(AliveTail::default),
                ..Tail::default()
            };
            let size = if alive { 60 } else { 24 };
            let lim = Limits {
                max_bytes: size,
                max_items: 0,
            };
            let b = v.encode(context(alive), lim).unwrap();
            assert_eq!(b, vec![0; size]);
            assert_eq!(Tail::decode(&b, context(alive), lim).unwrap(), v);
        }
    }
}
