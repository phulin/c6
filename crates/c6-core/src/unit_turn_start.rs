//! Bounded Linux XP2 per-actor activation, not a complete player turn.
//!
//! Source-only ordering from Player::TurnManager::DoTurnActivate (0x2ba0b60).
//! The caller owns the native current cursor/Find handle; this does not enumerate
//! units or perform the subsequent, separate formation pass. Native differential
//! acceptance and complete gameplay/action/turn support remain unimplemented.

use crate::unit::Unit;
use crate::unit_operations_clear::ClearOperationsContext;
use crate::unit_turn_upkeep::{
    restore_attacks_remaining, restore_movement, RestoreAttacksContext, RestoreAttacksError,
    RestoreMovementContext, RestoreMovementError,
};

/// Required live providers for Operations::Manager::TurnStartCheck (0x2da1960).
/// The stock manager binding, current queue, and handler registry must be known.
pub trait OperationsTurnStartContext: ClearOperationsContext {
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error>;
    /// Execute the actual registered handler's +0x38 virtual on the retained
    /// actor, returning its native bool. The supplied hash came from the current
    /// front AFTER timer reset. Unknown handlers/effects/RNG must fail explicitly.
    /// Do not choose new player orders or run native AI policy here.
    fn front_handler_turn_start(&mut self, operation_hash: u32) -> Result<bool, Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationsTurnStartOutcome {
    Empty,
    DelayedDeath,
    Retained,
    PopFrontCompleted,
}

/// The native timer write occurs even for empty queues and delayed-death units.
/// This is PopFront of the CURRENT front following the handler, never Clear of
/// queued player orders. PopFront's existing provider retains its selected node
/// identity across its own synchronous callbacks. Errors retain prior effects.
pub fn operations_turn_start<C: OperationsTurnStartContext>(
    context: &mut C,
) -> Result<OperationsTurnStartOutcome, C::Error> {
    context.unit_mut()?.i_operation_timer = 0;
    let Some(hash) = context
        .unit()?
        .k_operations
        .nodes
        .first()
        .map(|n| n.operation_hash)
    else {
        return Ok(OperationsTurnStartOutcome::Empty);
    };
    if context.unit()?.b_delayed_death {
        return Ok(OperationsTurnStartOutcome::DelayedDeath);
    }
    if !context.front_handler_turn_start(hash)? {
        return Ok(OperationsTurnStartOutcome::Retained);
    }
    context.pop_front_operation()?;
    Ok(OperationsTurnStartOutcome::PopFrontCompleted)
}

/// SetProcessedTurn(false), local-authority field effect at 0x2c27f70.
/// The setter reports 0xd70ba51f only on a changed TRUE value. This reset has no
/// report or RNG. Native archive bookkeeping follows the audited exclusion.
pub fn reset_processed_turn(unit: &mut Unit) {
    unit.b_processed_turn = false;
}

/// Providers must retain the same actor identity while resolving current
/// authoritative records after callbacks. Each manager preparation resolves the
/// CURRENT singleton again and verifies its supported stock virtual binding;
/// retaining a previous manager or silently accepting an override is invalid.
pub trait UnitTurnStartContext:
    RestoreMovementContext
    + RestoreAttacksContext<Error = <Self as RestoreMovementContext>::Error>
    + OperationsTurnStartContext<Error = <Self as RestoreMovementContext>::Error>
{
    fn current_unit_manager(&mut self) -> Result<(), <Self as RestoreMovementContext>::Error>;
    fn current_operations_manager(&mut self)
        -> Result<(), <Self as RestoreMovementContext>::Error>;
    /// Current EditExperience then CheckPromoteNotification (0x2c18a40).
    /// Includes CanPromote, its runtime notification flag write, and all required
    /// notifications/callbacks/RNG. A UI name does not authorize omitting effects.
    fn check_promote_notification(&mut self)
        -> Result<(), <Self as RestoreMovementContext>::Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnitTurnStartError<E> {
    MovementManager(E),
    Movement(RestoreMovementError<E>),
    AttacksManager(E),
    Attacks(RestoreAttacksError<E>),
    OperationsManager(E),
    Operations(E),
    Promotion(E),
    ProcessedTurn(E),
}

/// Execute one resolved Unit's native activation sequence, preserving eager
/// effects and fallible live readbacks. Missing support after prior effects is
/// partial internal progress; this is not a successful arena action. Healing of
/// damage belongs to end-turn work and is NOT called here. Movement restoration
/// itself assigns healing eligibility via the existing source-derived helper.
pub fn activate_unit<C: UnitTurnStartContext>(
    context: &mut C,
) -> Result<(), UnitTurnStartError<<C as RestoreMovementContext>::Error>> {
    context
        .current_unit_manager()
        .map_err(UnitTurnStartError::MovementManager)?;
    restore_movement(Some(&mut *context)).map_err(UnitTurnStartError::Movement)?;
    context
        .current_unit_manager()
        .map_err(UnitTurnStartError::AttacksManager)?;
    restore_attacks_remaining(context).map_err(UnitTurnStartError::Attacks)?;
    context
        .current_operations_manager()
        .map_err(UnitTurnStartError::OperationsManager)?;
    operations_turn_start(context).map_err(UnitTurnStartError::Operations)?;
    context
        .check_promote_notification()
        .map_err(UnitTurnStartError::Promotion)?;
    reset_processed_turn(
        OperationsTurnStartContext::unit_mut(context).map_err(UnitTurnStartError::ProcessedTurn)?,
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Context {
        unit: Unit,
        called: Vec<&'static str>,
        pop: bool,
        fail: bool,
    }
    impl ClearOperationsContext for Context {
        type Error = &'static str;
        fn unit(&self) -> Result<&Unit, Self::Error> {
            Ok(&self.unit)
        }
        fn deactivate_operations(&mut self) -> Result<(), Self::Error> {
            panic!("Clear must not be called")
        }
        fn pop_front_operation(&mut self) -> Result<(), Self::Error> {
            self.called.push("pop");
            // Synthetic callback-free provider, not production node identity.
            self.unit.k_operations.nodes.remove(0);
            Ok(())
        }
        fn clear_current_formation_path(&mut self) -> Result<(), Self::Error> {
            panic!("Clear must not be called")
        }
    }
    impl OperationsTurnStartContext for Context {
        fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
            Ok(&mut self.unit)
        }
        fn front_handler_turn_start(&mut self, _: u32) -> Result<bool, Self::Error> {
            assert_eq!(self.unit.i_operation_timer, 0);
            self.called.push("handler");
            self.unit.i_damage = 12;
            if self.fail {
                Err("handler unsupported")
            } else {
                Ok(self.pop)
            }
        }
    }
    impl RestoreMovementContext for Context {
        type Error = &'static str;
        fn unit(&self) -> Result<&Unit, Self::Error> {
            Ok(&self.unit)
        }
        fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
            Ok(&mut self.unit)
        }
        fn max_moves(&self) -> Result<i32, Self::Error> {
            Ok(2)
        }
        fn preflight_restore_movement(&self, _: bool) -> Result<(), Self::Error> {
            Ok(())
        }
        fn send_movement_report(
            &mut self,
            _: crate::unit_turn_upkeep::MovementReport,
        ) -> Result<(), Self::Error> {
            self.called.push("movement");
            Ok(())
        }
        fn sync_current_formation(&mut self) -> Result<(), Self::Error> {
            self.called.push("formation");
            Ok(())
        }
        fn owner_is_barbarian(&self) -> Result<bool, Self::Error> {
            Ok(false)
        }
    }
    impl RestoreAttacksContext for Context {
        type Error = &'static str;
        fn unit(&self) -> Result<&Unit, Self::Error> {
            Ok(&self.unit)
        }
        fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
            Ok(&mut self.unit)
        }
        fn combat_max_num_attacks(&self) -> Result<i32, Self::Error> {
            Ok(1)
        }
        fn preflight_attacks_report(&self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn send_attacks_report(
            &mut self,
            _: crate::unit_turn_upkeep::AttacksRemainingReport,
        ) -> Result<(), Self::Error> {
            self.called.push("attacks");
            self.unit.i_operation_timer = 9;
            Ok(())
        }
    }
    impl UnitTurnStartContext for Context {
        fn current_unit_manager(&mut self) -> Result<(), &'static str> {
            self.called.push("unit_manager");
            Ok(())
        }
        fn current_operations_manager(&mut self) -> Result<(), &'static str> {
            self.called.push("operations_manager");
            Ok(())
        }
        fn check_promote_notification(&mut self) -> Result<(), &'static str> {
            assert_eq!(self.unit.i_operation_timer, 0);
            assert!(self.unit.b_processed_turn);
            self.called.push("promotion");
            if self.fail {
                Err("promotion unsupported")
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn activation_computes_resource_writes_then_operations_promotion_processed() {
        let mut c = context();
        assert_eq!(activate_unit(&mut c), Ok(()));
        assert_eq!(
            c.called,
            [
                "unit_manager",
                "movement",
                "formation",
                "movement",
                "unit_manager",
                "attacks",
                "operations_manager",
                "promotion"
            ]
        );
        assert_eq!(c.unit.x_moves_remaining, 512);
        assert_eq!(c.unit.i_attacks_remaining, 1);
        assert_eq!(c.unit.i_operation_timer, 0);
        assert!(!c.unit.b_processed_turn);
        let mut c = context();
        c.fail = true;
        assert_eq!(
            activate_unit(&mut c),
            Err(UnitTurnStartError::Promotion("promotion unsupported"))
        );
        assert_eq!(c.unit.x_moves_remaining, 512);
        assert_eq!(c.unit.i_operation_timer, 0);
        assert!(c.unit.b_processed_turn);
    }
    fn context() -> Context {
        let mut unit = crate::unit_turn_upkeep::tests::source_only_unit();
        unit.i_operation_timer = 8;
        unit.b_processed_turn = true;
        Context {
            unit,
            called: vec![],
            pop: false,
            fail: false,
        }
    }
    fn node() -> crate::unit::OperationNode {
        crate::unit::OperationNode {
            operation_hash: 0x7eb896fb,
            directive: 0,
            flags: 0,
            push_turn: 0,
            previous_activity: 0,
            previous_automation: 0,
            parameters: crate::unit::OperationParameters::None,
        }
    }
    #[test]
    fn empty_and_death_still_reset_timer_but_preserve_queue_and_processed() {
        let mut c = context();
        assert_eq!(
            operations_turn_start(&mut c),
            Ok(OperationsTurnStartOutcome::Empty)
        );
        c.unit.k_operations.nodes.push(node());
        c.unit.b_delayed_death = true;
        c.unit.i_operation_timer = 6;
        assert_eq!(
            operations_turn_start(&mut c),
            Ok(OperationsTurnStartOutcome::DelayedDeath)
        );
        assert_eq!(c.unit.i_operation_timer, 0);
        assert_eq!(c.unit.k_operations.nodes.len(), 1);
        assert!(c.unit.b_processed_turn);
        assert!(c.called.is_empty());
    }
    #[test]
    fn handler_result_controls_single_pop_and_failure_preserves_prefix() {
        let mut c = context();
        c.unit.k_operations.nodes = vec![node(), node()];
        assert_eq!(
            operations_turn_start(&mut c),
            Ok(OperationsTurnStartOutcome::Retained)
        );
        assert_eq!(c.unit.k_operations.nodes.len(), 2);
        c.pop = true;
        assert_eq!(
            operations_turn_start(&mut c),
            Ok(OperationsTurnStartOutcome::PopFrontCompleted)
        );
        assert_eq!(c.unit.k_operations.nodes.len(), 1);
        assert_eq!(c.called, ["handler", "handler", "pop"]);
        c.fail = true;
        c.unit.i_operation_timer = 7;
        assert_eq!(operations_turn_start(&mut c), Err("handler unsupported"));
        assert_eq!(c.unit.i_operation_timer, 0);
        assert_eq!(c.unit.i_damage, 12);
        assert_eq!(c.unit.k_operations.nodes.len(), 1);
        assert!(c.unit.b_processed_turn);
        reset_processed_turn(&mut c.unit);
        assert!(!c.unit.b_processed_turn);
    }
}
