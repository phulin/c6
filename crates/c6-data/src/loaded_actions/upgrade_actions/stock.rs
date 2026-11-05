//! Upgrade lifecycle effects over the persistent stock runtime and state.
use super::{Effects, Scope};
use crate::{
    builder_improvement::death,
    loaded_actions::stock_notifications::{self, city_production::unit::Factory},
    unit_creation, unit_kill,
};
use c6_core::{
    loaded_state::LoadedState,
    unit::{ComponentId, PlotCoord, Unit},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Report {
    /// Reporting 0xfc7db95e: COMMAND_UPGRADE (0xd34b69c0), old CID, value 0.
    CommandStarted {
        unit: ComponentId,
    },
    UnitUpgraded {
        unit: ComponentId,
        unit_type: u32,
    },
    CommandComplete {
        unit: ComponentId,
    },
}

pub struct Current<'s, 'a, 'd, 'r, 't, 'q, P> {
    pub backend: &'s mut stock_notifications::Runtime<'a, P>,
    pub factory: Factory<'s, 'd, 'r>,
    pub death: &'s death::Inputs<'d, 'r>,
    pub transfer: &'s mut super::transfer::Inputs<'t>,
    pub resources: &'s super::resources::Inputs<'q>,
    pub reports: &'s mut Vec<Report>,
}
impl<P> Effects for Current<'_, '_, '_, '_, '_, '_, P> {
    type Error = String;
    fn common_guards(&self, state: &LoadedState, actor: (i32, u32)) -> Result<bool, String> {
        self.backend
            .check(state)
            .map_err(|e| format!("upgrade guards binding: {e:?}"))?;
        let unit =
            super::actor::<()>(state, actor).map_err(|e| format!("upgrade guards Unit: {e:?}"))?;
        if !self
            .factory
            .resources
            .phases()
            .from_processor(&state.retained().core.phase_processor, actor.0)
            .map_err(|e| format!("upgrade command phase: {e:?}"))?
        {
            return Ok(false);
        }
        if !state
            .turn_active(actor.0)
            .map_err(|e| format!("upgrade active turn: {e:?}"))?
        {
            return Ok(false);
        }
        if state
            .player_slot_status(actor.0)
            .map_err(|e| format!("upgrade player status: {e:?}"))?
            == 3
            && state
                .pause_storage()
                .map_err(|e| format!("upgrade pause: {e:?}"))?
                != 0
        {
            return Ok(false);
        }
        if unit.i_damage
            >= self
                .transfer
                .maximum
                .maximum_damage(unit)
                .map_err(|e| format!("upgrade maximum damage: {e:?}"))?
        {
            return Ok(false);
        }
        Ok(true)
    }
    fn query_scope(&self, state: &LoadedState, actor: (i32, u32)) -> Result<Scope, String> {
        self.backend
            .check(state)
            .map_err(|e| format!("upgrade binding: {e:?}"))?;
        ordinary_scope(state, actor, self.factory.queries)
    }
    fn preflight_transfer(
        &self,
        state: &LoadedState,
        actor: (i32, u32),
        target: u32,
    ) -> Result<(), String> {
        super::transfer::preflight(state, actor, target, self.transfer, self.backend)
    }
    fn command_started(&mut self, state: &mut LoadedState, old: ComponentId) -> Result<(), String> {
        self.backend
            .check(state)
            .map_err(|e| format!("upgrade command-start binding: {e:?}"))?;
        if !self.transfer.presentation_reports_qualified {
            return Err("unqualified upgrade command-start report consumers".into());
        }
        self.reports.push(Report::CommandStarted { unit: old });
        Ok(())
    }
    fn prepare_transfer(&mut self, state: &LoadedState, old: &Unit) -> Result<(), String> {
        super::transfer::prepare(state, old, self.transfer)
    }
    fn kill(&mut self, state: &mut LoadedState, actor: (i32, u32)) -> Result<(), String> {
        let input = self.death;
        let sight = &mut *self.factory.sight;
        let placement = self.factory.placement;
        let queries = self.factory.queries;
        self.backend
            .with_modifier_owners(|backend, owners| {
                let mut influence =
                    |state: &mut LoadedState,
                     visibility: &crate::unit_sight_state::Runtime,
                     services: &crate::unit_placement_services::Runtime,
                     owner: i32| {
                        let alive = services.alive_players.as_ref().ok_or(
                            unit_kill::stock::Error::Input("current Influence alive order"),
                        )?;
                        crate::influence_map_update::update(
                            state,
                            input.influence,
                            &crate::influence_map_update::Queries {
                                alive_players: alive,
                                capabilities: Some(input.capabilities),
                                visibility,
                                movement: input.movement,
                                formation: input.formation,
                                city_range: input.cities,
                                path: None,
                                path_runtime: None,
                                strength: None,
                            },
                            owner,
                        )
                        .map_err(|e| unit_kill::stock::Error::Influence(format!("{e:?}")))
                    };
                let mut stock = unit_kill::stock::Stock::new(
                    &mut owners.effects,
                    &mut owners.requirements,
                    input.unit_definitions,
                    input.metadata,
                )
                .map_err(|e| format!("upgrade death bindings: {e:?}"))?
                .with_collections(&mut owners.collections)
                .with_unit_definitions(input.units)
                .with_influence(&mut influence);
                let killed = unit_kill::kill_with_continuations(
                    state, sight, backend, &mut stock, placement, queries, actor, true, -1,
                )
                .map_err(|e| format!("upgrade Kill: {e:?}"))?;
                if !killed {
                    return Err("upgrade Kill lost current Unit".into());
                }
                Ok(())
            })
            .map_err(|e| format!("upgrade canonical lifecycle owners: {e:?}"))?
    }
    fn create(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        target: u32,
        location: PlotCoord,
    ) -> Result<Option<ComponentId>, String> {
        let created = unit_creation::PlayerContext {
            state,
            definitions: self.factory.definitions,
            resources: self.factory.resources,
            movement: self.factory.movement,
            combat: self.factory.combat,
            runtime: self.factory.runtime,
            tags: self.factory.tags,
            effects: self.backend,
        }
        .create_without_city(owner, target)
        .map_err(|e| format!("upgrade PlayerUnits.Create: {e:?}"))?;
        if let Some(unit) = created {
            crate::unit_placement_state::creation::set(
                state,
                self.factory.placement,
                self.factory.queries,
                self.factory.sight,
                self.backend,
                crate::unit_placement_state::Request {
                    actor: (owner, unit.id),
                    destination: location,
                    validation_flags: 1,
                },
                self.factory.tags,
            )
            .map_err(|e| format!("upgrade Placement.Set: {e:?}"))?;
        }
        Ok(created)
    }
    fn gold_changed(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), String> {
        crate::game_update_state::activation::treasury_deals::Effects::gold_changed(
            self.backend,
            state,
            owner,
        )
        .map_err(|e| format!("upgrade GoldChanged: {e:?}"))
    }
    fn transfer(
        &mut self,
        state: &mut LoadedState,
        old: &Unit,
        new: ComponentId,
    ) -> Result<(), String> {
        super::transfer::transfer(state, old, new, self.transfer, self.backend)
    }
    fn finish_moves(&mut self, state: &mut LoadedState, new: ComponentId) -> Result<(), String> {
        crate::loaded_upkeep::finish_unit_moves(
            state,
            i32::from(new.owner),
            new.id,
            self.factory.movement,
            self.backend,
        )
        .map_err(|e| format!("upgrade FinishMoves: {e:?}"))
    }
    fn processed_turn(&mut self, state: &mut LoadedState, new: ComponentId) -> Result<(), String> {
        self.backend.upgrade_processed_turn(state, new)
    }
    fn preflight_resource(
        &self,
        state: &LoadedState,
        resource: u32,
        amount: i32,
    ) -> Result<(), String> {
        super::resources::preflight_resource(state, self.resources, resource, amount)
    }
    fn consume_resource(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        resource: u32,
        amount: i32,
    ) -> Result<(), String> {
        super::resources::consume_resource(
            state,
            self.backend,
            self.resources,
            owner,
            resource,
            amount,
        )
    }
    fn upgraded(&mut self, state: &mut LoadedState, new: ComponentId) -> Result<(), String> {
        super::resources::upgraded(state, self.resources, new)?;
        let unit_type = state
            .world()
            .player(i32::from(new.owner))
            .map_err(|e| format!("upgrade report Player: {e:?}"))?
            .unit(new.id)
            .map_err(|e| format!("upgrade report Unit: {e:?}"))?
            .ok_or("upgrade report missing Unit")?
            .record
            .e_type_index;
        self.reports.push(Report::UnitUpgraded {
            unit: new,
            unit_type,
        });
        Ok(())
    }
    fn command_complete(
        &mut self,
        state: &mut LoadedState,
        old: ComponentId,
    ) -> Result<(), String> {
        self.backend
            .check(state)
            .map_err(|e| format!("upgrade command binding: {e:?}"))?;
        let local = self.backend.local_player().ok_or("upgrade local Player")?;
        if i32::from(old.owner) == local {
            self.reports.push(Report::CommandComplete { unit: old });
        }
        Ok(())
    }
}

/// Bounded ordinary territory branch. Foreign/suzerain/alliance territory needs
/// its actual diplomacy predicate before it can be admitted here.
fn ordinary_scope(
    state: &LoadedState,
    actor: (i32, u32),
    queries: &crate::unit_placement_state::Queries<'_>,
) -> Result<Scope, String> {
    let unit =
        super::actor::<()>(state, actor).map_err(|e| format!("upgrade scope Unit: {e:?}"))?;
    let level = queries
        .civilization_levels
        .and_then(|v| v.iter().find(|v| v.0 == actor.0))
        .ok_or("upgrade current civilization level")?
        .1;
    let barbarian = level == crate::registry::linux_type_hash(b"CIVILIZATION_LEVEL_TRIBE");
    let major = level == crate::registry::linux_type_hash(b"CIVILIZATION_LEVEL_FULL_CIV");
    if barbarian {
        return Ok(Scope {
            barbarian: true,
            major: false,
            diplomatic_territory: 0,
            ignore_resource: true,
        });
    }
    let index = crate::unit_placement_services::plot_index_at::<std::convert::Infallible>(
        state,
        unit.k_location,
    )
    .map_err(|e| format!("upgrade current plot: {e:?}"))?;
    let plot = index.and_then(|i| state.world().map().plot_by_index(i));
    let owner = plot
        .map(crate::movement_placement_state::plot_owner)
        .transpose()
        .map_err(|e| format!("upgrade plot owner: {e:?}"))?
        .unwrap_or(-1);
    let diplomatic_territory = if owner == actor.0 {
        1
    } else if owner == -1 {
        0
    } else {
        return Err("upgrade foreign diplomatic territory".into());
    };
    if unit.b_is_levied {
        return Err("upgrade levied resource exemption".into());
    }
    if !major && !barbarian {
        return Err("upgrade nonmajor resource exemption".into());
    }
    let city = plot
        .and_then(|p| p.city_attributes.as_ref())
        .filter(|p| p.owning_city != u32::MAX)
        .map(|p| state.city(i32::from(p.player), p.owning_city))
        .transpose()
        .map_err(|e| format!("upgrade owning City: {e:?}"))?;
    let ignore_resource = city.is_none_or(|c| c.build_queue.ignore_strategic_resource_requirements);
    Ok(Scope {
        barbarian,
        major,
        diplomatic_territory,
        ignore_resource,
    })
}
