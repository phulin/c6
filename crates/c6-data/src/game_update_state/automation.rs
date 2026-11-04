//! Concrete Operations.UpdateAutomation over CURRENT retained Unit/queue state.
//! This invokes existing queued activation, never request/Add or native policy.
use crate::loaded_actions::{self as actions, handlers::BoundHandlers, move_to};
use c6_core::{
    game_update::automation as core,
    loaded_state::LoadedState,
    skip_turn::{legality::ManagerQueries, StartData},
    unit::{OperationParameters, Unit},
};

/// The pinned stock registered handler classes all bind CheckAutoStart+40 to
/// BaseHandler2d964b0 (return true, no reads/writes/calls). Custom handler code
/// requires another implementation; a handler hash alone does not prove this.
#[derive(Clone, Copy)]
pub enum Scope {
    StockLinuxXp2RegisteredHandlers,
}

struct Current<'s, 'a, 'i, 'h, 'r, E, M> {
    state: &'s mut LoadedState,
    actor: (i32, u32),
    inputs: &'a actions::Inputs<'i, 'h>,
    handlers: &'a BoundHandlers<'h>,
    runtime: &'s mut move_to::Runtime<'r>,
    effects: &'s mut E,
    movement: &'s mut M,
}
impl<E: move_to::Effects, M: move_to::Movement<E>> Current<'_, '_, '_, '_, '_, E, M> {
    fn unit(&self) -> Result<&Unit, move_to::Error<move_to::EffectError<E>, M::Error>> {
        self.state
            .world()
            .player(self.actor.0)
            .map_err(move_to::Error::World)?
            .unit(self.actor.1)
            .map_err(move_to::Error::World)?
            .map(|u| u.record)
            .ok_or(move_to::Error::Unsupported(
                "automation retained unit removed",
            ))
    }
}
impl<E, M> core::Context for Current<'_, '_, '_, '_, '_, E, M>
where
    E: move_to::Effects
        + ManagerQueries<Unit, Parameters = OperationParameters, Error = &'static str>,
    M: move_to::Movement<E>,
{
    type Error = move_to::Error<move_to::EffectError<E>, M::Error>;
    fn reset_timer(&mut self) -> Result<(), Self::Error> {
        // Native archived CopyAssignment is unconditional, including value0.
        self.state
            .unit_mut(self.actor.0, self.actor.1)
            .map_err(move_to::Error::State)?
            .i_operation_timer = 0;
        Ok(())
    }
    fn front_hash(&mut self) -> Result<Option<u32>, Self::Error> {
        Ok(self
            .unit()?
            .k_operations
            .nodes
            .first()
            .map(|n| n.operation_hash))
    }
    fn delayed_death(&mut self) -> Result<bool, Self::Error> {
        Ok(self.unit()?.b_delayed_death)
    }
    fn check_auto_start(&mut self, hash: u32, _: &mut StartData) -> Result<bool, Self::Error> {
        self.handlers
            .validate_execution(self.state, self.inputs.handlers, self.inputs.operations)
            .map_err(|e| move_to::Error::Query(actions::Error::HandlerObservation(e)))?;
        let _ = self
            .handlers
            .runtime_id(hash)
            .map_err(|e| move_to::Error::Query(actions::Error::HandlerObservation(e)))?;
        // Source-qualified complete BaseHandler leaf. Unlike TurnStartCheck,
        // CheckAutoStart is TRUE; it leaves both StartData fields untouched.
        Ok(true)
    }
    fn activate(&mut self) -> Result<(), Self::Error> {
        move_to::activate_current_queued_move_to(
            self.state,
            self.actor,
            self.inputs,
            self.handlers,
            self.runtime,
            self.effects,
            self.movement,
        )
    }
    fn clear(&mut self) -> Result<(), Self::Error> {
        crate::loaded_upkeep::clear_unit_operations(
            self.state,
            self.actor.0,
            self.actor.1,
            self.effects,
        )
        .map_err(move_to::Error::Upkeep)
    }
}

/// One mandatory per-unit update, with the same persistent Effects/ScriptState.
/// Missing later handler coverage retains timer/earlier callbacks and errors.
/// The caller still owes Player.DoAutomation's unit iteration/attention and
/// diplomacy/reactivation tail; this is not a completed player deactivation.
#[allow(clippy::too_many_arguments)]
pub fn update<E, M>(
    state: &mut LoadedState,
    actor: (i32, u32),
    inputs: &actions::Inputs<'_, '_>,
    handlers: &BoundHandlers<'_>,
    runtime: &mut move_to::Runtime<'_>,
    effects: &mut E,
    movement: &mut M,
    _: Scope,
) -> Result<core::Outcome, move_to::Error<move_to::EffectError<E>, M::Error>>
where
    E: move_to::Effects
        + ManagerQueries<Unit, Parameters = OperationParameters, Error = &'static str>,
    M: move_to::Movement<E>,
{
    core::update(&mut Current {
        state,
        actor,
        inputs,
        handlers,
        runtime,
        effects,
        movement,
    })
}

#[derive(Debug)]
pub enum PlayerError<E, M> {
    Prefix(c6_core::turn_completion::Error),
    Operations(move_to::Error<E, M>),
    Readiness(actions::Error),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnitProgress {
    NoUnit,
    Advanced,
}

/// Current phase permissions use the existing exact phase definitions. No
/// temporary human-role override or copied processor array is introduced.
pub fn begin_player(
    state: &mut LoadedState,
    owner: i32,
    phases: &crate::unit_operation_manager::PhaseDefinitions,
) -> Result<c6_core::loaded_state::game_update::automation::Begin, c6_core::turn_completion::Error>
{
    c6_core::loaded_state::game_update::automation::begin(state, owner, |state, owner| {
        phases
            .from_processor(&state.retained().core.phase_processor, owner)
            .map_err(|_| {
                c6_core::turn_completion::Error::Unsupported("automation current tactical phase")
            })
    })
}

pub fn begin_after_deactivation_report(
    state: &mut LoadedState,
    runtime: &mut c6_core::loaded_state::game_update::Runtime,
    pending: &mut c6_core::loaded_state::game_update::deactivation::Continuation,
    phases: &crate::unit_operation_manager::PhaseDefinitions,
) -> Result<c6_core::loaded_state::game_update::automation::Begin, c6_core::turn_completion::Error>
{
    c6_core::loaded_state::game_update::deactivation::begin_automation(
        state,
        runtime,
        pending,
        |state, owner| {
            phases
                .from_processor(&state.retained().core.phase_processor, owner)
                .map_err(|_| {
                    c6_core::turn_completion::Error::Unsupported(
                        "automation current tactical phase",
                    )
                })
        },
    )
}

/// One current linked-node iteration. The native human flag and pre-update
/// HasPendingOperations are retained, while all later unit fields are current.
/// A completed unit pass still owes the exact diplomacy/reactivation tail.
#[allow(clippy::too_many_arguments)]
pub fn advance_player_unit<E, M>(
    state: &mut LoadedState,
    frame: &mut c6_core::loaded_state::game_update::automation::Frame,
    readiness: &actions::reporting::Provider<'_>,
    inputs: &actions::Inputs<'_, '_>,
    handlers: &BoundHandlers<'_>,
    runtime: &mut move_to::Runtime<'_>,
    effects: &mut E,
    movement: &mut M,
    scope: Scope,
    logging: Option<c6_core::loaded_state::game_update::deactivation::LoggingScope>,
) -> Result<UnitProgress, PlayerError<move_to::EffectError<E>, M::Error>>
where
    E: move_to::Effects
        + ManagerQueries<Unit, Parameters = OperationParameters, Error = &'static str>,
    M: move_to::Movement<E>,
{
    use c6_core::loaded_state::game_update::deactivation::LoggingScope;
    let result = (|| {
        let Some(work) = frame.take_unit(state).map_err(PlayerError::Prefix)? else {
            return Ok(UnitProgress::NoUnit);
        };
        update(
            state, work.actor, inputs, handlers, runtime, effects, movement, scope,
        )
        .map_err(PlayerError::Operations)?;
        // Stock Manager.IsDoingPartialMove2da2ae0 is constant false. This is
        // queried before the retained pre-update pending-operations boolean.
        if work.human
            && work.had_pending
            && frame.player_complete(state).map_err(PlayerError::Prefix)?
        {
            let ready = readiness
                .is_ready_to_move(
                    state,
                    frame.current_unit(state).map_err(PlayerError::Prefix)?,
                )
                .map_err(PlayerError::Readiness)?;
            if ready
                && !frame
                    .current_unit(state)
                    .map_err(PlayerError::Prefix)?
                    .b_delayed_death
                && frame
                    .current_unit(state)
                    .map_err(PlayerError::Prefix)?
                    .e_automation
                    == -1
                && !readiness
                    .can_cycle_past(
                        state,
                        frame.current_unit(state).map_err(PlayerError::Prefix)?,
                    )
                    .map_err(PlayerError::Readiness)?
            {
                frame.set_attention(state).map_err(PlayerError::Prefix)?;
                match logging {
                    Some(LoggingScope::StockFileSink) => {}
                    None => {
                        return Err(PlayerError::Prefix(
                            c6_core::turn_completion::Error::Unsupported(
                                "automation attention log callback effects",
                            ),
                        ))
                    }
                }
                frame.attention_logged();
            }
        }
        frame.advance(state).map_err(PlayerError::Prefix)?;
        Ok(UnitProgress::Advanced)
    })();
    if result.is_err() {
        frame.fail();
    }
    result
}
