//! Linux XP2 global Game.Stats3/4 wire/default projection only.
//! Both source array paths explicitly read hash/value overlays; no inferred
//! IndexSerializer mode or SQL/native-index mapping is used. Native Reset sizes
//! dense zero baselines from definitions; those arrays/remaps and archive/event
//! state are unapplied here. Ordered duplicate/unknown keys and float bits remain.
//! Byte/item limits bound input and counted entries, not a separate heap cap.

pub use crate::unit::{HashValue, Limits};
pub const VERSION: u32 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Game.Stats wire: {self:?}")
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
        if !matches!(actual, 3 | VERSION) {
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stats {
    pub wire_version: u32,
    /// Overlay over a native definition-sized zero-reset array, not its dense
    /// initialized result. Empty wire pairs do not establish an empty array.
    pub units_constructed_overlay: Vec<HashValue<i32>>,
    pub wonders_built_overlay: Vec<HashValue<i32>>,
    pub total_alliances_formed: i32,
    pub total_emergency_alliances_formed: i32,
    pub total_free_cities: i32,
    pub smallest_emergency_alliance_size: i32,
    pub largest_emergency_alliance_size: i32,
    /// Raw IEEE754 wire bits; no float arithmetic or NaN normalization occurs.
    pub average_emergency_alliance_size_bits: u32,
    /// Wire since4; native Read explicitly assigns0 at3.
    pub num_units_killed: Option<i32>,
}
impl Stats {
    /// Actual unit/building definition counts are not serialized by this reset.
    pub const NATIVE_ARRAY_RESET_VALUE: i32 = 0;
    pub fn omitted_units_killed_native_default(&self) -> Option<i32> {
        (self.wire_version == 3).then_some(0)
    }
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let (v, n, _) = Self::decode_prefix_with_items(bytes, limits)?;
        if n != bytes.len() {
            return Err(Error::Trailing);
        }
        Ok(v)
    }
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        let (v, n, _) = Self::decode_prefix_with_items(bytes, limits)?;
        Ok((v, n))
    }
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
        let wire_version = r.version()?;
        let v = Self {
            wire_version,
            units_constructed_overlay: Vec::read(&mut r)?,
            wonders_built_overlay: Vec::read(&mut r)?,
            total_alliances_formed: i32::read(&mut r)?,
            total_emergency_alliances_formed: i32::read(&mut r)?,
            total_free_cities: i32::read(&mut r)?,
            smallest_emergency_alliance_size: i32::read(&mut r)?,
            largest_emergency_alliance_size: i32::read(&mut r)?,
            average_emergency_alliance_size_bits: u32::read(&mut r)?,
            num_units_killed: if wire_version == VERSION {
                Some(i32::read(&mut r)?)
            } else {
                None
            },
        };
        Ok((v, r.pos, r.items))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(tag: u32) -> Vec<u8> {
        let mut b = Vec::new();
        for n in [
            tag,
            2,
            0x12345678,
            u32::MAX,
            0x12345678,
            2,
            1,
            0xfedcba98,
            u32::MAX - 2,
            11,
            12,
            13,
            14,
            15,
            0x7fc01234,
        ] {
            b.extend(n.to_le_bytes());
        }
        if tag == 4 {
            b.extend(77u32.to_le_bytes());
        }
        b
    }
    #[test]
    fn literal_overlays_order_float_bits_and_omission() {
        for tag in [3, 4] {
            let b = fixture(tag);
            let size = if tag == 3 { 60 } else { 64 };
            assert_eq!(b.len(), size);
            let (s, n, i) = Stats::decode_prefix_with_items(&b, Limits::default()).unwrap();
            assert_eq!((n, i), (size, 3));
            assert_eq!(
                s.units_constructed_overlay,
                [
                    HashValue {
                        definition_hash: 0x12345678,
                        value: -1
                    },
                    HashValue {
                        definition_hash: 0x12345678,
                        value: 2
                    }
                ]
            );
            assert_eq!(
                s.wonders_built_overlay,
                [HashValue {
                    definition_hash: 0xfedcba98,
                    value: -3
                }]
            );
            assert_eq!(
                [
                    s.total_alliances_formed,
                    s.total_emergency_alliances_formed,
                    s.total_free_cities,
                    s.smallest_emergency_alliance_size,
                    s.largest_emergency_alliance_size
                ],
                [11, 12, 13, 14, 15]
            );
            assert_eq!(s.average_emergency_alliance_size_bits, 0x7fc01234);
            assert_eq!(s.num_units_killed, (tag == 4).then_some(77));
            assert_eq!(
                s.omitted_units_killed_native_default(),
                (tag == 3).then_some(0)
            );
        }
        let mut b = fixture(3);
        b[56..60].copy_from_slice(&0x80000000u32.to_le_bytes());
        assert_eq!(
            Stats::decode(&b, Limits::default())
                .unwrap()
                .average_emergency_alliance_size_bits,
            0x80000000
        );
    }
    #[test]
    fn cuts_limits_tags_and_strict_tail() {
        for tag in [3, 4] {
            let b = fixture(tag);
            for cut in 0..b.len() {
                assert!(
                    Stats::decode(&b[..cut], Limits::default()).is_err(),
                    "tag {tag} cut {cut}"
                );
            }
            for cap in 0..b.len() {
                assert_eq!(
                    Stats::decode(
                        &b,
                        Limits {
                            max_bytes: cap,
                            ..Limits::default()
                        }
                    ),
                    Err(Error::ResourceLimit)
                );
            }
            for cap in 0..3 {
                assert_eq!(
                    Stats::decode(
                        &b,
                        Limits {
                            max_items: cap,
                            ..Limits::default()
                        }
                    ),
                    Err(Error::ResourceLimit)
                );
            }
            let mut tail = b.clone();
            tail.push(0);
            assert_eq!(
                Stats::decode(&tail, Limits::default()),
                Err(Error::Trailing)
            );
            assert_eq!(
                Stats::decode_prefix(&tail, Limits::default()).unwrap().1,
                b.len()
            );
            let mut hostile = b;
            hostile[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
            assert_eq!(
                Stats::decode(&hostile, Limits::default()),
                Err(Error::ResourceLimit)
            );
        }
        assert_eq!(
            Stats::decode(&2u32.to_le_bytes(), Limits::default()),
            Err(Error::Version {
                expected: 4,
                actual: 2
            })
        );
    }
}
