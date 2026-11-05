//! Stock phase callbacks and timer calculation on the current shared state.
//! Player selection and the surrounding round dispatcher remain caller-owned.
use super::activation::{DefinitionError, Phases};
use crate::{registry::linux_type_hash, Rules};
use c6_core::core_player::initialization::WireBinding;
use c6_core::{
    game_update::Error,
    loaded_state::{
        game_update::{activation, round},
        LoadedState,
    },
};
use std::collections::BTreeMap;

/// Initialized stock subscriptions, in source listener order. Added/custom
/// consumers require a different implementation rather than being dropped.
#[derive(Clone, Copy, Debug)]
pub enum Callback {
    Congress,
}
#[derive(Clone, Copy, Debug)]
pub enum Scope {
    StockNoAddedConsumers,
}
pub struct Bindings {
    binding: WireBinding,
    end: Vec<Callback>,
    begin: Vec<Callback>,
}
impl Bindings {
    pub fn from_initial(
        state: &LoadedState,
        end: Vec<Callback>,
        begin: Vec<Callback>,
        _: Scope,
    ) -> Self {
        Self {
            binding: state.binding().clone(),
            end,
            begin,
        }
    }
    fn matches(&self, state: &LoadedState) -> Result<(), Error> {
        if &self.binding == state.binding() {
            Ok(())
        } else {
            Err(Error::Unsupported("phase callback binding"))
        }
    }
}
/// All effects borrow the existing persistent gameplay backend. Activation
/// must execute the existing SetTurnActive continuation, not set a proxy flag.
pub trait Effects {
    fn report(&mut self, state: &mut LoadedState, event: u32, value: i32) -> Result<(), Error>;
    fn activate(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Error>;
    fn has_end_turn_blocking(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        kind: u32,
    ) -> Result<bool, Error>;
}
const RESOLUTIONS: u32 = 0xb52a667b;
const DISCUSSIONS: u32 = 0x2c2337c1;
fn congress(segment: u32, previous: Option<u32>) -> Result<(), Error> {
    // OnPhaseEnded and OnNewPhaseStarted return before all Congress mutation
    // and RNG on ordinary segments. Positive stages remain explicit.
    if [RESOLUTIONS, DISCUSSIONS].contains(&segment)
        || previous.is_some_and(|p| [RESOLUTIONS, DISCUSSIONS].contains(&p))
    {
        Err(Error::Unsupported("Congress phase gameplay"))
    } else {
        Ok(())
    }
}
pub fn end(
    state: &mut LoadedState,
    phases: &Phases,
    bindings: &Bindings,
    effects: &mut impl Effects,
) -> Result<(), Error> {
    bindings.matches(state)?;
    for callback in &bindings.end {
        match callback {
            Callback::Congress => congress(phases.current_segment(state)?, None)?,
        }
    }
    effects.report(
        state,
        0x93dd170f,
        state.retained().core.phase_processor.current_phase,
    )
}
pub fn begin(
    state: &mut LoadedState,
    phases: &Phases,
    bindings: &Bindings,
    alive: &[i32],
    previous: i32,
    effects: &mut impl Effects,
) -> Result<(), Error> {
    bindings.matches(state)?;
    round::begin_player_phase(state, alive, |state, event, value| {
        effects.report(state, event, value)
    })?;
    for callback in &bindings.begin {
        match callback {
            Callback::Congress => congress(phases.current_segment(state)?, Some(previous as u32))?,
        }
    }
    // The native fixed array is reread at each position after callbacks.
    for index in 0..64 {
        let owner = state.retained().core.game_processor.turn_order[index];
        if state
            .world()
            .player_is_valid(owner)
            .map_err(|_| Error::Unsupported("phase turn order validity"))?
            && activation::needs_simultaneous(state, owner, |s| phases.mode(s))?
        {
            effects.activate(state, owner)?;
        }
    }
    Ok(())
}
/// ActivatePlayersAtTurnStart prefix. The caller then invokes its existing
/// ActivateNextPlayer in the same round state machine.
pub fn set_turn_start_flags(state: &mut LoadedState, alive: &[i32]) -> Result<(), Error> {
    for &owner in alive {
        let mut simultaneous = true;
        if state
            .player_slot_status(owner)
            .map_err(|_| Error::Unsupported("phase current human control"))?
            == 3
        {
            for &other in alive {
                if other != owner
                    && state
                        .player_slot_status(other)
                        .map_err(|_| Error::Unsupported("phase other human control"))?
                        == 3
                    && state
                        .game_relations()
                        .map_err(|_| Error::Unsupported("phase current diplomacy"))?
                        .is_at_war(owner, other)
                        .map_err(|_| Error::Unsupported("phase human war"))?
                {
                    simultaneous = false;
                    break;
                }
            }
        }
        activation::set_simultaneous(state, owner, simultaneous)?;
    }
    Ok(())
}
#[derive(Clone, Copy)]
struct Segment {
    base: u32,
    city: u32,
    unit: u32,
    strategic: bool,
}
impl Segment {
    fn limit(self, cities: u32, units: u32, blocking: bool) -> u32 {
        self.base
            .wrapping_add(cities.wrapping_mul(self.city))
            .wrapping_add(units.wrapping_mul(self.unit))
            .wrapping_add(if blocking { 30 } else { 0 })
    }
}
pub struct Definitions {
    segments: BTreeMap<u32, Segment>,
}
impl Definitions {
    pub fn from_rules(rules: &Rules) -> Result<Self, DefinitionError> {
        let table = rules
            .gameplay()
            .table("TurnSegments")
            .ok_or(DefinitionError::Missing("TurnSegments"))?;
        let mut segments = BTreeMap::new();
        for row in table.rows() {
            let key = linux_type_hash(
                row.text("TurnSegmentType")
                    .map_err(DefinitionError::Rules)?
                    .as_bytes(),
            );
            segments.insert(
                key,
                Segment {
                    base: row
                        .integer("TimeLimit_Base")
                        .map_err(DefinitionError::Rules)? as u32,
                    city: row
                        .integer("TimeLimit_PerCity")
                        .map_err(DefinitionError::Rules)? as u32,
                    unit: row
                        .integer("TimeLimit_PerUnit")
                        .map_err(DefinitionError::Rules)? as u32,
                    strategic: row
                        .integer("AllowStrategicCommands")
                        .map_err(DefinitionError::Rules)?
                        != 0,
                },
            );
        }
        Ok(Self { segments })
    }
    /// Maxima are unsigned, not summed. Each human's counts precede its lazy
    /// blocking queries; subsequent humans are still visited after a match.
    pub fn time_limit(
        &self,
        state: &mut LoadedState,
        phases: &Phases,
        alive_majors: &[i32],
        effects: &mut impl Effects,
    ) -> Result<u32, Error> {
        let segment = phases.current_segment(state)?;
        if segment == u32::MAX {
            return Ok(0);
        }
        let segment = *self
            .segments
            .get(&segment)
            .ok_or(Error::Unsupported("current phase timer segment"))?;
        let (mut cities, mut units, mut blocking) = (0, 0, false);
        for &owner in alive_majors {
            if state
                .player_slot_status(owner)
                .map_err(|_| Error::Unsupported("phase timer human control"))?
                != 3
            {
                continue;
            }
            let player = state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == owner)
                .ok_or(Error::Unsupported("phase timer current player"))?;
            cities = cities.max(player.cities.records.entries.len() as u32);
            units = units.max(
                state
                    .world()
                    .player(owner)
                    .and_then(|p| p.unit_records())
                    .map_err(|_| Error::Unsupported("phase timer unit roster"))?
                    .len() as u32,
            );
            if segment.strategic {
                for kind in [0xa0064eea, 0x57bcfe42, 0x86bfebd1] {
                    if effects.has_end_turn_blocking(state, owner, kind)? {
                        blocking = true;
                        break;
                    }
                }
            }
        }
        Ok(segment.limit(cities, units, blocking))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timer_uses_raw_unsigned_wrapping_arithmetic() {
        let s = Segment {
            base: u32::MAX,
            city: 10,
            unit: 5,
            strategic: true,
        };
        assert_eq!(s.limit(3, 2, true), 69);
        assert_eq!(s.limit(u32::MAX, 0, false), u32::MAX.wrapping_sub(10));
    }
    #[test]
    fn ordinary_congress_callbacks_are_lazy_but_positive_stages_are_explicit() {
        assert!(congress(1, None).is_ok());
        assert!(congress(1, Some(u32::MAX)).is_ok());
        assert!(congress(RESOLUTIONS, None).is_err());
        assert!(congress(1, Some(DISCUSSIONS)).is_err());
    }
}
