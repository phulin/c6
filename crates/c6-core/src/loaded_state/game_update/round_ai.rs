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
/// Nonempty controls use their explicit live callbacks-pointer validity and
/// destroy marks. Wire-only unknown fields fail when their query is reached.
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
        mark_invalid_controllers(state, owner)?;
        super::npc_deactivation::cleanup_controllers(state, owner)?;
        change_depth(runtime, -1)
    })();
    if result.is_err() {
        runtime.failed = true;
    }
    result
}
/// Initialized reset subscribers. Analyzer/job readiness and recommendation
/// caches are external-policy state; their resets perform no gameplay query,
/// dispatch or RNG. Retained scheduler flags are changed on their sole owners.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StockReset {
    City,
    Culture,
    Diplomatic,
    Economic,
    Governors,
    GrandStrategic,
    Military,
    Research,
    Espionage,
    Religious,
}
pub fn stock_reset(state: &mut LoadedState, owner: i32, kind: StockReset) -> Result<bool, Error> {
    let i = state
        .retained
        .players
        .iter()
        .position(|p| p.requested_owner == owner)
        .ok_or(Error::Unsupported("round reset owner"))?;
    state.revision = state
        .revision
        .checked_add(1)
        .ok_or(Error::Unsupported("round reset revision"))?;
    let p = &mut state.retained.players[i];
    match kind {
        StockReset::City => {
            let m = p
                .ai_city_manager
                .as_mut()
                .ok_or(Error::Unsupported("round CityManager"))?;
            for city in &mut m.city_ais.entries {
                city.city_build.done_off_turn.0 = 0;
            }
        }
        StockReset::Culture => {
            p.ai_culture
                .as_mut()
                .ok_or(Error::Unsupported("round CultureAi"))?
                .done_off_turn
                .0 = 0;
        }
        StockReset::Diplomatic => {
            let d = p
                .ai_diplomatic
                .as_mut()
                .ok_or(Error::Unsupported("round Diplomatic"))?;
            d.diplomatic_states.is_updated.0 = 0;
            d.diplo_action_evaluator.is_updated.0 = 0;
            d.done_off_turn.0 = 0;
        }
        StockReset::Governors => {
            p.ai_governors
                .as_mut()
                .ok_or(Error::Unsupported("round Governors"))?
                .done_off_turn = 0;
        }
        StockReset::GrandStrategic => {
            let g = p
                .ai_grand_strategic
                .as_mut()
                .ok_or(Error::Unsupported("round GrandStrategic"))?;
            g.unit_production.has_targets_raw = 0;
            for region in &mut g.settle_cities {
                region.has_positive_eval_raw = 0;
            }
        }
        StockReset::Research => {
            p.ai_research
                .as_mut()
                .ok_or(Error::Unsupported("round Research"))?
                .done_off_turn
                .0 = 0;
        }
        // These leaves only reset unsaved native analysis-job readiness. They
        // neither select a policy nor execute it in the external-policy scope.
        StockReset::Economic
        | StockReset::Military
        | StockReset::Espionage
        | StockReset::Religious => {}
    }
    // Every stock callback's concrete wrapper returns true, including void
    // reset lambdas. CollectorOr must nevertheless visit the remaining ring.
    Ok(true)
}
/// Initial refinement of the unsaved callback pointer validity on the same
/// canonical Control.Instances. Do not reuse an answer for a different ID.
pub fn initialize_controller_validity(
    state: &mut LoadedState,
    binding: &crate::core_player::initialization::WireBinding,
    owner: i32,
    values: &[(u32, bool)],
) -> Result<(), Error> {
    if state.binding() != binding || !state.is_pristine() {
        return Err(Error::Unsupported("initial controller validity binding"));
    }
    let c = state
        .retained
        .players
        .iter_mut()
        .find(|p| p.requested_owner == owner)
        .and_then(|p| p.ai_control.as_mut())
        .ok_or(Error::Unsupported("initial controller validity owner"))?;
    if c.controls.instances.len() != values.len()
        || c.controls
            .instances
            .iter()
            .zip(values)
            .any(|(c, (id, v))| c.id != *id || c.is_valid.is_some_and(|old| old != *v))
    {
        return Err(Error::Unsupported("initial controller validity IDs"));
    }
    for (c, (_, value)) in c.controls.instances.iter_mut().zip(values) {
        c.is_valid = Some(*value);
    }
    Ok(())
}
fn mark_invalid_controllers(state: &mut LoadedState, owner: i32) -> Result<(), Error> {
    let c = state
        .retained
        .players
        .iter_mut()
        .find(|p| p.requested_owner == owner)
        .and_then(|p| p.ai_control.as_mut())
        .ok_or(Error::Unsupported("round controller owner"))?;
    for instance in &mut c.controls.instances {
        let valid = instance
            .is_valid
            .ok_or(Error::Unsupported("current controller callback validity"))?;
        if !valid {
            state.revision = state
                .revision
                .checked_add(1)
                .ok_or(Error::Unsupported("round controller mark revision"))?;
            instance.should_destroy = Some(true);
        }
    }
    Ok(())
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
    fn begin_marks_invalid_controllers_and_recycles_them_before_depth_close() {
        use crate::player_ai_control::Instance;
        let (mut state, mut runtime, owner) = fixture();
        let c = &mut state.retained.players[0]
            .ai_control
            .as_mut()
            .unwrap()
            .controls;
        c.capacity = 2;
        c.current_id_bits = 0x20000;
        c.instances = vec![
            Instance {
                index: 0,
                id: 65536,
                priority: -1,
                recruit_priority: 0,
                should_destroy: Some(false),
                is_valid: Some(true),
            },
            Instance {
                index: 1,
                id: 131073,
                priority: -1,
                recruit_priority: 0,
                should_destroy: Some(false),
                is_valid: Some(false),
            },
        ];
        install_processing_depth(&state, &mut runtime, 3).unwrap();
        begin_turn(
            &mut state,
            &mut runtime,
            owner,
            &mut Signal {
                depth: 4,
                fail: false,
            },
        )
        .unwrap();
        let c = &state.retained.players[0]
            .ai_control
            .as_ref()
            .unwrap()
            .controls;
        assert_eq!(
            c.instances.iter().map(|c| c.id).collect::<Vec<_>>(),
            [65536]
        );
        assert_eq!(c.free_indices, [1]);
        assert_eq!(
            (c.first_free, c.last_free, c.current_id_bits),
            (1, 1, 0x20000)
        );
        assert_eq!(processing_depth(&runtime), Some(3));
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
