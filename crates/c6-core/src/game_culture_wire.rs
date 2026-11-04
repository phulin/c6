//! Reached Linux XP2 Culture6 / CivicNode6 / GreatWork6 wire projection.
//! Preserves ordered hashes, prerequisite occurrences, signed Q8 raw costs,
//! player ordinals, creation turns, raw flags and byte names. Native civic
//! clearing/validation, definition remaps, archive/events and gameplay are absent.

pub use crate::unit::Limits;
pub const VERSION: u32 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Game.Culture wire: {self:?}")
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
pub struct CivicNode {
    pub wire_version: u32,
    pub civic_type_hash: u32,
    pub active_raw: u8,
    pub cost_q8_raw: i32,
    pub prerequisite_civic_hashes: Vec<u32>,
}
impl Wire for CivicNode {
    const MIN: usize = 17;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: r.version(VERSION)?,
            civic_type_hash: u32::read(r)?,
            active_raw: u8::read(r)?,
            cost_q8_raw: i32::read(r)?,
            prerequisite_civic_hashes: Vec::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GreatWork {
    pub wire_version: u32,
    pub great_work_type_hash: u32,
    pub player_ordinal: i32,
    pub turn_created: i32,
    pub great_person_name: Vec<u8>,
}
impl Wire for GreatWork {
    const MIN: usize = 20;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: r.version(VERSION)?,
            great_work_type_hash: u32::read(r)?,
            player_ordinal: i32::read(r)?,
            turn_created: i32::read(r)?,
            great_person_name: Vec::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Culture {
    pub wire_version: u32,
    pub civic_nodes: Vec<CivicNode>,
    pub active_civic_hashes: Vec<u32>,
    pub great_works: Vec<GreatWork>,
}
impl Culture {
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
            civic_nodes: Vec::read(&mut r)?,
            active_civic_hashes: Vec::read(&mut r)?,
            great_works: Vec::read(&mut r)?,
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
        words(&mut b, &[6, 2, 6, 0x81234567]);
        b.push(255);
        words(
            &mut b,
            &[(-256i32) as u32, 2, 0xabcdef01, 0xabcdef01, 6, 0x87654321],
        );
        b.push(2);
        words(
            &mut b,
            &[
                i32::MAX as u32,
                0,
                2,
                0x81234567,
                0x87654321,
                2,
                6,
                0xfedcba98,
                u32::MAX,
                3,
                3,
            ],
        );
        b.extend([0, 255, 128]);
        words(&mut b, &[6, 0xfedcba98, 63, u32::MAX, 0]);
        b
    }
    #[test]
    fn literal_independent_nodes_work_order_q8_bits_and_binary_names() {
        let b = fixture();
        assert_eq!(b.len(), 109);
        let (v, n, i) = Culture::decode_prefix_with_items(&b, Limits::default()).unwrap();
        assert_eq!((n, i), (109, 11));
        assert_eq!(
            v.civic_nodes[0],
            CivicNode {
                wire_version: 6,
                civic_type_hash: 0x81234567,
                active_raw: 255,
                cost_q8_raw: -256,
                prerequisite_civic_hashes: vec![0xabcdef01, 0xabcdef01]
            }
        );
        assert_eq!(v.civic_nodes[1].cost_q8_raw, i32::MAX);
        assert_eq!(v.civic_nodes[1].active_raw, 2);
        assert_eq!(v.active_civic_hashes, vec![0x81234567, 0x87654321]);
        assert_eq!(
            v.great_works[0],
            GreatWork {
                wire_version: 6,
                great_work_type_hash: 0xfedcba98,
                player_ordinal: -1,
                turn_created: 3,
                great_person_name: vec![0, 255, 128]
            }
        );
        assert_eq!(
            (
                v.great_works[1].player_ordinal,
                v.great_works[1].turn_created
            ),
            (63, -1)
        );
        assert!(v.great_works[1].great_person_name.is_empty());
    }
    #[test]
    fn cuts_caps_nested_counts_independent_versions_and_tail() {
        let b = fixture();
        for cut in 0..b.len() {
            assert!(
                Culture::decode(&b[..cut], Limits::default()).is_err(),
                "cut {cut}"
            );
        }
        for cap in 0..b.len() {
            assert_eq!(
                Culture::decode(
                    &b,
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
                Culture::decode(
                    &b,
                    Limits {
                        max_items: cap,
                        ..Limits::default()
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        for at in [0, 8, 33, 66, 89] {
            let mut bad = b.clone();
            bad[at..at + 4].copy_from_slice(&3u32.to_le_bytes());
            assert_eq!(
                Culture::decode(&bad, Limits::default()),
                Err(Error::Version {
                    expected: 6,
                    actual: 3
                })
            );
        }
        for at in [4, 21, 50, 62, 82] {
            let mut bad = b.clone();
            bad[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert_eq!(
                Culture::decode(&bad, Limits::default()),
                Err(Error::ResourceLimit)
            );
        }
        let mut tail = b.clone();
        tail.push(0);
        assert_eq!(
            Culture::decode(&tail, Limits::default()),
            Err(Error::Trailing)
        );
        assert_eq!(
            Culture::decode_prefix_with_items(&tail, Limits::default())
                .unwrap()
                .1,
            b.len()
        );
    }
}
