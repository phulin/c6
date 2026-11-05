//! Same stock Treasury, factory, placement and movement services for purchases.
use super::{Effects, Error};
use crate::{
    loaded_actions::stock_notifications::{self, city_production::unit::Factory},
    unit_creation,
};
use c6_core::{loaded_state::LoadedState, unit::ComponentId};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Report {
    UnitPurchased { city: ComponentId, unit: u32 },
    CommandComplete { city: ComponentId },
}
/// Reporting::Send418bc436/15d11e18 and BaseHandler1d103404 publish City event
/// stream/host presentation. No production/trained signals are invented here.
pub struct Current<'s, 'a, 'd, 'r, P> {
    pub backend: &'s mut stock_notifications::Runtime<'a, P>,
    pub factory: Factory<'s, 'd, 'r>,
    pub reports: &'s mut Vec<Report>,
}
impl<P> Effects for Current<'_, '_, '_, '_, P> {
    type Error = String;
    fn can_move_after_purchase(
        &self,
        state: &LoadedState,
        native_unit_index: usize,
    ) -> Result<bool, String> {
        if self.factory.runtime.binding != *state.binding() {
            return Err("purchase Definition property binding".into());
        }
        let values = self
            .factory
            .runtime
            .definition_properties
            .as_ref()
            .ok_or("purchase current Definition properties")?;
        let map = values
            .iter()
            .find(|v| v.native_unit_index == native_unit_index)
            .ok_or("purchase Definition property map")?;
        if map.values.entries.iter().any(|e| e.key == 0x73edafdc) {
            return Err("CAN_MOVE_AFTER_PURCHASE property conversion".into());
        }
        Ok(false)
    }

    fn gold_changed(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), String> {
        crate::game_update_state::activation::treasury_deals::Effects::gold_changed(
            self.backend,
            state,
            owner,
        )
        .map_err(|e| format!("purchase GoldChanged: {e:?}"))
    }
    fn create_unit(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        unit: u32,
        formation: i32,
    ) -> Result<Option<ComponentId>, String> {
        let c = state
            .city(i32::from(city.owner), city.id)
            .map_err(|e| format!("purchase City: {e:?}"))?;
        let location = c6_core::unit::PlotCoord {
            x: c.header.location_x,
            y: c.header.location_y,
        };
        unit_creation::Context {
            state,
            definitions: self.factory.definitions,
            resources: self.factory.resources,
            movement: self.factory.movement,
            combat: self.factory.combat,
            costs: self.factory.costs,
            placement: self.factory.placement,
            queries: self.factory.queries,
            runtime: self.factory.runtime,
            tags: self.factory.tags,
            sight: self.factory.sight,
            effects: self.backend,
        }
        .create_from_city(unit_creation::Parameters {
            city,
            unit_hash: unit,
            military_formation: formation,
            location,
        })
        .map_err(|e| format!("purchase City.CreateUnit: {e:?}"))
    }
    fn finish_moves(&mut self, state: &mut LoadedState, unit: ComponentId) -> Result<(), String> {
        crate::loaded_upkeep::finish_unit_moves(
            state,
            i32::from(unit.owner),
            unit.id,
            self.factory.movement,
            self.backend,
        )
        .map_err(|e| format!("purchase FinishMoves: {e:?}"))
    }
    fn purchased_report(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        unit: u32,
    ) -> Result<(), String> {
        self.backend
            .check(state)
            .map_err(|e| format!("purchase report binding: {e:?}"))?;
        self.reports.push(Report::UnitPurchased { city, unit });
        Ok(())
    }
    fn command_complete(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
    ) -> Result<(), String> {
        self.backend
            .check(state)
            .map_err(|e| format!("purchase command report binding: {e:?}"))?;
        // BaseHandler.Start reports only to the current local owner.
        let local = self.backend.local_player().ok_or("purchase local Player")?;
        if i32::from(city.owner) == local {
            self.reports.push(Report::CommandComplete { city });
        }
        Ok(())
    }
}
// Keep concrete error spelling available to caller-owned request adapters.
pub type StockError = Error<String>;
