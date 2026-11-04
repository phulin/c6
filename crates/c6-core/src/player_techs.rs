//! Linux XP2 Player.Techs v5/v6 wire transport, including nested records.
//! No live Reset, definition remapping, callbacks, gameplay, actions or turns.
use crate::unit::Limits;

pub const VERSION: u32 = 6;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    UnsupportedLegacyWrite,
    Truncated,
    Trailing,
    ResourceLimit,
    InvalidBool(u8),
    Prerequisites(crate::player_prerequisites::Error),
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player.Techs wire: {self:?}")
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
    fn remaining(&self) -> Limits {
        Limits {
            max_bytes: self.limits.max_bytes - self.pos,
            max_items: self.limits.max_items - self.items,
        }
    }
    fn raw(&mut self, n: usize) -> Result<&[u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let b = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(b)
    }
    fn charge(&mut self, n: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
    fn allocate<T>(&mut self, n: usize) -> Result<(), Error> {
        // Check each allocation's addressable layout; wire bytes are a separate budget.
        let bytes = n
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(Error::ResourceLimit)?;
        if bytes > isize::MAX as usize {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
    fn version(&mut self) -> Result<u32, Error> {
        let actual = u32::read(self)?;
        if actual != 5 && actual != VERSION {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
        }
        Ok(actual)
    }
}
struct Writer {
    data: Vec<u8>,
    items: usize,
    limits: Limits,
}
impl Writer {
    fn raw(&mut self, b: &[u8]) -> Result<(), Error> {
        let end = self
            .data
            .len()
            .checked_add(b.len())
            .ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        self.data
            .try_reserve(b.len())
            .map_err(|_| Error::ResourceLimit)?;
        self.data.extend_from_slice(b);
        Ok(())
    }
    fn charge(&mut self, n: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
}
trait Wire: Sized {
    const MIN: usize;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error>;
    fn write(&self, w: &mut Writer) -> Result<(), Error>;
}
impl Wire for u32 {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self::from_le_bytes(r.raw(4)?.try_into().unwrap()))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.raw(&self.to_le_bytes())
    }
}
impl Wire for i32 {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(u32::read(r)? as Self)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        (*self as u32).write(w)
    }
}
impl<T: Wire> Wire for Vec<T> {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = u32::read(r)? as usize;
        r.charge(n)?;
        let needed = n.checked_mul(T::MIN).ok_or(Error::ResourceLimit)?;
        if needed > r.remaining().max_bytes {
            return Err(Error::ResourceLimit);
        }
        if needed > r.data.len().saturating_sub(r.pos) {
            return Err(Error::Truncated);
        }
        r.allocate::<T>(n)?;
        let mut v = Vec::new();
        v.try_reserve_exact(n).map_err(|_| Error::ResourceLimit)?;
        for _ in 0..n {
            v.push(T::read(r)?)
        }
        Ok(v)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.charge(self.len())?;
        u32::try_from(self.len())
            .map_err(|_| Error::ResourceLimit)?
            .write(w)?;
        for x in self {
            x.write(w)?
        }
        Ok(())
    }
}
impl Wire for bool {
    const MIN: usize = 1;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        match r.raw(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            x => Err(Error::InvalidBool(x)),
        }
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.raw(&[u8::from(*self)])
    }
}

use crate::player_prerequisites::IndexedBools;
use crate::unit::HashValue;
impl Wire for HashValue<i32> {
    const MIN: usize = 8;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            definition_hash: u32::read(r)?,
            value: i32::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.definition_hash.write(w)?;
        self.value.write(w)
    }
}
impl Wire for IndexedBools {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let start = r.pos;
        let count = u32::read(r)? as usize;
        r.charge(count)?;
        r.allocate::<HashValue<bool>>(count)?;
        let body = count.checked_mul(5).ok_or(Error::ResourceLimit)?;
        r.raw(body)?;
        // The prerequisite codec also checks storage against max_bytes. Wire bytes
        // were checked above; permit this separate checked allocation layout.
        let storage = count
            .checked_mul(std::mem::size_of::<HashValue<bool>>())
            .ok_or(Error::ResourceLimit)?;
        let (value, used) = Self::decode_prefix(
            &r.data[start..r.pos],
            Limits {
                max_bytes: storage.max(r.pos - start),
                max_items: count,
            },
        )
        .map_err(Error::Prerequisites)?;
        debug_assert_eq!(used, r.pos - start);
        Ok(value)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.charge(self.pairs.len())?;
        let bytes = self
            .encode(Limits {
                max_bytes: w.limits.max_bytes - w.data.len(),
                max_items: self.pairs.len(),
            })
            .map_err(Error::Prerequisites)?;
        w.raw(&bytes)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Techs {
    pub wire_version: u32,
    pub research_queue_hashes: Vec<u32>,
    pub research_overflow_q8: i32,
    pub science_rate_change_q8: i32,
    pub modified_boost: i32,
    pub has_tech: IndexedBools,
    pub boost_triggered: IndexedBools,
    pub research_progress_q8: Vec<HashValue<i32>>,
    pub num_random_tech_boosts_on_new_era: i32,
    pub num_techs_repeated: i32,
    pub free_tech_boost_wonder_era: i32,
}
impl Wire for Techs {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = r.version()?;
        Ok(Self {
            wire_version,
            research_queue_hashes: <Vec<u32> as Wire>::read(r)?,
            research_overflow_q8: <i32 as Wire>::read(r)?,
            science_rate_change_q8: <i32 as Wire>::read(r)?,
            modified_boost: <i32 as Wire>::read(r)?,
            has_tech: <IndexedBools as Wire>::read(r)?,
            boost_triggered: <IndexedBools as Wire>::read(r)?,
            research_progress_q8: <Vec<HashValue<i32>> as Wire>::read(r)?,
            num_random_tech_boosts_on_new_era: <i32 as Wire>::read(r)?,
            num_techs_repeated: <i32 as Wire>::read(r)?,
            free_tech_boost_wonder_era: if wire_version == 5 {
                0
            } else {
                <i32 as Wire>::read(r)?
            },
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != VERSION {
            return Err(Error::UnsupportedLegacyWrite);
        }
        VERSION.write(w)?;
        self.research_queue_hashes.write(w)?;
        self.research_overflow_q8.write(w)?;
        self.science_rate_change_q8.write(w)?;
        self.modified_boost.write(w)?;
        self.has_tech.write(w)?;
        self.boost_triggered.write(w)?;
        self.research_progress_q8.write(w)?;
        self.num_random_tech_boosts_on_new_era.write(w)?;
        self.num_techs_repeated.write(w)?;
        self.free_tech_boost_wonder_era.write(w)?;
        Ok(())
    }
}

impl Techs {
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
        let v = Self::read(&mut r)?;
        Ok((v, r.pos, r.items))
    }
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        let (v, n, _) = Self::decode_prefix_with_items(bytes, limits)?;
        Ok((v, n))
    }
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let (v, n) = Self::decode_prefix(bytes, limits)?;
        if n != bytes.len() {
            return Err(Error::Trailing);
        }
        Ok(v)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        let mut w = Writer {
            data: Vec::new(),
            items: 0,
            limits,
        };
        self.write(&mut w)?;
        Ok((w.data, w.items))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        Ok(self.encode_with_items(limits)?.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> Limits {
        Limits {
            max_bytes: 1 << 20,
            max_items: 10000,
        }
    }
    fn specimen() -> Techs {
        Techs {
            wire_version: VERSION,
            research_queue_hashes: vec![0, 0x8000_0001, u32::MAX, 0x8000_0001, 1],
            research_overflow_q8: i32::MIN,
            science_rate_change_q8: i32::MAX,
            modified_boost: -17,
            has_tech: IndexedBools {
                pairs: vec![
                    HashValue {
                        definition_hash: 0x8000_0001,
                        value: true,
                    },
                    HashValue {
                        definition_hash: 0x8000_0001,
                        value: false,
                    },
                ],
            },
            boost_triggered: IndexedBools {
                pairs: vec![HashValue {
                    definition_hash: 0,
                    value: true,
                }],
            },
            research_progress_q8: vec![
                HashValue {
                    definition_hash: u32::MAX,
                    value: i32::MIN,
                },
                HashValue {
                    definition_hash: u32::MAX,
                    value: i32::MAX,
                },
            ],
            num_random_tech_boosts_on_new_era: -2,
            num_techs_repeated: i32::MAX,
            free_tech_boost_wonder_era: -1,
        }
    }
    #[test]
    fn nonempty_roundtrip_prefix_and_trailing() {
        let v = specimen();
        let (b, items) = v.encode_with_items(limits()).unwrap();
        assert_eq!(items, 10);
        assert_eq!(
            Techs::decode_prefix_with_items(&b, limits()).unwrap(),
            (v.clone(), b.len(), items)
        );
        let mut ext = b.clone();
        ext.push(99);
        assert_eq!(Techs::decode(&ext, limits()), Err(Error::Trailing));
        assert_eq!(Techs::decode_prefix(&ext, limits()).unwrap(), (v, b.len()));
    }
    #[test]
    fn every_prefix_truncates() {
        let b = specimen().encode(limits()).unwrap();
        for n in 0..b.len() {
            assert_eq!(
                Techs::decode(&b[..n], limits()),
                Err(Error::Truncated),
                "prefix {n}"
            );
        }
    }
    #[test]
    fn exact_and_short_budgets() {
        let v = specimen();
        let (b, items) = v.encode_with_items(limits()).unwrap();
        let exact = Limits {
            max_bytes: b.len(),
            max_items: items,
        };
        assert_eq!(Techs::decode(&b, exact).unwrap(), v);
        assert_eq!(v.encode(exact).unwrap(), b);
        for low in [
            Limits {
                max_bytes: b.len() - 1,
                ..exact
            },
            Limits {
                max_items: items - 1,
                ..exact
            },
        ] {
            assert!(Techs::decode(&b, low).is_err());
            assert!(v.encode(low).is_err());
        }
    }
    #[test]
    fn version_bools_and_hostile_counts() {
        let mut b = specimen().encode(limits()).unwrap();
        b[0] = 4;
        assert!(matches!(
            Techs::decode(&b, limits()),
            Err(Error::Version { .. })
        ));
        b[0] = VERSION as u8;
        b[48] = 2;
        assert_eq!(
            Techs::decode(&b, limits()),
            Err(Error::Prerequisites(
                crate::player_prerequisites::Error::InvalidBool
            ))
        );
        let mut b = VERSION.to_le_bytes().to_vec();
        b.extend(u32::MAX.to_le_bytes());
        assert_eq!(Techs::decode(&b, limits()), Err(Error::ResourceLimit));
    }
    #[test]
    fn bool_storage_is_separate_from_wire_budget() {
        let mut v = specimen();
        v.has_tech.pairs = (0..200)
            .map(|definition_hash| HashValue {
                definition_hash,
                value: true,
            })
            .collect();
        let (b, items) = v.encode_with_items(limits()).unwrap();
        let exact = Limits {
            max_bytes: b.len(),
            max_items: items,
        };
        assert_eq!(Techs::decode(&b, exact).unwrap(), v);
    }
    #[test]
    fn legacy5_omits_only_final_scalar_and_preserves_budgets() {
        let current = specimen();
        let mut b = current.encode(limits()).unwrap();
        b[..4].copy_from_slice(&5u32.to_le_bytes());
        b.truncate(b.len() - 4);
        let (v, n, items) = Techs::decode_prefix_with_items(&b, limits()).unwrap();
        let mut expected = current;
        expected.wire_version = 5;
        expected.free_tech_boost_wonder_era = 0;
        assert_eq!(v, expected);
        assert_eq!(n, b.len());
        assert_eq!(v.encode(limits()), Err(Error::UnsupportedLegacyWrite));
        assert!(Techs::decode_prefix_with_items(
            &b,
            Limits {
                max_bytes: b.len(),
                max_items: items
            }
        )
        .is_ok());
        assert_eq!(
            Techs::decode_prefix_with_items(
                &b,
                Limits {
                    max_bytes: b.len() - 1,
                    max_items: items
                }
            ),
            Err(Error::ResourceLimit)
        );
        assert_eq!(
            Techs::decode_prefix_with_items(
                &b,
                Limits {
                    max_bytes: b.len(),
                    max_items: items - 1
                }
            ),
            Err(Error::ResourceLimit)
        );
        for end in 0..b.len() {
            assert!(Techs::decode(&b[..end], limits()).is_err(), "prefix {end}");
        }
        b.push(0);
        assert_eq!(Techs::decode(&b, limits()), Err(Error::Trailing));
    }
}
