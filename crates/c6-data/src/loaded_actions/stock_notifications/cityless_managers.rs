//! Same stock owner for the reached activation manager effects.
use super::Runtime;
use crate::game_update_state::activation::{cityless_managers as data, maintenance};
use c6_core::{game_update::Error, loaded_state::LoadedState};
impl<P> data::Effects for Runtime<'_, P> {
    fn report_block(
        &mut self,
        state: &mut LoadedState,
        hash: u32,
        _begin: bool,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("activation resource report binding"))?;
        maintenance::stock_fuel_report_block(hash).map_err(Error::Unsupported)
    }
    fn diplomacy_tracker_changed(
        &mut self,
        state: &mut LoadedState,
        _owner: i32,
        _other: i32,
        _key: u32,
        _old: i32,
        _new: i32,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("diplomacy tracker report binding"))?;
        Err(Error::Unsupported("changed diplomacy tracker logging"))
    }
    fn congress_points_changed(
        &mut self,
        state: &mut LoadedState,
        _owner: i32,
        _delta: i32,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("Congress points report binding"))?;
        Err(Error::Unsupported("Congress points changed notification"))
    }
    fn notification(
        &mut self,
        state: &mut LoadedState,
        _owner: i32,
        _kind: u32,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("activation notification binding"))?;
        Err(Error::Unsupported(
            "activation manager positive notification",
        ))
    }
}
