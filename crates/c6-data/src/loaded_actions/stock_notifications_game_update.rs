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
    fn notify_turn_request(&mut self, state: &mut LoadedState, event: u32) -> Result<(), Error> {
        if self.binding != *state.binding() {
            return Err(Error::Unsupported("stock turn-request state binding"));
        }
        match event {
            c6_core::game_update::AI_PROCESSED
            | c6_core::game_update::INITIAL_AI_PROCESSED
            | c6_core::game_update::LOCAL_COMPUTER_PROCESSING => {
                // The pinned Linux stock utility address point 0x53a6c90 has
                // slot +0x70 -> host 0x32b4cf0 SendMessage(unsigned int). Its entire
                // body is XOR EAX,EAX; RET: no dispatch, host queue or RNG.
                // The adjacent MemoryOStream overload 0x32b4d00 is a different
                // slot and does broadcast; it is not reached by these calls.
                // Core callers retain their original last-processed writes
                // before/after this return, including their distinct ordering.
                self.reached.push("stock turn-request integer sink returned");
                Ok(())
            }
            _ => Err(Error::Unsupported("stock unknown turn-request event")),
        }
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
