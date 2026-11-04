//! Current turn services on the SAME persistent script/reporting backend.
use super::Runtime;
use crate::{game_update_state::Effects, unit_placement_services};
use c6_core::{
    game_update::Error,
    loaded_state::{game_update, LoadedState},
};
impl<P> Runtime<'_, P> {
    /// Retain the current player-list allocation and live service facts. This
    /// moves the one service owner; it never copies World or script state.
    pub fn bind_player_services(
        &mut self,
        state: &LoadedState,
        services: unit_placement_services::Runtime,
    ) -> Result<(), Error> {
        if self.binding != *state.binding()
            || services.binding != *state.binding()
            || !self.script.script().matches(state)
        {
            return Err(Error::Unsupported(
                "stock turn player services state binding",
            ));
        }
        if self.player_services.is_some() {
            return Err(Error::Unsupported(
                "stock turn player services already bound",
            ));
        }
        self.player_services = Some(services);
        Ok(())
    }
}
impl<P> Effects for Runtime<'_, P> {
    fn player_services(&self) -> Result<&unit_placement_services::Runtime, Error> {
        self.player_services
            .as_ref()
            .ok_or(Error::Unsupported("current stock turn player services"))
    }
    fn notify_turn_request(&mut self, _: &mut LoadedState, _: u32) -> Result<(), Error> {
        self.reached.push("Game.Update turn request");
        Err(Error::Unsupported("stock Game.Update turn-request service"))
    }
    fn autoplay_transition(
        &mut self,
        _: &mut LoadedState,
        _: &mut game_update::Runtime,
        _: bool,
    ) -> Result<(), Error> {
        self.reached.push("Game.Update Autoplay transition");
        Err(Error::Unsupported(
            "stock Game.Update Autoplay gameplay transition",
        ))
    }
}

impl<P> c6_core::loaded_state::turn_completion::Effects for Runtime<'_, P> {
    fn log(
        &mut self,
        _: &mut LoadedState,
        _: c6_core::turn_completion::LogBranch,
        _: i32,
    ) -> Result<(), Error> {
        self.reached.push("PlayerTurnComplete log branch");
        Err(Error::Unsupported("stock PlayerTurnComplete log callback"))
    }
    fn report(&mut self, _: &mut LoadedState, _: u32, _: i32) -> Result<(), Error> {
        self.reached.push("PlayerTurnComplete report");
        Err(Error::Unsupported(
            "stock PlayerTurnComplete synchronous report",
        ))
    }
    fn edit_build_queue(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<(), Error> {
        // Native accessor marks its archive before exposing the same queue.
        // The caller performs the subsequent locked/front scalar accesses.
        let _ = state
            .city_mut(owner, city)
            .map_err(|_| Error::Unsupported("completion current city queue archive"))?;
        Ok(())
    }
}
