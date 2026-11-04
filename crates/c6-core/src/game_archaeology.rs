//! Linux XP2 Archaeology2 / ArchaeologicalRecord2 wire projection only.
//! Record occurrence order, raw artifact types, coordinates, player ordinals,
//! Era hashes and flag bytes are retained. Native Era remaps, temporary maps,
//! player-count caches, site generation/RNG and gameplay remain unapplied.
//! Byte/item limits bound input and counted entries, not a separate heap cap.

pub use crate::unit::{Limits, PlotCoord};
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
        write!(f, "Linux Game.Archaeology wire: {self:?}")
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
pub struct ArchaeologicalRecord {
    pub wire_version: u32,
    pub artifact_type_raw: i32,
    pub action_plot: PlotCoord,
    pub acting_player_ordinal: i32,
    pub acting_player_era_hash: u32,
    pub target_player_ordinal: i32,
    pub target_player_era_hash: u32,
}
impl Wire for ArchaeologicalRecord {
    const MIN: usize = 32;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: r.version(VERSION)?,
            artifact_type_raw: i32::read(r)?,
            action_plot: PlotCoord::read(r)?,
            acting_player_ordinal: i32::read(r)?,
            acting_player_era_hash: u32::read(r)?,
            target_player_ordinal: i32::read(r)?,
            target_player_era_hash: u32::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Archaeology {
    pub wire_version: u32,
    pub records: Vec<ArchaeologicalRecord>,
    pub land_sites_generated_raw: u8,
    pub sea_sites_generated_raw: u8,
}
impl Archaeology {
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
            records: Vec::read(&mut r)?,
            land_sites_generated_raw: u8::read(&mut r)?,
            sea_sites_generated_raw: u8::read(&mut r)?,
        };
        Ok((v, r.pos, r.items))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let mut b = Vec::new();
        for n in [
            2,
            2,
            2,
            3,
            105,
            29,
            15,
            0x81234567,
            63,
            0xabcdef01,
            2,
            u32::MAX,
            (-9999i32) as u32,
            7,
            15,
            0x87654321,
            u32::MAX,
            0xfedcba98,
        ] {
            b.extend(n.to_le_bytes());
        }
        b.extend([2, 255]);
        b
    }
    #[test]
    fn literal_records_preserve_role_order_and_raw_flags() {
        let b = fixture();
        assert_eq!(b.len(), 74);
        let (a, n, i) = Archaeology::decode_prefix_with_items(&b, Limits::default()).unwrap();
        assert_eq!((n, i), (74, 2));
        assert_eq!(
            a.records[0],
            ArchaeologicalRecord {
                wire_version: 2,
                artifact_type_raw: 3,
                action_plot: PlotCoord { x: 105, y: 29 },
                acting_player_ordinal: 15,
                acting_player_era_hash: 0x81234567,
                target_player_ordinal: 63,
                target_player_era_hash: 0xabcdef01
            }
        );
        assert_eq!(a.records[1].artifact_type_raw, -1);
        assert_eq!(a.records[1].action_plot.x, -9999);
        assert_eq!(a.records[1].acting_player_ordinal, 15);
        assert_eq!(a.records[1].target_player_ordinal, -1);
        assert_eq!(a.records[1].acting_player_era_hash, 0x87654321);
        assert_eq!(a.records[1].target_player_era_hash, 0xfedcba98);
        assert_eq!(
            (a.land_sites_generated_raw, a.sea_sites_generated_raw),
            (2, 255)
        );
    }
    #[test]
    fn cuts_limits_independent_tags_and_strict_tail() {
        let b = fixture();
        for cut in 0..b.len() {
            assert!(
                Archaeology::decode(&b[..cut], Limits::default()).is_err(),
                "cut {cut}"
            );
        }
        for cap in 0..b.len() {
            assert_eq!(
                Archaeology::decode(
                    &b,
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
                Archaeology::decode(
                    &b,
                    Limits {
                        max_items: cap,
                        ..Limits::default()
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        for at in [0, 8, 40] {
            let mut bad = b.clone();
            bad[at..at + 4].copy_from_slice(&1u32.to_le_bytes());
            assert_eq!(
                Archaeology::decode(&bad, Limits::default()),
                Err(Error::Version {
                    expected: 2,
                    actual: 1
                })
            );
        }
        let mut tail = b.clone();
        tail.push(0);
        assert_eq!(
            Archaeology::decode(&tail, Limits::default()),
            Err(Error::Trailing)
        );
        assert_eq!(
            Archaeology::decode_prefix(&tail, Limits::default())
                .unwrap()
                .1,
            74
        );
        let mut hostile = b;
        hostile[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            Archaeology::decode(&hostile, Limits::default()),
            Err(Error::ResourceLimit)
        );
    }
}
