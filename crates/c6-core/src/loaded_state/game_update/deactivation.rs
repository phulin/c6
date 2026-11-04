//! Canonical deactivation selection and first SetTurnActive prefix. The returned
//! token retains the required logging/report/automation frontier, not success.
use super::{missing, Context, Services};
use crate::game_update::{deactivation as core, Error};

#[derive(Debug)]
pub struct Continuation {
    selected: Vec<i32>,
    effect: Option<core::Effect<i32>>,
    alive_nonhuman: bool,
    stamp: crate::loaded_state::ActionStamp,
    binding: crate::core_player::initialization::WireBinding,
}

/// Source-qualified stock Platform file logger only. Clock/formatting, channel
/// cache, offset counters and output have no gameplay/RNG effects. Custom log
/// callbacks must execute their actual effects. Reporting remains mandatory.
pub enum LoggingScope {
    StockFileSink,
}

pub fn omit_stock_file_log(
    state: &crate::loaded_state::LoadedState,
    runtime: &mut super::Runtime,
    pending: &mut Continuation,
    _: LoggingScope,
) -> Result<core::Effect<i32>, Error> {
    if runtime.failed
        || !runtime.deactivation_started
        || state.binding() != &runtime.binding
        || state.binding() != &pending.binding
        || state.action_stamp() != pending.stamp
    {
        return Err(Error::Unsupported(
            "SetTurnActive logging continuation binding",
        ));
    }
    let Some(core::Effect::TurnActiveLog {
        player,
        active: false,
    }) = pending.effect
    else {
        return Err(Error::Unsupported(
            "SetTurnActive logging continuation ordering",
        ));
    };
    let result = (|| {
        // Native GetID -> current local player -> fresh GetID, after logging.
        state.world.player(player).map_err(missing)?;
        let local = super::required(
            runtime.inputs.local_player,
            "SetTurnActive current local player",
        )?;
        let event = if player == local {
            0x6e63_a083
        } else {
            0xf167_21cf
        };
        state.world.player(player).map_err(missing)?;
        let effect = core::Effect::TurnActiveReport { player, event };
        pending.effect = Some(effect);
        Ok(effect)
    })();
    if result.is_err() {
        runtime.failed = true;
    }
    result
}
impl Continuation {
    pub fn selected(&self) -> &[i32] {
        &self.selected
    }
    pub fn effect(&self) -> Option<core::Effect<i32>> {
        self.effect
    }
    /// Captured before DoTurnDeactivate, committed to the outer accumulator
    /// only when its required report/automation tail has actually completed.
    pub fn alive_nonhuman(&self) -> bool {
        self.alive_nonhuman
    }
    /// An empty vector still requires the active-count check. ActivateNextPlayer
    /// is conditional on zero active players; it is never implied by emptiness.
    pub fn requires_post_deactivation_pass(&self) -> bool {
        self.selected.is_empty()
    }
}

/// Native selection is reached only after all selected Player.Update calls.
/// Any failure keeps its exact scalar prefix and forbids replay on this runtime.
pub fn begin<S: Services>(c: &mut Context<'_, S>) -> Result<Continuation, Error> {
    let progress = c.runtime.player_progress();
    if c.runtime.failed
        || c.runtime.deactivation_started
        || c.runtime.player_frame.is_some()
        || !matches!(
            c.runtime.pending,
            Some(crate::game_update::Continuation::PlayerUpdates(_))
        )
        || progress.completed != progress.selected
    {
        return Err(Error::Unsupported(
            "Game.Update deactivation continuation ordering",
        ));
    }
    c.runtime.deactivation_started = true;
    let result = (|| {
        let selected = core::select(c)?;
        let mut alive_nonhuman = false;
        let effect = if let Some(&id) = selected.first() {
            let p = core::World::player(c, id)?;
            if c.state.player_alive(p).map_err(missing)? {
                alive_nonhuman = !core::World::human(c, p)?;
            }
            Some(core::begin(c, p)?)
        } else {
            None
        };
        Ok(Continuation {
            selected,
            effect,
            alive_nonhuman,
            stamp: c.state.action_stamp(),
            binding: c.state.binding().clone(),
        })
    })();
    if result.is_err() {
        c.runtime.failed = true;
    }
    result
}
impl<S: Services> core::World for Context<'_, S> {
    type Player = i32;
    fn turn_order(&mut self, index: usize) -> Result<i32, Error> {
        self.state
            .retained
            .core
            .game_processor
            .turn_order
            .get(index)
            .copied()
            .ok_or(Error::Unsupported("turn-order physical slot"))
    }
    fn valid(&mut self, id: i32) -> Result<bool, Error> {
        self.state.world.player_is_valid(id).map_err(missing)
    }
    fn player(&mut self, id: i32) -> Result<i32, Error> {
        self.state.world.player(id).map_err(missing)?;
        Ok(id)
    }
    fn active(&mut self, p: i32) -> Result<bool, Error> {
        self.state.turn_active(p).map_err(missing)
    }
    fn id(&mut self, p: i32) -> Result<i32, Error> {
        core::World::player(self, p)
    }
    fn complete(&mut self, p: i32) -> Result<bool, Error> {
        let id = core::World::id(self, p)?;
        Ok(self.state.retained.core.player_processor.player_complete[Self::index(id)?])
    }
    fn game_complete(&mut self, p: i32) -> Result<bool, Error> {
        let id = core::World::id(self, p)?;
        Ok(self.state.retained.core.player_processor.game_complete[Self::index(id)?])
    }
    fn set_auto_player_game(&mut self, value: bool) -> Result<(), Error> {
        self.revision()?;
        self.runtime.inputs.auto_player_game = Some(value);
        Ok(())
    }
    fn human(&mut self, p: i32) -> Result<bool, Error> {
        <Self as crate::game_update::World>::human(self, p)
    }
    fn ai_done(&mut self, _: i32) -> Result<bool, Error> {
        Err(Error::Unsupported(
            "deactivation AI.Control.IsDoneTurn current gameplay callbacks",
        ))
    }
    fn validate_pending(&mut self, _: i32) -> Result<(), Error> {
        Err(Error::Unsupported("deactivation Diplomacy.ValidatePending"))
    }
    fn pending(&mut self, _: i32, _: core::Pending) -> Result<bool, Error> {
        Err(Error::Unsupported(
            "deactivation current diplomacy pending getters",
        ))
    }
    fn multiplayer(&mut self) -> Result<bool, Error> {
        Err(Error::Unsupported(
            "deactivation current configuration multiplayer",
        ))
    }
    fn unit_counts(&mut self, p: i32) -> Result<(), Error> {
        self.services.unavailable_units(self.state, p).map(|_| ())
    }
    fn active_count(&mut self) -> Result<u32, Error> {
        Ok(self.state.retained.core.player_processor.active_count)
    }
    fn set_active_count(&mut self, value: u32) -> Result<(), Error> {
        self.revision()?;
        self.state.retained.core.player_processor.active_count = value;
        Ok(())
    }
    fn assign_active(&mut self, p: i32, value: bool) -> Result<(), Error> {
        let index = self
            .state
            .controls
            .as_ref()
            .and_then(|c| c.players.iter().position(|v| v.owner == p))
            .ok_or(Error::Unsupported("SetTurnActive current player controls"))?;
        // CopyAssignmentDo2b3d480 only inserts the variable in an archive dirty
        // tree before the bool store. Logical revision retains dirty semantics;
        // native tree allocations/order have no gameplay/RNG consumers here.
        self.revision()?;
        self.state.controls.as_mut().expect("checked").players[index].turn_active = Some(value);
        Ok(())
    }
    fn clear_automation_processed(&mut self, id: i32) -> Result<(), Error> {
        let index = Self::index(id)?;
        self.revision()?;
        self.state
            .retained
            .core
            .player_processor
            .automation_processed[index] = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loaded_state::tests::{current_update_player_fixture, UpdateServices};
    fn fixture() -> (crate::loaded_state::LoadedState, super::super::Runtime, i32) {
        let (mut state, mut runtime, owner) = current_update_player_fixture();
        // Prior Player.Update and its required blocking callback completed.
        runtime.player_cursor = runtime.player_progress().selected;
        state.retained.core.game_processor.turn_order.fill(-1);
        state.retained.core.game_processor.turn_order[5] = owner;
        state.retained.core.player_processor.player_complete[owner as usize] = true;
        state.retained.core.player_processor.automation_processed[owner as usize] = true;
        state.retained.core.player_processor.active_count = 1;
        runtime.inputs.auto_player_game = Some(true);
        (state, runtime, owner)
    }
    #[test]
    fn completed_current_player_clears_auto_game_then_inactive_prefix_before_log() {
        let (mut state, mut runtime, owner) = fixture();
        // Native turn order is consumed physically; it is never deduplicated.
        state.retained.core.game_processor.turn_order[63] = owner;
        let pending =
            begin(&mut Context::new(&mut state, &mut runtime, UpdateServices).unwrap()).unwrap();
        assert_eq!(pending.selected(), &[owner, owner]);
        assert_eq!(
            pending.effect(),
            Some(core::Effect::TurnActiveLog {
                player: owner,
                active: false
            })
        );
        assert_eq!(runtime.inputs.auto_player_game, Some(false));
        assert_eq!(state.retained.core.player_processor.active_count, 0);
        assert!(!state.turn_active(owner).unwrap());
        assert!(!state.retained.core.player_processor.automation_processed[owner as usize]);
        assert!(
            begin(&mut Context::new(&mut state, &mut runtime, UpdateServices).unwrap()).is_err()
        );
    }
    #[test]
    fn inactive_entries_skip_completion_and_require_next_phase_without_assignment() {
        let (mut state, mut runtime, owner) = fixture();
        state
            .controls
            .as_mut()
            .unwrap()
            .players
            .iter_mut()
            .find(|p| p.owner == owner)
            .unwrap()
            .turn_active = Some(false);
        let revision = state.revision();
        let pending =
            begin(&mut Context::new(&mut state, &mut runtime, UpdateServices).unwrap()).unwrap();
        assert!(pending.requires_post_deactivation_pass());
        assert_eq!(pending.effect(), None);
        assert_eq!(state.revision(), revision);
        assert_eq!(runtime.inputs.auto_player_game, Some(true));
    }
    #[test]
    fn exhausted_revision_retains_active_count_prefix_before_archived_bool_assignment() {
        let (mut state, mut runtime, owner) = fixture();
        state.revision = u64::MAX - 2;
        state.retained.core.player_processor.active_count = 0;
        assert!(
            begin(&mut Context::new(&mut state, &mut runtime, UpdateServices).unwrap()).is_err()
        );
        assert_eq!(runtime.inputs.auto_player_game, Some(false));
        assert_eq!(state.retained.core.player_processor.active_count, u32::MAX);
        assert!(state.turn_active(owner).unwrap());
        assert!(state.retained.core.player_processor.automation_processed[owner as usize]);
        assert!(runtime.failed());
    }
    #[test]
    fn incomplete_human_remains_active_and_does_not_enter_ai_or_diplomacy() {
        let (mut state, mut runtime, owner) = fixture();
        state.retained.core.player_processor.player_complete[owner as usize] = false;
        state.retained.core.player_processor.game_complete[owner as usize] = false;
        let pending =
            begin(&mut Context::new(&mut state, &mut runtime, UpdateServices).unwrap()).unwrap();
        assert!(pending.requires_post_deactivation_pass());
        assert!(state.turn_active(owner).unwrap());
        assert_eq!(runtime.inputs.auto_player_game, Some(true));
    }
    #[test]
    fn incomplete_nonhuman_requires_actual_ai_done_callbacks_without_role_change() {
        let (mut state, mut runtime, owner) = fixture();
        state.retained.core.player_processor.player_complete[owner as usize] = false;
        state.retained.core.player_processor.game_complete[owner as usize] = false;
        state
            .controls
            .as_mut()
            .unwrap()
            .players
            .iter_mut()
            .find(|p| p.owner == owner)
            .unwrap()
            .slot_status = 4;
        let result = begin(&mut Context::new(&mut state, &mut runtime, UpdateServices).unwrap());
        assert!(matches!(
            result,
            Err(Error::Unsupported(
                "deactivation AI.Control.IsDoneTurn current gameplay callbacks"
            ))
        ));
        assert_eq!(state.player_slot_status(owner).unwrap(), 4);
        assert!(state.turn_active(owner).unwrap());
        assert_eq!(runtime.inputs.auto_player_game, Some(true));
    }
    #[test]
    fn source_qualified_log_omission_requires_current_local_player_and_preserves_report() {
        let (mut state, mut runtime, owner) = fixture();
        runtime.inputs.local_player = Some(owner);
        let mut pending =
            begin(&mut Context::new(&mut state, &mut runtime, UpdateServices).unwrap()).unwrap();
        let stamp = state.action_stamp();
        assert_eq!(
            omit_stock_file_log(
                &state,
                &mut runtime,
                &mut pending,
                LoggingScope::StockFileSink
            ),
            Ok(core::Effect::TurnActiveReport {
                player: owner,
                event: 0x6e63_a083
            })
        );
        assert_eq!(state.action_stamp(), stamp);
        assert!(omit_stock_file_log(
            &state,
            &mut runtime,
            &mut pending,
            LoggingScope::StockFileSink
        )
        .is_err());
    }
    #[test]
    fn log_omission_never_defaults_unknown_local_player() {
        let (mut state, mut runtime, _) = fixture();
        runtime.inputs.local_player = None;
        let mut pending =
            begin(&mut Context::new(&mut state, &mut runtime, UpdateServices).unwrap()).unwrap();
        assert!(omit_stock_file_log(
            &state,
            &mut runtime,
            &mut pending,
            LoggingScope::StockFileSink
        )
        .is_err());
        assert!(matches!(
            pending.effect(),
            Some(core::Effect::TurnActiveLog { .. })
        ));
        assert!(runtime.failed());
    }
}
