//! Linux XP2 NationalParks1 / ParkData1 wire projection only.
//! Ordered occurrences, signed native ordinals/plot indices and name bytes are
//! retained; park creation, geometry validation and gameplay are unapplied.
//! Byte/item limits bound input and counted entries, not a separate heap cap.

pub use crate::unit::Limits;
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
        write!(f, "Linux Game.NationalParks wire: {self:?}")
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParkData {
    pub wire_version: u32,
    pub founder_ordinal: i32,
    pub turn_founded: i32,
    pub plot_indices: Vec<i32>,
    /// Native length-prefixed payload, without UTF-8/terminator assumptions.
    /// Empty wire payload does not establish an existing native String's value.
    pub name_bytes: Vec<u8>,
}
impl Wire for ParkData {
    const MIN: usize = 20;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: r.version(VERSION)?,
            founder_ordinal: i32::read(r)?,
            turn_founded: i32::read(r)?,
            plot_indices: Vec::read(r)?,
            name_bytes: Vec::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NationalParks {
    pub wire_version: u32,
    pub parks: Vec<ParkData>,
}
impl NationalParks {
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
        let v = Self {
            wire_version: r.version(VERSION)?,
            parks: Vec::read(&mut r)?,
        };
        Ok((v, r.pos, r.items))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn words(b: &mut Vec<u8>, v: &[u32]) {
        for n in v {
            b.extend(n.to_le_bytes());
        }
    }
    fn fixture() -> Vec<u8> {
        let mut b = Vec::new();
        words(&mut b, &[1, 2, 1, u32::MAX, 333, 3, 0, u32::MAX, 0, 3]);
        b.extend([b'P', 0, 255]);
        words(&mut b, &[1, u32::MAX, u32::MAX, 0, 0]);
        b
    }
    #[test]
    fn literal_order_signed_indices_and_name_bytes() {
        let b = fixture();
        assert_eq!(b.len(), 63);
        let (v, n, i) = NationalParks::decode_prefix_with_items(&b, Limits::default()).unwrap();
        assert_eq!((n, i), (63, 8));
        assert_eq!(v.parks.len(), 2);
        assert_eq!(
            (v.parks[0].founder_ordinal, v.parks[0].turn_founded),
            (-1, 333)
        );
        assert_eq!(v.parks[0].plot_indices, [0, -1, 0]);
        assert_eq!(v.parks[0].name_bytes, [b'P', 0, 255]);
        assert_eq!(v.parks[1].founder_ordinal, -1);
        assert_eq!(v.parks[1].turn_founded, -1);
        assert!(v.parks[1].plot_indices.is_empty() && v.parks[1].name_bytes.is_empty());
    }
    #[test]
    fn every_cut_limit_and_independent_version() {
        let b = fixture();
        for cut in 0..b.len() {
            assert!(
                NationalParks::decode(&b[..cut], Limits::default()).is_err(),
                "cut {cut}"
            );
        }
        for cap in 0..b.len() {
            assert_eq!(
                NationalParks::decode(
                    &b,
                    Limits {
                        max_bytes: cap,
                        ..Limits::default()
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        for cap in 0..8 {
            assert_eq!(
                NationalParks::decode(
                    &b,
                    Limits {
                        max_items: cap,
                        ..Limits::default()
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        for at in [0, 8, 43] {
            let mut bad = b.clone();
            bad[at..at + 4].copy_from_slice(&99u32.to_le_bytes());
            assert_eq!(
                NationalParks::decode(&bad, Limits::default()),
                Err(Error::Version {
                    expected: 1,
                    actual: 99
                })
            );
        }
        let mut tail = b.clone();
        tail.push(0);
        assert_eq!(
            NationalParks::decode(&tail, Limits::default()),
            Err(Error::Trailing)
        );
        assert_eq!(
            NationalParks::decode_prefix(&tail, Limits::default())
                .unwrap()
                .1,
            63
        );
        let mut hostile = b;
        hostile[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            NationalParks::decode(&hostile, Limits::default()),
            Err(Error::ResourceLimit)
        );
    }
}
