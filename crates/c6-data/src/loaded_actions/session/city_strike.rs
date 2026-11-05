//! Explicit ordinary City ranged command on the persistent stock Session.
use super::{Error, Session};
use crate::{
    city_strike::{self as strike, error, reports::Effects as _},
    loaded_actions::city_range_attack as city,
    melee_attack as melee, unit_sight_state as sight,
};
use c6_core::{
    loaded_state::LoadedState,
    unit::{ComponentId, PlotCoord},
};

impl<'s, 'a> Session<'s, 'a> {
    /// Execute one explicit city/district strike against a current target. The
    /// actual District spends its shot; Unit damage, RNG and stock callbacks
    /// use the same owners retained by other actions. Unsupported continuations
    /// retain their reached prefix and freeze this Session.
    #[allow(clippy::too_many_arguments)]
    pub fn city_range_attack(
        &mut self,
        issuer: i32,
        city_id: u32,
        district_id: Option<u32>,
        target: PlotCoord,
        queries: &city::Queries<'_>,
        inputs: &melee::transition::Inputs<'_, '_, '_>,
        sight: &mut sight::Runtime,
        budget: &mut usize,
    ) -> Result<strike::Outcome, Error<strike::Error>> {
        self.ordinary_player_action(issuer)?;
        let before = self.state().action_stamp();
        let result = (|| {
            let attacker = current_district(self.state(), issuer, city_id, district_id)?;
            let admitted = can_start(self.state(), issuer, city_id, attacker, target, queries)?;
            let outcome = if admitted {
                let state = &mut *self.context.notifications.state;
                let district = state
                    .district(issuer, attacker.id)
                    .map_err(|e| error("execution district", e))?;
                // Start performs a fresh defender selection after CanStart.
                let defender =
                    city::select_ranged_defender(state, district, (target.x, target.y), queries)
                        .map_err(|e| error("execution target", e))?
                        .ok_or_else(|| error("execution target", "admitted target disappeared"))?;
                let defender = c6_core::map::UnitReference::from_component_id(ComponentId {
                    owner: defender.e_owner as i16,
                    component_type: 1,
                    id: defender.id,
                });
                let generated = self.context.notifications.backend.city_strike(
                    state,
                    sight,
                    queries.definitions,
                    inputs,
                    attacker,
                    defender,
                    target,
                )?;
                self.context
                    .notifications
                    .backend
                    .city_command_started(
                        state,
                        ComponentId {
                            owner: issuer as i16,
                            component_type: 2,
                            id: city_id,
                        },
                        attacker,
                        0x8abcccc1,
                    )
                    .map_err(|e| error("city command report", e))?;
                // RangeAttack.UpdatesEndTurnBlockingState is true. Manager.Start
                // performs this update, and CityCommand.Execute performs it again.
                self.context
                    .notifications
                    .update(issuer, budget)
                    .map_err(|e| error("manager blocking", e))?;
                self.context
                    .notifications
                    .update(issuer, budget)
                    .map_err(|e| error("message blocking", e))?;
                strike::Outcome::Applied(generated)
            } else {
                strike::Outcome::Rejected
            };
            // The outer queue publishes MessagesProcessed once even when the
            // current manager rejects a request; no UnitOperation diplomacy tail.
            self.context
                .messages_processed()
                .map_err(|e| error("messages processed", e))?;
            Ok(outcome)
        })();
        let partial = self.state().action_stamp() != before;
        self.retain(result, partial)
    }
}
fn current_district(
    state: &LoadedState,
    owner: i32,
    city_id: u32,
    requested: Option<u32>,
) -> Result<ComponentId, strike::Error> {
    let current_city = state.city(owner, city_id).map_err(|e| error("city", e))?;
    let district = if let Some(id) = requested {
        let district = state
            .district(owner, id)
            .map_err(|e| error("district", e))?;
        if district.city_id != city_id {
            return Err(error("district", "different parent City"));
        }
        district
    } else {
        let districts = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .map(|p| &p.districts)
            .ok_or_else(|| error("district", "missing owner roster"))?;
        city::center_district(current_city, districts)
            .ok_or_else(|| error("district", "missing center District"))?
    };
    Ok(ComponentId {
        owner: owner as i16,
        component_type: 3,
        id: district.id,
    })
}
#[allow(clippy::too_many_arguments)]
fn can_start(
    state: &LoadedState,
    owner: i32,
    city_id: u32,
    attacker: ComponentId,
    target: PlotCoord,
    queries: &city::Queries<'_>,
) -> Result<bool, strike::Error> {
    if !state
        .turn_active(owner)
        .map_err(|e| error("turn active", e))?
    {
        return Ok(false);
    }
    if !queries
        .phases
        .from_processor(&state.retained().core.phase_processor, owner)
        .map_err(|e| error("strategic phase", e))?
    {
        return Ok(false);
    }
    if state
        .player_slot_status(owner)
        .map_err(|e| error("human", e))?
        == 3
        && state.pause_storage().map_err(|e| error("paused", e))? != 0
    {
        return Ok(false);
    }
    let district = state
        .district(owner, attacker.id)
        .map_err(|e| error("district", e))?;
    let remaining = district.remaining_attacks > 0;
    let strength = city::attack_strength_current_q8(
        state,
        district,
        queries.definitions,
        queries.civilization_level_types,
    )
    .map_err(|e| error("attack strength", e))?;
    if strength <= 0 || !remaining || district.pillaged || !district.is_complete {
        return Ok(false);
    }
    let current_city = state.city(owner, city_id).map_err(|e| error("city", e))?;
    let _owner = city::district_owner(state, district).map_err(|e| error("plot owner", e))?;
    let range = city::attack_range(current_city, district, strength, queries.definitions);
    if !c6_core::geometry::plot_valid(target.x, target.y) {
        return Err(error("target", "explicit valid target required"));
    }
    if city::plot_distance(state, district.location, (target.x, target.y)) > range {
        return Ok(false);
    }
    let map = state.world().map();
    let x = c6_core::movement_world::coord_range(target.x, map.header.width, map.header.wrap_x);
    let y = c6_core::movement_world::coord_range(target.y, map.header.height, map.header.wrap_y);
    if map.plot_at(x, y).is_none() {
        return Ok(false);
    }
    let index = (y as u32)
        .wrapping_mul(map.header.width as u32)
        .wrapping_add(x as u32);
    if !city::sight_plots(state, district.location, range, queries.definitions)
        .map_err(|e| error("CanSeePlot", e))?
        .iter()
        .any(|entry| entry.index == index)
    {
        return Ok(false);
    }
    city::test_ranged_attack_into(state, district, (target.x, target.y), queries)
        .map_err(|e| error("CanStart target", e))
}
