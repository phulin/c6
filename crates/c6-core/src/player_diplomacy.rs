//! Bounded Linux XP2 Player.Diplomacy wire records (versions41/50).
//! Missing arrays retain explicit reset/registry effects. Version41 also carries
//! 64 typed Discussion lists; nested component versions remain independent/strict.
//!
//! Ordered maps retain wire order and duplicate keys. Registry hashes remain
//! hashes; this codec does not apply native Reset/Read fixups or advance gameplay.
use crate::player_diplomacy_blackboard::{Blackboard, Error as BlackboardError};
use crate::unit::{HashValue, Limits};

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    UnsupportedLegacyWrite,
    Trailing,
    ResourceLimit,
    InvalidBool(u8),
    Version { expected: u32, actual: u32 },
    Blackboard(BlackboardError),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player.Diplomacy wire: {self:?}")
    }
}
impl std::error::Error for Error {}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    items: usize,
    limits: Limits,
    last_version_component: &'static str,
}
impl Reader<'_> {
    fn raw(&mut self, n: usize) -> Result<&[u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let value = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(value)
    }
    fn charge(&mut self, n: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
    fn version(&mut self, expected: u32, component: &'static str) -> Result<(), Error> {
        self.last_version_component = component;
        let actual = u32::read(self)?;
        if actual != expected {
            return Err(Error::Version { expected, actual });
        }
        Ok(())
    }
    fn remaining(&self) -> Limits {
        Limits {
            max_bytes: self.limits.max_bytes - self.pos,
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
    fn raw(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let n = self
            .data
            .len()
            .checked_add(bytes.len())
            .ok_or(Error::ResourceLimit)?;
        if n > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        self.data
            .try_reserve(bytes.len())
            .map_err(|_| Error::ResourceLimit)?;
        self.data.extend_from_slice(bytes);
        Ok(())
    }
    fn charge(&mut self, n: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
    fn remaining(&self) -> Limits {
        Limits {
            max_bytes: self.limits.max_bytes - self.data.len(),
            max_items: self.limits.max_items - self.items,
        }
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
            value => Err(Error::InvalidBool(value)),
        }
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.raw(&[u8::from(*self)])
    }
}
impl<T: Wire + Copy + Default, const N: usize> Wire for [T; N] {
    const MIN: usize = T::MIN * N;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let mut out = [T::default(); N];
        for value in &mut out {
            *value = T::read(r)?;
        }
        Ok(out)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        for value in self {
            value.write(w)?;
        }
        Ok(())
    }
}
impl<T: Wire> Wire for Vec<T> {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = u32::read(r)? as usize;
        r.charge(n)?;
        if n > r.data.len().saturating_sub(r.pos) / T::MIN {
            return Err(Error::Truncated);
        }
        if n.checked_mul(std::mem::size_of::<T>())
            .ok_or(Error::ResourceLimit)?
            > r.limits.max_bytes
        {
            return Err(Error::ResourceLimit);
        }
        let mut out = Vec::new();
        out.try_reserve_exact(n).map_err(|_| Error::ResourceLimit)?;
        for _ in 0..n {
            out.push(T::read(r)?);
        }
        Ok(out)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.charge(self.len())?;
        u32::try_from(self.len())
            .map_err(|_| Error::ResourceLimit)?
            .write(w)?;
        for value in self {
            value.write(w)?;
        }
        Ok(())
    }
}
impl<A: Wire, B: Wire> Wire for (A, B) {
    const MIN: usize = A::MIN + B::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok((A::read(r)?, B::read(r)?))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.0.write(w)?;
        self.1.write(w)
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
impl Wire for Blackboard {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let (value, bytes, items) = Self::decode_prefix_with_items(&r.data[r.pos..], r.remaining())
            .map_err(Error::Blackboard)?;
        r.charge(items)?;
        r.raw(bytes)?;
        Ok(value)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        let (bytes, items) = self
            .encode_with_items(w.remaining())
            .map_err(Error::Blackboard)?;
        w.charge(items)?;
        w.raw(&bytes)
    }
}
macro_rules! record {
    ($name:ident, $version:expr, { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
        pub struct $name { $(pub $field: $ty),* }
        impl Wire for $name {
            const MIN: usize = 4 + $(<$ty as Wire>::MIN +)* 0;
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                r.version($version, stringify!($name))?;
                Ok(Self { $($field: <$ty>::read(r)?),* })
            }
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                ($version as u32).write(w)?;
                $(self.$field.write(w)?;)*
                Ok(())
            }
        }
    };
}
/// All nine booleans are present in admitted41/50. Native memory order differs
/// from wire order; this DTO follows wire order and retains the independent tag.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CasusBelliData {
    pub wire_version: u32,
    pub colonial_war: bool,
    pub holy_war: bool,
    pub liberation_war: bool,
    pub protectorate_war: bool,
    pub reconquest_war: bool,
    pub territorial_war: bool,
    pub golden_age_war: bool,
    pub retribution_war: bool,
    pub ideological_war: bool,
}
impl Default for CasusBelliData {
    /// Empty current wire data, not execution of a native constructor.
    fn default() -> Self {
        Self {
            wire_version: 50,
            colonial_war: false,
            holy_war: false,
            liberation_war: false,
            protectorate_war: false,
            reconquest_war: false,
            territorial_war: false,
            golden_age_war: false,
            retribution_war: false,
            ideological_war: false,
        }
    }
}
impl Wire for CasusBelliData {
    const MIN: usize = 13;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.last_version_component = "CasusBelliData";
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 41 | 50) {
            return Err(Error::Version {
                expected: 50,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            colonial_war: bool::read(r)?,
            holy_war: bool::read(r)?,
            liberation_war: bool::read(r)?,
            protectorate_war: bool::read(r)?,
            reconquest_war: bool::read(r)?,
            territorial_war: bool::read(r)?,
            golden_age_war: bool::read(r)?,
            retribution_war: bool::read(r)?,
            ideological_war: bool::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 50 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        50u32.write(w)?;
        self.colonial_war.write(w)?;
        self.holy_war.write(w)?;
        self.liberation_war.write(w)?;
        self.protectorate_war.write(w)?;
        self.reconquest_war.write(w)?;
        self.territorial_war.write(w)?;
        self.golden_age_war.write(w)?;
        self.retribution_war.write(w)?;
        self.ideological_war.write(w)?;
        Ok(())
    }
}
/// Independently versioned promise wire data; omitted41 booleans have explicit
/// native Read defaults, rather than a claim about native constructor state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PromiseData {
    pub wire_version: u32,
    pub turn_made: i32,
    pub skipped: bool,
    pub broken: bool,
    pub broken_notification_sent: bool,
    pub report_repeat_offense: bool,
    pub turn_broken: i32,
}
impl Default for PromiseData {
    fn default() -> Self {
        Self {
            wire_version: 50,
            turn_made: 0,
            skipped: false,
            broken: false,
            broken_notification_sent: false,
            report_repeat_offense: false,
            turn_broken: 0,
        }
    }
}
impl Wire for PromiseData {
    const MIN: usize = 14;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.last_version_component = "PromiseData";
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 41 | 50) {
            return Err(Error::Version {
                expected: 50,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            turn_made: i32::read(r)?,
            skipped: bool::read(r)?,
            broken: bool::read(r)?,
            broken_notification_sent: if wire_version == 50 {
                bool::read(r)?
            } else {
                false
            },
            report_repeat_offense: if wire_version == 50 {
                bool::read(r)?
            } else {
                false
            },
            turn_broken: i32::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 50 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        50u32.write(w)?;
        self.turn_made.write(w)?;
        self.skipped.write(w)?;
        self.broken.write(w)?;
        self.broken_notification_sent.write(w)?;
        self.report_repeat_offense.write(w)?;
        self.turn_broken.write(w)
    }
}
record!(MessageData, 1, { is_kudo: bool, is_hidden_agenda: bool, recipient: i32, message_hash: u32 });
/// A wire projection: source_hash is retained before native registry lookup.
/// The independent 41/50 tag is read but does not change these three fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VisibilityAddition {
    pub wire_version: u32,
    pub amount: i32,
    pub source_hash: u32,
    pub source_type: i32,
}
impl Default for VisibilityAddition {
    fn default() -> Self {
        Self {
            wire_version: 50,
            amount: 0,
            source_hash: 0,
            source_type: 0,
        }
    }
}
impl Wire for VisibilityAddition {
    const MIN: usize = 16;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.last_version_component = "VisibilityAddition";
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 41 | 50) {
            return Err(Error::Version {
                expected: 50,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            amount: i32::read(r)?,
            source_hash: u32::read(r)?,
            source_type: i32::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 50 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        50u32.write(w)?;
        self.amount.write(w)?;
        self.source_hash.write(w)?;
        self.source_type.write(w)
    }
}
record!(YieldModifier, 2, { source: i32, turns_active: i32, yield_type: i32, modifier: i32, stack: bool });
record!(MovementModifier, 1, { source: i32, turns_active: i32, extra_moves: i32 });
record!(CombatModifier, 1, { source: i32, turns_active: i32, combat_bonus: i32, religious_only: bool });
record!(ActionPrereqOverride, 1, { action_hash: u32, civic_hash: u32, tech_hash: u32 });
record!(BannedActionCivilization, 1, { action_hash: u32, civilization_hash: u32 });

/// Current version-1 inner vector_map framing (distinct from its u32 count).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeMap<T> {
    pub entries: Vec<T>,
}
impl<T> From<Vec<T>> for TypeMap<T> {
    fn from(entries: Vec<T>) -> Self {
        Self { entries }
    }
}
impl<T: Wire> Wire for TypeMap<T> {
    const MIN: usize = 8;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(1, "TypeMap")?;
        Ok(Self {
            entries: Vec::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        1u32.write(w)?;
        self.entries.write(w)
    }
}
/// Counted ordered player/enum maps. Keys remain raw native enum words.
pub type GossipReports = Vec<(i32, TypeMap<(i32, u32)>)>;
pub type Promises = Vec<(i32, TypeMap<(i32, PromiseData)>)>;
pub type Incursions = Vec<(i32, TypeMap<(i32, Vec<i32>)>)>;

/// Exactly64 serialized Discussion IndexLists, without an outer wire count.
/// Native Read sets the default tofalse and resets all actual Discussion
/// definition cells tofalse before overlaying these pairs. Definition counts
/// and mappings remain unknown here; empty pairs do not mean an empty array.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LegacyDiscussionBaseline {
    NativeFalseResetDiscussionDefinitionCountUnknown,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyCongressDiscussionLists {
    pub native_baseline: LegacyDiscussionBaseline,
    pub players: [Vec<HashValue<bool>>; 64],
}
impl LegacyCongressDiscussionLists {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let mut players = std::array::from_fn(|_| Vec::new());
        for row in &mut players {
            *row = Vec::read(r)?;
        }
        Ok(Self {
            players,
            native_baseline:
                LegacyDiscussionBaseline::NativeFalseResetDiscussionDefinitionCountUnknown,
        })
    }
}
/// Complete current serialized component only. Fixed 64-player arrays have no
/// wire count; their elements do not consume the dynamic item budget.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diplomacy {
    pub wire_version: u32,
    /// Present only in admitted41. None at50 means no wire data, not absent
    /// native/gameplay state; native discussion state is not excluded by name.
    pub legacy_available_congress_discussions: Option<LegacyCongressDiscussionLists>,
    pub had_friendly_meeting: [bool; 64],
    pub has_delegation: [bool; 64],
    pub has_embassy: [bool; 64],
    pub open_borders_from_count: [i32; 64],
    pub open_borders_ended_turn: [i32; 64],
    pub can_enforce_borders: bool,
    pub denounced: [i32; 64],
    pub cities_razed: [i32; 64],
    pub cities_liberated: [i32; 64],
    pub tiles_culture_bombed: [i32; 64],
    pub nukes_used_against_you: [i32; 64],
    pub converted_city: [bool; 64],
    pub occupies_original_city: [bool; 64],
    pub offered_deal: [i32; 64],
    pub war_weariness: [i32; 64],
    pub warmonger_modifier: [i32; 64],
    pub era_player_met: [i32; 64],
    pub yield_per_delegation: Vec<HashValue<i32>>,
    pub yield_per_embassy: Vec<HashValue<i32>>,
    pub war_and_alliance_modifiers: [i32; 10],
    pub warmonger_scaling_raw: i32,
    pub no_occupation_penalties: bool,
    pub blackboard: Blackboard,
    pub city_war_weariness: Vec<(u32, i32)>,
    pub casus_belli: [CasusBelliData; 64],
    pub yield_modifiers: Vec<YieldModifier>,
    pub movement_modifiers: Vec<MovementModifier>,
    pub combat_modifiers: Vec<CombatModifier>,
    pub action_prereq_overrides: Vec<ActionPrereqOverride>,
    pub banned_action_civilizations: Vec<BannedActionCivilization>,
    pub last_war_and_city_turns: [i32; 10],
    pub all_alliances_shared_visibility: bool,
    pub grievance_generation_modifier: i32,
    pub grievance_decay_modifier: i32,
    pub grievance_decay_disabled: bool,
    pub alliance_and_favor_modifiers: [i32; 9],
    pub visibility: Vec<i32>,
    pub visibility_source_hashes: Vec<u32>,
    pub visibility_additions: Vec<VisibilityAddition>,
    pub gossip_reports: GossipReports,
    pub held_messages: Vec<MessageData>,
    pub promises_made: Promises,
    pub incursions_made: Incursions,
    pub last_promises_made: Promises,
    pub units_blocked_entry: Vec<HashValue<bool>>,
    pub alliance_points_per_player: Vec<i32>,
    pub banned_action_counts: Vec<HashValue<i32>>,
}
impl Wire for Diplomacy {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 41 | 50) {
            return Err(Error::Version {
                expected: 50,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            had_friendly_meeting: <[bool; 64]>::read(r)?,
            has_delegation: <[bool; 64]>::read(r)?,
            has_embassy: <[bool; 64]>::read(r)?,
            open_borders_from_count: <[i32; 64]>::read(r)?,
            open_borders_ended_turn: <[i32; 64]>::read(r)?,
            can_enforce_borders: <bool>::read(r)?,
            denounced: <[i32; 64]>::read(r)?,
            cities_razed: <[i32; 64]>::read(r)?,
            cities_liberated: <[i32; 64]>::read(r)?,
            tiles_culture_bombed: <[i32; 64]>::read(r)?,
            nukes_used_against_you: <[i32; 64]>::read(r)?,
            converted_city: <[bool; 64]>::read(r)?,
            occupies_original_city: <[bool; 64]>::read(r)?,
            offered_deal: <[i32; 64]>::read(r)?,
            war_weariness: <[i32; 64]>::read(r)?,
            warmonger_modifier: <[i32; 64]>::read(r)?,
            era_player_met: <[i32; 64]>::read(r)?,
            yield_per_delegation: if wire_version == 50 {
                <Vec<HashValue<i32>>>::read(r)?
            } else {
                Vec::new()
            },
            yield_per_embassy: if wire_version == 50 {
                <Vec<HashValue<i32>>>::read(r)?
            } else {
                Vec::new()
            },
            war_and_alliance_modifiers: <[i32; 10]>::read(r)?,
            warmonger_scaling_raw: <i32>::read(r)?,
            no_occupation_penalties: <bool>::read(r)?,
            blackboard: <Blackboard>::read(r)?,
            city_war_weariness: <Vec<(u32, i32)>>::read(r)?,
            casus_belli: <[CasusBelliData; 64]>::read(r)?,
            yield_modifiers: <Vec<YieldModifier>>::read(r)?,
            movement_modifiers: <Vec<MovementModifier>>::read(r)?,
            combat_modifiers: <Vec<CombatModifier>>::read(r)?,
            action_prereq_overrides: <Vec<ActionPrereqOverride>>::read(r)?,
            banned_action_civilizations: <Vec<BannedActionCivilization>>::read(r)?,
            last_war_and_city_turns: <[i32; 10]>::read(r)?,
            all_alliances_shared_visibility: <bool>::read(r)?,
            grievance_generation_modifier: if wire_version == 50 {
                <i32>::read(r)?
            } else {
                0
            },
            grievance_decay_modifier: if wire_version == 50 {
                <i32>::read(r)?
            } else {
                0
            },
            grievance_decay_disabled: if wire_version == 50 {
                <bool>::read(r)?
            } else {
                false
            },
            alliance_and_favor_modifiers: {
                let mut values = [0; 9];
                for value in values
                    .iter_mut()
                    .take(if wire_version == 41 { 7 } else { 9 })
                {
                    *value = i32::read(r)?;
                }
                values
            },
            visibility: <Vec<i32>>::read(r)?,
            visibility_source_hashes: <Vec<u32>>::read(r)?,
            visibility_additions: <Vec<VisibilityAddition>>::read(r)?,
            gossip_reports: <GossipReports>::read(r)?,
            held_messages: <Vec<MessageData>>::read(r)?,
            promises_made: <Promises>::read(r)?,
            incursions_made: <Incursions>::read(r)?,
            last_promises_made: <Promises>::read(r)?,
            units_blocked_entry: if wire_version == 50 {
                <Vec<HashValue<bool>>>::read(r)?
            } else {
                Vec::new()
            },
            alliance_points_per_player: <Vec<i32>>::read(r)?,
            banned_action_counts: <Vec<HashValue<i32>>>::read(r)?,
            legacy_available_congress_discussions: if wire_version == 41 {
                Some(LegacyCongressDiscussionLists::read(r)?)
            } else {
                None
            },
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 50 || self.legacy_available_congress_discussions.is_some() {
            return Err(Error::UnsupportedLegacyWrite);
        }
        50u32.write(w)?;
        self.had_friendly_meeting.write(w)?;
        self.has_delegation.write(w)?;
        self.has_embassy.write(w)?;
        self.open_borders_from_count.write(w)?;
        self.open_borders_ended_turn.write(w)?;
        self.can_enforce_borders.write(w)?;
        self.denounced.write(w)?;
        self.cities_razed.write(w)?;
        self.cities_liberated.write(w)?;
        self.tiles_culture_bombed.write(w)?;
        self.nukes_used_against_you.write(w)?;
        self.converted_city.write(w)?;
        self.occupies_original_city.write(w)?;
        self.offered_deal.write(w)?;
        self.war_weariness.write(w)?;
        self.warmonger_modifier.write(w)?;
        self.era_player_met.write(w)?;
        self.yield_per_delegation.write(w)?;
        self.yield_per_embassy.write(w)?;
        self.war_and_alliance_modifiers.write(w)?;
        self.warmonger_scaling_raw.write(w)?;
        self.no_occupation_penalties.write(w)?;
        self.blackboard.write(w)?;
        self.city_war_weariness.write(w)?;
        self.casus_belli.write(w)?;
        self.yield_modifiers.write(w)?;
        self.movement_modifiers.write(w)?;
        self.combat_modifiers.write(w)?;
        self.action_prereq_overrides.write(w)?;
        self.banned_action_civilizations.write(w)?;
        self.last_war_and_city_turns.write(w)?;
        self.all_alliances_shared_visibility.write(w)?;
        self.grievance_generation_modifier.write(w)?;
        self.grievance_decay_modifier.write(w)?;
        self.grievance_decay_disabled.write(w)?;
        self.alliance_and_favor_modifiers.write(w)?;
        self.visibility.write(w)?;
        self.visibility_source_hashes.write(w)?;
        self.visibility_additions.write(w)?;
        self.gossip_reports.write(w)?;
        self.held_messages.write(w)?;
        self.promises_made.write(w)?;
        self.incursions_made.write(w)?;
        self.last_promises_made.write(w)?;
        self.units_blocked_entry.write(w)?;
        self.alliance_points_per_player.write(w)?;
        self.banned_action_counts.write(w)?;
        Ok(())
    }
}
/// Cursor attribution from the shared decoder. `consumed` is the number of
/// bytes consumed before failure, not an original native failure PC. Delegated
/// Blackboard errors retain that child's start because its decoder returns no
/// partial consumption. Direct version errors include their four-byte tag.
#[derive(Debug, Eq, PartialEq)]
pub struct Boundary {
    pub consumed: usize,
    pub component: &'static str,
    pub error: Error,
}
impl Diplomacy {
    /// False at41: native resets each omitted list to0 using the actual Yield
    /// definition count; empty transport vectors do not establish empty arrays.
    pub fn delegation_embassy_yields_wire_present(&self) -> bool {
        self.wire_version == 50
    }
    /// False at41: native assigns false at every actual Unit definition index.
    /// That registry/count is not reconstructed by wire transport.
    pub fn blocked_entry_wire_present(&self) -> bool {
        self.wire_version == 50
    }
    /// Dynamic item charges include all nested containers and Blackboard strings.
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        Self::decode_prefix_with_boundary(bytes, limits).map_err(|boundary| boundary.error)
    }
    /// Same parser and shared budget, additionally attributing nested version
    /// refusals without retrying, scanning or skipping input bytes.
    pub fn decode_prefix_with_boundary(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Boundary> {
        let mut r = Reader {
            last_version_component: "Diplomacy",
            data: bytes,
            pos: 0,
            items: 0,
            limits,
        };
        let value = Self::read(&mut r).map_err(|error| Boundary {
            consumed: r.pos,
            component: match &error {
                Error::Blackboard(_) => "Blackboard",
                Error::Version { .. } => r.last_version_component,
                _ => "Diplomacy field",
            },
            error,
        })?;
        Ok((value, r.pos, r.items))
    }
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        let (value, bytes, _) = Self::decode_prefix_with_items(bytes, limits)?;
        Ok((value, bytes))
    }
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let (value, used) = Self::decode_prefix(bytes, limits)?;
        if used != bytes.len() {
            return Err(Error::Trailing);
        }
        Ok(value)
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
        self.encode_with_items(limits).map(|x| x.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Diplomacy {
        let mut board_bytes = 13u32.to_le_bytes().to_vec();
        board_bytes.resize(4 + 4 * 4 + 3 * 4 + 9 * 64 * 4, 0);
        let blackboard = Blackboard::decode(&board_bytes, Limits::default()).unwrap();
        let mut value = Diplomacy {
            wire_version: 50,
            legacy_available_congress_discussions: None,
            had_friendly_meeting: [false; 64],
            has_delegation: [false; 64],
            has_embassy: [false; 64],
            open_borders_from_count: [0; 64],
            open_borders_ended_turn: [0; 64],
            can_enforce_borders: false,
            denounced: [0; 64],
            cities_razed: [0; 64],
            cities_liberated: [0; 64],
            tiles_culture_bombed: [0; 64],
            nukes_used_against_you: [0; 64],
            converted_city: [false; 64],
            occupies_original_city: [false; 64],
            offered_deal: [0; 64],
            war_weariness: [0; 64],
            warmonger_modifier: [0; 64],
            era_player_met: [0; 64],
            yield_per_delegation: Vec::new(),
            yield_per_embassy: Vec::new(),
            war_and_alliance_modifiers: [0; 10],
            warmonger_scaling_raw: 0,
            no_occupation_penalties: false,
            blackboard,
            city_war_weariness: Vec::new(),
            casus_belli: [CasusBelliData::default(); 64],
            yield_modifiers: Vec::new(),
            movement_modifiers: Vec::new(),
            combat_modifiers: Vec::new(),
            action_prereq_overrides: Vec::new(),
            banned_action_civilizations: Vec::new(),
            last_war_and_city_turns: [0; 10],
            all_alliances_shared_visibility: false,
            grievance_generation_modifier: 0,
            grievance_decay_modifier: 0,
            grievance_decay_disabled: false,
            alliance_and_favor_modifiers: [0; 9],
            visibility: Vec::new(),
            visibility_source_hashes: Vec::new(),
            visibility_additions: Vec::new(),
            gossip_reports: Vec::new(),
            held_messages: Vec::new(),
            promises_made: Vec::new(),
            incursions_made: Vec::new(),
            last_promises_made: Vec::new(),
            units_blocked_entry: Vec::new(),
            alliance_points_per_player: Vec::new(),
            banned_action_counts: Vec::new(),
        };
        value.had_friendly_meeting[63] = true;
        value.open_borders_from_count[0] = i32::MIN;
        value.era_player_met[63] = i32::MAX;
        value.warmonger_scaling_raw = -257;
        value.yield_per_delegation = vec![HashValue {
            definition_hash: u32::MAX,
            value: -7,
        }];
        value.yield_per_embassy = value.yield_per_delegation.clone();
        value.city_war_weariness = vec![(u32::MAX, i32::MIN)];
        value.casus_belli[3].ideological_war = true;
        value.yield_modifiers = vec![YieldModifier {
            source: -1,
            yield_type: 4,
            modifier: -333,
            stack: true,
            turns_active: 2,
        }];
        value.movement_modifiers = vec![MovementModifier {
            extra_moves: -9,
            ..MovementModifier::default()
        }];
        value.combat_modifiers = vec![CombatModifier {
            religious_only: true,
            ..CombatModifier::default()
        }];
        value.action_prereq_overrides = vec![ActionPrereqOverride {
            action_hash: 0xdeadbeef,
            civic_hash: 0x80000000,
            tech_hash: 0,
        }];
        value.banned_action_civilizations = vec![BannedActionCivilization {
            action_hash: u32::MAX,
            civilization_hash: 1,
        }];
        value.visibility = vec![-1, i32::MAX];
        value.visibility_source_hashes = vec![u32::MAX, 3];
        value.visibility_additions = vec![VisibilityAddition {
            wire_version: 50,
            amount: -1,
            source_hash: u32::MAX,
            source_type: -1,
        }];
        value.gossip_reports = vec![(1, vec![(2, u32::MAX), (2, 3)].into()), (1, vec![].into())];
        value.held_messages = vec![MessageData {
            is_kudo: true,
            is_hidden_agenda: true,
            recipient: -1,
            message_hash: u32::MAX,
        }];
        value.promises_made = vec![(
            2,
            vec![(
                3,
                PromiseData {
                    wire_version: 50,
                    turn_made: i32::MIN,
                    skipped: true,
                    broken: true,
                    broken_notification_sent: true,
                    report_repeat_offense: true,
                    turn_broken: i32::MAX,
                },
            )]
            .into(),
        )];
        value.last_promises_made = value.promises_made.clone();
        value.incursions_made = vec![(1, vec![(2, vec![i32::MIN, 4]), (2, vec![])].into())];
        value.units_blocked_entry = vec![HashValue {
            definition_hash: u32::MAX,
            value: true,
        }];
        value.alliance_points_per_player = vec![i32::MIN, i32::MAX];
        value.banned_action_counts = vec![HashValue {
            definition_hash: 0x80000000,
            value: -1,
        }];
        value
    }
    #[test]
    fn promise41_omitted_flags_default_false_without_consuming_tail() {
        let mut bytes = 41u32.to_le_bytes().to_vec();
        bytes.extend(i32::MIN.to_le_bytes());
        bytes.extend([1, 0]);
        bytes.extend(i32::MAX.to_le_bytes());
        bytes.extend([0xaa, 0xbb]);
        let limits = Limits {
            max_bytes: 14,
            max_items: 0,
        };
        let read = |data| {
            let mut r = Reader {
                data,
                pos: 0,
                items: 0,
                limits,
                last_version_component: "Diplomacy",
            };
            let value = PromiseData::read(&mut r)?;
            Ok::<_, Error>((value, r.pos, r.items))
        };
        let (value, used, items) = read(&bytes).unwrap();
        assert_eq!((used, items), (14, 0));
        assert_eq!(
            value,
            PromiseData {
                wire_version: 41,
                turn_made: i32::MIN,
                skipped: true,
                broken: false,
                broken_notification_sent: false,
                report_repeat_offense: false,
                turn_broken: i32::MAX
            }
        );
        for n in 0..14 {
            assert!(read(&bytes[..n]).is_err(), "{n}");
        }
        let mut bad = bytes.clone();
        bad[9] = 2;
        assert_eq!(read(&bad), Err(Error::InvalidBool(2)));
        let mut bad_version = bytes.clone();
        bad_version[..4].copy_from_slice(&43u32.to_le_bytes());
        assert_eq!(
            read(&bad_version),
            Err(Error::Version {
                expected: 50,
                actual: 43
            })
        );
        let mut w = Writer {
            data: Vec::new(),
            items: 0,
            limits,
        };
        assert_eq!(value.write(&mut w), Err(Error::UnsupportedLegacyWrite));
        assert!(w.data.is_empty());
    }
    #[test]
    fn visibility41_own_tag_preserves_wire_hash_and_signed_fields() {
        let mut bytes = 41u32.to_le_bytes().to_vec();
        bytes.extend(i32::MIN.to_le_bytes());
        bytes.extend(0xfedcba98u32.to_le_bytes());
        bytes.extend((-1i32).to_le_bytes());
        bytes.extend([0xaa, 0xbb]);
        let limits = Limits {
            max_bytes: 16,
            max_items: 0,
        };
        let read = |data| {
            let mut r = Reader {
                data,
                pos: 0,
                items: 0,
                limits,
                last_version_component: "Diplomacy",
            };
            let value = VisibilityAddition::read(&mut r)?;
            Ok::<_, Error>((value, r.pos, r.items))
        };
        let (value, used, items) = read(&bytes).unwrap();
        assert_eq!((used, items), (16, 0));
        assert_eq!(
            value,
            VisibilityAddition {
                wire_version: 41,
                amount: i32::MIN,
                source_hash: 0xfedcba98,
                source_type: -1
            }
        );
        for n in 0..16 {
            assert!(read(&bytes[..n]).is_err(), "{n}");
        }
        let mut bad = bytes.clone();
        bad[..4].copy_from_slice(&42u32.to_le_bytes());
        assert_eq!(
            read(&bad),
            Err(Error::Version {
                expected: 50,
                actual: 42
            })
        );
        let mut w = Writer {
            data: Vec::new(),
            items: 0,
            limits,
        };
        assert_eq!(value.write(&mut w), Err(Error::UnsupportedLegacyWrite));
        assert!(w.data.is_empty());
    }
    #[test]
    fn casus41_own_tag_and_nine_bool_order_match_current_shape() {
        let mut bytes = 41u32.to_le_bytes().to_vec();
        bytes.extend([1, 0, 1, 0, 1, 0, 1, 0, 1]);
        bytes.extend([0xaa, 0xbb]);
        let limits = Limits {
            max_bytes: 13,
            max_items: 0,
        };
        let mut r = Reader {
            data: &bytes,
            pos: 0,
            items: 0,
            limits,
            last_version_component: "Diplomacy",
        };
        let v = CasusBelliData::read(&mut r).unwrap();
        assert_eq!((r.pos, r.items), (13, 0));
        assert_eq!(v.wire_version, 41);
        assert_eq!(
            [
                v.colonial_war,
                v.holy_war,
                v.liberation_war,
                v.protectorate_war,
                v.reconquest_war,
                v.territorial_war,
                v.golden_age_war,
                v.retribution_war,
                v.ideological_war
            ],
            [true, false, true, false, true, false, true, false, true]
        );
        let mut w = Writer {
            data: Vec::new(),
            items: 0,
            limits,
        };
        assert_eq!(v.write(&mut w), Err(Error::UnsupportedLegacyWrite));
        for n in 0..13 {
            let mut r = Reader {
                data: &bytes[..n],
                pos: 0,
                items: 0,
                limits,
                last_version_component: "Diplomacy",
            };
            assert!(CasusBelliData::read(&mut r).is_err(), "{n}");
        }
        let mut bad = bytes.clone();
        bad[12] = 2;
        let mut r = Reader {
            data: &bad,
            pos: 0,
            items: 0,
            limits,
            last_version_component: "Diplomacy",
        };
        assert_eq!(CasusBelliData::read(&mut r), Err(Error::InvalidBool(2)));
    }
    #[test]
    fn diplomacy41_complete_outer_grammar_keeps_late_discussion_payload() {
        // Independent literal source grammar. Nested data intentionally uses
        // independent tags: Casus41 and Blackboard13. Other older nested
        // versions still refuse rather than borrowing the outer41 tag.
        let mut bytes = 41u32.to_le_bytes().to_vec();
        bytes.extend([0; 3 * 64]);
        bytes.extend([0; 2 * 64 * 4]);
        bytes.push(1);
        bytes.extend([0; 5 * 64 * 4]);
        bytes.extend([0; 2 * 64]);
        bytes.extend([0; 4 * 64 * 4]);
        // No delegation/embassy Yield lists at41.
        for value in 1i32..=10 {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend((-257i32).to_le_bytes());
        bytes.push(1);
        let mut board = 13u32.to_le_bytes().to_vec();
        board.resize(4 + 4 * 4 + 3 * 4 + 9 * 64 * 4, 0);
        bytes.extend(board);
        bytes.extend(0u32.to_le_bytes()); // City war weariness
        let casus_at = bytes.len();
        for _ in 0..64 {
            bytes.extend(41u32.to_le_bytes());
            bytes.extend([0; 9]);
        }
        for _ in 0..5 {
            bytes.extend(0u32.to_le_bytes());
        } // modifier/override vectors
        for value in 11i32..=20 {
            bytes.extend(value.to_le_bytes());
        }
        bytes.push(1); // shared alliance visibility is already present at41
                       // Three grievance scalars are absent, then seven of nine alliance/favor ints.
        for value in 101i32..=107 {
            bytes.extend(value.to_le_bytes());
        }
        for _ in 0..2 {
            bytes.extend(0u32.to_le_bytes());
        } // visibility lists
        bytes.extend(1u32.to_le_bytes()); // one independently tagged addition
        bytes.extend(41u32.to_le_bytes());
        bytes.extend((-9i32).to_le_bytes());
        bytes.extend(0xfedcba98u32.to_le_bytes());
        bytes.extend((-1i32).to_le_bytes());
        for _ in 0..2 {
            bytes.extend(0u32.to_le_bytes());
        } // gossip/messages
        bytes.extend(1u32.to_le_bytes()); // one player's promises
        bytes.extend(9i32.to_le_bytes());
        bytes.extend(1u32.to_le_bytes()); // TypeMap own version
        bytes.extend(1u32.to_le_bytes()); // one promise
        bytes.extend((-1i32).to_le_bytes());
        bytes.extend(41u32.to_le_bytes());
        bytes.extend((-77i32).to_le_bytes());
        bytes.extend([1, 0]);
        bytes.extend(123i32.to_le_bytes());
        for _ in 0..2 {
            bytes.extend(0u32.to_le_bytes());
        } // incursions/last promises
          // No Unit blocked-entry list at41; alliance points/action counts remain.
        for _ in 0..2 {
            bytes.extend(0u32.to_le_bytes());
        }
        for player in 0..64 {
            bytes.extend((if player == 3 { 2u32 } else { 0 }).to_le_bytes());
            if player == 3 {
                for value in [1u8, 0] {
                    bytes.extend(u32::MAX.to_le_bytes());
                    bytes.push(value);
                }
            }
        }
        let end = bytes.len();
        bytes.extend([0xaa, 0xbb]);
        let limits = Limits {
            max_bytes: end,
            max_items: 5,
        };
        let (v, used, items) = Diplomacy::decode_prefix_with_items(&bytes, limits).unwrap();
        assert_eq!((used, items), (end, 5));
        assert_eq!(v.wire_version, 41);
        assert!(
            v.can_enforce_borders && v.no_occupation_penalties && v.all_alliances_shared_visibility
        );
        assert_eq!(
            v.war_and_alliance_modifiers,
            [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        );
        assert_eq!(v.warmonger_scaling_raw, -257);
        assert_eq!(v.grievance_generation_modifier, 0);
        assert_eq!(v.grievance_decay_modifier, 0);
        assert!(!v.grievance_decay_disabled);
        assert_eq!(
            v.alliance_and_favor_modifiers,
            [101, 102, 103, 104, 105, 106, 107, 0, 0]
        );
        assert_eq!(
            v.visibility_additions,
            vec![VisibilityAddition {
                wire_version: 41,
                amount: -9,
                source_hash: 0xfedcba98,
                source_type: -1
            }]
        );
        assert!(!v.delegation_embassy_yields_wire_present());
        assert!(!v.blocked_entry_wire_present());
        assert_eq!(v.promises_made[0].0, 9);
        let promise = &v.promises_made[0].1.entries[0].1;
        assert_eq!(
            (promise.wire_version, promise.turn_made, promise.turn_broken),
            (41, -77, 123)
        );
        assert!(promise.skipped && !promise.broken);
        assert!(!promise.broken_notification_sent && !promise.report_repeat_offense);
        let legacy = v.legacy_available_congress_discussions.as_ref().unwrap();
        assert_eq!(legacy.players[3].len(), 2);
        assert_eq!(
            legacy.native_baseline,
            LegacyDiscussionBaseline::NativeFalseResetDiscussionDefinitionCountUnknown
        );
        assert_eq!(v.encode(limits), Err(Error::UnsupportedLegacyWrite));
        for n in 0..end {
            assert!(
                Diplomacy::decode_prefix(&bytes[..n], limits).is_err(),
                "{n}"
            );
        }
        assert_eq!(
            Diplomacy::decode_prefix(
                &bytes,
                Limits {
                    max_items: 1,
                    ..limits
                }
            ),
            Err(Error::ResourceLimit)
        );
        let mut nested = bytes.clone();
        nested[casus_at..casus_at + 4].copy_from_slice(&42u32.to_le_bytes());
        let boundary = Diplomacy::decode_prefix_with_boundary(&nested, limits).unwrap_err();
        assert_eq!(
            (boundary.component, boundary.consumed),
            ("CasusBelliData", casus_at + 4)
        );
        assert_eq!(
            boundary.error,
            Error::Version {
                expected: 50,
                actual: 42
            }
        );
        assert_eq!(
            Diplomacy::decode_prefix(&nested, limits),
            Err(boundary.error)
        );
        for version in [40u32, 42, 49, 51] {
            let mut bad = bytes.clone();
            bad[..4].copy_from_slice(&version.to_le_bytes());
            assert_eq!(
                Diplomacy::decode_prefix(&bad, limits),
                Err(Error::Version {
                    expected: 50,
                    actual: version
                })
            );
        }
    }
    #[test]
    fn legacy_discussion_lists_have_fixed64_framing_and_no_false_baseline() {
        let mut bytes = Vec::new();
        for player in 0..64 {
            let count = if player == 3 { 2u32 } else { 0 };
            bytes.extend(count.to_le_bytes());
            if count != 0 {
                for value in [1u8, 0] {
                    bytes.extend(u32::MAX.to_le_bytes());
                    bytes.push(value);
                }
            }
        }
        let end = bytes.len();
        bytes.extend([0xaa, 0xbb]);
        let mut r = Reader {
            last_version_component: "Diplomacy",
            data: &bytes,
            pos: 0,
            items: 0,
            limits: Limits {
                max_bytes: end,
                max_items: 2,
            },
        };
        let v = LegacyCongressDiscussionLists::read(&mut r).unwrap();
        assert_eq!((r.pos, r.items), (end, 2));
        assert_eq!(
            v.players[3],
            [
                HashValue {
                    definition_hash: u32::MAX,
                    value: true
                },
                HashValue {
                    definition_hash: u32::MAX,
                    value: false
                }
            ]
        );
        assert!(v.players[0].is_empty());
        for n in 0..end {
            let mut r = Reader {
                last_version_component: "Diplomacy",
                data: &bytes[..n],
                pos: 0,
                items: 0,
                limits: Limits::default(),
            };
            assert!(LegacyCongressDiscussionLists::read(&mut r).is_err(), "{n}");
        }
        let mut r = Reader {
            last_version_component: "Diplomacy",
            data: &bytes,
            pos: 0,
            items: 0,
            limits: Limits {
                max_bytes: end,
                max_items: 1,
            },
        };
        assert_eq!(
            LegacyCongressDiscussionLists::read(&mut r),
            Err(Error::ResourceLimit)
        );
    }
    #[test]
    fn nonempty_ordered_roundtrip_and_aggregate_budget() {
        let value = fixture();
        let (bytes, items) = value.encode_with_items(Limits::default()).unwrap();
        // Every counted vector/map entry is charged, including nested empty maps.
        assert_eq!(items, 31);
        let limits = Limits {
            max_bytes: bytes.len(),
            max_items: items,
        };
        assert_eq!(
            Diplomacy::decode_prefix_with_items(&bytes, limits).unwrap(),
            (value.clone(), bytes.len(), items)
        );
        assert_eq!(
            value.encode_with_items(limits).unwrap(),
            (bytes.clone(), items)
        );
        for limits in [
            Limits {
                max_bytes: bytes.len() - 1,
                max_items: items,
            },
            Limits {
                max_bytes: bytes.len(),
                max_items: items - 1,
            },
        ] {
            assert!(Diplomacy::decode(&bytes, limits).is_err());
            assert!(value.encode(limits).is_err());
        }
        let mut trailing = bytes.clone();
        trailing.push(8);
        assert_eq!(
            Diplomacy::decode(&trailing, Limits::default()),
            Err(Error::Trailing)
        );
        assert_eq!(
            Diplomacy::decode_prefix(&trailing, Limits::default())
                .unwrap()
                .1,
            bytes.len()
        );
    }
    #[test]
    fn every_truncation_and_invalid_outer_prefix() {
        let bytes = fixture().encode(Limits::default()).unwrap();
        for cut in 0..bytes.len() {
            assert!(
                Diplomacy::decode(&bytes[..cut], Limits::default()).is_err(),
                "cut {cut}"
            );
        }
        let mut bad = bytes.clone();
        bad[0] = 49;
        assert_eq!(
            Diplomacy::decode(&bad, Limits::default()),
            Err(Error::Version {
                expected: 50,
                actual: 49
            })
        );
        bad = bytes;
        bad[4] = 2;
        assert_eq!(
            Diplomacy::decode(&bad, Limits::default()),
            Err(Error::InvalidBool(2))
        );
    }
    fn nested<T: Wire + Eq + std::fmt::Debug>(value: T, version: u32) {
        let mut w = Writer {
            data: Vec::new(),
            items: 0,
            limits: Limits::default(),
        };
        value.write(&mut w).unwrap();
        let mut r = Reader {
            last_version_component: "Diplomacy",
            data: &w.data,
            pos: 0,
            items: 0,
            limits: Limits::default(),
        };
        assert_eq!(T::read(&mut r).unwrap(), value);
        assert_eq!(r.pos, w.data.len());
        for cut in 0..w.data.len() {
            let mut r = Reader {
                last_version_component: "Diplomacy",
                data: &w.data[..cut],
                pos: 0,
                items: 0,
                limits: Limits::default(),
            };
            assert!(T::read(&mut r).is_err());
        }
        w.data[..4].copy_from_slice(&(version - 1).to_le_bytes());
        let mut r = Reader {
            last_version_component: "Diplomacy",
            data: &w.data,
            pos: 0,
            items: 0,
            limits: Limits::default(),
        };
        assert!(matches!(T::read(&mut r), Err(Error::Version { .. })));
    }
    #[test]
    fn nested_versions_are_independent() {
        nested(CasusBelliData::default(), 50);
        nested(PromiseData::default(), 50);
        nested(MessageData::default(), 1);
        nested(VisibilityAddition::default(), 50);
        nested(YieldModifier::default(), 2);
        nested(MovementModifier::default(), 1);
        nested(CombatModifier::default(), 1);
        nested(ActionPrereqOverride::default(), 1);
        nested(BannedActionCivilization::default(), 1);
        nested(
            TypeMap {
                entries: vec![(i32::MIN, u32::MAX)],
            },
            1,
        );
        nested(TypeMap::<(i32, PromiseData)> { entries: vec![] }, 1);
    }
    #[test]
    fn malicious_count_rejected_before_allocation() {
        let bytes = u32::MAX.to_le_bytes();
        let mut r = Reader {
            last_version_component: "Diplomacy",
            data: &bytes,
            pos: 0,
            items: 0,
            limits: Limits::default(),
        };
        assert_eq!(Vec::<i32>::read(&mut r), Err(Error::ResourceLimit));
    }
}
