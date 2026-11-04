//! Linux XP2 AI.Tactical version1/2 saved wire. Native Read callbacks,
//! subscriptions, zombie-type replacement, archive effects and policy do not run.
use crate::{player_ai_homeland::MoveList, unit::Limits};
pub const VERSION: u32 = 2;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    UnsupportedLegacyWrite,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux AI.Tactical wire: {self:?}")
    }
}
impl std::error::Error for Error {}
impl From<crate::player_ai_homeland::Error> for Error {
    fn from(value: crate::player_ai_homeland::Error) -> Self {
        use crate::player_ai_homeland::Error as E;
        match value {
            E::UnsupportedLegacyWrite => Self::UnsupportedLegacyWrite,
            E::Truncated => Self::Truncated,
            E::Trailing => Self::Trailing,
            E::ResourceLimit => Self::ResourceLimit,
            E::Version { expected, actual } => Self::Version { expected, actual },
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tactical {
    pub wire_version: u32,
    pub player_id: i32,
    pub control_instance_id: u32,
    pub prioritized_moves: MoveList,
    /// Saved raw UnitAITypes ordinal at2; native Read default-1 at1.
    /// Native Read subsequently replaces this through definition lookup, which is not run.
    pub zombie_type: i32,
}
fn word(data: &[u8], pos: usize, limits: Limits) -> Result<u32, Error> {
    let end = pos.checked_add(4).ok_or(Error::ResourceLimit)?;
    if end > limits.max_bytes {
        return Err(Error::ResourceLimit);
    }
    Ok(u32::from_le_bytes(
        data.get(pos..end)
            .ok_or(Error::Truncated)?
            .try_into()
            .unwrap(),
    ))
}
impl Tactical {
    pub fn decode(data: &[u8], limits: Limits) -> Result<Self, Error> {
        let (v, n) = Self::decode_prefix(data, limits)?;
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
        let actual = word(data, 0, limits)?;
        if !matches!(actual, 1 | VERSION) {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
        }
        let player_id = word(data, 4, limits)? as i32;
        let control_instance_id = word(data, 8, limits)?;
        let (prioritized_moves, used, items) = MoveList::decode_prefix_with_items(
            &data[12..],
            Limits {
                max_bytes: limits.max_bytes - 12,
                max_items: limits.max_items,
            },
        )?;
        let pos = 12usize.checked_add(used).ok_or(Error::ResourceLimit)?;
        let (zombie_type, end) = if actual == VERSION {
            (
                word(data, pos, limits)? as i32,
                pos.checked_add(4).ok_or(Error::ResourceLimit)?,
            )
        } else {
            (-1, pos)
        };
        Ok((
            Self {
                wire_version: actual,
                player_id,
                control_instance_id,
                prioritized_moves,
                zombie_type,
            },
            end,
            items,
        ))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        self.encode_with_items(limits).map(|(v, _)| v)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        if self.wire_version != VERSION {
            return Err(Error::UnsupportedLegacyWrite);
        }
        let remaining = limits
            .max_bytes
            .checked_sub(16)
            .ok_or(Error::ResourceLimit)?;
        let (moves, items) = self.prioritized_moves.encode_with_items(Limits {
            max_bytes: remaining,
            max_items: limits.max_items,
        })?;
        let len = moves.len().checked_add(16).ok_or(Error::ResourceLimit)?;
        let mut out = Vec::new();
        out.try_reserve_exact(len)
            .map_err(|_| Error::ResourceLimit)?;
        for v in [VERSION, self.player_id as u32, self.control_instance_id] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&moves);
        out.extend_from_slice(&self.zombie_type.to_le_bytes());
        Ok((out, items))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::player_ai_homeland::Move;
    #[test]
    fn legacy_tactical_omits_zombie_word_and_keeps_move_ordinal() {
        let mut raw = Vec::new();
        for v in [
            1u32,
            u32::MAX,
            0x80000001,
            1,
            1,
            i32::MIN as u32,
            1,
            1,
            u32::MAX,
            (-257i32) as u32,
        ] {
            raw.extend_from_slice(&v.to_le_bytes());
        }
        let cap = Limits {
            max_bytes: 40,
            max_items: 2,
        };
        let (v, n, items) = Tactical::decode_prefix_with_items(&raw, cap).unwrap();
        assert_eq!((n, items), (40, 2));
        assert_eq!(v.wire_version, 1);
        assert_eq!(v.player_id, -1);
        assert_eq!(v.control_instance_id, 0x80000001);
        assert_eq!(v.zombie_type, -1);
        assert_eq!(
            v.prioritized_moves.moves[0].allowed_move,
            crate::player_ai_homeland::MoveReference::LegacyType(-1)
        );
        assert_eq!(v.encode(cap), Err(Error::UnsupportedLegacyWrite));
        for n in 0..40 {
            assert!(Tactical::decode(&raw[..n], cap).is_err());
        }
        for short in [
            Limits {
                max_bytes: 39,
                ..cap
            },
            Limits {
                max_items: 1,
                ..cap
            },
        ] {
            assert_eq!(Tactical::decode(&raw, short), Err(Error::ResourceLimit));
        }
        raw.extend_from_slice(&99u32.to_le_bytes());
        assert_eq!(
            Tactical::decode_prefix_with_items(&raw, cap),
            Ok((v, 40, 2))
        );
        assert_eq!(Tactical::decode(&raw, cap), Err(Error::Trailing));
    }
    #[test]
    fn boundaries() {
        let v = Tactical {
            wire_version: VERSION,
            player_id: i32::MIN,
            control_instance_id: u32::MAX,
            prioritized_moves: MoveList {
                base_moves: vec![i32::MIN, i32::MAX],
                moves: vec![Move {
                    allowed_move: crate::player_ai_homeland::MoveReference::AllowedMoveHash(
                        u32::MAX,
                    ),
                    priority: i32::MIN,
                }],
            },
            zombie_type: i32::MAX,
        };
        let (raw, items) = v.encode_with_items(Limits::default()).unwrap();
        let cap = Limits {
            max_bytes: raw.len(),
            max_items: items,
        };
        assert_eq!(items, 3);
        assert_eq!(Tactical::decode(&raw, cap), Ok(v.clone()));
        for end in 0..raw.len() {
            assert!(Tactical::decode(&raw[..end], cap).is_err());
        }
        for short in [
            Limits {
                max_bytes: raw.len() - 1,
                ..cap
            },
            Limits {
                max_items: items - 1,
                ..cap
            },
        ] {
            assert_eq!(Tactical::decode(&raw, short), Err(Error::ResourceLimit));
            assert_eq!(v.encode(short), Err(Error::ResourceLimit));
        }
        let mut tail = raw.clone();
        tail.push(90);
        assert_eq!(Tactical::decode(&tail, cap), Err(Error::Trailing));
        assert_eq!(
            Tactical::decode_prefix(&tail, cap),
            Ok((v.clone(), raw.len()))
        );
        assert_eq!(
            Tactical::decode_prefix_with_items(&tail, cap),
            Ok((v, raw.len(), items))
        );
        for offset in [0, 12, 32] {
            let mut bad = raw.clone();
            bad[offset..offset + 4].copy_from_slice(&99u32.to_le_bytes());
            assert!(matches!(
                Tactical::decode(&bad, cap),
                Err(Error::Version { .. })
            ));
        }
        for offset in [16, 28] {
            let mut bad = raw.clone();
            bad[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert_eq!(Tactical::decode(&bad, cap), Err(Error::ResourceLimit));
        }
        assert_eq!(
            Tactical::decode(
                &raw,
                Limits {
                    max_bytes: usize::MAX,
                    max_items: usize::MAX
                }
            )
            .unwrap()
            .encode(cap)
            .unwrap(),
            raw
        );
    }
}
