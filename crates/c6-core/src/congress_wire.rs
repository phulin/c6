//! Reached Linux XP2 Congress7 / Resolution5 / Discussion3 wire projection.
//! Legacy Resolution5 target decoding requires explicitly supplied configured
//! definitions. Only source-proven PlayerTarget1 is supported; other kinds and
//! versions fail explicitly. No native policy, callbacks or turn actions run.
//! Raw votes, definition hashes and ordering remain separate from native remaps.

pub use crate::unit::Limits;
pub const VERSION: u32 = 7;
pub const RESOLUTION_VERSION: u32 = 5;
pub const DISCUSSION_VERSION: u32 = 3;
pub const TARGET_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    MissingResolutionCatalog { type_hash: u32 },
    UnsupportedTargetKind { kind_hash: u32 },
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Congress wire: {self:?}")
    }
}
impl std::error::Error for Error {}
struct Reader<'a> {
    data: &'a [u8],
    catalog: &'a dyn ResolutionTargetCatalog,
    pos: usize,
    items: usize,
    limits: Limits,
}
impl Reader<'_> {
    fn raw(&mut self, size: usize) -> Result<&[u8], Error> {
        let end = self.pos.checked_add(size).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let value = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(value)
    }
    fn version(&mut self, expected: u32) -> Result<u32, Error> {
        let actual = u32::read(self)?;
        if actual != expected {
            return Err(Error::Version { expected, actual });
        }
        Ok(actual)
    }
}
trait Wire: Sized {
    const MIN: usize;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error>;
}
impl Wire for u32 {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self::from_le_bytes(r.raw(4)?.try_into().unwrap()))
    }
}
impl Wire for i32 {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(u32::read(r)? as i32)
    }
}
impl Wire for u8 {
    const MIN: usize = 1;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(r.raw(1)?[0])
    }
}
impl<T: Wire> Wire for Vec<T> {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let count = u32::read(r)? as usize;
        r.items = r.items.checked_add(count).ok_or(Error::ResourceLimit)?;
        if r.items > r.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        let minimum = count.checked_mul(T::MIN).ok_or(Error::ResourceLimit)?;
        if minimum > r.limits.max_bytes - r.pos {
            return Err(Error::ResourceLimit);
        }
        if minimum > r.data.len() - r.pos {
            return Err(Error::Truncated);
        }
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        for _ in 0..count {
            values.push(T::read(r)?);
        }
        Ok(values)
    }
}
pub trait ResolutionTargetCatalog {
    fn target_kind_hash(&self, resolution_type_hash: u32) -> Result<u32, Error>;
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OmittedPriorState {
    Unknown,
}
impl<T: Wire> Wire for [T; 64] {
    const MIN: usize = 64 * T::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.items = r.items.checked_add(64).ok_or(Error::ResourceLimit)?;
        if r.items > r.limits.max_items || Self::MIN > r.limits.max_bytes - r.pos {
            return Err(Error::ResourceLimit);
        }
        if Self::MIN > r.data.len() - r.pos {
            return Err(Error::Truncated);
        }
        let mut v = Vec::new();
        v.try_reserve_exact(64).map_err(|_| Error::ResourceLimit)?;
        for _ in 0..64 {
            v.push(T::read(r)?);
        }
        v.try_into().map_err(|_| Error::ResourceLimit)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerTarget {
    pub wrapper_version: u32,
    pub wire_version: u32,
    pub player_ordinal: i32,
    pub possible_player_ordinals: [Vec<i32>; 64],
    /// Serialized only since PlayerTarget2; no bytes at reached1.
    pub selected_player_ordinals: OmittedPriorState,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolution {
    pub wire_version: u32,
    pub player_votes: [i32; 64],
    pub resolution_type_hash: u32,
    pub winning_player_ordinal: i32,
    /// Read into a temporary and discarded natively through Resolution5.
    pub legacy_value_raw: i32,
    /// Selected option array is serialized since6, absent at5.
    pub selected_options: OmittedPriorState,
    pub target_kind_hash_from_catalog: u32,
    pub target: PlayerTarget,
}
impl Wire for Resolution {
    const MIN: usize = 540;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = r.version(RESOLUTION_VERSION)?;
        let player_votes = Wire::read(r)?;
        let resolution_type_hash = u32::read(r)?;
        let winning_player_ordinal = i32::read(r)?;
        let legacy_value_raw = i32::read(r)?;
        let kind = r.catalog.target_kind_hash(resolution_type_hash)?;
        if kind != 0x971e421c {
            return Err(Error::UnsupportedTargetKind { kind_hash: kind });
        }
        let target = PlayerTarget {
            wrapper_version: r.version(TARGET_VERSION)?,
            wire_version: r.version(TARGET_VERSION)?,
            player_ordinal: i32::read(r)?,
            possible_player_ordinals: Wire::read(r)?,
            selected_player_ordinals: OmittedPriorState::Unknown,
        };
        Ok(Self {
            wire_version,
            player_votes,
            resolution_type_hash,
            winning_player_ordinal,
            legacy_value_raw,
            selected_options: OmittedPriorState::Unknown,
            target_kind_hash_from_catalog: kind,
            target,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Discussion {
    pub wire_version: u32,
    pub target_item_raw: i32,
    pub player_votes: [i32; 64],
    pub proposing_player_ordinal: i32,
    pub discussion_point_hash: u32,
    pub possible_discussion_hashes: Vec<u32>,
    /// Exact native true-to-int1 default below6 (legacy4/5 conversion skipped3).
    pub passed_read_default: i32,
    /// Exact native -1 default below5.
    pub activated_turn_read_default: i32,
}
impl Wire for Discussion {
    const MIN: usize = 276;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: r.version(DISCUSSION_VERSION)?,
            target_item_raw: i32::read(r)?,
            player_votes: Wire::read(r)?,
            proposing_player_ordinal: i32::read(r)?,
            discussion_point_hash: u32::read(r)?,
            possible_discussion_hashes: Vec::read(r)?,
            passed_read_default: 1,
            activated_turn_read_default: -1,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Manager {
    pub wire_version: u32,
    pub accumulate_start_votes: i32,
    /// Exact native default before8.
    pub last_normal_session_read_default: i32,
    /// Exact native default before10.
    pub last_special_session_read_default: i32,
    /// Exact native default before9.
    pub established_turns_read_default: i32,
    pub resolutions: Vec<Resolution>,
    pub discussions: Vec<Discussion>,
    pub resolutions_for_injection: Vec<Resolution>,
    /// Since11, self-defaulting without a clear at reached7.
    pub last_resolutions_of_types: OmittedPriorState,
}
impl Manager {
    pub fn decode(
        bytes: &[u8],
        limits: Limits,
        catalog: &dyn ResolutionTargetCatalog,
    ) -> Result<Self, Error> {
        let (v, n, _) = Self::decode_prefix_with_items(bytes, limits, catalog)?;
        if n != bytes.len() {
            return Err(Error::Trailing);
        }
        Ok(v)
    }
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
        catalog: &dyn ResolutionTargetCatalog,
    ) -> Result<(Self, usize, usize), Error> {
        let mut r = Reader {
            data: bytes,
            catalog,
            pos: 0,
            items: 0,
            limits,
        };
        let v = Self {
            wire_version: r.version(VERSION)?,
            accumulate_start_votes: i32::read(&mut r)?,
            last_normal_session_read_default: -1,
            last_special_session_read_default: -1,
            established_turns_read_default: 0,
            resolutions: Vec::read(&mut r)?,
            discussions: Vec::read(&mut r)?,
            resolutions_for_injection: Vec::read(&mut r)?,
            last_resolutions_of_types: OmittedPriorState::Unknown,
        };
        Ok((v, r.pos, r.items))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Catalog(u32);
    impl ResolutionTargetCatalog for Catalog {
        fn target_kind_hash(&self, key: u32) -> Result<u32, Error> {
            if key == 0x81234567 {
                Ok(self.0)
            } else {
                Err(Error::MissingResolutionCatalog { type_hash: key })
            }
        }
    }
    fn words(b: &mut Vec<u8>, v: &[u32]) {
        for n in v {
            b.extend(n.to_le_bytes());
        }
    }
    fn fixture() -> Vec<u8> {
        let mut b = Vec::new();
        words(&mut b, &[7, 224, 1, 5]);
        for i in 0..64 {
            words(&mut b, &[(i as i32 - 32) as u32]);
        }
        words(&mut b, &[0x81234567, 63, 999, 1, 1, u32::MAX]);
        for i in 0..64 {
            match i {
                0 => words(&mut b, &[2, 14, u32::MAX]),
                63 => words(&mut b, &[1, 7]),
                _ => words(&mut b, &[0]),
            }
        }
        words(&mut b, &[1, 3, (-7i32) as u32]);
        for i in 0..64 {
            words(&mut b, &[(100 - i) as u32]);
        }
        words(&mut b, &[62, 0xabcdef01, 2, 0x87654321, 0x87654321, 0]);
        b
    }
    #[test]
    fn literal_votes_catalog_target_nested_order_and_native_defaults() {
        let b = fixture();
        assert_eq!(b.len(), 856);
        let (v, n, i) =
            Manager::decode_prefix_with_items(&b, Limits::default(), &Catalog(0x971e421c)).unwrap();
        assert_eq!((n, i), (856, 199));
        assert_eq!(v.accumulate_start_votes, 224);
        assert_eq!(
            (
                v.last_normal_session_read_default,
                v.last_special_session_read_default,
                v.established_turns_read_default
            ),
            (-1, -1, 0)
        );
        assert_eq!(v.last_resolutions_of_types, OmittedPriorState::Unknown);
        let r = &v.resolutions[0];
        assert_eq!((r.player_votes[0], r.player_votes[63]), (-32, 31));
        assert_eq!(
            (
                r.resolution_type_hash,
                r.winning_player_ordinal,
                r.legacy_value_raw
            ),
            (0x81234567, 63, 999)
        );
        assert_eq!(r.selected_options, OmittedPriorState::Unknown);
        assert_eq!(r.target.player_ordinal, -1);
        assert_eq!(r.target.possible_player_ordinals[0], vec![14, -1]);
        assert_eq!(r.target.possible_player_ordinals[63], vec![7]);
        assert_eq!(
            r.target.selected_player_ordinals,
            OmittedPriorState::Unknown
        );
        let d = &v.discussions[0];
        assert_eq!((d.target_item_raw, d.proposing_player_ordinal), (-7, 62));
        assert_eq!((d.player_votes[0], d.player_votes[63]), (100, 37));
        assert_eq!(d.discussion_point_hash, 0xabcdef01);
        assert_eq!(d.possible_discussion_hashes, vec![0x87654321, 0x87654321]);
        assert_eq!(
            (d.passed_read_default, d.activated_turn_read_default),
            (1, -1)
        );
        assert!(v.resolutions_for_injection.is_empty());
    }
    #[test]
    fn cuts_limits_independent_versions_catalog_refusal_and_trailing() {
        let b = fixture();
        let c = Catalog(0x971e421c);
        for cut in 0..b.len() {
            assert!(
                Manager::decode(&b[..cut], Limits::default(), &c).is_err(),
                "cut {cut}"
            );
        }
        for cap in 0..b.len() {
            assert_eq!(
                Manager::decode(
                    &b,
                    Limits {
                        max_bytes: cap,
                        ..Limits::default()
                    },
                    &c
                ),
                Err(Error::ResourceLimit)
            );
        }
        for cap in 0..199 {
            assert_eq!(
                Manager::decode(
                    &b,
                    Limits {
                        max_items: cap,
                        ..Limits::default()
                    },
                    &c
                ),
                Err(Error::ResourceLimit)
            );
        }
        for (at, expected) in [(0, 7), (12, 5), (284, 1), (288, 1), (568, 3)] {
            let mut bad = b.clone();
            bad[at..at + 4].copy_from_slice(&99u32.to_le_bytes());
            assert_eq!(
                Manager::decode(&bad, Limits::default(), &c),
                Err(Error::Version {
                    expected,
                    actual: 99
                }),
                "at {at}"
            );
        }
        assert_eq!(
            Manager::decode(&b, Limits::default(), &Catalog(0x12345678)),
            Err(Error::UnsupportedTargetKind {
                kind_hash: 0x12345678
            })
        );
        let mut bad = b.clone();
        bad[272..276].copy_from_slice(&123u32.to_le_bytes());
        assert_eq!(
            Manager::decode(&bad, Limits::default(), &c),
            Err(Error::MissingResolutionCatalog { type_hash: 123 })
        );
        for at in [8, 296, 564, 840, 852] {
            let mut bad = b.clone();
            bad[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert_eq!(
                Manager::decode(&bad, Limits::default(), &c),
                Err(Error::ResourceLimit),
                "at {at}"
            );
        }
        let mut tail = b.clone();
        tail.push(0);
        assert_eq!(
            Manager::decode(&tail, Limits::default(), &c),
            Err(Error::Trailing)
        );
        assert_eq!(
            Manager::decode_prefix_with_items(&tail, Limits::default(), &c)
                .unwrap()
                .1,
            b.len()
        );
    }
}
