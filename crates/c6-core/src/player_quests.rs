//! Current Linux XP2 Player.Quests saved wire only. Native initialization and
//! gameplay quest generation, outcomes and RNG are outside this transport.
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
        write!(f, "Linux Player.Quests wire: {self:?}")
    }
}
impl std::error::Error for Error {}

/// The complete current Player.Quests record contains only its parent identity.
/// QuestData records belong to other owners and are not hidden/skipped here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Quests {
    /// Raw signed PlayerTypes ordinal, not a definition hash.
    pub player: i32,
}
impl Quests {
    pub fn decode(data: &[u8], limits: Limits) -> Result<Self, Error> {
        let (value, n, _) = Self::decode_prefix_with_items(data, limits)?;
        if n != data.len() {
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
        if limits.max_bytes < 8 {
            return Err(Error::ResourceLimit);
        }
        let bytes = data.get(..8).ok_or(Error::Truncated)?;
        let actual = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        if actual != VERSION {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
        }
        let player = i32::from_le_bytes(bytes[4..].try_into().unwrap());
        Ok((Self { player }, 8, 0))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        self.encode_with_items(limits).map(|(b, _)| b)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        if limits.max_bytes < 8 {
            return Err(Error::ResourceLimit);
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(8)
            .map_err(|_| Error::ResourceLimit)?;
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        bytes.extend_from_slice(&self.player.to_le_bytes());
        Ok((bytes, 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_extrema_prefixes_and_caps() {
        let exact = Limits {
            max_bytes: 8,
            max_items: 0,
        };
        for player in [i32::MIN, -1, 0, 63, i32::MAX] {
            let v = Quests { player };
            let (b, items) = v.encode_with_items(exact).unwrap();
            assert_eq!(items, 0);
            assert_eq!(Quests::decode(&b, exact).unwrap(), v);
            for end in 0..8 {
                assert_eq!(Quests::decode(&b[..end], exact), Err(Error::Truncated));
            }
            let short = Limits {
                max_bytes: 7,
                ..exact
            };
            assert_eq!(v.encode(short), Err(Error::ResourceLimit));
            assert_eq!(Quests::decode(&b, short), Err(Error::ResourceLimit));
            let mut tail = b;
            tail.push(255);
            assert_eq!(Quests::decode(&tail, exact), Err(Error::Trailing));
            assert_eq!(Quests::decode_prefix(&tail, exact).unwrap(), (v.clone(), 8));
            assert_eq!(
                Quests::decode_prefix_with_items(&tail, exact).unwrap(),
                (v, 8, 0)
            );
        }
    }
    #[test]
    fn rejects_noncurrent_versions() {
        let limits = Limits {
            max_bytes: 8,
            max_items: 0,
        };
        for actual in [0, 2, u32::MAX] {
            let mut b = Quests { player: -1 }.encode(limits).unwrap();
            b[..4].copy_from_slice(&actual.to_le_bytes());
            assert_eq!(
                Quests::decode(&b, limits),
                Err(Error::Version {
                    expected: VERSION,
                    actual
                })
            );
        }
    }
}
