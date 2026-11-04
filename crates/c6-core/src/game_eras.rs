//! Linux XP2 Game.Eras26 and independently tagged26 rows, wire/read projection.
//! Hash words, raw flags, ordered rows and serialized vector tails are retained.
//! Native definition remaps, archive/event state and gameplay are unapplied.
//! Byte/item limits bound input and counted entries, not a separate heap cap.

pub use crate::unit::Limits;
pub const WIRE_VERSION: u32 = 26;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Game.Eras wire: {self:?}")
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
pub struct PlayerChoices {
    pub wire_version: u32,
    pub commemorate_option_hashes: Vec<u32>,
    pub compliment_options: Vec<i32>,
    pub active_commemoration_hashes: Vec<u32>,
    pub num_allowed_commemorations: i32,
}
impl Wire for PlayerChoices {
    const MIN: usize = 20;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: r.version(WIRE_VERSION)?,
            commemorate_option_hashes: Vec::read(r)?,
            compliment_options: Vec::read(r)?,
            active_commemoration_hashes: Vec::read(r)?,
            num_allowed_commemorations: i32::read(r)?,
        })
    }
}
macro_rules! rates {
    ($($field:ident),* $(,)?)=>{
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct ScoreRates {$(pub $field:i32),*}
        impl Wire for ScoreRates {
            const MIN:usize=72;
            fn read(r:&mut Reader<'_>)->Result<Self,Error> {Ok(Self {$($field:i32::read(r)?),*})}
        }
    }
}
// Serialization order differs from the object-member order.
rates!(
    city_religion_conversion,
    tech_boost,
    civic_boost,
    district_constructed,
    non_barbarian_unit_killed,
    corps_killed,
    army_killed,
    trade_route_completed,
    continent_discovered,
    natural_wonder_discovered,
    non_barbarian_sea_unit_killed,
    science_building_constructed,
    culture_building_constructed,
    aerodrome_building_constructed,
    industrial_building_constructed,
    spy_successful_mission,
    artifact_extracted,
    great_person_earned
);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OmittedScoreReadDefaults {
    pub current_tech: i32,
    pub current_civic: i32,
    pub non_barbarian_unit_killed_by_gdr: i32,
    pub unit_promotion_earned: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerScores {
    pub wire_version: u32,
    /// Complete serialized vector, before native resize22; surplus entries remain.
    pub era_scores: Vec<i32>,
    pub rates: ScoreRates,
}
impl PlayerScores {
    pub const OMITTED_READ_DEFAULTS: OmittedScoreReadDefaults = OmittedScoreReadDefaults {
        current_tech: 0,
        current_civic: 0,
        non_barbarian_unit_killed_by_gdr: 0,
        unit_promotion_earned: 0,
    };
    /// Source Read resizes to22, preserving its prefix, truncating surplus and
    /// zero-filling new cells. This derived view does not alter saved wire data.
    pub fn native_read_era_scores(&self) -> [i32; 22] {
        let mut v = [0; 22];
        let n = self.era_scores.len().min(v.len());
        v[..n].copy_from_slice(&self.era_scores[..n]);
        v
    }
}
impl Wire for PlayerScores {
    const MIN: usize = 80;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: r.version(WIRE_VERSION)?,
            era_scores: Vec::read(r)?,
            rates: ScoreRates::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerThresholds {
    pub wire_version: u32,
    pub golden_age_base_threshold: i32,
    pub dark_age_base_threshold: i32,
    pub baseline: i32,
    /// Complete wire vector before the mandatory native resize11.
    pub threshold_shifts: Vec<i32>,
}
impl PlayerThresholds {
    pub fn native_read_threshold_shifts(&self) -> [i32; 11] {
        let mut v = [0; 11];
        let n = self.threshold_shifts.len().min(v.len());
        v[..n].copy_from_slice(&self.threshold_shifts[..n]);
        v
    }
}
impl Wire for PlayerThresholds {
    const MIN: usize = 20;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: r.version(WIRE_VERSION)?,
            golden_age_base_threshold: i32::read(r)?,
            dark_age_base_threshold: i32::read(r)?,
            baseline: i32::read(r)?,
            threshold_shifts: Vec::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Eras {
    pub wire_version: u32,
    pub current_game_era_hash: u32,
    pub current_era_start_turn: i32,
    pub next_era_countdown: i32,
    pub player_eras_changed_raw: u8,
    pub current_golden_ages_raw: Vec<u8>,
    pub current_dark_ages_raw: Vec<u8>,
    pub current_choices: Vec<PlayerChoices>,
    pub current_scores: Vec<PlayerScores>,
    pub previous_scores: Vec<PlayerScores>,
    pub previous_previous_scores: Vec<PlayerScores>,
    pub current_thresholds: Vec<PlayerThresholds>,
    pub super_golden_age_credits: Vec<i32>,
    pub always_allow_commemoration_quest_counts: Vec<i32>,
    pub extra_era_score_per_pride_moment: Vec<i32>,
    pub min_score_for_extra_era_score: Vec<i32>,
    pub previous_thresholds: Vec<PlayerThresholds>,
}
impl Eras {
    /// This field is absent before27 and Read explicitly assigns TRUE, not false
    /// and not an unknown prior value. No final post-load initialization is run.
    pub fn omitted_starting_thresholds_calculated_native_default(&self) -> Option<u8> {
        (self.wire_version == 26).then_some(1)
    }
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
            current_game_era_hash: u32::read(&mut r)?,
            current_era_start_turn: i32::read(&mut r)?,
            next_era_countdown: i32::read(&mut r)?,
            player_eras_changed_raw: u8::read(&mut r)?,
            current_golden_ages_raw: Vec::read(&mut r)?,
            current_dark_ages_raw: Vec::read(&mut r)?,
            current_choices: Vec::read(&mut r)?,
            current_scores: Vec::read(&mut r)?,
            previous_scores: Vec::read(&mut r)?,
            previous_previous_scores: Vec::read(&mut r)?,
            current_thresholds: Vec::read(&mut r)?,
            super_golden_age_credits: Vec::read(&mut r)?,
            always_allow_commemoration_quest_counts: Vec::read(&mut r)?,
            extra_era_score_per_pride_moment: Vec::read(&mut r)?,
            min_score_for_extra_era_score: Vec::read(&mut r)?,
            previous_thresholds: Vec::read(&mut r)?,
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
    fn score(b: &mut Vec<u8>, base: u32) {
        words(b, &[26, 3, 7, u32::MAX, 9]);
        for n in base + 1..=base + 18 {
            words(b, &[n]);
        }
    }
    fn threshold(b: &mut Vec<u8>, base: u32) {
        words(
            b,
            &[26, base + 1, base + 2, base + 3, 2, (-2i32) as u32, 33],
        );
    }
    fn fixture() -> Vec<u8> {
        let mut b = Vec::new();
        words(&mut b, &[26, 0xfacade00, 418, u32::MAX]);
        b.push(2);
        words(&mut b, &[3]);
        b.extend([0, 2, 255]);
        words(&mut b, &[1]);
        b.push(1);
        words(
            &mut b,
            &[
                1,
                26,
                2,
                0x11223344,
                0x55667788,
                1,
                u32::MAX,
                1,
                0xdeadbeef,
                3,
            ],
        );
        for base in [100, 200, 300] {
            words(&mut b, &[1]);
            score(&mut b, base);
        }
        words(&mut b, &[1]);
        threshold(&mut b, 10);
        words(&mut b, &[2, 44, u32::MAX, 1, 55, 1, 66, 1, 77, 1]);
        threshold(&mut b, 40);
        b
    }
    #[test]
    fn literal_wire_order_hashes_and_true_omitted_flag() {
        let b = fixture();
        assert_eq!(b.len(), 457);
        let (e, n, i) = Eras::decode_prefix_with_items(&b, Limits::default()).unwrap();
        assert_eq!((n, i), (457, 32));
        assert_eq!(
            (
                e.current_game_era_hash,
                e.current_era_start_turn,
                e.next_era_countdown,
                e.player_eras_changed_raw
            ),
            (0xfacade00, 418, -1, 2)
        );
        assert_eq!(
            e.omitted_starting_thresholds_calculated_native_default(),
            Some(1)
        );
        assert_eq!(e.current_golden_ages_raw, [0, 2, 255]);
        assert_eq!(e.current_dark_ages_raw, [1]);
        let c = &e.current_choices[0];
        assert_eq!(c.commemorate_option_hashes, [0x11223344, 0x55667788]);
        assert_eq!(c.compliment_options, [-1]);
        assert_eq!(c.active_commemoration_hashes, [0xdeadbeef]);
        assert_eq!(c.num_allowed_commemorations, 3);
        let s = &e.current_scores[0];
        assert_eq!(s.rates.city_religion_conversion, 101);
        assert_eq!(s.rates.tech_boost, 102);
        assert_eq!(s.rates.civic_boost, 103);
        assert_eq!(s.rates.great_person_earned, 118);
        assert_eq!(e.previous_scores[0].rates.city_religion_conversion, 201);
        assert_eq!(
            e.previous_previous_scores[0].rates.city_religion_conversion,
            301
        );
        assert_eq!(
            (
                e.current_thresholds[0].golden_age_base_threshold,
                e.current_thresholds[0].dark_age_base_threshold,
                e.current_thresholds[0].baseline
            ),
            (11, 12, 13)
        );
        assert_eq!(e.previous_thresholds[0].baseline, 43);
        assert_eq!(e.super_golden_age_credits, [44, -1]);
        assert_eq!(e.always_allow_commemoration_quest_counts, [55]);
        assert_eq!(e.extra_era_score_per_pride_moment, [66]);
        assert_eq!(e.min_score_for_extra_era_score, [77]);
    }
    #[test]
    fn source_read_resizes_preserve_prefix_and_raw_surplus() {
        let e = Eras::decode(&fixture(), Limits::default()).unwrap();
        let mut s = e.current_scores[0].clone();
        let v = s.native_read_era_scores();
        assert_eq!(&v[..3], &[7, -1, 9]);
        assert!(v[3..].iter().all(|v| *v == 0));
        s.era_scores = (1..=30).collect();
        assert_eq!(
            s.native_read_era_scores(),
            std::array::from_fn(|i| i as i32 + 1)
        );
        assert_eq!(s.era_scores.len(), 30);
        let mut t = e.current_thresholds[0].clone();
        let v = t.native_read_threshold_shifts();
        assert_eq!(&v[..2], &[-2, 33]);
        assert!(v[2..].iter().all(|v| *v == 0));
        t.threshold_shifts = (1..=20).collect();
        assert_eq!(
            t.native_read_threshold_shifts(),
            std::array::from_fn(|i| i as i32 + 1)
        );
        assert_eq!(t.threshold_shifts.len(), 20);
        assert_eq!(PlayerScores::OMITTED_READ_DEFAULTS.current_tech, 0);
    }
    #[test]
    fn cuts_limits_independent_tags_and_strict_tail() {
        let b = fixture();
        for cut in 0..b.len() {
            assert!(
                Eras::decode(&b[..cut], Limits::default()).is_err(),
                "cut {cut}"
            );
        }
        for cap in 0..b.len() {
            assert_eq!(
                Eras::decode(
                    &b,
                    Limits {
                        max_bytes: cap,
                        ..Limits::default()
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        for cap in 0..32 {
            assert_eq!(
                Eras::decode(
                    &b,
                    Limits {
                        max_items: cap,
                        ..Limits::default()
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        for at in [0, 33, 73, 169, 265, 361, 429] {
            let mut bad = b.clone();
            bad[at..at + 4].copy_from_slice(&25u32.to_le_bytes());
            assert_eq!(
                Eras::decode(&bad, Limits::default()),
                Err(Error::Version {
                    expected: 26,
                    actual: 25
                })
            );
        }
        let mut tail = b.clone();
        tail.push(0);
        assert_eq!(Eras::decode(&tail, Limits::default()), Err(Error::Trailing));
        assert_eq!(
            Eras::decode_prefix(&tail, Limits::default()).unwrap().1,
            457
        );
        let mut hostile = b;
        hostile[17..21].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            Eras::decode(&hostile, Limits::default()),
            Err(Error::ResourceLimit)
        );
    }
}
