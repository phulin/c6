//! Linux XP2 Game.History2 / Moment8 wire projection only.
//! Raw type/data keys, ordered occurrences, names and independent counters are
//! retained. Native definition remaps, initialized strings, recording callbacks
//! and gameplay are unapplied. Moment8 skips the older definition-score fixup.
//! Byte/item limits bound input and counted entries, not a separate heap cap.

pub use crate::unit::{Limits, PlotCoord};
pub const VERSION: u32 = 2;
pub const MOMENT_VERSION: u32 = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Game.History wire: {self:?}")
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
impl Wire for PlotCoord {
    const MIN: usize = 8;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            x: i32::read(r)?,
            y: i32::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MomentData {
    /// Raw MomentDataTypes word; no definition/index conversion is inferred.
    pub kind_raw: u32,
    pub value: i32,
}
impl Wire for MomentData {
    const MIN: usize = 8;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            kind_raw: u32::read(r)?,
            value: i32::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Moment {
    pub wire_version: u32,
    /// Raw MomentTypes m_eTypeHash word, read directly without index remapping.
    pub type_hash: u32,
    pub turn: i32,
    pub action_plot: PlotCoord,
    pub acting_player_ordinal: i32,
    pub id: i32,
    pub has_ever_been_commemorated_raw: u8,
    pub game_era_hash: u32,
    pub extra_data: Vec<MomentData>,
    /// Encoding0 bytes. Empty payload does not establish a prior native String.
    pub city_name_bytes: Vec<u8>,
    /// Encoding1 still reads a byte count/char payload, without transcoding here.
    pub unit_citizen_name_bytes: Vec<u8>,
    /// Serialized after UnitCitizenName, despite its earlier object offset.
    pub other_city_name_bytes: Vec<u8>,
    pub era_score: i32,
}
impl Wire for Moment {
    const MIN: usize = 53;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: r.version(MOMENT_VERSION)?,
            type_hash: u32::read(r)?,
            turn: i32::read(r)?,
            action_plot: PlotCoord::read(r)?,
            acting_player_ordinal: i32::read(r)?,
            id: i32::read(r)?,
            has_ever_been_commemorated_raw: u8::read(r)?,
            game_era_hash: u32::read(r)?,
            extra_data: Vec::read(r)?,
            city_name_bytes: Vec::read(r)?,
            unit_citizen_name_bytes: Vec::read(r)?,
            other_city_name_bytes: Vec::read(r)?,
            era_score: i32::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Manager {
    pub wire_version: u32,
    pub moments: Vec<Moment>,
    /// Independent serialized counter, not inferred from list length or IDs.
    pub total_moments_recorded: i32,
}
impl Manager {
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
            moments: Vec::read(&mut r)?,
            total_moments_recorded: i32::read(&mut r)?,
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
        words(
            &mut b,
            &[
                2,
                1,
                8,
                0xfb65667e,
                11,
                (-9999i32) as u32,
                (-9999i32) as u32,
                1,
                7,
            ],
        );
        b.push(2);
        words(
            &mut b,
            &[0xfaceb00c, 2, 0x12345678, u32::MAX, 0x12345678, 123, 1],
        );
        b.push(b'C');
        words(&mut b, &[2]);
        b.extend([0xc3, 0xa9]);
        words(&mut b, &[3]);
        b.extend([b'O', 0, 255]);
        words(&mut b, &[(-4i32) as u32, 999]);
        b
    }
    #[test]
    fn literal_names_wire_order_keys_and_independent_counter() {
        let b = fixture();
        assert_eq!(b.len(), 87);
        let (m, n, i) = Manager::decode_prefix_with_items(&b, Limits::default()).unwrap();
        assert_eq!((n, i), (87, 9));
        assert_eq!(m.total_moments_recorded, 999);
        let v = &m.moments[0];
        assert_eq!(
            (v.type_hash, v.turn, v.acting_player_ordinal, v.id),
            (0xfb65667e, 11, 1, 7)
        );
        assert_eq!(v.action_plot, PlotCoord { x: -9999, y: -9999 });
        assert_eq!(v.has_ever_been_commemorated_raw, 2);
        assert_eq!(v.game_era_hash, 0xfaceb00c);
        assert_eq!(
            v.extra_data,
            [
                MomentData {
                    kind_raw: 0x12345678,
                    value: -1
                },
                MomentData {
                    kind_raw: 0x12345678,
                    value: 123
                }
            ]
        );
        assert_eq!(v.city_name_bytes, [b'C']);
        assert_eq!(v.unit_citizen_name_bytes, [0xc3, 0xa9]);
        assert_eq!(v.other_city_name_bytes, [b'O', 0, 255]);
        assert_eq!(v.era_score, -4);
    }
    #[test]
    fn cuts_limits_independent_tags_and_no_legacy_score_inference() {
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
        for cap in 0..9 {
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
        for (at, tag, expected) in [(0, 1, 2), (8, 7, 8)] {
            let mut bad = b.clone();
            bad[at..at + 4].copy_from_slice(&u32::to_le_bytes(tag));
            assert_eq!(
                Manager::decode(&bad, Limits::default()),
                Err(Error::Version {
                    expected,
                    actual: tag
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
            87
        );
        let mut hostile = b;
        hostile[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            Manager::decode(&hostile, Limits::default()),
            Err(Error::ResourceLimit)
        );
    }
}
