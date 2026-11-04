//! Linux XP2 Climate14 wire and explicit read-default projection only.
//! Legacy Resource/RandomEvent values are native ordinals here, not hashes.
//! Definition remaps, archive side effects and PostUnserialize/InitializeLists
//! are unapplied. Omitted self-defaulted collections retain unknown prior state.
//! Byte/item limits bound input and counted entries, not a separate heap cap.

pub use crate::unit::Limits;
pub const WIRE_VERSION: u32 = 14;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Game.Climate wire: {self:?}")
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
pub struct ResourceConsumed {
    pub wire_version: u32,
    pub player_ordinal: i32,
    /// Pre16 EnumFromStream stores the signed runtime ordinal directly.
    pub resource_ordinal: i32,
    /// Pre23 wire stores an integer, not Q8 raw bits.
    pub amount_integer: i32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceReadValues {
    pub amount_q8: i32,
    pub last_turn_amount_q8: i32,
    pub raw_amount_q8: i32,
    pub raw_last_turn_amount_q8: i32,
}
impl ResourceConsumed {
    /// Native assignment shifts left8 with 32-bit wrapping. At14 the last-turn
    /// amount is explicitly assigned integer0; both raw fields copy these values.
    pub fn read_values(&self) -> ResourceReadValues {
        let amount = self.amount_integer.wrapping_shl(8);
        ResourceReadValues {
            amount_q8: amount,
            last_turn_amount_q8: 0,
            raw_amount_q8: amount,
            raw_last_turn_amount_q8: 0,
        }
    }
}
impl Wire for ResourceConsumed {
    const MIN: usize = 16;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: r.version(WIRE_VERSION)?,
            player_ordinal: i32::read(r)?,
            resource_ordinal: i32::read(r)?,
            amount_integer: i32::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CarbonSequestered {
    pub wire_version: u32,
    pub player_ordinal: i32,
    pub amount: i32,
}
impl Wire for CarbonSequestered {
    const MIN: usize = 12;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: r.version(WIRE_VERSION)?,
            player_ordinal: i32::read(r)?,
            amount: i32::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Storm {
    pub wire_version: u32,
    pub random_event_ordinal: i32,
    pub severity_ordinal: i32,
    pub active_raw: u8,
    pub start_turn: i32,
    pub current_plot: i32,
    pub current_direction_ordinal: i32,
    pub turn_end_plots: Vec<i32>,
    pub plots_affected: Vec<i32>,
    /// Native length-prefixed bytes, including any NUL/non-UTF8 payload.
    pub name_bytes: Vec<u8>,
}
impl Storm {
    /// Start plot is absent before26 and native Read explicitly assigns-1.
    pub const OMITTED_START_PLOT: i32 = -1;
}
impl Wire for Storm {
    const MIN: usize = 37;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: r.version(WIRE_VERSION)?,
            random_event_ordinal: i32::read(r)?,
            severity_ordinal: i32::read(r)?,
            active_raw: u8::read(r)?,
            start_turn: i32::read(r)?,
            current_plot: i32::read(r)?,
            current_direction_ordinal: i32::read(r)?,
            turn_end_plots: Vec::read(r)?,
            plots_affected: Vec::read(r)?,
            name_bytes: Vec::read(r)?,
        })
    }
}
/// Explicit scalar assignments made by native Read without consuming wire bytes
/// at14. This does not run archive callbacks or reconstruct the native Manager.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OmittedReadDefaults {
    pub next_ice_loss_turns: i32,
    pub next_sea_level_rise_turns: i32,
    pub halt_flood_fertility_raw: u8,
    pub halt_storm_fertility_raw: u8,
    pub fertility_removal_chance: i32,
    pub last_turn_worst_co2_footprint: i32,
    pub last_turn_average_co2_footprint: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Manager {
    pub wire_version: u32,
    pub resources_expended: Vec<ResourceConsumed>,
    pub carbon_sequestered: Vec<CarbonSequestered>,
    pub climate_change_for_last_sea_level_event: i32,
    pub severity_for_last_sea_level_event: i32,
    pub tiles_flooded: i32,
    pub tiles_submerged: i32,
    pub total_forested_tiles: i32,
    pub turn_first_resource_consumed: i32,
    pub cumulative_deforestation_points: i32,
    pub co2_footprint_modifier: i32,
    /// This field already uses definition-hash serialization at14.
    pub deforestation_hash: u32,
    pub this_turn_co2_footprint: Vec<i32>,
    pub last_turn_co2_footprint: Vec<i32>,
    pub storms: Vec<Storm>,
}
impl Manager {
    /// These collections have no bytes at14 and self-default to retained prior
    /// native contents. Neither an empty collection nor fresh initialization is
    /// inferred. PostUnserialize also consults/rebuilds runtime active lists.
    pub const OMITTED_COLLECTIONS_RETAIN_PRIOR: [&'static str; 3] =
        ["co2_by_usage_type", "droughts", "one_offs"];
    pub const OMITTED_READ_DEFAULTS: OmittedReadDefaults = OmittedReadDefaults {
        next_ice_loss_turns: -1,
        next_sea_level_rise_turns: -1,
        halt_flood_fertility_raw: 0,
        halt_storm_fertility_raw: 0,
        fertility_removal_chance: 0,
        last_turn_worst_co2_footprint: 0,
        last_turn_average_co2_footprint: 0,
    };
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
            wire_version: r.version(WIRE_VERSION)?,
            resources_expended: Vec::read(&mut r)?,
            carbon_sequestered: Vec::read(&mut r)?,
            climate_change_for_last_sea_level_event: i32::read(&mut r)?,
            severity_for_last_sea_level_event: i32::read(&mut r)?,
            tiles_flooded: i32::read(&mut r)?,
            tiles_submerged: i32::read(&mut r)?,
            total_forested_tiles: i32::read(&mut r)?,
            turn_first_resource_consumed: i32::read(&mut r)?,
            cumulative_deforestation_points: i32::read(&mut r)?,
            co2_footprint_modifier: i32::read(&mut r)?,
            deforestation_hash: u32::read(&mut r)?,
            this_turn_co2_footprint: Vec::read(&mut r)?,
            last_turn_co2_footprint: Vec::read(&mut r)?,
            storms: Vec::read(&mut r)?,
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
        words(&mut b, &[14, 1, 14, u32::MAX, 41, 0x80000001, 1, 14, 7, 9]);
        words(&mut b, &[1, 2, 3, 4, 5, 6, 7, 8, 0xfedcba98]);
        words(&mut b, &[2, 10, u32::MAX, 1, 20, 1, 14, 2, 3]);
        b.push(255);
        words(&mut b, &[101, 202, u32::MAX, 2, 8, u32::MAX, 1, 9, 3]);
        b.extend([b'A', 0, 255]);
        b
    }
    #[test]
    fn literal_legacy_ordinals_q8_and_field_order() {
        let b = fixture();
        assert_eq!(b.len(), 152);
        let (m, n, i) = Manager::decode_prefix_with_items(&b, Limits::default()).unwrap();
        assert_eq!((n, i), (152, 12));
        assert_eq!(
            (
                m.resources_expended[0].player_ordinal,
                m.resources_expended[0].resource_ordinal
            ),
            (-1, 41)
        );
        assert_eq!(
            m.resources_expended[0].read_values(),
            ResourceReadValues {
                amount_q8: 256,
                last_turn_amount_q8: 0,
                raw_amount_q8: 256,
                raw_last_turn_amount_q8: 0
            }
        );
        assert_eq!(m.carbon_sequestered[0].amount, 9);
        assert_eq!(m.deforestation_hash, 0xfedcba98);
        assert_eq!(
            [
                m.climate_change_for_last_sea_level_event,
                m.severity_for_last_sea_level_event,
                m.tiles_flooded,
                m.tiles_submerged,
                m.total_forested_tiles,
                m.turn_first_resource_consumed,
                m.cumulative_deforestation_points,
                m.co2_footprint_modifier
            ],
            [1, 2, 3, 4, 5, 6, 7, 8]
        );
        assert_eq!(m.this_turn_co2_footprint, [10, -1]);
        assert_eq!(m.last_turn_co2_footprint, [20]);
        let storm = &m.storms[0];
        assert_eq!(
            (
                storm.random_event_ordinal,
                storm.severity_ordinal,
                storm.active_raw,
                storm.start_turn,
                storm.current_plot,
                storm.current_direction_ordinal
            ),
            (2, 3, 255, 101, 202, -1)
        );
        assert_eq!(storm.turn_end_plots, [8, -1]);
        assert_eq!(storm.plots_affected, [9]);
        assert_eq!(storm.name_bytes, [b'A', 0, 255]);
        assert_eq!(Storm::OMITTED_START_PLOT, -1);
        assert_eq!(
            Manager::OMITTED_COLLECTIONS_RETAIN_PRIOR,
            ["co2_by_usage_type", "droughts", "one_offs"]
        );
        assert_eq!(Manager::OMITTED_READ_DEFAULTS.next_ice_loss_turns, -1);
    }
    #[test]
    fn every_cut_limit_and_independent_record_tag() {
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
        for cap in 0..12 {
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
        for at in [0, 8, 28, 100] {
            let mut bad = b.clone();
            bad[at..at + 4].copy_from_slice(&99u32.to_le_bytes());
            assert_eq!(
                Manager::decode(&bad, Limits::default()),
                Err(Error::Version {
                    expected: 14,
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
            152
        );
        let mut hostile = b;
        hostile[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            Manager::decode(&hostile, Limits::default()),
            Err(Error::ResourceLimit)
        );
    }
}
