//! Bounded Linux XP2 City service-component wire records, not live-state application.
//! Resources admits7/10 with explicit omitted-list effects; Combat admits6/7
//! with an explicit HP-modifier0 default. Trade admits10/12 with omitted
//! Yield-list zero effects. Other components
//! retain their separately checked current versions.
//! Target SHA-256 a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa.
//! W/R: Religion 0x28c28e0/0x28c29b0, Resources 0x28c6190/0x28c6270,
//! Combat 0x2871980/0x2871a10, Trade 0x28c71e0/0x28c73f0,
//! CulturalIdentity 0x2872220/0x2872340, Power 0x28be2d0/0x28be390.
//! Constants at 0x2f24994/0x2f249c4/0x2f21560/0x2f249c8/0x2f215f0/0x2f246e8.
//! Nested ReligionInCity uses v5; PlayerIdentityPresence v14; all power sources v10.
//! Names and live omissions checked using matching quoted C++ DWARF types.
//! ReligionTypes (0x2d4f730), LoyaltyLevels (0x2bf6eb0), ResourceTypes
//! (0x2d4ee80), BuildingTypes (0x2d4ed30), ProjectTypes (0x2d4f500) save hashes.
//! PlayerTypes (0x2c9aba0), FreePowerSourceTypes (0x2d50230) and
//! IdentityConversionOutcome (0x2bf6d20) save raw signed ints.
//! Yield indexed arrays use hash/value framing (0x2194800); Yield enum scalars
//! elsewhere must not be confused with these arrays. Terrain/Resource indexed
//! providers are 0x2194ef0/0x1fe6a00. CityID (0x2c9ad10) saves i16 owner,
//! i16 type, u32 full ID; PlotCoord (0x2c9af20) saves two i32 coordinates.
//! Q8 values remain raw signed bits. Unknown/duplicate hashes retain wire order.
//!
//! Resources Read resolves saved CityID and calls Initialize before overlays;
//! other read-side ownership, FAuto/allocator effects and omitted live fields
//! require separate application. ReligionInCity remainder is omitted. These
//! codecs never synthesize constructor defaults or infer gameplay equivalence.
//! Complete City/Player/world import, gameplay RNG, actions and turns remain
//! outside this bounded structural component transport.
use crate::unit::{HashValue, Limits};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Version { expected: i32, actual: i32 },
    InvalidBool,
    UnsupportedLegacyWrite,
    Trailing,
    ResourceLimit,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux City services input: {self:?}")
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
macro_rules! read_field {
    ($ty:ty, $r:expr, $version:expr) => {
        <$ty as Wire>::read($r)
    };
    ($ty:ty, $r:expr, $version:expr, $first:literal, $last:literal) => {
        if ($first..=$last).contains(&$version) {
            <$ty as Wire>::read($r)
        } else {
            Ok(<$ty as Default>::default())
        }
    };
}
macro_rules! write_field {
    ($value:expr, $ty:ty, $w:expr, $version:expr) => {
        $value.write($w)
    };
    ($value:expr, $ty:ty, $w:expr, $version:expr, $first:literal, $last:literal) => {
        if ($first..=$last).contains(&$version) {
            $value.write($w)
        } else if $value == &<$ty as Default>::default() {
            Ok(())
        } else {
            Err(Error::UnsupportedLegacyWrite)
        }
    };
}
macro_rules! record {
    ($name:ident, $version:expr $(, legacy $legacy:expr, $version_field:ident)?, { $( $(#[$attr:meta])* $field:ident: $ty:ty $(=> ($first:literal, $last:literal))? ),* $(,)? }) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name {
            $( pub $version_field: i32, )?
            $( $(#[$attr])* pub $field: $ty, )*
        }
        impl Wire for $name {
            const MIN_SIZE: usize = {
                let _minimum_version = $version;
                $( let _minimum_version = $legacy; )?
                4 $(+ if true $(&& _minimum_version >= $first && _minimum_version <= $last)? { <$ty as Wire>::MIN_SIZE } else { 0 })*
            };
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                let actual = i32::read(r)?;
                if actual != $version $( && actual != $legacy )? { return Err(Error::Version { expected: $version, actual }); }
                Ok(Self {
                    $( $version_field: actual, )?
                    $( $field: read_field!($ty, r, actual $(, $first, $last)?)?, )*
                })
            }
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                $( if self.$version_field != $version { return Err(Error::UnsupportedLegacyWrite); } )?
                ($version).write(w)?;
                $( write_field!(&self.$field, $ty, w, $version $(, $first, $last)?)?; )*
                Ok(())
            }
        }
        impl $name {
            /// Decode only at an independently established structural boundary.
            pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
                let (value, consumed, _) = Self::decode_prefix_with_items(bytes, limits)?;
                Ok((value, consumed))
            }
            /// Also report aggregate dynamic elements consumed by this record.
            /// Callers can subtract this count from a surrounding record budget.
            pub fn decode_prefix_with_items(bytes: &[u8], limits: Limits) -> Result<(Self, usize, usize), Error> {
                let mut r = Reader { bytes, pos: 0, budget: Budget { limits, items: 0, allocated: 0 } };
                let value = Self::read(&mut r)?;
                Ok((value, r.pos, r.budget.items))
            }
            pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
                let (value, consumed) = Self::decode_prefix(bytes, limits)?;
                if consumed != bytes.len() { return Err(Error::Trailing); }
                Ok(value)
            }
            pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
                self.encode_with_items(limits).map(|(bytes, _)| bytes)
            }
            /// Also report aggregate dynamic elements charged during encoding.
            pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
                let mut w = Writer { bytes: Vec::new(), budget: Budget { limits, items: 0, allocated: 0 } };
                self.write(&mut w)?;
                Ok((w.bytes, w.budget.items))
            }
        }
    };
}
pub const RELIGION_VERSION: i32 = 5;
pub const RESOURCES_VERSION: i32 = 10;
pub const COMBAT_VERSION: i32 = 7;
pub const TRADE_VERSION: i32 = 12;
pub const CULTURAL_IDENTITY_VERSION: i32 = 14;
pub const POWER_VERSION: i32 = 10;

impl Wire for i16 {
    const MIN_SIZE: usize = 2;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self::from_le_bytes(r.take(2)?.try_into().unwrap()))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.put(&self.to_le_bytes())
    }
}
macro_rules! plain_record {
 ($name:ident, { $($field:ident:$ty:ty),* $(,)? }) => {
 #[derive(Clone, Debug, Eq, PartialEq)]
 pub struct $name { $(pub $field:$ty,)* }
 impl Wire for $name {
 const MIN_SIZE:usize=0 $(+ <$ty as Wire>::MIN_SIZE)*;
 fn read(r:&mut Reader<'_>)->Result<Self,Error>{Ok(Self{$($field:<$ty as Wire>::read(r)?,)*})}
 fn write(&self,w:&mut Writer)->Result<(),Error>{$(self.$field.write(w)?;)* Ok(())}
 }
 };
}
plain_record!(CityId, {owner:i16, component_type:i16, id:u32});
plain_record!(PlotCoord, {x:i32,y:i32});
record!(ReligionInCity, RELIGION_VERSION, {
 religion_hash:u32, followers:i32, pressure_raw_q8:i32, initiation_bonus_paid:bool,
});
record!(Religion, RELIGION_VERSION, {
 majority_religion_hash:u32, religion_status:Vec<ReligionInCity>,
 next_religion_hash:u32, turns_to_next_religion:i32, pressure_modifier:i32,
 religious_combat_bonus:i32, extra_religious_heal:i32, religion_ignore_pressure:bool,
 religion_ignore_combat:bool, num_extra_promotions:i32, previous_real_religion_hash:u32,
});
record!(Resources, RESOURCES_VERSION, legacy 7, wire_version, {
 city_id:CityId, harvest_bonus_modifier:i32, resource_yield_by_count:Vec<HashValue<i32>> => (9, 10),
 yield_by_number_of_resources:Vec<HashValue<i32>> => (10, 10), extra_accumulation_modifier:i32,
 extra_accumulation_for_strategic_diversity_modifier:i32, extra_amenity_modifier:i32,
 extra_accumulation_terrain_modifier:Vec<HashValue<i32>>,
 extra_accumulation_specific_resource_modifier:Vec<HashValue<i32>> => (8, 10),
});
/// Missing wire pairs do not describe an empty effective definition array.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceListReadEffect {
    PresentPairs,
    ZeroResetDefinitionCountUnknown,
    ZeroResetByNativeInitializeDefinitionCountUnknown,
    ZeroForEachDefinitionRegistryUnknown,
}
impl Resources {
    pub fn resource_yield_by_count_effect(&self) -> ResourceListReadEffect {
        if self.wire_version == 7 {
            ResourceListReadEffect::ZeroResetDefinitionCountUnknown
        } else {
            ResourceListReadEffect::PresentPairs
        }
    }
    pub fn yield_by_number_of_resources_effect(&self) -> ResourceListReadEffect {
        if self.wire_version == 7 {
            ResourceListReadEffect::ZeroResetByNativeInitializeDefinitionCountUnknown
        } else {
            ResourceListReadEffect::PresentPairs
        }
    }
    pub fn specific_resource_accumulation_effect(&self) -> ResourceListReadEffect {
        if self.wire_version == 7 {
            ResourceListReadEffect::ZeroForEachDefinitionRegistryUnknown
        } else {
            ResourceListReadEffect::PresentPairs
        }
    }
}
record!(Combat, COMBAT_VERSION, legacy 6, wire_version, {
 outer_defense_strength_modifier:i32, inner_defense_strength_modifier:i32,
 ranged_strike_strength_modifier:i32, siege_protection:bool, anti_air_defense_bonus:i32,
 city_combat_bonus:i32, city_friendly_combat_bonus:i32, attack_range_modifier:i32,
 hit_points_modifier:i32 => (7, 7),
});
record!(Trade, TRADE_VERSION, legacy 10, wire_version, {
 constructed_trading_posts:Vec<bool>, num_foreign_routes_passing_through:i32,
 flat_yield_bonus_to_others_for_domestic:Vec<HashValue<i32>>,
 flat_yield_bonus_to_others:Vec<HashValue<i32>>,
 flat_yield_bonus_from_others:Vec<HashValue<i32>>,
 yield_per_local_bonus_resource_for_domestic:Vec<HashValue<i32>>,
 yield_per_local_bonus_resource_for_international:Vec<HashValue<i32>>,
 yield_per_destination_luxury_resource_for_international:Vec<HashValue<i32>>,
 yield_per_destination_strategic_resource_for_domestic:Vec<HashValue<i32>>,
 yield_per_destination_strategic_resource_for_international:Vec<HashValue<i32>>,
 flat_yield_bonus_for_domestic:Vec<HashValue<i32>>,
 flat_yield_bonus_for_international:Vec<HashValue<i32>>,
 yield_from_foreign_routes_passing_through:Vec<HashValue<i32>>,
 yield_per_local_luxury_resource_for_international:Vec<HashValue<i32>> => (11, 12),
 yield_per_local_strategic_resource_for_international:Vec<HashValue<i32>> => (11, 12),
 yield_per_major_trade_partner:Vec<HashValue<i32>> => (12, 12),
});
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TradeLateYieldReadEffect {
    PresentPairs,
    ZeroForEachYieldDefinitionRegistryUnknown,
}
impl Trade {
    /// Applies to local Luxury/Strategic international yields and major-partner
    /// yields. No wire pairs at10 does not mean an empty native Yield array.
    pub fn later_yield_arrays_read_effect(&self) -> TradeLateYieldReadEffect {
        if self.wire_version == 10 {
            TradeLateYieldReadEffect::ZeroForEachYieldDefinitionRegistryUnknown
        } else {
            TradeLateYieldReadEffect::PresentPairs
        }
    }
}
record!(PlayerIdentityPresence, CULTURAL_IDENTITY_VERSION, {
 player:i32, identity_total_raw_q8:i32, rejected_city:bool,
});
record!(CulturalIdentity, CULTURAL_IDENTITY_VERSION, {
 loyalty_raw_q8:i32, last_loyalty_change_raw_q8:i32,
 identities_present:Vec<PlayerIdentityPresence>, conversion_outcome:i32,
 turns_to_conversion:i32, transfer_to:i32, identity_pressure_from_ages_raw_q8:i32,
 identity_pressure_from_governor_on_domestic_raw_q8:i32,
 identity_pressure_from_governor_on_foreign_raw_q8:i32,
 identity_pressure_from_modifiers_raw_q8:i32, turn_last_notified_losing_cultural_identity:i32,
 martial_law_target:i32, always_loyal_flag_count:i32, identity_per_turn_from_modifiers_raw_q8:i32,
 loyalty_level_hash:u32, extra_identity_per_turn_per_citizen_raw_q8:i32,
});
record!(FreePowerSource, POWER_VERSION, {
 amount_raw_q8:i32, source:i32, num_unique_sources:i32, location:PlotCoord,
});
record!(TemporaryPowerSource, POWER_VERSION, {
 amount_raw_q8:i32, source_resource_hash:u32, from_city:CityId,
});
record!(RequiredPowerSource, POWER_VERSION, {
 amount_raw_q8:i32, building_source_hash:u32, project_source_hash:u32,
});
record!(GeneratedPowerSource, POWER_VERSION, {
 amount_raw_q8:i32, source_resource_hash:u32, source_consumed_raw_q8:i32,
 to_city:CityId, location:PlotCoord,
});
record!(Power, POWER_VERSION, {
 free_power_raw_q8:i32, temporary_power_raw_q8:i32, required_power_total_raw_q8:i32,
 required_power_from_modifiers_raw_q8:i32, ever_generated_power:bool,
 turn_last_notified_need_power:i32, free_power_sources:Vec<FreePowerSource>,
 temporary_power_sources:Vec<TemporaryPowerSource>, required_power_sources:Vec<RequiredPowerSource>,
 generated_power_sources:Vec<GeneratedPowerSource>, modified_free_power:i32,
});

#[cfg(test)]
mod tests {
    use super::*;
    fn ints(b: &mut Vec<u8>, xs: &[i32]) {
        for x in xs {
            b.extend(x.to_le_bytes());
        }
    }
    fn id(b: &mut Vec<u8>) {
        b.extend((-7i16).to_le_bytes());
        b.extend((123i16).to_le_bytes());
        b.extend(u32::MAX.to_le_bytes());
    }
    fn pairs(b: &mut Vec<u8>, value: i32) {
        ints(b, &[2, -1, value, -1, -value]);
    }
    fn religion() -> Vec<u8> {
        let mut b = Vec::new();
        ints(&mut b, &[5, -1, 1, 5, -2, -3, i32::MIN]);
        b.push(1);
        ints(&mut b, &[-4, 7, 8, 9, 10]);
        b.extend([1, 0]);
        ints(&mut b, &[11, -5]);
        b
    }
    fn resources() -> Vec<u8> {
        let mut b = Vec::new();
        ints(&mut b, &[10]);
        id(&mut b);
        ints(&mut b, &[-1]);
        pairs(&mut b, 11);
        pairs(&mut b, 22);
        ints(&mut b, &[31, 32, 33]);
        pairs(&mut b, 44);
        pairs(&mut b, 55);
        b
    }
    fn combat() -> Vec<u8> {
        let mut b = Vec::new();
        ints(&mut b, &[7, -1, 2, -3]);
        b.push(1);
        ints(&mut b, &[4, -5, 6, -7, i32::MIN]);
        b
    }
    fn trade() -> Vec<u8> {
        let mut b = Vec::new();
        ints(&mut b, &[12, 3]);
        b.extend([1, 0, 1]);
        ints(&mut b, &[-1]);
        for i in 1..=14 {
            pairs(&mut b, i);
        }
        b
    }
    fn identity() -> Vec<u8> {
        let mut b = Vec::new();
        ints(&mut b, &[14, i32::MIN, i32::MAX, 1, 14, 64, -257]);
        b.push(1);
        for i in 1..=13 {
            ints(&mut b, &[-i]);
        }
        b
    }
    fn power() -> Vec<u8> {
        let mut b = Vec::new();
        ints(&mut b, &[10, -1, -2, -3, -4]);
        b.push(1);
        ints(&mut b, &[-5, 1, 10, -6, -7, -8, -9, -10, 1, 10, -11, -12]);
        id(&mut b);
        ints(&mut b, &[1, 10, -13, -14, -15, 1, 10, -16, -17, -18]);
        id(&mut b);
        ints(&mut b, &[-19, -20, -21]);
        b
    }
    macro_rules! check {
        ($ty:ident,$fixture:expr,$bool:expr,$items:expr) => {{
            let b = $fixture;
            let limits = Limits::default();
            let v = $ty::decode(&b, limits).unwrap();
            assert_eq!(v.encode(limits).unwrap(), b);
            for end in 0..b.len() {
                assert!(
                    $ty::decode(&b[..end], limits).is_err(),
                    "{} {end}",
                    stringify!($ty)
                );
            }
            let mut tail = b.clone();
            tail.push(0);
            assert_eq!($ty::decode(&tail, limits), Err(Error::Trailing));
            assert_eq!($ty::decode_prefix(&tail, limits).unwrap().1, b.len());
            let mut bad = b.clone();
            bad[0] = 0;
            assert!(matches!(
                $ty::decode(&bad, limits),
                Err(Error::Version { .. })
            ));
            if $bool != usize::MAX {
                bad = b.clone();
                bad[$bool] = 2;
                assert_eq!($ty::decode(&bad, limits), Err(Error::InvalidBool));
            }
            let short = Limits {
                max_bytes: b.len() - 1,
                max_items: 999,
            };
            assert_eq!($ty::decode(&b, short), Err(Error::ResourceLimit));
            assert_eq!(v.encode(short), Err(Error::ResourceLimit));
            if $items {
                let none = Limits {
                    max_bytes: 9999,
                    max_items: 0,
                };
                assert_eq!($ty::decode(&b, none), Err(Error::ResourceLimit));
                assert_eq!(v.encode(none), Err(Error::ResourceLimit));
            }
            v
        }};
    }
    #[test]
    fn trade10_omits_three_late_yield_lists_with_per_definition_zero() {
        let mut bytes = Vec::new();
        ints(&mut bytes, &[10, 3]);
        bytes.extend([1, 0, 1]);
        ints(&mut bytes, &[-1]);
        for i in 1..=11 {
            pairs(&mut bytes, i);
        }
        let end = bytes.len();
        bytes.extend([0xaa, 0xbb]);
        let limits = Limits {
            max_bytes: end,
            max_items: 25,
        };
        let (value, used, items) = Trade::decode_prefix_with_items(&bytes, limits).unwrap();
        assert_eq!((used, items), (end, 25));
        assert_eq!(value.wire_version, 10);
        assert_eq!(value.constructed_trading_posts, [true, false, true]);
        assert_eq!(
            value.yield_from_foreign_routes_passing_through,
            [
                HashValue {
                    definition_hash: u32::MAX,
                    value: 11
                },
                HashValue {
                    definition_hash: u32::MAX,
                    value: -11
                }
            ]
        );
        assert!(value
            .yield_per_local_luxury_resource_for_international
            .is_empty());
        assert!(value
            .yield_per_local_strategic_resource_for_international
            .is_empty());
        assert!(value.yield_per_major_trade_partner.is_empty());
        assert_eq!(
            value.later_yield_arrays_read_effect(),
            TradeLateYieldReadEffect::ZeroForEachYieldDefinitionRegistryUnknown
        );
        assert_eq!(value.encode(limits), Err(Error::UnsupportedLegacyWrite));
        for n in 0..end {
            assert!(Trade::decode_prefix(&bytes[..n], limits).is_err(), "{n}");
        }
        assert_eq!(
            Trade::decode_prefix(
                &bytes,
                Limits {
                    max_items: 24,
                    ..limits
                }
            ),
            Err(Error::ResourceLimit)
        );
        for version in [9i32, 11, 13] {
            let mut bad = bytes.clone();
            bad[..4].copy_from_slice(&version.to_le_bytes());
            assert_eq!(
                Trade::decode_prefix(&bad, limits),
                Err(Error::Version {
                    expected: 12,
                    actual: version
                })
            );
        }
        let current = Trade::decode(&trade(), Limits::default()).unwrap();
        assert_eq!(
            current.later_yield_arrays_read_effect(),
            TradeLateYieldReadEffect::PresentPairs
        );
        assert_eq!(current.encode(Limits::default()).unwrap(), trade());
    }
    #[test]
    fn combat6_omits_only_hit_points_with_explicit_zero() {
        let mut bytes = Vec::new();
        ints(&mut bytes, &[6, -1, 2, -3]);
        bytes.push(1);
        ints(&mut bytes, &[4, -5, 6, -7]);
        let end = bytes.len();
        bytes.extend([0xaa, 0xbb]);
        let limits = Limits {
            max_bytes: end,
            max_items: 0,
        };
        let (value, used, items) = Combat::decode_prefix_with_items(&bytes, limits).unwrap();
        assert_eq!((used, items), (end, 0));
        assert_eq!(value.wire_version, 6);
        assert_eq!(value.hit_points_modifier, 0);
        assert_eq!(value.attack_range_modifier, -7);
        assert!(value.siege_protection);
        assert_eq!(value.encode(limits), Err(Error::UnsupportedLegacyWrite));
        for n in 0..end {
            assert!(Combat::decode_prefix(&bytes[..n], limits).is_err(), "{n}");
        }
        assert_eq!(
            Combat::decode_prefix(
                &bytes,
                Limits {
                    max_bytes: end - 1,
                    ..limits
                }
            ),
            Err(Error::ResourceLimit)
        );
        for version in [5i32, 8] {
            let mut bad = bytes.clone();
            bad[..4].copy_from_slice(&version.to_le_bytes());
            assert_eq!(
                Combat::decode_prefix(&bad, limits),
                Err(Error::Version {
                    expected: 7,
                    actual: version
                })
            );
        }
        let mut bad = bytes.clone();
        bad[16] = 2;
        assert_eq!(Combat::decode_prefix(&bad, limits), Err(Error::InvalidBool));
        let current = Combat::decode(&combat(), Limits::default()).unwrap();
        assert_eq!(current.hit_points_modifier, i32::MIN);
        assert_eq!(current.encode(Limits::default()).unwrap(), combat());
    }
    #[test]
    fn resources7_omitted_lists_keep_proven_initialization_effects() {
        let mut bytes = Vec::new();
        ints(&mut bytes, &[7]);
        id(&mut bytes);
        ints(&mut bytes, &[-1, 31, 32, 33]);
        pairs(&mut bytes, 44);
        let end = bytes.len();
        bytes.extend([0xaa, 0xbb]);
        let limits = Limits {
            max_bytes: end,
            max_items: 2,
        };
        let (value, used, items) = Resources::decode_prefix_with_items(&bytes, limits).unwrap();
        assert_eq!((used, items), (end, 2));
        assert_eq!(value.wire_version, 7);
        assert_eq!(
            (
                value.city_id.owner,
                value.city_id.component_type,
                value.city_id.id
            ),
            (-7, 123, u32::MAX)
        );
        assert_eq!(
            (
                value.harvest_bonus_modifier,
                value.extra_accumulation_modifier,
                value.extra_accumulation_for_strategic_diversity_modifier,
                value.extra_amenity_modifier
            ),
            (-1, 31, 32, 33)
        );
        assert!(value.resource_yield_by_count.is_empty());
        assert!(value.yield_by_number_of_resources.is_empty());
        assert!(value
            .extra_accumulation_specific_resource_modifier
            .is_empty());
        assert_eq!(
            value.resource_yield_by_count_effect(),
            ResourceListReadEffect::ZeroResetDefinitionCountUnknown
        );
        assert_eq!(
            value.yield_by_number_of_resources_effect(),
            ResourceListReadEffect::ZeroResetByNativeInitializeDefinitionCountUnknown
        );
        assert_eq!(
            value.specific_resource_accumulation_effect(),
            ResourceListReadEffect::ZeroForEachDefinitionRegistryUnknown
        );
        assert_eq!(
            value.extra_accumulation_terrain_modifier,
            [
                HashValue {
                    definition_hash: u32::MAX,
                    value: 44
                },
                HashValue {
                    definition_hash: u32::MAX,
                    value: -44
                }
            ]
        );
        assert_eq!(value.encode(limits), Err(Error::UnsupportedLegacyWrite));
        for n in 0..end {
            assert!(
                Resources::decode_prefix(&bytes[..n], limits).is_err(),
                "{n}"
            );
        }
        assert_eq!(
            Resources::decode_prefix(
                &bytes,
                Limits {
                    max_items: 1,
                    ..limits
                }
            ),
            Err(Error::ResourceLimit)
        );
        for version in [6i32, 8, 9, 11] {
            let mut bad = bytes.clone();
            bad[..4].copy_from_slice(&version.to_le_bytes());
            assert_eq!(
                Resources::decode_prefix(&bad, limits),
                Err(Error::Version {
                    expected: 10,
                    actual: version
                })
            );
        }
        let current = Resources::decode(&resources(), Limits::default()).unwrap();
        assert_eq!(
            current.resource_yield_by_count_effect(),
            ResourceListReadEffect::PresentPairs
        );
        assert_eq!(current.encode(Limits::default()).unwrap(), resources());
    }
    #[test]
    fn service_records_nonempty_source_fixtures_and_refusals() {
        let r = check!(Religion, religion(), 28, true);
        assert_eq!(r.religion_status[0].pressure_raw_q8, i32::MIN);
        assert_eq!(r.previous_real_religion_hash, u32::MAX - 4);
        let r = check!(Resources, resources(), usize::MAX, true);
        assert_eq!(r.city_id.owner, -7);
        assert_eq!(r.city_id.component_type, 123);
        assert_eq!(r.city_id.id, u32::MAX);
        assert_eq!(
            r.extra_accumulation_specific_resource_modifier[1].value,
            -55
        );
        let c = check!(Combat, combat(), 16, false);
        assert_eq!(c.hit_points_modifier, i32::MIN);
        let t = check!(Trade, trade(), 8, true);
        assert_eq!(t.flat_yield_bonus_for_domestic[0].value, 9);
        assert_eq!(t.yield_per_major_trade_partner[1].value, -14);
        let c = check!(CulturalIdentity, identity(), 28, true);
        assert_eq!(c.identities_present[0].player, 64);
        assert_eq!(c.loyalty_level_hash, (-12i32) as u32);
        assert_eq!(c.extra_identity_per_turn_per_citizen_raw_q8, -13);
        let p = check!(Power, power(), 20, true);
        assert_eq!(
            p.free_power_sources[0].location,
            PlotCoord { x: -9, y: -10 }
        );
        assert_eq!(p.generated_power_sources[0].source_consumed_raw_q8, -18);
        assert_eq!(p.modified_free_power, -21);
    }
    #[test]
    fn service_nested_versions_and_counts_are_checked() {
        let mut b = religion();
        b[12] = 4;
        assert!(matches!(
            Religion::decode(&b, Limits::default()),
            Err(Error::Version {
                expected: 5,
                actual: 4
            })
        ));
        b = power();
        b[29] = 9;
        assert!(matches!(
            Power::decode(&b, Limits::default()),
            Err(Error::Version {
                expected: 10,
                actual: 9
            })
        ));
        b = trade();
        b[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            Trade::decode(&b, Limits::default()),
            Err(Error::ResourceLimit)
        );
        let limits = Limits {
            max_bytes: 9999,
            max_items: 27,
        };
        assert_eq!(Trade::decode(&trade(), limits), Err(Error::ResourceLimit));
    }
    #[test]
    fn item_accounting_composes_without_consuming_following_records() {
        let bytes = trade();
        let mut followed = bytes.clone();
        followed.extend(combat());
        let exact = Limits {
            max_bytes: bytes.len(),
            max_items: 31,
        };
        let (value, consumed, items) = Trade::decode_prefix_with_items(&followed, exact).unwrap();
        assert_eq!((consumed, items), (bytes.len(), 31));
        assert_eq!(value.encode_with_items(exact).unwrap(), (bytes, 31));
        let short = Limits {
            max_items: 30,
            ..exact
        };
        assert_eq!(
            Trade::decode_prefix_with_items(&followed, short),
            Err(Error::ResourceLimit)
        );
        assert_eq!(value.encode_with_items(short), Err(Error::ResourceLimit));
        let (value, consumed, items) =
            Combat::decode_prefix_with_items(&combat(), Limits::default()).unwrap();
        assert_eq!((consumed, items), (combat().len(), 0));
        assert_eq!(value.encode_with_items(Limits::default()).unwrap().1, 0);
    }
}
