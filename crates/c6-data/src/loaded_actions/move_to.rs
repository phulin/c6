//! Mutable queued MoveTo execution over authoritative loaded records.
//! This internal action pipeline preserves completed callback effects on errors;
//! complete outer message, diplomacy/notification and RNG parity is separate.
mod alert;
pub mod combat;
mod trade;

use c6_core::{
    skip_turn::legality::ManagerOperation,
    unit::{OperationParameters, PlotCoord},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub destination: PlotCoord,
    pub modifiers: u32,
    pub directive: i32,
    pub flags: i32,
    pub append: bool,
    pub from_user: bool,
}
impl Request {
    pub fn operation(self, runtime_id: i32) -> ManagerOperation<OperationParameters> {
        ManagerOperation {
            runtime_id,
            directive: self.directive,
            flags: self.flags,
            parameters: OperationParameters::MoveTo {
                location: self.destination,
                modifiers: self.modifiers,
            },
        }
    }
}

use super::{handlers::BoundHandlers, IdentityReport, Inputs};
use crate::{
    loaded_upkeep::{self, OperationReports, UnitHandle},
    unit_sight_state,
};
use c6_core::{
    loaded_state::LoadedState,
    map::UnitReference,
    skip_turn::{
        self,
        legality::{self, ManagerQueries},
    },
    unit::{ComponentId, Formation, OperationNode, Unit, MOVE_TO_HASH},
};

pub type EffectError<E> = <E as OperationReports>::Error;

/// Read the same current service owner used by Placement; no detached alive
/// list or per-viewer width snapshot is introduced by queue processing.
pub trait Effects: OperationReports {
    fn services(
        &self,
    ) -> Result<
        &crate::unit_placement_services::Runtime,
        crate::unit_placement_services::Error<Self::Error>,
    >;
}
impl<T> Effects for T
where
    T: OperationReports
        + crate::unit_placement_services::Services<Error = <T as OperationReports>::Error>,
{
    fn services(
        &self,
    ) -> Result<
        &crate::unit_placement_services::Runtime,
        crate::unit_placement_services::Error<Self::Error>,
    > {
        crate::unit_placement_services::Services::placement_service_runtime(self)
    }
}

/// Current lower movement and stock queued-handler scalar queries. Implementations must call the
/// computed Formation.TestMoveTo/MoveTo and actual Unit.IsValidLocation on the
/// same current state, including their synchronous effects. These are not path
/// answers or whole MoveTo-handler callbacks. A concrete production composition
/// can retain its search/definitions here while this module owns queue control.
pub trait Movement<E: OperationReports> {
    type Error;
    /// Opt-in exact FoundCity composition. The default leaves this operation
    /// unsupported before Add/Clear; it never supplies a success/no-op handler.
    fn found_city_rules(&self) -> Option<(&crate::found_city::Definitions, &crate::Rules)> {
        None
    }
    fn start_found_city(
        &mut self,
        _state: &mut LoadedState,
        _visibility: &mut unit_sight_state::Runtime,
        _actor: (i32, u32),
        _effects: &mut E,
    ) -> Result<Option<skip_turn::StartData>, Self::Error> {
        Ok(None)
    }
    /// Source TestAttackInto on the CURRENT next cached path node. None means
    /// this provider has not composed the combat kernel, never legal=false.
    fn test_attack_into(
        &mut self,
        _state: &mut LoadedState,
        _visibility: &unit_sight_state::Runtime,
        _actor: (i32, u32),
        _destination: PlotCoord,
        _effects: &E,
    ) -> Result<Option<bool>, Self::Error> {
        Ok(None)
    }
    /// Complete the actual AttackInto/Generate/Resolve call on the same state
    /// and persistent effects owner. Some contains its native bool-out value;
    /// an unfinished combat continuation must return an error, not Some.
    fn attack_into(
        &mut self,
        _state: &mut LoadedState,
        _visibility: &mut unit_sight_state::Runtime,
        _actor: (i32, u32),
        _destination: PlotCoord,
        _effects: &mut E,
    ) -> Result<Option<bool>, Self::Error> {
        Ok(None)
    }
    /// Optional ordinary Farm handler over the caller's retained rule/runtime owners.
    fn farm_can_start(&mut self, _state: &LoadedState, _unit: &Unit,
        _improvement: u32, _visible_only: bool, _effects: &E)
        -> Result<Option<bool>, Self::Error> { Ok(None) }
    fn start_farm(&mut self, _state: &mut LoadedState,
        _visibility: &mut unit_sight_state::Runtime, _actor: (i32, u32),
        _node: &OperationNode, _effects: &mut E)
        -> Result<Option<skip_turn::StartData>, Self::Error> { Ok(None) }
    fn range_can_start(
        &mut self,
        _state: &LoadedState,
        _sight: &unit_sight_state::Runtime,
        _actor: (i32, u32),
        _destination: PlotCoord,
        _visible_only: bool,
        _effects: &E,
    ) -> Result<Option<bool>, Self::Error> {
        Ok(None)
    }
    fn range_test(
        &mut self,
        _state: &mut LoadedState,
        _sight: &unit_sight_state::Runtime,
        _actor: (i32, u32),
        _destination: PlotCoord,
        _effects: &E,
    ) -> Result<Option<bool>, Self::Error> {
        Ok(None)
    }
    fn range_attack(
        &mut self,
        _state: &mut LoadedState,
        _sight: &mut unit_sight_state::Runtime,
        _actor: (i32, u32),
        _destination: PlotCoord,
        _effects: &mut E,
    ) -> Result<Option<bool>, Self::Error> {
        Ok(None)
    }
    fn fortify_turn_max(&self) -> i32;
    fn is_fortifyable(&self, state: &LoadedState, actor: (i32, u32)) -> Result<bool, Self::Error>;
    fn alert_enemy_visible(
        &self,
        state: &LoadedState,
        visibility: &unit_sight_state::Runtime,
        actor: (i32, u32),
    ) -> Result<bool, Self::Error>;
    /// Current CManager.EditCityAt, including CityAttributes::IsCity and the
    /// current logical owner/ID lookup. No initial city-location answer cache.
    fn city_at(
        &self,
        state: &LoadedState,
        location: PlotCoord,
    ) -> Result<Option<c6_core::trade::ComponentId>, Self::Error>;
    fn preflight(
        &self,
        state: &LoadedState,
        actor: (i32, u32),
        request: Request,
    ) -> Result<(), Self::Error>;
    fn test_move_to(
        &mut self,
        state: &mut LoadedState,
        visibility: &unit_sight_state::Runtime,
        formation: (i32, u32),
        destination: PlotCoord,
        flags: u32,
    ) -> Result<i32, Self::Error>;
    fn move_to(
        &mut self,
        state: &mut LoadedState,
        visibility: &mut unit_sight_state::Runtime,
        formation: (i32, u32),
        destination: PlotCoord,
        flags: u32,
        effects: &mut E,
    ) -> Result<i32, Self::Error>;
    fn valid_location(
        &mut self,
        state: &LoadedState,
        visibility: &unit_sight_state::Runtime,
        actor: (i32, u32),
        location: PlotCoord,
    ) -> Result<i32, Self::Error>;
}
/// Current runtime fields, not native predicate results. The alive registry is
/// the actual ordered GetAlive vector; retained player storage/order must remain
/// valid through this finite call. Shared arrays are the same mutable owner used
/// by Placement/Sight. Widths are raw Visibility+0xd0 values, not save group0.
pub struct Runtime<'a> {
    pub visibility: &'a mut unit_sight_state::Runtime,
    pub manager_width: u32,
}
#[derive(Debug)]
pub enum Error<E, M> {
    Query(super::Error),
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Upkeep(loaded_upkeep::Error<E>),
    Effect(E),
    Movement(M),
    Unsupported(&'static str),
    Runtime(&'static str),
    Services(crate::unit_placement_services::Error<E>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Applied {
    /// Native Add can leave an operation queued while busy, or stop the unit
    /// until a later turn. Neither condition runs a native AI policy here.
    pub queued: bool,
    pub delayed_death: bool,
}
fn unit<E, M>(state: &LoadedState, actor: (i32, u32)) -> Result<&Unit, Error<E, M>> {
    state
        .world()
        .player(actor.0)
        .map_err(Error::World)?
        .unit(actor.1)
        .map_err(Error::World)?
        .map(|u| u.record)
        .ok_or(Error::Unsupported("retained MoveTo actor removed"))
}
fn identity<E, M>(state: &LoadedState, actor: (i32, u32)) -> Result<ComponentId, Error<E, M>> {
    let u = unit(state, actor)?;
    Ok(ComponentId {
        owner: u.e_owner as i16,
        component_type: 1,
        id: u.id,
    })
}
fn formation_id<E, M>(state: &LoadedState, actor: (i32, u32)) -> Result<(i32, u32), Error<E, M>> {
    let u = unit(state, actor)?;
    state
        .unit_runtime(u.e_owner, u.id)
        .map_err(Error::State)?
        .formation
        .map(|f| (f.owner, f.requested_id))
        .ok_or(Error::Unsupported("MoveTo requires current Formation"))
}
fn formation<E, M>(state: &LoadedState, id: (i32, u32)) -> Result<&Formation, Error<E, M>> {
    state
        .world()
        .player(id.0)
        .map_err(Error::World)?
        .formation(id.1)
        .map_err(Error::World)?
        .map(|f| f.record)
        .ok_or(Error::Unsupported("retained MoveTo Formation removed"))
}
fn emit<E: OperationReports, M>(
    state: &mut LoadedState,
    actor: (i32, u32),
    effects: &mut E,
    event: u32,
    first: Option<i32>,
    second: Option<i32>,
) -> Result<(), Error<EffectError<E>, M>> {
    let report = IdentityReport {
        event,
        subtype: 0,
        identity: identity(state, actor)?,
        first,
        second,
    };
    let handle = UnitHandle::resolve(state, actor.0, actor.1)
        .map_err(Error::World)?
        .ok_or(Error::Unsupported("MoveTo report actor"))?;
    effects
        .preflight_operation_report(state, handle, report)
        .map_err(Error::Effect)?;
    effects
        .operation_report(state, report)
        .map_err(Error::Effect)?;
    Ok(())
}
fn target<E, M>(node: &OperationNode) -> Result<(PlotCoord, u32), Error<E, M>> {
    if !matches!(
        node.operation_hash,
        MOVE_TO_HASH | c6_core::unit::RANGE_ATTACK_HASH
    ) {
        return Err(Error::Unsupported("callback selected another handler"));
    }
    match node.parameters {
        OperationParameters::MoveTo {
            location,
            modifiers,
        } => Ok((location, modifiers)),
        _ => Err(Error::Unsupported("MoveTo SingleLocation parameters")),
    }
}
fn front<E, M>(state: &LoadedState, actor: (i32, u32)) -> Result<OperationNode, Error<E, M>> {
    unit(state, actor)?
        .k_operations
        .nodes
        .first()
        .cloned()
        .ok_or(Error::Unsupported("required queue front removed"))
}
fn visible_index<E, M>(runtime: &Runtime<'_>, owner: i32, index: u32) -> Result<bool, Error<E, M>> {
    let layers = runtime.visibility.shared(owner).map_err(Error::Runtime)?;
    let normal = layers
        .0
        .iter()
        .find(|layer| layer.type_hash == c6_core::visibility_change::BASE_LAYER_HASH)
        .ok_or(Error::Runtime("initialized shared normal visibility layer"))?;
    if index >= normal.count {
        return Ok(false);
    }
    Ok(*normal
        .values
        .get(index as usize)
        .ok_or(Error::Runtime("partial in-range shared counts"))?
        > 0)
}
fn visible<E, M>(
    state: &LoadedState,
    runtime: &Runtime<'_>,
    owner: i32,
    location: PlotCoord,
) -> Result<bool, Error<E, M>> {
    // Start uses the unsigned-index overload after GetTargetPlot.GetIndex.
    // It does not multiply by the individual Visibility instance's width.
    let (index, _) = c6_core::movement_world::plot_at(state.world().map(), location)
        .map_err(|_| Error::Runtime("current MoveTo target plot index"))?;
    visible_index(runtime, owner, index as u32)
}
fn any_human_visible<E: Effects, M>(
    state: &LoadedState,
    runtime: &Runtime<'_>,
    effects: &E,
    location: PlotCoord,
) -> Result<bool, Error<EffectError<E>, M>> {
    let index = runtime
        .manager_width
        .wrapping_mul(location.y as u32)
        .wrapping_add(location.x as u32);
    let (token, end) = effects
        .services()
        .map_err(Error::Services)?
        .alive_begin()
        .map_err(Error::Services)?;
    for at in 0..end {
        let owner = effects
            .services()
            .map_err(Error::Services)?
            .alive_at(token, at)
            .map_err(Error::Services)?;
        if state.player_slot_status(owner).map_err(Error::State)? != 3 {
            continue;
        }
        if visible_index(runtime, owner, index)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn evaluating<E, M>(
    state: &mut LoadedState,
    id: (i32, u32),
    value: bool,
) -> Result<(), Error<E, M>> {
    let (f, r) = state
        .formation_with_runtime_mut(id.0, id.1)
        .map_err(Error::State)?;
    r.set_evaluating_operation(f, value)
        .map_err(|_| Error::Unsupported("retained Formation evaluating identity"))
}
/// Source-closed no-candidate branch of Combat.IsEnemyAt: complete empty unit
/// membership in every layer and no district identity. A nonempty combat/district
/// branch is explicit until its actual effective defender logic is available.
fn enemy_at<E, M>(state: &LoadedState, location: PlotCoord) -> Result<bool, Error<E, M>> {
    let (index, plot) = c6_core::movement_world::plot_at(state.world().map(), location)
        .map_err(|_| Error::Unsupported("MoveTo enemy target plot"))?;
    for layer in &state.world().map().units.layers {
        if !state
            .world()
            .plot_units(layer.layer_id, index)
            .map_err(Error::World)?
            .is_empty()
        {
            return Err(Error::Unsupported(
                "MoveTo IsEnemyAt reached defender selection",
            ));
        }
    }
    if plot
        .city_attributes
        .as_ref()
        .is_some_and(|a| a.district != u32::MAX && a.player != -1)
    {
        return Err(Error::Unsupported(
            "MoveTo IsEnemyAt reached district selection",
        ));
    }
    Ok(false)
}
pub(crate) fn friendly_count<E, M>(
    state: &LoadedState,
    actor: (i32, u32),
    inputs: &Inputs<'_, '_>,
) -> Result<u32, Error<E, M>> {
    let owned = if state.game_relations_source().is_some() {
        let r = state.game_relations().map_err(Error::State)?;
        let war = r.war().ok_or(Error::Unsupported("owned war types"))?;
        Some(skip_turn::placement::WarRelations {
            player_count: r.player_count(),
            war_types: war.values(),
        })
    } else {
        None
    };
    let q = skip_turn::placement::Inputs {
        world: state.world(),
        definitions: inputs.placement_definitions,
        relations: owned.as_ref().or(inputs.relations),
        plot_unit_limit: inputs.plot_unit_limit,
    };
    let actor = unit(state, actor)?;
    let members = skip_turn::placement::layered_units(state.world(), actor)
        .map_err(|e| Error::Query(super::Error::Placement(e)))?;
    q.unit_count(actor, members)
        .map_err(|e| Error::Query(super::Error::Placement(e)))
}

/// The source parameterless FoundCity Add path. Runtime ID is resolved from
/// the admitted handler registry; no captured CanStart answer is used.
#[allow(clippy::too_many_arguments)]
pub fn execute_found_city_manager<E, M>(
    state: &mut LoadedState,
    actor: (i32, u32),
    inputs: &Inputs<'_, '_>,
    handlers: &BoundHandlers<'_>,
    runtime: &mut Runtime<'_>,
    effects: &mut E,
    movement: &mut M,
) -> Result<Applied, Error<EffectError<E>, M::Error>>
where
    E: Effects + ManagerQueries<Unit, Parameters = OperationParameters, Error = &'static str>,
    M: Movement<E>,
{
    if movement.found_city_rules().is_none() {
        return Err(Error::Unsupported("FoundCity composition unavailable"));
    }
    execute_immediate_manager(state, actor, c6_core::unit::FOUND_CITY_HASH,
        OperationParameters::None, inputs, handlers, runtime, effects, movement)
}
/// Ordinary typed BuildImprovement Add/Activate on the same queue and runtime.
#[allow(clippy::too_many_arguments)]
pub fn execute_farm_manager<E, M>(
    state: &mut LoadedState, actor: (i32, u32), improvement_hash: u32,
    modifiers: u32, inputs: &Inputs<'_, '_>, handlers: &BoundHandlers<'_>,
    runtime: &mut Runtime<'_>, effects: &mut E, movement: &mut M,
) -> Result<Applied, Error<EffectError<E>, M::Error>>
where E: Effects + ManagerQueries<Unit, Parameters = OperationParameters, Error = &'static str>,
    M: Movement<E>,
{
    execute_immediate_manager(state, actor, c6_core::unit::BUILD_IMPROVEMENT_HASH,
        OperationParameters::BuildImprovement { modifiers, improvement_hash },
        inputs, handlers, runtime, effects, movement)
}
#[allow(clippy::too_many_arguments)]
fn execute_immediate_manager<E, M>(
    state: &mut LoadedState, actor: (i32, u32), hash: u32,
    parameters: OperationParameters, inputs: &Inputs<'_, '_>, handlers: &BoundHandlers<'_>,
    runtime: &mut Runtime<'_>, effects: &mut E, movement: &mut M,
) -> Result<Applied, Error<EffectError<E>, M::Error>>
where E: Effects + ManagerQueries<Unit, Parameters = OperationParameters, Error = &'static str>,
    M: Movement<E>,
{
    handlers
        .validate_execution(state, inputs.handlers, inputs.operations)
        .map_err(|e| Error::Query(super::Error::HandlerObservation(e)))?;
    let id = handlers
        .runtime_id(hash)
        .map_err(|e| Error::Query(super::Error::HandlerObservation(e)))?
        .0;
    loaded_upkeep::clear_unit_operations(state, actor.0, actor.1, effects)
        .map_err(Error::Upkeep)?;
    let current = unit(state, actor)?;
    let node = OperationNode {
        operation_hash: hash,
        directive: -1,
        flags: 0,
        push_turn: state.retained().core.current_turn,
        previous_activity: current.e_activity,
        previous_automation: current.e_automation,
        parameters,
    };
    state
        .unit_mut(actor.0, actor.1)
        .map_err(Error::State)?
        .k_operations
        .nodes
        .push(node);
    emit(state, actor, effects, 0x15c35863, Some(id), None)?;
    if unit(state, actor)?.k_operations.nodes.len() == 1 {
        activate(state, actor, inputs, handlers, runtime, effects, movement)?;
    }
    Ok(Applied {
        queued: !unit(state, actor)?.k_operations.nodes.is_empty(),
        delayed_death: unit(state, actor)?.b_delayed_death,
    })
}

/// Execute real Manager.Add/Activate for the finite ordinary MoveTo request.
/// Callers must finish outer UnitOperation diplomacy/notification/message tails
/// before accepting a whole action. Errors preserve completed native-order
/// effects; this live leaf is deliberately not a transaction or Engine::apply.
#[allow(clippy::too_many_arguments)]
pub fn execute_manager<E, M>(
    state: &mut LoadedState,
    actor: (i32, u32),
    request: Request,
    inputs: &Inputs<'_, '_>,
    handlers: &BoundHandlers<'_>,
    runtime: &mut Runtime<'_>,
    effects: &mut E,
    movement: &mut M,
) -> Result<Applied, Error<EffectError<E>, M::Error>>
where
    E: Effects + ManagerQueries<Unit, Parameters = OperationParameters, Error = &'static str>,
    M: Movement<E>,
{
    execute_location_manager(
        state,
        actor,
        request,
        MOVE_TO_HASH,
        inputs,
        handlers,
        runtime,
        effects,
        movement,
    )
}
#[allow(clippy::too_many_arguments)]
fn execute_location_manager<E, M>(
    state: &mut LoadedState,
    actor: (i32, u32),
    request: Request,
    hash: u32,
    inputs: &Inputs<'_, '_>,
    handlers: &BoundHandlers<'_>,
    runtime: &mut Runtime<'_>,
    effects: &mut E,
    movement: &mut M,
) -> Result<Applied, Error<EffectError<E>, M::Error>>
where
    E: Effects + ManagerQueries<Unit, Parameters = OperationParameters, Error = &'static str>,
    M: Movement<E>,
{
    if hash == c6_core::unit::RANGE_ATTACK_HASH && (request.modifiers != 0 || request.append) {
        return Err(Error::Unsupported(
            "RangeAttack modifiers/append continuation",
        ));
    }
    if request.from_user || request.modifiers & !0x10 != 0 {
        return Err(Error::Unsupported("MoveTo fromUser/modifier continuation"));
    }
    movement
        .preflight(state, actor, request)
        .map_err(Error::Movement)?;
    let runtime_id = handlers
        .runtime_id(hash)
        .map_err(|e| Error::Query(super::Error::HandlerObservation(e)))?
        .0;
    handlers
        .validate_execution(state, inputs.handlers, inputs.operations)
        .map_err(|e| Error::Query(super::Error::HandlerObservation(e)))?;
    if !request.append {
        loaded_upkeep::clear_unit_operations(state, actor.0, actor.1, effects)
            .map_err(Error::Upkeep)?;
    }
    let u = unit(state, actor)?;
    let node = OperationNode {
        operation_hash: hash,
        directive: request.directive,
        flags: request.flags,
        push_turn: state.retained().core.current_turn,
        previous_activity: u.e_activity,
        previous_automation: u.e_automation,
        parameters: request.operation(runtime_id).parameters,
    };
    state
        .unit_mut(actor.0, actor.1)
        .map_err(Error::State)?
        .k_operations
        .nodes
        .push(node);
    emit(state, actor, effects, 0x15c35863, Some(runtime_id), None)?;
    if !request.append && unit(state, actor)?.k_operations.nodes.len() == 1 {
        activate(state, actor, inputs, handlers, runtime, effects, movement)?;
    }
    Ok(Applied {
        queued: !unit(state, actor)?.k_operations.nodes.is_empty(),
        delayed_death: unit(state, actor)?.b_delayed_death,
    })
}
/// Manager.Activate on the CURRENT retained queue, without Add, request
/// translation, replacement or a newly selected player order. Existing finite
/// handler coverage is unchanged. Errors preserve already completed prefixes.
#[allow(clippy::too_many_arguments)]
pub fn activate_current_queued_move_to<E, M>(
    state: &mut LoadedState,
    actor: (i32, u32),
    inputs: &Inputs<'_, '_>,
    handlers: &BoundHandlers<'_>,
    runtime: &mut Runtime<'_>,
    effects: &mut E,
    movement: &mut M,
) -> Result<(), Error<EffectError<E>, M::Error>>
where
    E: Effects + ManagerQueries<Unit, Parameters = OperationParameters, Error = &'static str>,
    M: Movement<E>,
{
    handlers
        .validate_execution(state, inputs.handlers, inputs.operations)
        .map_err(|e| Error::Query(super::Error::HandlerObservation(e)))?;
    activate(state, actor, inputs, handlers, runtime, effects, movement)
}

#[allow(clippy::too_many_arguments)]
fn activate<E, M>(
    state: &mut LoadedState,
    actor: (i32, u32),
    inputs: &Inputs<'_, '_>,
    handlers: &BoundHandlers<'_>,
    runtime: &mut Runtime<'_>,
    effects: &mut E,
    movement: &mut M,
) -> Result<(), Error<EffectError<E>, M::Error>>
where
    E: Effects + ManagerQueries<Unit, Parameters = OperationParameters, Error = &'static str>,
    M: Movement<E>,
{
    if unit(state, actor)?.k_operations.nodes.is_empty() || legality::is_busy(unit(state, actor)?) {
        return Ok(());
    }
    loop {
        let continuing = unit(state, actor)?.e_activity == 0x490cc8e1;
        let delete = if continuing {
            let delete = continue_operation(state, actor, inputs, runtime, effects, movement)?;
            if !delete {
                return Ok(());
            }
            delete
        } else {
            let node = front(state, actor)?;
            let target_before = if matches!(node.operation_hash,
                c6_core::unit::FOUND_CITY_HASH | c6_core::unit::BUILD_IMPROVEMENT_HASH) {
                None
            } else {
                Some(target(&node)?)
            };
            let id = handlers
                .runtime_id(node.operation_hash)
                .map_err(|e| Error::Query(super::Error::HandlerObservation(e)))?
                .0;
            let mut operation = ManagerOperation {
                runtime_id: id,
                directive: node.directive,
                flags: node.flags,
                parameters: node.parameters.clone(),
            };
            let reference = UnitReference::from_component_id(identity(state, actor)?);
            let is_range = node.operation_hash == c6_core::unit::RANGE_ATTACK_HASH;
            let allowed = if node.operation_hash == c6_core::unit::BUILD_IMPROVEMENT_HASH {
                let mut farm_handler = |state: &LoadedState, unit: &Unit, improvement: u32,
                    visible_only: bool, effects: &mut E| {
                    movement.farm_can_start(state, unit, improvement, visible_only, effects)
                        .map_err(|_| super::Error::Unsupported("Farm CanStart dependency"))?
                        .ok_or(super::Error::Unsupported("Farm CanStart composition"))
                };
                super::can_start_typed_with_handlers(state, reference, inputs, effects,
                    handlers, &mut operation, None, None, Some(&mut farm_handler))
            } else if is_range {
                let mut range_handler = |state: &LoadedState,
                                         unit: &Unit,
                                         destination: PlotCoord,
                                         visible_only: bool,
                                         effects: &mut E| {
                    movement
                        .range_can_start(
                            state,
                            runtime.visibility,
                            (unit.e_owner, unit.id),
                            destination,
                            visible_only,
                            effects,
                        )
                        .map_err(|_| super::Error::Unsupported("RangeAttack CanStart dependency"))?
                        .ok_or(super::Error::Unsupported(
                            "RangeAttack CanStart composition",
                        ))
                };
                super::can_start_typed_with_range(
                    state,
                    reference,
                    inputs,
                    effects,
                    handlers,
                    &mut operation,
                    None,
                    Some(&mut range_handler),
                )
            } else {
                super::can_start_typed_with_found_city(
                    state,
                    reference,
                    inputs,
                    effects,
                    handlers,
                    &mut operation,
                    movement.found_city_rules(),
                )
            }
            .map_err(Error::Query)?;
            if allowed {
                if operation.runtime_id != id
                    || operation.parameters != node.parameters
                    || operation.flags != node.flags
                    || operation.directive != node.directive
                {
                    return Err(Error::Unsupported(
                        "activation script modified retained operation reference",
                    ));
                }
                let activity = if legality::can_move(unit(state, actor)?) {
                    0x490cc8e1
                } else {
                    skip_turn::HOLD
                };
                let start_location = unit(state, actor)?.k_location;
                loaded_upkeep::set_unit_activity(state, actor.0, actor.1, effects, activity)
                    .map_err(Error::Upkeep)?;
                // Start selects CURRENT front after activity subscribers.
                let node = front(state, actor)?;
                if matches!(node.operation_hash,
                    c6_core::unit::FOUND_CITY_HASH | c6_core::unit::BUILD_IMPROVEMENT_HASH) {
                    let hash = node.operation_hash;
                    let data = if hash == c6_core::unit::BUILD_IMPROVEMENT_HASH {
                        movement.start_farm(state, runtime.visibility, actor, &node, effects)
                    } else {
                        movement.start_found_city(state, runtime.visibility, actor, effects)
                    }.map_err(Error::Movement)?.ok_or(Error::Unsupported(
                        "immediate operation Start composition unavailable"))?;
                    if !unit(state, actor)?.k_operations.nodes.is_empty()
                        && data.action
                        && state
                            .player_slot_status(unit(state, actor)?.e_owner)
                            .map_err(Error::State)?
                            == 3
                        && c6_core::geometry::plot_valid(start_location.x, start_location.y)
                        && any_human_visible(state, runtime, effects, start_location)?
                    {
                        if front(state, actor)?.operation_hash != hash {
                            return Err(Error::Unsupported(
                                "immediate operation callback replaced timer handler",
                            ));
                        }
                        // FoundCity uses BaseHandler.CalculateTimer2d956b0,
                        // which returns the supplied base1, independent of iteration.
                        state
                            .unit_mut(actor.0, actor.1)
                            .map_err(Error::State)?
                            .i_operation_timer = 1;
                    }
                    data.delete
                } else if node.operation_hash == c6_core::unit::RANGE_ATTACK_HASH {
                    false
                } else {
                    let (destination, modifiers) = target(&node)?;
                    if c6_core::movement_world::plot_at(state.world().map(), destination).is_ok()
                        && !visible(state, runtime, unit(state, actor)?.e_owner, destination)?
                    {
                        if let OperationParameters::MoveTo { modifiers, .. } = &mut state
                            .unit_mut(actor.0, actor.1)
                            .map_err(Error::State)?
                            .k_operations
                            .nodes[0]
                            .parameters
                        {
                            *modifiers &= !0x10;
                        }
                    }
                    let _ = (destination, modifiers);
                    if unit(state, actor)?.e_activity != 0x490cc8e1 {
                        return Ok(());
                    }
                    false
                }
            } else {
                let moved = crate::unit_movement_state::unit_has_moved(
                    state,
                    unit(state, actor)?,
                    inputs
                        .movement
                        .ok_or(Error::Unsupported("activation effective HasMoved inputs"))?,
                )
                .map_err(|e| Error::Upkeep(loaded_upkeep::Error::Movement(e)))?;
                if moved && node.push_turn >= state.retained().core.current_turn {
                    return Ok(());
                }
                let _ = target_before;
                true
            }
        };
        if delete {
            loaded_upkeep::pop_front_operation(state, actor.0, actor.1, effects)
                .map_err(Error::Upkeep)?;
        }
        if unit(state, actor)?.b_delayed_death || unit(state, actor)?.k_operations.nodes.is_empty()
        {
            return Ok(());
        }
    }
}

#[derive(Default)]
struct Processing {
    delete: bool,
    stop: bool,
    moved: bool,
    result: i32,
    last_valid: Option<PlotCoord>,
    combat_effects: bool,
}
#[allow(clippy::too_many_arguments)]
fn update<E: Effects, M: Movement<E>>(
    state: &mut LoadedState,
    actor: (i32, u32),
    retained: (i32, u32),
    iteration: i32,
    data: &mut Processing,
    inputs: &Inputs<'_, '_>,
    runtime: &mut Runtime<'_>,
    effects: &mut E,
    movement: &mut M,
) -> Result<(), Error<EffectError<E>, M::Error>> {
    let node = front(state, actor)?;
    if node.operation_hash == c6_core::unit::RANGE_ATTACK_HASH {
        let (destination, _) = target(&node)?;
        if movement
            .range_test(state, runtime.visibility, actor, destination, effects)
            .map_err(Error::Movement)?
            .ok_or(Error::Unsupported("RangeAttack Test composition"))?
        {
            let _ = movement
                .range_attack(state, runtime.visibility, actor, destination, effects)
                .map_err(Error::Movement)?
                .ok_or(Error::Unsupported("RangeAttack execution composition"))?;
            data.delete = true;
        }
        return Ok(());
    }
    if node.operation_hash == c6_core::unit::ALERT_HASH {
        return alert::update(state, actor, data, effects, movement);
    }
    if matches!(
        node.operation_hash,
        c6_core::unit::SPY_GAIN_SOURCES_HASH
            | c6_core::unit::SPY_SIPHON_FUNDS_HASH
            | c6_core::unit::SPY_SABOTAGE_PRODUCTION_HASH
            | c6_core::unit::SPY_DISRUPT_ROCKETRY_HASH
    ) {
        return Err(Error::Unsupported("Spy Update Unit.FinishMoves"));
    }
    if node.operation_hash == c6_core::unit::MAKE_TRADE_ROUTE_HASH {
        return Err(Error::Unsupported(
            "MakeTradeRoute Update and Trade.Movement.MoveTo",
        ));
    }
    let (destination, modifiers) = target(&node)?;
    if node.push_turn == state.retained().core.current_turn {
        // The native captured member-vector includes other units and clears
        // their queues before its own move. This finite entry preflights a
        // singleton Formation; it never replaces those effects with a no-op.
        if formation(state, retained)?.unit_ids.len() != 1 {
            return Err(Error::Unsupported(
                "MoveTo other Formation member Clear/awake",
            ));
        }
    }
    // Native2da4481 tests bit0x10, then push turn. Older requests with
    // that bit clear go directly to movement; other path flags are retained.
    if modifiers & 0x10 != 0 || node.push_turn == state.retained().core.current_turn {
        let processing = state.processing_automation().map_err(Error::State)?;
        let u = unit(state, actor)?;
        if processing == u.e_owner || u.e_automation != -1 {
            return Err(Error::Unsupported("MoveTo automation attack branch"));
        }
        if c6_core::movement_world::plot_at(state.world().map(), destination).is_err() {
            data.delete = true;
            return Ok(());
        }
        // Stock Tutorial.OnUnitMoveTo 0x2c11e20 is the fixed zero-return leaf.
        let current_formation = formation_id(state, actor)?;
        let tested = movement
            .test_move_to(
                state,
                runtime.visibility,
                current_formation,
                destination,
                modifiers,
            )
            .map_err(Error::Movement)?;
        if tested <= 0 {
            data.delete = true;
        } else {
            let f = formation(state, formation_id(state, actor)?)?;
            let first = f
                .last_path
                .get(f.last_path.len().saturating_sub(2))
                .cloned();
            if let Some(first) = first {
                if first.data2 <= 1 {
                    let next = PlotCoord {
                        x: i32::from(first.x),
                        y: i32::from(first.y),
                    };
                    let attack = if modifiers & 0x10 != 0 && first.flags & 1 == 0 {
                        movement
                            .test_attack_into(state, runtime.visibility, actor, next, effects)
                            .map_err(Error::Movement)?
                            .ok_or(Error::Unsupported("MoveTo TestAttackInto composition"))?
                    } else {
                        false
                    };
                    if attack {
                        data.combat_effects = movement
                            .attack_into(state, runtime.visibility, actor, next, effects)
                            .map_err(Error::Movement)?
                            .ok_or(Error::Unsupported("MoveTo complete AttackInto composition"))?;
                        data.delete = true;
                    } else if enemy_at(state, next)? {
                        data.delete = true;
                    }
                }
            }
        }
    }
    if !data.delete {
        let u = unit(state, actor)?;
        let domain = state
            .unit_runtime(u.e_owner, u.id)
            .map_err(Error::State)?
            .domain;
        if domain == 1 {
            return Err(Error::Unsupported("MoveTo air handler branch"));
        }
        let current_formation = formation_id(state, actor)?;
        if formation(state, current_formation)?.has_moved_into_zoc {
            data.stop = true;
        } else {
            let result = movement
                .move_to(
                    state,
                    runtime.visibility,
                    current_formation,
                    destination,
                    modifiers,
                    effects,
                )
                .map_err(Error::Movement)?;
            if result > 0 {
                data.moved = true;
            } else if result < 0 {
                data.stop = true;
            } else {
                data.delete = true;
            }
            if iteration == 0 {
                data.result = result;
            }
        }
    }
    let current = unit(state, actor)?.k_location;
    if !skip_turn::placement::coordinate_valid(current) {
        data.delete = true;
        return Ok(());
    }
    let count = friendly_count(state, actor, inputs)?;
    let limit = inputs
        .plot_unit_limit
        .ok_or(Error::Unsupported("MoveTo PLOT_UNIT_LIMIT"))?;
    let valid = count <= limit as u32
        && movement
            .valid_location(state, runtime.visibility, actor, current)
            .map_err(Error::Movement)?
            != 0;
    if valid {
        data.last_valid = Some(current);
    } else if data.delete
        && !data.combat_effects
        && data
            .last_valid
            .is_some_and(skip_turn::placement::coordinate_valid)
    {
        return Err(Error::Unsupported(
            "MoveTo invalid stacking Placement.Set flags1 rollback",
        ));
    }
    Ok(())
}
fn continue_operation<E: Effects, M: Movement<E>>(
    state: &mut LoadedState,
    actor: (i32, u32),
    inputs: &Inputs<'_, '_>,
    runtime: &mut Runtime<'_>,
    effects: &mut E,
    movement: &mut M,
) -> Result<bool, Error<EffectError<E>, M::Error>> {
    let retained = formation_id(state, actor)?;
    loaded_upkeep::clear_current_formation_path(state, actor.0, actor.1).map_err(Error::Upkeep)?;
    let mut data = Processing {
        result: -1,
        last_valid: Some(formation(state, retained)?.location),
        ..Processing::default()
    };
    let mut iteration = 0i32;
    loop {
        data.delete = false;
        data.moved = false;
        evaluating(state, retained, true)?;
        if unit(state, actor)?.k_operations.nodes.is_empty() {
            loaded_upkeep::set_unit_activity(state, actor.0, actor.1, effects, 0x6b25012e)
                .map_err(Error::Upkeep)?;
            evaluating(state, retained, false)?;
            return Ok(false);
        }
        if iteration > 99 {
            evaluating(state, retained, false)?;
            return Ok(data.delete);
        }
        if (legality::can_move(unit(state, actor)?)
            || formation(state, retained)?.moves_remaining_raw > 0)
            && !legality::is_busy(unit(state, actor)?)
        {
            update(
                state, actor, retained, iteration, &mut data, inputs, runtime, effects, movement,
            )?;
        }
        if !data.delete && !data.stop && !unit(state, actor)?.k_operations.nodes.is_empty() {
            // CheckComplete selects the CURRENT front following Update effects.
            let node = front(state, actor)?;
            if matches!(
                node.operation_hash,
                c6_core::unit::SPY_GAIN_SOURCES_HASH
                    | c6_core::unit::SPY_SIPHON_FUNDS_HASH
                    | c6_core::unit::SPY_SABOTAGE_PRODUCTION_HASH
                    | c6_core::unit::SPY_DISRUPT_ROCKETRY_HASH
            ) {
                // These four source bodies only clear ProcessingData.delete,
                // return true (ignored by Continue), and perform no queries.
                data.delete = false;
            } else if node.operation_hash == c6_core::unit::ALERT_HASH {
                alert::check_complete(state, actor, &mut data, runtime, effects, movement)?;
            } else if node.operation_hash == c6_core::unit::MAKE_TRADE_ROUTE_HASH {
                trade::check_complete(state, actor, &node, &mut data, movement)?;
            } else if node.operation_hash == c6_core::unit::RANGE_ATTACK_HASH {
                data.delete = true;
            } else {
                let (destination, _) = target(&node)?;
                if unit(state, actor)?.k_location == destination {
                    data.delete = true;
                }
            }
        }
        if unit(state, actor)?.k_operations.nodes.is_empty() {
            evaluating(state, retained, false)?;
            return Ok(data.delete);
        }
        if data.moved
            && (data.stop || data.delete || formation(state, retained)?.moves_remaining_raw <= 0)
            && any_human_visible(state, runtime, effects, unit(state, actor)?.k_location)?
        {
            // CalculateTimer's stock MoveTo override uses base1 and iteration.
            // ShouldShowMoves always returns false, so its local-observer-only
            // b6b48d9c report branch is unreachable for this stock manager.
            let owner = unit(state, actor)?.e_owner;
            let timer = if state.player_slot_status(owner).map_err(Error::State)? == 3 {
                let (destination, _) = target(&front(state, actor)?)?;
                if c6_core::movement_world::plot_at(state.world().map(), destination).is_ok()
                    && unit(state, actor)?.k_location == destination
                {
                    1i32.wrapping_add(iteration)
                } else {
                    1
                }
            } else {
                0
            };
            state
                .unit_mut(actor.0, actor.1)
                .map_err(Error::State)?
                .i_operation_timer = timer;
        }
        if data.delete {
            // MoveTo.TimerComplete 0x2da4950 is fixed false/no effects.
            loaded_upkeep::complete_unit_movement(state, actor.0, actor.1, effects)
                .map_err(Error::Upkeep)?;
            evaluating(state, retained, false)?;
            return Ok(true);
        }
        if !data.stop
            && formation(state, retained)?.moves_remaining_raw > 0
            && !legality::is_busy(unit(state, actor)?)
        {
            iteration = iteration.wrapping_add(1);
            continue;
        }
        let current_formation = formation_id(state, actor)?;
        loaded_upkeep::complete_formation_movement(
            state,
            current_formation.0,
            current_formation.1,
            effects,
        )
        .map_err(Error::Upkeep)?;
        let hash = front(state, actor)?.operation_hash;
        emit(
            state,
            actor,
            effects,
            0xcb0345f1,
            Some(hash as i32),
            Some(0),
        )?;
        evaluating(state, retained, false)?;
        return Ok(false);
    }
}

/// Production lower-movement composition. Search, rule definitions, current
/// query configuration and real Placement effects are reused directly. No test
/// path/result or native after-state is accepted as transition input.
pub struct ComputedMovement<'a, 'r> {
    pub path: &'a crate::formation_path_state::Definitions<'a, 'r>,
    pub placement: &'a crate::unit_placement_state::Definitions<'a, 'r>,
    pub queries: &'a crate::unit_placement_state::Queries<'a>,
    pub search: &'a mut c6_core::pathfinder::TwoLayer,
    pub execution: crate::formation_path_state::ExecutionContext,
}
#[derive(Debug)]
pub enum ComputedError<E> {
    Path(crate::formation_path_state::Error),
    Step(crate::formation_move_to::Error<E, crate::formation_move_to::PlacementError<E>>),
    Placement(crate::movement_placement_state::Error),
    Alert(loaded_upkeep::Error<&'static str>),
    Unsupported(&'static str),
}
impl<E> Movement<E> for ComputedMovement<'_, '_>
where
    E: crate::unit_placement_state::Effects
        + loaded_upkeep::FormationRestoreEffects<Error = crate::unit_placement_state::EffectsError<E>>,
{
    type Error = ComputedError<crate::unit_placement_state::EffectsError<E>>;
    fn fortify_turn_max(&self) -> i32 {
        self.path.fortify_turn_max
    }
    fn is_fortifyable(&self, state: &LoadedState, actor: (i32, u32)) -> Result<bool, Self::Error> {
        let unit = state
            .world()
            .player(actor.0)
            .map_err(|e| ComputedError::Path(crate::formation_path_state::Error::World(e)))?
            .unit(actor.1)
            .map_err(|e| ComputedError::Path(crate::formation_path_state::Error::World(e)))?
            .ok_or(ComputedError::Unsupported("Alert current unit"))?
            .record;
        let rules = &self.path.formation;
        let combat =
            rules
                .combat(state, unit)
                .map_err(|e| ComputedError::Path(crate::formation_path_state::Error::Rules(e)))?
                > 0
                || rules.ranged(state, unit).map_err(|e| {
                    ComputedError::Path(crate::formation_path_state::Error::Rules(e))
                })? > 0
                || rules.bombard(state, unit).map_err(|e| {
                    ComputedError::Path(crate::formation_path_state::Error::Rules(e))
                })? > 0;
        if !combat {
            return Ok(false);
        }
        let domain = state
            .unit_runtime(actor.0, actor.1)
            .map_err(|e| ComputedError::Path(crate::formation_path_state::Error::State(e)))?
            .domain;
        // NoDefensiveBonus reads domain0 before the domain2/domain3 test.
        if domain == 0 {
            return Ok(false);
        }
        Ok(domain == 2 || domain == 3)
    }
    fn alert_enemy_visible(
        &self,
        state: &LoadedState,
        visibility: &unit_sight_state::Runtime,
        actor: (i32, u32),
    ) -> Result<bool, Self::Error> {
        let unit = state
            .world()
            .player(actor.0)
            .map_err(|e| ComputedError::Path(crate::formation_path_state::Error::World(e)))?
            .unit(actor.1)
            .map_err(|e| ComputedError::Path(crate::formation_path_state::Error::World(e)))?
            .ok_or(ComputedError::Unsupported("Alert current unit"))?
            .record;
        loaded_upkeep::alert_enemy_visible(
            state,
            unit,
            &self.path.formation,
            &alert::Visibility(visibility),
        )
        .map_err(ComputedError::Alert)
    }
    fn city_at(
        &self,
        state: &LoadedState,
        location: PlotCoord,
    ) -> Result<Option<c6_core::trade::ComponentId>, Self::Error> {
        if !c6_core::skip_turn::placement::coordinate_valid(location) {
            return Ok(None);
        }
        let map = state.world().map();
        let index = location
            .y
            .wrapping_mul(map.header.width)
            .wrapping_add(location.x) as u32;
        let Some(plot) = map.plots.get(index as usize) else {
            return Ok(None);
        };
        let Some(district) = self
            .path
            .movement
            .district_at(state, plot)
            .map_err(|e| ComputedError::Path(crate::formation_path_state::Error::Cost(e)))?
        else {
            return Ok(None);
        };
        if !self
            .path
            .movement
            .district_is_city(district)
            .map_err(|e| ComputedError::Path(crate::formation_path_state::Error::Cost(e)))?
        {
            return Ok(None);
        }
        let Some(attrs) = &plot.city_attributes else {
            return Ok(None);
        };
        let city = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == i32::from(attrs.player))
            .and_then(|p| {
                p.cities.records.entries.iter().find(|e| {
                    e.value.header.owner == i32::from(attrs.player)
                        && e.value.header.id == attrs.owning_city
                })
            });
        Ok(city.map(|e| c6_core::trade::ComponentId {
            owner: e.value.header.owner as i16,
            component_type: 5,
            id: e.value.header.id,
        }))
    }
    fn preflight(
        &self,
        state: &LoadedState,
        actor: (i32, u32),
        _: Request,
    ) -> Result<(), Self::Error> {
        let actor = state
            .world()
            .player(actor.0)
            .map_err(|e| ComputedError::Path(crate::formation_path_state::Error::World(e)))?
            .unit(actor.1)
            .map_err(|e| ComputedError::Path(crate::formation_path_state::Error::World(e)))?
            .ok_or(ComputedError::Unsupported("MoveTo actor"))?
            .record;
        let runtime = state
            .unit_runtime(actor.e_owner, actor.id)
            .map_err(|e| ComputedError::Path(crate::formation_path_state::Error::State(e)))?;
        let f = runtime
            .formation
            .ok_or(ComputedError::Unsupported("MoveTo Formation"))?;
        let record = state
            .world()
            .player(f.owner)
            .map_err(|e| ComputedError::Path(crate::formation_path_state::Error::World(e)))?
            .formation(f.requested_id)
            .map_err(|e| ComputedError::Path(crate::formation_path_state::Error::World(e)))?
            .ok_or(ComputedError::Unsupported("MoveTo Formation"))?
            .record;
        if record.unit_ids != [actor.id] {
            return Err(ComputedError::Unsupported(
                "ordinary MoveTo singleton member effects",
            ));
        }
        Ok(())
    }
    fn test_move_to(
        &mut self,
        state: &mut LoadedState,
        visibility: &unit_sight_state::Runtime,
        formation: (i32, u32),
        destination: PlotCoord,
        flags: u32,
    ) -> Result<i32, Self::Error> {
        let binding = state.binding().clone();
        let runtime = crate::formation_path_state::Runtime {
            visibility,
            civilization_levels: self.queries.civilization_levels.map(|values| {
                crate::movement_placement_state::CurrentCivilizationLevels {
                    binding: &binding,
                    values,
                }
            }),
            city_domains: self.queries.city_domains.map(|masks| {
                crate::formation_path_state::CurrentCityDomains {
                    binding: &binding,
                    stamp: state.action_stamp(),
                    masks,
                }
            }),
            execution: self.execution,
            is_evaluating_operation: state
                .formation_runtime(formation.0, formation.1)
                .map_err(|e| ComputedError::Path(crate::formation_path_state::Error::State(e)))?
                .evaluating_operation(),
        };
        crate::formation_move_to::test_move_to(
            state,
            self.path,
            runtime,
            self.search,
            formation,
            destination,
            flags,
        )
        .map_err(ComputedError::Path)
    }
    fn move_to(
        &mut self,
        state: &mut LoadedState,
        visibility: &mut unit_sight_state::Runtime,
        formation: (i32, u32),
        destination: PlotCoord,
        flags: u32,
        effects: &mut E,
    ) -> Result<i32, Self::Error> {
        crate::formation_move_to::move_to(
            state,
            self.path,
            self.placement,
            self.queries,
            visibility,
            self.search,
            formation,
            destination,
            flags,
            self.execution,
            effects,
        )
        .map_err(ComputedError::Step)
    }
    fn valid_location(
        &mut self,
        state: &LoadedState,
        _: &unit_sight_state::Runtime,
        actor: (i32, u32),
        location: PlotCoord,
    ) -> Result<i32, Self::Error> {
        self.queries
            .placement_context(state, self.placement.placement)
            .map_err(ComputedError::Placement)?
            .unit_placement(
                actor.0,
                actor.1,
                location,
                c6_core::movement_shore::PlacementRequest::from_validate_options(0),
            )
            .map_err(ComputedError::Placement)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IgnoredRequest {
    InvalidPlayer,
    MissingUnit,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestOutcome {
    Ignored(IgnoredRequest),
    /// Manager returned for an actual resolved unit. The caller must run the
    /// outer owned diplomacy/blocking aftermath even if the queue is now empty.
    Executed(Applied),
}
/// Trusted, serialized headless active-player request entry. The policy issuer
/// must own the addressed Unit. Native UnitOperation.Execute's invalid-player/
/// missing-unit branches return without Add or aftermath. Real registered MoveTo
/// parameters and source defaults are represented by Request, not a fabricated
/// runtime handler ordinal or an initial CanStart answer.
///
/// Broadcast's earlier TurnUnready helper is source-proven inert when the actual
/// player +0x290 IsTurnActive getter is true (CanUnreadyTurn immediately returns
/// false before phase/timer/EngineUtility calls, writes or RNG). Inactive-player
/// unready delivery remains an explicit unsupported continuation. Native cached
/// UI admission and transport representation are outside this trusted boundary.
/// A returned Executed still requires outer UnitOperation aftermath and the
/// final messages-processed observer; it is not whole-action acceptance.
#[allow(clippy::too_many_arguments)]
pub fn request_active_player<E, M>(
    state: &mut LoadedState,
    issuer: i32,
    actor: (i32, u32),
    request: Request,
    inputs: &Inputs<'_, '_>,
    handlers: &BoundHandlers<'_>,
    runtime: &mut Runtime<'_>,
    effects: &mut E,
    movement: &mut M,
) -> Result<RequestOutcome, Error<EffectError<E>, M::Error>>
where
    E: Effects + ManagerQueries<Unit, Parameters = OperationParameters, Error = &'static str>,
    M: Movement<E>,
{
    request_operation_active_player(
        state,
        issuer,
        actor,
        OperationRequest::MoveTo(request),
        inputs,
        handlers,
        runtime,
        effects,
        movement,
    )
}
#[allow(clippy::too_many_arguments)]
pub fn request_range_active_player<E, M>(
    state: &mut LoadedState,
    issuer: i32,
    actor: (i32, u32),
    request: Request,
    inputs: &Inputs<'_, '_>,
    handlers: &BoundHandlers<'_>,
    runtime: &mut Runtime<'_>,
    effects: &mut E,
    movement: &mut M,
) -> Result<RequestOutcome, Error<EffectError<E>, M::Error>>
where
    E: Effects + ManagerQueries<Unit, Parameters = OperationParameters, Error = &'static str>,
    M: Movement<E>,
{
    request_operation_active_player(
        state,
        issuer,
        actor,
        OperationRequest::RangeAttack(request),
        inputs,
        handlers,
        runtime,
        effects,
        movement,
    )
}
#[derive(Clone, Copy, Debug)]
enum OperationRequest {
    MoveTo(Request),
    RangeAttack(Request),
    FoundCity,
    Farm { improvement_hash: u32, modifiers: u32 },
}
#[allow(clippy::too_many_arguments)]
fn request_operation_active_player<E, M>(
    state: &mut LoadedState,
    issuer: i32,
    actor: (i32, u32),
    request: OperationRequest,
    inputs: &Inputs<'_, '_>,
    handlers: &BoundHandlers<'_>,
    runtime: &mut Runtime<'_>,
    effects: &mut E,
    movement: &mut M,
) -> Result<RequestOutcome, Error<EffectError<E>, M::Error>>
where
    E: Effects + ManagerQueries<Unit, Parameters = OperationParameters, Error = &'static str>,
    M: Movement<E>,
{
    if issuer != actor.0 {
        return Err(Error::Unsupported(
            "policy issuer does not own requested player",
        ));
    }
    if issuer as u32 >= 64
        || !state
            .world()
            .player_is_valid(issuer)
            .map_err(Error::World)?
    {
        return Ok(RequestOutcome::Ignored(IgnoredRequest::InvalidPlayer));
    }
    if matches!(
        request,
        OperationRequest::MoveTo(
            Request { append: true, .. }
                | Request {
                    from_user: true,
                    ..
                }
        )
    ) {
        return Err(Error::Unsupported(
            "full request nondefault transient append/fromUser metadata",
        ));
    }
    if !state.turn_active(issuer).map_err(Error::State)? {
        return Err(Error::Unsupported(
            "inactive requested player public-request continuation",
        ));
    }
    // Broadcast consults the actual global local player, independently of the
    // addressed owner. Preserve that read even in a trusted multi-player arena.
    let local = state.retained().core.local_player;
    if local < 0 || !state.turn_active(local).map_err(Error::State)? {
        return Err(Error::Unsupported(
            "local-player RequestTurnUnready phase/message continuation",
        ));
    }
    let Some(found) = state
        .world()
        .player(actor.0)
        .map_err(Error::World)?
        .unit(actor.1)
        .map_err(Error::World)?
    else {
        return Ok(RequestOutcome::Ignored(IgnoredRequest::MissingUnit));
    };
    if found.record.e_owner != issuer {
        return Err(Error::Unsupported(
            "policy issuer does not own resolved Unit",
        ));
    }

    match request {
        OperationRequest::MoveTo(request) => execute_manager(
            state, actor, request, inputs, handlers, runtime, effects, movement,
        ),
        OperationRequest::RangeAttack(request) => execute_location_manager(
            state,
            actor,
            request,
            c6_core::unit::RANGE_ATTACK_HASH,
            inputs,
            handlers,
            runtime,
            effects,
            movement,
        ),
        OperationRequest::FoundCity => {
            execute_found_city_manager(state, actor, inputs, handlers, runtime, effects, movement)
        }
        OperationRequest::Farm { improvement_hash, modifiers } => execute_farm_manager(
            state, actor, improvement_hash, modifiers, inputs, handlers, runtime, effects, movement),
    }
    .map(RequestOutcome::Executed)
}
/// Parameterless FoundCity request through the same trusted ownership/turn and
/// resolved-Unit checks as the ordinary UnitOperation entry.
#[allow(clippy::too_many_arguments)]
pub fn request_found_city_active_player<E, M>(
    state: &mut LoadedState,
    issuer: i32,
    actor: (i32, u32),
    inputs: &Inputs<'_, '_>,
    handlers: &BoundHandlers<'_>,
    runtime: &mut Runtime<'_>,
    effects: &mut E,
    movement: &mut M,
) -> Result<RequestOutcome, Error<EffectError<E>, M::Error>>
where
    E: Effects + ManagerQueries<Unit, Parameters = OperationParameters, Error = &'static str>,
    M: Movement<E>,
{
    request_operation_active_player(
        state,
        issuer,
        actor,
        OperationRequest::FoundCity,
        inputs,
        handlers,
        runtime,
        effects,
        movement,
    )
}

#[derive(Debug)]
pub enum ActionError<E, M, O> {
    Manager(Error<E, M>),
    Aftermath(super::current_aftermath::Error<O>),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessedRequest {
    /// None is a delivered message whose Unit lookup returned null. Native
    /// Execute skips Add/aftermath there but its outer batch observer still runs.
    pub manager: Option<Applied>,
    pub boundary: super::transition::ExecutionBoundary,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionOutcome {
    NotSent(IgnoredRequest),
    Processed(ProcessedRequest),
}
/// Execute one serialized active-player request through the supported headless
/// MessagesProcessed boundary. The existing aftermath context keeps the SAME
/// owned World, backend, script registry and pending notification allocations.
/// No native AI policy is called. A failure retains its completed prefix and
/// must not be retried as if the original request had never run.
///
/// This joins existing gameplay algorithms; callback/backend admission remains
/// required. Success is not evidence of complete Civ VI state/RNG/turn parity.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn execute_active_request<B, M>(
    context: &mut super::current_aftermath::Context<'_, B>,
    issuer: i32,
    actor: (i32, u32),
    request: Request,
    inputs: &Inputs<'_, '_>,
    handlers: &BoundHandlers<'_>,
    runtime: &mut Runtime<'_>,
    movement: &mut M,
    budget: &mut usize,
) -> Result<
    ActionOutcome,
    ActionError<EffectError<B>, M::Error, <B as super::current_aftermath::Backend>::ObserverError>,
>
where
    B: Effects
        + ManagerQueries<Unit, Parameters = OperationParameters, Error = &'static str>
        + super::current_aftermath::Backend
        + c6_core::diplomacy_manager::ProcessStatement<LoadedState>,
    M: Movement<B>,
{
    execute_operation_active_request(
        context,
        issuer,
        actor,
        OperationRequest::MoveTo(request),
        inputs,
        handlers,
        runtime,
        movement,
        budget,
    )
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn execute_range_active_request<B, M>(
    context: &mut super::current_aftermath::Context<'_, B>,
    issuer: i32,
    actor: (i32, u32),
    request: Request,
    inputs: &Inputs<'_, '_>,
    handlers: &BoundHandlers<'_>,
    runtime: &mut Runtime<'_>,
    movement: &mut M,
    budget: &mut usize,
) -> Result<
    ActionOutcome,
    ActionError<EffectError<B>, M::Error, <B as super::current_aftermath::Backend>::ObserverError>,
>
where
    B: Effects
        + ManagerQueries<Unit, Parameters = OperationParameters, Error = &'static str>
        + super::current_aftermath::Backend
        + c6_core::diplomacy_manager::ProcessStatement<LoadedState>,
    M: Movement<B>,
{
    execute_operation_active_request(
        context,
        issuer,
        actor,
        OperationRequest::RangeAttack(request),
        inputs,
        handlers,
        runtime,
        movement,
        budget,
    )
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn execute_operation_active_request<B, M>(
    context: &mut super::current_aftermath::Context<'_, B>,
    issuer: i32,
    actor: (i32, u32),
    request: OperationRequest,
    inputs: &Inputs<'_, '_>,
    handlers: &BoundHandlers<'_>,
    runtime: &mut Runtime<'_>,
    movement: &mut M,
    budget: &mut usize,
) -> Result<
    ActionOutcome,
    ActionError<EffectError<B>, M::Error, <B as super::current_aftermath::Backend>::ObserverError>,
>
where
    B: Effects
        + ManagerQueries<Unit, Parameters = OperationParameters, Error = &'static str>
        + super::current_aftermath::Backend
        + c6_core::diplomacy_manager::ProcessStatement<LoadedState>,
    M: Movement<B>,
{
    if let Some(stage) = context.failed_at() {
        return Err(ActionError::Aftermath(
            super::current_aftermath::Error::FailedContinuation(stage),
        ));
    }
    let outcome = {
        let provider = &mut context.notifications;
        request_operation_active_player(
            provider.state,
            issuer,
            actor,
            request,
            inputs,
            handlers,
            runtime,
            &mut provider.backend,
            movement,
        )
        .map_err(ActionError::Manager)?
    };
    let manager = match outcome {
        RequestOutcome::Ignored(IgnoredRequest::InvalidPlayer) => {
            return Ok(ActionOutcome::NotSent(IgnoredRequest::InvalidPlayer))
        }
        RequestOutcome::Ignored(IgnoredRequest::MissingUnit) => None,
        RequestOutcome::Executed(manager) => {
            context
                .after_manager(actor.0, budget)
                .map_err(ActionError::Aftermath)?;
            Some(manager)
        }
    };
    context
        .messages_processed()
        .map_err(ActionError::Aftermath)?;
    Ok(ActionOutcome::Processed(ProcessedRequest {
        manager,
        boundary: super::transition::ExecutionBoundary::HeadlessMessagesProcessed,
    }))
}

/// Execute parameterless FoundCity through the same diplomacy/notification and
/// MessagesProcessed continuation as MoveTo, without a fabricated target plot.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn execute_found_city_active_request<B, M>(
    context: &mut super::current_aftermath::Context<'_, B>,
    issuer: i32,
    actor: (i32, u32),
    inputs: &Inputs<'_, '_>,
    handlers: &BoundHandlers<'_>,
    runtime: &mut Runtime<'_>,
    movement: &mut M,
    budget: &mut usize,
) -> Result<
    ActionOutcome,
    ActionError<EffectError<B>, M::Error, <B as super::current_aftermath::Backend>::ObserverError>,
>
where
    B: Effects
        + ManagerQueries<Unit, Parameters = OperationParameters, Error = &'static str>
        + super::current_aftermath::Backend
        + c6_core::diplomacy_manager::ProcessStatement<LoadedState>,
    M: Movement<B>,
{
    execute_operation_active_request(
        context,
        issuer,
        actor,
        OperationRequest::FoundCity,
        inputs,
        handlers,
        runtime,
        movement,
        budget,
    )
}

/// Execute ordinary Farm through the same retained active-player message boundary.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn execute_farm_active_request<B, M>(
    context: &mut super::current_aftermath::Context<'_, B>,
    issuer: i32,
    actor: (i32, u32),
    improvement_hash: u32,
    modifiers: u32,
    inputs: &Inputs<'_, '_>,
    handlers: &BoundHandlers<'_>,
    runtime: &mut Runtime<'_>,
    movement: &mut M,
    budget: &mut usize,
) -> Result<
    ActionOutcome,
    ActionError<EffectError<B>, M::Error, <B as super::current_aftermath::Backend>::ObserverError>,
>
where
    B: Effects
        + ManagerQueries<Unit, Parameters = OperationParameters, Error = &'static str>
        + super::current_aftermath::Backend
        + c6_core::diplomacy_manager::ProcessStatement<LoadedState>,
    M: Movement<B>,
{
    execute_operation_active_request(
        context,
        issuer,
        actor,
        OperationRequest::Farm { improvement_hash, modifiers },
        inputs,
        handlers,
        runtime,
        movement,
        budget,
    )
}
