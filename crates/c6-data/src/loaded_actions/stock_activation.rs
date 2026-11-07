//! Additional stock service contracts used by the existing unit activation.
//! Equal attack counts and empty operation queues require no absent callbacks.
use super::{movement::Error, Runtime};
use crate::loaded_upkeep::{self, UnitHandle};
use c6_core::{
    loaded_state::LoadedState, unit_turn_upkeep::AttacksRemainingReport,
    visibility_change::SharedLayers,
};
impl<P> loaded_upkeep::AttacksReports for Runtime<'_, P> {
    type Error = Error;
    fn preflight_attacks_report(
        &self,
        state: &LoadedState,
        actor: UnitHandle,
    ) -> Result<(), Error> {
        if self.binding != *state.binding() {
            return Err(Error::Missing("activation attacks state identity"));
        }
        actor.get(state).map_err(Error::World)?;
        if self.movement()?.reporting_suspended {
            return Ok(());
        }
        self.reporting
            .preflight_attacks_changed(state)
            .map_err(Error::Rules)
    }
    fn attacks_report(
        &mut self,
        state: &mut LoadedState,
        report: AttacksRemainingReport,
    ) -> Result<(), Error> {
        if self.binding != *state.binding() {
            return Err(Error::Missing("activation attacks state identity"));
        }
        if self.movement()?.reporting_suspended {
            return Ok(());
        }
        self.reporting
            .attacks_changed(state, report)
            .map_err(Error::Rules)
    }
}
impl<P> loaded_upkeep::AlertVisibility for Runtime<'_, P> {
    type Error = Error;
    fn shared_layers(&self, _: &LoadedState, _: i32) -> Result<&SharedLayers, Error> {
        Err(Error::Missing("activation Alert current shared visibility"))
    }
    fn grid_width(&self, state: &LoadedState, owner: i32) -> Result<u32, Error> {
        if self.binding != *state.binding() {
            return Err(Error::Missing("activation state identity"));
        }
        self.player_services
            .as_ref()
            .ok_or(Error::Missing("activation player services"))?
            .player::<Error>(owner)
            .map_err(|e| Error::Requirement(format!("{e:?}")))?
            .native_visibility_width
            .ok_or(Error::Missing("activation native visibility width"))
    }
    fn hidden_layer0(&self, _: &LoadedState) -> Result<u32, Error> {
        Err(Error::Missing("activation Alert hidden layer definition"))
    }
}
