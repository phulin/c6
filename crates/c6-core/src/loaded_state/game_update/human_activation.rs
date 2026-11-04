//! Canonical human AI.Control.Manager activation ordering (Linux XP2).
//! Planning policies remain external. Required off-turn, priority and report
//! consumers are synchronous; an error retains the actual completed prefix.
use crate::{game_update::Error, loaded_state::LoadedState, player_ai_control::ControlManager};

fn control(state: &LoadedState, owner: i32) -> Result<&ControlManager, Error> {
    let control = state
        .retained
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .and_then(|p| p.ai_control.as_ref())
        .ok_or(Error::Unsupported("human activation Control.Manager"))?;
    if control.player_id != owner {
        return Err(Error::Unsupported("human activation Control owner"));
    }
    Ok(control)
}
fn edit(state: &mut LoadedState, owner: i32) -> Result<&mut ControlManager, Error> {
    control(state, owner)?;
    let index = state
        .retained
        .players
        .iter()
        .position(|p| p.header.owner == owner)
        .expect("checked");
    state.revision = state
        .revision
        .checked_add(1)
        .ok_or(Error::Unsupported("human activation revision overflow"))?;
    Ok(state.retained.players[index]
        .ai_control
        .as_mut()
        .expect("checked"))
}
/// These are actual source invocation boundaries, not successful defaults.
pub trait Effects {
    fn wait_for_jobs(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Error>;
    /// CollectorOne result: nonzero subscriber status means another outer
    /// dispatch is needed; status2 also terminates that inner ring traversal.
    fn off_turn(&mut self, state: &mut LoadedState, owner: i32) -> Result<bool, Error>;
    fn priority_callbacks(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        full_turn: bool,
    ) -> Result<(), Error>;
    fn report(&mut self, state: &mut LoadedState, owner: i32, event: u32) -> Result<(), Error>;
}
pub fn current_priority(state: &LoadedState, owner: i32) -> Result<i32, Error> {
    Ok(control(state, owner)?.priority)
}
/// FAutoVariable priority assignment only archives the scalar. It does not
/// synchronously emit the distinct Player.Signals control-priority signal.
pub fn set_priority(state: &mut LoadedState, owner: i32, value: i32) -> Result<(), Error> {
    edit(state, owner)?.priority = value;
    Ok(())
}
fn cleanup_controllers(state: &mut LoadedState, owner: i32, units: &[u32]) -> Result<(), Error> {
    for &id in units {
        let controller = state
            .world()
            .player(owner)
            .map_err(|_| Error::Unsupported("activation current unit owner"))?
            .unit(id)
            .map_err(|_| Error::Unsupported("activation current ordered unit"))?
            .ok_or(Error::Unsupported(
                "activation current ordered unit missing",
            ))?
            .record
            .ai_controller;
        if controller == u32::MAX {
            continue;
        }
        if !control(state, owner)?
            .controls
            .instances
            .iter()
            .any(|c| c.id == controller)
        {
            state
                .unit_mut(owner, id)
                .map_err(|_| Error::Unsupported("activation stale controller assignment"))?
                .ai_controller = u32::MAX;
        }
    }
    Ok(())
}
fn drain_off_turn<E: Effects>(
    state: &mut LoadedState,
    owner: i32,
    effects: &mut E,
) -> Result<(), Error> {
    let mut retries = u32::MAX;
    loop {
        let pending = effects.off_turn(state, owner)?;
        effects.wait_for_jobs(state, owner)?;
        if !pending {
            break;
        }
        retries = retries.wrapping_add(1);
        if retries >= 25 {
            break;
        }
    }
    Ok(())
}
/// Profiling clock and exclusive host critical-section mechanics are omitted;
/// callback order, current control flags and unit ownership remain canonical.
pub fn activate<E: Effects>(
    state: &mut LoadedState,
    owner: i32,
    units: &[u32],
    full_turn: bool,
    effects: &mut E,
) -> Result<(), Error> {
    if state
        .player_slot_status(owner)
        .map_err(|_| Error::Unsupported("activation current human control"))?
        != 3
    {
        return Err(Error::ExternalDecisionRequired(
            "nonhuman AI activation policies",
        ));
    }
    edit(state, owner)?.running_as_human = true;
    // Native ProcessAi branch is guarded by !runningAsHuman && !IsHuman.
    effects.wait_for_jobs(state, owner)?;
    drain_off_turn(state, owner, effects)?;
    edit(state, owner)?.has_out_of_turn_processing = false;
    cleanup_controllers(state, owner, units)?;
    set_priority(state, owner, 4)?;
    while current_priority(state, owner)? >= 0 {
        effects.priority_callbacks(state, owner, full_turn)?;
        let next = current_priority(state, owner)?.wrapping_sub(1);
        set_priority(state, owner, next)?;
    }
    set_priority(state, owner, 4)?;
    effects.report(state, owner, 0xa235ddf7)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Calls {
        pending: usize,
        calls: Vec<&'static str>,
        fail_wait: bool,
    }
    impl Effects for Calls {
        fn wait_for_jobs(&mut self, _: &mut LoadedState, _: i32) -> Result<(), Error> {
            self.calls.push("wait");
            if self.fail_wait {
                Err(Error::Unsupported("join"))
            } else {
                Ok(())
            }
        }
        fn off_turn(&mut self, _: &mut LoadedState, _: i32) -> Result<bool, Error> {
            self.calls.push("dispatch");
            let result = self.pending > 0;
            self.pending = self.pending.saturating_sub(1);
            Ok(result)
        }
        fn priority_callbacks(
            &mut self,
            _: &mut LoadedState,
            _: i32,
            _: bool,
        ) -> Result<(), Error> {
            unreachable!()
        }
        fn report(&mut self, _: &mut LoadedState, _: i32, _: u32) -> Result<(), Error> {
            unreachable!()
        }
    }
    #[test]
    fn completed_dispatch_still_joins_jobs() {
        let mut state = crate::loaded_state::tests::state();
        let mut calls = Calls {
            pending: 0,
            calls: vec![],
            fail_wait: false,
        };
        drain_off_turn(&mut state, 0, &mut calls).unwrap();
        assert_eq!(calls.calls, ["dispatch", "wait"]);
    }
    #[test]
    fn native_retry_counter_allows_twenty_six_dispatches() {
        let mut state = crate::loaded_state::tests::state();
        let mut calls = Calls {
            pending: usize::MAX,
            calls: vec![],
            fail_wait: false,
        };
        drain_off_turn(&mut state, 0, &mut calls).unwrap();
        assert_eq!(calls.calls.len(), 52);
        assert!(calls
            .calls
            .chunks_exact(2)
            .all(|pair| pair == ["dispatch", "wait"]));
    }
    #[test]
    fn failed_job_join_skips_later_dispatches() {
        let mut state = crate::loaded_state::tests::state();
        let mut calls = Calls {
            pending: 2,
            calls: vec![],
            fail_wait: true,
        };
        assert!(drain_off_turn(&mut state, 0, &mut calls).is_err());
        assert_eq!(calls.calls, ["dispatch", "wait"]);
    }
}
