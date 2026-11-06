//! Stock operation callbacks and retained native storage-slot deletion.
use super::*;
use crate::loaded_actions::IdentityReport;
use c6_core::{
    unit::{ComponentId, ALERT_HASH, FORTIFY_HASH, MOVE_TO_HASH, SKIP_TURN_HASH},
    unit_operations_clear::{self, BaseDeactivationContext, OperationDeactivatedReport},
};

/// Required synchronous operation subscribers. They may mutate authoritative
/// Unit fields and queues through canonical OperationNodes methods. Preflight
/// admits all reached gameplay/RNG effects; collecting reports is insufficient.
pub trait OperationReports {
    type Error;
    fn preflight_operation_report(
        &self,
        state: &LoadedState,
        actor: UnitHandle,
        report: IdentityReport,
    ) -> Result<(), Self::Error>;
    fn preflight_placement_report(
        &self,
        state: &LoadedState,
        actor: UnitHandle,
        event: u32,
    ) -> Result<(), Self::Error>;
    fn placement_report(
        &mut self,
        state: &mut LoadedState,
        report: c6_core::unit_placement_transition::PlacementReport,
    ) -> Result<(), Self::Error>;
    fn operation_report(
        &mut self,
        state: &mut LoadedState,
        report: IdentityReport,
    ) -> Result<i32, Self::Error>;
}
impl<C> OperationReports for RulesEffects<'_, C>
where
    C: Continuations + OperationReports<Error = <C as Continuations>::Error>,
{
    type Error = RulesEffectError<<C as Continuations>::Error>;
    fn preflight_operation_report(
        &self,
        state: &LoadedState,
        actor: UnitHandle,
        report: IdentityReport,
    ) -> Result<(), Self::Error> {
        self.continuations
            .preflight_operation_report(state, actor, report)
            .map_err(RulesEffectError::Continuation)
    }
    fn preflight_placement_report(
        &self,
        state: &LoadedState,
        actor: UnitHandle,
        event: u32,
    ) -> Result<(), Self::Error> {
        self.continuations
            .preflight_placement_report(state, actor, event)
            .map_err(RulesEffectError::Continuation)
    }
    fn placement_report(
        &mut self,
        state: &mut LoadedState,
        report: c6_core::unit_placement_transition::PlacementReport,
    ) -> Result<(), Self::Error> {
        self.continuations
            .placement_report(state, report)
            .map_err(RulesEffectError::Continuation)
    }
    fn operation_report(
        &mut self,
        state: &mut LoadedState,
        report: IdentityReport,
    ) -> Result<i32, Self::Error> {
        self.continuations
            .operation_report(state, report)
            .map_err(RulesEffectError::Continuation)
    }
}
fn identity<E>(state: &LoadedState, actor: UnitHandle) -> Result<ComponentId, Error<E>> {
    let unit = actor.get(state).map_err(Error::World)?;
    Ok(ComponentId {
        owner: unit.e_owner as i16,
        component_type: 1,
        id: unit.id,
    })
}
fn emit<E: OperationReports>(
    state: &mut LoadedState,
    actor: UnitHandle,
    effects: &mut E,
    report: IdentityReport,
) -> Result<(), Error<E::Error>> {
    effects
        .preflight_operation_report(state, actor, report)
        .map_err(Error::Provider)?;
    effects
        .operation_report(state, report)
        .map_err(Error::Provider)?;
    Ok(())
}
// Only splits the existing setter at its synchronous report boundary. This
// recorder executes no subscriber and supplies no native result; the real
// required delivery immediately follows after the mutable Unit borrow ends.
struct ActivityBoundary(Option<c6_core::skip_turn::IdentityIntReport>);
impl c6_core::skip_turn::StartEffects for ActivityBoundary {
    fn mark_activity_dirty(&mut self, _: &Unit, _: u32) {
        unreachable!("local-authority transport excluded");
    }
    fn send(&mut self, _: &mut Unit, report: c6_core::skip_turn::IdentityIntReport) -> i32 {
        self.0 = Some(report);
        0
    }
}
pub(super) fn set_activity<E: OperationReports>(
    state: &mut LoadedState,
    actor: UnitHandle,
    effects: &mut E,
    activity: i32,
) -> Result<(), Error<E::Error>> {
    if actor.get(state).map_err(Error::World)?.e_activity == activity {
        return Ok(());
    }
    let id = identity(state, actor)?;
    let report = IdentityReport {
        event: 0xe23b112d,
        subtype: 0,
        identity: id,
        first: Some(activity),
        second: None,
    };
    effects
        .preflight_operation_report(state, actor, report)
        .map_err(Error::Provider)?;
    let mut boundary = ActivityBoundary(None);
    c6_core::skip_turn::set_activity(
        actor.get_mut(state).map_err(Error::State)?,
        id,
        activity,
        false,
        &mut boundary,
    )
    .map_err(|_| Error::Unsupported("current activity identity"))?;
    if boundary.0.is_some() {
        effects
            .operation_report(state, report)
            .map_err(Error::Provider)?;
    }
    Ok(())
}
fn clear_path<E>(state: &mut LoadedState, actor: UnitHandle) -> Result<(), Error<E>> {
    let unit = actor.get(state).map_err(Error::World)?;
    if state
        .unit_runtime(unit.e_owner, unit.id)
        .map_err(Error::State)?
        .formation
        .is_none()
    {
        return Ok(());
    }
    let unit = actor.get(state).map_err(Error::World)?;
    let formation = state
        .unit_runtime(unit.e_owner, unit.id)
        .map_err(Error::State)?
        .formation
        .ok_or(Error::Unsupported("current Formation attachment changed"))?;
    let (record, runtime) = state
        .formation_with_runtime_mut(formation.owner, formation.requested_id)
        .map_err(Error::State)?;
    runtime
        .clear_path_cache(record)
        .map_err(Error::FormationRuntime)
}
struct Base<'s, E> {
    state: &'s mut LoadedState,
    actor: UnitHandle,
    effects: &'s mut E,
}
impl<E: OperationReports> BaseDeactivationContext for Base<'_, E> {
    type Error = Error<E::Error>;
    fn unit(&self) -> Result<&Unit, Self::Error> {
        self.actor.get(self.state).map_err(Error::World)
    }
    fn local_player(&self) -> Result<i32, Self::Error> {
        Ok(self.state.retained().core.local_player)
    }
    fn send_operation_deactivated(
        &mut self,
        report: OperationDeactivatedReport,
    ) -> Result<(), Self::Error> {
        emit(
            self.state,
            self.actor,
            self.effects,
            IdentityReport {
                event: report.event_hash,
                subtype: 0,
                identity: report.identity,
                first: Some(report.operation_hash as i32),
                second: Some(i32::from(report.ready_to_select)),
            },
        )
    }
}

impl<E: OperationReports> c6_core::unit_placement_transition::PlacementStateContext
    for Base<'_, E>
{
    type Error = Error<E::Error>;
    fn unit(&self) -> Result<&Unit, Self::Error> {
        self.actor.get(self.state).map_err(Error::World)
    }
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
        self.actor.get_mut(self.state).map_err(Error::State)
    }
    fn preflight_placement_report(&self, event: u32) -> Result<(), Self::Error> {
        self.effects
            .preflight_placement_report(self.state, self.actor, event)
            .map_err(Error::Provider)
    }
    fn send_placement_report(
        &mut self,
        report: c6_core::unit_placement_transition::PlacementReport,
    ) -> Result<(), Self::Error> {
        self.effects
            .placement_report(self.state, report)
            .map_err(Error::Provider)
    }
}

pub fn set_unit_activity<E: OperationReports>(
    state: &mut LoadedState,
    owner: i32,
    id: u32,
    effects: &mut E,
    activity: i32,
) -> Result<(), Error<E::Error>> {
    set_activity(state, UnitHandle { owner, id }, effects, activity)
}
pub fn clear_current_formation_path<E>(
    state: &mut LoadedState,
    owner: i32,
    id: u32,
) -> Result<(), Error<E>> {
    clear_path(state, UnitHandle { owner, id })
}
impl<E: OperationReports> unit_operations_clear::ClearOperationsContext for Base<'_, E> {
    type Error = Error<E::Error>;
    fn unit(&self) -> Result<&Unit, Self::Error> {
        self.actor.get(self.state).map_err(Error::World)
    }
    fn deactivate_operations(&mut self) -> Result<(), Self::Error> {
        if self
            .actor
            .get(self.state)
            .map_err(Error::World)?
            .k_operations
            .nodes
            .is_empty()
        {
            return Ok(());
        }
        if self.actor.get(self.state).map_err(Error::World)?.e_activity
            == unit_operations_clear::ACTIVITY_OPERATION
        {
            set_activity(
                self.state,
                self.actor,
                self.effects,
                unit_operations_clear::ACTIVITY_AWAKE,
            )?;
        }
        self.actor
            .get_mut(self.state)
            .map_err(Error::State)?
            .i_operation_timer = 0;
        Ok(())
    }
    fn pop_front_operation(&mut self) -> Result<(), Self::Error> {
        pop_front_operation(self.state, self.actor.owner, self.actor.id, self.effects)
    }
    fn clear_current_formation_path(&mut self) -> Result<(), Self::Error> {
        clear_path(self.state, self.actor)
    }
}
/// Real Manager.Clear with current queue rereads after every subscriber. This
/// preserves capacity/storage IDs rather than rebuilding an empty DTO vector.
pub fn clear_unit_operations<E: OperationReports>(
    state: &mut LoadedState,
    owner: i32,
    id: u32,
    effects: &mut E,
) -> Result<(), Error<E::Error>> {
    unit_operations_clear::clear_operations(&mut Base {
        state,
        actor: UnitHandle { owner, id },
        effects,
    })
}

/// Complete the current owned Unit's real pending movement before continuing.
/// Subscribers see the cleared flag and may mutate the same current state.
/// Errors retain the source-ordered completed prefix.
pub fn complete_unit_movement<E: OperationReports>(
    state: &mut LoadedState,
    owner: i32,
    id: u32,
    effects: &mut E,
) -> Result<bool, Error<E::Error>> {
    use c6_core::unit_placement_transition::{complete_movement, PlacementStateError};
    complete_movement(&mut Base {
        state,
        actor: UnitHandle { owner, id },
        effects,
    })
    .map_err(|error| match error {
        PlacementStateError::EffectsUnavailable(e)
        | PlacementStateError::Context(e)
        | PlacementStateError::ReportFailed(e) => e,
    })
}

/// Original PopFront selects the current front on entry, then retains its
/// allocation/slot through handler callbacks. DeleteNode compares that token to
/// current Front, deactivates only on equality, and removes the CURRENT payload
/// at the retained slot. Occurrence IDs do not substitute for this behavior.
/// Completed effects survive errors; unknown capacity/reuse and unsupported
/// handlers remain explicit limits, never a removal of an arbitrary Vec front.
pub fn pop_front_operation<E: OperationReports>(
    state: &mut LoadedState,
    owner: i32,
    id: u32,
    effects: &mut E,
) -> Result<(), Error<E::Error>> {
    let actor = UnitHandle { owner, id };
    let Some(selected) = actor
        .get(state)
        .map_err(Error::World)?
        .k_operations
        .nodes
        .front_storage()
        .map_err(Error::OperationStorage)?
    else {
        return Ok(());
    };
    let hash = actor.get(state).map_err(Error::World)?.k_operations.nodes[0].operation_hash;
    match hash {
        ALERT_HASH => {
            if actor.get(state).map_err(Error::World)?.e_activity == 0xd0aed661_u32 as i32 {
                set_activity(state, actor, effects, unit_operations_clear::ACTIVITY_AWAKE)?;
            }
        }
        // XP2 BuildImprovement's Deactivate slot3b033a8 binds the same
        // BaseHandler::Deactivate2d964c0; it has no derived prelude.
        // XP2 Pillage slot3b03ac8 (basePillage3affca8) binds the same body.
        // XP2 Repair relocation slot3b03dc8 also binds BaseHandler2d964c0.
        // XP2 RemoveFeature slot3b03bc8 inherits the same verified body.
        SKIP_TURN_HASH | FORTIFY_HASH | c6_core::unit::RANGE_ATTACK_HASH
        | c6_core::unit::BUILD_IMPROVEMENT_HASH | c6_core::unit::PILLAGE_HASH | c6_core::unit::REPAIR_HASH | c6_core::unit::REMOVE_FEATURE_HASH => {}
        MOVE_TO_HASH => {
            complete_unit_movement(state, owner, id, effects)?;
        }
        _ => {
            return Err(Error::Unsupported(
                "selected handler Deactivate continuation",
            ))
        }
    }
    pop_front_selected(state, actor, effects, selected, hash)
}

fn pop_front_selected<E: OperationReports>(
    state: &mut LoadedState,
    actor: UnitHandle,
    effects: &mut E,
    selected: c6_core::unit::OperationStorageHandle,
    hash: u32,
) -> Result<(), Error<E::Error>> {
    let delete = !unit_operations_clear::deactivate_base_handler(
        &mut Base {
            state,
            actor,
            effects,
        },
        hash,
    )?;
    if delete {
        if actor
            .get(state)
            .map_err(Error::World)?
            .k_operations
            .nodes
            .storage_is_front(selected)
            .map_err(Error::OperationStorage)?
            && !actor
                .get(state)
                .map_err(Error::World)?
                .k_operations
                .nodes
                .is_empty()
        {
            if actor.get(state).map_err(Error::World)?.e_activity
                == unit_operations_clear::ACTIVITY_OPERATION
            {
                set_activity(state, actor, effects, unit_operations_clear::ACTIVITY_AWAKE)?;
            }
            actor
                .get_mut(state)
                .map_err(Error::State)?
                .i_operation_timer = 0;
        }
        actor
            .get_mut(state)
            .map_err(Error::State)?
            .k_operations
            .nodes
            .remove_storage(selected)
            .map_err(Error::OperationStorage)?;
        if actor
            .get(state)
            .map_err(Error::World)?
            .k_operations
            .nodes
            .is_empty()
        {
            clear_path(state, actor)?;
        }
    }
    if actor
        .get(state)
        .map_err(Error::World)?
        .k_operations
        .nodes
        .is_empty()
    {
        emit(
            state,
            actor,
            effects,
            IdentityReport {
                event: 0x383669f1,
                subtype: 0,
                identity: identity(state, actor)?,
                first: None,
                second: None,
            },
        )?;
    }
    Ok(())
}

/// MakeTradeRoute::Deactivate2da0da0 followed by the same retained-slot
/// BaseHandler/DeleteNode tail. Active cancellation remains explicit; normal
/// ExpireRoute has already made HasOutgoingRoute false before this call.
pub fn pop_front_expired_trade_route<E>(
    state: &mut LoadedState,
    owner: i32,
    id: u32,
    inputs: &Inputs<'_>,
    effects: &mut E,
) -> Result<(), Error<<E as Effects>::Error>>
where
    E: Effects + OperationReports<Error = <E as Effects>::Error>,
{
    let actor = UnitHandle { owner, id };
    let Some(selected) = actor
        .get(state)
        .map_err(Error::World)?
        .k_operations
        .nodes
        .front_storage()
        .map_err(Error::OperationStorage)?
    else {
        return Ok(());
    };
    let node = &actor.get(state).map_err(Error::World)?.k_operations.nodes[0];
    if node.operation_hash != c6_core::unit::MAKE_TRADE_ROUTE_HASH {
        return Err(Error::Unsupported("Trade Deactivate selected handler"));
    }
    let c6_core::unit::OperationParameters::MakeTradeRoute {
        location0,
        location1,
        ..
    } = node.parameters
    else {
        return Err(Error::Unsupported("Trade Deactivate parameters"));
    };
    let origin = crate::loaded_actions::trade_route_actions::city::<()>(state, location1)
        .map_err(Error::TradeRoute)?;
    let destination = crate::loaded_actions::trade_route_actions::city::<()>(state, location0)
        .map_err(Error::TradeRoute)?;
    if let (Some(origin), Some(destination)) = (origin, destination) {
        let current = identity(state, actor)?;
        let trader = c6_core::trade::ComponentId {
            owner: current.owner,
            component_type: current.component_type,
            id: current.id,
        };
        if state
            .trade_graph()
            .map_err(Error::State)?
            .has_outgoing_route(origin, destination, trader)
        {
            return Err(Error::Unsupported(
                "Trade Deactivate live CancelRoute continuation",
            ));
        }
    }
    finish_unit_moves(state, owner, id, inputs, effects)
        .map_err(|e| Error::UnitMovement(Box::new(e)))?;
    pop_front_selected(
        state,
        actor,
        effects,
        selected,
        c6_core::unit::MAKE_TRADE_ROUTE_HASH,
    )
}
