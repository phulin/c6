//! Complete current Linux XP2 City Culture/Gold/Growth wire records only.
//!
//! Target SHA-256: a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa.
//! Culture v17 W/R: 0x287f9e0/0x287fad0; Gold v11: 0x2887340/0x2887460;
//! Growth v23: 0x2889970/0x2889bf0. Version constants: 0x2f21e80,
//! 0x2f226b0, 0x2f22790. Names/order are checked against matching DWARF.
//! TourismModifier W/R 0x287f7d0/0x287f840 and GreatWorkYieldModifier
//! 0x287f8f0/0x287f950 use Culture's version 17, including inside vectors.
//! Their counted sequence providers are 0x28832c0/0x2883370.
//! Era (0x2d4eda0) and Improvement (0x2d4ee10) serialize definition hashes;
//! GreatWorkObject (0x2d4ff90) and Yield (0x2d4fb70) serialize raw signed ints.
//! Q8 (0x2c9afe0) serializes its raw signed int. Gold's indexed-int writers:
//! Unit 0x2009620, Building 0x28893a0, District 0x2003280, Terrain 0x2194ef0.
//! Growth explicitly writes Feature hash/appeal pairs rather than raw structs.
//!
//! These preserve wire order, unknown/duplicate hashes and all raw Q8 bits.
//! No registry mapping, constructor defaults or City.Read application is implied.
//! Original readers do not invoke Initialize; existing ownership/parent pointers
//! and allocation/FAuto effects are outside this codec. Growth's current reader
//! clears and rebuilds its Feature list, mapping every hash (including misses);
//! Gold overlays resized definition arrays, requiring actual prior storage and
//! registries. Gold resize explicitly zero-initializes newly exposed slots, while
//! retaining the old prefix; this does not establish zero for omitted old slots.
//! No constructor defaults are inferred. This is not complete City state,
//! real save loading, gameplay RNG, action execution or turn equivalence.
use crate::unit::{HashValue, Limits};

pub const CULTURE_VERSION: i32 = 17;
pub const GOLD_VERSION: i32 = 11;
pub const GROWTH_VERSION: i32 = 23;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Version { expected: i32, actual: i32 },
    InvalidBool,
    Trailing,
    ResourceLimit,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux City economy input: {self:?}")
    }
}
impl std::error::Error for Error {}

struct Budget {
    limits: Limits,
    items: usize,
    allocated: usize,
}
impl Budget {
    fn list<T>(&mut self, count: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(count).ok_or(Error::ResourceLimit)?;
        self.allocated = self
            .allocated
            .checked_add(
                count
                    .checked_mul(std::mem::size_of::<T>())
                    .ok_or(Error::ResourceLimit)?,
            )
            .ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items || self.allocated > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
}
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    budget: Budget,
}
impl Reader<'_> {
    fn take(&mut self, count: usize) -> Result<&[u8], Error> {
        let end = self.pos.checked_add(count).ok_or(Error::ResourceLimit)?;
        if end > self.budget.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let bytes = self.bytes.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(bytes)
    }
}
struct Writer {
    bytes: Vec<u8>,
    budget: Budget,
}
impl Writer {
    fn put(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let end = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or(Error::ResourceLimit)?;
        if end > self.budget.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        self.bytes
            .try_reserve(bytes.len())
            .map_err(|_| Error::ResourceLimit)?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
}
trait Wire: Sized {
    const MIN_SIZE: usize;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error>;
    fn write(&self, w: &mut Writer) -> Result<(), Error>;
}
impl Wire for i32 {
    const MIN_SIZE: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(u32::read(r)? as i32)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.put(&self.to_le_bytes())
    }
}
impl Wire for u32 {
    const MIN_SIZE: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self::from_le_bytes(r.take(4)?.try_into().unwrap()))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.put(&self.to_le_bytes())
    }
}
impl Wire for bool {
    const MIN_SIZE: usize = 1;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        match r.take(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::InvalidBool),
        }
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.put(&[u8::from(*self)])
    }
}
impl Wire for HashValue<i32> {
    const MIN_SIZE: usize = 8;
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
impl<T: Wire> Wire for Vec<T> {
    const MIN_SIZE: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let count = u32::read(r)? as usize;
        r.budget.list::<T>(count)?;
        let size = count.checked_mul(T::MIN_SIZE).ok_or(Error::ResourceLimit)?;
        let end = r.pos.checked_add(size).ok_or(Error::ResourceLimit)?;
        if end > r.budget.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        if end > r.bytes.len() {
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
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        let count = u32::try_from(self.len()).map_err(|_| Error::ResourceLimit)?;
        w.budget.list::<T>(self.len())?;
        count.write(w)?;
        for value in self {
            value.write(w)?;
        }
        Ok(())
    }
}
macro_rules! record {
    ($name:ident, $version:expr, { $( $(#[$attr:meta])* $field:ident: $ty:ty ),* $(,)? }) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name { $( $(#[$attr])* pub $field: $ty, )* }
        impl Wire for $name {
            const MIN_SIZE: usize = 4 $(+ <$ty as Wire>::MIN_SIZE)*;
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                let actual = i32::read(r)?;
                if actual != $version { return Err(Error::Version { expected: $version, actual }); }
                Ok(Self { $( $field: <$ty as Wire>::read(r)?, )* })
            }
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                ($version).write(w)?;
                $( self.$field.write(w)?; )*
                Ok(())
            }
        }
        impl $name {
            /// Decode only at an independently established structural boundary.
            pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
                let mut r = Reader { bytes, pos: 0, budget: Budget { limits, items: 0, allocated: 0 } };
                let value = Self::read(&mut r)?;
                Ok((value, r.pos))
            }
            pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
                let (value, consumed) = Self::decode_prefix(bytes, limits)?;
                if consumed != bytes.len() { return Err(Error::Trailing); }
                Ok(value)
            }
            pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
                let mut w = Writer { bytes: Vec::new(), budget: Budget { limits, items: 0, allocated: 0 } };
                self.write(&mut w)?;
                Ok(w.bytes)
            }
        }
    };
}
record!(TourismModifier, CULTURE_VERSION, {
    improvement_hash: u32,
    great_work_object: i32,
    affects_wonder: bool,
    affects_religious_tourism: bool,
    scaling_factor: i32,
    tourism_modifier_id: i32,
});
record!(GreatWorkYieldModifier, CULTURE_VERSION, {
    great_work_object: i32,
    yield_type: i32,
    yield_change: i32,
    scaling_factor: i32,
});
record!(Culture, CULTURE_VERSION, {
    culture_raw_q8: i32,
    plots_acquired: i32,
    border_expansion_modifier: i32,
    min_era_enhanced_tourism_hash: u32,
    enhanced_tourism_modifier: i32,
    improvement_tourism_modifier: i32,
    national_park_tourism_modifier: i32,
    tourism_from_green_energy_modifier: i32,
    tourism_modifiers: Vec<TourismModifier>,
    great_work_yield_modifiers: Vec<GreatWorkYieldModifier>,
    expended_great_person_tiles: i32,
    plots_acquired_from_culture: i32,
    next_buyable_plot: i32,
    tourism_per_feature: i32,
    loyalty_from_great_works_citizen_modifier: i32,
    no_culture_border_expansion: bool,
});
record!(Gold, GOLD_VERSION, {
    plot_cost_modifier: i32,
    unit_cost_modifier: Vec<HashValue<i32>>,
    building_cost_modifier: Vec<HashValue<i32>>,
    district_cost_modifier: Vec<HashValue<i32>>,
    plot_cost_terrain_modifier: Vec<HashValue<i32>>,
    siphoned_gold: i32,
    building_faith_purchase_enabled: Vec<HashValue<i32>>,
    unit_faith_purchase_enabled: Vec<HashValue<i32>>,
    can_purchase_districts: bool,
    gold_from_citizens: i32,
});
record!(Growth, GROWTH_VERSION, {
    food_raw_q8: i32,
    other_growth_modifier_raw_q8: i32,
    housing_from_water_modifier: i32,
    housing_from_buildings_modifier: i32,
    housing_from_improvements_modifier: i32,
    housing_from_districts_modifier: i32,
    housing_from_civics_modifier: i32,
    amenities_from_entertainment_modifier: i32,
    amenities_from_civics_modifier: i32,
    extra_appeal: i32,
    universal_fresh_water_housing_bonus: bool,
    housing_from_great_people_modifier: i32,
    housing_from_starting_era_modifier: i32,
    housing_from_great_works_modifier: i32,
    amenities_from_great_people_modifier: i32,
    amenities_from_religion_modifier: i32,
    amenities_from_national_parks_modifier: i32,
    amenities_from_starting_era_modifier: i32,
    amenities_from_improvements_modifier: i32,
    turn_last_notified_starvation: i32,
    turn_last_notified_food_focus: i32,
    turn_last_notified_need_housing: i32,
    turn_last_notified_need_amenities: i32,
    amenities_from_city_states_modifier: i32,
    amenities_from_governors_modifier: i32,
    amenities_from_districts_modifier: i32,
    amenities_from_natural_wonders_modifier: i32,
    amenities_from_traits_modifier: i32,
    no_fresh_water_housing: i32,
    add_amenity_per_adjacent_luxury: i32,
    amenities_from_religion_followers: i32,
    amenities_from_religion_minimum_followers: i32,
    feature_appeal_modifiers: Vec<HashValue<i32>>,
});

#[cfg(test)]
mod tests {
    use super::*;
    fn int(bytes: &mut Vec<u8>, value: i32) {
        bytes.extend(value.to_le_bytes());
    }
    fn pair(bytes: &mut Vec<u8>, hash: u32, value: i32) {
        bytes.extend(hash.to_le_bytes());
        int(bytes, value);
    }
    fn culture_bytes() -> Vec<u8> {
        let mut b = Vec::new();
        int(&mut b, 17);
        for x in [i32::MIN, -4, 13, -1, 15, -16, 17, i32::MAX] {
            int(&mut b, x);
        }
        int(&mut b, 1);
        int(&mut b, 17);
        int(&mut b, -1);
        int(&mut b, -22);
        b.extend([1, 0]);
        int(&mut b, -23);
        int(&mut b, 24);
        int(&mut b, 1);
        int(&mut b, 17);
        for x in [-31, -32, -33, 34] {
            int(&mut b, x);
        }
        for x in [41, 42, -43, 44, 45] {
            int(&mut b, x);
        }
        b.push(1);
        b
    }
    fn gold_bytes() -> Vec<u8> {
        let mut b = Vec::new();
        int(&mut b, 11);
        int(&mut b, -7);
        for index in 0..6 {
            if index == 4 {
                int(&mut b, i32::MIN);
            }
            int(&mut b, 2);
            pair(&mut b, 0xdeadbeef, index);
            pair(&mut b, 0xdeadbeef, -index);
        }
        b.push(1);
        int(&mut b, i32::MAX);
        b
    }
    fn growth_bytes() -> Vec<u8> {
        let mut b = Vec::new();
        int(&mut b, 23);
        for x in 0..10 {
            int(&mut b, i32::MIN.wrapping_add(x));
        }
        b.push(1);
        for x in 0..21 {
            int(&mut b, x - 15);
        }
        int(&mut b, 2);
        pair(&mut b, u32::MAX, -64);
        pair(&mut b, u32::MAX, 64);
        b
    }
    #[test]
    fn economic_records_preserve_nested_versions_raw_bits_and_order() {
        let limits = Limits::default();
        let c = Culture::decode(&culture_bytes(), limits).unwrap();
        assert_eq!(c.culture_raw_q8, i32::MIN);
        assert_eq!(c.min_era_enhanced_tourism_hash, u32::MAX);
        assert_eq!(c.tourism_modifiers[0].great_work_object, -22);
        assert_eq!(c.great_work_yield_modifiers[0].yield_type, -32);
        assert_eq!(c.encode(limits).unwrap(), culture_bytes());
        let g = Gold::decode(&gold_bytes(), limits).unwrap();
        assert_eq!(g.siphoned_gold, i32::MIN);
        assert_eq!(g.unit_faith_purchase_enabled[1].value, -5);
        assert_eq!(g.encode(limits).unwrap(), gold_bytes());
        let g = Growth::decode(&growth_bytes(), limits).unwrap();
        assert_eq!(g.food_raw_q8, i32::MIN);
        assert_eq!(g.housing_from_great_people_modifier, -15);
        assert_eq!(g.amenities_from_religion_minimum_followers, 5);
        assert_eq!(g.feature_appeal_modifiers[0].definition_hash, u32::MAX);
        assert_eq!(g.encode(limits).unwrap(), growth_bytes());
    }
    macro_rules! refuse {
        ($name:ident, $fixture:expr, $bool_pos:expr) => {{
            let b = $fixture;
            for end in 0..b.len() {
                assert!(
                    $name::decode(&b[..end], Limits::default()).is_err(),
                    "{} {end}",
                    stringify!($name)
                );
            }
            let mut tail = b.clone();
            tail.push(99);
            assert_eq!(
                $name::decode(&tail, Limits::default()),
                Err(Error::Trailing)
            );
            assert_eq!(
                $name::decode_prefix(&tail, Limits::default()).unwrap().1,
                b.len()
            );
            let mut bad = b.clone();
            bad[0] = 0;
            assert!(matches!(
                $name::decode(&bad, Limits::default()),
                Err(Error::Version { .. })
            ));
            bad = b.clone();
            bad[$bool_pos] = 2;
            assert_eq!(
                $name::decode(&bad, Limits::default()),
                Err(Error::InvalidBool)
            );
            let value = $name::decode(&b, Limits::default()).unwrap();
            for limits in [
                Limits {
                    max_bytes: b.len() - 1,
                    max_items: 999,
                },
                Limits {
                    max_bytes: 9999,
                    max_items: 0,
                },
            ] {
                assert_eq!($name::decode(&b, limits), Err(Error::ResourceLimit));
                assert_eq!(value.encode(limits), Err(Error::ResourceLimit));
            }
        }};
    }
    #[test]
    fn economic_records_reject_truncation_versions_bools_and_limits() {
        refuse!(Culture, culture_bytes(), 52);
        refuse!(Gold, gold_bytes(), 132);
        refuse!(Growth, growth_bytes(), 44);
        let mut bytes = culture_bytes();
        bytes[40] = 16;
        assert_eq!(
            Culture::decode(&bytes, Limits::default()),
            Err(Error::Version {
                expected: 17,
                actual: 16
            })
        );
        let mut bytes = culture_bytes();
        bytes[66..70].copy_from_slice(&16i32.to_le_bytes());
        assert_eq!(
            Culture::decode(&bytes, Limits::default()),
            Err(Error::Version {
                expected: 17,
                actual: 16
            })
        );
        let tourism = TourismModifier::decode(&culture_bytes()[40..62], Limits::default()).unwrap();
        assert_eq!(
            tourism.encode(Limits::default()).unwrap(),
            culture_bytes()[40..62]
        );
        let great_work =
            GreatWorkYieldModifier::decode(&culture_bytes()[66..86], Limits::default()).unwrap();
        assert_eq!(
            great_work.encode(Limits::default()).unwrap(),
            culture_bytes()[66..86]
        );
        let mut bytes = growth_bytes();
        bytes[129..133].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            Growth::decode(&bytes, Limits::default()),
            Err(Error::ResourceLimit)
        );
    }
}
