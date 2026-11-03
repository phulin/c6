//! Current Linux XP2 Player.Stats v27 wire transport, including nested records.
//! No live Reset, definition remapping, callbacks, gameplay, actions or turns.
use crate::tracked_value::TrackedValue;
use crate::unit::{HashValue, Limits};

pub const VERSION: u32 = 27;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    InvalidBool(u8),
    InvalidContainer,
    TrackedValue(crate::tracked_value::Error),
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player.Stats wire: {self:?}")
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
    fn version(&mut self) -> Result<(), Error> {
        let actual = u32::read(self)?;
        if actual != VERSION {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
        }
        Ok(())
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
macro_rules! record {
    ($name:ident, $version:expr, { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name { $(pub $field: $ty),* }
        impl Wire for $name {
            const MIN: usize = (if $version { 4 } else { 0 }) $(+ <$ty as Wire>::MIN)*;
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                if $version { r.version()?; }
                Ok(Self { $($field: <$ty as Wire>::read(r)?),* })
            }
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                if $version { VERSION.write(w)?; }
                $(self.$field.write(w)?;)*
                Ok(())
            }
        }
    };
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
/// Native WriteArray framing requires exactly 64 entries for these seven fields.
impl Wire for [i32; 64] {
    const MIN: usize = 260;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        if u32::read(r)? != 64 {
            return Err(Error::InvalidContainer);
        }
        r.charge(64)?;
        let mut values = [0; 64];
        for value in &mut values {
            *value = i32::read(r)?;
        }
        Ok(values)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.charge(64)?;
        64u32.write(w)?;
        for value in self {
            value.write(w)?;
        }
        Ok(())
    }
}
impl Wire for TrackedValue {
    const MIN: usize = 15;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let (v, n, items) = Self::decode_prefix_with_items(&r.data[r.pos..], r.remaining())
            .map_err(Error::TrackedValue)?;
        r.raw(n)?;
        r.charge(items)?;
        Ok(v)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        let limits = Limits {
            max_bytes: w.limits.max_bytes - w.data.len(),
            max_items: w.limits.max_items - w.items,
        };
        let (bytes, items) = self
            .encode_with_items(limits)
            .map_err(Error::TrackedValue)?;
        w.charge(items)?;
        w.raw(&bytes)
    }
}
record!(Stats,true,{
    max_melee_strength_trained:i32,
    max_ranged_strength_trained:i32,
    max_bombard_strength_trained:i32,
    military_strength:i32,
    military_strength_from_gold:i32,
    air_military_strength:i32,
    land_military_strength:i32,
    naval_military_strength:i32,
    barbarians_killed:i32,
    barbarian_camps_cleared:i32,
    tiles_pillaged:i32,
    wmd_city_hits:i32,
    cities_revolted:i32,
    cities_annexed:i32,
    total_cities_lost_to_disloyalty:i32,
    total_cities_gained_from_disloyalty:i32,
    total_number_of_alliances_formed:i32,
    total_number_of_emergency_alliances_formed:i32,
    gold_spent_levying:i32,
    emergencies_joined:i32,
    total_governors_appointed:i32,
    total_governor_assignments:i32,
    diplomatic_victory_points:TrackedValue,
    last_turn_favor:i32,
    current_turn_favor:i32,
    lifetime_favor:i32,
    science_victory_points_q8:i32,
    science_victory_points_per_turn_q8:i32,
    num_units_constructed:Vec<HashValue<i32>>,
    num_projects_advanced:Vec<HashValue<i32>>,
    wonders_lost_plots:Vec<i32>,
    cached_yields_q8:Vec<HashValue<i32>>,
    removed_feature_counts:[i32; 64],
    national_park_counts:[i32; 64],
    placed_forest_counts:[i32; 64],
    known_wmd_counts:[i32; 64],
    cleared_bandit_camp_counts:[i32; 64],
    city_state_war_counts:[i32; 64],
    players_revived:[i32; 64],
    fertility_added_by_droughts:i32,
    tiles_damaged_by_droughts:i32,
    population_lost_by_droughts:i32,
    units_killed_by_droughts:i32,
    fertility_added_by_floods:i32,
    tiles_damaged_by_floods:i32,
    population_lost_by_floods:i32,
    units_killed_by_floods:i32,
    fertility_added_by_storms:i32,
    tiles_damaged_by_storms:i32,
    population_lost_by_storms:i32,
    units_killed_by_storms:i32,
    fertility_added_by_volcanos:i32,
    tiles_damaged_by_volcanos:i32,
    population_lost_by_volcanos:i32,
    units_killed_by_volcanos:i32,
});
impl Stats {
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
    fn specimen() -> Stats {
        Stats {
            max_melee_strength_trained: i32::MIN,
            max_ranged_strength_trained: i32::MIN,
            max_bombard_strength_trained: i32::MIN,
            military_strength: i32::MIN,
            military_strength_from_gold: i32::MIN,
            air_military_strength: i32::MIN,
            land_military_strength: i32::MIN,
            naval_military_strength: i32::MIN,
            barbarians_killed: i32::MIN,
            barbarian_camps_cleared: i32::MIN,
            tiles_pillaged: i32::MIN,
            wmd_city_hits: i32::MIN,
            cities_revolted: i32::MIN,
            cities_annexed: i32::MIN,
            total_cities_lost_to_disloyalty: i32::MIN,
            total_cities_gained_from_disloyalty: i32::MIN,
            total_number_of_alliances_formed: i32::MIN,
            total_number_of_emergency_alliances_formed: i32::MIN,
            gold_spent_levying: i32::MIN,
            emergencies_joined: i32::MIN,
            total_governors_appointed: i32::MIN,
            total_governor_assignments: i32::MIN,
            diplomatic_victory_points: TrackedValue {
                value_q8: i32::MIN,
                include_steps: false,
                min_q8: Some(-1),
                max_q8: Some(i32::MAX),
                steps: (0..2)
                    .map(|_| crate::tracked_value::Step {
                        priority: i32::MIN,
                        id: 0x80000001,
                        label: crate::map::LocalizedText {
                            base: vec![0, 0xff, b'a'],
                            arguments: vec![crate::map::LocalizedArgument::Long64(i64::MIN)],
                            named_arguments: vec![],
                        },
                        value: TrackedValue {
                            value_q8: -257,
                            include_steps: true,
                            min_q8: None,
                            max_q8: None,
                            steps: vec![],
                        },
                    })
                    .collect(),
            },
            last_turn_favor: i32::MIN,
            current_turn_favor: i32::MIN,
            lifetime_favor: i32::MIN,
            science_victory_points_q8: i32::MIN,
            science_victory_points_per_turn_q8: i32::MIN,
            num_units_constructed: vec![
                HashValue {
                    definition_hash: 0x80000001,
                    value: -1,
                },
                HashValue {
                    definition_hash: 0x80000001,
                    value: i32::MAX,
                },
            ],
            num_projects_advanced: vec![
                HashValue {
                    definition_hash: 0x80000001,
                    value: -1,
                },
                HashValue {
                    definition_hash: 0x80000001,
                    value: i32::MAX,
                },
            ],
            wonders_lost_plots: vec![-1, i32::MAX],
            cached_yields_q8: vec![
                HashValue {
                    definition_hash: 0x80000001,
                    value: -1,
                },
                HashValue {
                    definition_hash: 0x80000001,
                    value: i32::MAX,
                },
            ],
            removed_feature_counts: [i32::MIN; 64],
            national_park_counts: [i32::MIN; 64],
            placed_forest_counts: [i32::MIN; 64],
            known_wmd_counts: [i32::MIN; 64],
            cleared_bandit_camp_counts: [i32::MIN; 64],
            city_state_war_counts: [i32::MIN; 64],
            players_revived: [i32::MIN; 64],
            fertility_added_by_droughts: i32::MIN,
            tiles_damaged_by_droughts: i32::MIN,
            population_lost_by_droughts: i32::MIN,
            units_killed_by_droughts: i32::MIN,
            fertility_added_by_floods: i32::MIN,
            tiles_damaged_by_floods: i32::MIN,
            population_lost_by_floods: i32::MIN,
            units_killed_by_floods: i32::MIN,
            fertility_added_by_storms: i32::MIN,
            tiles_damaged_by_storms: i32::MIN,
            population_lost_by_storms: i32::MIN,
            units_killed_by_storms: i32::MIN,
            fertility_added_by_volcanos: i32::MIN,
            tiles_damaged_by_volcanos: i32::MIN,
            population_lost_by_volcanos: i32::MIN,
            units_killed_by_volcanos: i32::MIN,
        }
    }
    #[test]
    fn nonempty_round_trip_and_budgets() {
        let v = specimen();
        let (bytes, items) = v.encode_with_items(limits()).unwrap();
        let exact = Limits {
            max_bytes: bytes.len(),
            max_items: items,
        };
        assert_eq!(v, Stats::decode(&bytes, exact).unwrap());
        assert_eq!((bytes.clone(), items), v.encode_with_items(exact).unwrap());
        let mut carrier = bytes.clone();
        carrier.extend([0x81, 0x82]);
        assert_eq!(
            (v.clone(), bytes.len(), items),
            Stats::decode_prefix_with_items(&carrier, exact).unwrap()
        );
        for short in [
            Limits {
                max_bytes: bytes.len() - 1,
                ..exact
            },
            Limits {
                max_items: items - 1,
                ..exact
            },
        ] {
            assert!(v.encode(short).is_err());
            assert!(Stats::decode(&bytes, short).is_err());
        }
    }
    #[test]
    fn every_prefix_and_trailing() {
        let bytes = specimen().encode(limits()).unwrap();
        for n in 0..bytes.len() {
            assert!(Stats::decode(&bytes[..n], limits()).is_err(), "prefix {n}");
        }
        let mut bytes = bytes;
        bytes.push(0);
        assert_eq!(Stats::decode(&bytes, limits()), Err(Error::Trailing));
    }
    #[test]
    fn versions_and_nested_bool() {
        let mut bytes = specimen().encode(limits()).unwrap();
        bytes[0] = 26;
        assert!(matches!(
            Stats::decode(&bytes, limits()),
            Err(Error::Version { .. })
        ));
        bytes[0] = 27;
        bytes[92] = 2;
        assert!(Stats::decode(&bytes, limits()).is_err());
        bytes[92] = 1;
        bytes[100] = 2;
        assert!(Stats::decode(&bytes, limits()).is_err());
    }
    #[test]
    fn fixed_array_count_and_hostile_vector() {
        let v = specimen();
        let bytes = v.encode(limits()).unwrap();
        // The leading scalar block ends after 22 i32s.
        let tracked_len = v
            .diplomatic_victory_points
            .encode_with_items(limits())
            .unwrap()
            .0
            .len();
        let first_vector = 4 + 22 * 4 + tracked_len + 5 * 4;
        let mut bad = bytes.clone();
        bad[first_vector..first_vector + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(Stats::decode(&bad, limits()).is_err());
        let first_array = first_vector + 3 * (4 + 2 * 8) + (4 + 2 * 4);
        let mut bad = bytes;
        bad[first_array..first_array + 4].copy_from_slice(&63u32.to_le_bytes());
        assert_eq!(Stats::decode(&bad, limits()), Err(Error::InvalidContainer));
    }
}
