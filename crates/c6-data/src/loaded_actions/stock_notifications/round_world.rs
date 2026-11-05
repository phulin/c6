//! XP2 world upkeep over the existing climate, trade and action owners.
use super::Runtime;
use crate::game_update_state::{
    activation::cityless_managers, round_climate, round_random_events, round_trade,
};
use c6_core::{
    game_capabilities::Membership,
    game_update::Error,
    loaded_state::{
        game_update::round::{self, WorldManager},
        LoadedState,
    },
};

/// Remaining map and world-manager callbacks stay at their native positions.
/// Implementations must retain actual gameplay/RNG effects, or return an error.
pub trait Effects<P> {
    fn map_component_count(&mut self, state: &LoadedState) -> Result<u32, Error>;
    fn map_component_turn(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        index: u32,
    ) -> Result<(), Error>;
    fn manager_turn(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        manager: WorldManager,
    ) -> Result<(), Error>;
}

pub struct Current<'a, 'b, P, E> {
    pub backend: &'a mut Runtime<'b, P>,
    pub climate: &'a mut round_climate::Runtime,
    pub cityless: &'a cityless_managers::Runtime,
    pub carbon: round_climate::CarbonRules<'a>,
    pub alive_major_ids: &'a [i32],
    pub trade: &'a mut round_trade::Runtime,
    pub random_definitions: &'a round_random_events::Definitions,
    pub random: &'a mut round_random_events::Runtime,
    pub capabilities: &'a Membership,
    pub effects: &'a mut E,
}

impl<P> round_trade::Effects for Runtime<'_, P> {
    fn temporary_routes_cleared(&mut self, _: &mut LoadedState) -> Result<(), round_trade::Error> {
        Err(round_trade::Error::Requires(
            "temporary-route-cleared signal consumers",
        ))
    }
}

impl<P, E: Effects<P>> round::WorldEffects for Current<'_, '_, P, E> {
    fn map_component_count(&mut self, state: &LoadedState) -> Result<u32, Error> {
        self.effects.map_component_count(state)
    }
    fn map_component_turn(&mut self, state: &mut LoadedState, index: u32) -> Result<(), Error> {
        self.effects.map_component_turn(state, self.backend, index)
    }
    fn manager_turn(
        &mut self,
        state: &mut LoadedState,
        manager: WorldManager,
    ) -> Result<(), Error> {
        let result = match manager {
            WorldManager::Climate => round_climate::do_turn(
                state,
                self.climate,
                self.cityless,
                self.alive_major_ids,
                state.retained().core.current_turn,
                &self.carbon,
            )
            .map_err(|e| format!("{e:?}")),
            WorldManager::Trade => {
                round_trade::run(state, self.trade, self.backend).map_err(|e| format!("{e:?}"))
            }
            WorldManager::RandomEvents => round_random_events::run(
                state,
                self.random_definitions,
                self.random,
                &mut round_random_events::CurrentClimate {
                    runtime: self.climate,
                    cityless: self.cityless,
                },
                self.capabilities,
                self.backend,
            )
            .map_err(|e| format!("{e:?}")),
            _ => return self.effects.manager_turn(state, self.backend, manager),
        };
        result.map_err(|detail| {
            self.backend.last_error = Some(format!("world {manager:?}: {detail}"));
            Error::Unsupported("world manager continuation")
        })
    }
}
