//! Game.EndTurn/BeginTurn using the persistent stock player and world owners.
use super::{round_end, round_world, Runtime};
use c6_core::{
    game_update::Error,
    loaded_state::{
        game_update::{self as loaded, round},
        LoadedState,
    },
};

/// Synchronous processor signals and the queued completion message retain their
/// native ordering. The completion broadcast must not execute reentrantly.
pub trait Effects<P>: round_world::Effects<P> {
    fn report(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        event: u32,
        value: i32,
    ) -> Result<(), Error>;
    fn queue_game_turn_complete(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
    ) -> Result<(), Error>;
    fn turn_signal(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        begin: bool,
        turn: i32,
    ) -> Result<(), Error>;
    fn activate_at_turn_start(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
    ) -> Result<(), Error>;
    fn autoplay_end(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        runtime: &mut loaded::Runtime,
    ) -> Result<(), Error>;
}

pub struct Current<'a, 'b, 'd, 'r, P, E> {
    pub world: round_world::Current<'a, 'b, P, E>,
    pub player_end: &'a round_end::Inputs<'a, 'd, 'r>,
}
impl<P, E: Effects<P>> round::TurnEffects for Current<'_, '_, '_, '_, P, E> {
    fn report(&mut self, state: &mut LoadedState, event: u32, value: i32) -> Result<(), Error> {
        self.world
            .effects
            .report(state, self.world.backend, event, value)
    }
    fn player_begin(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut loaded::Runtime,
        owner: i32,
    ) -> Result<(), Error> {
        self.world.backend.begin_round_ai(state, runtime, owner)
    }
    fn player_end(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Error> {
        self.world
            .backend
            .end_player_round(state, self.player_end, owner)
            .map_err(|e| {
                self.world.backend.last_error = Some(e.to_string());
                Error::Unsupported("player round end continuation")
            })
    }
    fn player_end_tail(&mut self, state: &mut LoadedState) -> Result<(), Error> {
        self.world
            .effects
            .queue_game_turn_complete(state, self.world.backend)
    }
    fn world_begin(&mut self, state: &mut LoadedState) -> Result<(), Error> {
        round::xp2_begin_world(state, &mut self.world)
    }
    fn turn_signal(
        &mut self,
        state: &mut LoadedState,
        begin: bool,
        turn: i32,
    ) -> Result<(), Error> {
        self.world
            .effects
            .turn_signal(state, self.world.backend, begin, turn)
    }
    fn activate_at_turn_start(&mut self, state: &mut LoadedState) -> Result<(), Error> {
        self.world
            .effects
            .activate_at_turn_start(state, self.world.backend)
    }
    fn autoplay_end(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut loaded::Runtime,
    ) -> Result<(), Error> {
        self.world
            .effects
            .autoplay_end(state, self.world.backend, runtime)
    }
}
