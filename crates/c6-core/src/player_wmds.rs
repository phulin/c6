//! Current Linux XP2 Player.WMDs saved wire. Weapon decisions remain external;
//! native hash remapping, sync archive effects and gameplay are not performed.
use crate::unit::Limits;

pub const VERSION: u32 = 1;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player.WMDs wire: {self:?}")
    }
}
impl std::error::Error for Error {}

/// One saved definition type hash and signed weapon count; never a SQL ordinal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeaponCount {
    pub definition_hash: u32,
    pub count: i32,
}
/// Complete current saved record, preserving definition order and duplicates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Wmds {
    pub weapons: Vec<WeaponCount>,
}
impl Wmds {
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
        if limits.max_bytes < 4 {
            return Err(Error::ResourceLimit);
        }
        let b = data.get(..4).ok_or(Error::Truncated)?;
        let actual = u32::from_le_bytes(b.try_into().unwrap());
        if actual != VERSION {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
        }
        if limits.max_bytes < 8 {
            return Err(Error::ResourceLimit);
        }
        let b = data.get(4..8).ok_or(Error::Truncated)?;
        // Native count is unsigned. High-bit counts are bounded, never reinterpreted as signed.
        let n = usize::try_from(u32::from_le_bytes(b.try_into().unwrap()))
            .map_err(|_| Error::ResourceLimit)?;
        let bytes = Self::size(n, limits)?;
        let payload = data.get(8..bytes).ok_or(Error::Truncated)?;
        // Preflight both complete wire extent and typed allocation before allocation.
        n.checked_mul(std::mem::size_of::<WeaponCount>())
            .filter(|&b| b <= isize::MAX as usize)
            .ok_or(Error::ResourceLimit)?;
        let mut weapons = Vec::new();
        weapons
            .try_reserve_exact(n)
            .map_err(|_| Error::ResourceLimit)?;
        for pair in payload.chunks_exact(8) {
            weapons.push(WeaponCount {
                definition_hash: u32::from_le_bytes(pair[..4].try_into().unwrap()),
                count: i32::from_le_bytes(pair[4..].try_into().unwrap()),
            });
        }
        Ok((Self { weapons }, bytes, n))
    }
    fn size(n: usize, limits: Limits) -> Result<usize, Error> {
        if n > limits.max_items || u32::try_from(n).is_err() {
            return Err(Error::ResourceLimit);
        }
        let bytes = n
            .checked_mul(8)
            .and_then(|b| b.checked_add(8))
            .ok_or(Error::ResourceLimit)?;
        if bytes > limits.max_bytes || bytes > isize::MAX as usize {
            return Err(Error::ResourceLimit);
        }
        Ok(bytes)
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        self.encode_with_items(limits).map(|(b, _)| b)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        let n = self.weapons.len();
        let bytes = Self::size(n, limits)?;
        let mut out = Vec::new();
        out.try_reserve_exact(bytes)
            .map_err(|_| Error::ResourceLimit)?;
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&(n as u32).to_le_bytes());
        for w in &self.weapons {
            out.extend_from_slice(&w.definition_hash.to_le_bytes());
            out.extend_from_slice(&w.count.to_le_bytes());
        }
        Ok((out, n))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nonempty_order_duplicates_extrema_prefix_and_caps() {
        let v = Wmds {
            weapons: vec![
                WeaponCount {
                    definition_hash: u32::MAX,
                    count: i32::MIN,
                },
                WeaponCount {
                    definition_hash: 0,
                    count: i32::MAX,
                },
                WeaponCount {
                    definition_hash: u32::MAX,
                    count: -1,
                },
            ],
        };
        let exact = Limits {
            max_bytes: 32,
            max_items: 3,
        };
        let (b, items) = v.encode_with_items(exact).unwrap();
        assert_eq!(items, 3);
        assert_eq!(Wmds::decode(&b, exact).unwrap(), v);
        assert_eq!(
            Wmds::decode_prefix_with_items(&b, exact).unwrap(),
            (v.clone(), 32, 3)
        );
        for end in 0..b.len() {
            assert!(Wmds::decode(&b[..end], exact).is_err());
        }
        for cap in [
            Limits {
                max_bytes: 31,
                ..exact
            },
            Limits {
                max_items: 2,
                ..exact
            },
        ] {
            assert_eq!(v.encode(cap), Err(Error::ResourceLimit));
            assert_eq!(Wmds::decode(&b, cap), Err(Error::ResourceLimit));
        }
        let mut tail = b;
        tail.push(255);
        assert_eq!(Wmds::decode(&tail, exact), Err(Error::Trailing));
        assert_eq!(Wmds::decode_prefix(&tail, exact).unwrap(), (v, 32));
    }
    #[test]
    fn empty_versions_unsigned_counts() {
        let exact = Limits {
            max_bytes: 8,
            max_items: 0,
        };
        let v = Wmds { weapons: vec![] };
        let b = v.encode(exact).unwrap();
        assert_eq!(Wmds::decode(&b, exact).unwrap(), v);
        for actual in [0u32, 2, u32::MAX] {
            let mut bad = b.clone();
            bad[..4].copy_from_slice(&actual.to_le_bytes());
            assert_eq!(
                Wmds::decode(&bad, exact),
                Err(Error::Version {
                    expected: VERSION,
                    actual
                })
            );
        }
        for count in [1u32, 0x80000000, u32::MAX] {
            let mut bad = b.clone();
            bad[4..].copy_from_slice(&count.to_le_bytes());
            assert_eq!(Wmds::decode(&bad, exact), Err(Error::ResourceLimit));
        }
    }
}
