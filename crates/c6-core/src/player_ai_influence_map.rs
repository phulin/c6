//! Current Linux XP2 InfluenceMap saved wire only. Values remain unclassified;
//! native initialization, callbacks, gameplay and RNG are not reproduced.
use crate::unit::Limits;

pub const VERSION: u32 = 2;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux AI.InfluenceMap wire: {self:?}")
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InfluenceMap {
    pub influence: Vec<i16>,
    pub religious_influence: Vec<i16>,
}
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    items: usize,
    limits: Limits,
}
impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let end = self.pos.checked_add(N).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let raw = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(raw.try_into().unwrap())
    }
    fn vector(&mut self) -> Result<Vec<i16>, Error> {
        let count = u32::from_le_bytes(self.take()?) as usize;
        self.items = self.items.checked_add(count).ok_or(Error::ResourceLimit)?;
        let end = self
            .pos
            .checked_add(count.checked_mul(2).ok_or(Error::ResourceLimit)?)
            .ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items || end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        if end > self.data.len() {
            return Err(Error::Truncated);
        }
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        for _ in 0..count {
            values.push(i16::from_le_bytes(self.take()?));
        }
        Ok(values)
    }
}
impl InfluenceMap {
    pub fn decode(data: &[u8], limits: Limits) -> Result<Self, Error> {
        let (value, used) = Self::decode_prefix(data, limits)?;
        if used != data.len() {
            return Err(Error::Trailing);
        }
        Ok(value)
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
            limits,
        };
        let actual = u32::from_le_bytes(r.take()?);
        if actual != VERSION {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
        }
        let value = Self {
            influence: r.vector()?,
            religious_influence: r.vector()?,
        };
        Ok((value, r.pos, r.items))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        self.encode_with_items(limits).map(|(v, _)| v)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        let a = u32::try_from(self.influence.len()).map_err(|_| Error::ResourceLimit)?;
        let b = u32::try_from(self.religious_influence.len()).map_err(|_| Error::ResourceLimit)?;
        let items = self
            .influence
            .len()
            .checked_add(self.religious_influence.len())
            .ok_or(Error::ResourceLimit)?;
        let bytes = items
            .checked_mul(2)
            .and_then(|n| n.checked_add(12))
            .ok_or(Error::ResourceLimit)?;
        if items > limits.max_items || bytes > limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let mut out = Vec::new();
        out.try_reserve_exact(bytes)
            .map_err(|_| Error::ResourceLimit)?;
        out.extend_from_slice(&VERSION.to_le_bytes());
        for (count, values) in [(a, &self.influence), (b, &self.religious_influence)] {
            out.extend_from_slice(&count.to_le_bytes());
            for value in values {
                out.extend_from_slice(&value.to_le_bytes());
            }
        }
        Ok((out, items))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_order_and_bounds() {
        let v = InfluenceMap {
            influence: vec![i16::MIN, -1, 0, i16::MAX],
            religious_influence: vec![8, -8],
        };
        let limits = Limits {
            max_bytes: 24,
            max_items: 6,
        };
        let raw = v.encode(limits).unwrap();
        assert_eq!(InfluenceMap::decode(&raw, limits), Ok(v.clone()));
        for end in 0..raw.len() {
            assert!(InfluenceMap::decode(&raw[..end], limits).is_err());
        }
        for cap in [
            Limits {
                max_bytes: 23,
                ..limits
            },
            Limits {
                max_items: 5,
                ..limits
            },
        ] {
            assert_eq!(v.encode(cap), Err(Error::ResourceLimit));
            assert_eq!(InfluenceMap::decode(&raw, cap), Err(Error::ResourceLimit));
        }
        let mut tail = raw.clone();
        tail.extend_from_slice(&[9; 100]);
        assert_eq!(InfluenceMap::decode(&tail, limits), Err(Error::Trailing));
        assert_eq!(
            InfluenceMap::decode_prefix_with_items(&tail, limits),
            Ok((v, 24, 6))
        );
    }
    #[test]
    fn empty_hostile_and_versions() {
        let v = InfluenceMap {
            influence: vec![],
            religious_influence: vec![],
        };
        let limits = Limits {
            max_bytes: 12,
            max_items: 0,
        };
        let raw = v.encode(limits).unwrap();
        assert_eq!(InfluenceMap::decode(&raw, limits), Ok(v));
        for offset in [4, 8] {
            let mut bad = raw.clone();
            bad[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert_eq!(
                InfluenceMap::decode(&bad, limits),
                Err(Error::ResourceLimit)
            );
        }
        for actual in [0u32, 1, 3, u32::MAX] {
            let mut bad = raw.clone();
            bad[..4].copy_from_slice(&actual.to_le_bytes());
            assert_eq!(
                InfluenceMap::decode(&bad, limits),
                Err(Error::Version {
                    expected: 2,
                    actual
                })
            );
        }
    }
}
