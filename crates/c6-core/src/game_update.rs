//! Source-ordered Linux XP2 Game.Update prefix and player selection.
//! This is an internal partial continuation, never a completed Game.Update or
//! an external player decision. Unimplemented upkeep must remain unsupported.
pub use crate::turn_completion::Error;
pub mod automation;
pub mod deactivation;

pub const AI_PROCESSED: u32 = 0x4fc6_296a;
pub const INITIAL_AI_PROCESSED: u32 = 0x91d5_ac36;
pub const LOCAL_COMPUTER_PROCESSING: u32 = 0x715c_87d1;

/// Live identities and current getters. The two alive traversals retain their
/// respective original end and storage identity, rereading entries/fields.
pub trait World {
    type Player: Copy;
    type Alive: Copy;
    fn autoplay_end_signaled(&mut self) -> Result<bool, Error>;
    fn game_local_player(&mut self) -> Result<i32, Error>;
    fn player_valid(&mut self, id: i32) -> Result<bool, Error>;
    fn player(&mut self, id: i32) -> Result<Self::Player, Error>;
    fn turn_active(&mut self, player: Self::Player) -> Result<bool, Error>;
    fn autoplay_has_pending(&mut self) -> Result<bool, Error>;
    fn autoplay_pending(&mut self) -> Result<bool, Error>;
    fn autoplay_active(&mut self) -> Result<bool, Error>;
    fn set_autoplay_has_pending(&mut self, value: bool) -> Result<(), Error>;
    /// Exact remaining CheckStateChange effects after hasPending=false and a
    /// differing pending/active state: control/local-player/observer changes,
    /// Lua event, active assignment and zero-turn follow-up. The argument is
    /// the first pending-state read only: native rereads pending after Lua.
    /// No default/no-op.
    fn autoplay_transition(&mut self, pending: bool) -> Result<(), Error>;
    fn set_autoplay_end_signaled(&mut self, value: bool) -> Result<(), Error>;
    fn set_needs_update(&mut self, value: bool) -> Result<(), Error>;
    fn turn_complete(&mut self) -> Result<bool, Error>;
    fn advance_turn(&mut self) -> Result<bool, Error>;
    fn new_game(&mut self) -> Result<bool, Error>;
    fn alive_begin(&mut self) -> Result<(Self::Alive, usize), Error>;
    fn alive_at(&mut self, list: Self::Alive, index: usize) -> Result<Self::Player, Error>;
    fn human(&mut self, player: Self::Player) -> Result<bool, Error>;
    fn player_id(&mut self, player: Self::Player) -> Result<i32, Error>;
    fn current_turn(&mut self) -> Result<i32, Error>;
    fn last_ai_processed(&mut self) -> Result<i32, Error>;
    fn set_last_ai_processed(&mut self, turn: i32) -> Result<(), Error>;
    fn notify_turn_request(&mut self, event: u32) -> Result<(), Error>;
    fn slot_status(&mut self, id: i32) -> Result<i32, Error>;
    fn auto_player_game(&mut self) -> Result<bool, Error>;
    fn last_initial_ai_processed(&mut self, id: i32) -> Result<i32, Error>;
    fn unavailable_units(&mut self, player: Self::Player) -> Result<i32, Error>;
    fn set_last_initial_ai_processed(&mut self, id: i32, turn: i32) -> Result<(), Error>;
    fn context_local_player(&mut self) -> Result<i32, Error>;
    fn has_out_of_turn_processing(&mut self, player: Self::Player) -> Result<bool, Error>;
}

/// 2824a30. A pending transition consumes its flag before comparing states or
/// invoking effects. An error retains that prefix and skips the outer clear.
pub fn check_autoplay_state<W: World>(w: &mut W) -> Result<(), Error> {
    if w.autoplay_has_pending()? {
        w.set_autoplay_has_pending(false)?;
        let pending = w.autoplay_pending()?;
        if pending != w.autoplay_active()? {
            w.autoplay_transition(pending)?;
        }
    }
    Ok(())
}

/// 28249b0. In particular, the invalid-local-player branch checks pending state
/// only there; endTurnSignaled bypasses local-player resolution altogether.
pub fn update_autoplay<W: World>(w: &mut W) -> Result<(), Error> {
    let check = if w.autoplay_end_signaled()? {
        true
    } else {
        let id = w.game_local_player()?;
        if w.player_valid(id)? {
            let player = w.player(id)?;
            w.turn_active(player)?
        } else {
            w.autoplay_has_pending()? && w.autoplay_pending()?
        }
    };
    if check {
        check_autoplay_state(w)?;
    }
    w.set_autoplay_end_signaled(false)
}

/// 2a0c430: preserve actual lazy read order, rather than evaluating NewGame
/// first from the algebraically equivalent expression.
pub fn is_new_turn<W: World>(w: &mut W) -> Result<bool, Error> {
    if w.turn_complete()? && w.advance_turn()? {
        Ok(true)
    } else {
        w.new_game()
    }
}

fn set_ai_processed<W: World>(w: &mut W) -> Result<(), Error> {
    let turn = w.current_turn()?;
    if w.last_ai_processed()? != turn {
        w.notify_turn_request(AI_PROCESSED)?;
        w.set_last_ai_processed(turn)?;
    }
    Ok(())
}

fn set_initial_ai_processed<W: World>(w: &mut W, id: i32) -> Result<(), Error> {
    let turn = w.current_turn()?;
    if w.last_initial_ai_processed(id)? != turn {
        let player = w.player(id)?;
        // Native GetUnitCounts precedes IsTurnActive, even for inactive players.
        let count = w.unavailable_units(player)?;
        if !w.turn_active(player)? || count == 0 {
            w.set_last_initial_ai_processed(id, turn)?;
            if w.context_local_player()? == id {
                w.notify_turn_request(INITIAL_AI_PROCESSED)?;
            }
        }
    }
    Ok(())
}

/// 2a0c0c0. Selection itself has real counter/notification effects. Selected
/// identities are the native temporary vector; mutable player fields aren't
/// snapshotted. Never sort or infer completion excludes an active human.
pub fn players_to_process<W: World>(w: &mut W) -> Result<Vec<i32>, Error> {
    let (list, end) = w.alive_begin()?;
    for i in 0..end {
        let player = w.alive_at(list, i)?;
        if w.turn_active(player)? && !w.human(player)? {
            // Original vector stores GetID; resolve again at the Update caller.
            let id = w.player_id(player)?;
            return Ok(vec![id]);
        }
    }
    let _ = w.current_turn()?;
    set_ai_processed(w)?;
    let local = w.game_local_player()?;
    if local != -1 && w.slot_status(local)? == 4 {
        w.notify_turn_request(LOCAL_COMPUTER_PROCESSING)?;
    }
    let mut included_inactive = w.auto_player_game()?;
    let (list, end) = w.alive_begin()?;
    let mut result = Vec::new();
    for i in 0..end {
        let player = w.alive_at(list, i)?;
        let id = w.player_id(player)?;
        set_initial_ai_processed(w, id)?;
        if w.turn_active(player)? && w.human(player)? {
            let id = w.player_id(player)?;
            result.push(id);
        } else if !included_inactive
            && !w.turn_active(player)?
            && w.has_out_of_turn_processing(player)?
        {
            let id = w.player_id(player)?;
            result.push(id);
            included_inactive = true;
        }
    }
    Ok(result)
}

/// Required next internal phase. No variant certifies completion or a genuine
/// external decision. No Clone: callers must retain one continuation owner.
#[derive(Debug, Eq, PartialEq)]
#[must_use = "the remainder of Game.Update is still mandatory"]
pub enum Continuation {
    BeginTurn,
    PlayerUpdates(Vec<i32>),
}

/// 2661ba0 -> 2a0cb70 through the first ordinary player vector. The new-turn
/// branch stops BEFORE BeginTurn, whose mandatory world upkeep is not skipped.
pub fn prepare<W: World>(w: &mut W) -> Result<Continuation, Error> {
    update_autoplay(w)?;
    w.set_needs_update(false)?;
    if is_new_turn(w)? {
        Ok(Continuation::BeginTurn)
    } else {
        Ok(Continuation::PlayerUpdates(players_to_process(w)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Player {
        id: i32,
        human: bool,
        active: bool,
        out: bool,
        initial: i32,
    }
    struct Probe {
        players: Vec<Player>,
        trace: Vec<String>,
        end: bool,
        pending: bool,
        pending_state: bool,
        active: bool,
        needs: bool,
        complete: bool,
        advance: Option<bool>,
        new_game: Option<bool>,
        turn: i32,
        last: i32,
        auto: bool,
        local: i32,
        transition_error: bool,
        count_error: Option<i32>,
        notify_advances_turn: bool,
    }
    impl Probe {
        fn new(players: Vec<Player>) -> Self {
            Self {
                players,
                trace: vec![],
                end: true,
                pending: false,
                pending_state: false,
                active: false,
                needs: true,
                complete: false,
                advance: None,
                new_game: Some(false),
                turn: 7,
                last: 7,
                auto: false,
                local: -1,
                transition_error: false,
                count_error: None,
                notify_advances_turn: false,
            }
        }
        fn row(&self, id: i32) -> &Player {
            self.players.iter().find(|p| p.id == id).unwrap()
        }
        fn note(&mut self, s: impl Into<String>) {
            self.trace.push(s.into());
        }
    }
    impl World for Probe {
        type Player = i32;
        type Alive = usize;
        fn autoplay_end_signaled(&mut self) -> Result<bool, Error> {
            self.note("end?");
            Ok(self.end)
        }
        fn game_local_player(&mut self) -> Result<i32, Error> {
            self.note("game.local");
            Ok(self.local)
        }
        fn player_valid(&mut self, id: i32) -> Result<bool, Error> {
            Ok(self.players.iter().any(|p| p.id == id))
        }
        fn player(&mut self, id: i32) -> Result<i32, Error> {
            self.note(format!("resolve:{id}"));
            Ok(id)
        }
        fn turn_active(&mut self, id: i32) -> Result<bool, Error> {
            self.note(format!("active:{id}"));
            Ok(self.row(id).active)
        }
        fn autoplay_has_pending(&mut self) -> Result<bool, Error> {
            self.note("pending?");
            Ok(self.pending)
        }
        fn autoplay_pending(&mut self) -> Result<bool, Error> {
            self.note("pending.state");
            Ok(self.pending_state)
        }
        fn autoplay_active(&mut self) -> Result<bool, Error> {
            self.note("autoplay.active");
            Ok(self.active)
        }
        fn set_autoplay_has_pending(&mut self, v: bool) -> Result<(), Error> {
            self.note("pending.write");
            self.pending = v;
            Ok(())
        }
        fn autoplay_transition(&mut self, _: bool) -> Result<(), Error> {
            self.note("transition");
            if self.transition_error {
                Err(Error::Unsupported("transition"))
            } else {
                panic!("unimplemented transition must not succeed")
            }
        }
        fn set_autoplay_end_signaled(&mut self, v: bool) -> Result<(), Error> {
            self.note("end.write");
            self.end = v;
            Ok(())
        }
        fn set_needs_update(&mut self, v: bool) -> Result<(), Error> {
            self.note("needs.write");
            self.needs = v;
            Ok(())
        }
        fn turn_complete(&mut self) -> Result<bool, Error> {
            self.note("complete?");
            Ok(self.complete)
        }
        fn advance_turn(&mut self) -> Result<bool, Error> {
            self.note("advance?");
            Ok(self.advance.expect("unreachable AdvanceTurn"))
        }
        fn new_game(&mut self) -> Result<bool, Error> {
            self.note("new?");
            Ok(self.new_game.expect("unreachable NewGame"))
        }
        fn alive_begin(&mut self) -> Result<(usize, usize), Error> {
            self.note("alive.begin");
            Ok((0, self.players.len()))
        }
        fn alive_at(&mut self, _: usize, i: usize) -> Result<i32, Error> {
            Ok(self.players[i].id)
        }
        fn human(&mut self, id: i32) -> Result<bool, Error> {
            self.note(format!("human:{id}"));
            Ok(self.row(id).human)
        }
        fn player_id(&mut self, id: i32) -> Result<i32, Error> {
            self.note(format!("id:{id}"));
            Ok(id)
        }
        fn current_turn(&mut self) -> Result<i32, Error> {
            self.note("turn?");
            Ok(self.turn)
        }
        fn last_ai_processed(&mut self) -> Result<i32, Error> {
            Ok(self.last)
        }
        fn set_last_ai_processed(&mut self, t: i32) -> Result<(), Error> {
            self.note("last.write");
            self.last = t;
            Ok(())
        }
        fn notify_turn_request(&mut self, e: u32) -> Result<(), Error> {
            self.note(format!("notify:{e:x}"));
            if self.notify_advances_turn {
                self.turn += 1;
            }
            Ok(())
        }
        fn slot_status(&mut self, _: i32) -> Result<i32, Error> {
            panic!("unreachable slot-status")
        }
        fn auto_player_game(&mut self) -> Result<bool, Error> {
            self.note("auto?");
            Ok(self.auto)
        }
        fn last_initial_ai_processed(&mut self, id: i32) -> Result<i32, Error> {
            Ok(self.row(id).initial)
        }
        fn unavailable_units(&mut self, id: i32) -> Result<i32, Error> {
            self.note(format!("counts:{id}"));
            if self.count_error == Some(id) {
                Err(Error::Unsupported("counts"))
            } else {
                Ok(0)
            }
        }
        fn set_last_initial_ai_processed(&mut self, id: i32, t: i32) -> Result<(), Error> {
            self.note(format!("initial.write:{id}"));
            self.players
                .iter_mut()
                .find(|p| p.id == id)
                .unwrap()
                .initial = t;
            Ok(())
        }
        fn context_local_player(&mut self) -> Result<i32, Error> {
            self.note("context.local");
            Ok(self.local)
        }
        fn has_out_of_turn_processing(&mut self, id: i32) -> Result<bool, Error> {
            self.note(format!("out:{id}"));
            Ok(self.row(id).out)
        }
    }
    fn p(id: i32, human: bool, active: bool, out: bool, initial: i32) -> Player {
        Player {
            id,
            human,
            active,
            out,
            initial,
        }
    }
    #[test]
    fn active_nonhuman_shortcut_preserves_order_and_skips_stateful_second_pass() {
        let mut w = Probe::new(vec![
            p(9, false, false, false, 0),
            p(7, false, true, false, 0),
            p(0, true, true, false, 0),
        ]);
        assert_eq!(prepare(&mut w), Ok(Continuation::PlayerUpdates(vec![7])));
        assert!(!w.end && !w.needs);
        assert!(!w
            .trace
            .iter()
            .any(|s| s == "game.local" || s == "turn?" || s == "auto?"));
        assert_eq!(w.trace.iter().filter(|s| *s == "alive.begin").count(), 1);
        assert!(!w.trace.iter().any(|s| s.starts_with("resolve:")));
    }
    #[test]
    fn pending_transition_failure_retains_consumed_flag_before_outer_clears() {
        let mut w = Probe::new(vec![]);
        w.pending = true;
        w.pending_state = true;
        w.transition_error = true;
        assert_eq!(prepare(&mut w), Err(Error::Unsupported("transition")));
        assert!(!w.pending && w.end && w.needs);
        assert_eq!(
            w.trace,
            [
                "end?",
                "pending?",
                "pending.write",
                "pending.state",
                "autoplay.active",
                "transition"
            ]
        );
    }
    #[test]
    fn new_turn_short_circuits_new_game_and_stops_before_upkeep() {
        let mut w = Probe::new(vec![]);
        w.complete = true;
        w.advance = Some(true);
        w.new_game = None;
        assert_eq!(prepare(&mut w), Ok(Continuation::BeginTurn));
        assert!(!w.needs && !w.end);
        assert!(!w.trace.iter().any(|s| s == "new?" || s == "alive.begin"));
    }
    #[test]
    fn selection_keeps_active_human_then_only_one_inactive_and_store_uses_pre_notify_turn() {
        let mut w = Probe::new(vec![
            p(0, true, true, false, 8),
            p(7, false, false, true, 8),
            p(5, false, false, true, 8),
        ]);
        w.last = 6;
        w.notify_advances_turn = true;
        assert_eq!(prepare(&mut w), Ok(Continuation::PlayerUpdates(vec![0, 7])));
        assert_eq!((w.last, w.turn), (7, 8));
        let notify = w.trace.iter().position(|s| s == "notify:4fc6296a").unwrap();
        let store = w.trace.iter().position(|s| s == "last.write").unwrap();
        assert!(notify < store);
        assert!(!w.trace.iter().any(|s| s == "out:5"));
    }
    #[test]
    fn unit_count_failure_retains_global_prefix_and_precedes_initial_active_test() {
        let mut w = Probe::new(vec![
            p(0, true, true, false, 7),
            p(7, false, false, true, 6),
        ]);
        w.last = 6;
        w.count_error = Some(7);
        assert_eq!(prepare(&mut w), Err(Error::Unsupported("counts")));
        assert_eq!(w.last, 7);
        assert!(!w.needs && !w.end);
        assert_eq!(w.trace.last().map(String::as_str), Some("counts:7"));
        assert_eq!(w.row(7).initial, 6);
    }
}

/// Required callback inside one selected Player.Update. These are gameplay
/// continuations, not inferred player decisions. Activation flags have already
/// been consumed when Activation is returned; its report still follows the hook.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayerEffect<P> {
    Activation { player: P, first: bool },
    ActiveControl { player: P },
    InactiveControl { player: P },
    Blocking { player: P },
}

/// Scalar source body of Player.Update/DoTurnActivate/DoTurn. Actual callback
/// bodies are required separately; unknown AI-control work is never a no-op.
pub trait PlayerWorld {
    type Player: Copy;
    fn active(&mut self, player: Self::Player) -> Result<bool, Error>;
    fn id(&mut self, player: Self::Player) -> Result<i32, Error>;
    fn activation_event(&mut self, id: i32) -> Result<bool, Error>;
    fn first_activation(&mut self, id: i32) -> Result<bool, Error>;
    fn set_first_activation(&mut self, id: i32, value: bool) -> Result<(), Error>;
    fn set_activation_event(&mut self, id: i32, value: bool) -> Result<(), Error>;
    fn human(&mut self, player: Self::Player) -> Result<bool, Error>;
    fn phase_updates(&mut self, id: i32) -> Result<u32, Error>;
    fn set_phase_updates(&mut self, id: i32, value: u32) -> Result<(), Error>;
    fn turn_updates(&mut self, id: i32) -> Result<u32, Error>;
    fn set_turn_updates(&mut self, id: i32, value: u32) -> Result<(), Error>;
    fn ai_updates(&mut self, id: i32) -> Result<u32, Error>;
    fn set_ai_updates(&mut self, id: i32, value: u32) -> Result<(), Error>;
    fn set_blocker(&mut self, player: Self::Player, value: i32) -> Result<(), Error>;
}

/// Source continuation after TurnManager.DoTurn returns. Human DoTurn returns
/// directly; nonhuman callers must first execute the real mixed Control.DoTurn.
pub fn after_turn_manager<W: PlayerWorld>(
    w: &mut W,
    player: W::Player,
) -> Result<PlayerEffect<W::Player>, Error> {
    let id = w.id(player)?;
    let value = w.phase_updates(id)?.wrapping_add(1);
    w.set_phase_updates(id, value)?;
    let id = w.id(player)?;
    let value = w.turn_updates(id)?.wrapping_add(1);
    w.set_turn_updates(id, value)?;
    if !w.human(player)? {
        let id = w.id(player)?;
        let value = w.ai_updates(id)?.wrapping_add(1);
        w.set_ai_updates(id, value)?;
    }
    // Player.Update rereads the phase count AFTER DoTurn's counter stores.
    let id = w.id(player)?;
    if w.phase_updates(id)? == 1 {
        w.set_blocker(player, -1)?;
    }
    Ok(PlayerEffect::Blocking { player })
}

/// 2b5c720 -> 2b5ac00 -> 2b5acb0. Source human/no-activation-event path reaches
/// real blocking update; other hooks retain a precise unfinished callback.
pub fn prepare_player<W: PlayerWorld>(
    w: &mut W,
    player: W::Player,
) -> Result<PlayerEffect<W::Player>, Error> {
    if !w.active(player)? {
        return Ok(PlayerEffect::InactiveControl { player });
    }
    let id = w.id(player)?;
    if w.activation_event(id)? {
        let first = w.first_activation(id)?;
        w.set_first_activation(id, false)?;
        w.set_activation_event(id, false)?;
        return Ok(PlayerEffect::Activation { player, first });
    }
    // TurnManager.DoTurn checks the current owner's actual IsHuman. Do not
    // replace native roles to make external policies easier to implement.
    if !w.human(player)? {
        return Ok(PlayerEffect::ActiveControl { player });
    }
    after_turn_manager(w, player)
}

#[cfg(test)]
mod player_tests {
    use super::*;
    struct Probe {
        active: bool,
        human: bool,
        event: bool,
        first: bool,
        phase: u32,
        turn: u32,
        ai: u32,
        blocker: i32,
        trace: Vec<&'static str>,
    }
    impl Probe {
        fn current() -> Self {
            Self {
                active: true,
                human: true,
                event: false,
                first: false,
                phase: 5,
                turn: 5,
                ai: 0,
                blocker: 7,
                trace: vec![],
            }
        }
    }
    impl PlayerWorld for Probe {
        type Player = i32;
        fn active(&mut self, _: i32) -> Result<bool, Error> {
            self.trace.push("active");
            Ok(self.active)
        }
        fn id(&mut self, p: i32) -> Result<i32, Error> {
            Ok(p)
        }
        fn activation_event(&mut self, _: i32) -> Result<bool, Error> {
            self.trace.push("event");
            Ok(self.event)
        }
        fn first_activation(&mut self, _: i32) -> Result<bool, Error> {
            self.trace.push("first");
            Ok(self.first)
        }
        fn set_first_activation(&mut self, _: i32, v: bool) -> Result<(), Error> {
            self.trace.push("first.write");
            self.first = v;
            Ok(())
        }
        fn set_activation_event(&mut self, _: i32, v: bool) -> Result<(), Error> {
            self.trace.push("event.write");
            self.event = v;
            Ok(())
        }
        fn human(&mut self, _: i32) -> Result<bool, Error> {
            self.trace.push("human");
            Ok(self.human)
        }
        fn phase_updates(&mut self, _: i32) -> Result<u32, Error> {
            Ok(self.phase)
        }
        fn set_phase_updates(&mut self, _: i32, v: u32) -> Result<(), Error> {
            self.trace.push("phase.write");
            self.phase = v;
            Ok(())
        }
        fn turn_updates(&mut self, _: i32) -> Result<u32, Error> {
            Ok(self.turn)
        }
        fn set_turn_updates(&mut self, _: i32, v: u32) -> Result<(), Error> {
            self.trace.push("turn.write");
            self.turn = v;
            Ok(())
        }
        fn ai_updates(&mut self, _: i32) -> Result<u32, Error> {
            Ok(self.ai)
        }
        fn set_ai_updates(&mut self, _: i32, v: u32) -> Result<(), Error> {
            self.trace.push("ai.write");
            self.ai = v;
            Ok(())
        }
        fn set_blocker(&mut self, _: i32, v: i32) -> Result<(), Error> {
            self.trace.push("blocker.write");
            self.blocker = v;
            Ok(())
        }
    }
    #[test]
    fn actual_human_leaf_advances_five_to_six_before_blocking_without_reset() {
        let mut w = Probe::current();
        assert_eq!(
            prepare_player(&mut w, 0),
            Ok(PlayerEffect::Blocking { player: 0 })
        );
        assert_eq!((w.phase, w.turn, w.ai, w.blocker), (6, 6, 0, 7));
        assert_eq!(
            w.trace,
            [
                "active",
                "event",
                "human",
                "phase.write",
                "turn.write",
                "human"
            ]
        );
    }
    #[test]
    fn first_phase_update_resets_blocker_but_wrapping_to_zero_does_not() {
        let mut w = Probe::current();
        w.phase = 0;
        assert_eq!(
            prepare_player(&mut w, 0),
            Ok(PlayerEffect::Blocking { player: 0 })
        );
        assert_eq!((w.phase, w.blocker), (1, -1));
        let mut w = Probe::current();
        w.phase = u32::MAX;
        w.turn = u32::MAX;
        assert_eq!(
            prepare_player(&mut w, 0),
            Ok(PlayerEffect::Blocking { player: 0 })
        );
        assert_eq!((w.phase, w.turn, w.blocker), (0, 0, 7));
    }
    #[test]
    fn activation_consumes_both_flags_before_unimplemented_hook_without_counters() {
        let mut w = Probe::current();
        w.event = true;
        w.first = true;
        assert_eq!(
            prepare_player(&mut w, 0),
            Ok(PlayerEffect::Activation {
                player: 0,
                first: true
            })
        );
        assert!(!w.event && !w.first);
        assert_eq!((w.phase, w.turn), (5, 5));
        assert_eq!(
            w.trace,
            ["active", "event", "first", "first.write", "event.write"]
        );
    }
    #[test]
    fn inactive_and_nonhuman_paths_require_real_control_work_before_counters() {
        let mut w = Probe::current();
        w.active = false;
        assert_eq!(
            prepare_player(&mut w, 7),
            Ok(PlayerEffect::InactiveControl { player: 7 })
        );
        assert_eq!(w.trace, ["active"]);
        let mut w = Probe::current();
        w.human = false;
        assert_eq!(
            prepare_player(&mut w, 7),
            Ok(PlayerEffect::ActiveControl { player: 7 })
        );
        assert_eq!((w.phase, w.turn, w.ai), (5, 5, 0));
        assert!(!w.human);
    }
}
