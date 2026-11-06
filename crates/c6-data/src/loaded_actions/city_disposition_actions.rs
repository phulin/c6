//! Explicit Keep for the bounded Linux combat-capture decision.
//!
//! This transition preserves the ordinary XP1 Destroy handler's gameplay order.
//! Reached subscribers are mandatory services on the current state; no empty
//! default acknowledges an unimplemented callback. Other disposition directives,
//! cultural-identity transfers and dead/free-city previous owners remain outside
//! this combat-capture profile.
use crate::{
    city_capture::{self, Disposition, PendingDisposition},
    Rules,
};
use c6_core::{
    core_player::initialization::WireBinding, loaded_state::LoadedState, unit::ComponentId,
};

pub mod keep;
pub mod requirements;

pub const DESTROY_COMMAND: u32 = 0x7b72_d9ea;
pub const KEEP_DIRECTIVE: u32 = 0x029a_5a40;

pub struct Definitions {
    binding: WireBinding,
    capture: city_capture::Definitions,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, String> {
        Ok(Self {
            binding: state.binding().clone(),
            capture: city_capture::Definitions::from_rules(state, rules)?,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    Admission,
    Markers,
    CommandStarted,
    Occupation,
    OccupiedChanged,
    CityOccupied,
    Expulsion,
    Loyalty,
    Blocking,
    MessagesProcessed,
    Queue,
}
#[derive(Debug)]
pub struct Error {
    pub stage: Stage,
    pub detail: String,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Keep City {:?}: {}", self.stage, self.detail)
    }
}
impl std::error::Error for Error {}
pub(super) fn error(stage: Stage, detail: impl std::fmt::Debug) -> Error {
    Error {
        stage,
        detail: format!("{detail:?}"),
    }
}

/// Native callback sites, in source order. These describe synchronous work,
/// rather than a deferred event log. Each consumer must finish before returning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    /// BaseHandler.Start reporting 1d103404, command DESTROY/Keep.
    CommandStarted {
        city: ComponentId,
        district: ComponentId,
    },
    /// SetOccupied reporting de2d2813, only when the stored flag changes.
    OccupiedChanged { city: ComponentId },
    /// GameSignals.CityOccupied at +360; the first argument is the previous
    /// owner, not the current City owner. Includes diplomacy and requirements.
    CityOccupied {
        city: ComponentId,
        previous_owner: i32,
    },
    /// Diplomacy.ExpelForeignUnitsFromTerritory(reason5), before loyalty.
    ExpelForeignUnits { owner: i32 },
    /// CulturalIdentity.DoCityKept -> RecalculateConversionTrends. Reuse the
    /// ordinary loyalty kernel and current food/happiness/property/report owners.
    CityKept { city: ComponentId },
}
pub trait Effects {
    fn event(&mut self, state: &mut LoadedState, event: Event) -> Result<(), String>;
}

/// Project an initialized completed combat capture's semantic pending decision
/// from its actual current City, center, marker and rules. This never constructs
/// a historical combat record or marks a source-timed in-flight queue complete.
pub fn pending_from_current(
    state: &LoadedState,
    city: ComponentId,
    definitions: &Definitions,
    levels: &[(i32, u32)],
) -> Result<PendingDisposition, Error> {
    let owner = i32::from(city.owner);
    let value = state
        .city(owner, city.id)
        .map_err(|e| error(Stage::Admission, e))?;
    let mut center = None;
    for &id in &value.districts.district_ids {
        let district = state
            .district(owner, id)
            .map_err(|e| error(Stage::Admission, e))?;
        if district.district_hash == c6_core::city_district::CITY_CENTER_HASH
            && district.city_id == city.id
        {
            if center.is_some() {
                return Err(error(Stage::Admission, "multiple current CityCenters"));
            }
            center = Some(ComponentId {
                owner: city.owner,
                component_type: 3,
                id,
            });
        }
    }
    let choice = PendingDisposition {
        city,
        district: center.ok_or_else(|| error(Stage::Admission, "current CityCenter"))?,
        previous_owner: value.header.just_conquered_from,
        legal: city_capture::legal_dispositions(state, city, &definitions.capture, levels)
            .map_err(|e| error(Stage::Admission, e))?,
    };
    admit(state, &choice, definitions, levels)?;
    Ok(choice)
}

/// Revalidate the queue choice against the live city and rules. This does not
/// mutate state or consume RNG. The caller also authenticates queue membership
/// and completion of the enclosing melee before invoking this transition.
pub fn admit(
    state: &LoadedState,
    choice: &PendingDisposition,
    definitions: &Definitions,
    levels: &[(i32, u32)],
) -> Result<(), Error> {
    let fail = |e| error(Stage::Admission, e);
    if state.binding() != &definitions.binding
        || choice.city.component_type != 2
        || choice.district.component_type != 3
        || choice.district.owner != choice.city.owner
        || !choice.legal.contains(&Disposition::Keep)
    {
        return Err(fail("Keep decision identity/definition binding"));
    }
    let owner = i32::from(choice.city.owner);
    let city = state
        .city(owner, choice.city.id)
        .map_err(|e| error(Stage::Admission, e))?;
    let district = state
        .district(owner, choice.district.id)
        .map_err(|e| error(Stage::Admission, e))?;
    if city.header.owner != owner
        || city.header.just_conquered_from != choice.previous_owner
        || choice.previous_owner == owner
        || choice.previous_owner == -1
        || district.city_id != choice.city.id
        || district.district_hash != c6_core::city_district::CITY_CENTER_HASH
        || !city.districts.district_ids.contains(&choice.district.id)
    {
        return Err(fail("current captured City/CityCenter identity"));
    }
    if city.header.last_transfer_type as u32 != c6_core::city_capture::TRANSFER_COMBAT
        || city.header.owner_before_occupation != choice.previous_owner
    {
        return Err(fail("Keep supports ordinary combat capture only"));
    }
    let major = crate::registry::linux_type_hash(b"CIVILIZATION_LEVEL_FULL_CIV");
    for player in [owner, choice.previous_owner] {
        if levels.iter().find(|r| r.0 == player).map(|r| r.1) != Some(major)
            || !state
                .player_alive(player)
                .map_err(|e| error(Stage::Admission, e))?
        {
            return Err(fail("Keep requires current surviving major owners"));
        }
    }
    if !city_capture::legal_dispositions(state, choice.city, &definitions.capture, levels)
        .map_err(|e| error(Stage::Admission, e))?
        .contains(&Disposition::Keep)
    {
        return Err(fail("Keep no longer legal"));
    }
    Ok(())
}

/// XP1 Destroy.Start2820030, ordinary combat Keep branch28204e5.
/// An error after admission retains its applied prefix; the enclosing Session
/// must remain failed and keep its pending semantic choice for inspection.
pub fn keep(
    state: &mut LoadedState,
    choice: &PendingDisposition,
    definitions: &Definitions,
    levels: &[(i32, u32)],
    effects: &mut impl Effects,
) -> Result<(), Error> {
    admit(state, choice, definitions, levels)?;
    let owner = i32::from(choice.city.owner);
    let city = state
        .city_mut(owner, choice.city.id)
        .map_err(|e| error(Stage::Markers, e))?;
    city.header.just_conquered_from = -1;
    city.header.just_rebelled_from = -1;
    effects
        .event(
            state,
            Event::CommandStarted {
                city: choice.city,
                district: choice.district,
            },
        )
        .map_err(|e| error(Stage::CommandStarted, e))?;
    // The previous owner survives and is a major: the dead/free-city global
    // parameter branch and CityCeded signal are not reached in this profile.
    let occupied = state
        .game_relations()
        .map_err(|e| error(Stage::Occupation, e))?
        .is_at_war(owner, choice.previous_owner)
        .map_err(|e| error(Stage::Occupation, e))?;
    let previous = state
        .city(owner, choice.city.id)
        .map_err(|e| error(Stage::Occupation, e))?
        .header
        .occupied;
    state
        .city_mut(owner, choice.city.id)
        .map_err(|e| error(Stage::Occupation, e))?
        .header
        .occupied = occupied;
    if previous != occupied {
        effects
            .event(state, Event::OccupiedChanged { city: choice.city })
            .map_err(|e| error(Stage::OccupiedChanged, e))?;
    }
    // Native assigns true even when IsAtWar returned false; do not replace
    // this with CheckIfOccupyingCities or derive it from the resulting flag.
    let at = usize::try_from(choice.previous_owner)
        .ok()
        .filter(|&i| i < 64)
        .ok_or_else(|| error(Stage::Occupation, "occupation target"))?;
    state
        .war_diplomacy_mut(owner)
        .map_err(|e| error(Stage::Occupation, e))?
        .occupies_original_city[at] = true;
    effects
        .event(
            state,
            Event::CityOccupied {
                city: choice.city,
                previous_owner: choice.previous_owner,
            },
        )
        .map_err(|e| error(Stage::CityOccupied, e))?;
    effects
        .event(state, Event::ExpelForeignUnits { owner })
        .map_err(|e| error(Stage::Expulsion, e))?;
    effects
        .event(state, Event::CityKept { city: choice.city })
        .map_err(|e| error(Stage::Loyalty, e))
}

pub(super) fn allow_strategic(
    phases: &crate::unit_operation_manager::PhaseDefinitions,
    current: i32,
) -> Result<bool, Error> {
    if current == -1 {
        return Ok(true);
    }
    let phase = usize::try_from(current)
        .ok()
        .and_then(|i| phases.slots().get(i))
        .ok_or_else(|| error(Stage::Admission, "unchecked City command phase index"))?;
    Ok(phase
        .as_ref()
        .and_then(|p| p.active_segment.as_ref())
        .is_none_or(|segment| segment.flags & 1 != 0))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit_operation_manager::{
        PhaseDefinitions, TurnPhaseDefinition, TurnSegmentDefinition,
    };
    #[test]
    fn keep_city_command_uses_strategic_permission_independently_of_tactical() {
        let phase = |flags| {
            Some(TurnPhaseDefinition {
                row_id: crate::RulesRowId(1),
                phase_type_hash: 0,
                phase_order: 0,
                active_segment: Some(TurnSegmentDefinition {
                    row_id: crate::RulesRowId(1),
                    flags,
                }),
            })
        };
        let phases = PhaseDefinitions::from_native_slots(vec![phase(1), phase(2), None]);
        assert!(allow_strategic(&phases, 0).unwrap());
        assert!(!allow_strategic(&phases, 1).unwrap());
        assert!(allow_strategic(&phases, 2).unwrap());
        assert!(allow_strategic(&phases, -1).unwrap());
        assert!(allow_strategic(&phases, -2).is_err());
        assert!(allow_strategic(&phases, 3).is_err());
    }
}
