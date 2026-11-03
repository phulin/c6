//! Exact current Linux XP2 AI.Tactical saved wire. Native Read callbacks,
//! subscriptions, zombie-type replacement, archive effects and policy do not run.
use crate::{player_ai_homeland::MoveList, unit::Limits};
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
        write!(f, "Linux AI.Tactical wire: {self:?}")
    }
}
impl std::error::Error for Error {}
impl From<crate::player_ai_homeland::Error> for Error {
    fn from(value: crate::player_ai_homeland::Error) -> Self {
        use crate::player_ai_homeland::Error as E;
        match value {
            E::Truncated => Self::Truncated,
            E::Trailing => Self::Trailing,
            E::ResourceLimit => Self::ResourceLimit,
            E::Version { expected, actual } => Self::Version { expected, actual },
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tactical {
    pub player_id: i32,
    pub control_instance_id: u32,
    pub prioritized_moves: MoveList,
    /// Saved raw UnitAITypes ordinal, not a definition hash.
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
        if actual != VERSION {
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
        let zombie_type = word(data, pos, limits)? as i32;
        Ok((
            Self {
                player_id,
                control_instance_id,
                prioritized_moves,
                zombie_type,
            },
            pos + 4,
            items,
        ))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        self.encode_with_items(limits).map(|(v, _)| v)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
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
    fn boundaries() {
        let v = Tactical {
            player_id: i32::MIN,
            control_instance_id: u32::MAX,
            prioritized_moves: MoveList {
                base_moves: vec![i32::MIN, i32::MAX],
                moves: vec![Move {
                    allowed_move_hash: u32::MAX,
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
