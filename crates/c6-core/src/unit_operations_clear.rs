//! Linux XP2 Operations::Manager::Clear and inherited handler deactivation.
//!
//! Clear's loop is shared orchestration over authoritative operation providers.
//! PopFront/DeleteNode and retained node identity remain required providers;
//! these functions do not approximate them by removing the current Vec front.
//! Native differential validation and complete turn integration remain open.

use crate::unit::{ComponentId, Unit};

pub const OPERATION_DEACTIVATED: u32 = 0xa60328c5;
pub const ACTIVITY_OPERATION: i32 = 0x490cc8e1;
pub const ACTIVITY_AWAKE: i32 = 0x6b25012e;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationDeactivatedReport {
    pub event_hash: u32,
    pub identity: ComponentId,
    pub operation_hash: u32,
    pub ready_to_select: bool,
}

/// Source-bound BaseHandler::Deactivate providers. Heal's actual +0x68 vtable
/// slot inherits this handler. Other handlers require their own vtable proof.
pub trait BaseDeactivationContext {
    type Error;
    fn unit(&self) -> Result<&Unit, Self::Error>;
    fn local_player(&self) -> Result<i32, Self::Error>;
    /// Full synchronous Event::Send(Type, ComponentID, int hash, int ready).
    /// Preserve every subscriber mutation/RNG draw before returning.
    fn send_operation_deactivated(
        &mut self,
        report: OperationDeactivatedReport,
    ) -> Result<(), Self::Error>;
}

/// BaseHandler::Deactivate (0x2d964c0), including IsReadyToSelect (0x2d96520).
/// `operation_hash` is the selected live handler's GetHash result, not a queue
/// numeric ID or a freshly reselected front after a synchronous callback.
/// Returns native false after report delivery, causing PopFront's DeleteNode.
pub fn deactivate_base_handler<C: BaseDeactivationContext>(
    context: &mut C,
    operation_hash: u32,
) -> Result<bool, C::Error> {
    let actor = context.unit()?;
    let identity = ComponentId {
        owner: actor.e_owner as i16,
        component_type: 1,
        id: actor.id,
    };
    let owner = actor.e_owner;
    let local = context.local_player()?;
    let actor = context.unit()?;
    let ready_to_select = owner == local
        && actor.e_automation == -1
        && crate::skip_turn::legality::can_move(actor)
        && actor.k_operations.nodes.len() <= 1
        && [ACTIVITY_OPERATION, ACTIVITY_AWAKE].contains(&actor.e_activity);
    context.send_operation_deactivated(OperationDeactivatedReport {
        event_hash: OPERATION_DEACTIVATED,
        identity,
        operation_hash,
        ready_to_select,
    })?;
    Ok(false)
}

pub trait ClearOperationsContext {
    type Error;
    /// Re-resolve the retained actor after every synchronous continuation.
    /// A removed actor is Err; a detached snapshot must not be revived.
    fn unit(&self) -> Result<&Unit, Self::Error>;
    /// Manager::Deactivate, not the handler's Deactivate: if nonempty, change
    /// OPERATION activity to AWAKE synchronously, then set current timer to 0.
    fn deactivate_operations(&mut self) -> Result<(), Self::Error>;
    /// Complete Manager::PopFront for the current front. Retain the ORIGINAL
    /// node's logical identity across handler/report callbacks. DeleteNode
    /// deactivates only if that original node is still front, removes that
    /// original node, clears current formation if empty, and PopFront sends
    /// the empty-queue report after a further current-queue read.
    ///
    /// Insertion before the retained node must not remove the inserted front.
    /// Missing/replaced original nodes require an explicit identity-resolution
    /// error unless their actual native identity behavior has been established.
    /// Vec::remove(0) after callbacks is not this provider's implementation.
    fn pop_front_operation(&mut self) -> Result<(), Self::Error>;
    /// Resolve CURRENT EditFormation; if present perform ClearPathCache.
    /// A source-qualified absent formation is a successful no-op; unknown
    /// formation state must return Err. This call is required even if PopFront
    /// already cleared the same path when it removed the last operation.
    fn clear_current_formation_path(&mut self) -> Result<(), Self::Error>;
}

/// Manager::Clear (0x2da2710): empty entry preserves timer/path; nonempty entry
/// deactivates once, pops repeatedly using CURRENT queue state, and finally
/// resolves/clears current formation. No fixed queue length or iteration cap
/// replaces the original loop. Subscriber insertions remain visible.
///
/// The caller must admit all required providers before invoking this function.
/// Provider errors retain completed state/callback effects; this is not a
/// transaction and does not convert missing inputs into native success.
pub fn clear_operations<C: ClearOperationsContext>(context: &mut C) -> Result<(), C::Error> {
    if context.unit()?.k_operations.nodes.is_empty() {
        return Ok(());
    }
    context.deactivate_operations()?;
    while !context.unit()?.k_operations.nodes.is_empty() {
        context.pop_front_operation()?;
    }
    context.clear_current_formation_path()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Failure {
        Removed,
    }

    struct Context {
        actor: Unit,
        local: i32,
        report: Option<OperationDeactivatedReport>,
        log: Vec<&'static str>,
        append_once: bool,
        remove_after_pop: bool,
        removed: bool,
    }

    fn node() -> crate::unit::OperationNode {
        crate::unit::OperationNode {
            operation_hash: crate::unit_healing::HEAL_OPERATION_HASH,
            directive: 0,
            flags: 0,
            push_turn: 0,
            previous_activity: 0,
            previous_automation: 0,
            parameters: crate::unit::OperationParameters::None,
        }
    }

    impl Context {
        fn new(count: usize) -> Self {
            let mut actor = crate::unit_turn_upkeep::tests::source_only_unit();
            actor.id = 17;
            actor.e_owner = 2;
            actor.e_automation = -1;
            actor.e_activity = ACTIVITY_OPERATION;
            actor.x_moves_remaining = 1;
            actor.i_operation_timer = 8;
            actor.k_operations.nodes = vec![node(); count].into();
            Self {
                actor,
                local: 2,
                report: None,
                log: Vec::new(),
                append_once: false,
                remove_after_pop: false,
                removed: false,
            }
        }
    }

    impl BaseDeactivationContext for Context {
        type Error = Failure;
        fn unit(&self) -> Result<&Unit, Failure> {
            if self.removed {
                Err(Failure::Removed)
            } else {
                Ok(&self.actor)
            }
        }
        fn local_player(&self) -> Result<i32, Failure> {
            Ok(self.local)
        }
        fn send_operation_deactivated(
            &mut self,
            report: OperationDeactivatedReport,
        ) -> Result<(), Failure> {
            self.report = Some(report);
            self.actor.i_damage = 47;
            Ok(())
        }
    }

    impl ClearOperationsContext for Context {
        type Error = Failure;
        fn unit(&self) -> Result<&Unit, Failure> {
            BaseDeactivationContext::unit(self)
        }
        fn deactivate_operations(&mut self) -> Result<(), Failure> {
            self.log.push("manager_deactivate");
            self.actor.e_activity = ACTIVITY_AWAKE;
            self.actor.i_operation_timer = 0;
            Ok(())
        }
        fn pop_front_operation(&mut self) -> Result<(), Failure> {
            // Orchestration fixture: no synchronous callbacks before this
            // removal. This is not a production PopFront/DeleteNode provider.
            self.log.push("pop");
            self.actor.k_operations.nodes.remove(0);
            if self.append_once {
                self.actor.k_operations.nodes.push(node());
                self.append_once = false;
            }
            self.removed = self.remove_after_pop;
            Ok(())
        }
        fn clear_current_formation_path(&mut self) -> Result<(), Failure> {
            self.log.push("current_formation_clear");
            Ok(())
        }
    }

    #[test]
    fn base_deactivation_computes_readiness_and_delivers_report_before_false() {
        let mut c = Context::new(1);
        assert_eq!(deactivate_base_handler(&mut c, 0x7eb896fb), Ok(false));
        assert_eq!(
            c.report,
            Some(OperationDeactivatedReport {
                event_hash: OPERATION_DEACTIVATED,
                identity: ComponentId {
                    owner: 2,
                    component_type: 1,
                    id: 17
                },
                operation_hash: 0x7eb896fb,
                ready_to_select: true,
            })
        );
        assert_eq!(c.actor.i_damage, 47);
        for variation in 0..5 {
            let mut c = Context::new(1);
            match variation {
                0 => c.local = 3,
                1 => c.actor.e_automation = 0,
                2 => c.actor.x_moves_remaining = 0,
                3 => c.actor.k_operations.nodes.push(node()),
                _ => c.actor.e_activity = crate::skip_turn::HOLD,
            }
            deactivate_base_handler(&mut c, 0x7eb896fb).unwrap();
            assert!(!c.report.unwrap().ready_to_select);
        }
    }

    #[test]
    fn empty_clear_preserves_timer_and_path_and_nonempty_rereads_queue() {
        let mut c = Context::new(0);
        clear_operations(&mut c).unwrap();
        assert!(c.log.is_empty());
        assert_eq!(c.actor.i_operation_timer, 8);
        let mut c = Context::new(2);
        c.append_once = true;
        clear_operations(&mut c).unwrap();
        assert_eq!(
            c.log,
            [
                "manager_deactivate",
                "pop",
                "pop",
                "pop",
                "current_formation_clear"
            ]
        );
        assert!(c.actor.k_operations.nodes.is_empty());
        assert_eq!(c.actor.i_operation_timer, 0);
    }

    #[test]
    fn removed_actor_stops_before_path_without_copyback() {
        let mut c = Context::new(1);
        c.remove_after_pop = true;
        assert_eq!(clear_operations(&mut c), Err(Failure::Removed));
        assert_eq!(c.log, ["manager_deactivate", "pop"]);
        assert!(c.removed);
    }
}
