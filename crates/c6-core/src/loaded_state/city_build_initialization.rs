//! Exclusive canonical CityBuild manager access for normal City initialization.
use super::LoadedState;
use crate::{city_build_initialization::Error, player_ai_city_manager::CityManager};
impl LoadedState {
    pub(crate) fn city_founding_strategic_parts(
        &mut self,
        owner: i32,
    ) -> Result<
        (
            &crate::map::Map,
            &mut crate::player_ai_grand_strategic::GrandStrategic,
        ),
        crate::city_build_initialization::founding::Error,
    > {
        use crate::city_build_initialization::founding::Error as E;
        let player = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(E::MissingPlayer)?;
        let value = self.retained.players[player]
            .ai_grand_strategic
            .as_ref()
            .ok_or(E::MissingGrandStrategic)?;
        if value.player_id != owner || value.city_planner.player_id != owner {
            return Err(E::Binding);
        }
        self.revision = self.revision.checked_add(1).ok_or(E::Allocation)?;
        Ok((
            self.world.map(),
            self.retained.players[player]
                .ai_grand_strategic
                .as_mut()
                .unwrap(),
        ))
    }
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
