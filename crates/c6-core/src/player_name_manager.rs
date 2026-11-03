//! Complete current Linux XP2 Player.NameManager v2 wire transport.
//! This preserves ordered signed usage counts. Native configuration-dependent
//! Read padding, historical v1 migration, gameplay, actions and turns are absent.
use crate::unit::Limits;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    InvalidContainer,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player.NameManager wire: {self:?}")
    }
}
impl std::error::Error for Error {}

/// `m_aiCitizenNameUsageCount`, in native vector order; no pointer identities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NameManager {
    pub citizen_name_usage_counts: Vec<i16>,
}
impl NameManager {
    pub fn decode(data: &[u8], limits: Limits) -> Result<Self, Error> {
        let (value, used, _) = Self::decode_prefix_with_items(data, limits)?;
        if used != data.len() {
            return Err(Error::Trailing);
        }
        Ok(value)
    }
    pub fn decode_prefix(data: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        let (value, used, _) = Self::decode_prefix_with_items(data, limits)?;
        Ok((value, used))
    }
    pub fn decode_prefix_with_items(
        data: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        fn word(data: &[u8], offset: usize, limits: Limits) -> Result<u32, Error> {
            if offset + 4 > limits.max_bytes {
                return Err(Error::ResourceLimit);
            }
            Ok(u32::from_le_bytes(
                data.get(offset..offset + 4)
                    .ok_or(Error::Truncated)?
                    .try_into()
                    .unwrap(),
            ))
        }
        let actual = word(data, 0, limits)?;
        if actual != 2 {
            return Err(Error::Version {
                expected: 2,
                actual,
            });
        }
        let count = word(data, 4, limits)?;
        if count > i32::MAX as u32 {
            return Err(Error::InvalidContainer);
        }
        let count = count as usize;
        if count > limits.max_items {
            return Err(Error::ResourceLimit);
        }
        let array_count = word(data, 8, limits)? as usize;
        if array_count != count {
            return Err(Error::InvalidContainer);
        }
        let end = count
            .checked_mul(2)
            .and_then(|n| n.checked_add(12))
            .ok_or(Error::ResourceLimit)?;
        if end > limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let raw = data.get(12..end).ok_or(Error::Truncated)?;
        let mut counts = Vec::new();
        counts
            .try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        for bytes in raw.as_chunks::<2>().0 {
            counts.push(i16::from_le_bytes(*bytes));
        }
        Ok((
            Self {
                citizen_name_usage_counts: counts,
            },
            end,
            count,
        ))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        Ok(self.encode_with_items(limits)?.0)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        let n = self.citizen_name_usage_counts.len();
        if n > i32::MAX as usize {
            return Err(Error::InvalidContainer);
        }
        let bytes = n
            .checked_mul(2)
            .and_then(|x| x.checked_add(12))
            .ok_or(Error::ResourceLimit)?;
        if n > limits.max_items || bytes > limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let mut out = Vec::new();
        out.try_reserve_exact(bytes)
            .map_err(|_| Error::ResourceLimit)?;
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&(n as u32).to_le_bytes());
        out.extend_from_slice(&(n as u32).to_le_bytes());
        for value in &self.citizen_name_usage_counts {
            out.extend_from_slice(&value.to_le_bytes());
        }
        Ok((out, n))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn limits(bytes: usize, items: usize) -> Limits {
        Limits {
            max_bytes: bytes,
            max_items: items,
        }
    }
    #[test]
    fn nonempty_order_signed_values_and_exact_budgets() {
        let value = NameManager {
            citizen_name_usage_counts: vec![i16::MIN, 2, -1, 2, i16::MAX],
        };
        let exact = limits(22, 5);
        let (raw, items) = value.encode_with_items(exact).unwrap();
        assert_eq!(items, 5);
        assert_eq!(&raw[..12], &[2, 0, 0, 0, 5, 0, 0, 0, 5, 0, 0, 0]);
        assert_eq!(NameManager::decode(&raw, exact), Ok(value.clone()));
        assert_eq!(
            NameManager::decode_prefix_with_items(&raw, exact),
            Ok((value.clone(), 22, 5))
        );
        for short in [limits(21, 5), limits(22, 4)] {
            assert_eq!(value.encode(short), Err(Error::ResourceLimit));
            assert_eq!(NameManager::decode(&raw, short), Err(Error::ResourceLimit));
        }
    }
    #[test]
    fn malformed_and_prefix() {
        let value = NameManager {
            citizen_name_usage_counts: vec![],
        };
        let cap = limits(100, 100);
        let mut raw = value.encode(cap).unwrap();
        for n in 0..12 {
            assert_eq!(NameManager::decode(&raw[..n], cap), Err(Error::Truncated));
        }
        raw.push(99);
        assert_eq!(NameManager::decode(&raw, cap), Err(Error::Trailing));
        assert_eq!(NameManager::decode_prefix(&raw, cap), Ok((value, 12)));
        raw[0] = 1;
        assert_eq!(
            NameManager::decode(&raw, cap),
            Err(Error::Version {
                expected: 2,
                actual: 1
            })
        );
        raw[0] = 2;
        raw[8] = 1;
        assert_eq!(NameManager::decode(&raw, cap), Err(Error::InvalidContainer));
        raw[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(NameManager::decode(&raw, cap), Err(Error::InvalidContainer));
    }
    #[test]
    fn payload_truncation_and_allocation_preflight() {
        let value = NameManager {
            citizen_name_usage_counts: vec![1, -2],
        };
        let raw = value.encode(limits(16, 2)).unwrap();
        for n in 12..16 {
            assert_eq!(
                NameManager::decode(&raw[..n], limits(16, 2)),
                Err(Error::Truncated)
            );
        }
        let hostile = [
            2u32.to_le_bytes(),
            1_000_000u32.to_le_bytes(),
            1_000_000u32.to_le_bytes(),
        ]
        .concat();
        assert_eq!(
            NameManager::decode(&hostile, limits(12, 1_000_000)),
            Err(Error::ResourceLimit)
        );
    }
}
