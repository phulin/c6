//! Concrete selected phase inputs for the Linux round processor. Callbacks
//! remain mandatory for reached automation, signals and timer notification queries.
use super::Phases;
use crate::unit_operation_manager::PhaseDefinitions;
use c6_core::{
    game_update::Error,
    loaded_state::{game_update::round as core, LoadedState},
};

/// Actual effects outside selected-definition lookup. In particular, disabled
/// segment lookup needs the live native definition index/remapping; saved pair
/// order alone does not establish that index list.
pub trait Effects {
    fn automation(&mut self, state: &mut LoadedState) -> Result<(), Error>;
    fn end_phase(&mut self, state: &mut LoadedState) -> Result<(), Error>;
    fn segment_disabled(&mut self, state: &LoadedState, hash: u32) -> Result<bool, Error>;
    fn time_limit(&mut self, state: &mut LoadedState) -> Result<u32, Error>;
    fn begin_phase(&mut self, state: &mut LoadedState, previous: i32) -> Result<(), Error>;
}
struct Current<'a, E> {
    phases: &'a Phases,
    definitions: &'a PhaseDefinitions,
    effects: &'a mut E,
}
impl<E: Effects> core::PhaseEffects for Current<'_, E> {
    fn allow_tactical(&mut self, state: &LoadedState) -> Result<bool, Error> {
        self.definitions
            .allow_tactical_commands(state.retained().core.phase_processor.current_phase, -1)
            .map_err(|_| Error::Unsupported("round selected phase tactical permission"))
    }
    fn automation(&mut self, state: &mut LoadedState) -> Result<(), Error> {
        self.effects.automation(state)
    }
    fn end_phase(&mut self, state: &mut LoadedState) -> Result<(), Error> {
        self.effects.end_phase(state)
    }
    fn current_segment(&mut self, state: &LoadedState) -> Result<u32, Error> {
        self.phases.current_segment(state)
    }
    fn selected_count(&mut self) -> Result<usize, Error> {
        Ok(self.phases.segments.len())
    }
    fn segment_at(&mut self, index: usize) -> Result<u32, Error> {
        // ActivateNextTurnPhase dereferences the actual selected phase pointer;
        // a null slot is not the nullable GetCurrentTurnSegment fallback.
        self.phases
            .segments
            .get(index)
            .copied()
            .flatten()
            .ok_or(Error::Unsupported(
                "round null selected phase/active segment",
            ))
    }
    fn segment_disabled(&mut self, state: &LoadedState, hash: u32) -> Result<bool, Error> {
        self.effects.segment_disabled(state, hash)
    }
    fn time_limit(&mut self, state: &mut LoadedState) -> Result<u32, Error> {
        self.effects.time_limit(state)
    }
    fn begin_phase(&mut self, state: &mut LoadedState, previous: i32) -> Result<(), Error> {
        self.effects.begin_phase(state, previous)
    }
}
/// Advance with the same actual selected phase slots used by next-player
/// selection. Exhaustion preserves current index/time limit; caller still owes
/// ActivateNextPlayer's completion/automation checks before Game.EndTurn.
pub fn advance_phase(
    state: &mut LoadedState,
    phases: &Phases,
    definitions: &PhaseDefinitions,
    changed: &mut bool,
    effects: &mut impl Effects,
) -> Result<core::PhaseAdvance, Error> {
    core::advance_phase(
        state,
        changed,
        &mut Current {
            phases,
            definitions,
            effects,
        },
    )
}
