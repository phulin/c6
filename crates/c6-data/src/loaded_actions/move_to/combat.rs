//! Compose the existing MoveTo handler with complete combat on the same state
//! and effects owner. A combat continuation/error cannot become attack success.
use super::{Movement, OperationReports, PlotCoord, Request};
use crate::unit_sight_state;
use c6_core::loaded_state::LoadedState;

pub trait Combat<E: OperationReports> {
    type Error;
    fn test(
        &mut self,
        state: &mut LoadedState,
        sight: &unit_sight_state::Runtime,
        actor: (i32, u32),
        destination: PlotCoord,
        effects: &E,
    ) -> Result<bool, Self::Error>;
    fn attack(
        &mut self,
        state: &mut LoadedState,
        sight: &mut unit_sight_state::Runtime,
        actor: (i32, u32),
        destination: PlotCoord,
        effects: &mut E,
    ) -> Result<bool, Self::Error>;
}
pub struct WithCombat<M, C> {
    pub movement: M,
    pub combat: C,
}
#[derive(Debug)]
pub enum Error<M, C> {
    Movement(M),
    Combat(C),
}
impl<E: OperationReports, M: Movement<E>, C: Combat<E>> Movement<E> for WithCombat<M, C> {
    type Error = Error<M::Error, C::Error>;
    fn found_city_rules(&self) -> Option<(&crate::found_city::Definitions, &crate::Rules)> {
        self.movement.found_city_rules()
    }
    fn start_found_city(
        &mut self,
        state: &mut LoadedState,
        sight: &mut unit_sight_state::Runtime,
        actor: (i32, u32),
        effects: &mut E,
    ) -> Result<Option<c6_core::skip_turn::StartData>, Self::Error> {
        self.movement
            .start_found_city(state, sight, actor, effects)
            .map_err(Error::Movement)
    }
    fn test_attack_into(
        &mut self,
        state: &mut LoadedState,
        sight: &unit_sight_state::Runtime,
        actor: (i32, u32),
        destination: PlotCoord,
        effects: &E,
    ) -> Result<Option<bool>, Self::Error> {
        self.combat
            .test(state, sight, actor, destination, effects)
            .map(Some)
            .map_err(Error::Combat)
    }
    fn attack_into(
        &mut self,
        state: &mut LoadedState,
        sight: &mut unit_sight_state::Runtime,
        actor: (i32, u32),
        destination: PlotCoord,
        effects: &mut E,
    ) -> Result<Option<bool>, Self::Error> {
        self.combat
            .attack(state, sight, actor, destination, effects)
            .map(Some)
            .map_err(Error::Combat)
    }
    fn fortify_turn_max(&self) -> i32 {
        self.movement.fortify_turn_max()
    }
    fn is_fortifyable(&self, state: &LoadedState, actor: (i32, u32)) -> Result<bool, Self::Error> {
        self.movement
            .is_fortifyable(state, actor)
            .map_err(Error::Movement)
    }
    fn alert_enemy_visible(
        &self,
        state: &LoadedState,
        sight: &unit_sight_state::Runtime,
        actor: (i32, u32),
    ) -> Result<bool, Self::Error> {
        self.movement
            .alert_enemy_visible(state, sight, actor)
            .map_err(Error::Movement)
    }
    fn city_at(
        &self,
        state: &LoadedState,
        location: PlotCoord,
    ) -> Result<Option<c6_core::trade::ComponentId>, Self::Error> {
        self.movement
            .city_at(state, location)
            .map_err(Error::Movement)
    }
    fn preflight(
        &self,
        state: &LoadedState,
        actor: (i32, u32),
        request: Request,
    ) -> Result<(), Self::Error> {
        self.movement
            .preflight(state, actor, request)
            .map_err(Error::Movement)
    }
    fn test_move_to(
        &mut self,
        state: &mut LoadedState,
        sight: &unit_sight_state::Runtime,
        formation: (i32, u32),
        destination: PlotCoord,
        flags: u32,
    ) -> Result<i32, Self::Error> {
        self.movement
            .test_move_to(state, sight, formation, destination, flags)
            .map_err(Error::Movement)
    }
    fn move_to(
        &mut self,
        state: &mut LoadedState,
        sight: &mut unit_sight_state::Runtime,
        formation: (i32, u32),
        destination: PlotCoord,
        flags: u32,
        effects: &mut E,
    ) -> Result<i32, Self::Error> {
        self.movement
            .move_to(state, sight, formation, destination, flags, effects)
            .map_err(Error::Movement)
    }
    fn valid_location(
        &mut self,
        state: &LoadedState,
        sight: &unit_sight_state::Runtime,
        actor: (i32, u32),
        location: PlotCoord,
    ) -> Result<i32, Self::Error> {
        self.movement
            .valid_location(state, sight, actor, location)
            .map_err(Error::Movement)
    }
}
