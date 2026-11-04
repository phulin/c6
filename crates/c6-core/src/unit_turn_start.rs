//! Bounded Linux XP2 units/formation activation suffix, not a complete player turn.
//!
//! Source-only ordering from Player::TurnManager::DoTurnActivate (0x2ba0b60).
//! Includes first-activation gating, native cursor orchestration and the separate
//! formation pass, after earlier player/city/resource upkeep. Native differential
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
    PromotionNotificationContext
    + RestoreAttacksContext<Error = <Self as RestoreMovementContext>::Error>
    + OperationsTurnStartContext<Error = <Self as RestoreMovementContext>::Error>
{
    fn current_unit_manager(&mut self) -> Result<(), <Self as RestoreMovementContext>::Error>;
    fn current_operations_manager(&mut self)
        -> Result<(), <Self as RestoreMovementContext>::Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnitTurnStartError<E> {
    NullUnit,
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
    activate_unit_resolved(context, true)
}

fn activate_unit_resolved<C: UnitTurnStartContext>(
    context: &mut C,
    found: bool,
) -> Result<(), UnitTurnStartError<<C as RestoreMovementContext>::Error>> {
    context
        .current_unit_manager()
        .map_err(UnitTurnStartError::MovementManager)?;
    restore_movement(if found { Some(&mut *context) } else { None })
        .map_err(UnitTurnStartError::Movement)?;
    context
        .current_unit_manager()
        .map_err(UnitTurnStartError::AttacksManager)?;
    if !found {
        return Err(UnitTurnStartError::NullUnit);
    }
    restore_attacks_remaining(context).map_err(UnitTurnStartError::Attacks)?;
    context
        .current_operations_manager()
        .map_err(UnitTurnStartError::OperationsManager)?;
    operations_turn_start(context).map_err(UnitTurnStartError::Operations)?;
    check_promote_notification(context).map_err(UnitTurnStartError::Promotion)?;
    reset_processed_turn(
        OperationsTurnStartContext::unit_mut(context).map_err(UnitTurnStartError::ProcessedTurn)?,
    );
    Ok(())
}

/// Providers for CheckPromoteNotification, retaining the actual experience
/// parent and notification identities across synchronous calls. No provider may
/// select a promotion: external policies own that separate player decision.
pub trait PromotionNotificationContext: RestoreMovementContext {
    type Notification: Copy;
    type NotificationManager: Copy;
    fn promotion_owner_is_human(&mut self) -> Result<bool, Self::Error>;
    fn notification_manager(&mut self) -> Result<Self::NotificationManager, Self::Error>;
    fn promotion_owner(&mut self) -> Result<i32, Self::Error>;
    fn find_promote_notification(
        &mut self,
        manager: Self::NotificationManager,
        owner: i32,
    ) -> Result<Option<Self::Notification>, Self::Error>;
    /// Actual Create(0x2ba6e111, owner, false, null typed-variant args). Null is distinct from error.
    fn create_promote_notification(
        &mut self,
        manager: Self::NotificationManager,
        owner: i32,
    ) -> Result<Option<Self::Notification>, Self::Error>;
    fn add_promotable_unit(
        &mut self,
        notification: Self::Notification,
        identity: crate::unit::ComponentId,
    ) -> Result<(), Self::Error>;
    fn publish_promote_notification(
        &mut self,
        manager: Self::NotificationManager,
        owner: i32,
        notification: Self::Notification,
    ) -> Result<(), Self::Error>;
}

/// Computes flags and find/create/add/publish branches, not a promotion choice.
/// Clearing notify-needed precedes owner queries and remains on later failure.
pub fn check_promote_notification<C: PromotionNotificationContext>(
    c: &mut C,
) -> Result<(), C::Error> {
    if !c.unit()?.k_experience.b_can_promote || !c.unit()?.k_experience.b_can_promote_notify_needed
    {
        return Ok(());
    }
    c.unit_mut()?.k_experience.b_can_promote_notify_needed = false;
    if !c.promotion_owner_is_human()? {
        return Ok(());
    }
    let manager = c.notification_manager()?;
    let owner = c.promotion_owner()?;
    let (notification, created) =
        if let Some(notification) = c.find_promote_notification(manager, owner)? {
            (notification, false)
        } else {
            let manager = c.notification_manager()?;
            let owner = c.promotion_owner()?;
            let Some(notification) = c.create_promote_notification(manager, owner)? else {
                return Ok(());
            };
            (notification, true)
        };
    let actor = c.unit()?;
    let identity = crate::unit::ComponentId {
        owner: actor.e_owner as i16,
        component_type: 1,
        id: actor.id,
    };
    c.add_promotable_unit(notification, identity)?;
    if created {
        let manager = c.notification_manager()?;
        let owner = c.promotion_owner()?;
        c.publish_promote_notification(manager, owner, notification)?;
    }
    Ok(())
}

use crate::formation_upkeep::{
    update_attributes, FormationUpkeepContext, FormationUpkeepError, IntegerQuery,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormationRestoredReport {
    pub event_hash: u32,
    pub identity: crate::unit::ComponentId,
    /// Raw Q8 for 0x3d75b3ed; integer bool for 0xc52231ab.
    pub value: i32,
}

pub trait FormationRestoreContext: FormationUpkeepContext {
    fn unit_mut(&mut self, handle: Self::UnitHandle) -> Result<&mut Unit, Self::Error>;
    /// Complete synchronous subscribers, including all gameplay and RNG effects.
    fn send_formation_restored(
        &mut self,
        report: FormationRestoredReport,
    ) -> Result<(), Self::Error>;
}

/// Formation::RestoreMovement (0x293d190), including GetMaxMoves/SetHasMoved.
/// The integer-max fold caches End; the attention loop refreshes End. Native
/// member cursors must remain valid; unavailable records fail without rollback.
pub fn restore_formation_movement<C: FormationRestoreContext>(
    c: &mut C,
) -> Result<(), FormationUpkeepError<C::Error>> {
    let result = |e| FormationUpkeepError::ContinuationFailed(e);
    let units = c.units().map_err(result)?;
    let end = c.formation().map_err(result)?.unit_ids.len();
    let mut minimum = 9001_i32;
    let mut escort_maximum = 0;
    for index in 0..end {
        let id = *c
            .formation()
            .map_err(result)?
            .unit_ids
            .get(index)
            .ok_or(FormationUpkeepError::InvalidatedMemberCursor { index })?;
        if let Some(handle) = c.find_unit(units, id).map_err(result)? {
            let moves = c.integer(handle, IntegerQuery::MaxMoves).map_err(result)?;
            minimum = minimum.min(moves);
            if c.unit(handle)
                .map_err(result)?
                .k_movement_modifiers
                .b_escort_mobility
            {
                escort_maximum = escort_maximum.max(moves);
            }
        }
    }
    let maximum = minimum.max(escort_maximum);
    let changed = c.formation().map_err(result)?.moves_remaining_raw != maximum.wrapping_shl(8);
    // Native compares AsInt separately; a fractional remainder may preserve the
    // field here, while still selecting the later changed report.
    if c.formation().map_err(result)?.moves_remaining_raw >> 8 != maximum {
        c.formation_mut().map_err(result)?.moves_remaining_raw = maximum.wrapping_shl(8);
    }
    crate::movement_execution::set_formation_has_moved(c, false, |c, handle, attention| {
        c.unit_mut(handle)?.b_needs_attention = attention;
        Ok(())
    })?;
    let old_zoc = c.formation().map_err(result)?.has_moved_into_zoc;
    c.formation_mut().map_err(result)?.has_moved_into_zoc = false;
    update_attributes(c)?;
    if changed {
        let f = c.formation().map_err(result)?;
        let identity = crate::unit::ComponentId {
            owner: f.owner as i16,
            component_type: 0x2c,
            id: f.id,
        };
        let value = c.formation().map_err(result)?.moves_remaining_raw;
        c.send_formation_restored(FormationRestoredReport {
            event_hash: 0x3d75b3ed,
            identity,
            value,
        })
        .map_err(result)?;
    }
    if old_zoc {
        let f = c.formation().map_err(result)?;
        let identity = crate::unit::ComponentId {
            owner: f.owner as i16,
            component_type: 0x2c,
            id: f.id,
        };
        let value = i32::from(c.formation().map_err(result)?.has_moved_into_zoc);
        c.send_formation_restored(FormationRestoredReport {
            event_hash: 0xc52231ab,
            identity,
            value,
        })
        .map_err(result)?;
    }
    Ok(())
}

/// Bind these fields to the current player's actual processor flags. No default
/// or inferred initialization is supplied. This is not a complete Processor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessorActivationFlags {
    pub needs_first_turn_activate: bool,
    pub needs_turn_activate_event: bool,
}

/// Processor::DoTurnActivate (0x2b5ac00) flag prefix only: absent request has no
/// effects; otherwise capture first-activation, clear it, then clear the event
/// request, before calling TurnManager. The returned bool must survive earlier
/// player/city/resource work before entering the suffix. False is a real repeated
/// activation request, distinct from None. This does not run TurnManager or its
/// final event/signals/AI continuation and does not claim a complete activation.
pub fn take_processor_activation_request(flags: &mut ProcessorActivationFlags) -> Option<bool> {
    if !flags.needs_turn_activate_event {
        return None;
    }
    let first = flags.needs_first_turn_activate;
    flags.needs_first_turn_activate = false;
    flags.needs_turn_activate_event = false;
    Some(first)
}

/// Native cursor/receiver providers. These represent actual recycled-container
/// cursors, not a sorted ID list or a snapshot of the initial members. Invalid
/// cursor dereference/increment/collection receivers must return an error.
pub trait ActivationSuffixContext {
    type Error;
    type Units: Copy;
    type UnitCursor;
    type UnitHandle: Copy;
    type Formations: Copy;
    type FormationCursor;
    type FormationHandle: Copy;
    type Actor: UnitTurnStartContext + RestoreMovementContext<Error = Self::Error>;
    type Formation: FormationRestoreContext<Error = Self::Error>;
    fn player_units(&mut self) -> Result<Self::Units, Self::Error>;
    fn units_begin(&mut self, units: Self::Units) -> Result<Self::UnitCursor, Self::Error>;
    fn units_end(&mut self, units: Self::Units) -> Result<Self::UnitCursor, Self::Error>;
    fn unit_cursors_equal(
        &mut self,
        cursor: &Self::UnitCursor,
        end: &Self::UnitCursor,
    ) -> Result<bool, Self::Error>;
    /// Native iterator-> followed by GetID; null/invalid dereference is Err.
    fn unit_cursor_id(&mut self, cursor: &Self::UnitCursor) -> Result<u32, Self::Error>;
    fn find_activation_unit(
        &mut self,
        units: Self::Units,
        id: u32,
    ) -> Result<Option<Self::UnitHandle>, Self::Error>;
    /// Retain exactly the Find result, even null. Null contexts must still allow
    /// manager lookups; no actor field access occurs before explicit NullUnit.
    fn actor(&mut self, handle: Option<Self::UnitHandle>) -> Result<&mut Self::Actor, Self::Error>;
    fn increment_unit_cursor(&mut self, cursor: &mut Self::UnitCursor) -> Result<(), Self::Error>;
    fn player_formations(&mut self) -> Result<Self::Formations, Self::Error>;
    fn formations_end(
        &mut self,
        formations: Self::Formations,
    ) -> Result<Self::FormationCursor, Self::Error>;
    fn formations_begin(
        &mut self,
        formations: Self::Formations,
    ) -> Result<Self::FormationCursor, Self::Error>;
    fn formation_cursors_equal(
        &mut self,
        cursor: &Self::FormationCursor,
        end: &Self::FormationCursor,
    ) -> Result<bool, Self::Error>;
    fn formation_at_cursor(
        &mut self,
        cursor: &Self::FormationCursor,
    ) -> Result<Option<Self::FormationHandle>, Self::Error>;
    fn formation(
        &mut self,
        handle: Self::FormationHandle,
    ) -> Result<&mut Self::Formation, Self::Error>;
    fn increment_formation_cursor(
        &mut self,
        cursor: &mut Self::FormationCursor,
    ) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActivationSuffixError<E> {
    CursorOrReceiver(E),
    Actor(UnitTurnStartError<E>),
    NullFormation,
    Formation(FormationUpkeepError<E>),
}

/// Only the units/formation suffix AFTER earlier first-activation upkeep. The
/// bool is the actual captured TurnManager input, not proof of initialization.
/// False skips both passes; it does not skip the later activation continuation,
/// which is outside this function. No signals/influence/native AI are run here.
/// Failures retain every completed actor/formation effect in original order.
pub fn activate_units_and_formations_suffix<C: ActivationSuffixContext>(
    c: &mut C,
    first_activation: bool,
) -> Result<(), ActivationSuffixError<C::Error>> {
    if !first_activation {
        return Ok(());
    }
    let access = |e| ActivationSuffixError::CursorOrReceiver(e);
    let units = c.player_units().map_err(access)?;
    let mut cursor = c.units_begin(units).map_err(access)?;
    let mut end = c.units_end(units).map_err(access)?;
    while !c.unit_cursors_equal(&cursor, &end).map_err(access)? {
        let id = c.unit_cursor_id(&cursor).map_err(access)?;
        let handle = c.find_activation_unit(units, id).map_err(access)?;
        let actor = c.actor(handle).map_err(access)?;
        activate_unit_resolved(actor, handle.is_some()).map_err(ActivationSuffixError::Actor)?;
        c.increment_unit_cursor(&mut cursor).map_err(access)?;
        end = c.units_end(units).map_err(access)?;
    }
    let formations = c.player_formations().map_err(access)?;
    let end = c.formations_end(formations).map_err(access)?;
    let mut cursor = c.formations_begin(formations).map_err(access)?;
    while !c.formation_cursors_equal(&cursor, &end).map_err(access)? {
        let handle = c
            .formation_at_cursor(&cursor)
            .map_err(access)?
            .ok_or(ActivationSuffixError::NullFormation)?;
        restore_formation_movement(c.formation(handle).map_err(access)?)
            .map_err(ActivationSuffixError::Formation)?;
        c.increment_formation_cursor(&mut cursor).map_err(access)?;
    }
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
        human: bool,
        existing: Option<u32>,
        created: Option<u32>,
        added: Option<crate::unit::ComponentId>,
        units_count: Option<std::rc::Rc<std::cell::Cell<usize>>>,
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
            if let Some(count) = &self.units_count {
                count.set(2);
            }
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
    }
    impl PromotionNotificationContext for Context {
        type Notification = u32;
        type NotificationManager = u32;
        fn promotion_owner_is_human(&mut self) -> Result<bool, &'static str> {
            assert_eq!(self.unit.i_operation_timer, 0);
            assert!(self.unit.b_processed_turn);
            assert!(!self.unit.k_experience.b_can_promote_notify_needed);
            self.called.push("promotion");
            if self.fail {
                Err("promotion unsupported")
            } else {
                Ok(self.human)
            }
        }
        fn notification_manager(&mut self) -> Result<u32, &'static str> {
            self.called.push("notification_manager");
            Ok(0)
        }
        fn promotion_owner(&mut self) -> Result<i32, &'static str> {
            self.called.push("owner");
            Ok(self.unit.e_owner)
        }
        fn find_promote_notification(
            &mut self,
            _: u32,
            _: i32,
        ) -> Result<Option<u32>, &'static str> {
            self.called.push("find");
            Ok(self.existing)
        }
        fn create_promote_notification(
            &mut self,
            _: u32,
            _: i32,
        ) -> Result<Option<u32>, &'static str> {
            self.called.push("create");
            Ok(self.created)
        }
        fn add_promotable_unit(
            &mut self,
            _: u32,
            identity: crate::unit::ComponentId,
        ) -> Result<(), &'static str> {
            self.called.push("add");
            self.added = Some(identity);
            self.unit.e_owner = 8;
            Ok(())
        }
        fn publish_promote_notification(
            &mut self,
            _: u32,
            owner: i32,
            _: u32,
        ) -> Result<(), &'static str> {
            self.called.push("publish");
            assert_eq!(owner, 8);
            Ok(())
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
        unit.k_experience.b_can_promote = true;
        unit.k_experience.b_can_promote_notify_needed = true;
        Context {
            unit,
            called: vec![],
            pop: false,
            fail: false,
            human: false,
            existing: None,
            created: None,
            added: None,
            units_count: None,
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
        c.unit.k_operations.nodes = vec![node(), node()].into();
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
    struct Form {
        formation: crate::unit::Formation,
        unit: Unit,
        maximum: i32,
        before_update: std::cell::Cell<i32>,
        reports: Vec<FormationRestoredReport>,
        fail_report: bool,
        formation_count: Option<std::rc::Rc<std::cell::Cell<usize>>>,
    }
    impl Form {
        fn new() -> Self {
            let mut unit = crate::unit_turn_upkeep::tests::source_only_unit();
            unit.x_moves_remaining = 512;
            unit.b_needs_attention = true;
            Self {
                formation: crate::unit::Formation {
                    id: 9,
                    location: crate::unit::PlotCoord { x: 0, y: 0 },
                    owner: 2,
                    moves_remaining_raw: 1,
                    domain: 0,
                    is_embarked: false,
                    can_embark: false,
                    can_attack: false,
                    is_ranged_combat: false,
                    can_capture: false,
                    has_moved: true,
                    has_moved_into_zoc: true,
                    ignores_zoc: false,
                    last_path: vec![],
                    last_path_cache_location: crate::unit::PlotCoord { x: 0, y: 0 },
                    unit_ids: vec![0],
                },
                unit,
                maximum: 3,
                before_update: std::cell::Cell::new(0),
                reports: vec![],
                fail_report: false,
                formation_count: None,
            }
        }
    }
    impl FormationUpkeepContext for Form {
        type Error = &'static str;
        type UnitHandle = u32;
        type Definitions = ();
        type Units = ();
        fn formation(&self) -> Result<&crate::unit::Formation, Self::Error> {
            Ok(&self.formation)
        }
        fn formation_mut(&mut self) -> Result<&mut crate::unit::Formation, Self::Error> {
            Ok(&mut self.formation)
        }
        fn preflight_update_attributes(&self) -> Result<(), Self::Error> {
            self.before_update.set(self.formation.moves_remaining_raw);
            if self.formation.unit_ids.contains(&0) {
                assert!(!self.unit.b_needs_attention);
            }
            assert!(!self.formation.has_moved);
            Ok(())
        }
        fn definitions(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn units(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn find_unit(&mut self, _: (), id: u32) -> Result<Option<u32>, Self::Error> {
            Ok((id == 0).then_some(id))
        }
        fn unit(&self, _: u32) -> Result<&Unit, Self::Error> {
            Ok(&self.unit)
        }
        fn integer(&mut self, _: u32, query: IntegerQuery) -> Result<i32, Self::Error> {
            Ok(match query {
                IntegerQuery::MaxMoves => self.maximum,
                IntegerQuery::Domain => 0,
                _ => 1,
            })
        }
        fn boolean(
            &mut self,
            _: u32,
            query: crate::formation_upkeep::BooleanQuery,
        ) -> Result<bool, Self::Error> {
            Ok(query != crate::formation_upkeep::BooleanQuery::MovementDisabledThisTurn)
        }
        fn native_type_index(&mut self, _: u32) -> Result<u32, Self::Error> {
            Ok(0)
        }
        fn definition_can_capture(&mut self, _: (), _: u32) -> Result<Option<bool>, Self::Error> {
            Ok(Some(true))
        }
        fn send_member_movement(
            &mut self,
            _: crate::unit_turn_upkeep::MovementReport,
        ) -> Result<(), Self::Error> {
            Ok(())
        }
    }
    impl FormationRestoreContext for Form {
        fn unit_mut(&mut self, _: u32) -> Result<&mut Unit, Self::Error> {
            Ok(&mut self.unit)
        }
        fn send_formation_restored(
            &mut self,
            report: FormationRestoredReport,
        ) -> Result<(), Self::Error> {
            self.reports.push(report);
            if report.event_hash == 0x3d75b3ed {
                // First subscriber changes identity/ZOC seen by the second.
                self.formation.owner = 8;
                self.formation.has_moved_into_zoc = true;
                if let Some(count) = &self.formation_count {
                    count.set(2);
                }
            }
            if self.fail_report {
                Err("formation report unsupported")
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn formation_restoration_uses_current_post_update_payload_and_callback_state() {
        let mut c = Form::new();
        assert_eq!(restore_formation_movement(&mut c), Ok(()));
        assert_eq!(c.before_update.get(), 768);
        assert_eq!(c.formation.moves_remaining_raw, 512);
        assert!(c.formation.has_moved); // UpdateAttributes sees 512 < 3*256.
        assert!(!c.unit.b_needs_attention);
        assert_eq!(
            c.reports
                .iter()
                .map(|r| (r.event_hash, r.identity.owner, r.value))
                .collect::<Vec<_>>(),
            [(0x3d75b3ed, 2, 512), (0xc52231ab, 8, 1)]
        );
        assert_eq!(c.reports[0].identity.component_type, 0x2c);
    }
    #[test]
    fn formation_fractional_compare_and_failure_keep_native_prefix() {
        let mut c = Form::new();
        c.maximum = 2;
        c.formation.has_moved = false; // Attention loop runs even for equal false.
        c.formation.moves_remaining_raw = 513;
        assert_eq!(restore_formation_movement(&mut c), Ok(()));
        assert_eq!(c.before_update.get(), 513); // AsInt equals max: no initial write.
        assert_eq!(c.reports[0].value, 512); // raw inequality still sends report.
        let mut c = Form::new();
        c.fail_report = true;
        assert_eq!(
            restore_formation_movement(&mut c),
            Err(FormationUpkeepError::ContinuationFailed(
                "formation report unsupported"
            ))
        );
        assert_eq!(c.formation.moves_remaining_raw, 512);
        assert!(!c.unit.b_needs_attention);
        assert_eq!(c.reports.len(), 1);
        assert!(c.formation.has_moved_into_zoc);
    }
    struct Outer {
        actors: Vec<Context>,
        form: Form,
        units_count: std::rc::Rc<std::cell::Cell<usize>>,
        formations_count: std::rc::Rc<std::cell::Cell<usize>>,
        log: Vec<&'static str>,
        found: Vec<u32>,
        null_unit: bool,
        null_formation: bool,
    }
    impl Outer {
        fn new() -> Self {
            let units_count = std::rc::Rc::new(std::cell::Cell::new(1));
            let formations_count = std::rc::Rc::new(std::cell::Cell::new(1));
            let mut actors = vec![context(), context()];
            actors[0].unit.id = 42;
            actors[1].unit.id = 7;
            actors[0].units_count = Some(units_count.clone());
            let mut form = Form::new();
            form.formation_count = Some(formations_count.clone());
            Self {
                actors,
                form,
                units_count,
                formations_count,
                log: vec![],
                found: vec![],
                null_unit: false,
                null_formation: false,
            }
        }
    }
    impl ActivationSuffixContext for Outer {
        type Error = &'static str;
        type Units = ();
        type UnitCursor = usize;
        type UnitHandle = usize;
        type Formations = ();
        type FormationCursor = usize;
        type FormationHandle = ();
        type Actor = Context;
        type Formation = Form;
        fn player_units(&mut self) -> Result<(), Self::Error> {
            self.log.push("units");
            Ok(())
        }
        fn units_begin(&mut self, _: ()) -> Result<usize, Self::Error> {
            self.log.push("unit_begin");
            Ok(0)
        }
        fn units_end(&mut self, _: ()) -> Result<usize, Self::Error> {
            self.log.push("unit_end");
            Ok(self.units_count.get())
        }
        fn unit_cursors_equal(&mut self, a: &usize, b: &usize) -> Result<bool, Self::Error> {
            Ok(a == b)
        }
        fn unit_cursor_id(&mut self, cursor: &usize) -> Result<u32, Self::Error> {
            self.actors
                .get(*cursor)
                .map(|c| c.unit.id)
                .ok_or("invalid unit cursor")
        }
        fn find_activation_unit(&mut self, _: (), id: u32) -> Result<Option<usize>, Self::Error> {
            self.found.push(id);
            Ok(if self.null_unit {
                None
            } else {
                self.actors.iter().position(|c| c.unit.id == id)
            })
        }
        fn actor(&mut self, handle: Option<usize>) -> Result<&mut Context, Self::Error> {
            Ok(&mut self.actors[handle.unwrap_or(0)])
        }
        fn increment_unit_cursor(&mut self, c: &mut usize) -> Result<(), Self::Error> {
            self.log.push("unit_increment");
            *c += 1;
            Ok(())
        }
        fn player_formations(&mut self) -> Result<(), Self::Error> {
            self.log.push("formations");
            Ok(())
        }
        fn formations_end(&mut self, _: ()) -> Result<usize, Self::Error> {
            self.log.push("formation_end");
            Ok(self.formations_count.get())
        }
        fn formations_begin(&mut self, _: ()) -> Result<usize, Self::Error> {
            self.log.push("formation_begin");
            Ok(0)
        }
        fn formation_cursors_equal(&mut self, a: &usize, b: &usize) -> Result<bool, Self::Error> {
            Ok(a == b)
        }
        fn formation_at_cursor(&mut self, c: &usize) -> Result<Option<()>, Self::Error> {
            assert_eq!(
                *c, 0,
                "cached formation end must prevent newly appended visit"
            );
            Ok((!self.null_formation).then_some(()))
        }
        fn formation(&mut self, _: ()) -> Result<&mut Form, Self::Error> {
            Ok(&mut self.form)
        }
        fn increment_formation_cursor(&mut self, c: &mut usize) -> Result<(), Self::Error> {
            self.log.push("formation_increment");
            *c += 1;
            Ok(())
        }
    }
    #[test]
    fn suffix_refreshes_unit_end_but_caches_formation_end_before_begin() {
        let mut c = Outer::new();
        assert_eq!(activate_units_and_formations_suffix(&mut c, true), Ok(()));
        assert_eq!(c.found, [42, 7]);
        assert!(c.actors.iter().all(|a| !a.unit.b_processed_turn));
        assert_eq!(
            c.log,
            [
                "units",
                "unit_begin",
                "unit_end",
                "unit_increment",
                "unit_end",
                "unit_increment",
                "unit_end",
                "formations",
                "formation_end",
                "formation_begin",
                "formation_increment"
            ]
        );
        assert_eq!(c.formations_count.get(), 2);
    }
    #[test]
    fn false_suffix_and_null_find_preserve_exact_access_and_partial_effects() {
        let mut c = Outer::new();
        assert_eq!(activate_units_and_formations_suffix(&mut c, false), Ok(()));
        assert!(c.log.is_empty());
        assert!(c.actors[0].unit.b_processed_turn);
        c.null_unit = true;
        assert_eq!(
            activate_units_and_formations_suffix(&mut c, true),
            Err(ActivationSuffixError::Actor(UnitTurnStartError::NullUnit))
        );
        assert_eq!(c.actors[0].called, ["unit_manager", "unit_manager"]);
        assert_eq!(c.actors[0].unit.x_moves_remaining, 0);
        assert_eq!(c.actors[0].unit.i_operation_timer, 8);
        let mut c = Outer::new();
        c.null_formation = true;
        assert_eq!(
            activate_units_and_formations_suffix(&mut c, true),
            Err(ActivationSuffixError::NullFormation)
        );
        assert!(c.actors.iter().all(|a| !a.unit.b_processed_turn));
    }
    #[test]
    fn promotion_computes_cached_flags_find_create_and_fresh_publish_owner() {
        let mut c = context();
        c.human = true;
        c.unit.i_operation_timer = 0;
        c.created = Some(3);
        assert_eq!(check_promote_notification(&mut c), Ok(()));
        assert_eq!(
            c.called,
            [
                "promotion",
                "notification_manager",
                "owner",
                "find",
                "notification_manager",
                "owner",
                "create",
                "add",
                "notification_manager",
                "owner",
                "publish"
            ]
        );
        assert!(!c.unit.k_experience.b_can_promote_notify_needed);
        assert_eq!(c.added.unwrap().owner, 0);
        assert_eq!(c.unit.e_owner, 8);
        let mut c = context();
        c.human = true;
        c.unit.i_operation_timer = 0;
        c.existing = Some(4);
        assert_eq!(check_promote_notification(&mut c), Ok(()));
        assert!(c.called.contains(&"add"));
        assert!(!c.called.contains(&"create"));
        assert!(!c.called.contains(&"publish"));
        let mut c = context();
        c.human = true;
        c.unit.i_operation_timer = 0;
        assert_eq!(check_promote_notification(&mut c), Ok(()));
        assert!(c.called.contains(&"create"));
        assert!(!c.called.contains(&"add"));
        let mut c = context();
        c.unit.k_experience.b_can_promote = false;
        assert_eq!(check_promote_notification(&mut c), Ok(()));
        assert!(c.unit.k_experience.b_can_promote_notify_needed);
        assert!(c.called.is_empty());
    }
    #[test]
    fn processor_flags_capture_and_clear_only_requested_activation() {
        let mut flags = ProcessorActivationFlags {
            needs_first_turn_activate: true,
            needs_turn_activate_event: false,
        };
        assert_eq!(take_processor_activation_request(&mut flags), None);
        assert!(flags.needs_first_turn_activate);
        flags.needs_turn_activate_event = true;
        assert_eq!(take_processor_activation_request(&mut flags), Some(true));
        assert_eq!(
            flags,
            ProcessorActivationFlags {
                needs_first_turn_activate: false,
                needs_turn_activate_event: false
            }
        );
        flags.needs_turn_activate_event = true;
        assert_eq!(take_processor_activation_request(&mut flags), Some(false));
        assert_eq!(take_processor_activation_request(&mut flags), None);
    }
    #[test]
    fn formation_empty_maximum_sentinel_and_no_old_zoc_skip_second_report() {
        let mut c = Form::new();
        c.formation.unit_ids.clear();
        c.formation.has_moved_into_zoc = false;
        assert_eq!(restore_formation_movement(&mut c), Ok(()));
        assert_eq!(c.before_update.get(), 9001_i32 << 8);
        assert_eq!(c.formation.moves_remaining_raw, 9001_i32 << 8);
        assert!(c.unit.b_needs_attention); // No member was visited.
        assert_eq!(c.reports.len(), 1); // Subscriber sets ZOC, but cached old value was false.
    }
}
