//! Partial Game.Update services using the current shared player-list owner and
//! existing concrete UnitCounts queries. No complete end-turn support implied.
use crate::{
    loaded_actions::{command_units, reporting},
    unit_placement_services,
};
use c6_core::{
    game_update::Error,
    loaded_state::{
        game_update::{self as loaded, Runtime},
        LoadedState,
    },
};

/// Actual effect environment. Autoplay transitions and EngineUtility+70 may
/// dispatch gameplay/Lua/RNG consumers; there are no successful default stubs.
/// The returned player runtime is the SAME owner used by movement/services.
pub trait Effects {
    fn player_services(&self) -> Result<&unit_placement_services::Runtime, Error>;
    fn notify_turn_request(&mut self, state: &mut LoadedState, event: u32) -> Result<(), Error>;
    fn autoplay_transition(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        pending: bool,
    ) -> Result<(), Error>;
}

pub struct Services<'a, 'q, E> {
    pub readiness: &'a reporting::Provider<'q>,
    pub effects: &'a mut E,
    /// Preserve the concrete current query failure beneath the core boundary.
    pub query_error: Option<crate::loaded_actions::Error>,
}
impl<E: Effects> Services<'_, '_, E> {
    fn player_services(
        &self,
        state: &LoadedState,
    ) -> Result<&unit_placement_services::Runtime, Error> {
        let runtime = self.effects.player_services()?;
        if &runtime.binding != state.binding() {
            return Err(Error::Unsupported(
                "Game.Update current player services binding",
            ));
        }
        Ok(runtime)
    }
}
impl<E: Effects> loaded::Services for Services<'_, '_, E> {
    type Alive = unit_placement_services::AliveToken;
    fn alive_begin(&mut self, state: &LoadedState) -> Result<(Self::Alive, usize), Error> {
        self.player_services(state)?
            .alive_begin::<()>()
            .map_err(|_| Error::Unsupported("Game.Update retained GetAlive storage"))
    }
    fn alive_at(
        &mut self,
        state: &LoadedState,
        list: Self::Alive,
        index: usize,
    ) -> Result<i32, Error> {
        self.player_services(state)?
            .alive_at::<()>(list, index)
            .map_err(|_| Error::Unsupported("Game.Update GetAlive storage changed"))
    }
    fn unavailable_units(&mut self, state: &mut LoadedState, owner: i32) -> Result<i32, Error> {
        // SetInitialAIProcessed2a0b343 reads UnitCounts+8: unavailable,
        // NOT movable_noncycling(+4) or the command_count difference.
        match command_units::processor_unit_counts(state, owner, self.readiness) {
            Ok(counts) => Ok(counts.unavailable),
            Err(error) => {
                self.query_error = Some(error);
                Err(Error::Unsupported("Game.Update current GetUnitCounts"))
            }
        }
    }
    fn notify_turn_request(&mut self, state: &mut LoadedState, event: u32) -> Result<(), Error> {
        self.effects.notify_turn_request(state, event)
    }
    fn autoplay_transition(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        pending: bool,
    ) -> Result<(), Error> {
        self.effects.autoplay_transition(state, runtime, pending)
    }
}

/// Borrow current state/runtime and cached concrete readiness queries. No World,
/// saved processor arrays, player fields or Script/effect owner are cloned.
pub fn context<'a, 'q, E: Effects>(
    state: &'a mut LoadedState,
    runtime: &'a mut Runtime,
    readiness: &'a reporting::Provider<'q>,
    effects: &'a mut E,
) -> Result<loaded::Context<'a, Services<'a, 'q, E>>, Error> {
    loaded::Context::new(
        state,
        runtime,
        Services {
            readiness,
            effects,
            query_error: None,
        },
    )
}
