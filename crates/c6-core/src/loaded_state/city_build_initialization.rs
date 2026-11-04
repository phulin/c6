//! Exclusive canonical CityBuild manager access for normal City initialization.
use super::LoadedState;
use crate::{city_build_initialization::Error, player_ai_city_manager::CityManager};
impl LoadedState {
    pub(crate) fn city_build_manager_mut(&mut self, owner: i32) -> Result<&mut CityManager, Error> {
        let index = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(Error::MissingManager(owner))?;
        let manager = self.retained.players[index]
            .ai_city_manager
            .as_ref()
            .ok_or(Error::MissingManager(owner))?;
        if manager.player_id != owner {
            return Err(Error::Owner);
        }
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        Ok(self.retained.players[index]
            .ai_city_manager
            .as_mut()
            .unwrap())
    }
}

#[cfg(test)]
mod tests {
    use crate::city_build_initialization::{create, Error, Inputs};
    #[test]
    fn uninitialized_guard_and_wrong_state_binding_do_not_mutate() {
        let mut state = super::super::tests::state();
        let inputs = Inputs::bind(&state, 0, false);
        let stamp = state.action_stamp();
        assert_eq!(create(&mut state, 0, u32::MAX, &inputs).unwrap(), None);
        assert_eq!(state.action_stamp(), stamp);
        let mut other = super::super::tests::state();
        let stamp = other.action_stamp();
        assert!(matches!(
            create(&mut other, 0, 0, &inputs),
            Err(Error::Binding)
        ));
        assert_eq!(other.action_stamp(), stamp);
        assert!(matches!(
            create(&mut state, 1, 0, &inputs),
            Err(Error::Binding)
        ));
    }
}
