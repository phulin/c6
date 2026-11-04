//! Reached Linux XP2 Fallout2 / Instance2 wire projection only.
//! Plot-vector order, signed remaining turns and raw prevent-work flags survive.
//! Native map/event installation and fallout gameplay are not applied. Reactor
//! list (since3) is absent2; damage override (since4) has explicit native -1.

pub use crate::unit::Limits;
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
        write!(f, "Linux Game.Fallout wire: {self:?}")
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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OmittedPriorState {
    Unknown,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Instance {
    pub wire_version: u32,
    pub turns_remaining: i32,
    pub prevent_work_raw: u8,
}
impl Wire for Instance {
    const MIN: usize = 9;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: r.version(VERSION)?,
            turns_remaining: i32::read(r)?,
            prevent_work_raw: u8::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Manager {
    pub wire_version: u32,
    pub plots: Vec<Instance>,
    pub nuclear_reactors: OmittedPriorState,
    pub damage_override_read_default: i32,
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
            plots: Vec::read(&mut r)?,
            nuclear_reactors: OmittedPriorState::Unknown,
            damage_override_read_default: -1,
        };
        Ok((v, r.pos, r.items))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &[u8] = &[
        2, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 255, 255, 255, 255, 255, 2, 0, 0, 0, 7, 0, 0, 0, 2,
    ];
    #[test]
    fn independent_record_tags_raw_flags_signed_turns_and_omissions() {
        let (v, n, i) = Manager::decode_prefix_with_items(FIXTURE, Limits::default()).unwrap();
        assert_eq!((n, i), (26, 2));
        assert_eq!(
            v.plots,
            vec![
                Instance {
                    wire_version: 2,
                    turns_remaining: -1,
                    prevent_work_raw: 255
                },
                Instance {
                    wire_version: 2,
                    turns_remaining: 7,
                    prevent_work_raw: 2
                }
            ]
        );
        assert_eq!(v.nuclear_reactors, OmittedPriorState::Unknown);
        assert_eq!(v.damage_override_read_default, -1);
    }
    #[test]
    fn all_cuts_caps_wrong_tags_hostile_count_and_tail() {
        for cut in 0..FIXTURE.len() {
            assert!(Manager::decode(&FIXTURE[..cut], Limits::default()).is_err());
        }
        for cap in 0..FIXTURE.len() {
            assert_eq!(
                Manager::decode(
                    FIXTURE,
                    Limits {
                        max_bytes: cap,
                        ..Limits::default()
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        for cap in 0..2 {
            assert_eq!(
                Manager::decode(
                    FIXTURE,
                    Limits {
                        max_items: cap,
                        ..Limits::default()
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        for at in [0, 8, 17] {
            let mut bad = FIXTURE.to_vec();
            bad[at..at + 4].copy_from_slice(&3u32.to_le_bytes());
            assert_eq!(
                Manager::decode(&bad, Limits::default()),
                Err(Error::Version {
                    expected: 2,
                    actual: 3
                })
            );
        }
        let mut bad = FIXTURE.to_vec();
        bad[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            Manager::decode(&bad, Limits::default()),
            Err(Error::ResourceLimit)
        );
        let mut tail = FIXTURE.to_vec();
        tail.push(99);
        assert_eq!(
            Manager::decode(&tail, Limits::default()),
            Err(Error::Trailing)
        );
        assert_eq!(
            Manager::decode_prefix_with_items(&tail, Limits::default())
                .unwrap()
                .1,
            FIXTURE.len()
        );
    }
}
