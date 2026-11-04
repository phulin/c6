//! Current Game.Processor.ActivateNextPlayer selection (2a0bf90). Selection
//! does not perform SetTurnActive, its report, or first-activation upkeep.
use crate::{game_update::Error, loaded_state::LoadedState};

#[derive(Debug)]
#[must_use = "SetTurnActive(true) still owes its log/flags/report continuation"]
pub struct Active {
    owner: i32,
    stamp: crate::loaded_state::ActionStamp,
}
#[derive(Debug)]
#[must_use = "the actual synchronous activation report is still mandatory"]
pub struct ActiveReport {
    pub owner: i32,
    pub event: u32,
    stamp: crate::loaded_state::ActionStamp,
}
fn revision(state: &mut LoadedState) -> Result<(), Error> {
    state.revision = state
        .revision
        .checked_add(1)
        .ok_or(Error::Unsupported("activation revision capacity"))?;
    Ok(())
}
/// SetTurnActive2b5adc0 through the logging boundary. An already-active player
/// returns without writes or callbacks, exactly as the native XOR guard.
pub fn begin_active(state: &mut LoadedState, owner: i32) -> Result<Option<Active>, Error> {
    if state
        .turn_active(owner)
        .map_err(|_| Error::Unsupported("activation current activity"))?
    {
        return Ok(None);
    }
    revision(state)?;
    state.retained.core.player_processor.active_count = state
        .retained
        .core
        .player_processor
        .active_count
        .wrapping_add(1);
    let controls = state
        .controls
        .as_ref()
        .and_then(|c| c.players.iter().position(|p| p.owner == owner))
        .ok_or(Error::Unsupported("activation current controls"))?;
    let header = state
        .retained
        .players
        .iter()
        .position(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("activation canonical header"))?;
    revision(state)?;
    state.controls.as_mut().expect("checked").players[controls].turn_active = Some(true);
    state.retained.players[header].header.ordered_flags[2] = true;
    let owner_index = index(owner)?;
    revision(state)?;
    state.retained.core.player_processor.automation_processed[owner_index] = false;
    Ok(Some(Active {
        owner,
        stamp: state.action_stamp(),
    }))
}
/// Finish the source-qualified file log, then completion/activation flags,
/// human-first bookkeeping, optional actual hotseat switch and report lookup.
/// Native planning and first-activation gameplay are not executed here.
pub fn after_stock_log(
    state: &mut LoadedState,
    runtime: &mut super::Runtime,
    active: Active,
    _: super::deactivation::LoggingScope,
    hotseat: impl FnOnce(&LoadedState) -> Result<bool, Error>,
    set_local_player: impl FnOnce(&mut LoadedState, &mut super::Runtime, i32) -> Result<(), Error>,
) -> Result<ActiveReport, Error> {
    if state.action_stamp() != active.stamp || state.binding() != &runtime.binding {
        return Err(Error::Unsupported("activation log continuation state"));
    }
    let i = index(active.owner)?;
    revision(state)?;
    state.retained.core.player_processor.player_complete[i] = false;
    revision(state)?;
    state.retained.core.player_processor.activation_needed[i] = false;
    revision(state)?;
    state.retained.core.player_processor.activation_event[i] = true;
    if !state.retained.core.player_processor.human_first_activated
        && state
            .player_slot_status(active.owner)
            .map_err(|_| Error::Unsupported("activation human control"))?
            == 3
    {
        revision(state)?;
        state.retained.core.player_processor.human_first_activated = true;
    }
    if hotseat(state)?
        && state
            .player_slot_status(active.owner)
            .map_err(|_| Error::Unsupported("activation hotseat human control"))?
            == 3
    {
        set_local_player(state, runtime, active.owner)?;
    }
    let local = runtime
        .inputs
        .local_player
        .ok_or(Error::Unsupported("activation current local player"))?;
    Ok(ActiveReport {
        owner: active.owner,
        event: if active.owner == local {
            0x11c07b90
        } else {
            0xd7b3572b
        },
        stamp: state.action_stamp(),
    })
}
impl ActiveReport {
    pub fn deliver<E>(
        self,
        state: &mut LoadedState,
        send: impl FnOnce(&mut LoadedState, u32, i32) -> Result<(), E>,
    ) -> Result<(), super::deactivation::ReportError<E>> {
        if state.action_stamp() != self.stamp {
            return Err(super::deactivation::ReportError::Prefix(
                Error::Unsupported("activation report continuation state"),
            ));
        }
        send(state, self.event, self.owner).map_err(super::deactivation::ReportError::Effect)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use = "the selected activation or phase continuation remains mandatory"]
pub enum Selection {
    Sequential(i32),
    AdvancePhase,
    NoSequentialPlayer,
}
fn index(owner: i32) -> Result<usize, Error> {
    usize::try_from(owner)
        .ok()
        .filter(|&v| v < 64)
        .ok_or(Error::Unsupported("activation player index"))
}
fn needs(state: &LoadedState, owner: i32) -> Result<bool, Error> {
    if !state
        .player_alive(owner)
        .map_err(|_| Error::Unsupported("activation alive player"))?
        || state
            .turn_active(owner)
            .map_err(|_| Error::Unsupported("activation current activity"))?
    {
        return Ok(false);
    }
    Ok(state.retained.core.player_processor.activation_needed[index(owner)?])
}
fn sequential(
    state: &LoadedState,
    owner: i32,
    mode: &mut impl FnMut(&LoadedState) -> Result<u32, Error>,
) -> Result<bool, Error> {
    if !needs(state, owner)? {
        return Ok(false);
    }
    if state
        .player_slot_status(owner)
        .map_err(|_| Error::Unsupported("activation current control"))?
        != 3
    {
        return Ok(true);
    }
    let simultaneous = match mode(state)? {
        0xfd4af2d1 => true,
        0xbe9088c4 => state
            .retained
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Unsupported("activation current player header"))?
            .header
            .saved_simultaneous_turns(),
        _ => false,
    };
    Ok(!simultaneous)
}
/// `alive` is the current native GetAlive order. This selection has no callbacks
/// capable of replacing that storage, and all mutable fields are read currently.
/// The caller reaches this only after every selected deactivation completed.
pub fn select(
    state: &LoadedState,
    network_multiplayer: bool,
    alive: &[i32],
    mut phase_mode: impl FnMut(&LoadedState) -> Result<u32, Error>,
) -> Result<Selection, Error> {
    if state.retained.core.player_processor.active_count != 0 {
        return Err(Error::Unsupported(
            "next activation requires completed deactivation",
        ));
    }
    if network_multiplayer {
        for &owner in &state.retained.core.game_processor.turn_order {
            if state
                .world
                .player_is_valid(owner)
                .map_err(|_| Error::Unsupported("activation turn-order validity"))?
                && sequential(state, owner, &mut phase_mode)?
                && state
                    .player_slot_status(owner)
                    .map_err(|_| Error::Unsupported("activation current control"))?
                    != 3
            {
                return Ok(Selection::Sequential(owner));
            }
        }
    }
    if state.retained.core.phase_processor.current_phase == -1 {
        return Ok(Selection::AdvancePhase);
    }
    let mut any = false;
    for &owner in alive {
        if needs(state, owner)? {
            any = true;
            break;
        }
    }
    if !any {
        return Ok(Selection::AdvancePhase);
    }
    for &owner in &state.retained.core.game_processor.turn_order {
        if state
            .world
            .player_is_valid(owner)
            .map_err(|_| Error::Unsupported("activation turn-order validity"))?
            && sequential(state, owner, &mut phase_mode)?
        {
            return Ok(Selection::Sequential(owner));
        }
    }
    Ok(Selection::NoSequentialPlayer)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn current() -> (LoadedState, i32) {
        let (mut state, _, owner) = crate::loaded_state::tests::current_update_player_fixture();
        state
            .controls
            .as_mut()
            .unwrap()
            .players
            .iter_mut()
            .find(|p| p.owner == owner)
            .unwrap()
            .turn_active = Some(false);
        state.retained.players[0].header.ordered_flags[2] = false;
        state.retained.core.player_processor.active_count = 0;
        state.retained.core.player_processor.activation_needed[owner as usize] = true;
        state.retained.core.phase_processor.current_phase = 0;
        state.retained.core.game_processor.turn_order.fill(-1);
        state.retained.core.game_processor.turn_order[17] = owner;
        (state, owner)
    }
    #[test]
    fn dynamic_uses_canonical_simultaneous_flag_without_changing_control_roles() {
        let (mut state, owner) = current();
        state.retained.players[0].header.ordered_flags[3] = true;
        assert_eq!(
            select(&state, false, &[owner], |_| Ok(0xbe9088c4)),
            Ok(Selection::NoSequentialPlayer)
        );
        state.retained.players[0].header.ordered_flags[3] = false;
        assert_eq!(
            select(&state, false, &[owner], |_| Ok(0xbe9088c4)),
            Ok(Selection::Sequential(owner))
        );
        assert_eq!(state.player_slot_status(owner).unwrap(), 3);
    }
    #[test]
    fn active_prefix_and_hotseat_failure_keep_exact_flags_before_required_callback() {
        let (mut state, owner) = current();
        let mut runtime = super::super::Runtime::new(
            &state,
            super::super::Inputs {
                autoplay: super::super::AutoplayInputs {
                    end_turn_signaled: Some(false),
                    has_pending_state_change: Some(false),
                    pending_state: Some(false),
                    active: Some(false),
                },
                needs_update: Some(false),
                auto_player_game: Some(false),
                local_player: Some(owner),
            },
        );
        state.retained.core.player_processor.player_complete[owner as usize] = true;
        state.retained.core.player_processor.automation_processed[owner as usize] = true;
        let pending = begin_active(&mut state, owner).unwrap().unwrap();
        assert_eq!(state.retained.core.player_processor.active_count, 1);
        assert!(state.retained.players[0].header.saved_turn_active());
        assert!(state.retained.core.player_processor.player_complete[owner as usize]);
        assert!(!state.retained.core.player_processor.automation_processed[owner as usize]);
        let result = after_stock_log(
            &mut state,
            &mut runtime,
            pending,
            super::super::deactivation::LoggingScope::StockFileSink,
            |_| Err(Error::Unsupported("current hotseat query")),
            |_, _, _| panic!("no hotseat switch"),
        );
        assert!(result.is_err());
        assert!(!state.retained.core.player_processor.player_complete[owner as usize]);
        assert!(!state.retained.core.player_processor.activation_needed[owner as usize]);
        assert!(state.retained.core.player_processor.activation_event[owner as usize]);
    }
    #[test]
    fn already_active_skips_all_assignment_and_continuation_work() {
        let (mut state, owner) = current();
        let _ = begin_active(&mut state, owner).unwrap().unwrap();
        let stamp = state.action_stamp();
        assert!(begin_active(&mut state, owner).unwrap().is_none());
        assert_eq!(state.action_stamp(), stamp);
        assert_eq!(state.retained.core.player_processor.active_count, 1);
    }
    #[test]
    fn absent_phase_or_no_needed_player_skips_unreachable_mode_queries() {
        let (mut state, owner) = current();
        state.retained.core.phase_processor.current_phase = -1;
        assert_eq!(
            select(&state, false, &[-123], |_| panic!("phase not entered")),
            Ok(Selection::AdvancePhase)
        );
        state.retained.core.phase_processor.current_phase = 0;
        state.retained.core.player_processor.activation_needed[owner as usize] = false;
        assert_eq!(
            select(&state, false, &[owner], |_| panic!("no pending activation")),
            Ok(Selection::AdvancePhase)
        );
    }
}
