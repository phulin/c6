//! Stock movement services on the persistent action/notification owner.
//! Native pointer qualification and capture conversion are caller input work.
//! Unknown reached gameplay callbacks remain explicit errors.
use crate::loaded_actions::{reporting, stock_notifications::Runtime, IdentityReport};
use crate::{
    formation_rules, loaded_upkeep, movement_placement_state as placement,
    unit_placement_services as services, unit_placement_state as normal, unit_sight_state as sight,
};
use c6_core::{
    loaded_state::LoadedState,
    map::{Plot, UnitReference},
    unit::{ComponentId, PlotCoord, Unit},
};

#[derive(Debug)]
pub enum Error {
    Missing(&'static str),
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Rules(crate::loaded_actions::Error),
    Placement(placement::Error),
    Script(super::stock_script::Error),
    Requirement(String),
    RequirementChanged { receiver: u64, old: i32, new: i32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stock movement: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Two pure current-state queries, implemented by the existing admitted
/// placement definitions. This erases rule-borrow lifetimes, not gameplay work.
pub trait PlacementQueries {
    fn water(&self, state: &LoadedState, plot: &Plot) -> Result<bool, placement::Error>;
    fn can_embark(
        &self,
        state: &LoadedState,
        unit: &Unit,
        levels: &[(i32, u32)],
    ) -> Result<bool, placement::Error>;
}
impl PlacementQueries for placement::Definitions<'_, '_> {
    fn water(&self, state: &LoadedState, plot: &Plot) -> Result<bool, placement::Error> {
        placement::Context::from_definitions(state, self)?.water(plot)
    }
    fn can_embark(
        &self,
        state: &LoadedState,
        unit: &Unit,
        levels: &[(i32, u32)],
    ) -> Result<bool, placement::Error> {
        placement::Context::from_definitions(state, self)?
            .with_civilization_levels(placement::CurrentCivilizationLevels {
                binding: state.binding(),
                values: levels,
            })?
            .can_embark(unit)
    }
}
pub struct Definitions<'a> {
    pub formation: &'a formation_rules::Definitions,
    pub placement: &'a dyn PlacementQueries,
    pub features: &'a crate::registry::LinuxDefinitionRegistry,
    pub services: &'a services::Definitions,
    pub moved_requirements: Option<&'a crate::unit_moved_requirements::Definitions>,
    pub near_requirements: Option<&'a crate::unit_near_religious_city_requirement::Definitions>,
}
/// Current source inputs. These are not answers to movement predicates.
pub struct State<'a> {
    pub definitions: Definitions<'a>,
    pub civilization_levels: Vec<(i32, u32)>,
    pub reporting_suspended: bool,
    pub local_observer: Option<i32>,
    pub unit_orders: std::collections::BTreeMap<i32, Vec<UnitReference>>,
    pub signal: c6_core::unit_moved_signal::Registry,
    pub plot_requirements: Option<crate::plot_adjacent_owner_requirement::Runtime>,
    pub moved_requirements: Option<crate::unit_moved_requirements::Runtime>,
    pub near_requirements: Option<crate::unit_near_religious_city_requirement::Runtime>,
    pub tag_requirements: Option<crate::plot_adjacent_friendly_unit_tag_requirement::Runtime>,
    pub requirement_attempts: Vec<u64>,
    pub requirement_changes: Vec<(u64, i32, i32)>,
}
impl<'a, P> Runtime<'a, P> {
    pub fn bind_movement(&mut self, state: &LoadedState, movement: State<'a>) -> Result<(), Error> {
        if self.binding != *state.binding() || !self.script.script().matches(state) {
            return Err(Error::Missing("movement state identity"));
        }
        if self.movement.is_some() {
            return Err(Error::Missing("movement services already bound"));
        }
        movement
            .definitions
            .formation
            .definition_can_capture(state, u32::MAX)
            .map_err(Error::Rules)?;
        self.movement = Some(movement);
        Ok(())
    }
    fn movement(&self) -> Result<&State<'a>, Error> {
        self.movement
            .as_ref()
            .ok_or(Error::Missing("movement services"))
    }
    fn current_actor(&self, state: &LoadedState, actor: (i32, u32)) -> Result<(), Error> {
        if self.binding != *state.binding() {
            return Err(Error::Missing("movement state identity"));
        }
        state
            .world()
            .player(actor.0)
            .map_err(Error::World)?
            .unit(actor.1)
            .map_err(Error::World)?
            .ok_or(Error::Missing("movement Unit"))?;
        Ok(())
    }
    fn visibility_owner(
        &self,
        state: &LoadedState,
        runtime: &sight::Runtime,
        owner: i32,
        plot: Option<u32>,
    ) -> Result<(), Error> {
        if self.binding != *state.binding() {
            return Err(Error::Missing("movement state identity"));
        }
        runtime.shared(owner).map_err(Error::Missing)?;
        if let Some(index) = plot {
            state
                .world()
                .map()
                .plot_by_index(index)
                .ok_or(Error::Missing("visibility plot"))?;
        }
        Ok(())
    }
}

impl<P> services::Services for Runtime<'_, P> {
    type Error = Error;
    fn placement_service_runtime(&self) -> Result<&services::Runtime, services::Error<Error>> {
        self.player_services
            .as_ref()
            .ok_or(services::Error::Missing("current player services"))
    }
    fn placement_service_runtime_mut(
        &mut self,
    ) -> Result<&mut services::Runtime, services::Error<Error>> {
        self.player_services
            .as_mut()
            .ok_or(services::Error::Missing("current player services"))
    }
    fn placement_service_definitions(
        &self,
    ) -> Result<&services::Definitions, services::Error<Error>> {
        Ok(self
            .movement()
            .map_err(services::Error::Effect)?
            .definitions
            .services)
    }
    fn preflight_encountered_unit(
        &self,
        state: &LoadedState,
        owner: i32,
        unit_type: i32,
    ) -> Result<(), services::Error<Error>> {
        let runtime = self.placement_service_runtime()?;
        if runtime.binding != *state.binding() {
            return Err(services::Error::Binding);
        }
        let values = runtime
            .player::<Error>(owner)?
            .encountered_units
            .as_ref()
            .ok_or(services::Error::Missing("encountered unit flags"))?;
        usize::try_from(unit_type)
            .ok()
            .and_then(|index| values.get(index))
            .ok_or(services::Error::Missing("encountered unit index"))?;
        Ok(())
    }
}
impl<P> loaded_upkeep::OperationReports for Runtime<'_, P> {
    type Error = Error;
    fn preflight_operation_report(
        &self,
        state: &LoadedState,
        actor: loaded_upkeep::UnitHandle,
        _: IdentityReport,
    ) -> Result<(), Error> {
        self.current_actor(state, (actor.collection_owner(), actor.requested_id()))
    }
    fn preflight_placement_report(
        &self,
        state: &LoadedState,
        actor: loaded_upkeep::UnitHandle,
        _: u32,
    ) -> Result<(), Error> {
        self.current_actor(state, (actor.collection_owner(), actor.requested_id()))
    }
    fn operation_report(
        &mut self,
        state: &mut LoadedState,
        report: IdentityReport,
    ) -> Result<i32, Error> {
        if !matches!(
            report.event,
            0xe23b112d | 0xa60328c5 | 0x383669f1 | 0x15c35863 | 0x3d4006f2 | 0xcb0345f1
        ) {
            return Err(Error::Missing("operation report subscribers"));
        }
        if self.movement()?.reporting_suspended {
            return Ok(-1);
        }
        self.reporting
            .send_loaded_report(
                state,
                UnitReference::from_component_id(report.identity),
                report,
            )
            .map_err(Error::Rules)?;
        Ok(0)
    }
    fn placement_report(
        &mut self,
        state: &mut LoadedState,
        report: c6_core::unit_placement_transition::PlacementReport,
    ) -> Result<(), Error> {
        if self.movement()?.reporting_suspended {
            return Ok(());
        }
        match report.event_hash {
            0xb8515a41 => self.reporting.observe_initial_placement_report(
                state, UnitReference::from_component_id(report.identity), report,
            ).map_err(Error::Rules),
            0x9deb82e7 => self.reporting.observe_teleported_report(
                state, UnitReference::from_component_id(report.identity), report,
            ).map_err(Error::Rules),
            0xc1300ee7 => self.reporting.observe_visibility_report(state, report)
                .map_err(Error::Rules),
            0xbe121408 => self
                .reporting
                .send_moved_report(
                    state,
                    UnitReference::from_component_id(report.identity),
                    report,
                )
                .map_err(Error::Rules),
            0x6e7ef927 => {
                let movement = self
                    .movement
                    .as_ref()
                    .ok_or(Error::Missing("movement services"))?;
                self.reporting
                    .send_move_complete_report(
                        state,
                        report,
                        reporting::MoveCompleteInputs {
                            local_observer: movement.local_observer,
                            native_unit_order: movement
                                .unit_orders
                                .get(&i32::from(report.identity.owner))
                                .map(Vec::as_slice),
                        },
                    )
                    .map_err(Error::Rules)
            }
            _ => Err(Error::Missing("placement report subscribers")),
        }
    }
}
impl<'a, P> loaded_upkeep::Effects for Runtime<'a, P> {
    type Error = Error;
    type Definitions = &'a formation_rules::Definitions;
    fn definitions(&mut self, state: &mut LoadedState) -> Result<Self::Definitions, Error> {
        let defs = self.movement()?.definitions.formation;
        defs.definition_can_capture(state, u32::MAX)
            .map_err(Error::Rules)?;
        Ok(defs)
    }
    fn preflight_movement(
        &self,
        state: &LoadedState,
        actor: loaded_upkeep::UnitHandle,
        _: bool,
    ) -> Result<(), Error> {
        self.current_actor(state, (actor.collection_owner(), actor.requested_id()))
    }
    fn preflight_formation(&self, state: &LoadedState, owner: i32, id: u32) -> Result<(), Error> {
        if self.binding != *state.binding() {
            return Err(Error::Missing("formation state identity"));
        }
        state
            .world()
            .player(owner)
            .map_err(Error::World)?
            .formation(id)
            .map_err(Error::World)?
            .ok_or(Error::Missing("current Formation"))?;
        Ok(())
    }
    fn integer(
        &mut self,
        state: &mut LoadedState,
        actor: loaded_upkeep::UnitHandle,
        query: c6_core::formation_upkeep::IntegerQuery,
    ) -> Result<i32, Error> {
        use c6_core::formation_upkeep::IntegerQuery::*;
        let unit = actor.get(state).map_err(Error::World)?;
        let defs = self.movement()?.definitions.formation;
        match query {
            Combat => defs.combat(state, unit),
            RangedCombat => defs.ranged(state, unit),
            BombardCombat => defs.bombard(state, unit),
            _ => return Err(Error::Missing("internal formation integer query")),
        }
        .map_err(Error::Rules)
    }
    fn boolean(
        &mut self,
        state: &mut LoadedState,
        actor: loaded_upkeep::UnitHandle,
        query: c6_core::formation_upkeep::BooleanQuery,
    ) -> Result<bool, Error> {
        use c6_core::formation_upkeep::BooleanQuery::*;
        let unit = actor.get(state).map_err(Error::World)?;
        let movement = self.movement()?;
        match query {
            CanEmbark => movement
                .definitions
                .placement
                .can_embark(state, unit, &movement.civilization_levels)
                .map_err(Error::Placement),
            CanAttack => movement
                .definitions
                .formation
                .can_attack(state, unit)
                .map_err(Error::Rules),
            MovementDisabledThisTurn => movement
                .definitions
                .formation
                .movement_disabled(state, unit)
                .map_err(Error::Rules),
        }
    }
    fn definition_can_capture(
        &mut self,
        state: &mut LoadedState,
        defs: Self::Definitions,
        index: u32,
    ) -> Result<Option<bool>, Error> {
        defs.definition_can_capture(state, index)
            .map_err(Error::Rules)
    }
    fn report(
        &mut self,
        state: &mut LoadedState,
        report: c6_core::unit_turn_upkeep::MovementReport,
    ) -> Result<(), Error> {
        if !matches!(report.event_hash, 0xc50d66d6 | 0xe992f3cd | 0x6e8a2f93) {
            return Err(Error::Missing("movement report subscribers"));
        }
        if !self.movement()?.reporting_suspended {
            self.reporting
                .send_movement_report(state, report)
                .map_err(Error::Rules)?;
        }
        Ok(())
    }
    fn owner_is_barbarian(&self, state: &LoadedState, owner: i32) -> Result<bool, Error> {
        if self.binding != *state.binding() {
            return Err(Error::Missing("player state identity"));
        }
        if let Some(services) = &self.player_services {
            let mut rows = services.players.iter().filter(|row| row.owner == owner);
            let level = rows
                .next()
                .and_then(|row| row.civilization_level)
                .ok_or(Error::Missing("current civilization level"))?;
            if rows.next().is_some() {
                return Err(Error::Missing("ambiguous civilization level"));
            }
            return Ok(level == 0xd959ec24);
        }
        let mut rows = self
            .movement()?
            .civilization_levels
            .iter()
            .filter(|row| row.0 == owner);
        let value = rows.next().ok_or(Error::Missing("civilization level"))?.1;
        if rows.next().is_some() {
            return Err(Error::Missing("ambiguous civilization level"));
        }
        Ok(value == 0xd959ec24)
    }
}
impl<P> loaded_upkeep::FormationRestoreEffects for Runtime<'_, P> {
    fn formation_restored(
        &mut self,
        state: &mut LoadedState,
        report: c6_core::unit_turn_start::FormationRestoredReport,
    ) -> Result<(), Error> {
        if report.event_hash != 0xc52231ab {
            return Err(Error::Missing("formation report subscribers"));
        }
        if !self.movement()?.reporting_suspended {
            self.reporting
                .observe_formation_snapshot(state, report.identity)
                .map_err(Error::Rules)?;
        }
        Ok(())
    }
}
impl<P> loaded_upkeep::placement_tails::Reports for Runtime<'_, P> {
    type Error = Error;
    fn preflight_spread_report(
        &self,
        _: &LoadedState,
        _: loaded_upkeep::UnitHandle,
    ) -> Result<(), Error> {
        Err(Error::Missing("spread report subscribers"))
    }
    fn spread_report(
        &mut self,
        _: &mut LoadedState,
        _: c6_core::unit_placement_tails::SpreadChargesReport,
    ) -> Result<(), Error> {
        Err(Error::Missing("spread report subscribers"))
    }
}
#[path = "stock_movement_requirements.rs"]
mod requirement_services;
#[path = "stock_movement_sight.rs"]
mod sight_services;
