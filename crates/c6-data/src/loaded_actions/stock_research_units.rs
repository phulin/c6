//! Positive research completion reuses the persistent movement services.
use super::{Error, Runtime};
use c6_core::loaded_state::LoadedState;
impl<'a, P> Runtime<'a, P> {
    /// Borrow immutable rules once; every update reads current Unit/Formation
    /// state and dispatches through this same reporting/effects owner.
    pub fn bind_research_unit_updates(
        &mut self,
        state: &LoadedState,
        definitions: &'a crate::unit_movement_state::Definitions,
    ) -> Result<(), super::processing::Error> {
        self.check(state)?;
        if self.research_unit_updates.is_some() {
            return Err(super::processing::Error::Unsupported(
                "research unit definitions already bound",
            ));
        }
        self.research_unit_updates = Some(definitions);
        Ok(())
    }
    pub(super) fn update_research_units(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
    ) -> Result<(), Error> {
        let effect = crate::research::Effect::UpdateUnitsForTech(owner);
        let units = state
            .world()
            .player(owner)
            .and_then(|p| p.unit_records())
            .map_err(|_| Error::Effect(effect.clone()))?;
        let mut any = false;
        for unit in units {
            if !unit.value.b_delayed_death
                && state
                    .unit_runtime(owner, unit.value.id)
                    .map_err(|_| Error::Effect(effect.clone()))?
                    .formation
                    .is_some()
            {
                any = true;
                break;
            }
        }
        if any {
            let definitions = self.research_unit_updates.ok_or(Error::Effect(effect))?;
            crate::research::units::update_formations(
                state,
                owner,
                &crate::unit_movement_state::Inputs { definitions },
                self,
            )
            .map_err(Error::UnitUpdates)?;
        }
        self.reached
            .push("stock research current formation updates");
        Ok(())
    }
}
