//! Retained internal round continuations of the current Game.Update.
use super::{deactivation, Runtime};
use c6_core::{
    game_update::Error,
    loaded_state::{game_update as loaded, LoadedState},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Entry {
    AdvancePhase,
    BeginTurn,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Progress {
    PlayerUpdates,
    Returned,
}
#[derive(Clone, Copy)]
pub(crate) enum TurnMessage {
    PlayerComplete { player: i32, reason: u32 },
    GameComplete { player: i32 },
}
/// The bound service borrows current rule/query owners. It receives the same
/// backend and update runtime for every call, including later rounds.
pub trait Services<P> {
    fn advance(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        runtime: &mut loaded::Runtime,
        inputs: &deactivation::Inputs<'_, '_, '_>,
        entry: Entry,
        budget: &mut usize,
    ) -> Result<Progress, Error>;
}
impl<'a, P> Runtime<'a, P> {
    /// Player.Processor.EndTurn broadcasts this message. Its Execute is drained
    /// only after the current parent message returns, using the same FIFO as
    /// externally queued NPC completions.
    pub fn broadcast_game_turn_complete(&mut self, state: &LoadedState) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("GameTurnComplete queue binding"))?;
        self.turn_messages.push_back(TurnMessage::GameComplete {
            player: state.retained().core.local_player,
        });
        Ok(())
    }
    pub fn bind_round_services(
        &mut self,
        state: &LoadedState,
        services: Box<dyn Services<P> + 'a>,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("round backend binding"))?;
        if self.round_services.is_some() {
            return Err(Error::Unsupported("round services already bound"));
        }
        self.round_services = Some(services);
        Ok(())
    }
    pub(crate) fn continue_round(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut loaded::Runtime,
        inputs: &deactivation::Inputs<'_, '_, '_>,
        entry: Entry,
        budget: &mut usize,
    ) -> Result<Progress, Error> {
        let mut services = self
            .round_services
            .take()
            .ok_or(Error::Unsupported("current round services"))?;
        let result = services.advance(state, self, runtime, inputs, entry, budget);
        self.round_services = Some(services);
        result
    }
}
