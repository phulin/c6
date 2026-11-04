//! Current-state partial Game.Update over canonical saved processor/control data.
//! Unsaved globals are explicit live inputs; no defaults or duplicate saved state.
use super::LoadedState;
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
}
impl Runtime {
    /// Binding checks identity, not provenance of caller-supplied runtime facts.
    pub fn new(state: &LoadedState, inputs: Inputs) -> Self {
        Self {
            binding: state.binding().clone(),
            inputs,
            pending: None,
            failed: false,
        }
    }
    pub fn pending(&self) -> Option<&core::Continuation> {
        self.pending.as_ref()
    }
    pub fn failed(&self) -> bool {
        self.failed
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
