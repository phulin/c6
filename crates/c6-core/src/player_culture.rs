//! Bounded Linux XP2 Player.Culture v38/v45 wire transport, including strict nested records.
//! No live Reset, definition remapping, callbacks, gameplay, actions or turns.
use crate::player_prerequisites::IndexedBools;
use crate::unit::{HashValue, Limits};

pub const VERSION: u32 = 45;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    UnsupportedLegacyWrite,
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
/// Independently tagged TourismIntoFavor38/45. Both signed words are
/// unconditional on the wire; no conversion or favor gameplay runs here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TourismIntoFavor {
    pub wire_version: u32,
    pub tourism: i32,
    pub favor: i32,
}
impl Wire for TourismIntoFavor {
    const MIN: usize = 12;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 38 | VERSION) {
            return Err(Error::Version {
                expected: VERSION,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            tourism: i32::read(r)?,
            favor: i32::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != VERSION {
            return Err(Error::UnsupportedLegacyWrite);
        }
        VERSION.write(w)?;
        self.tourism.write(w)?;
        self.favor.write(w)
    }
}
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
/// Player.Culture38/45 wire/default projection. The legacy GreatWork list is
/// retained as wire data; native Read discards and rebuilds it, as described by
/// `great_work_modifier_read_effect`, rather than using these values at38.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Culture {
    pub wire_version: u32,
    pub current_government_hash: u32,
    pub post_anarchy_government_hash: u32,
    pub anarchy_end_turn: i32,
    pub turn_adopted: i32,
    pub modified_boost: i32,
    pub government_change_considered: bool,
    pub government_unlocked: IndexedBools,
    pub government_usage_count: Vec<HashValue<i32>>,
    pub last_government_used: Vec<HashValue<u32>>,
    pub policy_unlocked: IndexedBools,
    pub policy_purchased: IndexedBools,
    pub policy_has_been_enacted: IndexedBools,
    pub policy_is_in_place: IndexedBools,
    pub policy_banned: IndexedBools,
    pub active_policies: GovernmentSlots,
    pub modified_slots: GovernmentSlots,
    pub modified_removed_slots: GovernmentSlots,
    pub has_civic: IndexedBools,
    pub boost_triggered: IndexedBools,
    pub cultural_progress_bits: Vec<HashValue<i32>>,
    pub civic_queue_hashes: Vec<u32>,
    pub cultural_overflow_bits: i32,
    pub culture_rate_change_bits: i32,
    pub civic_completed_this_turn: bool,
    pub civic_completed_this_turn_hash: u32,
    pub turn_last_policy_change: i32,
    pub num_random_civic_boosts_on_new_era: i32,
    pub free_civic_boost_wonder_era: i32,
    pub tourism_on_bits: Vec<i32>,
    pub dominant_over: Vec<bool>,
    pub turns_until_dominant: Vec<i32>,
    pub tourism_modifier: i32,
    pub cached_alliance_tourism_bonus_bits: i32,
    pub cached_alliance_culture_bonus_bits: i32,
    pub religious_tourism_reduction: i32,
    pub overall_tourism_reduction: i32,
    pub always_full_religious_tourism: i32,
    pub lifetime_culture_bits: i32,
    pub staycationers: i32,
    pub last_turn_staycationers: i32,
    pub last_turn_tourists_to: i32,
    pub tourists_from: Vec<i32>,
    pub extra_trade_route_tourism_modifier: i32,
    pub auto_themed_building_hash: u32,
    pub tourism_locations: Vec<TourismLocation>,
    pub culture_bomb_triggers: Vec<CultureBombTrigger>,
    pub tourism_into_favor: Vec<TourismIntoFavor>,
    pub culture_bomb_converts_city: bool,
    pub unlock_features: Vec<FeatureUnlock>,
    pub tourism_national_parks_modifier_bits: i32,
    pub great_work_object_tourism_modifiers: Vec<GreatWorkObjectTourismModifier>,
    pub policy_favor_list: Vec<PolicyFavor>,
    pub building_favor_list: Vec<BuildingFavor>,
    pub other_government_intolerance_multiplier_bits: i32,
    pub other_government_intolerance_modifier: i32,
    pub previous_government_policies: GovernmentSlots,
    pub num_civics_repeated: i32,
    pub auto_themed_buildings_with_x_slots: i32,
    pub auto_themed_wonders_with_x_slots: i32,
    pub all_great_works_tourism_modifier: i32,
    pub all_great_works_yield_modifier: i32,
    pub replaced_slots: Vec<GovernmentSlotReplacement>,
    pub government_type_modified_slots: Vec<GovernmentTypeSlotModifier>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GreatWorkModifierReadEffect {
    WireValues,
    /// One entry per actual GreatWorkObject definition in native iteration
    /// order, using its collection index, modifier0 and noTourism=false.
    DiscardWireAndRebuildInActualDefinitionOrder,
}
impl Culture {
    /// The absent38 vector consumes no wire entries. Native Reset establishes
    /// these64 values, preserved by the versioned self-copy. Not applied here.
    pub fn turns_until_dominant_read_reset(&self) -> Option<[i32; 64]> {
        (self.wire_version == 38).then_some([-1; 64])
    }
    /// At38, absent unlock_features and building_favor_list retain native
    /// Reset-cleared logical empty vectors. No constructor baseline is assumed.
    pub fn omitted_lists_read_reset_empty(&self) -> bool {
        self.wire_version == 38
    }
    pub fn great_work_modifier_read_effect(&self) -> GreatWorkModifierReadEffect {
        if self.wire_version == 38 {
            GreatWorkModifierReadEffect::DiscardWireAndRebuildInActualDefinitionOrder
        } else {
            GreatWorkModifierReadEffect::WireValues
        }
    }
}
impl Wire for Culture {
    const MIN: usize = 4
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <bool as Wire>::MIN
        + <IndexedBools as Wire>::MIN
        + <Vec<HashValue<i32>> as Wire>::MIN
        + <Vec<HashValue<u32>> as Wire>::MIN
        + <IndexedBools as Wire>::MIN
        + <IndexedBools as Wire>::MIN
        + <IndexedBools as Wire>::MIN
        + <IndexedBools as Wire>::MIN
        + <IndexedBools as Wire>::MIN
        + <GovernmentSlots as Wire>::MIN
        + <GovernmentSlots as Wire>::MIN
        + <GovernmentSlots as Wire>::MIN
        + <IndexedBools as Wire>::MIN
        + <IndexedBools as Wire>::MIN
        + <Vec<HashValue<i32>> as Wire>::MIN
        + <Vec<u32> as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <bool as Wire>::MIN
        + <u32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <Vec<i32> as Wire>::MIN
        + <Vec<bool> as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <Vec<i32> as Wire>::MIN
        + <i32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <Vec<TourismLocation> as Wire>::MIN
        + <Vec<CultureBombTrigger> as Wire>::MIN
        + <Vec<TourismIntoFavor> as Wire>::MIN
        + <bool as Wire>::MIN
        + <i32 as Wire>::MIN
        + <Vec<GreatWorkObjectTourismModifier> as Wire>::MIN
        + <Vec<PolicyFavor> as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <GovernmentSlots as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <Vec<GovernmentSlotReplacement> as Wire>::MIN
        + <Vec<GovernmentTypeSlotModifier> as Wire>::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 38 | 45) {
            return Err(Error::Version {
                expected: VERSION,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            current_government_hash: <u32 as Wire>::read(r)?,
            post_anarchy_government_hash: <u32 as Wire>::read(r)?,
            anarchy_end_turn: <i32 as Wire>::read(r)?,
            turn_adopted: <i32 as Wire>::read(r)?,
            modified_boost: <i32 as Wire>::read(r)?,
            government_change_considered: <bool as Wire>::read(r)?,
            government_unlocked: <IndexedBools as Wire>::read(r)?,
            government_usage_count: <Vec<HashValue<i32>> as Wire>::read(r)?,
            last_government_used: <Vec<HashValue<u32>> as Wire>::read(r)?,
            policy_unlocked: <IndexedBools as Wire>::read(r)?,
            policy_purchased: <IndexedBools as Wire>::read(r)?,
            policy_has_been_enacted: <IndexedBools as Wire>::read(r)?,
            policy_is_in_place: <IndexedBools as Wire>::read(r)?,
            policy_banned: <IndexedBools as Wire>::read(r)?,
            active_policies: <GovernmentSlots as Wire>::read(r)?,
            modified_slots: <GovernmentSlots as Wire>::read(r)?,
            modified_removed_slots: <GovernmentSlots as Wire>::read(r)?,
            has_civic: <IndexedBools as Wire>::read(r)?,
            boost_triggered: <IndexedBools as Wire>::read(r)?,
            cultural_progress_bits: <Vec<HashValue<i32>> as Wire>::read(r)?,
            civic_queue_hashes: <Vec<u32> as Wire>::read(r)?,
            cultural_overflow_bits: <i32 as Wire>::read(r)?,
            culture_rate_change_bits: <i32 as Wire>::read(r)?,
            civic_completed_this_turn: <bool as Wire>::read(r)?,
            civic_completed_this_turn_hash: <u32 as Wire>::read(r)?,
            turn_last_policy_change: <i32 as Wire>::read(r)?,
            num_random_civic_boosts_on_new_era: <i32 as Wire>::read(r)?,
            free_civic_boost_wonder_era: if wire_version == 45 {
                <i32 as Wire>::read(r)?
            } else {
                0
            },
            tourism_on_bits: <Vec<i32> as Wire>::read(r)?,
            dominant_over: <Vec<bool> as Wire>::read(r)?,
            turns_until_dominant: if wire_version == 45 {
                <Vec<i32> as Wire>::read(r)?
            } else {
                Vec::new()
            },
            tourism_modifier: <i32 as Wire>::read(r)?,
            cached_alliance_tourism_bonus_bits: <i32 as Wire>::read(r)?,
            cached_alliance_culture_bonus_bits: <i32 as Wire>::read(r)?,
            religious_tourism_reduction: <i32 as Wire>::read(r)?,
            overall_tourism_reduction: <i32 as Wire>::read(r)?,
            always_full_religious_tourism: <i32 as Wire>::read(r)?,
            lifetime_culture_bits: <i32 as Wire>::read(r)?,
            staycationers: <i32 as Wire>::read(r)?,
            last_turn_staycationers: if wire_version == 45 {
                <i32 as Wire>::read(r)?
            } else {
                0
            },
            last_turn_tourists_to: if wire_version == 45 {
                <i32 as Wire>::read(r)?
            } else {
                0
            },
            tourists_from: <Vec<i32> as Wire>::read(r)?,
            extra_trade_route_tourism_modifier: <i32 as Wire>::read(r)?,
            auto_themed_building_hash: <u32 as Wire>::read(r)?,
            tourism_locations: <Vec<TourismLocation> as Wire>::read(r)?,
            culture_bomb_triggers: <Vec<CultureBombTrigger> as Wire>::read(r)?,
            tourism_into_favor: <Vec<TourismIntoFavor> as Wire>::read(r)?,
            culture_bomb_converts_city: <bool as Wire>::read(r)?,
            unlock_features: if wire_version == 45 {
                <Vec<FeatureUnlock> as Wire>::read(r)?
            } else {
                Vec::new()
            },
            tourism_national_parks_modifier_bits: <i32 as Wire>::read(r)?,
            great_work_object_tourism_modifiers:
                <Vec<GreatWorkObjectTourismModifier> as Wire>::read(r)?,
            policy_favor_list: <Vec<PolicyFavor> as Wire>::read(r)?,
            building_favor_list: if wire_version == 45 {
                <Vec<BuildingFavor> as Wire>::read(r)?
            } else {
                Vec::new()
            },
            other_government_intolerance_multiplier_bits: <i32 as Wire>::read(r)?,
            other_government_intolerance_modifier: <i32 as Wire>::read(r)?,
            previous_government_policies: <GovernmentSlots as Wire>::read(r)?,
            num_civics_repeated: <i32 as Wire>::read(r)?,
            auto_themed_buildings_with_x_slots: <i32 as Wire>::read(r)?,
            auto_themed_wonders_with_x_slots: if wire_version == 45 {
                <i32 as Wire>::read(r)?
            } else {
                i32::MAX
            },
            all_great_works_tourism_modifier: <i32 as Wire>::read(r)?,
            all_great_works_yield_modifier: <i32 as Wire>::read(r)?,
            replaced_slots: <Vec<GovernmentSlotReplacement> as Wire>::read(r)?,
            government_type_modified_slots: <Vec<GovernmentTypeSlotModifier> as Wire>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != VERSION {
            return Err(Error::UnsupportedLegacyWrite);
        }
        VERSION.write(w)?;
        self.current_government_hash.write(w)?;
        self.post_anarchy_government_hash.write(w)?;
        self.anarchy_end_turn.write(w)?;
        self.turn_adopted.write(w)?;
        self.modified_boost.write(w)?;
        self.government_change_considered.write(w)?;
        self.government_unlocked.write(w)?;
        self.government_usage_count.write(w)?;
        self.last_government_used.write(w)?;
        self.policy_unlocked.write(w)?;
        self.policy_purchased.write(w)?;
        self.policy_has_been_enacted.write(w)?;
        self.policy_is_in_place.write(w)?;
        self.policy_banned.write(w)?;
        self.active_policies.write(w)?;
        self.modified_slots.write(w)?;
        self.modified_removed_slots.write(w)?;
        self.has_civic.write(w)?;
        self.boost_triggered.write(w)?;
        self.cultural_progress_bits.write(w)?;
        self.civic_queue_hashes.write(w)?;
        self.cultural_overflow_bits.write(w)?;
        self.culture_rate_change_bits.write(w)?;
        self.civic_completed_this_turn.write(w)?;
        self.civic_completed_this_turn_hash.write(w)?;
        self.turn_last_policy_change.write(w)?;
        self.num_random_civic_boosts_on_new_era.write(w)?;
        self.free_civic_boost_wonder_era.write(w)?;
        self.tourism_on_bits.write(w)?;
        self.dominant_over.write(w)?;
        self.turns_until_dominant.write(w)?;
        self.tourism_modifier.write(w)?;
        self.cached_alliance_tourism_bonus_bits.write(w)?;
        self.cached_alliance_culture_bonus_bits.write(w)?;
        self.religious_tourism_reduction.write(w)?;
        self.overall_tourism_reduction.write(w)?;
        self.always_full_religious_tourism.write(w)?;
        self.lifetime_culture_bits.write(w)?;
        self.staycationers.write(w)?;
        self.last_turn_staycationers.write(w)?;
        self.last_turn_tourists_to.write(w)?;
        self.tourists_from.write(w)?;
        self.extra_trade_route_tourism_modifier.write(w)?;
        self.auto_themed_building_hash.write(w)?;
        self.tourism_locations.write(w)?;
        self.culture_bomb_triggers.write(w)?;
        self.tourism_into_favor.write(w)?;
        self.culture_bomb_converts_city.write(w)?;
        self.unlock_features.write(w)?;
        self.tourism_national_parks_modifier_bits.write(w)?;
        self.great_work_object_tourism_modifiers.write(w)?;
        self.policy_favor_list.write(w)?;
        self.building_favor_list.write(w)?;
        self.other_government_intolerance_multiplier_bits.write(w)?;
        self.other_government_intolerance_modifier.write(w)?;
        self.previous_government_policies.write(w)?;
        self.num_civics_repeated.write(w)?;
        self.auto_themed_buildings_with_x_slots.write(w)?;
        self.auto_themed_wonders_with_x_slots.write(w)?;
        self.all_great_works_tourism_modifier.write(w)?;
        self.all_great_works_yield_modifier.write(w)?;
        self.replaced_slots.write(w)?;
        self.government_type_modified_slots.write(w)?;
        Ok(())
    }
}

impl Culture {
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
    fn specimen() -> Culture {
        Culture {
            wire_version: VERSION,
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
                wire_version: VERSION,
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
    fn tourism_into_favor38_keeps_signed_words_and_independent_version() {
        let mut b = Vec::new();
        for word in [38u32, i32::MIN as u32, i32::MAX as u32, 0xdeadbeef] {
            b.extend_from_slice(&word.to_le_bytes());
        }
        let decode = |data: &[u8], max_bytes| {
            let mut r = Reader {
                data,
                pos: 0,
                items: 0,
                limits: Limits {
                    max_bytes,
                    max_items: 0,
                },
            };
            TourismIntoFavor::read(&mut r).map(|v| (v, r.pos, r.items))
        };
        let (v, n, i) = decode(&b, 12).unwrap();
        assert_eq!(
            (v.wire_version, v.tourism, v.favor, n, i),
            (38, i32::MIN, i32::MAX, 12, 0)
        );
        for cut in 0..12 {
            assert!(decode(&b[..cut], 12).is_err());
        }
        assert!(decode(&b, 11).is_err());
        let mut w = Writer {
            data: Vec::new(),
            items: 0,
            limits: limits(),
        };
        assert_eq!(v.write(&mut w), Err(Error::UnsupportedLegacyWrite));
        assert!(w.data.is_empty());
        b[..4].copy_from_slice(&39u32.to_le_bytes());
        assert_eq!(
            decode(&b, 12),
            Err(Error::Version {
                expected: 45,
                actual: 39
            })
        );
        b[..4].copy_from_slice(&45u32.to_le_bytes());
        decode(&b, 12).unwrap().0.write(&mut w).unwrap();
        assert_eq!(w.data, &b[..12]);
    }
    #[test]
    fn culture38_literal_omissions_and_unapplied_definition_rebuild() {
        fn words(bytes: &mut Vec<u8>, values: &[i32]) {
            for value in values {
                bytes.extend(value.to_le_bytes());
            }
        }
        let mut b = Vec::new();
        words(&mut b, &[38, -1, -2, 101, 102, 103]);
        b.push(1);
        words(&mut b, &[0; 8]); // three government arrays and five policy arrays
        words(&mut b, &[0; 6]); // three present GovernmentSlots, each double-counted
        words(&mut b, &[0; 4]); // civic/boost/progress arrays and queue
        words(&mut b, &[-257, 513]);
        b.push(1);
        words(&mut b, &[-3, 104, 105]); // completed civic, last change, era boosts
                                        // FreeCivicBoostWonderEra45 is absent, followed by two present vectors.
        words(&mut b, &[0, 0]);
        // TurnsUntilDominant41 absent; eight present tourism/culture scalars.
        words(&mut b, &[106, -769, 1025, 107, 108, 109, 1281, 110]);
        // Both last-turn counts41 absent; tourists-from, trade bonus, building.
        words(&mut b, &[0, 111, -4]);
        words(&mut b, &[0, 0, 0]); // locations, bombs, tourism favor
        b.push(1); // converts city; unlockFeatures43 absent
        words(&mut b, &[1537, 1]); // national parks and legacy wire modifier list
        let great_work_tag = b.len();
        words(&mut b, &[45, 17, 222]);
        b.push(1);
        words(&mut b, &[0]); // policy favor; buildingFavor42 absent
        words(&mut b, &[1793, 112, 0, 0, 113, 114]);
        // AutoThemedWonders40 absent; last modifiers and replacements.
        words(&mut b, &[115, 116, 1, 45, 3, 4]);
        b.push(1);
        words(&mut b, &[0]); // government type modified slots38 present
        let end = b.len();
        b.extend([0xaa, 0xbb]);
        let exact = Limits {
            max_bytes: end,
            max_items: 2,
        };
        let (v, used, items) = Culture::decode_prefix_with_items(&b, exact).unwrap();
        assert_eq!((used, items, v.wire_version), (end, 2, 38));
        assert_eq!(v.num_random_civic_boosts_on_new_era, 105);
        assert_eq!(v.free_civic_boost_wonder_era, 0);
        assert_eq!((v.last_turn_staycationers, v.last_turn_tourists_to), (0, 0));
        assert_eq!(v.auto_themed_wonders_with_x_slots, i32::MAX);
        assert_eq!(v.turns_until_dominant_read_reset(), Some([-1; 64]));
        assert!(v.turns_until_dominant.is_empty()); // no wire vector at38
        assert!(v.unlock_features.is_empty() && v.building_favor_list.is_empty());
        assert!(v.omitted_lists_read_reset_empty());
        // Preserve wire data despite native Read discarding/rebuilding it.
        assert_eq!(
            v.great_work_object_tourism_modifiers[0].tourism_modifier,
            222
        );
        assert!(v.great_work_object_tourism_modifiers[0].no_tourism);
        assert_eq!(
            v.great_work_modifier_read_effect(),
            GreatWorkModifierReadEffect::DiscardWireAndRebuildInActualDefinitionOrder
        );
        assert_eq!(v.government_type_modified_slots.len(), 0);
        assert_eq!(v.replaced_slots[0].added_slot_ordinal, 4);
        assert_eq!(v.encode(exact), Err(Error::UnsupportedLegacyWrite));
        for n in 0..end {
            assert!(Culture::decode_prefix(&b[..n], exact).is_err(), "{n}");
        }
        assert_eq!(
            Culture::decode_prefix(
                &b,
                Limits {
                    max_items: 1,
                    ..exact
                }
            ),
            Err(Error::ResourceLimit)
        );
        let mut nested = b.clone();
        nested[great_work_tag..great_work_tag + 4].copy_from_slice(&38u32.to_le_bytes());
        assert_eq!(
            Culture::decode_prefix(&nested, exact),
            Err(Error::Version {
                expected: 45,
                actual: 38
            })
        );
        let mut unknown = b.clone();
        unknown[..4].copy_from_slice(&39u32.to_le_bytes());
        assert_eq!(
            Culture::decode_prefix(&unknown, exact),
            Err(Error::Version {
                expected: 45,
                actual: 39
            })
        );
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
            limits: limits(),
        };
        assert_eq!(Vec::<u32>::read(&mut r).unwrap().len(), 5);
    }
    #[test]
    fn independent_allocations_share_wire_and_item_budgets() {
        let values = vec![TourismLocation {
            plot_index: 17,
            lifetime_tourism: -3,
            tourist_counts: vec![],
        }];
        let mut w = Writer {
            data: vec![],
            items: 0,
            limits: limits(),
        };
        values.write(&mut w).unwrap();
        values.write(&mut w).unwrap();
        assert_eq!(w.data.len(), 32);
        let storage = std::mem::size_of::<TourismLocation>();
        assert!(storage <= w.data.len());
        assert!(storage * 2 > w.data.len());
        let exact = Limits {
            max_bytes: w.data.len(),
            max_items: 2,
        };
        let mut r = Reader {
            data: &w.data,
            pos: 0,
            items: 0,
            limits: exact,
        };
        assert_eq!(Vec::<TourismLocation>::read(&mut r).unwrap(), values);
        assert_eq!(Vec::<TourismLocation>::read(&mut r).unwrap(), values);
        assert_eq!((r.pos, r.items), (w.data.len(), 2));
        let mut short = Reader {
            data: &w.data,
            pos: 0,
            items: 0,
            limits: Limits {
                max_bytes: 31,
                ..exact
            },
        };
        // Each individual decoded allocation is 32 bytes; reject it before
        // reservation even though its wire representation occupies 16 bytes.
        assert_eq!(
            Vec::<TourismLocation>::read(&mut short),
            Err(Error::ResourceLimit)
        );
        assert_eq!(short.pos, 4);
        let mut short_items = Reader {
            data: &w.data,
            pos: 0,
            items: 0,
            limits: Limits {
                max_items: 1,
                ..exact
            },
        };
        Vec::<TourismLocation>::read(&mut short_items).unwrap();
        assert_eq!(
            Vec::<TourismLocation>::read(&mut short_items),
            Err(Error::ResourceLimit)
        );
    }
}
