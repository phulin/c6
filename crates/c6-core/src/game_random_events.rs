//! Linux XP2 RandomEvents5 / HistoricalEvent3/4 wire projection only.
//! Q8 raw values, ordered occurrences, hashes and raw flag bytes are retained.
//! Native definition remaps, CompilePossibleEvents and gameplay are unapplied.
//! Byte/item limits bound input and counted entries, not a separate heap cap.

pub use crate::unit::Limits;
pub const WIRE_VERSION: u32 = 5;
pub const HISTORY_VERSION: u32 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Game.RandomEvents wire: {self:?}")
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
pub struct HistoricalEvent {
    pub wire_version: u32,
    pub random_event_hash: u32,
    pub named_river_hash: u32,
    pub named_volcano_hash: u32,
    pub natural_wonder_volcano_feature_hash: u32,
    pub start_plot: i32,
    pub current_plot: i32,
    pub current_direction_ordinal: i32,
    pub start_turn: i32,
    pub end_turn: i32,
    pub tiles_damaged: i32,
    pub population_lost: i32,
    pub units_lost: i32,
    pub fertility_added: i32,
    /// Length-prefixed bytes. Empty wire payload need not assign an existing
    /// native String; initialized object identity is outside this projection.
    pub name_bytes: Vec<u8>,
    /// Present since4; native Read explicitly assigns false below4.
    pub spread_raw: Option<u8>,
}
impl HistoricalEvent {
    pub fn omitted_spread_native_default(&self) -> Option<u8> {
        (self.wire_version == 3).then_some(0)
    }
}
impl Wire for HistoricalEvent {
    const MIN: usize = 60;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 3 | HISTORY_VERSION) {
            return Err(Error::Version {
                expected: HISTORY_VERSION,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            random_event_hash: u32::read(r)?,
            named_river_hash: u32::read(r)?,
            named_volcano_hash: u32::read(r)?,
            natural_wonder_volcano_feature_hash: u32::read(r)?,
            start_plot: i32::read(r)?,
            current_plot: i32::read(r)?,
            current_direction_ordinal: i32::read(r)?,
            start_turn: i32::read(r)?,
            end_turn: i32::read(r)?,
            tiles_damaged: i32::read(r)?,
            population_lost: i32::read(r)?,
            units_lost: i32::read(r)?,
            fertility_added: i32::read(r)?,
            name_bytes: Vec::read(r)?,
            spread_raw: if wire_version == HISTORY_VERSION {
                Some(u8::read(r)?)
            } else {
                None
            },
        })
    }
}
/// Native assignments for fields absent from RandomEvents5, kept separate from
/// serialized raw Q8 values. No possible-event lists are fabricated from this.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OmittedReadDefaults {
    pub drought_percent_chance_q8: i32,
    pub fire_percent_chance_q8: i32,
    pub one_off_percent_chance_q8: i32,
    pub drought_climate_increased_chance_q8: i32,
    pub fire_climate_increased_chance_q8: i32,
    pub one_off_climate_increased_chance_q8: i32,
    pub realism_override_ordinal: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RandomEvents {
    pub wire_version: u32,
    pub storm_percent_chance_q8: i32,
    pub flood_percent_chance_q8: i32,
    pub eruption_percent_chance_q8: i32,
    pub storm_climate_increased_chance_q8: i32,
    pub flood_climate_increased_chance_q8: i32,
    pub eruption_climate_increased_chance_q8: i32,
    pub historical_events: Vec<HistoricalEvent>,
}
impl RandomEvents {
    pub const OMITTED_READ_DEFAULTS: OmittedReadDefaults = OmittedReadDefaults {
        drought_percent_chance_q8: 0,
        fire_percent_chance_q8: 0,
        one_off_percent_chance_q8: 0,
        drought_climate_increased_chance_q8: 0,
        fire_climate_increased_chance_q8: 0,
        one_off_climate_increased_chance_q8: 0,
        realism_override_ordinal: -1,
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
            storm_percent_chance_q8: i32::read(&mut r)?,
            flood_percent_chance_q8: i32::read(&mut r)?,
            eruption_percent_chance_q8: i32::read(&mut r)?,
            storm_climate_increased_chance_q8: i32::read(&mut r)?,
            flood_climate_increased_chance_q8: i32::read(&mut r)?,
            eruption_climate_increased_chance_q8: i32::read(&mut r)?,
            historical_events: Vec::read(&mut r)?,
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
        words(&mut b, &[5, 0x80000001, 1024, 3072, 1, 768, u32::MAX, 2]);
        for tag in [3, 4] {
            words(
                &mut b,
                &[
                    tag,
                    0xa1234567,
                    u32::MAX,
                    0x81234567,
                    0x91234567,
                    11,
                    12,
                    u32::MAX,
                    100,
                    101,
                    1,
                    2,
                    3,
                    4,
                    3,
                ],
            );
            b.extend([b'X', 0, 255]);
            if tag == 4 {
                b.push(2);
            }
        }
        b
    }
    #[test]
    fn literal_raw_q8_hashes_and_nested_spread_versions() {
        let b = fixture();
        assert_eq!(b.len(), 159);
        let (m, n, i) = RandomEvents::decode_prefix_with_items(&b, Limits::default()).unwrap();
        assert_eq!((n, i), (159, 8));
        assert_eq!(
            [
                m.storm_percent_chance_q8,
                m.flood_percent_chance_q8,
                m.eruption_percent_chance_q8,
                m.storm_climate_increased_chance_q8,
                m.flood_climate_increased_chance_q8,
                m.eruption_climate_increased_chance_q8
            ],
            [i32::MIN + 1, 1024, 3072, 1, 768, -1]
        );
        let h = &m.historical_events[0];
        assert_eq!(
            (
                h.random_event_hash,
                h.named_river_hash,
                h.named_volcano_hash,
                h.natural_wonder_volcano_feature_hash
            ),
            (0xa1234567, u32::MAX, 0x81234567, 0x91234567)
        );
        assert_eq!(
            [
                h.start_plot,
                h.current_plot,
                h.current_direction_ordinal,
                h.start_turn,
                h.end_turn,
                h.tiles_damaged,
                h.population_lost,
                h.units_lost,
                h.fertility_added
            ],
            [11, 12, -1, 100, 101, 1, 2, 3, 4]
        );
        assert_eq!(h.name_bytes, [b'X', 0, 255]);
        assert_eq!(h.spread_raw, None);
        assert_eq!(h.omitted_spread_native_default(), Some(0));
        assert_eq!(m.historical_events[1].spread_raw, Some(2));
        assert_eq!(m.historical_events[1].omitted_spread_native_default(), None);
        assert_eq!(
            RandomEvents::OMITTED_READ_DEFAULTS.realism_override_ordinal,
            -1
        );
    }
    #[test]
    fn cuts_limits_prefix_and_independent_versions() {
        let b = fixture();
        for cut in 0..b.len() {
            assert!(
                RandomEvents::decode(&b[..cut], Limits::default()).is_err(),
                "cut {cut}"
            );
        }
        for cap in 0..b.len() {
            assert_eq!(
                RandomEvents::decode(
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
                RandomEvents::decode(
                    &b,
                    Limits {
                        max_items: cap,
                        ..Limits::default()
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        for (at, expected) in [(0, 5), (32, 4), (95, 4)] {
            let mut bad = b.clone();
            bad[at..at + 4].copy_from_slice(&99u32.to_le_bytes());
            assert_eq!(
                RandomEvents::decode(&bad, Limits::default()),
                Err(Error::Version {
                    expected,
                    actual: 99
                })
            );
        }
        let mut tail = b.clone();
        tail.push(0);
        assert_eq!(
            RandomEvents::decode(&tail, Limits::default()),
            Err(Error::Trailing)
        );
        assert_eq!(
            RandomEvents::decode_prefix(&tail, Limits::default())
                .unwrap()
                .1,
            159
        );
        let mut hostile = b;
        hostile[28..32].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            RandomEvents::decode(&hostile, Limits::default()),
            Err(Error::ResourceLimit)
        );
    }
}
