//! Linux XP2 InfluenceMap v1/v2 saved wire only. Values remain unclassified;
//! native initialization, callbacks, gameplay and RNG are not reproduced.
use crate::unit::Limits;

pub const VERSION: u32 = 2;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    UnsupportedLegacyWrite,
    MissingPriorReligiousInfluence,
    MissingRetainedReligiousInfluence,
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
    pub wire_version: u32,
    pub influence: Vec<i16>,
    /// None means no serialized vector at version 1. Its native Read retains
    /// actual prior state; it does not establish a zero or empty vector.
    pub religious_influence: Option<Vec<i16>>,
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
    /// Native version 1 takes an exact self-copy return before archive access.
    /// Preserve the supplied actual prior slice without allocation or mutation.
    pub fn religious_influence_after_read<'a>(
        &'a self,
        prior: Option<&'a [i16]>,
    ) -> Result<&'a [i16], Error> {
        match self.wire_version {
            1 => prior.ok_or(Error::MissingPriorReligiousInfluence),
            VERSION => self
                .religious_influence
                .as_deref()
                .ok_or(Error::MissingRetainedReligiousInfluence),
            actual => Err(Error::Version {
                expected: VERSION,
                actual,
            }),
        }
    }

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
        if actual != 1 && actual != VERSION {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
        }
        let value = Self {
            wire_version: actual,
            influence: r.vector()?,
            religious_influence: if actual == 1 { None } else { Some(r.vector()?) },
        };
        Ok((value, r.pos, r.items))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        self.encode_with_items(limits).map(|(v, _)| v)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        if self.wire_version != VERSION {
            return Err(Error::UnsupportedLegacyWrite);
        }
        let religious = self
            .religious_influence
            .as_ref()
            .ok_or(Error::MissingRetainedReligiousInfluence)?;
        let a = u32::try_from(self.influence.len()).map_err(|_| Error::ResourceLimit)?;
        let b = u32::try_from(religious.len()).map_err(|_| Error::ResourceLimit)?;
        let items = self
            .influence
            .len()
            .checked_add(religious.len())
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
        for (count, values) in [(a, &self.influence), (b, religious)] {
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
            wire_version: VERSION,
            influence: vec![i16::MIN, -1, 0, i16::MAX],
            religious_influence: Some(vec![8, -8]),
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
            wire_version: VERSION,
            influence: vec![],
            religious_influence: Some(vec![]),
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
        for actual in [0u32, 3, u32::MAX] {
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
    #[test]
    fn legacy1_omitted_religious_vector_keeps_prior_explicit() {
        let mut bytes = 1u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&4u32.to_le_bytes());
        let values = vec![i16::MIN, -1, 0, i16::MAX];
        for x in &values {
            bytes.extend_from_slice(&x.to_le_bytes());
        }
        let exact = Limits {
            max_bytes: 16,
            max_items: 4,
        };
        let (v, n, items) = InfluenceMap::decode_prefix_with_items(&bytes, exact).unwrap();
        assert_eq!((n, items), (16, 4));
        assert_eq!(v.wire_version, 1);
        assert_eq!(v.influence, values);
        assert_eq!(v.religious_influence, None);
        assert_eq!(
            v.religious_influence_after_read(None),
            Err(Error::MissingPriorReligiousInfluence)
        );
        let prior = [-8, 7, i16::MIN];
        let retained = v.religious_influence_after_read(Some(&prior)).unwrap();
        assert_eq!(retained, prior);
        assert_eq!(retained.as_ptr(), prior.as_ptr());
        assert_eq!(v.encode(exact), Err(Error::UnsupportedLegacyWrite));
        for n in 0..bytes.len() {
            assert!(InfluenceMap::decode(&bytes[..n], exact).is_err());
        }
        assert_eq!(
            InfluenceMap::decode(
                &bytes,
                Limits {
                    max_bytes: 15,
                    ..exact
                }
            ),
            Err(Error::ResourceLimit)
        );
        assert_eq!(
            InfluenceMap::decode(
                &bytes,
                Limits {
                    max_items: 3,
                    ..exact
                }
            ),
            Err(Error::ResourceLimit)
        );
        let invalid = InfluenceMap {
            wire_version: 2,
            influence: vec![],
            religious_influence: None,
        };
        assert_eq!(
            invalid.encode(Limits::default()),
            Err(Error::MissingRetainedReligiousInfluence)
        );
        assert_eq!(
            invalid.religious_influence_after_read(None),
            Err(Error::MissingRetainedReligiousInfluence)
        );
        let present = InfluenceMap {
            religious_influence: Some(vec![]),
            ..invalid
        };
        assert_eq!(present.religious_influence_after_read(None).unwrap(), &[]);
        assert_eq!(
            InfluenceMap::decode(
                &present.encode(Limits::default()).unwrap(),
                Limits::default()
            )
            .unwrap(),
            present
        );
    }
}
