//! Current Processor.Update footer for the exclusive synchronous executor.
//! Native worker joins have no outstanding gameplay job in this context. A
//! repeated pass selects a fresh stateful player vector without replaying the
//! Game.Update prefix or running its final Autoplay/diplomacy tail.
use super::{core, missing, Context, Error, LoadedState, Services};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use = "a repeated processor pass remains inside the same Game.Update"]
pub enum ProcessorPass {
    ContinuePlayerUpdates,
    Returned,
}
impl<S: Services> Context<'_, S> {
    /// 2a0d274..2a0d3cd: after actual EndTurn the changed-phase result forces
    /// the current Update to return. BeginTurn belongs to a later Update.
    pub fn finish_round_update(
        &mut self,
        pending_diplomacy_tail: impl FnOnce(&mut LoadedState, &mut S) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let progress = self.runtime.player_progress();
        if self.runtime.failed
            || self.runtime.player_frame.is_some()
            || !matches!(
                self.runtime.pending,
                Some(core::Continuation::PlayerUpdates(_))
            )
            || progress.completed != progress.selected
            || self.state.retained.core.player_processor.active_count != 0
            || !self.state.retained.core.game_processor.turn_complete
            || !self.state.retained.core.game_processor.advance_turn
        {
            return Err(Error::Unsupported("Game.Update unfinished round end"));
        }
        let result = (|| {
            core::update_autoplay(self)?;
            pending_diplomacy_tail(self.state, &mut self.services)
        })();
        if result.is_err() {
            self.runtime.failed = true;
            return result;
        }
        self.runtime.pending = None;
        self.runtime.player_cursor = 0;
        self.runtime.pass_has_human = false;
        self.runtime.pass_processed_ai = false;
        self.runtime.deactivation_started = false;
        self.runtime.processor_pass_iterations = 0;
        Ok(())
    }
    pub fn finish_exclusive_processor_pass(
        &mut self,
        mut any_multiplayer: impl FnMut(&mut LoadedState, &mut S) -> Result<bool, Error>,
        mut blocking_diplomacy: impl FnMut(&mut LoadedState, &mut S, i32) -> Result<bool, Error>,
        pending_diplomacy_tail: impl FnOnce(&mut LoadedState, &mut S) -> Result<(), Error>,
    ) -> Result<ProcessorPass, Error> {
        let progress = self.runtime.player_progress();
        if self.runtime.failed
            || self.runtime.player_frame.is_some()
            || !matches!(
                self.runtime.pending,
                Some(core::Continuation::PlayerUpdates(_))
            )
            || progress.completed != progress.selected
            || self.state.retained.core.player_processor.active_count == 0
        {
            return Err(Error::Unsupported(
                "Game.Update unfinished processor pass/round end",
            ));
        }
        let result = (|| {
            // Native WaitForJobs precedes these reads. Supported services have
            // executed all gameplay callbacks synchronously before this point.
            let mut stop = progress.has_human;
            if !stop && !any_multiplayer(self.state, &mut self.services)? {
                let (list, end) = self.services.alive_begin(self.state)?;
                for i in 0..end {
                    let owner = self.services.alive_at(self.state, list, i)?;
                    if self.state.turn_active(owner).map_err(missing)?
                        && blocking_diplomacy(self.state, &mut self.services, owner)?
                    {
                        stop = true;
                        break;
                    }
                }
            }
            if !stop {
                let previous = self.runtime.processor_pass_iterations;
                self.runtime.processor_pass_iterations = previous.wrapping_add(1);
                if previous <= 98 {
                    let ids = core::players_to_process(self)?;
                    self.runtime.pending = Some(core::Continuation::PlayerUpdates(ids));
                    self.runtime.player_cursor = 0;
                    self.runtime.pass_has_human = false;
                    // processed_ai accumulates across the SAME Update.
                    self.runtime.deactivation_started = false;
                    return Ok(ProcessorPass::ContinuePlayerUpdates);
                }
            }
            core::update_autoplay(self)?;
            pending_diplomacy_tail(self.state, &mut self.services)?;
            self.runtime.pending = None;
            self.runtime.player_cursor = 0;
            self.runtime.pass_has_human = false;
            self.runtime.pass_processed_ai = false;
            self.runtime.deactivation_started = false;
            self.runtime.processor_pass_iterations = 0;
            Ok(ProcessorPass::Returned)
        })();
        if result.is_err() {
            self.runtime.failed = true;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loaded_state::game_update::{AutoplayInputs, Runtime};
    struct Probe {
        owner: i32,
        alive_reads: usize,
    }
    impl Services for Probe {
        type Alive = ();
        fn alive_begin(&mut self, _: &LoadedState) -> Result<((), usize), Error> {
            self.alive_reads += 1;
            Ok(((), 1))
        }
        fn alive_at(&mut self, _: &LoadedState, _: (), _: usize) -> Result<i32, Error> {
            Ok(self.owner)
        }
        fn unavailable_units(&mut self, _: &mut LoadedState, _: i32) -> Result<i32, Error> {
            Err(Error::Unsupported("unreached count"))
        }
        fn notify_turn_request(&mut self, _: &mut LoadedState, _: u32) -> Result<(), Error> {
            Err(Error::Unsupported("unreached notify"))
        }
        fn autoplay_transition(
            &mut self,
            _: &mut LoadedState,
            _: &mut Runtime,
            _: bool,
        ) -> Result<(), Error> {
            Err(Error::Unsupported("unreached autoplay transition"))
        }
    }
    fn fixture() -> (LoadedState, Runtime, Probe) {
        let (mut state, mut runtime, owner) =
            crate::loaded_state::tests::current_update_player_fixture();
        state.controls.as_mut().unwrap().players[0].slot_status = 2;
        state.retained.core.player_processor.active_count = 1;
        runtime.pending = Some(core::Continuation::PlayerUpdates(vec![owner]));
        runtime.player_cursor = 1;
        runtime.pass_has_human = false;
        runtime.pass_processed_ai = true;
        runtime.inputs.needs_update = Some(true);
        runtime.inputs.autoplay = AutoplayInputs {
            end_turn_signaled: Some(true),
            has_pending_state_change: Some(false),
            pending_state: None,
            active: None,
        };
        (
            state,
            runtime,
            Probe {
                owner,
                alive_reads: 0,
            },
        )
    }
    #[test]
    fn round_end_returns_without_replaying_prefix_or_beginning_next_turn() {
        let (mut s, mut r, p) = fixture();
        s.retained.core.player_processor.active_count = 0;
        s.retained.core.game_processor.turn_complete = true;
        s.retained.core.game_processor.advance_turn = true;
        let turn = s.retained.core.current_turn;
        let mut c = Context::new(&mut s, &mut r, p).unwrap();
        c.finish_round_update(|state, _| {
            assert_eq!(state.retained.core.current_turn, turn);
            assert!(state.retained.core.game_processor.advance_turn);
            Ok(())
        })
        .unwrap();
        assert!(c.runtime.pending.is_none());
        assert_eq!(c.runtime.inputs.needs_update, Some(true));
        assert_eq!(c.services.alive_reads, 0);
        assert_eq!(c.state.retained.core.current_turn, turn);
        assert!(c.state.retained.core.game_processor.turn_complete);
        assert!(c.state.retained.core.game_processor.advance_turn);
    }
    #[test]
    fn npc_backedge_preserves_update_prefix_and_skips_both_final_tails() {
        let (mut s, mut r, p) = fixture();
        let owner = p.owner;
        let mut c = Context::new(&mut s, &mut r, p).unwrap();
        let result = c.finish_exclusive_processor_pass(
            |_, _| Ok(true),
            |_, _, _| panic!("multiplayer bypass"),
            |_, _| panic!("not returning"),
        );
        assert_eq!(result.unwrap(), ProcessorPass::ContinuePlayerUpdates);
        assert_eq!(
            c.runtime.pending,
            Some(core::Continuation::PlayerUpdates(vec![owner]))
        );
        assert_eq!(c.runtime.player_cursor, 0);
        assert!(c.runtime.pass_processed_ai);
        assert_eq!(c.runtime.inputs.needs_update, Some(true));
        assert_eq!(c.runtime.inputs.autoplay.end_turn_signaled, Some(true));
        assert_eq!(c.services.alive_reads, 1);
    }
    #[test]
    fn pending_diplomacy_stops_current_update_and_runs_final_tails() {
        let (mut s, mut r, p) = fixture();
        let mut tail = 0;
        let mut c = Context::new(&mut s, &mut r, p).unwrap();
        assert_eq!(
            c.finish_exclusive_processor_pass(
                |_, _| Ok(false),
                |_, _, _| Ok(true),
                |_, _| {
                    tail += 1;
                    Ok(())
                }
            )
            .unwrap(),
            ProcessorPass::Returned
        );
        assert_eq!(tail, 1);
        assert!(c.runtime.pending.is_none());
        assert_eq!(c.runtime.inputs.autoplay.end_turn_signaled, Some(false));
        assert!(!c.runtime.pass_processed_ai);
    }
    #[test]
    fn human_pass_returns_without_querying_multiplayer_or_active_dialogue() {
        let (mut s, mut r, p) = fixture();
        r.pass_has_human = true;
        let mut tail = 0;
        let mut c = Context::new(&mut s, &mut r, p).unwrap();
        assert_eq!(
            c.finish_exclusive_processor_pass(
                |_, _| panic!("human pass does not query mode"),
                |_, _, _| panic!("human pass does not query active dialogue"),
                |_, _| {
                    tail += 1;
                    Ok(())
                },
            )
            .unwrap(),
            ProcessorPass::Returned
        );
        assert_eq!(tail, 1);
        assert_eq!(c.services.alive_reads, 0);
    }
    #[test]
    fn hundredth_pass_returns_without_selecting_another_player_vector() {
        let (mut s, mut r, p) = fixture();
        r.processor_pass_iterations = 99;
        let mut tail = 0;
        let mut c = Context::new(&mut s, &mut r, p).unwrap();
        assert_eq!(
            c.finish_exclusive_processor_pass(
                |_, _| Ok(true),
                |_, _, _| panic!("multiplayer bypass"),
                |_, _| {
                    tail += 1;
                    Ok(())
                }
            )
            .unwrap(),
            ProcessorPass::Returned
        );
        assert_eq!(tail, 1);
        assert_eq!(c.services.alive_reads, 0);
        assert_eq!(c.runtime.processor_pass_iterations, 0);
    }
}
