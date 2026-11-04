//! Reached Context5 trailing Tutorial byte and Achievements8 wire transport.
//! Independent guards, ordered saved counters and raw flags remain unchanged.
//! Omitted prior fields, explicit native read defaults and the V8 Dido vector
//! adjustment are distinct from the saved data. Native installation, subscribers,
//! map clearing, vector resizing and Context/IndexSerializer PostLoad are unapplied.
pub use crate::unit::Limits;
pub const ACHIEVEMENTS_VERSION: u32 = 8;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Context5 suffix wire: {self:?}")
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
pub enum CometStrikeReadDefault {
    CopyNewtonRaw,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DistrictMapReadEffect {
    ClearPriorMapBeforeReading,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DidoReadAdjustment {
    ResizeToFiveSetFifthToMax,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OmittedReadState {
    pub started: OmittedPriorState,
    pub units_trained: OmittedPriorState,
    pub cities_conquered: OmittedPriorState,
    pub carbon_recaptures: OmittedPriorState,
    pub nobel_bits: OmittedPriorState,
    pub plague_doctors_trained: OmittedPriorState,
    pub cities_with_full_health_per_turn: OmittedPriorState,
    pub comet_strike_default: CometStrikeReadDefault,
    pub player_cities_lost_in_dark_age_default: i32,
    pub player_tech_boosts_past_current_era_default: i32,
    pub districts_pillaged_by_random_events: DistrictMapReadEffect,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Achievements {
    pub opening_guard: Vec<u8>,
    pub wire_version: u32,
    pub turn_last_civic_boost: Vec<i32>,
    pub turn_last_tech_boost: Vec<i32>,
    pub num_civic_boosts_this_turn: Vec<i32>,
    pub num_tech_boosts_this_turn: Vec<i32>,
    pub turn_formal_war: Vec<i32>,
    pub turn_surprise_war: Vec<i32>,
    pub emergencies: Vec<i32>,
    pub golden_ages: Vec<i32>,
    pub golden_age_era: Vec<i32>,
    pub darwin_raw: u8,
    pub newton_raw: u8,
    /// Saved vector before the native V8 resize-to-five/fifth=0xffffffff effect.
    pub dido_move_continents: Vec<u32>,
    pub dido_native_read_adjustment: DidoReadAdjustment,
    pub omitted: OmittedReadState,
    pub closing_guard: Vec<u8>,
}
impl Wire for Achievements {
    const MIN: usize = 54;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            opening_guard: Vec::read(r)?,
            wire_version: r.version(ACHIEVEMENTS_VERSION)?,
            turn_last_civic_boost: Vec::read(r)?,
            turn_last_tech_boost: Vec::read(r)?,
            num_civic_boosts_this_turn: Vec::read(r)?,
            num_tech_boosts_this_turn: Vec::read(r)?,
            turn_formal_war: Vec::read(r)?,
            turn_surprise_war: Vec::read(r)?,
            emergencies: Vec::read(r)?,
            golden_ages: Vec::read(r)?,
            golden_age_era: Vec::read(r)?,
            darwin_raw: u8::read(r)?,
            newton_raw: u8::read(r)?,
            dido_move_continents: Vec::read(r)?,
            dido_native_read_adjustment: DidoReadAdjustment::ResizeToFiveSetFifthToMax,
            omitted: OmittedReadState {
                started: OmittedPriorState::Unknown,
                units_trained: OmittedPriorState::Unknown,
                cities_conquered: OmittedPriorState::Unknown,
                carbon_recaptures: OmittedPriorState::Unknown,
                nobel_bits: OmittedPriorState::Unknown,
                plague_doctors_trained: OmittedPriorState::Unknown,
                cities_with_full_health_per_turn: OmittedPriorState::Unknown,
                comet_strike_default: CometStrikeReadDefault::CopyNewtonRaw,
                player_cities_lost_in_dark_age_default: 0,
                player_tech_boosts_past_current_era_default: 0,
                districts_pillaged_by_random_events:
                    DistrictMapReadEffect::ClearPriorMapBeforeReading,
            },
            closing_guard: Vec::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Context5Suffix {
    pub tutorial_raw: u8,
    pub achievements: Achievements,
}
impl Context5Suffix {
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
            tutorial_raw: u8::read(&mut r)?,
            achievements: Achievements::read(&mut r)?,
        };
        Ok((v, r.pos, r.items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &[u8] = &[
        255, 2, 0, 0, 0, 255, 0, 8, 0, 0, 0, 3, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 127, 0,
        0, 0, 128, 0, 0, 0, 0, 1, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
        0, 247, 255, 255, 255, 1, 0, 0, 0, 42, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0,
        0, 0, 255, 255, 255, 255, 2, 254, 3, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 128,
        1, 0, 0, 0, 128,
    ];
    #[test]
    fn saved_vectors_and_flags_are_separate_from_native_read_effects() {
        let limits = Limits {
            max_bytes: FIXTURE.len(),
            max_items: 16,
        };
        let (v, n, i) = Context5Suffix::decode_prefix_with_items(FIXTURE, limits).unwrap();
        assert_eq!((n, i), (FIXTURE.len(), 16));
        assert_eq!(v.tutorial_raw, 255);
        let a = &v.achievements;
        assert_eq!(a.opening_guard, [255, 0]);
        assert_eq!(a.closing_guard, [128]);
        assert_eq!(a.turn_last_civic_boost, [-1, i32::MAX, i32::MIN]);
        assert!(a.turn_last_tech_boost.is_empty());
        assert_eq!(a.num_civic_boosts_this_turn, [7]);
        assert!(a.num_tech_boosts_this_turn.is_empty());
        assert_eq!(a.turn_formal_war, [0]);
        assert_eq!(a.turn_surprise_war, [-9]);
        assert_eq!(a.emergencies, [42]);
        assert_eq!(a.golden_ages, [1, 1]);
        assert_eq!(a.golden_age_era, [-1]);
        assert_eq!((a.darwin_raw, a.newton_raw), (2, 254));
        assert_eq!(a.dido_move_continents, [u32::MAX, 0, 0x80000000]);
        assert_eq!(
            a.dido_native_read_adjustment,
            DidoReadAdjustment::ResizeToFiveSetFifthToMax
        );
        assert_eq!(a.omitted.units_trained, OmittedPriorState::Unknown);
        assert_eq!(
            a.omitted.comet_strike_default,
            CometStrikeReadDefault::CopyNewtonRaw
        );
        assert_eq!(a.omitted.player_cities_lost_in_dark_age_default, 0);
        assert_eq!(a.omitted.player_tech_boosts_past_current_era_default, 0);
        assert_eq!(
            a.omitted.districts_pillaged_by_random_events,
            DistrictMapReadEffect::ClearPriorMapBeforeReading
        );
        let mut b = FIXTURE.to_vec();
        b.extend([0x55, 0xaa]);
        assert_eq!(
            Context5Suffix::decode_prefix_with_items(&b, limits)
                .unwrap()
                .1,
            FIXTURE.len()
        );
        assert_eq!(
            Context5Suffix::decode(&b, Limits::default()),
            Err(Error::Trailing)
        );
    }
    #[test]
    fn all_byte_item_count_and_version_boundaries_are_bounded() {
        for cut in 0..FIXTURE.len() {
            assert!(Context5Suffix::decode(&FIXTURE[..cut], Limits::default()).is_err());
            assert!(Context5Suffix::decode(
                FIXTURE,
                Limits {
                    max_bytes: cut,
                    max_items: 16
                }
            )
            .is_err());
        }
        for max_items in 0..16 {
            assert!(Context5Suffix::decode(
                FIXTURE,
                Limits {
                    max_bytes: FIXTURE.len(),
                    max_items
                }
            )
            .is_err());
        }
        for at in COUNTS {
            let mut b = FIXTURE.to_vec();
            b[*at..*at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert!(Context5Suffix::decode(&b, Limits::default()).is_err());
        }
        let mut b = FIXTURE.to_vec();
        b[7..11].copy_from_slice(&99u32.to_le_bytes());
        assert_eq!(
            Context5Suffix::decode(&b, Limits::default()),
            Err(Error::Version {
                expected: 8,
                actual: 99
            })
        );
    }
    const COUNTS: &[usize] = &[1, 11, 27, 31, 39, 43, 51, 59, 67, 79, 89, 105];
}
