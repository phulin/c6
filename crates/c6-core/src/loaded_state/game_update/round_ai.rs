//! Player.TurnManager.DoBeginTurn2ba1750 -> AI.Control.Manager.BeginTurn201e660.
//! Processing depth is an unsaved current value on the SAME Game.Update runtime.
use super::{Error, LoadedState, Runtime};

/// Install the observed GlobalAi+110 processing depth. No seed/default is used.
/// As with other Runtime live inputs, callers supply facts for this state.
pub fn install_processing_depth(
    state: &LoadedState,
    runtime: &mut Runtime,
    depth: i32,
) -> Result<(), Error> {
    if runtime.failed || state.binding() != &runtime.binding {
        return Err(Error::Unsupported("current GlobalAi processing owner"));
    }
    runtime.global_ai_processing_depth = Some(depth);
    Ok(())
}
pub fn processing_depth(runtime: &Runtime) -> Option<i32> {
    runtime.global_ai_processing_depth
}
fn change_depth(runtime: &mut Runtime, amount: i32) -> Result<(), Error> {
    let value = runtime
        .global_ai_processing_depth
        .as_mut()
        .ok_or(Error::Unsupported("current GlobalAi processing depth"))?;
    // ProcessAi2020cb0 and EndAiProcessing2020e80 each perform one wrapping add.
    *value = value.wrapping_add(amount);
    Ok(())
}
fn set_processing(state: &mut LoadedState, owner: i32, value: bool) -> Result<(), Error> {
    state.revision = state
        .revision
        .checked_add(1)
        .ok_or(Error::Unsupported("AI begin revision capacity"))?;
    state
        .retained
        .players
        .iter_mut()
        .find(|p| p.requested_owner == owner)
        .and_then(|p| p.ai_control.as_mut())
        .ok_or(Error::Unsupported("current AI Control Manager"))?
        .has_out_of_turn_processing = value;
    Ok(())
}
/// Dispatch the actual Player.Signals+1e0 CollectorOr ring on the same owner.
/// It may create controls or change gameplay state; no human bypass is valid.
pub trait PriorityEffects {
    fn priority_requested(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        owner: i32,
        priority: i32,
    ) -> Result<bool, Error>;
}
/// Execute current processing-depth/flag changes and the real priority signal.
/// The native empty control collection needs no validity/destruction queries.
/// Nonempty controls require their omitted current pointer/destruction fields;
/// the saved four-field Instance is insufficient to invent those answers.
pub fn begin_turn(
    state: &mut LoadedState,
    runtime: &mut Runtime,
    owner: i32,
    effects: &mut impl PriorityEffects,
) -> Result<(), Error> {
    if runtime.failed || state.binding() != &runtime.binding {
        return Err(Error::Unsupported("AI begin current owner"));
    }
    state
        .retained
        .players
        .iter()
        .find(|p| p.requested_owner == owner)
        .and_then(|p| p.ai_control.as_ref())
        .ok_or(Error::Unsupported("current AI Control Manager"))?;
    let result = (|| {
        change_depth(runtime, 1)?;
        set_processing(state, owner, false)?;
        let requested = effects.priority_requested(state, runtime, owner, -1)?;
        set_processing(state, owner, requested)?;
        let control = state
            .retained
            .players
            .iter()
            .find(|p| p.requested_owner == owner)
            .and_then(|p| p.ai_control.as_ref())
            .ok_or(Error::Unsupported(
                "current AI Control Manager after priority",
            ))?;
        if !control.controls.instances.is_empty() {
            return Err(Error::Unsupported(
                "current Control Instance validity/destruction fields",
            ));
        }
        change_depth(runtime, -1)
    })();
    if result.is_err() {
        runtime.failed = true;
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        loaded_state::game_update::{AutoplayInputs, Inputs},
        player_ai_control::{ControlManager, RecycledInstances},
    };
    fn fixture() -> (LoadedState, Runtime, i32) {
        let mut state = crate::loaded_state::tests::state();
        let owner = state.retained.players[0].requested_owner;
        state.retained.players[0].ai_control = Some(ControlManager {
            player_id: owner,
            priority: -1,
            controls: RecycledInstances {
                capacity: 0,
                instances: vec![],
                current_id_bits: 0,
                first_free: u32::MAX,
                last_free: u32::MAX,
                free_indices: vec![],
            },
            has_out_of_turn_processing: true,
            running_as_human: false,
            contracts: vec![],
        });
        let runtime = Runtime::new(
            &state,
            Inputs {
                autoplay: AutoplayInputs {
                    end_turn_signaled: None,
                    has_pending_state_change: None,
                    pending_state: None,
                    active: None,
                },
                needs_update: None,
                auto_player_game: None,
                local_player: None,
            },
        );
        (state, runtime, owner)
    }
    struct Signal {
        depth: i32,
        fail: bool,
    }
    impl PriorityEffects for Signal {
        fn priority_requested(
            &mut self,
            state: &mut LoadedState,
            runtime: &mut Runtime,
            owner: i32,
            priority: i32,
        ) -> Result<bool, Error> {
            assert_eq!(priority, -1);
            assert_eq!(processing_depth(runtime), Some(self.depth));
            assert!(
                !state
                    .retained
                    .players
                    .iter()
                    .find(|p| p.requested_owner == owner)
                    .unwrap()
                    .ai_control
                    .as_ref()
                    .unwrap()
                    .has_out_of_turn_processing
            );
            if self.fail {
                Err(Error::Unsupported("actual priority leaf"))
            } else {
                Ok(true)
            }
        }
    }
    #[test]
    fn depth_and_flag_writes_bracket_actual_priority_signal() {
        let (mut state, mut runtime, owner) = fixture();
        install_processing_depth(&state, &mut runtime, i32::MAX).unwrap();
        begin_turn(
            &mut state,
            &mut runtime,
            owner,
            &mut Signal {
                depth: i32::MIN,
                fail: false,
            },
        )
        .unwrap();
        assert_eq!(processing_depth(&runtime), Some(i32::MAX));
        assert!(
            state.retained.players[0]
                .ai_control
                .as_ref()
                .unwrap()
                .has_out_of_turn_processing
        );
    }
    #[test]
    fn unsupported_callback_retains_real_prefix_and_prevents_replay() {
        let (mut state, mut runtime, owner) = fixture();
        install_processing_depth(&state, &mut runtime, 4).unwrap();
        assert!(begin_turn(
            &mut state,
            &mut runtime,
            owner,
            &mut Signal {
                depth: 5,
                fail: true
            }
        )
        .is_err());
        assert_eq!(processing_depth(&runtime), Some(5));
        assert!(
            !state.retained.players[0]
                .ai_control
                .as_ref()
                .unwrap()
                .has_out_of_turn_processing
        );
        assert!(runtime.failed());
        assert!(begin_turn(
            &mut state,
            &mut runtime,
            owner,
            &mut Signal {
                depth: 6,
                fail: false
            }
        )
        .is_err());
        assert_eq!(processing_depth(&runtime), Some(5));
    }
    #[test]
    fn unavailable_processing_depth_does_not_default_to_zero() {
        let (mut state, mut runtime, owner) = fixture();
        assert!(begin_turn(
            &mut state,
            &mut runtime,
            owner,
            &mut Signal {
                depth: 1,
                fail: false
            }
        )
        .is_err());
        assert_eq!(processing_depth(&runtime), None);
        assert!(
            state.retained.players[0]
                .ai_control
                .as_ref()
                .unwrap()
                .has_out_of_turn_processing
        );
    }
}
