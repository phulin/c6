//! SetLocalPlayerTo's role-preserving hotseat handoff (2b52c80/2b52ca0).
use super::super::Runtime;
use crate::{game_update::Error, loaded_state::LoadedState};
#[derive(Debug)]
#[must_use = "changed local observer still requires its report and visibility refresh"]
pub struct Change {
    refresh: bool,
    stamp: crate::loaded_state::ActionStamp,
}
impl Runtime {
    /// Actual current Player.Manager+1c6c; independent of saved local player.
    pub fn bind_local_observer(&mut self, state: &LoadedState, observer: i32) -> Result<(), Error> {
        if state.binding() != &self.binding || self.local_observer.is_some() {
            return Err(Error::Unsupported("current local observer binding"));
        }
        self.local_observer = Some(observer);
        Ok(())
    }
    pub fn local_observer(&self) -> Option<i32> {
        self.local_observer
    }
}
pub fn set(state: &mut LoadedState, runtime: &mut Runtime, owner: i32) -> Result<Change, Error> {
    if runtime.failed || state.binding() != &runtime.binding {
        return Err(Error::Unsupported("current local-player runtime"));
    }
    // Native wrapper reads the current observer before its helper reads local.
    let old_observer = runtime
        .local_observer
        .ok_or(Error::Unsupported("current local observer"))?;
    let old = runtime
        .inputs
        .local_player
        .ok_or(Error::Unsupported("current local player"))?;
    let changed = old != owner;
    if changed {
        super::revision(state)?;
        state.retained.core.local_player = owner;
        runtime.inputs.local_player = Some(owner);
    }
    let observer = if owner == -1 { old_observer } else { owner };
    if observer != old_observer {
        super::revision(state)?;
        runtime.local_observer = Some(observer);
    }
    Ok(Change {
        refresh: changed || observer != old_observer,
        stamp: state.action_stamp(),
    })
}
impl Change {
    /// Actual disabled-query, conditionalc0372152 report, then RestoreVisibility.
    /// The scalar prefix is retained if either required callback fails.
    pub fn deliver<E>(
        self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        disabled: impl FnOnce(&LoadedState) -> Result<bool, E>,
        report: impl FnOnce(&mut LoadedState, u32, i32) -> Result<(), E>,
        restore: impl FnOnce(&mut LoadedState, i32) -> Result<(), E>,
    ) -> Result<(), super::super::deactivation::ReportError<E>> {
        use super::super::deactivation::ReportError;
        if runtime.failed
            || state.binding() != &runtime.binding
            || state.action_stamp() != self.stamp
        {
            return Err(ReportError::Prefix(Error::Unsupported(
                "local-player continuation",
            )));
        }
        if !self.refresh {
            return Ok(());
        }
        let result = (|| {
            if !disabled(state)? {
                report(
                    state,
                    0xc0372152,
                    runtime.inputs.local_player.expect("set checked local"),
                )?;
            }
            restore(state, runtime.local_observer.expect("set checked observer"))
        })();
        if let Err(error) = result {
            runtime.failed = true;
            return Err(ReportError::Effect(error));
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn runtime(state: &LoadedState) -> Runtime {
        Runtime::new(
            state,
            super::super::super::Inputs {
                autoplay: super::super::super::AutoplayInputs {
                    end_turn_signaled: None,
                    has_pending_state_change: None,
                    pending_state: None,
                    active: None,
                },
                needs_update: None,
                auto_player_game: None,
                local_player: Some(0),
            },
        )
    }
    #[test]
    fn report_failure_retains_local_and_observer_writes_and_skips_restore() {
        let mut state = crate::loaded_state::tests::state();
        let mut runtime = runtime(&state);
        runtime.bind_local_observer(&state, 0).unwrap();
        let change = set(&mut state, &mut runtime, 1).unwrap();
        assert_eq!(state.retained().core.local_player, 1);
        assert_eq!(runtime.local_observer(), Some(1));
        assert!(change
            .deliver(
                &mut state,
                &mut runtime,
                |_| Ok(false),
                |state, event, owner| {
                    assert_eq!(event, 0xc0372152);
                    assert_eq!(owner, 1);
                    assert_eq!(state.retained().core.local_player, 1);
                    Err(())
                },
                |_, _| panic!("report failed")
            )
            .is_err());
        assert!(runtime.failed());
    }
    #[test]
    fn unchanged_handoff_never_requires_visibility_services() {
        let mut state = crate::loaded_state::tests::state();
        let mut runtime = runtime(&state);
        runtime.bind_local_observer(&state, 0).unwrap();
        let change = set(&mut state, &mut runtime, 0).unwrap();
        change
            .deliver::<()>(
                &mut state,
                &mut runtime,
                |_| panic!("unchanged"),
                |_, _, _| panic!("unchanged"),
                |_, _| panic!("unchanged"),
            )
            .unwrap();
    }
}
