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
        self.change_city_trade_route_count(owner, city_ai_id, origin, 1)
    }
    /// AI.Trade.City.EndTradeRouteFrom21b5060 decrements this same canonical
    /// bookkeeping for the matching origin, independently of policy selection.
    pub fn observe_city_trade_route_ended(
        &mut self,
        owner: i32,
        city_ai_id: u32,
        origin: crate::trade::ComponentId,
    ) -> Result<(), Error> {
        self.change_city_trade_route_count(owner, city_ai_id, origin, -1)
    }
    fn change_city_trade_route_count(
        &mut self,
        owner: i32,
        city_ai_id: u32,
        origin: crate::trade::ComponentId,
        delta: i32,
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
        city.city_build.city_trade.current_routes = city
            .city_build
            .city_trade
            .current_routes
            .wrapping_add(delta);
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
    /// Final recycled removal after the concrete CityBuild destructor. No
    /// subscriptions, contracts or event trees are discarded by this accessor.
    pub fn release_city_build_after_destruction(
        &mut self,
        owner: i32,
        id: u32,
    ) -> Result<(), Error> {
        use crate::{
            map::{Recycled, Slot},
            player_ai_city_manager::CityBuildEntry,
        };
        let manager = self
            .retained
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .and_then(|p| p.ai_city_manager.as_ref())
            .ok_or(Error::MissingManager(owner))?;
        if !manager
            .city_ais
            .entries
            .iter()
            .any(|e| e.city_build.city_ai_id == id)
        {
            return Err(Error::Owner);
        }
        let saved = &mut self.city_build_manager_mut(owner)?.city_ais;
        let mut records = Recycled {
            capacity: saved.capacity,
            entries: std::mem::take(&mut saved.entries)
                .into_iter()
                .map(|e| Slot {
                    index: e.index,
                    value: e.city_build,
                })
                .collect(),
            current_id_bits: saved.current_id_bits,
            free_head: saved.first_free,
            free_tail: saved.last_free,
            free_links: std::mem::take(&mut saved.free_indices),
        };
        let result = crate::city_initialization::release_slot(&mut records, id & 0xffff)
            .map_err(Error::Allocator)
            .and_then(|removed| removed.map(|_| ()).ok_or(Error::Owner));
        saved.entries = records
            .entries
            .into_iter()
            .map(|e| CityBuildEntry {
                index: e.index,
                city_build: e.value,
            })
            .collect();
        saved.first_free = records.free_head;
        saved.last_free = records.free_tail;
        saved.free_indices = records.free_links;
        result
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
    fn final_removal_rejects_stale_id_and_preserves_other_city_builds() {
        use crate::player_ai_city_manager::{
            CityBuild, CityBuildEntry, CityManager, RecycledCityBuilds,
        };
        let mut state = super::super::tests::state();
        state.retained.players[0].ai_city_manager = Some(CityManager {
            player_id: 0,
            city_ais: RecycledCityBuilds {
                capacity: 8,
                entries: vec![
                    CityBuildEntry {
                        index: 0,
                        city_build: CityBuild {
                            city_ai_id: 0x10000,
                            event_trees: vec![99],
                            ..Default::default()
                        },
                    },
                    CityBuildEntry {
                        index: 1,
                        city_build: CityBuild {
                            city_ai_id: 0x20001,
                            ..Default::default()
                        },
                    },
                ],
                current_id_bits: 0x20000,
                first_free: 2,
                last_free: 7,
                free_indices: (2..8).collect(),
            },
        });
        let before = state.revision();
        assert!(state
            .release_city_build_after_destruction(0, 0x30001)
            .is_err());
        assert_eq!(state.revision(), before);
        state
            .release_city_build_after_destruction(0, 0x20001)
            .unwrap();
        let records = &state.retained.players[0]
            .ai_city_manager
            .as_ref()
            .unwrap()
            .city_ais;
        assert_eq!(records.current_id_bits, 0x20000);
        assert_eq!((records.first_free, records.last_free), (1, 7));
        assert_eq!(records.free_indices, [1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(records.entries.len(), 1);
        assert_eq!(records.entries[0].city_build.city_ai_id, 0x10000);
        assert_eq!(records.entries[0].city_build.event_trees, [99]);
    }
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
