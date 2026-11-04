//! Linux XP2 Barbarian Manager3/5 and Instance8/15 wire/default transport.
//! No live Reset, definition remapping, callbacks, gameplay, actions or turns.
use crate::unit::Limits;

pub const VERSION: u32 = 5;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    UnsupportedLegacyWrite,
    MissingCurrentField,
    Variant(crate::unit::variants::VariantError),
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Barbarian wire: {self:?}")
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
        Ok(())
    }
    fn version(&mut self, expected: u32) -> Result<(), Error> {
        let actual = u32::read(self)?;
        if actual != expected {
            return Err(Error::Version { expected, actual });
        }
        Ok(())
    }
}
impl Writer {
    fn remaining(&self) -> Limits {
        Limits {
            max_bytes: self.limits.max_bytes - self.data.len(),
            max_items: self.limits.max_items - self.items,
        }
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

impl Wire for u8 {
    const MIN: usize = 1;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(r.raw(1)?[0])
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.raw(&[*self])
    }
}
impl Wire for i16 {
    const MIN: usize = 2;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self::from_le_bytes(r.raw(2)?.try_into().unwrap()))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.raw(&self.to_le_bytes())
    }
}
macro_rules! record {
    ($name:ident, $version:expr, { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name { $(pub $field: $ty),* }
        impl Wire for $name {
            const MIN: usize = (if $version > 0 { 4 } else { 0 }) $(+ <$ty as Wire>::MIN)*;
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                if $version > 0 { r.version($version)?; }
                Ok(Self { $($field: <$ty as Wire>::read(r)?),* })
            }
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                if $version > 0 { ($version as u32).write(w)?; }
                $(self.$field.write(w)?;)*
                Ok(())
            }
        }
    };
}

use crate::unit::variants::{DecodeBudget, Limits as VariantLimits, TypedVariantMap};
impl Wire for TypedVariantMap {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let left = r.remaining();
        let mut budget = DecodeBudget::new(VariantLimits {
            max_bytes: left.max_bytes,
            max_items: left.max_items,
            max_depth: 64,
        });
        let (v, n) = Self::decode_prefix(&r.data[r.pos..], &mut budget).map_err(|e| {
            if e == crate::unit::variants::VariantError::Limit {
                Error::ResourceLimit
            } else {
                Error::Variant(e)
            }
        })?;
        r.charge(budget.items_used())?;
        r.raw(n)?;
        Ok(v)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        let left = w.remaining();
        let (b, n) = self
            .encode(VariantLimits {
                max_bytes: left.max_bytes,
                max_items: left.max_items,
                max_depth: 64,
            })
            .map_err(|e| {
                if e == crate::unit::variants::VariantError::Limit {
                    Error::ResourceLimit
                } else {
                    Error::Variant(e)
                }
            })?;
        w.charge(n)?;
        w.raw(&b)
    }
}
// Strings are counted bytes without a serialized terminator. No UTF-8 conversion.
record!(AsciiString,0,{bytes:Vec<u8>});
record!(ComponentId,0,{owner:i16,component_type:i16,id:u32});
record!(PlotCoord,0,{x:i32,y:i32});
record!(WeightedPlayer,0,{player_ordinal:i32,weight:i32});
record!(CampLocation,1,{plot_index:i32});
record!(UniqueUnitClaimData,3,{unit_hash:u32,value_4:i32,value_8:i32});
record!(ClansManager,3,{unique_unit_claims:Vec<UniqueUnitClaimData>,value_100:i32});
record!(ClanPlayerStats,15,{
    raid_count:i32,hire_count:i32,bribe_count:i32,incite_count:i32,ransom_count:i32,
    unit_hashes_hired:Vec<u32>,last_turn_raided:i32,last_turn_hired:i32,
    last_turn_bribed:i32,last_turn_incited:i32,last_turn_ransomed:i32,
});
/// Instance8/15 wire/default projection. The five absent8 carriers preserve
/// unknown native self-default state; no clan/NPC gameplay state is excluded.
/// Native player/definition binding and archive effects remain unapplied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Instance {
    pub wire_version: u32,
    pub player_ordinal: i32,
    pub tribe_name_hash: u32,
    pub tribe_index: i32,
    pub camp_location: PlotCoord,
    pub living_raw: u8,
    pub rampage_target_ordinal: i32,
    pub boldness: i32,
    pub max_num_scouts: i32,
    pub cur_num_scouts: i32,
    pub turns_since_scout_death: i32,
    pub max_standard_military_units: i32,
    pub num_standard_military_units: i32,
    pub turns_since_last_warrior_spawn: i32,
    pub percent_ranged_units: i32,
    pub turns_between_warrior_spawn: i32,
    pub required_melee_spawn: i32,
    pub required_ranged_spawn: i32,
    pub required_siege_spawn: i32,
    pub required_support_spawn: i32,
    pub needed_unit_tags: Vec<AsciiString>,
    pub scouting_behavior_tree: u32,
    pub raiding_behavior_tree: u32,
    pub city_attack_behavior_tree: u32,
    pub scouted_cities: Vec<ComponentId>,
    pub raided_cities: Vec<ComponentId>,
    pub raiding_score: i32,
    pub turns_in_enemy_territory: i32,
    pub properties: Option<TypedVariantMap>,
    pub clan_tech_hashes: Option<Vec<u32>>,
    pub tech_steal_turns_cooldown: i32,
    pub conversion_points: i32,
    pub player_bribe_turns_remaining: Option<Vec<i32>>,
    pub clan_player_stats: Option<Vec<ClanPlayerStats>>,
    pub incite_targets: Option<Vec<WeightedPlayer>>,
}
impl Wire for Instance {
    const MIN: usize = 4
        + <i32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <PlotCoord as Wire>::MIN
        + <u8 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <Vec<AsciiString> as Wire>::MIN
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <Vec<ComponentId> as Wire>::MIN
        + <Vec<ComponentId> as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 8 | 15) {
            return Err(Error::Version {
                expected: 15,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            player_ordinal: <i32 as Wire>::read(r)?,
            tribe_name_hash: <u32 as Wire>::read(r)?,
            tribe_index: <i32 as Wire>::read(r)?,
            camp_location: <PlotCoord as Wire>::read(r)?,
            living_raw: <u8 as Wire>::read(r)?,
            rampage_target_ordinal: <i32 as Wire>::read(r)?,
            boldness: <i32 as Wire>::read(r)?,
            max_num_scouts: <i32 as Wire>::read(r)?,
            cur_num_scouts: <i32 as Wire>::read(r)?,
            turns_since_scout_death: <i32 as Wire>::read(r)?,
            max_standard_military_units: <i32 as Wire>::read(r)?,
            num_standard_military_units: <i32 as Wire>::read(r)?,
            turns_since_last_warrior_spawn: <i32 as Wire>::read(r)?,
            percent_ranged_units: <i32 as Wire>::read(r)?,
            turns_between_warrior_spawn: <i32 as Wire>::read(r)?,
            required_melee_spawn: <i32 as Wire>::read(r)?,
            required_ranged_spawn: <i32 as Wire>::read(r)?,
            required_siege_spawn: <i32 as Wire>::read(r)?,
            required_support_spawn: <i32 as Wire>::read(r)?,
            needed_unit_tags: <Vec<AsciiString> as Wire>::read(r)?,
            scouting_behavior_tree: <u32 as Wire>::read(r)?,
            raiding_behavior_tree: <u32 as Wire>::read(r)?,
            city_attack_behavior_tree: <u32 as Wire>::read(r)?,
            scouted_cities: <Vec<ComponentId> as Wire>::read(r)?,
            raided_cities: <Vec<ComponentId> as Wire>::read(r)?,
            raiding_score: <i32 as Wire>::read(r)?,
            turns_in_enemy_territory: <i32 as Wire>::read(r)?,
            properties: if wire_version == 15 {
                Some(<TypedVariantMap as Wire>::read(r)?)
            } else {
                None
            },
            clan_tech_hashes: if wire_version == 15 {
                Some(<Vec<u32> as Wire>::read(r)?)
            } else {
                None
            },
            tech_steal_turns_cooldown: if wire_version == 15 {
                i32::read(r)?
            } else {
                -1
            },
            conversion_points: if wire_version == 15 { i32::read(r)? } else { 0 },
            player_bribe_turns_remaining: if wire_version == 15 {
                Some(<Vec<i32> as Wire>::read(r)?)
            } else {
                None
            },
            clan_player_stats: if wire_version == 15 {
                Some(<Vec<ClanPlayerStats> as Wire>::read(r)?)
            } else {
                None
            },
            incite_targets: if wire_version == 15 {
                Some(<Vec<WeightedPlayer> as Wire>::read(r)?)
            } else {
                None
            },
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 15 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        let properties = self.properties.as_ref().ok_or(Error::MissingCurrentField)?;
        let clan_tech_hashes = self
            .clan_tech_hashes
            .as_ref()
            .ok_or(Error::MissingCurrentField)?;
        let player_bribe_turns_remaining = self
            .player_bribe_turns_remaining
            .as_ref()
            .ok_or(Error::MissingCurrentField)?;
        let clan_player_stats = self
            .clan_player_stats
            .as_ref()
            .ok_or(Error::MissingCurrentField)?;
        let incite_targets = self
            .incite_targets
            .as_ref()
            .ok_or(Error::MissingCurrentField)?;
        15u32.write(w)?;
        self.player_ordinal.write(w)?;
        self.tribe_name_hash.write(w)?;
        self.tribe_index.write(w)?;
        self.camp_location.write(w)?;
        self.living_raw.write(w)?;
        self.rampage_target_ordinal.write(w)?;
        self.boldness.write(w)?;
        self.max_num_scouts.write(w)?;
        self.cur_num_scouts.write(w)?;
        self.turns_since_scout_death.write(w)?;
        self.max_standard_military_units.write(w)?;
        self.num_standard_military_units.write(w)?;
        self.turns_since_last_warrior_spawn.write(w)?;
        self.percent_ranged_units.write(w)?;
        self.turns_between_warrior_spawn.write(w)?;
        self.required_melee_spawn.write(w)?;
        self.required_ranged_spawn.write(w)?;
        self.required_siege_spawn.write(w)?;
        self.required_support_spawn.write(w)?;
        self.needed_unit_tags.write(w)?;
        self.scouting_behavior_tree.write(w)?;
        self.raiding_behavior_tree.write(w)?;
        self.city_attack_behavior_tree.write(w)?;
        self.scouted_cities.write(w)?;
        self.raided_cities.write(w)?;
        self.raiding_score.write(w)?;
        self.turns_in_enemy_territory.write(w)?;
        properties.write(w)?;
        clan_tech_hashes.write(w)?;
        self.tech_steal_turns_cooldown.write(w)?;
        self.conversion_points.write(w)?;
        player_bribe_turns_remaining.write(w)?;
        clan_player_stats.write(w)?;
        incite_targets.write(w)?;
        Ok(())
    }
}

/// Barbarian Manager3/5 wire projection. Missing camp locations(since4)
/// and clan manager(since5) self-default, leaving their prior state unknown.
/// Native player binding, signals/evaluators and definition remaps do not run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Manager {
    pub wire_version: u32,
    pub player_ordinal: i32,
    pub value_c8: u32,
    pub value_e0: u32,
    pub value_f8: u32,
    pub control_priority_ordinal: i32,
    pub improvement_hash: u32,
    pub flag_168_raw: u8,
    pub values_1a8: Vec<i32>,
    pub instances: Vec<Instance>,
    pub camp_locations: Option<Vec<CampLocation>>,
    pub clans: Option<ClansManager>,
}
impl Wire for Manager {
    const MIN: usize = 37;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 3 | VERSION) {
            return Err(Error::Version {
                expected: VERSION,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            player_ordinal: <i32 as Wire>::read(r)?,
            value_c8: <u32 as Wire>::read(r)?,
            value_e0: <u32 as Wire>::read(r)?,
            value_f8: <u32 as Wire>::read(r)?,
            control_priority_ordinal: <i32 as Wire>::read(r)?,
            improvement_hash: <u32 as Wire>::read(r)?,
            flag_168_raw: <u8 as Wire>::read(r)?,
            values_1a8: <Vec<i32> as Wire>::read(r)?,
            instances: <Vec<Instance> as Wire>::read(r)?,
            camp_locations: if wire_version == VERSION {
                Some(Vec::<CampLocation>::read(r)?)
            } else {
                None
            },
            clans: if wire_version == VERSION {
                Some(ClansManager::read(r)?)
            } else {
                None
            },
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != VERSION {
            return Err(Error::UnsupportedLegacyWrite);
        }
        let camp_locations = self
            .camp_locations
            .as_ref()
            .ok_or(Error::MissingCurrentField)?;
        let clans = self.clans.as_ref().ok_or(Error::MissingCurrentField)?;
        VERSION.write(w)?;
        self.player_ordinal.write(w)?;
        self.value_c8.write(w)?;
        self.value_e0.write(w)?;
        self.value_f8.write(w)?;
        self.control_priority_ordinal.write(w)?;
        self.improvement_hash.write(w)?;
        self.flag_168_raw.write(w)?;
        self.values_1a8.write(w)?;
        self.instances.write(w)?;
        camp_locations.write(w)?;
        clans.write(w)
    }
}
impl Manager {
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
    fn generous() -> Limits {
        Limits {
            max_bytes: 1 << 20,
            max_items: 10000,
        }
    }
    fn properties() -> TypedVariantMap {
        use crate::unit::variants::*;
        TypedVariantMap {
            header: KeyedHeader::Prefix(0x80000001),
            entries: vec![TypedEntry {
                key: u32::MAX,
                name: Some(vec![0xff, 0]),
                value: TypedVariant {
                    type_bits: 5,
                    modifier_bits: 0,
                    value: ArrayVariant {
                        count_flags: 0x01000003,
                        element_size: 1,
                        payload: VariantPayload::Char8(vec![0xff, 0x80, 0]),
                    },
                },
            }],
        }
    }
    fn specimen(flag: u8) -> Manager {
        Manager {
            wire_version: VERSION,
            player_ordinal: i32::MIN,
            value_c8: u32::MAX,
            value_e0: u32::MAX,
            value_f8: u32::MAX,
            control_priority_ordinal: i32::MIN,
            improvement_hash: u32::MAX,
            flag_168_raw: flag,
            values_1a8: vec![i32::MIN],
            instances: vec![Instance {
                wire_version: 15,
                player_ordinal: i32::MIN,
                tribe_name_hash: u32::MAX,
                tribe_index: i32::MIN,
                camp_location: PlotCoord {
                    x: i32::MIN,
                    y: i32::MIN,
                },
                living_raw: flag,
                rampage_target_ordinal: i32::MIN,
                boldness: i32::MIN,
                max_num_scouts: i32::MIN,
                cur_num_scouts: i32::MIN,
                turns_since_scout_death: i32::MIN,
                max_standard_military_units: i32::MIN,
                num_standard_military_units: i32::MIN,
                turns_since_last_warrior_spawn: i32::MIN,
                percent_ranged_units: i32::MIN,
                turns_between_warrior_spawn: i32::MIN,
                required_melee_spawn: i32::MIN,
                required_ranged_spawn: i32::MIN,
                required_siege_spawn: i32::MIN,
                required_support_spawn: i32::MIN,
                needed_unit_tags: vec![AsciiString { bytes: vec![flag] }],
                scouting_behavior_tree: u32::MAX,
                raiding_behavior_tree: u32::MAX,
                city_attack_behavior_tree: u32::MAX,
                scouted_cities: vec![ComponentId {
                    owner: i16::MIN,
                    component_type: i16::MIN,
                    id: u32::MAX,
                }],
                raided_cities: vec![ComponentId {
                    owner: i16::MIN,
                    component_type: i16::MIN,
                    id: u32::MAX,
                }],
                raiding_score: i32::MIN,
                turns_in_enemy_territory: i32::MIN,
                properties: Some(properties()),
                clan_tech_hashes: Some(vec![u32::MAX]),
                tech_steal_turns_cooldown: i32::MIN,
                conversion_points: i32::MIN,
                player_bribe_turns_remaining: Some(vec![i32::MIN]),
                clan_player_stats: Some(vec![ClanPlayerStats {
                    raid_count: i32::MIN,
                    hire_count: i32::MIN,
                    bribe_count: i32::MIN,
                    incite_count: i32::MIN,
                    ransom_count: i32::MIN,
                    unit_hashes_hired: vec![u32::MAX],
                    last_turn_raided: i32::MIN,
                    last_turn_hired: i32::MIN,
                    last_turn_bribed: i32::MIN,
                    last_turn_incited: i32::MIN,
                    last_turn_ransomed: i32::MIN,
                }]),
                incite_targets: Some(vec![WeightedPlayer {
                    player_ordinal: i32::MIN,
                    weight: i32::MIN,
                }]),
            }],
            camp_locations: Some(vec![CampLocation {
                plot_index: i32::MIN,
            }]),
            clans: Some(ClansManager {
                unique_unit_claims: vec![UniqueUnitClaimData {
                    unit_hash: u32::MAX,
                    value_4: i32::MIN,
                    value_8: i32::MIN,
                }],
                value_100: i32::MIN,
            }),
        }
    }
    #[test]
    fn instance8_preserves_omitted_prior_and_literal_defaults_without_wire_counts() {
        let mut b = Vec::new();
        for word in [8u32, 63, 0x81234567, 7, 100, (-100i32) as u32] {
            b.extend_from_slice(&word.to_le_bytes());
        }
        b.push(0xfe);
        for word in [u32::MAX, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13] {
            b.extend_from_slice(&word.to_le_bytes());
        }
        for word in [
            0u32,
            0x80000001,
            0,
            u32::MAX,
            0,
            0,
            i32::MIN as u32,
            i32::MAX as u32,
        ] {
            b.extend_from_slice(&word.to_le_bytes());
        }
        b.push(0xaa);
        let decode = |data: &[u8], max_bytes| {
            let mut r = Reader {
                data,
                pos: 0,
                items: 0,
                allocated: 0,
                limits: Limits {
                    max_bytes,
                    max_items: 0,
                },
            };
            Instance::read(&mut r).map(|v| (v, r.pos, r.items))
        };
        let (v, n, i) = decode(&b, 113).unwrap();
        assert_eq!(
            (v.wire_version, v.player_ordinal, v.living_raw, n, i),
            (8, 63, 0xfe, 113, 0)
        );
        assert_eq!(
            (
                v.rampage_target_ordinal,
                v.required_support_spawn,
                v.raiding_score,
                v.turns_in_enemy_territory
            ),
            (-1, 13, i32::MIN, i32::MAX)
        );
        assert_eq!((v.tech_steal_turns_cooldown, v.conversion_points), (-1, 0));
        assert!(
            v.properties.is_none()
                && v.clan_tech_hashes.is_none()
                && v.player_bribe_turns_remaining.is_none()
                && v.clan_player_stats.is_none()
                && v.incite_targets.is_none()
        );
        for cut in 0..113 {
            assert!(decode(&b[..cut], 113).is_err());
        }
        assert!(decode(&b, 112).is_err());
        let mut w = Writer {
            data: Vec::new(),
            items: 0,
            limits: Limits::default(),
        };
        assert_eq!(v.write(&mut w), Err(Error::UnsupportedLegacyWrite));
        assert!(w.data.is_empty());
        let mut current = specimen(255).instances.remove(0);
        current.properties = None;
        assert_eq!(current.write(&mut w), Err(Error::MissingCurrentField));
        assert!(w.data.is_empty());
        b[..4].copy_from_slice(&9u32.to_le_bytes());
        assert_eq!(
            decode(&b, 113),
            Err(Error::Version {
                expected: 15,
                actual: 9
            })
        );
    }
    #[test]
    fn manager3_missing_camps_and_clans_remain_unknown_prior() {
        let mut b = Vec::new();
        for word in [
            3u32,
            u32::MAX,
            0x81234567,
            0,
            u32::MAX,
            i32::MIN as u32,
            u32::MAX,
        ] {
            b.extend_from_slice(&word.to_le_bytes());
        }
        b.push(0xfe);
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.push(0xaa);
        let cap = Limits {
            max_bytes: 37,
            max_items: 0,
        };
        let (v, n, i) = Manager::decode_prefix_with_items(&b, cap).unwrap();
        assert_eq!(
            (v.wire_version, v.player_ordinal, v.flag_168_raw, n, i),
            (3, -1, 0xfe, 37, 0)
        );
        assert!(v.camp_locations.is_none() && v.clans.is_none());
        assert!(v.values_1a8.is_empty());
        assert_eq!(v.encode(cap), Err(Error::UnsupportedLegacyWrite));
        for cut in 0..37 {
            assert!(Manager::decode_prefix(&b[..cut], cap).is_err());
        }
        assert!(Manager::decode_prefix(
            &b,
            Limits {
                max_bytes: 36,
                ..cap
            }
        )
        .is_err());
        let mut current = specimen(255);
        current.clans = None;
        assert_eq!(
            current.encode(Limits::default()),
            Err(Error::MissingCurrentField)
        );
        b[..4].copy_from_slice(&4u32.to_le_bytes());
        assert_eq!(
            Manager::decode_prefix(&b, cap),
            Err(Error::Version {
                expected: 5,
                actual: 4
            })
        );
    }
    #[test]
    fn all_bool_carriers_and_small_truncations() {
        for flag in 0..=255 {
            let value = specimen(flag);
            let (bytes, items) = value.encode_with_items(generous()).unwrap();
            let exact = Limits {
                max_bytes: bytes.len(),
                max_items: items,
            };
            assert_eq!(
                Manager::decode_prefix_with_items(&bytes, exact).unwrap(),
                (value.clone(), bytes.len(), items)
            );
            assert_eq!(Manager::decode(&bytes, exact).unwrap(), value);
            for end in 0..bytes.len() {
                assert!(Manager::decode(&bytes[..end], exact).is_err());
            }
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
                assert_eq!(Manager::decode(&bytes, short), Err(Error::ResourceLimit));
                assert_eq!(value.encode(short), Err(Error::ResourceLimit));
            }
            let mut suffixed = bytes.clone();
            suffixed.push(0);
            assert_eq!(Manager::decode(&suffixed, exact), Err(Error::Trailing));
            assert_eq!(
                Manager::decode_prefix(&suffixed, exact).unwrap(),
                (value, bytes.len())
            );
        }
    }
    #[test]
    fn nested_versions_counts_and_hash_order() {
        let mut value = specimen(255);
        value.values_1a8 = vec![-1, 0, i32::MAX];
        value.instances[0].clan_tech_hashes = Some(vec![0xffffffff, 0, 0xffffffff, 0x80000000]);
        value.instances[0].needed_unit_tags = vec![
            AsciiString { bytes: vec![] },
            AsciiString {
                bytes: vec![0, 255, 128],
            },
        ];
        let bytes = value.encode(generous()).unwrap();
        assert_eq!(Manager::decode(&bytes, generous()).unwrap(), value);
        let mut r = Reader {
            data: &bytes,
            pos: 0,
            items: 0,
            allocated: 0,
            limits: generous(),
        };
        r.version(5).unwrap();
        r.raw(25).unwrap();
        Vec::<i32>::read(&mut r).unwrap();
        let count_site = r.pos;
        r.raw(4).unwrap();
        let instance_site = r.pos;
        let mut bad = bytes.clone();
        bad[instance_site..instance_site + 4].copy_from_slice(&14u32.to_le_bytes());
        assert_eq!(
            Manager::decode(&bad, generous()),
            Err(Error::Version {
                expected: 15,
                actual: 14
            })
        );
        for site in [29, count_site] {
            let mut bad = bytes.clone();
            bad[site..site + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert_eq!(Manager::decode(&bad, generous()), Err(Error::ResourceLimit));
        }
        let mut bad = bytes;
        bad[..4].copy_from_slice(&4u32.to_le_bytes());
        assert_eq!(
            Manager::decode(&bad, generous()),
            Err(Error::Version {
                expected: 5,
                actual: 4
            })
        );
    }
    #[test]
    fn empty_manager_has_no_context_defaults() {
        let value = Manager {
            wire_version: VERSION,
            player_ordinal: -1,
            value_c8: 0,
            value_e0: 1,
            value_f8: u32::MAX,
            control_priority_ordinal: i32::MIN,
            improvement_hash: u32::MAX,
            flag_168_raw: 128,
            values_1a8: vec![],
            instances: vec![],
            camp_locations: Some(vec![]),
            clans: Some(ClansManager {
                unique_unit_claims: vec![],
                value_100: -1,
            }),
        };
        let bytes = value
            .encode(Limits {
                max_bytes: 53,
                max_items: 0,
            })
            .unwrap();
        assert_eq!(bytes.len(), 53);
        assert_eq!(
            Manager::decode(
                &bytes,
                Limits {
                    max_bytes: 53,
                    max_items: 0
                }
            )
            .unwrap(),
            value
        );
    }
}
