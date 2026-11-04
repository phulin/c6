//! Bounded Linux XP2 Player.Religion v15/v20 wire transport, with strict nested records.
//! No live Reset, definition remapping, callbacks, gameplay, actions or turns.
use crate::unit::{HashValue, Limits};

pub const VERSION: u32 = 20;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    UnsupportedLegacyWrite,
    Trailing,
    ResourceLimit,
    InvalidBool(u8),
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player.Religion wire: {self:?}")
    }
}
impl std::error::Error for Error {}
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    items: usize,
    limits: Limits,
    version_component: &'static str,
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
    fn allocate<T>(&self, n: usize) -> Result<(), Error> {
        // Limits.max_bytes bounds wire consumption and each individual vector
        // allocation, matching unit::Limits; it is not a cumulative heap cap.
        let bytes = n
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(Error::ResourceLimit)?;
        if bytes > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
    fn version(&mut self, component: &'static str) -> Result<(), Error> {
        self.version_component = component;
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
impl<T: Wire> Wire for HashValue<T> {
    const MIN: usize = 4 + T::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            definition_hash: u32::read(r)?,
            value: T::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.definition_hash.write(w)?;
        self.value.write(w)
    }
}
macro_rules! record {
    ($name:ident, $version:expr, { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name { $(pub $field: $ty),* }
        impl Wire for $name {
            const MIN: usize = (if $version { 4 } else { 0 }) $(+ <$ty as Wire>::MIN)*;
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                if $version { r.version(stringify!($name))?; }
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
record!(PlotCoord, false, {x:i32,y:i32});
record!(BeliefYield, true, {belief_yield_ordinal:i32,yield_ordinal:i32,amount:i32,per_x_items:i32,district_hash:u32});
record!(ReligionBuildingYieldModifier, true, {yield_ordinal:i32,multiplier:i32,founder_only:bool});
/// Player.Religion15/20 wire/default projection. Absent15 vectors retain
/// distinct native Read effects; empty wire vectors are not native empty state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Religion {
    pub wire_version: u32,
    pub faith_balance_q8: i32,
    pub faith_rate_change_q8: i32,
    pub pantheon_hash: u32,
    pub num_beliefs_earned: i32,
    pub holy_city_coord: PlotCoord,
    pub belief_yields: Vec<BeliefYield>,
    pub building_yield_modifiers: Vec<ReligionBuildingYieldModifier>,
    pub religion_building_discount: i32,
    pub spread_distance: i32,
    pub combat_loss: i32,
    pub inquisition: i32,
    pub gains_all_follower_beliefs: bool,
    pub founder_belief_of_majority_religion: bool,
    pub faith_per_peaceful_religion_founder: i32,
    pub add_faith_harvest_modifier: i32,
    pub add_faith_chop_modifier: i32,
    pub add_faith_barbarian_plunder_modifier: i32,
    pub adopts_ally_founded_religions: i32,
    pub treat_ally_capital_as_holy_city: i32,
    pub treat_holy_sites_as_holy_city: i32,
    pub spread_multiplier: i32,
    pub spread_enhancing_tech_hash: u32,
    pub tech_enabled_spread_multiplier: i32,
    pub natural_wonder_relic: i32,
    pub faith_from_dispersal: i32,
    pub religious_pressure_per_great_person_activated: i32,
    pub enable_wall_attack_whole_game_same_religion: i32,
    pub enable_wall_attack_whole_game_same_religion_promotion_class: Vec<HashValue<i32>>,
    pub start_inquisition_charges: i32,
    pub spread_trade_route_pressure_destination_modifier: i32,
    pub spread_trade_route_pressure_origin_modifier: i32,
    pub great_person_founder_hash: u32,
    pub units_religion_strength_by_religion: Vec<i32>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReligionArrayReadEffect {
    /// Zero at each actual promotion-class definition's collection index.
    /// Registry/count, dense ordering and unrepresented slots are not inferred.
    ZeroAtActualPromotionClassIndexes,
    UnknownRetainedBaseline,
}
impl Religion {
    pub fn promotion_class_wall_attack_read_effect(&self) -> Option<ReligionArrayReadEffect> {
        (self.wire_version == 15)
            .then_some(ReligionArrayReadEffect::ZeroAtActualPromotionClassIndexes)
    }
    pub fn units_religion_strength_read_effect(&self) -> Option<ReligionArrayReadEffect> {
        (self.wire_version == 15).then_some(ReligionArrayReadEffect::UnknownRetainedBaseline)
    }
}
impl Wire for Religion {
    const MIN: usize = 4
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <PlotCoord as Wire>::MIN
        + <Vec<BeliefYield> as Wire>::MIN
        + <Vec<ReligionBuildingYieldModifier> as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <bool as Wire>::MIN
        + <bool as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <u32 as Wire>::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version_component = "Religion";
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 15 | 20) {
            return Err(Error::Version {
                expected: VERSION,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            faith_balance_q8: <i32 as Wire>::read(r)?,
            faith_rate_change_q8: <i32 as Wire>::read(r)?,
            pantheon_hash: <u32 as Wire>::read(r)?,
            num_beliefs_earned: <i32 as Wire>::read(r)?,
            holy_city_coord: <PlotCoord as Wire>::read(r)?,
            belief_yields: <Vec<BeliefYield> as Wire>::read(r)?,
            building_yield_modifiers: <Vec<ReligionBuildingYieldModifier> as Wire>::read(r)?,
            religion_building_discount: <i32 as Wire>::read(r)?,
            spread_distance: <i32 as Wire>::read(r)?,
            combat_loss: <i32 as Wire>::read(r)?,
            inquisition: <i32 as Wire>::read(r)?,
            gains_all_follower_beliefs: <bool as Wire>::read(r)?,
            founder_belief_of_majority_religion: <bool as Wire>::read(r)?,
            faith_per_peaceful_religion_founder: <i32 as Wire>::read(r)?,
            add_faith_harvest_modifier: <i32 as Wire>::read(r)?,
            add_faith_chop_modifier: <i32 as Wire>::read(r)?,
            add_faith_barbarian_plunder_modifier: <i32 as Wire>::read(r)?,
            adopts_ally_founded_religions: <i32 as Wire>::read(r)?,
            treat_ally_capital_as_holy_city: <i32 as Wire>::read(r)?,
            treat_holy_sites_as_holy_city: <i32 as Wire>::read(r)?,
            spread_multiplier: <i32 as Wire>::read(r)?,
            spread_enhancing_tech_hash: <u32 as Wire>::read(r)?,
            tech_enabled_spread_multiplier: <i32 as Wire>::read(r)?,
            natural_wonder_relic: <i32 as Wire>::read(r)?,
            faith_from_dispersal: <i32 as Wire>::read(r)?,
            religious_pressure_per_great_person_activated: if wire_version == 20 {
                <i32 as Wire>::read(r)?
            } else {
                0
            },
            enable_wall_attack_whole_game_same_religion: if wire_version == 20 {
                <i32 as Wire>::read(r)?
            } else {
                0
            },
            enable_wall_attack_whole_game_same_religion_promotion_class: if wire_version == 20 {
                <Vec<HashValue<i32>> as Wire>::read(r)?
            } else {
                Vec::new()
            },
            start_inquisition_charges: <i32 as Wire>::read(r)?,
            spread_trade_route_pressure_destination_modifier: if wire_version == 20 {
                <i32 as Wire>::read(r)?
            } else {
                0
            },
            spread_trade_route_pressure_origin_modifier: if wire_version == 20 {
                <i32 as Wire>::read(r)?
            } else {
                0
            },
            great_person_founder_hash: <u32 as Wire>::read(r)?,
            units_religion_strength_by_religion: if wire_version == 20 {
                <Vec<i32> as Wire>::read(r)?
            } else {
                Vec::new()
            },
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != VERSION {
            return Err(Error::UnsupportedLegacyWrite);
        }
        VERSION.write(w)?;
        self.faith_balance_q8.write(w)?;
        self.faith_rate_change_q8.write(w)?;
        self.pantheon_hash.write(w)?;
        self.num_beliefs_earned.write(w)?;
        self.holy_city_coord.write(w)?;
        self.belief_yields.write(w)?;
        self.building_yield_modifiers.write(w)?;
        self.religion_building_discount.write(w)?;
        self.spread_distance.write(w)?;
        self.combat_loss.write(w)?;
        self.inquisition.write(w)?;
        self.gains_all_follower_beliefs.write(w)?;
        self.founder_belief_of_majority_religion.write(w)?;
        self.faith_per_peaceful_religion_founder.write(w)?;
        self.add_faith_harvest_modifier.write(w)?;
        self.add_faith_chop_modifier.write(w)?;
        self.add_faith_barbarian_plunder_modifier.write(w)?;
        self.adopts_ally_founded_religions.write(w)?;
        self.treat_ally_capital_as_holy_city.write(w)?;
        self.treat_holy_sites_as_holy_city.write(w)?;
        self.spread_multiplier.write(w)?;
        self.spread_enhancing_tech_hash.write(w)?;
        self.tech_enabled_spread_multiplier.write(w)?;
        self.natural_wonder_relic.write(w)?;
        self.faith_from_dispersal.write(w)?;
        self.religious_pressure_per_great_person_activated
            .write(w)?;
        self.enable_wall_attack_whole_game_same_religion.write(w)?;
        self.enable_wall_attack_whole_game_same_religion_promotion_class
            .write(w)?;
        self.start_inquisition_charges.write(w)?;
        self.spread_trade_route_pressure_destination_modifier
            .write(w)?;
        self.spread_trade_route_pressure_origin_modifier.write(w)?;
        self.great_person_founder_hash.write(w)?;
        self.units_religion_strength_by_religion.write(w)?;
        Ok(())
    }
}

/// Exact Rust decoder cursor before refusal. A direct version error includes
/// its4-byte tag; this is not an original native execution failure PC.
#[derive(Debug, Eq, PartialEq)]
pub struct Boundary {
    pub consumed: usize,
    pub component: &'static str,
    pub error: Error,
}
impl Religion {
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        Self::decode_prefix_with_boundary(bytes, limits).map_err(|b| b.error)
    }
    pub fn decode_prefix_with_boundary(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Boundary> {
        let mut r = Reader {
            data: bytes,
            pos: 0,
            items: 0,
            limits,
            version_component: "Religion",
        };
        match Self::read(&mut r) {
            Ok(v) => Ok((v, r.pos, r.items)),
            Err(error) => Err(Boundary {
                consumed: r.pos,
                component: if matches!(error, Error::Version { .. }) {
                    r.version_component
                } else {
                    "Religion field"
                },
                error,
            }),
        }
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
    // Independent source carrier follows the native writer's order, with nonempty
    // vectors, high-bit hashes, signed Q8, noncanonical i32 b-prefixed fields.
    fn source() -> Vec<u8> {
        let mut b = Vec::new();
        let word = |b: &mut Vec<u8>, x: i32| b.extend_from_slice(&x.to_le_bytes());
        for x in [
            20,
            -257,
            513,
            0x81234567u32 as i32,
            3,
            -8,
            9,
            1,
            20,
            7,
            -1,
            -90,
            4,
            0xfedcba98u32 as i32,
            1,
            20,
            5,
            -71,
        ] {
            word(&mut b, x)
        }
        b.push(1);
        for x in [-11, 12, -13, 42] {
            word(&mut b, x)
        }
        b.extend_from_slice(&[0, 1]);
        for x in 1..=14 {
            word(&mut b, -x)
        }
        word(&mut b, 2);
        for x in [
            0x87654321u32 as i32,
            -17,
            0x87654321u32 as i32,
            18,
            19,
            -20,
            21,
            0xabcdef01u32 as i32,
            3,
            -22,
            23,
            -24,
        ] {
            word(&mut b, x)
        }
        b
    }
    #[test]
    fn religion15_literal_defaults_keep_distinct_native_array_effects() {
        fn words(b: &mut Vec<u8>, values: &[i32]) {
            for v in values {
                b.extend(v.to_le_bytes());
            }
        }
        let mut b = Vec::new();
        words(&mut b, &[15, -257, 513, 0x81234567u32 as i32, 3, -8, 9, 1]);
        let belief_tag = b.len();
        words(&mut b, &[20, 7, -1, -90, 4, 0xfedcba98u32 as i32]);
        words(&mut b, &[1, 20, 5, -71]);
        b.push(1);
        words(&mut b, &[-11, 12, -13, 42]);
        b.extend([0, 1]);
        for v in 1..=12 {
            words(&mut b, &[-v]);
        }
        // No pressure18, wall19, promotion-class20 array, or route17 modifiers.
        words(&mut b, &[-20, 0xabcdef01u32 as i32]);
        // The since16 units-strength vector is absent, with unknown prior state.
        let end = b.len();
        b.extend([0xaa, 0xbb]);
        let exact = Limits {
            max_bytes: end,
            max_items: 2,
        };
        let (v, used, items) = Religion::decode_prefix_with_items(&b, exact).unwrap();
        assert_eq!((used, items, v.wire_version), (end, 2, 15));
        assert_eq!(v.faith_balance_q8, -257);
        assert_eq!(v.pantheon_hash, 0x81234567);
        assert_eq!(v.belief_yields[0].district_hash, 0xfedcba98);
        assert!(v.building_yield_modifiers[0].founder_only);
        assert_eq!(v.inquisition, 42);
        assert_eq!(v.faith_from_dispersal, -12);
        assert_eq!(
            (
                v.religious_pressure_per_great_person_activated,
                v.enable_wall_attack_whole_game_same_religion
            ),
            (0, 0)
        );
        assert_eq!(
            (
                v.spread_trade_route_pressure_destination_modifier,
                v.spread_trade_route_pressure_origin_modifier
            ),
            (0, 0)
        );
        assert_eq!(v.start_inquisition_charges, -20);
        assert_eq!(v.great_person_founder_hash, 0xabcdef01);
        assert!(v
            .enable_wall_attack_whole_game_same_religion_promotion_class
            .is_empty());
        assert!(v.units_religion_strength_by_religion.is_empty());
        assert_eq!(
            v.promotion_class_wall_attack_read_effect(),
            Some(ReligionArrayReadEffect::ZeroAtActualPromotionClassIndexes)
        );
        assert_eq!(
            v.units_religion_strength_read_effect(),
            Some(ReligionArrayReadEffect::UnknownRetainedBaseline)
        );
        let mut w = Writer {
            data: vec![0xaa],
            items: 0,
            limits: exact,
        };
        assert_eq!(v.write(&mut w), Err(Error::UnsupportedLegacyWrite));
        assert_eq!(w.data, [0xaa]);
        for n in 0..end {
            assert!(Religion::decode_prefix(&b[..n], exact).is_err(), "{n}");
        }
        assert_eq!(
            Religion::decode_prefix(
                &b,
                Limits {
                    max_items: 1,
                    ..exact
                }
            ),
            Err(Error::ResourceLimit)
        );
        let mut nested = b.clone();
        nested[belief_tag..belief_tag + 4].copy_from_slice(&15u32.to_le_bytes());
        let boundary = Religion::decode_prefix_with_boundary(&nested, exact).unwrap_err();
        assert_eq!(
            (boundary.component, boundary.consumed),
            ("BeliefYield", belief_tag + 4)
        );
        assert_eq!(
            boundary.error,
            Error::Version {
                expected: 20,
                actual: 15
            }
        );
        assert_eq!(
            Religion::decode_prefix(&nested, exact),
            Err(Error::Version {
                expected: 20,
                actual: 15
            })
        );
        let mut unknown = b.clone();
        unknown[..4].copy_from_slice(&16u32.to_le_bytes());
        assert_eq!(
            Religion::decode_prefix(&unknown, exact),
            Err(Error::Version {
                expected: 20,
                actual: 16
            })
        );
    }
    #[test]
    fn nonempty_source() {
        let b = source();
        let (v, n, i) = Religion::decode_prefix_with_items(&b, limits()).unwrap();
        assert_eq!(n, b.len());
        assert_eq!(v.wire_version, 20);
        assert_eq!(v.promotion_class_wall_attack_read_effect(), None);
        assert_eq!(v.units_religion_strength_read_effect(), None);
        assert_eq!(i, 7);
        assert_eq!(v.faith_balance_q8, -257);
        assert_eq!(v.pantheon_hash, 0x81234567);
        assert_eq!(v.belief_yields[0].district_hash, 0xfedcba98);
        assert!(v.building_yield_modifiers[0].founder_only);
        assert_eq!(v.inquisition, 42);
        assert_eq!(
            v.enable_wall_attack_whole_game_same_religion_promotion_class[0].definition_hash,
            v.enable_wall_attack_whole_game_same_religion_promotion_class[1].definition_hash
        );
        assert_eq!(v.units_religion_strength_by_religion, [-22, 23, -24]);
        assert_eq!(v.encode_with_items(limits()).unwrap(), (b, i));
    }
    #[test]
    fn all_truncations() {
        let b = source();
        for n in 0..b.len() {
            assert!(Religion::decode(&b[..n], limits()).is_err(), "{n}");
        }
    }
    #[test]
    fn strict_versions_bool_and_trailing() {
        let b = source();
        for p in [0, 32, 60] {
            let mut bad = b.clone();
            bad[p] = 19;
            assert!(
                matches!(Religion::decode(&bad, limits()), Err(Error::Version { .. })),
                "{p}"
            );
        }
        for p in [72, 89, 90] {
            let mut bad = b.clone();
            bad[p] = 2;
            assert!(
                matches!(Religion::decode(&bad, limits()), Err(Error::InvalidBool(2))),
                "{p}"
            );
        }
        let mut bad = b.clone();
        bad.push(0);
        assert_eq!(Religion::decode(&bad, limits()), Err(Error::Trailing));
        assert_eq!(Religion::decode_prefix(&bad, limits()).unwrap().1, b.len());
    }
    #[test]
    fn exact_caps() {
        let b = source();
        let v = Religion::decode(&b, limits()).unwrap();
        let exact = Limits {
            max_bytes: b.len(),
            max_items: 7,
        };
        assert_eq!(v.encode_with_items(exact).unwrap(), (b.clone(), 7));
        assert_eq!(Religion::decode(&b, exact).unwrap(), v);
        for short in [
            Limits {
                max_bytes: b.len() - 1,
                ..exact
            },
            Limits {
                max_items: 6,
                ..exact
            },
        ] {
            assert_eq!(Religion::decode(&b, short), Err(Error::ResourceLimit));
            assert_eq!(v.encode(short), Err(Error::ResourceLimit));
        }
    }
    #[test]
    fn hostile_count() {
        let mut b = source();
        b[28..32].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(Religion::decode(&b, limits()), Err(Error::ResourceLimit));
    }
}
