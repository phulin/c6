//! Reached Linux XP2 Game.NameManager1 serialized used-name byte strings.
//! Preserves occurrence order, duplicates, empty strings and arbitrary bytes.
//! Native String initialization/archive state and name-selection gameplay are
//! not applied. This is distinct from Player.NameManager usage counts.

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
        write!(f, "Linux Game.NameManager wire: {self:?}")
    }
}
impl std::error::Error for Error {}
struct Reader<'a> {
    data: &'a [u8],
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NameManager {
    pub wire_version: u32,
    pub used_names: Vec<Vec<u8>>,
}
impl NameManager {
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
            limits,
        };
        let v = Self {
            wire_version: r.version(VERSION)?,
            used_names: Vec::read(&mut r)?,
        };
        Ok((v, r.pos, r.items))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &[u8] = &[
        1, 0, 0, 0, 4, 0, 0, 0, 3, 0, 0, 0, 0, 255, 128, 0, 0, 0, 0, 1, 0, 0, 0, 65, 3, 0, 0, 0, 0,
        255, 128,
    ];
    #[test]
    fn ordered_duplicates_empty_and_arbitrary_string_bytes() {
        let (v, n, i) = NameManager::decode_prefix_with_items(FIXTURE, Limits::default()).unwrap();
        assert_eq!((n, i), (31, 11));
        assert_eq!(
            v.used_names,
            vec![vec![0, 255, 128], vec![], vec![b'A'], vec![0, 255, 128]]
        );
    }
    #[test]
    fn cuts_budget_nested_counts_version_and_prefix_boundary() {
        for cut in 0..FIXTURE.len() {
            assert!(NameManager::decode(&FIXTURE[..cut], Limits::default()).is_err());
        }
        for cap in 0..FIXTURE.len() {
            assert_eq!(
                NameManager::decode(
                    FIXTURE,
                    Limits {
                        max_bytes: cap,
                        ..Limits::default()
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        for cap in 0..11 {
            assert_eq!(
                NameManager::decode(
                    FIXTURE,
                    Limits {
                        max_items: cap,
                        ..Limits::default()
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        for at in [4, 8] {
            let mut bad = FIXTURE.to_vec();
            bad[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert_eq!(
                NameManager::decode(&bad, Limits::default()),
                Err(Error::ResourceLimit)
            );
        }
        let mut bad = FIXTURE.to_vec();
        bad[..4].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            NameManager::decode(&bad, Limits::default()),
            Err(Error::Version {
                expected: 1,
                actual: 2
            })
        );
        let mut tail = FIXTURE.to_vec();
        tail.push(99);
        assert_eq!(
            NameManager::decode(&tail, Limits::default()),
            Err(Error::Trailing)
        );
        assert_eq!(
            NameManager::decode_prefix(&tail, Limits::default())
                .unwrap()
                .1,
            FIXTURE.len()
        );
    }
}
