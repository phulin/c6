//! Current Linux XP2 Game sync Random wire record; not complete live RNG state.
//! Reuses the Linux-native-validated raw Random codec. No gameplay or live Read.
use crate::rng::native_serialization::{self, SerializedRandomRecord};
use crate::unit::Limits;

pub const VERSION: u32 = 1;
pub const LENGTH: usize = native_serialization::LENGTH;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
    UnsupportedLegacy { flag: u8 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Game Random wire: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Serialized fields preserve all raw u32/bool carriers. Lock count, vptr and
/// padding are absent from this record. This does not inventory every game RNG.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Random {
    pub record: SerializedRandomRecord,
}
impl Random {
    /// Current writer grammar only. Native Read ignores version; this bounded
    /// transport explicitly rejects other versions rather than normalizing them.
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        if limits.max_bytes < LENGTH {
            return Err(Error::ResourceLimit);
        }
        let raw = bytes.get(..LENGTH).ok_or(Error::Truncated)?;
        let actual = u32::from_le_bytes(raw[..4].try_into().unwrap());
        if actual != VERSION {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
        }
        let (record, n) = SerializedRandomRecord::decode_prefix(raw).map_err(|e| match e {
            native_serialization::Error::UnsupportedLegacy { flag } => {
                Error::UnsupportedLegacy { flag }
            }
            native_serialization::Error::ShortInput { .. } => Error::Truncated,
            // Raw decoding never converts synchronous storage to a logical bool.
            native_serialization::Error::UnrepresentableSynchronous { .. } => unreachable!(),
        })?;
        Ok((Self { record }, n, 0))
    }
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        let (value, n, _) = Self::decode_prefix_with_items(bytes, limits)?;
        Ok((value, n))
    }
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let (value, n) = Self::decode_prefix(bytes, limits)?;
        if n != bytes.len() {
            return Err(Error::Trailing);
        }
        Ok(value)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        if limits.max_bytes < LENGTH {
            return Err(Error::ResourceLimit);
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(LENGTH)
            .map_err(|_| Error::ResourceLimit)?;
        bytes.extend_from_slice(&self.record.encode());
        Ok((bytes, 0))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        Ok(self.encode_with_items(limits)?.0)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const LIMITS: Limits = Limits {
        max_bytes: LENGTH,
        max_items: 0,
    };
    #[test]
    fn raw_extremes_and_every_synchronous_carrier() {
        for synchronous_byte in 0..=u8::MAX {
            for seed in [0, 1, 0x80000000, u32::MAX] {
                let value = Random {
                    record: SerializedRandomRecord {
                        seed,
                        calls: u32::MAX - seed,
                        resets: seed,
                        synchronous_byte,
                    },
                };
                let bytes = value.encode(LIMITS).unwrap();
                assert_eq!(Random::decode(&bytes, LIMITS), Ok(value));
                assert_eq!(value.encode_with_items(LIMITS), Ok((bytes.clone(), 0)));
                assert_eq!(
                    Random::decode_prefix_with_items(&bytes, LIMITS),
                    Ok((value, LENGTH, 0))
                );
            }
        }
    }
    #[test]
    fn truncation_budget_and_suffix() {
        let value = Random {
            record: SerializedRandomRecord {
                seed: 0,
                calls: 0,
                resets: 0,
                synchronous_byte: 255,
            },
        };
        let mut bytes = value.encode(LIMITS).unwrap();
        for n in 0..LENGTH {
            assert_eq!(Random::decode(&bytes[..n], LIMITS), Err(Error::Truncated));
            let short = Limits {
                max_bytes: n,
                ..LIMITS
            };
            assert_eq!(Random::decode(&bytes, short), Err(Error::ResourceLimit));
            assert_eq!(value.encode(short), Err(Error::ResourceLimit));
        }
        bytes.push(91);
        assert_eq!(Random::decode(&bytes, LIMITS), Err(Error::Trailing));
        assert_eq!(
            Random::decode_prefix_with_items(&bytes, LIMITS),
            Ok((value, LENGTH, 0))
        );
    }
    #[test]
    fn versions_and_legacy_flags_are_explicit() {
        let value = Random {
            record: SerializedRandomRecord {
                seed: 0,
                calls: 0,
                resets: 0,
                synchronous_byte: 0,
            },
        };
        for actual in [0u32, 2, u32::MAX] {
            let mut bytes = value.encode(LIMITS).unwrap();
            bytes[..4].copy_from_slice(&actual.to_le_bytes());
            assert_eq!(
                Random::decode(&bytes, LIMITS),
                Err(Error::Version {
                    expected: VERSION,
                    actual
                })
            );
        }
        for flag in 1..=255 {
            let mut bytes = value.encode(LIMITS).unwrap();
            bytes[17] = flag;
            assert_eq!(
                Random::decode(&bytes, LIMITS),
                Err(Error::UnsupportedLegacy { flag })
            );
        }
    }
}
