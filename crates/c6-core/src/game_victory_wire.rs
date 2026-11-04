//! Reached Linux Game.VictoryManager3 saved-state transport only.
//! Preserves winning team, raw victory hash and ordered history tuple words.
//! Native type lookup, requirement release/rebuild, callbacks and gameplay
//! effects remain unapplied. The saved history hash remains independent even
//! though the target reader's history construction consults the outer hash.
pub use crate::unit::Limits;
pub const VERSION: u32 = 3;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux VictoryManager wire: {self:?}")
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
pub struct HistoryEntry {
    pub team: i32,
    pub victory_type_hash: u32,
    pub first_data: i32,
    pub second_data: i32,
}
impl Wire for HistoryEntry {
    const MIN: usize = 16;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            team: i32::read(r)?,
            victory_type_hash: u32::read(r)?,
            first_data: i32::read(r)?,
            second_data: i32::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Manager {
    pub wire_version: u32,
    pub winning_team: i32,
    pub winning_victory_type_hash: u32,
    pub history: Vec<HistoryEntry>,
}
impl Manager {
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let (v, n, _) = Self::decode_prefix_with_items(bytes, limits)?;
        if n != bytes.len() {
            return Err(Error::Trailing);
        }
        Ok(v)
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
            winning_team: i32::read(&mut r)?,
            winning_victory_type_hash: u32::read(&mut r)?,
            history: Vec::read(&mut r)?,
        };
        Ok((v, r.pos, r.items))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &[u8] = &[
        3, 0, 0, 0, 255, 255, 255, 255, 120, 86, 52, 18, 2, 0, 0, 0, 254, 255, 255, 255, 170, 170,
        170, 170, 0, 0, 0, 128, 255, 255, 255, 127, 63, 0, 0, 0, 187, 187, 187, 187, 255, 255, 255,
        255, 7, 0, 0, 0,
    ];
    #[test]
    fn saved_hashes_and_ordered_history_are_independent_of_native_resolution() {
        let limits = Limits {
            max_bytes: 48,
            max_items: 2,
        };
        let (v, n, i) = Manager::decode_prefix_with_items(FIXTURE, limits).unwrap();
        assert_eq!((n, i), (48, 2));
        assert_eq!(
            (v.winning_team, v.winning_victory_type_hash),
            (-1, 0x12345678)
        );
        assert_eq!(
            v.history,
            vec![
                HistoryEntry {
                    team: -2,
                    victory_type_hash: 0xaaaaaaaa,
                    first_data: i32::MIN,
                    second_data: i32::MAX
                },
                HistoryEntry {
                    team: 63,
                    victory_type_hash: 0xbbbbbbbb,
                    first_data: -1,
                    second_data: 7
                }
            ]
        );
        let mut b = FIXTURE.to_vec();
        b.extend([0x55, 0xaa]);
        assert_eq!(Manager::decode_prefix_with_items(&b, limits).unwrap().1, 48);
        assert_eq!(Manager::decode(&b, Limits::default()), Err(Error::Trailing));
    }
    #[test]
    fn all_cuts_budgets_versions_and_hostile_counts_are_bounded() {
        for cut in 0..48 {
            assert!(Manager::decode(&FIXTURE[..cut], Limits::default()).is_err());
            assert!(Manager::decode(
                FIXTURE,
                Limits {
                    max_bytes: cut,
                    max_items: 2
                }
            )
            .is_err());
        }
        for max_items in 0..2 {
            assert_eq!(
                Manager::decode(
                    FIXTURE,
                    Limits {
                        max_bytes: 48,
                        max_items
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        let mut b = FIXTURE.to_vec();
        b[..4].copy_from_slice(&99u32.to_le_bytes());
        assert_eq!(
            Manager::decode(&b, Limits::default()),
            Err(Error::Version {
                expected: 3,
                actual: 99
            })
        );
        b[..4].copy_from_slice(&3u32.to_le_bytes());
        b[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            Manager::decode(&b, Limits::default()),
            Err(Error::ResourceLimit)
        );
    }
}
