//! Linux XP2 round processor mutations. Required synchronous effects are
//! supplied at their exact call sites; failure retains the executed prefix.
//! These functions do not certify a completed Game.Update or external decision.
use super::{Error, LoadedState, Runtime};

fn changed(state: &mut LoadedState) -> Result<(), Error> {
    state.revision = state
        .revision
        .checked_add(1)
        .ok_or(Error::Unsupported("round revision capacity"))?;
    Ok(())
}

/// Final Player.TurnManager.DoEndTurn leaf, after Units and Cities complete.
/// Culture.DoEndTurn2a736c0 clears only the civic-completed boolean (FAV+458,
/// value+468; getter27cb5b0). The recorded civic hash and government flag stay.
pub fn end_player_culture(state: &mut LoadedState, owner: i32) -> Result<(), Error> {
    let index = state
        .retained
        .players
        .iter()
        .position(|p| p.requested_owner == owner)
        .ok_or(Error::Unsupported("current end-turn Culture owner"))?;
    changed(state)?;
    state.retained.players[index].culture.civic_completed_this_turn = false;
    Ok(())
}

/// Mandatory effects outside the processor scalar bodies. Implementations must
/// execute the reached gameplay/RNG consumers or return Unsupported. No native
/// policy selection is authorized by a successful processor transition.
pub trait TurnEffects {
    fn report(&mut self, state: &mut LoadedState, event: u32, value: i32) -> Result<(), Error>;
    /// 2ba1750 -> 201e660, including GlobalAi.ProcessAi and priority subscribers.
    fn player_begin(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        owner: i32,
    ) -> Result<(), Error>;
    /// 2ba1770: units, cities, culture, in that order.
    fn player_end(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Error>;
    /// 2b5aaa0 onward: reset accumulated AI timing, timing report, then broadcast
    /// GameTurnComplete for the actual current local player. Do not execute it inline.
    fn player_end_tail(&mut self, state: &mut LoadedState) -> Result<(), Error>;
    /// Actual XP2 game virtual +30, including Map components before managers.
    fn world_begin(&mut self, state: &mut LoadedState) -> Result<(), Error>;
    /// Game signals +708 (begin) or +750 (end), with current turn reread.
    fn turn_signal(&mut self, state: &mut LoadedState, begin: bool, turn: i32)
        -> Result<(), Error>;
    fn activate_at_turn_start(&mut self, state: &mut LoadedState) -> Result<(), Error>;
    fn autoplay_end(&mut self, state: &mut LoadedState, runtime: &mut Runtime)
        -> Result<(), Error>;
}

#[derive(Debug, Eq, PartialEq)]
#[must_use = "post-BeginTurn selection and Game.Update tail remain mandatory"]
pub struct Begun {
    stamp: crate::loaded_state::ActionStamp,
}

/// 2a0c460, 2b5a8c0, 2a54860. Slot resets interleave each alive player's
/// BeginTurn; a failing slot never resets later slots or increments the turn.
/// Stock file logging is omitted at this headless boundary.
pub fn begin_turn(
    state: &mut LoadedState,
    runtime: &mut Runtime,
    effects: &mut impl TurnEffects,
) -> Result<Begun, Error> {
    if runtime.failed
        || runtime.round_begin_ready
        || state.binding() != &runtime.binding
        || !matches!(
            runtime.pending,
            Some(crate::game_update::Continuation::BeginTurn)
        )
    {
        return Err(Error::Unsupported("round BeginTurn continuation"));
    }
    let result = (|| {
        let reported = state
            .retained
            .core
            .current_turn
            .wrapping_add(i32::from(state.retained.core.game_processor.advance_turn));
        effects.report(state, 0x1e39e493, reported)?;
        if state.controls.is_none() {
            return Err(Error::Unsupported(
                "round current processingAutomation controls",
            ));
        }
        changed(state)?;
        state.retained.core.player_processor.human_first_activated = false;
        state.retained.core.player_processor.active_count = 0;
        state
            .controls
            .as_mut()
            .expect("checked")
            .processing_automation = Some(-1);
        for owner in 0..64 {
            changed(state)?;
            reset_slot(&mut state.retained.core.player_processor, owner as usize);
            if state.world.player_is_valid(owner).map_err(super::missing)? {
                state
                    .store_player_end_turn_blocking(owner, 0)
                    .map_err(super::missing)?;
                if state.player_alive(owner).map_err(super::missing)? {
                    effects.player_begin(state, runtime, owner)?;
                }
            }
        }
        changed(state)?;
        state.retained.core.phase_processor.current_phase = -1;
        if state.retained.core.game_processor.advance_turn {
            changed(state)?;
            state.retained.core.current_turn = state.retained.core.current_turn.wrapping_add(1);
        }
        effects.world_begin(state)?;
        effects.report(state, 0x21535587, state.retained.core.current_turn)?;
        effects.turn_signal(state, true, state.retained.core.current_turn)?;
        effects.activate_at_turn_start(state)?;
        changed(state)?;
        state.retained.core.game_processor.turn_complete = false;
        state.retained.core.game_processor.advance_turn = false;
        state.retained.core.game_processor.new_game = false;
        // The Processor.Update caller must select/check the actual new player
        // vector before ordinary player passes. Do not run prepare() again.
        runtime.round_begin_ready = true;
        Ok(Begun {
            stamp: state.action_stamp(),
        })
    })();
    if result.is_err() {
        runtime.failed = true;
    }
    result
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use = "the Game.Instance.Update diplomacy/cache tail remains mandatory"]
pub enum AfterBegin {
    HumanExit,
    PlayerUpdates,
}

/// 2a0cbe7..2a0cc90: the first vector is used only to test human early exit.
/// Without a live selected human native obtains a SECOND stateful vector.
/// Human exit still runs the final Autoplay.Update before returning.
pub fn after_begin_turn<S: super::Services>(
    context: &mut super::Context<'_, S>,
    begun: Begun,
) -> Result<AfterBegin, Error> {
    if context.runtime.failed
        || !context.runtime.round_begin_ready
        || context.state.action_stamp() != begun.stamp
        || !matches!(
            context.runtime.pending,
            Some(crate::game_update::Continuation::BeginTurn)
        )
    {
        return Err(Error::Unsupported("post-BeginTurn continuation state"));
    }
    let result = (|| {
        context.runtime.round_begin_ready = false;
        let ids = crate::game_update::players_to_process(context)?;
        let mut human = false;
        for owner in ids {
            // Do not short circuit later entries once one live human is found.
            if context
                .state
                .player_slot_status(owner)
                .map_err(super::missing)?
                == 3
                && context.state.player_alive(owner).map_err(super::missing)?
            {
                human = true;
            }
        }
        if human {
            crate::game_update::update_autoplay(context)?;
            context.runtime.pending = None;
            context.runtime.player_cursor = 0;
            context.runtime.pass_has_human = false;
            context.runtime.deactivation_started = false;
            Ok(AfterBegin::HumanExit)
        } else {
            let ids = crate::game_update::players_to_process(context)?;
            context.runtime.pending = Some(crate::game_update::Continuation::PlayerUpdates(ids));
            context.runtime.player_cursor = 0;
            context.runtime.pass_has_human = false;
            context.runtime.deactivation_started = false;
            Ok(AfterBegin::PlayerUpdates)
        }
    })();
    if result.is_err() {
        context.runtime.failed = true;
    }
    result
}

fn reset_slot(p: &mut crate::core_stream::PlayerProcessor, i: usize) {
    p.activation_event[i] = true;
    p.first_activation[i] = true;
    p.activation_needed[i] = true;
    p.automation_processed[i] = false;
    p.player_complete[i] = false;
    p.game_complete[i] = false;
    p.updates_this_turn[i] = 0;
    p.ai_updates_this_turn[i] = 0;
    // UpdatesThisPhase belongs to BeginPhase, not BeginTurn.
}

/// 2a0c750 -> 2b5aa00. Caller must establish the native no-active-player,
/// completed-automation branch before entering this internal function.
pub fn end_turn(
    state: &mut LoadedState,
    runtime: &mut Runtime,
    effects: &mut impl TurnEffects,
) -> Result<(), Error> {
    if runtime.failed || state.binding() != &runtime.binding {
        return Err(Error::Unsupported("round EndTurn runtime"));
    }
    let result = (|| {
        changed(state)?;
        state.retained.core.game_processor.advance_turn = true;
        state.retained.core.game_processor.turn_complete = true;
        for owner in 0..64 {
            if state.world.player_is_valid(owner).map_err(super::missing)?
                && state.player_alive(owner).map_err(super::missing)?
            {
                effects.player_end(state, owner)?;
            }
        }
        effects.player_end_tail(state)?;
        changed(state)?;
        state.retained.core.phase_processor.current_phase = -1;
        effects.report(state, 0x66ade3b3, state.retained.core.current_turn)?;
        effects.turn_signal(state, false, state.retained.core.current_turn)?;
        effects.autoplay_end(state, runtime)
    })();
    if result.is_err() {
        runtime.failed = true;
    }
    result
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhaseAdvance {
    AutomationActivated,
    BeganPhase,
    Exhausted,
}

/// 2b5c250(true): completed game-turn slots do not block the round end even
/// when their automation flag is false. Iterate the actual alive order.
pub fn unprocessed_automation(state: &LoadedState, alive: &[i32]) -> Result<i32, Error> {
    let mut count = 0i32;
    for &owner in alive {
        let index = usize::try_from(owner).ok().filter(|&i| i < 64)
            .ok_or(Error::Unsupported("round automation player slot"))?;
        let players = &state.retained.core.player_processor;
        if !players.automation_processed[index] && !players.game_complete[index] {
            count = count.wrapping_add(1);
        }
    }
    Ok(count)
}

pub trait PhaseEffects {
    fn allow_tactical(&mut self, state: &LoadedState) -> Result<bool, Error>;
    fn automation(&mut self, state: &mut LoadedState) -> Result<(), Error>;
    /// Game signal+738, followed by Player.EndPhase report93dd170f.
    fn end_phase(&mut self, state: &mut LoadedState) -> Result<(), Error>;
    fn current_segment(&mut self, state: &LoadedState) -> Result<u32, Error>;
    fn selected_count(&mut self) -> Result<usize, Error>;
    fn segment_at(&mut self, index: usize) -> Result<u32, Error>;
    fn segment_disabled(&mut self, state: &LoadedState, hash: u32) -> Result<bool, Error>;
    fn time_limit(&mut self, state: &mut LoadedState) -> Result<u32, Error>;
    /// Player.BeginPhase, Game signal+720, simultaneous activation in turn order.
    fn begin_phase(&mut self, state: &mut LoadedState, last_segment: i32) -> Result<(), Error>;
}

/// 2a54880. `phase_changed` is the actual bool& out parameter, including the
/// true-on-exhaustion branch; exhausted leaves current_phase unchanged.
pub fn advance_phase(
    state: &mut LoadedState,
    phase_changed: &mut bool,
    effects: &mut impl PhaseEffects,
) -> Result<PhaseAdvance, Error> {
    if state.retained.core.phase_processor.current_phase != -1 {
        if effects.allow_tactical(state)? {
            effects.automation(state)?;
            if state.retained.core.player_processor.active_count != 0 {
                *phase_changed = false;
                return Ok(PhaseAdvance::AutomationActivated);
            }
        }
        if state.retained.core.phase_processor.current_phase != -1 {
            let previous = effects.current_segment(state)? as i32;
            changed(state)?;
            state.retained.core.phase_processor.last_valid_segment = previous;
            effects.end_phase(state)?;
        }
    }
    *phase_changed = true;
    let mut next = state
        .retained
        .core
        .phase_processor
        .current_phase
        .wrapping_add(1);
    let mut index = 0usize;
    while index < effects.selected_count()? {
        if index as i32 == next {
            let hash = effects.segment_at(index)?;
            // Unknown type indices are disabled natively (2a54f90).
            let disabled = effects.segment_disabled(state, hash)?;
            if !disabled {
                break;
            }
            next = next.wrapping_add(1);
        }
        index += 1;
    }
    if index == effects.selected_count()? {
        return Ok(PhaseAdvance::Exhausted);
    }
    changed(state)?;
    state.retained.core.phase_processor.current_phase = next;
    let time = effects.time_limit(state)?;
    changed(state)?;
    state.retained.core.phase_processor.time_limit = time;
    effects.begin_phase(
        state,
        state.retained.core.phase_processor.last_valid_segment,
    )?;
    Ok(PhaseAdvance::BeganPhase)
}

/// Player.BeginPhase2b5ab10. `alive` is the actual current native GetAlive
/// vector order. There are no callbacks until the final phase report, so the
/// bounded traversal cannot replace its storage while these scalar writes run.
pub fn begin_player_phase(
    state: &mut LoadedState,
    alive: &[i32],
    report: impl FnOnce(&mut LoadedState, u32, i32) -> Result<(), Error>,
) -> Result<(), Error> {
    for &owner in alive {
        if state.player_slot_status(owner).map_err(super::missing)? == 3 {
            let index = usize::try_from(owner)
                .ok()
                .filter(|&i| i < 64)
                .ok_or(Error::Unsupported("round BeginPhase player slot"))?;
            changed(state)?;
            state.retained.core.player_processor.activation_needed[index] = true;
        }
    }
    for index in 0..64 {
        changed(state)?;
        state.retained.core.player_processor.updates_this_phase[index] = 0;
    }
    report(
        state,
        0xd3f37746,
        state.retained.core.phase_processor.current_phase,
    )
}

/// Actual XP2 DoBeginTurn2d77fa0 order after Map.DoTurn2a480e0.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorldManager {
    Climate,
    Trade,
    GreatPeople,
    Religion,
    Quests,
    RandomEvents,
    Fallout,
    DiplomaticRelations,
    Eras,
    Emergency,
    Congress,
    Economic,
    Heroes,
}
pub trait WorldEffects {
    /// Count is reread after every component callback, as in native Map.DoTurn.
    fn map_component_count(&mut self, state: &LoadedState) -> Result<u32, Error>;
    fn map_component_turn(&mut self, state: &mut LoadedState, index: u32) -> Result<(), Error>;
    /// Invoke the actual loaded direct/virtual manager implementation. A manager
    /// name never establishes empty state, absent gameplay effects or no RNG.
    fn manager_turn(&mut self, state: &mut LoadedState, manager: WorldManager)
        -> Result<(), Error>;
}
pub fn xp2_begin_world(
    state: &mut LoadedState,
    effects: &mut impl WorldEffects,
) -> Result<(), Error> {
    let mut index = 0u32;
    while index < effects.map_component_count(state)? {
        effects.map_component_turn(state, index)?;
        index = index.wrapping_add(1);
    }
    use WorldManager::*;
    for manager in [
        Climate,
        Trade,
        GreatPeople,
        Religion,
        Quests,
        RandomEvents,
        Fallout,
        DiplomaticRelations,
        Eras,
        Emergency,
        Congress,
        Economic,
        Heroes,
    ] {
        effects.manager_turn(state, manager)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn culture_end_turn_clears_completion_without_rewriting_hash_or_government() {
        let mut state = crate::loaded_state::tests::state();
        let owner = state.retained.players[0].requested_owner;
        let culture = &mut state.retained.players[0].culture;
        culture.civic_completed_this_turn = true;
        culture.civic_completed_this_turn_hash = 0x12345678;
        culture.government_change_considered = true;
        super::end_player_culture(&mut state, owner).unwrap();
        let culture = &state.retained.players[0].culture;
        assert!(!culture.civic_completed_this_turn);
        assert_eq!(culture.civic_completed_this_turn_hash, 0x12345678);
        assert!(culture.government_change_considered);
    }
    use super::*;
    #[test]
    fn slot_reset_does_not_touch_other_slots_or_phase_counts() {
        let mut p = crate::core_stream::PlayerProcessor {
            activation_event: [false; 64],
            first_activation: [false; 64],
            activation_needed: [false; 64],
            automation_processed: [true; 64],
            player_complete: [true; 64],
            game_complete: [true; 64],
            updates_this_phase: [17; 64],
            updates_this_turn: [23; 64],
            ai_updates_this_turn: [29; 64],
            active_count: 7,
            human_first_activated: true,
        };
        reset_slot(&mut p, 0);
        assert!(p.activation_event[0] && p.first_activation[0] && p.activation_needed[0]);
        assert!(!p.player_complete[0] && !p.game_complete[0] && !p.automation_processed[0]);
        assert_eq!((p.updates_this_turn[0], p.ai_updates_this_turn[0]), (0, 0));
        assert_eq!((p.updates_this_phase[0], p.updates_this_turn[1]), (17, 23));
    }
    struct FailingBegin {
        reports: Vec<(u32, i32)>,
        reached: Option<i32>,
    }
    impl TurnEffects for FailingBegin {
        fn report(&mut self, _: &mut LoadedState, event: u32, value: i32) -> Result<(), Error> {
            self.reports.push((event, value));
            Ok(())
        }
        fn player_begin(
            &mut self,
            _: &mut LoadedState,
            _: &mut Runtime,
            owner: i32,
        ) -> Result<(), Error> {
            self.reached = Some(owner);
            Err(Error::Unsupported("first reached GlobalAi.ProcessAi"))
        }
        fn player_end(&mut self, _: &mut LoadedState, _: i32) -> Result<(), Error> {
            panic!("unreached")
        }
        fn player_end_tail(&mut self, _: &mut LoadedState) -> Result<(), Error> {
            panic!("unreached")
        }
        fn world_begin(&mut self, _: &mut LoadedState) -> Result<(), Error> {
            panic!("must not skip player BeginTurn")
        }
        fn turn_signal(&mut self, _: &mut LoadedState, _: bool, _: i32) -> Result<(), Error> {
            panic!("unreached")
        }
        fn activate_at_turn_start(&mut self, _: &mut LoadedState) -> Result<(), Error> {
            panic!("unreached")
        }
        fn autoplay_end(&mut self, _: &mut LoadedState, _: &mut Runtime) -> Result<(), Error> {
            panic!("unreached")
        }
    }
    #[test]
    fn begin_failure_preserves_interleaved_prefix_and_prevents_replay() {
        let (mut state, mut runtime, owner) =
            crate::loaded_state::tests::current_update_player_fixture();
        runtime.pending = Some(crate::game_update::Continuation::BeginTurn);
        let turn = state.retained.core.current_turn;
        let phase = state.retained.core.phase_processor.current_phase;
        state.retained.core.game_processor.advance_turn = true;
        state
            .retained
            .core
            .player_processor
            .player_complete
            .fill(true);
        let mut effects = FailingBegin {
            reports: Vec::new(),
            reached: None,
        };
        assert_eq!(
            begin_turn(&mut state, &mut runtime, &mut effects),
            Err(Error::Unsupported("first reached GlobalAi.ProcessAi"))
        );
        assert_eq!(effects.reports, vec![(0x1e39e493, turn.wrapping_add(1))]);
        assert_eq!(effects.reached, Some(owner));
        assert_eq!(state.retained.core.current_turn, turn);
        assert_eq!(state.retained.core.phase_processor.current_phase, phase);
        assert_eq!(state.player_end_turn_blocking(owner).unwrap(), 0);
        assert!(!state.retained.core.player_processor.player_complete[owner as usize]);
        if owner < 63 {
            assert!(state.retained.core.player_processor.player_complete[owner as usize + 1]);
        }
        assert!(runtime.failed());
        assert_eq!(
            begin_turn(&mut state, &mut runtime, &mut effects),
            Err(Error::Unsupported("round BeginTurn continuation"))
        );
    }
    struct PhaseProbe {
        segments: Vec<u32>,
        trace: Vec<&'static str>,
        tactical: bool,
        activate: bool,
    }
    impl PhaseEffects for PhaseProbe {
        fn allow_tactical(&mut self, _: &LoadedState) -> Result<bool, Error> {
            self.trace.push("tactical");
            Ok(self.tactical)
        }
        fn automation(&mut self, s: &mut LoadedState) -> Result<(), Error> {
            self.trace.push("automation");
            if self.activate {
                s.retained.core.player_processor.active_count = 1;
            }
            Ok(())
        }
        fn end_phase(&mut self, _: &mut LoadedState) -> Result<(), Error> {
            self.trace.push("end");
            Ok(())
        }
        fn current_segment(&mut self, _: &LoadedState) -> Result<u32, Error> {
            Ok(101)
        }
        fn selected_count(&mut self) -> Result<usize, Error> {
            Ok(self.segments.len())
        }
        fn segment_at(&mut self, i: usize) -> Result<u32, Error> {
            Ok(self.segments[i])
        }
        fn segment_disabled(&mut self, _: &LoadedState, h: u32) -> Result<bool, Error> {
            Ok(h == 102)
        }
        fn time_limit(&mut self, s: &mut LoadedState) -> Result<u32, Error> {
            self.trace.push("timer");
            assert_eq!(s.retained.core.phase_processor.current_phase, 2);
            Ok(47)
        }
        fn begin_phase(&mut self, _: &mut LoadedState, p: i32) -> Result<(), Error> {
            self.trace.push("begin");
            assert_eq!(p, 101);
            Ok(())
        }
    }
    #[test]
    fn phase_skips_disabled_and_exhaustion_keeps_last_current_index() {
        let (mut state, _, _) = crate::loaded_state::tests::current_update_player_fixture();
        state.retained.core.phase_processor.current_phase = 0;
        let mut p = PhaseProbe {
            segments: vec![101, 102, 103],
            trace: vec![],
            tactical: false,
            activate: false,
        };
        let mut changed = false;
        assert_eq!(
            advance_phase(&mut state, &mut changed, &mut p),
            Ok(PhaseAdvance::BeganPhase)
        );
        assert!(changed);
        assert_eq!(p.trace, vec!["tactical", "end", "timer", "begin"]);
        assert_eq!(state.retained.core.phase_processor.time_limit, 47);
        p.trace.clear();
        assert_eq!(
            advance_phase(&mut state, &mut changed, &mut p),
            Ok(PhaseAdvance::Exhausted)
        );
        assert_eq!(state.retained.core.phase_processor.current_phase, 2);
        assert_eq!(state.retained.core.phase_processor.time_limit, 47);
        assert_eq!(p.trace, vec!["tactical", "end"]);
    }
    #[test]
    fn tactical_automation_activation_suppresses_phase_change() {
        let (mut state, _, _) = crate::loaded_state::tests::current_update_player_fixture();
        state.retained.core.phase_processor.current_phase = 0;
        let before = state.retained.core.phase_processor.clone();
        let mut p = PhaseProbe {
            segments: vec![101],
            trace: vec![],
            tactical: true,
            activate: true,
        };
        let mut changed = true;
        assert_eq!(
            advance_phase(&mut state, &mut changed, &mut p),
            Ok(PhaseAdvance::AutomationActivated)
        );
        assert!(!changed);
        assert_eq!(p.trace, vec!["tactical", "automation"]);
        assert_eq!(state.retained.core.phase_processor, before);
    }
    #[test]
    fn world_rechecks_component_count_and_stops_at_first_required_manager() {
        struct Probe {
            count: u32,
            components: Vec<u32>,
            managers: Vec<WorldManager>,
        }
        impl WorldEffects for Probe {
            fn map_component_count(&mut self, _: &LoadedState) -> Result<u32, Error> {
                Ok(self.count)
            }
            fn map_component_turn(&mut self, _: &mut LoadedState, i: u32) -> Result<(), Error> {
                self.components.push(i);
                if i == 0 {
                    self.count = 2;
                }
                Ok(())
            }
            fn manager_turn(&mut self, _: &mut LoadedState, m: WorldManager) -> Result<(), Error> {
                self.managers.push(m);
                if m == WorldManager::RandomEvents {
                    Err(Error::Unsupported("actual random-event turn"))
                } else {
                    Ok(())
                }
            }
        }
        let mut s = crate::loaded_state::tests::state();
        let mut p = Probe {
            count: 1,
            components: vec![],
            managers: vec![],
        };
        assert_eq!(
            xp2_begin_world(&mut s, &mut p),
            Err(Error::Unsupported("actual random-event turn"))
        );
        assert_eq!(p.components, vec![0, 1]);
        assert_eq!(
            p.managers,
            vec![
                WorldManager::Climate,
                WorldManager::Trade,
                WorldManager::GreatPeople,
                WorldManager::Religion,
                WorldManager::Quests,
                WorldManager::RandomEvents
            ]
        );
    }
}
