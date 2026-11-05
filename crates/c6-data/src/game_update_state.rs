//! Partial Game.Update services using the current shared player-list owner and
//! existing concrete UnitCounts queries. No complete end-turn support implied.
pub mod automation;
pub mod deactivation;
pub mod activation;
pub mod round_units;
pub mod round_cities;
pub mod round_trade;
pub mod round_climate;
pub mod round_map;
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

#[derive(Debug)]
pub enum PlayerStepError {
    Prefix(Error),
    Notifications(c6_core::unit_operation_aftermath::Error),
}
impl std::fmt::Display for PlayerStepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "current Player.Update continuation: {self:?}")
    }
}
impl std::error::Error for PlayerStepError {}

/// This outcome covers one selected Player.Update, not Game.Update completion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayerProgress {
    MoreSelectedPlayers(loaded::PlayerPassProgress),
    RequiresDeactivation(loaded::PlayerPassProgress),
}

/// Continue using the SAME persistent notification provider/backend/ScriptState.
/// Failed allocation, reporting or subtype callbacks remain in that provider;
/// there is no temporary backend, copied state or dropped pending candidate.
/// Activation/AI-control callbacks remain unsupported before their actual body,
/// with the exact preceding flag/counter mutations retained for inspection.
pub fn advance_current_player<B: crate::loaded_actions::notifications::processing::Backend>(
    notifications: &mut crate::loaded_actions::notifications::processing::Provider<'_, B>,
    runtime: &mut Runtime,
    budget: &mut usize,
) -> Result<PlayerProgress, PlayerStepError> {
    let effect = loaded::prepare_current_player(notifications.state, runtime)
        .map_err(PlayerStepError::Prefix)?;
    let Some(effect) = effect else {
        return Ok(PlayerProgress::RequiresDeactivation(
            runtime.player_progress(),
        ));
    };
    let owner = match effect {
        c6_core::game_update::PlayerEffect::Blocking { player } => player,
        c6_core::game_update::PlayerEffect::Activation { .. } => {
            return Err(PlayerStepError::Prefix(Error::Unsupported(
                "Player.Update requires activation upkeep and reporting",
            )))
        }
        c6_core::game_update::PlayerEffect::ActiveControl { .. } => {
            return Err(PlayerStepError::Prefix(Error::Unsupported(
                "Player.Update requires active mixed AI Control work",
            )))
        }
        c6_core::game_update::PlayerEffect::InactiveControl { .. } => {
            return Err(PlayerStepError::Prefix(Error::Unsupported(
                "Player.Update requires inactive off-turn Control work",
            )))
        }
    };
    if let Err(error) = notifications.update(owner, budget) {
        runtime.fail_player_callback();
        return Err(PlayerStepError::Notifications(error));
    }
    let progress = match loaded::complete_current_player(notifications.state, runtime) {
        Ok(progress) => progress,
        Err(error) => {
            runtime.fail_player_callback();
            return Err(PlayerStepError::Prefix(error));
        }
    };
    Ok(if progress.completed == progress.selected {
        PlayerProgress::RequiresDeactivation(progress)
    } else {
        PlayerProgress::MoreSelectedPlayers(progress)
    })
}

pub mod round_random_events;
