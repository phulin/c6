//! Complete current Linux XP2 Player.Eras v2 wire transport.
//! Era definition hashes remain hashes; native definition remapping, archive
//! ownership, gameplay, player decisions, actions and turns are not implemented.
use crate::unit::Limits;

pub const VERSION: u32 = 2;
pub const WIRE_BYTES: usize = 12;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player.Eras wire: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Saved `m_eEra` and `m_eGameEra` definition hashes, in native write order.
/// Every 32-bit hash pattern is preserved, including unmapped and sentinel values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Eras {
    pub era_hash: u32,
    pub game_era_hash: u32,
}
impl Eras {
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
        if actual != VERSION {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
        }
        let era_hash = word(data, 4, limits)?;
        let game_era_hash = word(data, 8, limits)?;
        // Fixed scalar fields do not consume collection items under unit::Limits.
        Ok((
            Self {
                era_hash,
                game_era_hash,
            },
            WIRE_BYTES,
            0,
        ))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        Ok(self.encode_with_items(limits)?.0)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        if limits.max_bytes < WIRE_BYTES {
            return Err(Error::ResourceLimit);
        }
        let mut out = Vec::new();
        // The only allocation is the wire output, bounded independently of typed
        // storage. There is no dynamic typed storage and no aggregate heap claim.
        out.try_reserve_exact(WIRE_BYTES)
            .map_err(|_| Error::ResourceLimit)?;
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&self.era_hash.to_le_bytes());
        out.extend_from_slice(&self.game_era_hash.to_le_bytes());
        Ok((out, 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const EXACT: Limits = Limits {
        max_bytes: WIRE_BYTES,
        max_items: 0,
    };
    #[test]
    fn nonempty_hash_patterns_and_exact_caps() {
        for (era_hash, game_era_hash) in [(0, u32::MAX), (0x81234567, 0x76543210), (7, 7)] {
            let value = Eras {
                era_hash,
                game_era_hash,
            };
            let (bytes, items) = value.encode_with_items(EXACT).unwrap();
            assert_eq!(items, 0);
            assert_eq!(&bytes[..4], &2u32.to_le_bytes());
            assert_eq!(&bytes[4..8], &era_hash.to_le_bytes());
            assert_eq!(&bytes[8..], &game_era_hash.to_le_bytes());
            assert_eq!(Eras::decode(&bytes, EXACT).unwrap(), value);
            assert_eq!(
                Eras::decode_prefix_with_items(&bytes, EXACT).unwrap(),
                (value, 12, 0)
            );
            for end in 0..bytes.len() {
                assert_eq!(Eras::decode(&bytes[..end], EXACT), Err(Error::Truncated));
            }
            for cap in 0..WIRE_BYTES {
                let short = Limits {
                    max_bytes: cap,
                    ..EXACT
                };
                assert_eq!(value.encode(short), Err(Error::ResourceLimit));
                assert_eq!(Eras::decode(&bytes, short), Err(Error::ResourceLimit));
            }
        }
    }
    #[test]
    fn strict_prefix_and_version() {
        let value = Eras {
            era_hash: 19,
            game_era_hash: 23,
        };
        let mut bytes = value.encode(EXACT).unwrap();
        bytes.push(255);
        assert_eq!(Eras::decode(&bytes, EXACT), Err(Error::Trailing));
        assert_eq!(Eras::decode_prefix(&bytes, EXACT).unwrap(), (value, 12));
        for version in [0, 1, 3, u32::MAX] {
            bytes[..4].copy_from_slice(&version.to_le_bytes());
            assert_eq!(
                Eras::decode_prefix(&bytes, EXACT),
                Err(Error::Version {
                    expected: VERSION,
                    actual: version
                })
            );
        }
    }
}
