//! Exclusive canonical CityBuild manager access for normal City initialization.
use super::LoadedState;
use crate::{city_build_initialization::Error, player_ai_city_manager::CityManager};
impl LoadedState {
    /// AI.Trade.City.StartTradeRouteFrom21b5020 updates its bookkeeping even
    /// when all player policy decisions are supplied externally.
    pub fn observe_city_trade_route_started(
        &mut self,
        owner: i32,
        city_ai_id: u32,
        origin: crate::trade::ComponentId,
    ) -> Result<(), Error> {
        let manager = self
            .retained
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .and_then(|p| p.ai_city_manager.as_ref())
            .ok_or(Error::MissingManager(owner))?;
        let city = manager
            .city_ais
            .entries
            .iter()
            .find(|e| e.city_build.city_ai_id == city_ai_id)
            .ok_or(Error::Owner)?;
        let id = &city.city_build.city_trade.city;
        if (id.owner, id.component_type, id.id) != (origin.owner, origin.component_type, origin.id)
        {
            return Ok(());
        }
        let manager = self.city_build_manager_mut(owner)?;
        let city = manager
            .city_ais
            .entries
            .iter_mut()
            .find(|e| e.city_build.city_ai_id == city_ai_id)
            .unwrap();
        city.city_build.city_trade.current_routes =
            city.city_build.city_trade.current_routes.wrapping_add(1);
        Ok(())
    }
    /// Mark native production analyses unavailable when production decisions
    /// are supplied externally. This changes no controller or contract owner.
    pub fn omit_city_build_planning(&mut self, owner: i32) -> Result<(), Error> {
        use crate::player_ai_city_manager::PlanningData;
        let manager = self.city_build_manager_mut(owner)?;
        for entry in &mut manager.city_ais.entries {
            entry.city_build.planning_data = PlanningData::ExternalPolicyOmitted;
        }
        Ok(())
    }
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
    fn external_city_planning_is_unavailable_without_mutating_contract_or_tree_references() {
        use crate::player_ai_city_manager::{
            CityBuild, CityBuildEntry, CityManager, PlanningData, RecycledCityBuilds,
        };
        let mut state = super::super::tests::state();
        let city = CityBuild {
            event_trees: vec![7, u32::MAX],
            ..Default::default()
        };
        let contract = city.managed_working_contract.clone();
        state.retained.players[0].ai_city_manager = Some(CityManager {
            player_id: 0,
            city_ais: RecycledCityBuilds {
                entries: vec![CityBuildEntry {
                    index: 0,
                    city_build: city,
                }],
                ..Default::default()
            },
        });
        state.omit_city_build_planning(0).unwrap();
        let city = &state.retained.players[0]
            .ai_city_manager
            .as_ref()
            .unwrap()
            .city_ais
            .entries[0]
            .city_build;
        assert_eq!(city.planning_data, PlanningData::ExternalPolicyOmitted);
        assert!(city.require_retained_planning().is_err());
        assert_eq!(city.event_trees, [7, u32::MAX]);
        assert_eq!(city.managed_working_contract, contract);
    }
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
