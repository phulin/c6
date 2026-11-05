//! Explicit external unit decisions at finishProcessingFirst. This executes
//! source Add/completion effects; it never chooses orders or completes a roster.
use super::Runtime;
use crate::{
    loaded_actions::{self as actions, move_to},
    loaded_upkeep,
};
use c6_core::{
    loaded_state::LoadedState,
    map::UnitReference,
    unit::{ComponentId, OperationParameters, PlotCoord},
};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Operation {
    Fortify,
    MoveTo {
        destination: PlotCoord,
        modifiers: u32,
    },
    SkipTurn,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Decision {
    /// Manager.Add with source InsertMode::USE_REQUEST (0). Metadata belongs
    /// to the supplied choice; unsupported append/fromUser remains explicit.
    Add {
        actor: (i32, u32),
        operation: Operation,
        append: bool,
        from_user: bool,
    },
    FinishMoves {
        actor: (i32, u32),
    },
    SetProcessed {
        actor: (i32, u32),
        value: bool,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    /// Native Manager.Add returns void. This is submission, never acceptance.
    AddAttempted,
    MovesFinished,
    ProcessedSet,
}
/// Borrow the existing action/rules, search and sight owners. The optional
/// external provider owns this adapter; no extra world or manager is created.
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
impl Current<'_, '_, '_> {
    pub fn execute(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, OperationParameters>,
        decision: Decision,
    ) -> Result<Outcome, String> {
        backend
            .check(state)
            .map_err(|e| format!("external unit runtime: {e:?}"))?;
        match decision {
            Decision::Add {
                actor,
                operation,
                append,
                from_user,
            } => {
                if append || from_user {
                    return Err("external USE_REQUEST Add append/fromUser branch".into());
                }
                // This source-specific entry is the inactive, nonautomation
                // deactivation boundary. Manager.CanStart2da20f0 rejects before
                // handler/MoveTo modifier interpretation. Do not invent active
                // execution for a new provider site or reinterpret modifier64.
                if state
                    .turn_active(actor.0)
                    .map_err(|e| format!("external Add current owner: {e:?}"))?
                    || state
                        .processing_automation()
                        .map_err(|e| format!("external Add automation: {e:?}"))?
                        == actor.0
                {
                    return Err("external Add active/automation execution continuation".into());
                }
                let (hash, parameters) = match operation {
                    Operation::Fortify => (c6_core::unit::FORTIFY_HASH, OperationParameters::None),
                    Operation::MoveTo {
                        destination,
                        modifiers,
                    } => (
                        c6_core::unit::MOVE_TO_HASH,
                        OperationParameters::MoveTo {
                            location: destination,
                            modifiers,
                        },
                    ),
                    Operation::SkipTurn => {
                        (c6_core::unit::SKIP_TURN_HASH, OperationParameters::None)
                    }
                };
                let mut sight = self
                    .sight
                    .try_borrow_mut()
                    .map_err(|e| format!("external Add sight: {e}"))?;
                let mut search = self
                    .search
                    .try_borrow_mut()
                    .map_err(|e| format!("external Add search: {e}"))?;
                let mut movement = move_to::ComputedMovement {
                    path: self.path,
                    placement: self.placement,
                    queries: self.queries,
                    search: &mut search,
                    execution: crate::formation_path_state::ExecutionContext::Exclusive,
                };
                // CheckInsert is the stock Base leaf (true) for these three
                // handlers. Existing Add owns clear/defaults/push/report and
                // Activate's real failed-legality/pop/previous-activity tail.
                move_to::execute_queued_manager(
                    state,
                    actor,
                    hash,
                    parameters,
                    self.inputs,
                    self.handlers,
                    &mut move_to::Runtime {
                        visibility: &mut sight,
                        manager_width: self.manager_width,
                    },
                    backend,
                    &mut movement,
                )
                .map_err(|e| format!("external unit Add: {e:?}"))?;
                Ok(Outcome::AddAttempted)
            }
            Decision::FinishMoves { actor } => {
                loaded_upkeep::finish_unit_moves(
                    state,
                    actor.0,
                    actor.1,
                    self.inputs
                        .movement
                        .ok_or("external FinishMoves current movement inputs")?,
                    backend,
                )
                .map_err(|e| format!("external FinishMoves: {e:?}"))?;
                Ok(Outcome::MovesFinished)
            }
            Decision::SetProcessed { actor, value } => {
                let reference = UnitReference::from_component_id(ComponentId {
                    owner: i16::try_from(actor.0).map_err(|_| "external processed owner range")?,
                    component_type: 1,
                    id: actor.1,
                });
                let unit = state
                    .world()
                    .unit(reference)
                    .map_err(|e| format!("external processed current unit: {e:?}"))?
                    .ok_or("external processed Unit missing")?;
                if unit.record.b_processed_turn != value {
                    state
                        .unit_mut(actor.0, actor.1)
                        .map_err(|e| format!("external processed store: {e:?}"))?
                        .b_processed_turn = value;
                    // SetProcessedTurn2c27f70 reports d70ba51f only on change.
                    // Reuse the qualified creation/control route and CURRENT
                    // snapshot provider; no native cache DTO is manufactured.
                    let snapshot = backend
                        .unit_creation
                        .as_ref()
                        .and_then(|s| s.control.as_ref())
                        .ok_or("external processed report binding")?
                        .processed_report_snapshot;
                    if !backend
                        .movement()
                        .map_err(|e| format!("external processed movement owner: {e:?}"))?
                        .reporting_suspended
                        && snapshot
                    {
                        backend
                            .reporting
                            .observe_unit_snapshot(state, reference)
                            .map_err(|e| format!("external processed report: {e:?}"))?;
                    }
                }
                Ok(Outcome::ProcessedSet)
            }
        }
    }
}
