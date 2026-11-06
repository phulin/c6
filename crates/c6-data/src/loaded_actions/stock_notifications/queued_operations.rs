//! Persistent queued-operation continuation inside Player.DoAutomation.
//! This resumes the existing manager on the current linked Unit; it never
//! constructs another request, replaces the world, or selects a policy.
use super::{movement, Runtime};
use crate::{
    game_update_state::automation,
    loaded_actions::{self as actions, move_to},
};
use c6_core::{
    loaded_state::{game_update::automation::Frame, LoadedState},
    unit::OperationParameters,
};

pub trait Services<P> {
    fn idle_trade_paths(
        &mut self,
        _state: &mut LoadedState,
        _backend: &mut Runtime<'_, P>,
        _owner: i32,
        _actor: u32,
    ) -> Result<(), String> {
        Err("current idle Trader path definitions".into())
    }

    fn advance(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        frame: &mut Frame,
        readiness: &actions::reporting::Provider<'_>,
    ) -> Result<(), String>;
}
/// Existing rules, current visibility and path-search owners retained across
/// requests and turns. No captured handler answer or AFTER value is supplied.
pub struct Current<'s, 'a, 'r> {
    pub inputs: &'s actions::Inputs<'s, 'r>,
    pub handlers: &'s actions::handlers::BoundHandlers<'s>,
    pub path: &'s crate::formation_path_state::Definitions<'a, 'r>,
    pub placement: &'s crate::unit_placement_state::Definitions<'a, 'r>,
    pub queries: &'s crate::unit_placement_state::Queries<'a>,
    pub search: &'s std::cell::RefCell<c6_core::pathfinder::TwoLayer>,
    pub sight: &'s std::cell::RefCell<crate::unit_sight_state::Runtime>,
    pub manager_width: u32,
}
impl Services<OperationParameters> for Current<'_, '_, '_> {
    fn advance(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, OperationParameters>,
        frame: &mut Frame,
        readiness: &actions::reporting::Provider<'_>,
    ) -> Result<(), String> {
        let mut sight = self
            .sight
            .try_borrow_mut()
            .map_err(|e| format!("queued sight: {e}"))?;
        let mut search = self
            .search
            .try_borrow_mut()
            .map_err(|e| format!("queued search: {e}"))?;
        let mut movement = move_to::ComputedMovement {
            path: self.path,
            placement: self.placement,
            queries: self.queries,
            search: &mut search,
            execution: crate::formation_path_state::ExecutionContext::Exclusive,
        };
        let result = automation::advance_player_unit(
            state,
            frame,
            readiness,
            self.inputs,
            self.handlers,
            &mut move_to::Runtime {
                visibility: &mut sight,
                manager_width: self.manager_width,
            },
            backend,
            &mut movement,
            automation::Scope::StockLinuxXp2RegisteredHandlers,
            Some(c6_core::loaded_state::game_update::deactivation::LoggingScope::StockFileSink),
        )
        .map_err(|e| format!("queued unit: {e:?}"))?;
        match result {
            automation::UnitProgress::Advanced => Ok(()),
            automation::UnitProgress::NoUnit => {
                Err("queued current Unit disappeared before continuation".into())
            }
        }
    }
}
impl<'a, P> Runtime<'a, P> {
    pub fn bind_queued_operations(
        &mut self,
        state: &LoadedState,
        services: Box<dyn Services<P> + 'a>,
    ) -> Result<(), movement::Error> {
        self.check(state)
            .map_err(|_| movement::Error::Missing("queued operation binding"))?;
        if self.queued_operations.is_some() {
            return Err(movement::Error::Missing("queued operations already bound"));
        }
        self.queued_operations = Some(services);
        Ok(())
    }
    pub(crate) fn refresh_idle_trade_paths(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        actor: u32,
    ) -> Result<(), String> {
        let mut services = self
            .queued_operations
            .take()
            .ok_or("current idle Trader services")?;
        let result = services.idle_trade_paths(state, self, owner, actor);
        self.queued_operations = Some(services);
        result
    }
    pub(crate) fn continue_queued_operation(
        &mut self,
        state: &mut LoadedState,
        frame: &mut Frame,
        readiness: &actions::reporting::Provider<'_>,
    ) -> Result<(), String> {
        let mut services = self
            .queued_operations
            .take()
            .ok_or("current queued-operation services")?;
        let result = services.advance(state, self, frame, readiness);
        self.queued_operations = Some(services);
        result
    }
}

/// Persistent existing trade orders use the same queue and placement pipeline
/// as other automation. This never selects a destination or creates an order.
pub struct Trade<'s, 'a, 'r> {
    pub current: Current<'s, 'a, 'r>,
    pub definitions: &'s actions::trade_route_actions::Definitions,
    pub path_definitions: &'s actions::trade_route_actions::path::Definitions,
    pub lifetime: &'s actions::trade_route_actions::lifetime::Definitions,
    pub roads: &'s actions::trade_route_actions::stock::RoadInputs<'a>,
    pub ending: Option<&'s actions::trade_route_actions::ending_stock::Inputs<'a>>,
    pub ending_reports: Option<
        &'s mut dyn actions::trade_route_actions::ending_stock::Reports<OperationParameters>,
    >,
    pub trade_search: &'s std::cell::RefCell<c6_core::pathfinder::TwoLayer>,
}
impl Services<OperationParameters> for Trade<'_, '_, '_> {
    fn idle_trade_paths(
        &mut self,
        state: &mut LoadedState,
        _backend: &mut Runtime<'_, OperationParameters>,
        owner: i32,
        actor: u32,
    ) -> Result<(), String> {
        use actions::trade_route_actions::path;
        use c6_core::{trade::ComponentId, trade_graph_state::city_vertex, unit::PlotCoord};
        let unit = state
            .world()
            .player(owner)
            .map_err(|e| format!("{e:?}"))?
            .unit(actor)
            .map_err(|e| format!("{e:?}"))?
            .ok_or("idle Trader disappeared")?
            .record;
        let location = unit.k_location;
        let origin = state
            .retained()
            .players
            .iter()
            .flat_map(|p| p.cities.records.entries.iter())
            .find(|c| {
                c.value.header.location_x == location.x && c.value.header.location_y == location.y
            })
            .map(|c| ComponentId {
                owner: c.value.header.owner as i16,
                component_type: 2,
                id: c.value.header.id,
            });
        let Some(origin) = origin.filter(|c| i32::from(c.owner) == unit.e_owner) else {
            return Ok(());
        };
        let major = self
            .current
            .queries
            .civilization_levels
            .and_then(|rows| rows.iter().find(|r| r.0 == owner))
            .ok_or("idle Trader current civilization level")?
            .1
            == crate::registry::linux_type_hash(b"CIVILIZATION_LEVEL_FULL_CIV");
        let (water_allowed, domains) = {
            let placement = self
                .current
                .queries
                .placement_context(state, self.current.placement.placement)
                .map_err(|e| format!("idle Trader placement {e:?}"))?;
            (
                placement
                    .can_embark(unit)
                    .map_err(|e| format!("idle Trader embark {e:?}"))?,
                placement
                    .city_domain_masks()
                    .ok_or("idle Trader city domains")?
                    .to_vec(),
            )
        };
        let mut destinations = Vec::new();
        for player in &state.retained().players {
            if !state
                .player_alive(player.header.owner)
                .map_err(|e| format!("idle Trader alive player {e:?}"))?
            {
                continue;
            }
            for city in &player.cities.records.entries {
                let id = ComponentId {
                    owner: city.value.header.owner as i16,
                    component_type: 2,
                    id: city.value.header.id,
                };
                if id != origin {
                    destinations.push((
                        id,
                        PlotCoord {
                            x: city.value.header.location_x,
                            y: city.value.header.location_y,
                        },
                    ));
                }
            }
        }
        let mut search = self
            .trade_search
            .try_borrow_mut()
            .map_err(|e| format!("idle Trader search {e}"))?;
        for (destination, at) in destinations {
            let active = state
                .trade_graph()
                .map_err(|e| format!("{e:?}"))?
                .graph()
                .get_edge(city_vertex(origin), city_vertex(destination))
                .ok_or("idle Trader new graph edge allocation")?
                .has_active_route;
            if active {
                continue;
            }
            if let Some(nodes) = path::find(
                state,
                self.path_definitions,
                path::Inputs {
                    owner,
                    major,
                    water_allowed,
                    domains: &domains,
                    movement: &self.current.path.units,
                },
                &mut search,
                location,
                at,
            )
            .map_err(|e| format!("idle Trader path {e:?}"))?
            {
                state
                    .trade_graph_mut()
                    .map_err(|e| format!("{e:?}"))?
                    .replace_inactive_path(origin, destination, nodes)
                    .ok_or("idle Trader inactive graph path store")?;
            }
        }
        Ok(())
    }

    fn advance(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, OperationParameters>,
        frame: &mut Frame,
        readiness: &actions::reporting::Provider<'_>,
    ) -> Result<(), String> {
        let mut sight = self
            .current
            .sight
            .try_borrow_mut()
            .map_err(|e| format!("queued trade sight: {e}"))?;
        let mut search = self
            .current
            .search
            .try_borrow_mut()
            .map_err(|e| format!("queued trade placement search: {e}"))?;
        let mut trade_search = self
            .trade_search
            .try_borrow_mut()
            .map_err(|e| format!("queued trade route search: {e}"))?;
        let mut movement = move_to::ComputedMovement {
            path: self.current.path,
            placement: self.current.placement,
            queries: self.current.queries,
            search: &mut search,
            execution: crate::formation_path_state::ExecutionContext::Exclusive,
        };
        let mut continuations = actions::trade_route_actions::stock::Ongoing {
            lifetime: self.lifetime,
            roads: self.roads,
            ending: self.ending,
            ending_reports: self.ending_reports.as_deref_mut(),
        };
        let mut movement = actions::trade_route_actions::current::Current {
            movement: &mut movement,
            definitions: self.definitions,
            path_definitions: self.path_definitions,
            search: &mut trade_search,
            manager: None,
            continuations: &mut continuations,
        };
        let result = automation::advance_player_unit(
            state,
            frame,
            readiness,
            self.current.inputs,
            self.current.handlers,
            &mut move_to::Runtime {
                visibility: &mut sight,
                manager_width: self.current.manager_width,
            },
            backend,
            &mut movement,
            automation::Scope::StockLinuxXp2RegisteredHandlers,
            Some(c6_core::loaded_state::game_update::deactivation::LoggingScope::StockFileSink),
        )
        .map_err(|e| format!("queued trade unit: {e:?}"))?;
        match result {
            automation::UnitProgress::Advanced => Ok(()),
            automation::UnitProgress::NoUnit => {
                Err("queued trade Unit disappeared before continuation".into())
            }
        }
    }
}
