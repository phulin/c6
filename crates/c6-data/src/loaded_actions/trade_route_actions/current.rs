//! Trade initiation on the same computed placement, queue, and Session backend.
//! The continuation owner dispatches actual subscribed gameplay effects; a
//! missing stock subscriber cannot be replaced with a successful empty service.
use super::{path, round_trade, Definitions, Error, OperationSite, Pathfinder};
use crate::{
    loaded_actions::move_to as queued, loaded_upkeep, unit_placement_state, unit_sight_state,
};
use c6_core::{
    loaded_state::LoadedState,
    pathfinder::TwoLayer,
    skip_turn,
    trade::ComponentId,
    unit::{OperationNode, OperationParameters, PlotCoord, Unit},
};

#[derive(Clone, Copy, Debug)]
pub enum Event {
    TraderRouteChanged(ComponentId),
    DiplomaticVisibilityChanged {
        owner: i32,
        other: i32,
    },
    ResourceStarted {
        origin: ComponentId,
        destination: ComponentId,
    },
    ImmediateTradingPost {
        origin: ComponentId,
        destination: ComponentId,
    },
    RouteStarted {
        origin: ComponentId,
        destination: ComponentId,
    },
    ManagerReports {
        actor: (i32, u32),
        origin: ComponentId,
        destination: ComponentId,
    },
    HandlerReports {
        actor: (i32, u32),
    },
}
pub trait Continuations<E> {
    fn expire_route(
        &mut self,
        _state: &mut LoadedState,
        _backend: &mut E,
        _origin: ComponentId,
        _destination: ComponentId,
    ) -> Result<Option<()>, String> {
        Ok(None)
    }

    fn earliest_end_turn(
        &self,
        _state: &LoadedState,
        _push_turn: i32,
    ) -> Result<Option<i32>, String> {
        Ok(None)
    }

    fn event(
        &mut self,
        state: &mut LoadedState,
        backend: &mut E,
        event: Event,
    ) -> Result<(), String>;
    fn road(
        &mut self,
        state: &mut LoadedState,
        backend: &mut E,
        sight: &unit_sight_state::Runtime,
        plot: u32,
        owner: i32,
    ) -> Result<(), String>;
    fn resource_started(
        &mut self,
        state: &mut LoadedState,
        backend: &mut E,
        manager: &mut round_trade::Runtime,
        origin: ComponentId,
        destination: ComponentId,
    ) -> Result<(), String>;
}
pub struct Current<'s, 'a, 'r, S> {
    pub movement: &'s mut queued::ComputedMovement<'a, 'r>,
    pub definitions: &'s Definitions,
    pub path_definitions: &'s path::Definitions,
    pub search: &'s mut TwoLayer,
    /// Required only when creating a new route. Ongoing queue execution uses
    /// the graph and leaves the canonical round-upkeep manager with its owner.
    pub manager: Option<&'s mut round_trade::Runtime>,
    pub continuations: &'s mut S,
}
#[derive(Debug)]
pub enum Failure<E> {
    Movement(queued::ComputedError<E>),
    Trade(Error<String>),
    Missing(&'static str),
    Placement(crate::unit_placement_state::Error<E>),
    Finish(c6_core::movement_execution::FormationMovementError<loaded_upkeep::Error<E>>),
    Deactivate(loaded_upkeep::Error<E>),
}
fn major(movement: &queued::ComputedMovement<'_, '_>, owner: i32) -> Result<bool, &'static str> {
    let level = movement
        .queries
        .civilization_levels
        .and_then(|rows| rows.iter().find(|r| r.0 == owner))
        .map(|r| r.1)
        .ok_or("current Trade civilization level")?;
    Ok(level == crate::registry::linux_type_hash(b"CIVILIZATION_LEVEL_FULL_CIV"))
}
struct Effects<'e, E, S> {
    backend: &'e mut E,
    continuations: &'e mut S,
}
impl<E, S: Continuations<E>> super::Effects for Effects<'_, E, S> {
    type Error = String;
    fn trader_route_changed(
        &mut self,
        state: &mut LoadedState,
        trader: ComponentId,
    ) -> Result<(), String> {
        self.continuations
            .event(state, self.backend, Event::TraderRouteChanged(trader))
    }
    fn resource_route_started(
        &mut self,
        state: &mut LoadedState,
        manager: &mut round_trade::Runtime,
        origin: ComponentId,
        destination: ComponentId,
    ) -> Result<(), String> {
        self.continuations
            .resource_started(state, self.backend, manager, origin, destination)
    }
    fn immediate_trading_post(
        &mut self,
        state: &mut LoadedState,
        origin: ComponentId,
        destination: ComponentId,
    ) -> Result<(), String> {
        self.continuations.event(
            state,
            self.backend,
            Event::ImmediateTradingPost {
                origin,
                destination,
            },
        )
    }
    fn route_started(
        &mut self,
        state: &mut LoadedState,
        origin: ComponentId,
        destination: ComponentId,
    ) -> Result<(), String> {
        self.continuations.event(
            state,
            self.backend,
            Event::RouteStarted {
                origin,
                destination,
            },
        )
    }
    fn route_reports(
        &mut self,
        state: &mut LoadedState,
        actor: (i32, u32),
        origin: ComponentId,
        destination: ComponentId,
    ) -> Result<(), String> {
        self.continuations.event(
            state,
            self.backend,
            Event::ManagerReports {
                actor,
                origin,
                destination,
            },
        )
    }
}
impl<E, S> queued::Movement<E> for Current<'_, '_, '_, S>
where
    E: unit_placement_state::Effects
        + loaded_upkeep::FormationRestoreEffects<Error = unit_placement_state::EffectsError<E>>,
    S: Continuations<E>,
{
    type Error = Failure<unit_placement_state::EffectsError<E>>;
    fn pop_expired_trade_route(&mut self,state:&mut LoadedState,actor:(i32,u32),effects:&mut E)->Result<Option<()>,Self::Error> {
        loaded_upkeep::pop_front_expired_trade_route(state,actor.0,actor.1,&crate::unit_movement_state::Inputs{definitions:&self.movement.path.units},effects).map_err(Failure::Deactivate)?;
        Ok(Some(()))
    }
    fn trade_can_start(
        &mut self,
        state: &LoadedState,
        unit: &Unit,
        parameters: &OperationParameters,
        visible_only: bool,
        _: &mut E,
    ) -> Result<Option<bool>, Self::Error> {
        let major = major(self.movement, unit.e_owner).map_err(Failure::Missing)?;
        let placement = self
            .movement
            .queries
            .placement_context(state, self.movement.placement.placement)
            .map_err(|_| Failure::Missing("Trade computed placement"))?;
        let water_allowed = placement
            .can_embark(unit)
            .map_err(|_| Failure::Missing("Trade Player.Units.CanEmbark"))?;
        let domains = placement
            .city_domain_masks()
            .ok_or(Failure::Missing("Trade computed city domains"))?;
        let mut pathfinder = Pathfinder {
            civilization_levels: self.movement.queries.civilization_levels
                .ok_or(Failure::Missing("Trade civilization levels"))?,
            search: self.search,
            definitions: self.path_definitions,
            inputs: path::Inputs {
                owner: unit.e_owner,
                major,
                water_allowed,
                domains,
                movement: &self.movement.path.units,
            },
        };
        super::handler_can_start(
            state,
            self.definitions,
            unit,
            parameters,
            visible_only,
            major,
            OperationSite::CurrentFront,
            Some(&mut pathfinder),
        )
        .map(Some)
        .map_err(Failure::Trade)
    }
    fn start_trade(
        &mut self,
        state: &mut LoadedState,
        actor: (i32, u32),
        node: &OperationNode,
        backend: &mut E,
    ) -> Result<Option<skip_turn::StartData>, Self::Error> {
        let OperationParameters::MakeTradeRoute {
            location0: destination,
            resource_hash,
            ..
        } = node.parameters
        else {
            return Err(Failure::Missing("Trade Start parameters"));
        };
        if resource_hash != u32::MAX {
            return Err(Failure::Missing("attached Trade resource"));
        }
        let major = major(self.movement, actor.0).map_err(Failure::Missing)?;
        let unit = state
            .world()
            .player(actor.0)
            .map_err(|_| Failure::Missing("Trade owner"))?
            .unit(actor.1)
            .map_err(|_| Failure::Missing("Trade actor"))?
            .ok_or(Failure::Missing("Trade actor"))?
            .record;
        let placement = self
            .movement
            .queries
            .placement_context(state, self.movement.placement.placement)
            .map_err(|_| Failure::Missing("Trade computed placement"))?;
        let water_allowed = placement
            .can_embark(unit)
            .map_err(|_| Failure::Missing("Trade Player.Units.CanEmbark"))?;
        let domains = placement
            .city_domain_masks()
            .ok_or(Failure::Missing("Trade computed city domains"))?
            .to_vec();
        drop(placement);
        let mut pathfinder = Pathfinder {
            civilization_levels: self.movement.queries.civilization_levels
                .ok_or(Failure::Missing("Trade civilization levels"))?,
            search: self.search,
            definitions: self.path_definitions,
            inputs: path::Inputs {
                owner: actor.0,
                major,
                water_allowed,
                domains: &domains,
                movement: &self.movement.path.units,
            },
        };
        let mut effects = Effects {
            backend,
            continuations: self.continuations,
        };
        let _ = super::start(
            state,
            self.definitions,
            self.manager
                .as_deref_mut()
                .ok_or(Failure::Missing("Trade initiation manager owner"))?,
            actor,
            destination,
            major,
            &mut effects,
            Some(&mut pathfinder),
        )
        .map_err(Failure::Trade)?;
        // XP2 Start queries the destination owner after StartRoute. This
        // major-player branch omits only the unreachable city-state bonus.
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == actor.0)
            .ok_or(Failure::Missing("Trade destination owner"))?;
        if player.trade.anyone_trade_to_favor > 0 {
            return Err(Failure::Missing("positive Trade diplomatic favor"));
        }
        effects
            .continuations
            .event(state, effects.backend, Event::HandlerReports { actor })
            .map_err(|e| Failure::Trade(Error::Effect(e)))?;
        Ok(Some(skip_turn::StartData {
            delete: false,
            action: true,
        }))
    }
    fn trade_expire(
        &mut self,
        state: &mut LoadedState,
        origin: ComponentId,
        destination: ComponentId,
        backend: &mut E,
    ) -> Result<Option<()>, Self::Error> {
        self.continuations
            .expire_route(state, backend, origin, destination)
            .map_err(|e| Failure::Trade(Error::Effect(e)))
    }
    fn trade_earliest_end_turn(
        &self,
        state: &LoadedState,
        push_turn: i32,
    ) -> Result<Option<i32>, Self::Error> {
        self.continuations
            .earliest_end_turn(state, push_turn)
            .map_err(|e| Failure::Trade(Error::Effect(e)))
    }
    fn trade_move(
        &mut self,
        state: &mut LoadedState,
        sight: &mut unit_sight_state::Runtime,
        formation: (i32, u32),
        destination: PlotCoord,
        backend: &mut E,
    ) -> Result<Option<()>, Self::Error> {
        let record = state
            .world()
            .player(formation.0)
            .map_err(|_| Failure::Missing("Trade formation owner"))?
            .formation(formation.1)
            .map_err(|_| Failure::Missing("Trade formation"))?
            .ok_or(Failure::Missing("Trade formation"))?
            .record;
        if record.moves_remaining_raw <= 0 {
            return Ok(Some(()));
        }
        let owner = record.owner;
        let count = record.unit_ids.len();
        for index in 0..count {
            let record = state
                .world()
                .player(formation.0)
                .map_err(|_| Failure::Missing("Trade retained formation owner"))?
                .formation(formation.1)
                .map_err(|_| Failure::Missing("Trade retained formation"))?
                .ok_or(Failure::Missing("Trade retained formation"))?
                .record;
            let id = *record
                .unit_ids
                .get(index)
                .ok_or(Failure::Missing("Trade invalidated member cursor"))?;
            if loaded_upkeep::UnitHandle::resolve(state, owner, id)
                .map_err(|_| Failure::Missing("Trade member lookup"))?
                .is_none()
            {
                continue;
            }
            let result = unit_placement_state::set(
                state,
                self.movement.placement,
                self.movement.queries,
                sight,
                backend,
                unit_placement_state::Request {
                    actor: (owner, id),
                    destination,
                    validation_flags: 2,
                },
            )
            .map_err(Failure::Placement)?;
            if result != 1 {
                break;
            }
        }
        loaded_upkeep::finish_formation_moves(
            state,
            formation.0,
            formation.1,
            &crate::unit_movement_state::Inputs {
                definitions: &self.movement.path.units,
            },
            backend,
        )
        .map_err(Failure::Finish)?;
        let (index, _) = c6_core::movement_world::plot_at(state.world().map(), destination)
            .map_err(|_| Failure::Missing("Trade moved plot"))?;
        if self.definitions.create_road {
            self.continuations
                .road(state, backend, sight, index as u32, owner)
                .map_err(|e| Failure::Trade(Error::Effect(e)))?;
        }
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Failure::Missing("Trade moved Player"))?;
        if player.trade.gain_tiles_en_route_radius > 0 {
            let plot = &state.world().map().plots[index];
            if plot
                .city_attributes
                .as_ref()
                .map_or(-1, |a| i32::from(a.player))
                == -1
            {
                return Err(Failure::Missing(
                    "Trade gain tiles en route nearest city/acquisition",
                ));
            }
        }
        Ok(Some(()))
    }
    fn fortify_turn_max(&self) -> i32 {
        <queued::ComputedMovement<'_, '_> as queued::Movement<E>>::fortify_turn_max(self.movement)
    }
    fn is_fortifyable(&self, state: &LoadedState, actor: (i32, u32)) -> Result<bool, Self::Error> {
        <queued::ComputedMovement<'_, '_> as queued::Movement<E>>::is_fortifyable(
            self.movement,
            state,
            actor,
        )
        .map_err(Failure::Movement)
    }
    fn alert_enemy_visible(
        &self,
        state: &LoadedState,
        sight: &unit_sight_state::Runtime,
        actor: (i32, u32),
    ) -> Result<bool, Self::Error> {
        <queued::ComputedMovement<'_, '_> as queued::Movement<E>>::alert_enemy_visible(
            self.movement,
            state,
            sight,
            actor,
        )
        .map_err(Failure::Movement)
    }
    fn city_at(
        &self,
        state: &LoadedState,
        at: PlotCoord,
    ) -> Result<Option<ComponentId>, Self::Error> {
        super::city(state, at).map_err(Failure::Trade)
    }
    fn preflight(
        &self,
        state: &LoadedState,
        actor: (i32, u32),
        request: queued::Request,
    ) -> Result<(), Self::Error> {
        <queued::ComputedMovement<'_, '_> as queued::Movement<E>>::preflight(
            self.movement,
            state,
            actor,
            request,
        )
        .map_err(Failure::Movement)
    }
    fn test_move_to(
        &mut self,
        state: &mut LoadedState,
        sight: &unit_sight_state::Runtime,
        formation: (i32, u32),
        at: PlotCoord,
        flags: u32,
    ) -> Result<i32, Self::Error> {
        <queued::ComputedMovement<'_, '_> as queued::Movement<E>>::test_move_to(
            self.movement,
            state,
            sight,
            formation,
            at,
            flags,
        )
        .map_err(Failure::Movement)
    }
    fn move_to(
        &mut self,
        state: &mut LoadedState,
        sight: &mut unit_sight_state::Runtime,
        formation: (i32, u32),
        at: PlotCoord,
        flags: u32,
        effects: &mut E,
    ) -> Result<i32, Self::Error> {
        self.movement
            .move_to(state, sight, formation, at, flags, effects)
            .map_err(Failure::Movement)
    }
    fn valid_location(
        &mut self,
        state: &LoadedState,
        sight: &unit_sight_state::Runtime,
        actor: (i32, u32),
        at: PlotCoord,
    ) -> Result<i32, Self::Error> {
        <queued::ComputedMovement<'_, '_> as queued::Movement<E>>::valid_location(
            self.movement,
            state,
            sight,
            actor,
            at,
        )
        .map_err(Failure::Movement)
    }
}
