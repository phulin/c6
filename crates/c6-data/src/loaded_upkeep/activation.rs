//! Stock Linux manager semantics composed on one retained loaded Unit.
use super::*;
use c6_core::{
    unit::{ComponentId, FORTIFY_HASH, MAKE_TRADE_ROUTE_HASH, MOVE_TO_HASH, SKIP_TURN_HASH},
    unit_operations_clear::ClearOperationsContext,
    unit_turn_start::{
        OperationsTurnStartContext, PromotionNotificationContext, UnitTurnStartContext,
    },
};

struct Activation<'s, 'i, 'd, 'r, E> {
    movement: Movement<'s, 'i, 'd, E>,
    definitions: &'r crate::formation_rules::Definitions,
}
impl<E: Effects> RestoreMovementContext for Activation<'_, '_, '_, '_, E> {
    type Error = Error<E::Error>;
    fn unit(&self) -> Result<&Unit, Self::Error> {
        self.movement.unit()
    }
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
        self.movement.unit_mut()
    }
    fn max_moves(&self) -> Result<i32, Self::Error> {
        self.movement.max_moves()
    }
    fn preflight_restore_movement(&self, branch: bool) -> Result<(), Self::Error> {
        self.movement.preflight_restore_movement(branch)
    }
    fn send_movement_report(&mut self, report: MovementReport) -> Result<(), Self::Error> {
        self.movement.send_movement_report(report)
    }
    fn sync_current_formation(&mut self) -> Result<(), Self::Error> {
        self.movement.sync_current_formation()
    }
    fn owner_is_barbarian(&self) -> Result<bool, Self::Error> {
        self.movement.owner_is_barbarian()
    }
}
impl<E> RestoreAttacksContext for Activation<'_, '_, '_, '_, E>
where
    E: Effects + AttacksReports<Error = <E as Effects>::Error>,
{
    type Error = Error<<E as Effects>::Error>;
    fn unit(&self) -> Result<&Unit, Self::Error> {
        self.movement.unit()
    }
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
        self.movement.unit_mut()
    }
    fn combat_max_num_attacks(&self) -> Result<i32, Self::Error> {
        self.definitions
            .combat_max_num_attacks(self.movement.state)
            .map_err(Error::Rules)
    }
    fn preflight_attacks_report(&self) -> Result<(), Self::Error> {
        self.movement
            .effects
            .preflight_attacks_report(self.movement.state, self.movement.actor)
            .map_err(Error::Provider)
    }
    fn send_attacks_report(&mut self, report: AttacksRemainingReport) -> Result<(), Self::Error> {
        self.movement
            .effects
            .attacks_report(self.movement.state, report)
            .map_err(Error::Provider)
    }
}
impl<E> ClearOperationsContext for Activation<'_, '_, '_, '_, E>
where
    E: Effects + OperationReports<Error = <E as Effects>::Error>,
{
    type Error = Error<<E as Effects>::Error>;
    fn unit(&self) -> Result<&Unit, Self::Error> {
        self.movement.unit()
    }
    fn deactivate_operations(&mut self) -> Result<(), Self::Error> {
        Err(Error::Unsupported(
            "activation does not dispatch Manager::Clear",
        ))
    }
    fn pop_front_operation(&mut self) -> Result<(), Self::Error> {
        operations::pop_front_operation(
            self.movement.state,
            self.movement.actor.owner,
            self.movement.actor.id,
            self.movement.effects,
        )
    }
    fn clear_current_formation_path(&mut self) -> Result<(), Self::Error> {
        Err(Error::Unsupported(
            "activation does not dispatch Manager::Clear",
        ))
    }
}
impl<E> OperationsTurnStartContext for Activation<'_, '_, '_, '_, E>
where
    E: Effects
        + AlertVisibility<Error = <E as Effects>::Error>
        + OperationReports<Error = <E as Effects>::Error>,
{
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
        self.movement.unit_mut()
    }
    fn front_handler_turn_start(&mut self, hash: u32) -> Result<bool, Self::Error> {
        match hash {
            // These stock handlers inherit the complete constant-false leaf.
            // No queue mutation, policy selection, report or RNG is performed.
            SKIP_TURN_HASH | FORTIFY_HASH | MOVE_TO_HASH | MAKE_TRADE_ROUTE_HASH | c6_core::unit::RANGE_ATTACK_HASH | c6_core::unit::PILLAGE_HASH => Ok(false),
            c6_core::unit::ALERT_HASH => {
                let actor = self.movement.unit()?;
                if !alert_enemy_visible(
                    self.movement.state,
                    actor,
                    self.definitions,
                    self.movement.effects,
                )? {
                    Ok(false)
                } else {
                    operations::set_activity(
                        self.movement.state,
                        self.movement.actor,
                        self.movement.effects,
                        c6_core::unit_operations_clear::ACTIVITY_AWAKE,
                    )?;
                    Ok(true)
                }
            }
            _ => Err(Error::Unsupported(
                "queued handler TurnStartCheck continuation",
            )),
        }
    }
}
impl<E: Effects> PromotionNotificationContext for Activation<'_, '_, '_, '_, E> {
    type Notification = ();
    type NotificationManager = ();
    fn promotion_owner_is_human(&mut self) -> Result<bool, Self::Error> {
        let owner = self.movement.unit()?.e_owner;
        self.movement
            .state
            .player_slot_status(owner)
            .map(|status| status == 3)
            .map_err(Error::State)
    }
    fn promotion_owner(&mut self) -> Result<i32, Self::Error> {
        Ok(self.movement.unit()?.e_owner)
    }
    fn notification_manager(&mut self) -> Result<(), Self::Error> {
        Err(Error::Unsupported(
            "promotion notification Find/Create/Add/Publish continuation",
        ))
    }
    fn find_promote_notification(&mut self, _: (), _: i32) -> Result<Option<()>, Self::Error> {
        Err(Error::Unsupported(
            "promotion notification Find continuation",
        ))
    }
    fn create_promote_notification(&mut self, _: (), _: i32) -> Result<Option<()>, Self::Error> {
        Err(Error::Unsupported(
            "promotion notification Create continuation",
        ))
    }
    fn add_promotable_unit(&mut self, _: (), _: ComponentId) -> Result<(), Self::Error> {
        Err(Error::Unsupported(
            "promotion notification Add continuation",
        ))
    }
    fn publish_promote_notification(&mut self, _: (), _: i32, _: ()) -> Result<(), Self::Error> {
        Err(Error::Unsupported(
            "promotion notification Publish continuation",
        ))
    }
}
impl<E> UnitTurnStartContext for Activation<'_, '_, '_, '_, E>
where
    E: Effects
        + AttacksReports<Error = <E as Effects>::Error>
        + AlertVisibility<Error = <E as Effects>::Error>
        + OperationReports<Error = <E as Effects>::Error>,
{
    fn current_unit_manager(&mut self) -> Result<(), Error<<E as Effects>::Error>> {
        // XP2 registers the concrete stock factory. Native addresses and vptrs
        // do not carry gameplay state for these stateless manager operations.
        self.definitions
            .check(self.movement.state)
            .map_err(Error::Rules)
    }
    fn current_operations_manager(&mut self) -> Result<(), Error<<E as Effects>::Error>> {
        self.definitions
            .check(self.movement.state)
            .map_err(Error::Rules)
    }
}

/// Execute the supported stock per-Unit activation sequence on authoritative
/// records: movement, attacks, queued-handler turn-start, promotion notification
/// check, then processed-turn reset. Each callback reads/writes the same World.
///
/// Inherited constant-false queue handlers retain the entire current queue.
/// Other handler effects and reached human promotion notification effects return
/// explicit errors, preserving prior writes; no native policy chooses orders or
/// promotions. This finite source-derived suffix is not a complete player turn
/// or an atomic action, and has no native transition acceptance claim.
pub fn activate_unit<E>(
    state: &mut LoadedState,
    owner: i32,
    id: u32,
    inputs: &Inputs<'_>,
    definitions: &crate::formation_rules::Definitions,
    effects: &mut E,
) -> Result<(), unit_turn_start::UnitTurnStartError<Error<<E as Effects>::Error>>>
where
    E: Effects
        + AttacksReports<Error = <E as Effects>::Error>
        + AlertVisibility<Error = <E as Effects>::Error>
        + OperationReports<Error = <E as Effects>::Error>,
{
    unit_turn_start::activate_unit(&mut Activation {
        movement: Movement {
            state,
            inputs,
            effects,
            actor: UnitHandle { owner, id },
        },
        definitions,
    })
}
