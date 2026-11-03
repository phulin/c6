//! Current Linux XP2 AI.Planning.Manager saved identity, not planning policy.
//!
//! Native Read also resolves Globals.GetPlayer, subscribes signals, initializes
//! its archive and marks the manager initialized. None of those live effects,
//! omitted priority/planner fields, RNG or gameplay are reproduced here.
use crate::unit::Limits;

pub const VERSION: u32 = 1;
pub const WIRE_BYTES: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux AI.Planning.Manager identity wire: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Exact saved signed Player identity; no pointer or global lookup is invented.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlanningManagerIdentity {
    pub player_id: i32,
}
impl PlanningManagerIdentity {
    pub fn decode(data: &[u8], limits: Limits) -> Result<Self, Error> {
        let (value, consumed) = Self::decode_prefix(data, limits)?;
        if consumed != data.len() {
            return Err(Error::Trailing);
        }
        Ok(value)
    }
    pub fn decode_prefix(data: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        Self::decode_prefix_with_items(data, limits).map(|(v, bytes, _)| (v, bytes))
    }
    /// This fixed record allocates no dynamic items, including when max_items=0.
    pub fn decode_prefix_with_items(
        data: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        if limits.max_bytes < WIRE_BYTES {
            return Err(Error::ResourceLimit);
        }
        let wire = data.get(..WIRE_BYTES).ok_or(Error::Truncated)?;
        let actual = u32::from_le_bytes(wire[..4].try_into().unwrap());
        if actual != VERSION {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
        }
        Ok((
            Self {
                player_id: i32::from_le_bytes(wire[4..].try_into().unwrap()),
            },
            WIRE_BYTES,
            0,
        ))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        self.encode_with_items(limits).map(|(wire, _)| wire)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        if limits.max_bytes < WIRE_BYTES {
            return Err(Error::ResourceLimit);
        }
        let mut wire = Vec::new();
        wire.try_reserve_exact(WIRE_BYTES)
            .map_err(|_| Error::ResourceLimit)?;
        wire.extend_from_slice(&VERSION.to_le_bytes());
        wire.extend_from_slice(&self.player_id.to_le_bytes());
        Ok((wire, 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_identity_exact_extent_and_composition_budget() {
        let limits = Limits {
            max_bytes: WIRE_BYTES,
            max_items: 0,
        };
        for player_id in [i32::MIN, -1, 0, 42, i32::MAX] {
            let value = PlanningManagerIdentity { player_id };
            let (wire, items) = value.encode_with_items(limits).unwrap();
            assert_eq!(items, 0);
            assert_eq!(&wire[..4], &[1, 0, 0, 0]);
            assert_eq!(&wire[4..], &player_id.to_le_bytes());
            assert_eq!(PlanningManagerIdentity::decode(&wire, limits), Ok(value));
            for end in 0..WIRE_BYTES {
                assert_eq!(
                    PlanningManagerIdentity::decode(&wire[..end], limits),
                    Err(Error::Truncated)
                );
            }
            let short = Limits {
                max_bytes: WIRE_BYTES - 1,
                ..limits
            };
            assert_eq!(value.encode(short), Err(Error::ResourceLimit));
            assert_eq!(
                PlanningManagerIdentity::decode(&wire, short),
                Err(Error::ResourceLimit)
            );
            let mut suffix = wire;
            suffix.extend_from_slice(&[9, 8, 7]);
            assert_eq!(
                PlanningManagerIdentity::decode_prefix_with_items(&suffix, limits),
                Ok((value, WIRE_BYTES, 0))
            );
            assert_eq!(
                PlanningManagerIdentity::decode(&suffix, limits),
                Err(Error::Trailing)
            );
        }
    }
    #[test]
    fn unsupported_versions_are_explicit() {
        let limits = Limits {
            max_bytes: WIRE_BYTES,
            max_items: 0,
        };
        for actual in [0u32, 2, u32::MAX] {
            let mut wire = actual.to_le_bytes().to_vec();
            wire.extend_from_slice(&(-1i32).to_le_bytes());
            assert_eq!(
                PlanningManagerIdentity::decode(&wire, limits),
                Err(Error::Version {
                    expected: VERSION,
                    actual
                })
            );
        }
    }
}
