//! Current Linux XP2 Player.Culture v45 wire transport, including nested records.
//! No live Reset, definition remapping, callbacks, gameplay, actions or turns.
use crate::player_prerequisites::IndexedBools;
use crate::unit::{HashValue, Limits};

pub const VERSION: u32 = 45;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    InvalidContainer,
    InvalidBool(u8),
    Version { expected: u32, actual: u32 },
    Prerequisites(crate::player_prerequisites::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player.Culture wire: {self:?}")
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
        self.allocated = self
            .allocated
            .checked_add(
                n.checked_mul(std::mem::size_of::<T>())
                    .ok_or(Error::ResourceLimit)?,
            )
            .ok_or(Error::ResourceLimit)?;
        if self.allocated > self.limits.max_bytes {
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
impl Wire for IndexedBools {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = u32::from_le_bytes(
            r.data
                .get(r.pos..r.pos.checked_add(4).ok_or(Error::ResourceLimit)?)
                .ok_or(Error::Truncated)?
                .try_into()
                .unwrap(),
        ) as usize;
        r.allocate::<HashValue<bool>>(n)?;
        let (v, used) =
            Self::decode_prefix(&r.data[r.pos..], r.remaining()).map_err(Error::Prerequisites)?;
        r.charge(v.pairs.len())?;
        r.raw(used)?;
        Ok(v)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.charge(self.pairs.len())?;
        let b = self
            .encode(Limits {
                max_bytes: w.limits.max_bytes - w.data.len(),
                max_items: self.pairs.len(),
            })
            .map_err(Error::Prerequisites)?;
        w.raw(&b)
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
/// Player.Culture explicitly writes a signed length and then WriteArray's u32
/// length. Current original writers emit equal nonnegative lengths.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernmentSlots {
    pub entries: Vec<GovernmentSlot>,
}
impl Wire for GovernmentSlots {
    const MIN: usize = 8;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let outer = u32::read(r)?;
        if outer > i32::MAX as u32 || outer as usize > r.remaining().max_items {
            return Err(Error::ResourceLimit);
        }
        let entries = Vec::<GovernmentSlot>::read(r)?;
        if outer as usize != entries.len() {
            return Err(Error::InvalidContainer);
        }
        Ok(Self { entries })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        i32::try_from(self.entries.len())
            .map_err(|_| Error::ResourceLimit)?
            .write(w)?;
        self.entries.write(w)
    }
}
// Wire order differs from native in-memory field order for GovernmentSlot.
record!(GovernmentSlot, false, {
    policy_hash: u32,
    slot_ordinal: i32,
});
record!(GovernmentSlotReplacement, true, {
    replaced_slot_ordinal: i32,
    added_slot_ordinal: i32,
    replaces_all: bool,
});
record!(GovernmentTypeSlotModifier, true, {
    government_hash: u32,
    slot_ordinal: i32,
    delta: i32,
});
record!(TouristCount, false, {
    player_ordinal: i32,
    count: i32,
});
record!(TourismLocation, false, {
    plot_index: i32,
    lifetime_tourism: i32,
    tourist_counts: Vec<TouristCount>,
});
record!(CultureBombTrigger, true, {
    building_hash: u32,
    district_hash: u32,
    improvement_hash: u32,
    captures_owned_territory: bool,
});
record!(TourismIntoFavor, true, {
    tourism: i32,
    favor: i32,
});
record!(FeatureUnlock, true, {
    feature_hash: u32,
    civic_hash: u32,
});
record!(GreatWorkObjectTourismModifier, true, {
    great_work_object_ordinal: i32,
    tourism_modifier: i32,
    no_tourism: bool,
});
record!(PolicyFavor, true, {
    policy_hash: u32,
    favor_per_turn: i32,
});
record!(BuildingFavor, true, {
    building_hash: u32,
    favor_per_turn: i32,
});
record!(Culture, true, {
    current_government_hash: u32,
    post_anarchy_government_hash: u32,
    anarchy_end_turn: i32,
    turn_adopted: i32,
    modified_boost: i32,
    government_change_considered: bool,
    government_unlocked: IndexedBools,
    government_usage_count: Vec<HashValue<i32>>,
    last_government_used: Vec<HashValue<u32>>,
    policy_unlocked: IndexedBools,
    policy_purchased: IndexedBools,
    policy_has_been_enacted: IndexedBools,
    policy_is_in_place: IndexedBools,
    policy_banned: IndexedBools,
    active_policies: GovernmentSlots,
    modified_slots: GovernmentSlots,
    modified_removed_slots: GovernmentSlots,
    has_civic: IndexedBools,
    boost_triggered: IndexedBools,
    cultural_progress_bits: Vec<HashValue<i32>>,
    civic_queue_hashes: Vec<u32>,
    cultural_overflow_bits: i32,
    culture_rate_change_bits: i32,
    civic_completed_this_turn: bool,
    civic_completed_this_turn_hash: u32,
    turn_last_policy_change: i32,
    num_random_civic_boosts_on_new_era: i32,
    free_civic_boost_wonder_era: i32,
    tourism_on_bits: Vec<i32>,
    dominant_over: Vec<bool>,
    turns_until_dominant: Vec<i32>,
    tourism_modifier: i32,
    cached_alliance_tourism_bonus_bits: i32,
    cached_alliance_culture_bonus_bits: i32,
    religious_tourism_reduction: i32,
    overall_tourism_reduction: i32,
    always_full_religious_tourism: i32,
    lifetime_culture_bits: i32,
    staycationers: i32,
    last_turn_staycationers: i32,
    last_turn_tourists_to: i32,
    tourists_from: Vec<i32>,
    extra_trade_route_tourism_modifier: i32,
    auto_themed_building_hash: u32,
    tourism_locations: Vec<TourismLocation>,
    culture_bomb_triggers: Vec<CultureBombTrigger>,
    tourism_into_favor: Vec<TourismIntoFavor>,
    culture_bomb_converts_city: bool,
    unlock_features: Vec<FeatureUnlock>,
    tourism_national_parks_modifier_bits: i32,
    great_work_object_tourism_modifiers: Vec<GreatWorkObjectTourismModifier>,
    policy_favor_list: Vec<PolicyFavor>,
    building_favor_list: Vec<BuildingFavor>,
    other_government_intolerance_multiplier_bits: i32,
    other_government_intolerance_modifier: i32,
    previous_government_policies: GovernmentSlots,
    num_civics_repeated: i32,
    auto_themed_buildings_with_x_slots: i32,
    auto_themed_wonders_with_x_slots: i32,
    all_great_works_tourism_modifier: i32,
    all_great_works_yield_modifier: i32,
    replaced_slots: Vec<GovernmentSlotReplacement>,
    government_type_modified_slots: Vec<GovernmentTypeSlotModifier>
});
impl Culture {
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
    fn specimen() -> Culture {
        Culture {
            current_government_hash: 3735879680,
            post_anarchy_government_hash: 3735879681,
            anarchy_end_turn: -123456787,
            turn_adopted: -123456786,
            modified_boost: -123456785,
            government_change_considered: true,
            government_unlocked: IndexedBools {
                pairs: vec![
                    HashValue {
                        definition_hash: 0xf1234567,
                        value: true,
                    },
                    HashValue {
                        definition_hash: 0xf1234567,
                        value: false,
                    },
                ],
            },
            government_usage_count: vec![
                HashValue {
                    definition_hash: 0xabcdef01,
                    value: i32::MIN,
                },
                HashValue {
                    definition_hash: 0xabcdef01,
                    value: i32::MAX,
                },
            ],
            last_government_used: vec![HashValue {
                definition_hash: 0xabcdef01,
                value: 0xfedcba98,
            }],
            policy_unlocked: IndexedBools {
                pairs: vec![
                    HashValue {
                        definition_hash: 0xf1234567,
                        value: true,
                    },
                    HashValue {
                        definition_hash: 0xf1234567,
                        value: false,
                    },
                ],
            },
            policy_purchased: IndexedBools {
                pairs: vec![
                    HashValue {
                        definition_hash: 0xf1234567,
                        value: true,
                    },
                    HashValue {
                        definition_hash: 0xf1234567,
                        value: false,
                    },
                ],
            },
            policy_has_been_enacted: IndexedBools {
                pairs: vec![
                    HashValue {
                        definition_hash: 0xf1234567,
                        value: true,
                    },
                    HashValue {
                        definition_hash: 0xf1234567,
                        value: false,
                    },
                ],
            },
            policy_is_in_place: IndexedBools {
                pairs: vec![
                    HashValue {
                        definition_hash: 0xf1234567,
                        value: true,
                    },
                    HashValue {
                        definition_hash: 0xf1234567,
                        value: false,
                    },
                ],
            },
            policy_banned: IndexedBools {
                pairs: vec![
                    HashValue {
                        definition_hash: 0xf1234567,
                        value: true,
                    },
                    HashValue {
                        definition_hash: 0xf1234567,
                        value: false,
                    },
                ],
            },
            active_policies: GovernmentSlots {
                entries: vec![GovernmentSlot {
                    policy_hash: 0x12345678,
                    slot_ordinal: -7,
                }],
            },
            modified_slots: GovernmentSlots {
                entries: vec![GovernmentSlot {
                    policy_hash: 0x12345678,
                    slot_ordinal: -7,
                }],
            },
            modified_removed_slots: GovernmentSlots {
                entries: vec![GovernmentSlot {
                    policy_hash: 0x12345678,
                    slot_ordinal: -7,
                }],
            },
            has_civic: IndexedBools {
                pairs: vec![
                    HashValue {
                        definition_hash: 0xf1234567,
                        value: true,
                    },
                    HashValue {
                        definition_hash: 0xf1234567,
                        value: false,
                    },
                ],
            },
            boost_triggered: IndexedBools {
                pairs: vec![
                    HashValue {
                        definition_hash: 0xf1234567,
                        value: true,
                    },
                    HashValue {
                        definition_hash: 0xf1234567,
                        value: false,
                    },
                ],
            },
            cultural_progress_bits: vec![
                HashValue {
                    definition_hash: 0xabcdef01,
                    value: i32::MIN,
                },
                HashValue {
                    definition_hash: 0xabcdef01,
                    value: i32::MAX,
                },
            ],
            civic_queue_hashes: vec![0xffffffff, 0xf1234567, 0xf1234567, 0],
            cultural_overflow_bits: -123456768,
            culture_rate_change_bits: -123456767,
            civic_completed_this_turn: true,
            civic_completed_this_turn_hash: 3735879704,
            turn_last_policy_change: -123456764,
            num_random_civic_boosts_on_new_era: -123456763,
            free_civic_boost_wonder_era: -123456762,
            tourism_on_bits: vec![i32::MIN, -1, 0, i32::MAX],
            dominant_over: vec![false, true, true, false],
            turns_until_dominant: vec![i32::MIN, -1, 0, i32::MAX],
            tourism_modifier: -123456758,
            cached_alliance_tourism_bonus_bits: -123456757,
            cached_alliance_culture_bonus_bits: -123456756,
            religious_tourism_reduction: -123456755,
            overall_tourism_reduction: -123456754,
            always_full_religious_tourism: -123456753,
            lifetime_culture_bits: -123456752,
            staycationers: -123456751,
            last_turn_staycationers: -123456750,
            last_turn_tourists_to: -123456749,
            tourists_from: vec![i32::MIN, -1, 0, i32::MAX],
            extra_trade_route_tourism_modifier: -123456747,
            auto_themed_building_hash: 3735879723,
            tourism_locations: vec![TourismLocation {
                plot_index: -9,
                lifetime_tourism: i32::MAX,
                tourist_counts: vec![
                    TouristCount {
                        player_ordinal: 62,
                        count: 42,
                    },
                    TouristCount {
                        player_ordinal: 62,
                        count: -19,
                    },
                ],
            }],
            culture_bomb_triggers: vec![CultureBombTrigger {
                building_hash: 0x80000000,
                district_hash: 0xffffffff,
                improvement_hash: 4,
                captures_owned_territory: false,
            }],
            tourism_into_favor: vec![TourismIntoFavor {
                tourism: -123,
                favor: 456,
            }],
            culture_bomb_converts_city: true,
            unlock_features: vec![FeatureUnlock {
                feature_hash: 0xff001122,
                civic_hash: 0x11223344,
            }],
            tourism_national_parks_modifier_bits: -123456740,
            great_work_object_tourism_modifiers: vec![GreatWorkObjectTourismModifier {
                great_work_object_ordinal: -99,
                tourism_modifier: 27,
                no_tourism: true,
            }],
            policy_favor_list: vec![PolicyFavor {
                policy_hash: 0x87654321,
                favor_per_turn: -29,
            }],
            building_favor_list: vec![BuildingFavor {
                building_hash: 0x11111111,
                favor_per_turn: 311,
            }],
            other_government_intolerance_multiplier_bits: -123456736,
            other_government_intolerance_modifier: -123456735,
            previous_government_policies: GovernmentSlots {
                entries: vec![GovernmentSlot {
                    policy_hash: 0x12345678,
                    slot_ordinal: -7,
                }],
            },
            num_civics_repeated: -123456733,
            auto_themed_buildings_with_x_slots: -123456732,
            auto_themed_wonders_with_x_slots: -123456731,
            all_great_works_tourism_modifier: -123456730,
            all_great_works_yield_modifier: -123456729,
            replaced_slots: vec![GovernmentSlotReplacement {
                replaced_slot_ordinal: -2,
                added_slot_ordinal: 9,
                replaces_all: true,
            }],
            government_type_modified_slots: vec![GovernmentTypeSlotModifier {
                government_hash: 0xf1234567,
                slot_ordinal: -8,
                delta: i32::MIN,
            }],
        }
    }
    #[test]
    fn nonempty_roundtrip_and_prefix() {
        let v = specimen();
        let (b, items) = v.encode_with_items(limits()).unwrap();
        let (decoded, n, charged) = Culture::decode_prefix_with_items(&b, limits()).unwrap();
        assert_eq!(decoded, v);
        assert_eq!((n, charged), (b.len(), items));
        assert!(items > 50);
        assert_eq!(Culture::decode(&b, limits()).unwrap(), v);
        let mut tail = b.clone();
        tail.push(19);
        assert_eq!(Culture::decode(&tail, limits()), Err(Error::Trailing));
        assert_eq!(Culture::decode_prefix(&tail, limits()).unwrap().1, b.len());
    }
    #[test]
    fn every_truncation_rejected() {
        let b = specimen().encode(limits()).unwrap();
        for end in 0..b.len() {
            assert!(
                Culture::decode(&b[..end], limits()).is_err(),
                "accepted truncation {end}"
            );
        }
    }
    #[test]
    fn strict_versions_and_bool() {
        let mut b = specimen().encode(limits()).unwrap();
        b[0] = 44;
        assert_eq!(
            Culture::decode(&b, limits()),
            Err(Error::Version {
                expected: 45,
                actual: 44
            })
        );
        b[0] = 45;
        b[24] = 2;
        assert_eq!(Culture::decode(&b, limits()), Err(Error::InvalidBool(2)));
        let mut w = Writer {
            data: vec![],
            items: 0,
            limits: limits(),
        };
        specimen().replaced_slots[0].write(&mut w).unwrap();
        w.data[0] = 44;
        let mut r = Reader {
            data: &w.data,
            pos: 0,
            items: 0,
            allocated: 0,
            limits: limits(),
        };
        assert!(matches!(
            GovernmentSlotReplacement::read(&mut r),
            Err(Error::Version { actual: 44, .. })
        ));
    }
    #[test]
    fn exact_aggregate_budgets() {
        let v = specimen();
        let (b, items) = v.encode_with_items(limits()).unwrap();
        let exact = Limits {
            max_bytes: b.len(),
            max_items: items,
        };
        assert_eq!(v.encode(exact).unwrap(), b);
        assert_eq!(Culture::decode(&b, exact).unwrap(), v);
        for low in [
            Limits {
                max_bytes: b.len() - 1,
                ..exact
            },
            Limits {
                max_items: items - 1,
                ..exact
            },
        ] {
            assert!(v.encode(low).is_err());
            assert!(Culture::decode(&b, low).is_err());
        }
    }
    #[test]
    fn allocation_preflight() {
        let mut r = Reader {
            data: &[0xff, 0xff, 0xff, 0x7f],
            pos: 0,
            items: 0,
            allocated: 0,
            limits: limits(),
        };
        assert_eq!(
            Vec::<TourismLocation>::read(&mut r),
            Err(Error::ResourceLimit)
        );
        let mut r = Reader {
            data: &[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            pos: 0,
            items: 0,
            allocated: 0,
            limits: Limits {
                max_bytes: 12,
                max_items: 2,
            },
        };
        assert_eq!(
            Vec::<TourismLocation>::read(&mut r),
            Err(Error::ResourceLimit)
        );
    }
    #[test]
    fn nested_versions_bools_and_double_count_contract() {
        let mut w = Writer {
            data: vec![],
            items: 0,
            limits: limits(),
        };
        specimen().active_policies.write(&mut w).unwrap();
        assert_eq!(&w.data[..8], &[1, 0, 0, 0, 1, 0, 0, 0]);
        w.data[0] = 0;
        let mut r = Reader {
            data: &w.data,
            pos: 0,
            items: 0,
            allocated: 0,
            limits: limits(),
        };
        assert_eq!(GovernmentSlots::read(&mut r), Err(Error::InvalidContainer));
        let mut w = Writer {
            data: vec![],
            items: 0,
            limits: limits(),
        };
        specimen().culture_bomb_triggers[0].write(&mut w).unwrap();
        w.data[16] = 2;
        let mut r = Reader {
            data: &w.data,
            pos: 0,
            items: 0,
            allocated: 0,
            limits: limits(),
        };
        assert_eq!(CultureBombTrigger::read(&mut r), Err(Error::InvalidBool(2)));
        let mut w = Writer {
            data: vec![],
            items: 0,
            limits: limits(),
        };
        specimen().has_civic.write(&mut w).unwrap();
        w.data[8] = 2;
        let mut r = Reader {
            data: &w.data,
            pos: 0,
            items: 0,
            allocated: 0,
            limits: limits(),
        };
        assert_eq!(
            IndexedBools::read(&mut r),
            Err(Error::Prerequisites(
                crate::player_prerequisites::Error::InvalidBool
            ))
        );
        // The current original queue serializer is count + ordered hash values,
        // rather than its native small-buffer capacity or in-memory pointers.
        let mut w = Writer {
            data: vec![],
            items: 0,
            limits: limits(),
        };
        vec![0xf1234567u32; 5].write(&mut w).unwrap();
        let mut r = Reader {
            data: &w.data,
            pos: 0,
            items: 0,
            allocated: 0,
            limits: limits(),
        };
        assert_eq!(Vec::<u32>::read(&mut r).unwrap().len(), 5);
    }
}
