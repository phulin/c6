//! Linux XP2 Game.GreatPeople4/5 wire/default projection only.
//! Ordered hash pairs retain unknown/duplicate keys. Native definition remaps,
//! initialized class arrays, recruitment, callbacks and gameplay are unapplied.
//! Byte/item limits bound wire consumption and counted entries; checked logical
//! allocation arithmetic and fallible reservations are not a separate heap cap.

pub use crate::unit::{HashValue, Limits};
pub const VERSION: u32 = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Game.GreatPeople wire: {self:?}")
    }
}
impl std::error::Error for Error {}
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    items: usize,
    allocated: usize,
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
    fn version(&mut self) -> Result<u32, Error> {
        let actual = u32::read(self)?;
        if !matches!(actual, 4 | VERSION) {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
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
        r.allocated = r
            .allocated
            .checked_add(
                count
                    .checked_mul(std::mem::size_of::<T>())
                    .ok_or(Error::ResourceLimit)?,
            )
            .ok_or(Error::ResourceLimit)?;
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
impl<T: Wire> Wire for HashValue<T> {
    const MIN: usize = 4 + T::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            definition_hash: u32::read(r)?,
            value: T::read(r)?,
        })
    }
}
macro_rules! record {
    ($name:ident, {$($field:ident:$ty:ty),* $(,)?}) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name {pub wire_version:u32, $(pub $field:$ty),*}
        impl Wire for $name {
            const MIN:usize = 4 $(+ <$ty as Wire>::MIN)*;
            fn read(r:&mut Reader<'_>)->Result<Self,Error> {
                let wire_version=r.version()?;
                Ok(Self {wire_version,$($field:<$ty as Wire>::read(r)?),*})
            }
        }
    }
}
record!(TimelineEntry, {
    individual_hash:u32, class_hash:u32, era_hash:u32,
    cost:i32, claimant_ordinal:i32, turn_granted:i32,
});
record!(RejectionEntry, {individual_hash:u32, rejecting_player_ordinal:i32});
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuaranteeEntry {
    pub wire_version: u32,
    pub class_hash: u32,
    pub player_ordinal: i32,
    /// None at4: native default is EraTypes index-1, not a serialized hashMAX.
    pub era_hash: Option<u32>,
}
impl GuaranteeEntry {
    pub fn omitted_era_native_index(&self) -> Option<i32> {
        (self.wire_version == 4).then_some(-1)
    }
}
impl Wire for GuaranteeEntry {
    const MIN: usize = 12;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = r.version()?;
        Ok(Self {
            wire_version,
            class_hash: u32::read(r)?,
            player_ordinal: i32::read(r)?,
            era_hash: if wire_version == VERSION {
                Some(u32::read(r)?)
            } else {
                None
            },
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GreatPeople {
    pub wire_version: u32,
    /// Raw carrier. Native Read consumes all lists regardless of this flag.
    pub initialized_raw: u8,
    pub timeline: Vec<TimelineEntry>,
    /// Ordered serialized overlays only, not the initialized dense class array.
    pub current_timeline_by_class: Vec<HashValue<TimelineEntry>>,
    pub rejections: Vec<RejectionEntry>,
    pub guarantees: Vec<GuaranteeEntry>,
}
impl Wire for GreatPeople {
    const MIN: usize = 21;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: r.version()?,
            initialized_raw: u8::read(r)?,
            timeline: Vec::<TimelineEntry>::read(r)?,
            current_timeline_by_class: Vec::<HashValue<TimelineEntry>>::read(r)?,
            rejections: Vec::<RejectionEntry>::read(r)?,
            guarantees: Vec::<GuaranteeEntry>::read(r)?,
        })
    }
}
impl GreatPeople {
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        let mut r = Reader {
            data: bytes,
            pos: 0,
            items: 0,
            allocated: 0,
            limits,
        };
        let value = Self::read(&mut r)?;
        Ok((value, r.pos, r.items))
    }
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let (value, used, _) = Self::decode_prefix_with_items(bytes, limits)?;
        if used != bytes.len() {
            return Err(Error::Trailing);
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn words(bytes: &mut Vec<u8>, values: &[u32]) {
        for value in values {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    fn specimen() -> Vec<u8> {
        let mut b = Vec::new();
        words(&mut b, &[4]);
        b.push(0xfe);
        words(
            &mut b,
            &[
                1,
                4,
                0x81234567,
                u32::MAX,
                0x80000001,
                i32::MIN as u32,
                u32::MAX,
                i32::MAX as u32,
                2,
            ],
        );
        for version in [4, 5] {
            words(&mut b, &[0x81234567, version, 1, 2, 3, 4, 5, 6]);
        }
        words(
            &mut b,
            &[
                1,
                4,
                0x80000001,
                i32::MIN as u32,
                2,
                4,
                0x81234567,
                u32::MAX,
                5,
                u32::MAX,
                i32::MIN as u32,
                0x80000001,
            ],
        );
        b
    }
    #[test]
    fn independently_tagged_records_and_hash_pairs_keep_order_and_omitted_era() {
        let b = specimen();
        assert_eq!(b.len(), 153);
        let cap = Limits {
            max_bytes: 153,
            max_items: 6,
        };
        let (v, n, i) = GreatPeople::decode_prefix_with_items(&b, cap).unwrap();
        assert_eq!((n, i), (153, 6));
        assert_eq!(
            (
                v.wire_version,
                v.initialized_raw,
                v.timeline[0].cost,
                v.timeline[0].claimant_ordinal
            ),
            (4, 0xfe, i32::MIN, -1)
        );
        assert_eq!(
            v.current_timeline_by_class
                .iter()
                .map(|x| x.definition_hash)
                .collect::<Vec<_>>(),
            vec![0x81234567; 2]
        );
        assert_eq!(v.current_timeline_by_class[1].value.wire_version, 5);
        assert_eq!(
            (
                v.guarantees[0].era_hash,
                v.guarantees[0].omitted_era_native_index()
            ),
            (None, Some(-1))
        );
        assert_eq!(
            (
                v.guarantees[1].era_hash,
                v.guarantees[1].omitted_era_native_index()
            ),
            (Some(0x80000001), None)
        );
        let mut trailing = b.clone();
        trailing.push(0xaa);
        assert_eq!(
            GreatPeople::decode_prefix_with_items(&trailing, cap)
                .unwrap()
                .1,
            153
        );
        assert_eq!(GreatPeople::decode(&trailing, cap), Err(Error::Trailing));
        for cut in 0..b.len() {
            assert!(GreatPeople::decode_prefix_with_items(&b[..cut], cap).is_err());
        }
        assert_eq!(
            GreatPeople::decode(
                &b,
                Limits {
                    max_bytes: 152,
                    ..cap
                }
            ),
            Err(Error::ResourceLimit)
        );
        assert_eq!(
            GreatPeople::decode(
                &b,
                Limits {
                    max_items: 5,
                    ..cap
                }
            ),
            Err(Error::ResourceLimit)
        );
        for at in [0, 9, 45, 77, 109, 125, 137] {
            let mut bad = b.clone();
            bad[at..at + 4].copy_from_slice(&3u32.to_le_bytes());
            assert!(
                matches!(
                    GreatPeople::decode(&bad, cap),
                    Err(Error::Version {
                        expected: 5,
                        actual: 3
                    })
                ),
                "{at}"
            );
        }
    }
    #[test]
    fn initialized_flag_never_omits_the_four_lists_and_counts_are_bounded() {
        for version in [4u32, 5] {
            for flag in 0..=255 {
                let mut b = version.to_le_bytes().to_vec();
                b.push(flag);
                assert!(GreatPeople::decode(&b, Limits::default()).is_err());
                words(&mut b, &[0, 0, 0, 0]);
                let v = GreatPeople::decode(
                    &b,
                    Limits {
                        max_bytes: 21,
                        max_items: 0,
                    },
                )
                .unwrap();
                assert_eq!((v.wire_version, v.initialized_raw), (version, flag));
                for at in [5, 9, 13, 17] {
                    let mut hostile = b.clone();
                    hostile[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
                    assert!(GreatPeople::decode(
                        &hostile,
                        Limits {
                            max_bytes: 21,
                            max_items: usize::MAX
                        }
                    )
                    .is_err());
                }
            }
        }
    }
}
