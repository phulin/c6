//! Current-state partial Game.Update over canonical saved processor/control data.
//! Unsaved globals are explicit live inputs; no defaults or duplicate saved state.
use super::LoadedState;
pub mod activation;
pub mod automation;
pub mod deactivation;
pub mod human_activation;
pub mod human_control;
pub mod inactive_control;
pub mod npc_activation;
pub mod npc_deactivation;
pub mod maintenance;
pub mod resources;
pub mod round;
pub mod round_ai;
use crate::{
    core_player::initialization::WireBinding,
    game_update::{self as core, Error},
};

#[derive(Clone, Copy, Debug)]
pub struct AutoplayInputs {
    pub end_turn_signaled: Option<bool>,
    pub has_pending_state_change: Option<bool>,
    pub pending_state: Option<bool>,
    pub active: Option<bool>,
}
#[derive(Debug)]
pub struct Inputs {
    pub autoplay: AutoplayInputs,
    pub needs_update: Option<bool>,
    pub auto_player_game: Option<bool>,
    /// Current Player.Manager local-player field for this exclusive Game.
    /// Game.GetInstance and Context.Globals.GetLocalPlayer resolve the same
    /// current Game/Manager (2660650/290ba20/2661af0). No saved scalar fallback.
    pub local_player: Option<i32>,
}

/// One owner for omitted live globals and the unfinished internal continuation.
/// Saved game/player processor and AI Control fields remain in LoadedState.
pub struct Runtime {
    binding: WireBinding,
    pub inputs: Inputs,
    pending: Option<core::Continuation>,
    failed: bool,
    player_cursor: usize,
    pass_has_human: bool,
    pass_processed_ai: bool,
    player_frame: Option<PlayerFrame>,
    deactivation_started: bool,
    local_observer: Option<i32>,
    /// Unsaved GlobalAi+110 processing depth. None requires a live input.
    global_ai_processing_depth: Option<i32>,
    round_begin_ready: bool,
    npc_activation: Option<npc_activation::Frame>,
}
impl Runtime {
    /// Binding checks identity, not provenance of caller-supplied runtime facts.
    pub fn new(state: &LoadedState, inputs: Inputs) -> Self {
        Self {
            binding: state.binding().clone(),
            inputs,
            pending: None,
            failed: false,
            player_cursor: 0,
            pass_has_human: false,
            pass_processed_ai: false,
            player_frame: None,
            deactivation_started: false,
            local_observer: None,
            global_ai_processing_depth: None,
            round_begin_ready: false,
            npc_activation: None,
        }
    }
    pub fn pending(&self) -> Option<&core::Continuation> {
        self.pending.as_ref()
    }
    pub fn failed(&self) -> bool {
        self.failed
    }
}

struct PlayerFrame {
    player: i32,
    effect: Option<core::PlayerEffect<i32>>,
    next_has_human: bool,
    alive_nonhuman: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlayerPassProgress {
    pub completed: usize,
    pub selected: usize,
    pub has_human: bool,
    pub processed_ai: bool,
}
impl Runtime {
    pub fn player_progress(&self) -> PlayerPassProgress {
        PlayerPassProgress {
            completed: self.player_cursor,
            selected: match &self.pending {
                Some(core::Continuation::PlayerUpdates(ids)) => ids.len(),
                _ => 0,
            },
            has_human: self.pass_has_human,
            processed_ai: self.pass_processed_ai,
        }
    }
    pub fn pending_player(&self) -> Option<i32> {
        self.player_frame.as_ref().map(|f| f.player)
    }
    pub fn pending_player_effect(&self) -> Option<core::PlayerEffect<i32>> {
        self.player_frame.as_ref().and_then(|f| f.effect)
    }
    /// A failed required synchronous callback cannot be retried as unchanged.
    pub fn fail_player_callback(&mut self) {
        self.failed = true;
    }
}

/// Only effectful services outside this slice. Alive tokens refer to the actual
/// shared manager list and captured end, never a freshly sorted owner snapshot.
pub trait Services {
    type Alive: Copy;
    fn alive_begin(&mut self, state: &LoadedState) -> Result<(Self::Alive, usize), Error>;
    fn alive_at(
        &mut self,
        state: &LoadedState,
        list: Self::Alive,
        index: usize,
    ) -> Result<i32, Error>;
    fn unavailable_units(&mut self, state: &mut LoadedState, owner: i32) -> Result<i32, Error>;
    /// Exact EngineUtility+70 notification and every synchronous consumer.
    fn notify_turn_request(&mut self, state: &mut LoadedState, event: u32) -> Result<(), Error>;
    /// Remaining source CheckStateChange after hasPending=false. It must retain
    /// real control/config/local-player/observer and Lua/RNG effects, including
    /// active/pending follow-up writes in this same runtime; unsupported is fine.
    fn autoplay_transition(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        pending: bool,
    ) -> Result<(), Error>;
}

pub struct Context<'a, S> {
    pub state: &'a mut LoadedState,
    pub runtime: &'a mut Runtime,
    pub services: S,
}
fn missing(_: impl std::fmt::Debug) -> Error {
    Error::Unsupported("Game.Update current state unavailable")
}
fn required<T>(value: Option<T>, field: &'static str) -> Result<T, Error> {
    value.ok_or(Error::Unsupported(field))
}
impl<'a, S: Services> Context<'a, S> {
    pub fn new(
        state: &'a mut LoadedState,
        runtime: &'a mut Runtime,
        services: S,
    ) -> Result<Self, Error> {
        if state.binding() != &runtime.binding {
            return Err(Error::Unsupported("Game.Update runtime binding"));
        }
        Ok(Self {
            state,
            runtime,
            services,
        })
    }
    /// Save exactly one partial continuation. This is never whole-Update success.
    pub fn prepare_prefix(&mut self) -> Result<&core::Continuation, Error> {
        if self.state.binding() != &self.runtime.binding
            || self.runtime.failed
            || self.runtime.pending.is_some()
        {
            return Err(Error::Unsupported(
                "Game.Update prefix already pending or failed",
            ));
        }
        match core::prepare(self) {
            Ok(next) => {
                self.runtime.pending = Some(next);
                Ok(self.runtime.pending.as_ref().expect("installed"))
            }
            Err(error) => {
                self.runtime.failed = true;
                Err(error)
            }
        }
    }
    /// Callable from a whole-Update provider only as an explicitly failing
    /// partial implementation. No unfinished prefix becomes successful Update.
    pub fn run_prefix(&mut self) -> Result<(), Error> {
        let error = match self.prepare_prefix()? {
            core::Continuation::BeginTurn => "Game.Update requires Game.BeginTurn upkeep",
            core::Continuation::PlayerUpdates(players) if players.is_empty() => {
                "Game.Update requires player deactivation selection"
            }
            core::Continuation::PlayerUpdates(_) => {
                "Game.Update requires selected Player.Update gameplay callbacks"
            }
        };
        Err(Error::Unsupported(error))
    }
    /// Finish the current stock human pass after deactivation/next selection.
    /// This exclusive synchronous executor has no outstanding native AI jobs.
    /// The processed-AI bit only controls the native Stopwatch at2a0d3b3;
    /// completed inactive-controller callbacks do not prevent this human exit.
    /// The supplied callback is Game.Instance.Update's mandatory pending-
    /// diplomacy tail, executed after the final Autoplay.Update.
    pub fn finish_exclusive_human_update(
        &mut self,
        pending_diplomacy: impl FnOnce(&mut LoadedState, &mut S) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let progress = self.runtime.player_progress();
        if self.runtime.failed
            || self.runtime.player_frame.is_some()
            || !matches!(
                self.runtime.pending,
                Some(core::Continuation::PlayerUpdates(_))
            )
            || progress.completed != progress.selected
            || !progress.has_human
            || self.state.retained.core.player_processor.active_count == 0
        {
            return Err(Error::Unsupported(
                "Game.Update unfinished human processor pass",
            ));
        }
        let result = (|| {
            core::update_autoplay(self)?;
            pending_diplomacy(self.state, &mut self.services)
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
        Ok(())
    }
    fn revision(&mut self) -> Result<(), Error> {
        self.state.revision = self
            .state
            .revision
            .checked_add(1)
            .ok_or(Error::Unsupported("loaded revision exhausted"))?;
        Ok(())
    }
    fn index(id: i32) -> Result<usize, Error> {
        usize::try_from(id)
            .ok()
            .filter(|&i| i < 64)
            .ok_or(Error::Unsupported("Game.Update player slot"))
    }
}
impl<S: Services> core::World for Context<'_, S> {
    type Player = i32;
    type Alive = S::Alive;
    fn autoplay_end_signaled(&mut self) -> Result<bool, Error> {
        required(
            self.runtime.inputs.autoplay.end_turn_signaled,
            "Autoplay endTurnSignaled",
        )
    }
    fn game_local_player(&mut self) -> Result<i32, Error> {
        required(
            self.runtime.inputs.local_player,
            "Game.GetLocalPlayer current source",
        )
    }
    fn player_valid(&mut self, id: i32) -> Result<bool, Error> {
        self.state.world.player_is_valid(id).map_err(missing)
    }
    fn player(&mut self, id: i32) -> Result<i32, Error> {
        self.state.world.player(id).map_err(missing)?;
        Ok(id)
    }
    fn turn_active(&mut self, player: i32) -> Result<bool, Error> {
        self.state.turn_active(player).map_err(missing)
    }
    fn autoplay_has_pending(&mut self) -> Result<bool, Error> {
        required(
            self.runtime.inputs.autoplay.has_pending_state_change,
            "Autoplay hasPendingStateChange",
        )
    }
    fn autoplay_pending(&mut self) -> Result<bool, Error> {
        required(
            self.runtime.inputs.autoplay.pending_state,
            "Autoplay pendingState",
        )
    }
    fn autoplay_active(&mut self) -> Result<bool, Error> {
        required(self.runtime.inputs.autoplay.active, "Autoplay active")
    }
    fn set_autoplay_has_pending(&mut self, value: bool) -> Result<(), Error> {
        self.revision()?;
        self.runtime.inputs.autoplay.has_pending_state_change = Some(value);
        Ok(())
    }
    fn autoplay_transition(&mut self, pending: bool) -> Result<(), Error> {
        self.services
            .autoplay_transition(self.state, self.runtime, pending)
    }
    fn set_autoplay_end_signaled(&mut self, value: bool) -> Result<(), Error> {
        self.revision()?;
        self.runtime.inputs.autoplay.end_turn_signaled = Some(value);
        Ok(())
    }
    fn set_needs_update(&mut self, value: bool) -> Result<(), Error> {
        self.revision()?;
        self.runtime.inputs.needs_update = Some(value);
        Ok(())
    }
    fn turn_complete(&mut self) -> Result<bool, Error> {
        Ok(self.state.retained.core.game_processor.turn_complete)
    }
    fn advance_turn(&mut self) -> Result<bool, Error> {
        Ok(self.state.retained.core.game_processor.advance_turn)
    }
    fn new_game(&mut self) -> Result<bool, Error> {
        Ok(self.state.retained.core.game_processor.new_game)
    }
    fn alive_begin(&mut self) -> Result<(Self::Alive, usize), Error> {
        self.services.alive_begin(self.state)
    }
    fn alive_at(&mut self, list: Self::Alive, index: usize) -> Result<i32, Error> {
        self.services.alive_at(self.state, list, index)
    }
    fn human(&mut self, player: i32) -> Result<bool, Error> {
        Ok(self.state.player_slot_status(player).map_err(missing)? == 3)
    }
    fn player_id(&mut self, player: i32) -> Result<i32, Error> {
        self.state.world.player(player).map_err(missing)?;
        Ok(player)
    }
    fn current_turn(&mut self) -> Result<i32, Error> {
        Ok(self.state.retained.core.current_turn)
    }
    fn last_ai_processed(&mut self) -> Result<i32, Error> {
        Ok(self
            .state
            .retained
            .core
            .game_processor
            .last_ai_processed_turn)
    }
    fn set_last_ai_processed(&mut self, turn: i32) -> Result<(), Error> {
        self.revision()?;
        self.state
            .retained
            .core
            .game_processor
            .last_ai_processed_turn = turn;
        Ok(())
    }
    fn notify_turn_request(&mut self, event: u32) -> Result<(), Error> {
        self.services.notify_turn_request(self.state, event)
    }
    fn slot_status(&mut self, id: i32) -> Result<i32, Error> {
        self.state.player_slot_status(id).map_err(missing)
    }
    fn auto_player_game(&mut self) -> Result<bool, Error> {
        required(
            self.runtime.inputs.auto_player_game,
            "Game autoPlayerGame current source",
        )
    }
    fn last_initial_ai_processed(&mut self, id: i32) -> Result<i32, Error> {
        Ok(self.state.retained.core.game_processor.last_initial_ai[Self::index(id)?])
    }
    fn unavailable_units(&mut self, player: i32) -> Result<i32, Error> {
        self.services.unavailable_units(self.state, player)
    }
    fn set_last_initial_ai_processed(&mut self, id: i32, turn: i32) -> Result<(), Error> {
        let i = Self::index(id)?;
        self.revision()?;
        self.state.retained.core.game_processor.last_initial_ai[i] = turn;
        Ok(())
    }
    fn context_local_player(&mut self) -> Result<i32, Error> {
        required(
            self.runtime.inputs.local_player,
            "Context.GetLocalPlayer current source",
        )
    }
    fn has_out_of_turn_processing(&mut self, player: i32) -> Result<bool, Error> {
        let retained = self
            .state
            .retained
            .players
            .iter()
            .find(|p| p.header.owner == player)
            .ok_or(Error::Unsupported("Game.Update AI Control owner"))?;
        let control = retained
            .ai_control
            .as_ref()
            .ok_or(Error::Unsupported("Game.Update AI Control current state"))?;
        if control.player_id != player {
            return Err(Error::Unsupported("Game.Update AI Control receiver"));
        }
        Ok(control.has_out_of_turn_processing)
    }
}

struct PlayerState<'a> {
    state: &'a mut LoadedState,
}
impl PlayerState<'_> {
    fn index(id: i32) -> Result<usize, Error> {
        usize::try_from(id)
            .ok()
            .filter(|&i| i < 64)
            .ok_or(Error::Unsupported("Game.Update player counter index"))
    }
    fn revision(&mut self) -> Result<(), Error> {
        self.state.revision = self
            .state
            .revision
            .checked_add(1)
            .ok_or(Error::Unsupported("loaded revision exhausted"))?;
        Ok(())
    }
}
impl core::PlayerWorld for PlayerState<'_> {
    type Player = i32;
    fn active(&mut self, player: i32) -> Result<bool, Error> {
        self.state.turn_active(player).map_err(missing)
    }
    fn id(&mut self, player: i32) -> Result<i32, Error> {
        self.state.world.player(player).map_err(missing)?;
        Ok(player)
    }
    fn activation_event(&mut self, id: i32) -> Result<bool, Error> {
        Ok(self.state.retained.core.player_processor.activation_event[Self::index(id)?])
    }
    fn first_activation(&mut self, id: i32) -> Result<bool, Error> {
        Ok(self.state.retained.core.player_processor.first_activation[Self::index(id)?])
    }
    fn set_first_activation(&mut self, id: i32, value: bool) -> Result<(), Error> {
        let i = Self::index(id)?;
        self.revision()?;
        self.state.retained.core.player_processor.first_activation[i] = value;
        Ok(())
    }
    fn set_activation_event(&mut self, id: i32, value: bool) -> Result<(), Error> {
        let i = Self::index(id)?;
        self.revision()?;
        self.state.retained.core.player_processor.activation_event[i] = value;
        Ok(())
    }
    fn human(&mut self, player: i32) -> Result<bool, Error> {
        Ok(self.state.player_slot_status(player).map_err(missing)? == 3)
    }
    fn phase_updates(&mut self, id: i32) -> Result<u32, Error> {
        Ok(self.state.retained.core.player_processor.updates_this_phase[Self::index(id)?])
    }
    fn set_phase_updates(&mut self, id: i32, value: u32) -> Result<(), Error> {
        let i = Self::index(id)?;
        self.revision()?;
        self.state.retained.core.player_processor.updates_this_phase[i] = value;
        Ok(())
    }
    fn turn_updates(&mut self, id: i32) -> Result<u32, Error> {
        Ok(self.state.retained.core.player_processor.updates_this_turn[Self::index(id)?])
    }
    fn set_turn_updates(&mut self, id: i32, value: u32) -> Result<(), Error> {
        let i = Self::index(id)?;
        self.revision()?;
        self.state.retained.core.player_processor.updates_this_turn[i] = value;
        Ok(())
    }
    fn ai_updates(&mut self, id: i32) -> Result<u32, Error> {
        Ok(self
            .state
            .retained
            .core
            .player_processor
            .ai_updates_this_turn[Self::index(id)?])
    }
    fn set_ai_updates(&mut self, id: i32, value: u32) -> Result<(), Error> {
        let i = Self::index(id)?;
        self.revision()?;
        self.state
            .retained
            .core
            .player_processor
            .ai_updates_this_turn[i] = value;
        Ok(())
    }
    fn set_blocker(&mut self, player: i32, value: i32) -> Result<(), Error> {
        self.state
            .store_player_end_turn_blocking(player, value)
            .map_err(missing)?;
        Ok(())
    }
}

/// Advance the actual next selected player through the supported scalar body.
/// None means the DEACTIVATION phase is required, not complete Game.Update.
/// Native outer-loop human/alive facts are captured before Player.Update and
/// retained across its callbacks; actual mutable player fields are never copied.
pub fn prepare_current_player(
    state: &mut LoadedState,
    runtime: &mut Runtime,
) -> Result<Option<core::PlayerEffect<i32>>, Error> {
    if state.binding() != &runtime.binding || runtime.failed || runtime.player_frame.is_some() {
        return Err(Error::Unsupported(
            "Game.Update player continuation pending or failed",
        ));
    }
    let player = match &runtime.pending {
        Some(core::Continuation::PlayerUpdates(ids)) => ids.get(runtime.player_cursor).copied(),
        Some(core::Continuation::BeginTurn) => {
            return Err(Error::Unsupported(
                "Game.Update requires Game.BeginTurn upkeep",
            ))
        }
        None => {
            return Err(Error::Unsupported(
                "Game.Update prefix has not prepared a player pass",
            ))
        }
    };
    let Some(player) = player else {
        return Ok(None);
    };
    let result = (|| {
        state.world.player(player).map_err(missing)?;
        let next_has_human =
            state.player_slot_status(player).map_err(missing)? == 3 || runtime.pass_has_human;
        let alive_nonhuman = if state.player_alive(player).map_err(missing)? {
            state.player_slot_status(player).map_err(missing)? != 3
        } else {
            false
        };
        runtime.player_frame = Some(PlayerFrame {
            player,
            effect: None,
            next_has_human,
            alive_nonhuman,
        });
        let effect = core::prepare_player(&mut PlayerState { state }, player)?;
        runtime.player_frame.as_mut().expect("installed").effect = Some(effect);
        Ok(Some(effect))
    })();
    if result.is_err() {
        runtime.failed = true;
    }
    result
}

/// Resume Player.Update after the actual activation upkeep and activation
/// report returned. Human DoTurn then returns without a policy callback.
pub fn complete_activation(
    state: &mut LoadedState,
    runtime: &mut Runtime,
) -> Result<core::PlayerEffect<i32>, Error> {
    if state.binding() != &runtime.binding || runtime.failed {
        return Err(Error::Unsupported(
            "Game.Update activation binding or failure",
        ));
    }
    let frame = runtime
        .player_frame
        .as_ref()
        .ok_or(Error::Unsupported("Game.Update no activation continuation"))?;
    if !matches!(frame.effect, Some(core::PlayerEffect::Activation { player, .. }) if player == frame.player)
    {
        return Err(Error::Unsupported(
            "Game.Update activation continuation ordering",
        ));
    }
    let player = frame.player;
    if state.player_slot_status(player).map_err(missing)? != 3 {
        let effect = core::PlayerEffect::ActiveControl { player };
        runtime
            .player_frame
            .as_mut()
            .expect("retained activation")
            .effect = Some(effect);
        return Ok(effect);
    }
    let effect = core::after_turn_manager(&mut PlayerState { state }, player)?;
    runtime
        .player_frame
        .as_mut()
        .expect("retained activation")
        .effect = Some(effect);
    Ok(effect)
}

/// Resume only after the actual active nonhuman Control.DoTurn returned.
pub fn complete_active_control(
    state: &mut LoadedState,
    runtime: &mut Runtime,
) -> Result<core::PlayerEffect<i32>, Error> {
    if state.binding() != &runtime.binding || runtime.failed {
        return Err(Error::Unsupported(
            "Game.Update active control binding or failure",
        ));
    }
    let frame = runtime.player_frame.as_ref().ok_or(Error::Unsupported(
        "Game.Update no active control continuation",
    ))?;
    if frame.effect
        != Some(core::PlayerEffect::ActiveControl {
            player: frame.player,
        })
    {
        return Err(Error::Unsupported(
            "Game.Update active control continuation ordering",
        ));
    }
    let player = frame.player;
    let effect = core::after_turn_manager(&mut PlayerState { state }, player)?;
    runtime
        .player_frame
        .as_mut()
        .expect("retained active control")
        .effect = Some(effect);
    Ok(effect)
}

/// Inactive Player.Update returns directly from Control.DoTurn to profiling.
/// It does not increment turn counters or run the active blocking update.
pub fn complete_inactive_player(
    state: &LoadedState,
    runtime: &mut Runtime,
) -> Result<PlayerPassProgress, Error> {
    if state.binding() != &runtime.binding || runtime.failed {
        return Err(Error::Unsupported(
            "Game.Update inactive continuation binding or failure",
        ));
    }
    let frame = runtime
        .player_frame
        .as_ref()
        .ok_or(Error::Unsupported("Game.Update no inactive continuation"))?;
    if frame.effect
        != Some(core::PlayerEffect::InactiveControl {
            player: frame.player,
        })
    {
        return Err(Error::Unsupported(
            "Game.Update inactive continuation ordering",
        ));
    }
    runtime.player_cursor = runtime
        .player_cursor
        .checked_add(1)
        .ok_or(Error::Unsupported("Game.Update player cursor capacity"))?;
    runtime.pass_has_human = frame.next_has_human;
    runtime.pass_processed_ai |= frame.alive_nonhuman;
    runtime.player_frame = None;
    Ok(runtime.player_progress())
}

/// Invoke ONLY after the SAME current notification provider completed the
/// required blocking update. Current human tail: SetDisabled(true) writes the
/// local stopwatch+41, End returns before logging, destructor changes only
/// profiler nesting. No gameplay/RNG is omitted in this source-bound branch.
/// A changed control role requires its own remaining continuation.
pub fn complete_current_player(
    state: &LoadedState,
    runtime: &mut Runtime,
) -> Result<PlayerPassProgress, Error> {
    if state.binding() != &runtime.binding || runtime.failed {
        return Err(Error::Unsupported(
            "Game.Update current player binding or failure",
        ));
    }
    let frame = runtime
        .player_frame
        .as_ref()
        .ok_or(Error::Unsupported("Game.Update no current player callback"))?;
    if frame.effect
        != Some(core::PlayerEffect::Blocking {
            player: frame.player,
        })
    {
        return Err(Error::Unsupported(
            "Game.Update blocking callback has not been reached",
        ));
    }
    // Fresh IsHuman2b5c7bd only selects stopwatch formatting/disabled state.
    // It does not prevent the nonhuman return after blocking gameplay.
    state.player_slot_status(frame.player).map_err(missing)?;
    runtime.player_cursor = runtime
        .player_cursor
        .checked_add(1)
        .ok_or(Error::Unsupported("Game.Update player cursor capacity"))?;
    runtime.pass_has_human = frame.next_has_human;
    runtime.pass_processed_ai |= frame.alive_nonhuman;
    runtime.player_frame = None;
    Ok(runtime.player_progress())
}

/// Refine only previously unread turn-activity facts from the SAME observed
/// initial snapshot, before any gameplay mutation. Existing facts must agree;
/// decoder fields, other controls and source identity are never overwritten.
/// Caller-supplied values still require an attributed actual before readset.
pub fn extend_observed_turn_activity(
    state: &mut LoadedState,
    witness: &super::ObservedSnapshotWitness,
    values: &[(i32, bool)],
) -> Result<(), super::Error> {
    if !state.is_pristine() {
        return Err(super::Error::NotPristine);
    }
    if state.observed_snapshot.as_ref() != Some(witness) {
        return Err(super::Error::Binding);
    }
    let controls = state.controls.as_ref().ok_or(super::Error::Binding)?;
    let mut positions = Vec::new();
    for (n, (owner, active)) in values.iter().enumerate() {
        if values[..n].iter().any(|(other, _)| other == owner) {
            return Err(super::Error::Binding);
        }
        let index = controls
            .players
            .iter()
            .position(|p| p.owner == *owner)
            .ok_or(super::Error::Missing(super::MissingInput::Player(*owner)))?;
        match controls.players[index].turn_active {
            Some(old) if old != *active => return Err(super::Error::Binding),
            Some(_) => {}
            None => positions.push((index, *active)),
        }
    }
    if !positions.is_empty() {
        state.advance_admission_generation()?;
        let controls = state.controls.as_mut().expect("checked");
        for (index, active) in positions {
            controls.players[index].turn_active = Some(active);
        }
    }
    Ok(())
}

/// Install actual current symmetric contact cells on the existing canonical
/// relation owner. No missing relation matrix or saved prefix is synthesized.
pub fn install_current_game_contacts(
    state: &mut LoadedState,
    witness: &crate::loaded_state::ObservedSnapshotWitness,
    player_count: i32,
    contacts: Vec<crate::game_diplomatic_relations::BoolRelationship>,
) -> Result<(), crate::loaded_state::Error> {
    use crate::loaded_state::Error;
    if !state.is_pristine() || state.observed_snapshot() != Some(witness) {
        return Err(Error::Binding);
    }
    let mut owner = match &state.game_relations {
        Some(existing) => existing.clone(),
        None => crate::game_diplomatic_relations::GameDiplomaticRelations::from_current_contacts(
            player_count,
            contacts.clone(),
        )
        .map_err(Error::GameRelations)?,
    };
    owner
        .initialize_current_contacts(player_count, contacts)
        .map_err(Error::GameRelations)?;
    state.advance_admission_generation()?;
    state.game_relations = Some(owner);
    Ok(())
}

#[cfg(test)]
mod inactive_tests {
    use super::*;
    #[test]
    fn nonhuman_activation_waits_for_real_control_before_counters() {
        let mut state = crate::loaded_state::tests::state();
        let owner = state.world.players()[0].owner();
        let mut mapping = [-1; 65];
        mapping[owner as usize] = 0;
        let mut slots = [false; 65];
        slots[owner as usize] = true;
        state
            .bind_controls(crate::loaded_state::ControlInputs {
                binding: state.binding().clone(),
                player_mapping: mapping,
                player_slots: slots,
                players: vec![crate::loaded_state::PlayerControls {
                    owner,
                    slot_status: 2,
                    team: 0,
                    is_alive: true,
                    turn_active: true,
                }],
                processing_automation: None,
                pause_storage: None,
            })
            .unwrap();
        let mut runtime = Runtime::new(
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
        runtime.pending = Some(core::Continuation::PlayerUpdates(vec![owner]));
        runtime.player_frame = Some(PlayerFrame {
            player: owner,
            effect: Some(core::PlayerEffect::Activation {
                player: owner,
                first: true,
            }),
            next_has_human: false,
            alive_nonhuman: true,
        });
        let before = state.retained.core.player_processor.clone();
        assert_eq!(
            complete_activation(&mut state, &mut runtime).unwrap(),
            core::PlayerEffect::ActiveControl { player: owner }
        );
        assert_eq!(state.retained.core.player_processor, before);
        assert_eq!(
            complete_active_control(&mut state, &mut runtime).unwrap(),
            core::PlayerEffect::Blocking { player: owner }
        );
        let i = owner as usize;
        assert_eq!(
            state.retained.core.player_processor.updates_this_turn[i],
            before.updates_this_turn[i].wrapping_add(1)
        );
        assert_eq!(
            state.retained.core.player_processor.ai_updates_this_turn[i],
            before.ai_updates_this_turn[i].wrapping_add(1)
        );
        let after_control = state.retained.core.player_processor.clone();
        assert!(complete_active_control(&mut state, &mut runtime).is_err());
        complete_current_player(&state, &mut runtime).unwrap();
        assert_eq!(state.retained.core.player_processor, after_control);
        assert_eq!(runtime.player_progress().completed, 1);
    }
    #[test]
    fn inactive_return_advances_pass_without_active_player_counter_or_blocking_writes() {
        let state = crate::loaded_state::tests::state();
        let before = state.retained.core.player_processor.clone();
        let mut runtime = Runtime::new(
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
        runtime.pending = Some(core::Continuation::PlayerUpdates(vec![0]));
        runtime.player_frame = Some(PlayerFrame {
            player: 0,
            effect: Some(core::PlayerEffect::InactiveControl { player: 0 }),
            next_has_human: true,
            alive_nonhuman: true,
        });
        let progress = complete_inactive_player(&state, &mut runtime).unwrap();
        assert_eq!(progress.completed, 1);
        assert!(progress.has_human && progress.processed_ai);
        assert_eq!(state.retained.core.player_processor, before);
        assert!(runtime.pending_player_effect().is_none());
        assert!(complete_inactive_player(&state, &mut runtime).is_err());
        assert_eq!(runtime.player_progress().completed, 1);
    }
}
