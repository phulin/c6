//! Linux XP2 Emergency Manager3 / Instance10 / Signal1 wire projection.
//! Ordered occurrences, raw boolean bytes and definition hashes are retained.
//! Definition remaps, initialized objects, listeners and gameplay are unapplied.
//! Byte/item limits bound input and counted entries, not a separate heap cap.

pub use crate::unit::Limits;
pub const VERSION: u32 = 3;
pub const INSTANCE_VERSION: u32 = 10;
pub const SIGNAL_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Game.Emergency wire: {self:?}")
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
/// Native length-prefixed bytes; no UTF-8, terminator or normalization assumed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ByteString(pub Vec<u8>);
impl Wire for ByteString {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self(Vec::<u8>::read(r)?))
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Instance {
    pub wire_version: u32,
    pub target_ordinal: i32,
    pub emergency_hash: u32,
    pub starting_turn: i32,
    pub ending_turn: i32,
    pub complete_raw: u8,
    pub expired_raw: u8,
    pub members: Vec<i32>,
    pub data: Vec<u32>,
    pub success_raw: Vec<u8>,
    pub data_type_raw: u32,
    pub invitees_looked_raw: Vec<u8>,
    pub string_data: Vec<ByteString>,
    pub force_target_win_raw: u8,
    /// Serialized vector replaces the native pre-read 64-zero scratch default.
    /// No fixed length or player/index association is inferred from its count.
    pub player_scores: Vec<i32>,
}
impl Wire for Instance {
    const MIN: usize = 51;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: r.version(INSTANCE_VERSION)?,
            target_ordinal: i32::read(r)?,
            emergency_hash: u32::read(r)?,
            starting_turn: i32::read(r)?,
            ending_turn: i32::read(r)?,
            complete_raw: u8::read(r)?,
            expired_raw: u8::read(r)?,
            members: Vec::read(r)?,
            data: Vec::read(r)?,
            success_raw: Vec::read(r)?,
            data_type_raw: u32::read(r)?,
            invitees_looked_raw: Vec::read(r)?,
            string_data: Vec::read(r)?,
            force_target_win_raw: u8::read(r)?,
            player_scores: Vec::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Signal {
    pub wire_version: u32,
    pub player_ordinal: i32,
    pub emergency_hash: u32,
    pub target_ordinal: i32,
    pub starting_turn: i32,
    pub event_ordinal: i32,
}
impl Wire for Signal {
    const MIN: usize = 24;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: r.version(SIGNAL_VERSION)?,
            player_ordinal: i32::read(r)?,
            emergency_hash: u32::read(r)?,
            target_ordinal: i32::read(r)?,
            starting_turn: i32::read(r)?,
            event_ordinal: i32::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Manager {
    pub wire_version: u32,
    pub emergencies: Vec<Instance>,
    pub signals: Vec<Signal>,
}
impl Manager {
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let (value, n, _) = Self::decode_prefix_with_items(bytes, limits)?;
        if n != bytes.len() {
            return Err(Error::Trailing);
        }
        Ok(value)
    }
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        let (value, n, _) = Self::decode_prefix_with_items(bytes, limits)?;
        Ok((value, n))
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
        let value = Self {
            wire_version: r.version(VERSION)?,
            emergencies: Vec::read(&mut r)?,
            signals: Vec::read(&mut r)?,
        };
        Ok((value, r.pos, r.items))
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
        words(&mut b, &[3, 1, 10, u32::MAX, 0xf1234567, 142, 172]);
        b.extend([2, 255]);
        words(&mut b, &[2, 7, u32::MAX, 1, 0xabcdef12, 2]);
        b.extend([0, 3]);
        words(&mut b, &[0x98765432, 3]);
        b.extend([1, 2, 255]);
        words(&mut b, &[2, 3]);
        b.extend([b'A', 0, 255]);
        words(&mut b, &[0]);
        b.push(128);
        words(
            &mut b,
            &[2, 0, u32::MAX, 1, 1, 9, 0x87654321, 4, 90, u32::MAX],
        );
        b
    }
    #[test]
    fn literal_current_order_and_complete_accounting() {
        let b = fixture();
        assert_eq!(b.len(), 123);
        let (m, n, items) = Manager::decode_prefix_with_items(&b, Limits::default()).unwrap();
        assert_eq!((n, items), (123, 17));
        let v = &m.emergencies[0];
        assert_eq!(
            (
                v.target_ordinal,
                v.emergency_hash,
                v.starting_turn,
                v.ending_turn
            ),
            (-1, 0xf1234567, 142, 172)
        );
        assert_eq!(
            (v.complete_raw, v.expired_raw, v.force_target_win_raw),
            (2, 255, 128)
        );
        assert_eq!(v.members, [7, -1]);
        assert_eq!(v.data, [0xabcdef12]);
        assert_eq!(v.success_raw, [0, 3]);
        assert_eq!(v.data_type_raw, 0x98765432);
        assert_eq!(v.invitees_looked_raw, [1, 2, 255]);
        assert_eq!(
            v.string_data,
            [ByteString(vec![b'A', 0, 255]), ByteString(vec![])]
        );
        assert_eq!(v.player_scores, [0, -1]);
        assert_eq!(
            m.signals,
            [Signal {
                wire_version: 1,
                player_ordinal: 9,
                emergency_hash: 0x87654321,
                target_ordinal: 4,
                starting_turn: 90,
                event_ordinal: -1
            }]
        );
    }
    #[test]
    fn cuts_limits_trailing_and_independent_versions() {
        let b = fixture();
        for cut in 0..b.len() {
            assert!(
                Manager::decode(&b[..cut], Limits::default()).is_err(),
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
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        for cap in 0..17 {
            assert_eq!(
                Manager::decode(
                    &b,
                    Limits {
                        max_items: cap,
                        ..Limits::default()
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        for (at, expected) in [(0, 3), (8, 10), (99, 1)] {
            let mut bad = b.clone();
            bad[at..at + 4].copy_from_slice(&99u32.to_le_bytes());
            assert_eq!(
                Manager::decode(&bad, Limits::default()),
                Err(Error::Version {
                    expected,
                    actual: 99
                })
            );
        }
        let mut tail = b.clone();
        tail.push(0);
        assert_eq!(
            Manager::decode(&tail, Limits::default()),
            Err(Error::Trailing)
        );
        assert_eq!(
            Manager::decode_prefix(&tail, Limits::default()).unwrap().1,
            123
        );
        let mut hostile = b;
        hostile[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            Manager::decode(&hostile, Limits::default()),
            Err(Error::ResourceLimit)
        );
    }
    #[test]
    fn empty_manager_has_no_invented_runtime_defaults() {
        let b = [3u32.to_le_bytes(), 0u32.to_le_bytes(), 0u32.to_le_bytes()].concat();
        let (m, n, i) = Manager::decode_prefix_with_items(&b, Limits::default()).unwrap();
        assert_eq!((n, i), (12, 0));
        assert!(m.emergencies.is_empty() && m.signals.is_empty());
    }
}
