//! Current Linux Player.Heroes and transitive discovery/claim saved wire only.
//! Decisions remain external; native allocation, remapping and archive effects are not run.
use crate::unit::Limits;
pub const VERSION: u32 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
    NoncanonicalBool(u8),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player.Heroes wire: {self:?}")
    }
}
impl std::error::Error for Error {}
/// Ordered hash/value entries; hashes are saved definition identities, not ordinals.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredHero {
    pub hero_class_hash: u32,
    pub discovered: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveryWeight {
    pub hero_class_hash: u32,
    pub weight: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Claim {
    pub hero_class_hash: u32,
    pub claim_era_hash: u32,
    pub claim_turn: i32,
    pub last_appearance_era_hash: u32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Heroes {
    pub discovered: Vec<DiscoveredHero>,
    pub goody_hut_discovery_chance: i32,
    pub player_influence_chances: Vec<i32>,
    pub player_suzerain_count: Vec<i32>,
    pub discovery_order: Vec<DiscoveryWeight>,
    pub claims: Vec<Claim>,
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
    fn heroes(&mut self) -> Result<Heroes, Error> {
        self.version(VERSION)?;
        Ok(Heroes {
            discovered: self.list(5, |r| {
                Ok(DiscoveredHero {
                    hero_class_hash: r.u32()?,
                    discovered: r.bool()?,
                })
            })?,
            goody_hut_discovery_chance: self.i32()?,
            player_influence_chances: self.list(4, Self::i32)?,
            player_suzerain_count: self.list(4, Self::i32)?,
            discovery_order: self.list(12, |r| {
                r.version(VERSION)?;
                Ok(DiscoveryWeight {
                    hero_class_hash: r.u32()?,
                    weight: r.i32()?,
                })
            })?,
            claims: self.list(20, |r| {
                r.version(VERSION)?;
                Ok(Claim {
                    hero_class_hash: r.u32()?,
                    claim_era_hash: r.u32()?,
                    claim_turn: r.i32()?,
                    last_appearance_era_hash: r.u32()?,
                })
            })?,
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
    fn heroes(&mut self, h: &Heroes) -> Result<(), Error> {
        self.u32(VERSION)?;
        self.count::<DiscoveredHero>(h.discovered.len())?;
        for d in &h.discovered {
            self.u32(d.hero_class_hash)?;
            self.bool(d.discovered)?;
        }
        self.i32(h.goody_hut_discovery_chance)?;
        self.count::<i32>(h.player_influence_chances.len())?;
        for &v in &h.player_influence_chances {
            self.i32(v)?;
        }
        self.count::<i32>(h.player_suzerain_count.len())?;
        for &v in &h.player_suzerain_count {
            self.i32(v)?;
        }
        self.count::<DiscoveryWeight>(h.discovery_order.len())?;
        for d in &h.discovery_order {
            self.u32(VERSION)?;
            self.u32(d.hero_class_hash)?;
            self.i32(d.weight)?;
        }
        self.count::<Claim>(h.claims.len())?;
        for c in &h.claims {
            self.u32(VERSION)?;
            self.u32(c.hero_class_hash)?;
            self.u32(c.claim_era_hash)?;
            self.i32(c.claim_turn)?;
            self.u32(c.last_appearance_era_hash)?;
        }
        Ok(())
    }
}
impl Heroes {
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
        r.heroes()?;
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
        let value = r.heroes()?;
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
        w.heroes(self)?;
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
        w.heroes(self)?;
        Ok((w.data, w.items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn value() -> Heroes {
        Heroes {
            discovered: vec![
                DiscoveredHero {
                    hero_class_hash: u32::MAX,
                    discovered: true,
                },
                DiscoveredHero {
                    hero_class_hash: 0,
                    discovered: false,
                },
            ],
            goody_hut_discovery_chance: i32::MIN,
            player_influence_chances: vec![i32::MIN, -1, i32::MAX],
            player_suzerain_count: vec![-123, 0],
            discovery_order: vec![DiscoveryWeight {
                hero_class_hash: 0x81234567,
                weight: i32::MAX,
            }],
            claims: vec![Claim {
                hero_class_hash: 0,
                claim_era_hash: u32::MAX,
                claim_turn: -1,
                last_appearance_era_hash: 0x87654321,
            }],
        }
    }
    #[test]
    fn roundtrip_exact_caps_prefix_and_all_truncations() {
        let v = value();
        let lim = Limits {
            max_bytes: 1024,
            max_items: 9,
        };
        let b = v.encode(lim).unwrap();
        let exact = Limits {
            max_bytes: b.len(),
            max_items: 9,
        };
        assert_eq!(
            Heroes::decode_prefix_with_items(&b, exact).unwrap(),
            (v.clone(), b.len(), 9)
        );
        assert_eq!(v.encode_with_items(exact).unwrap(), (b.clone(), 9));
        for n in 0..b.len() {
            assert!(Heroes::decode(&b[..n], exact).is_err());
        }
        for cap in [
            Limits {
                max_bytes: b.len() - 1,
                ..exact
            },
            Limits {
                max_items: 8,
                ..exact
            },
        ] {
            assert_eq!(Heroes::decode(&b, cap), Err(Error::ResourceLimit));
            assert_eq!(v.encode(cap), Err(Error::ResourceLimit));
        }
        let mut suffix = b.clone();
        suffix.push(42);
        assert_eq!(Heroes::decode(&suffix, exact), Err(Error::Trailing));
        assert_eq!(Heroes::decode_prefix(&suffix, exact).unwrap(), (v, b.len()));
    }
    #[test]
    fn hostile_counts_versions_and_boolean() {
        let lim = Limits {
            max_bytes: 1024,
            max_items: 1024,
        };
        let b = value().encode(lim).unwrap();
        for offset in [4, 22, 38, 50, 66] {
            let mut bad = b.clone();
            bad[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert_eq!(Heroes::decode(&bad, lim), Err(Error::ResourceLimit));
        }
        for offset in [0, 54, 70] {
            let mut bad = b.clone();
            bad[offset..offset + 4].copy_from_slice(&5u32.to_le_bytes());
            assert_eq!(
                Heroes::decode(&bad, lim),
                Err(Error::Version {
                    expected: 6,
                    actual: 5
                })
            );
        }
        let mut bad = b;
        bad[12] = 2;
        assert_eq!(Heroes::decode(&bad, lim), Err(Error::NoncanonicalBool(2)));
    }
    #[test]
    fn empty_vectors_cost_no_items() {
        let mut b = vec![0u8; 28];
        b[..4].copy_from_slice(&VERSION.to_le_bytes());
        let lim = Limits {
            max_bytes: 28,
            max_items: 0,
        };
        assert_eq!(
            Heroes::decode(&b, lim)
                .unwrap()
                .encode_with_items(lim)
                .unwrap(),
            (b, 0)
        );
    }
}
