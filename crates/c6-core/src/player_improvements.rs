//! Current Linux XP2 Player.Improvements saved wire. Improvement decisions remain external;
//! native sync archive effects and gameplay are not performed.
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
        write!(f, "Linux Player.Improvements wire: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Complete current saved Player component, distinct from Map Plot and City improvements.
/// Indices are ordered signed plot indices, not definition hashes or SQL ordinals.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Improvements {
    pub plot_indices: Vec<i32>,
    /// Independently serialized native member; preserved even if unequal to vector length.
    pub improvement_array_size: u32,
}
impl Improvements {
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
        let payload = data.get(8..bytes - 4).ok_or(Error::Truncated)?;
        let improvement_array_size = u32::from_le_bytes(
            data.get(bytes - 4..bytes)
                .ok_or(Error::Truncated)?
                .try_into()
                .unwrap(),
        );
        // Preflight both complete wire extent and typed allocation before allocation.
        n.checked_mul(std::mem::size_of::<i32>())
            .filter(|&b| b <= isize::MAX as usize)
            .ok_or(Error::ResourceLimit)?;
        let mut plot_indices = Vec::new();
        plot_indices
            .try_reserve_exact(n)
            .map_err(|_| Error::ResourceLimit)?;
        for word in payload.as_chunks::<4>().0 {
            plot_indices.push(i32::from_le_bytes(*word));
        }
        Ok((
            Self {
                plot_indices,
                improvement_array_size,
            },
            bytes,
            n,
        ))
    }
    fn size(n: usize, limits: Limits) -> Result<usize, Error> {
        if n > limits.max_items || u32::try_from(n).is_err() {
            return Err(Error::ResourceLimit);
        }
        let bytes = n
            .checked_mul(4)
            .and_then(|b| b.checked_add(12))
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
        let n = self.plot_indices.len();
        let bytes = Self::size(n, limits)?;
        let mut out = Vec::new();
        out.try_reserve_exact(bytes)
            .map_err(|_| Error::ResourceLimit)?;
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&(n as u32).to_le_bytes());
        for index in &self.plot_indices {
            out.extend_from_slice(&index.to_le_bytes());
        }
        out.extend_from_slice(&self.improvement_array_size.to_le_bytes());
        Ok((out, n))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nonempty_order_duplicates_extrema_prefix_and_caps() {
        let v = Improvements {
            plot_indices: vec![i32::MIN, 0, i32::MAX, i32::MIN],
            improvement_array_size: u32::MAX,
        };
        let exact = Limits {
            max_bytes: 28,
            max_items: 4,
        };
        let (b, items) = v.encode_with_items(exact).unwrap();
        assert_eq!(items, 4);
        let mut expected = vec![1, 0, 0, 0, 4, 0, 0, 0];
        for x in [i32::MIN, 0, i32::MAX, i32::MIN] {
            expected.extend_from_slice(&x.to_le_bytes());
        }
        expected.extend_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(b, expected);
        assert_eq!(Improvements::decode(&b, exact).unwrap(), v);
        assert_eq!(
            Improvements::decode_prefix_with_items(&b, exact).unwrap(),
            (v.clone(), 28, 4)
        );
        for end in 0..b.len() {
            assert!(Improvements::decode(&b[..end], exact).is_err());
        }
        for cap in [
            Limits {
                max_bytes: 27,
                ..exact
            },
            Limits {
                max_items: 3,
                ..exact
            },
        ] {
            assert_eq!(v.encode(cap), Err(Error::ResourceLimit));
            assert_eq!(Improvements::decode(&b, cap), Err(Error::ResourceLimit));
        }
        let mut tail = b;
        tail.push(255);
        assert_eq!(Improvements::decode(&tail, exact), Err(Error::Trailing));
        assert_eq!(Improvements::decode_prefix(&tail, exact).unwrap(), (v, 28));
    }
    #[test]
    fn empty_versions_unsigned_counts_and_independent_size() {
        let exact = Limits {
            max_bytes: 12,
            max_items: 0,
        };
        let v = Improvements {
            plot_indices: vec![],
            improvement_array_size: 0x80000001,
        };
        let b = v.encode(exact).unwrap();
        assert_eq!(Improvements::decode(&b, exact).unwrap(), v);
        for actual in [0u32, 2, u32::MAX] {
            let mut bad = b.clone();
            bad[..4].copy_from_slice(&actual.to_le_bytes());
            assert_eq!(
                Improvements::decode(&bad, exact),
                Err(Error::Version {
                    expected: VERSION,
                    actual
                })
            );
        }
        for count in [1u32, 0x80000000, u32::MAX] {
            let mut bad = b.clone();
            bad[4..8].copy_from_slice(&count.to_le_bytes());
            assert_eq!(Improvements::decode(&bad, exact), Err(Error::ResourceLimit));
        }
    }
}
