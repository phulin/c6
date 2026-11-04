//! Mandatory Player.DoAutomation tail after current unit iteration (2b5b62a).
//! No native policy is run here; reached callbacks remain required services.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pending {
    SessionStatements,
    OpenSession,
    Action,
    Deal,
    AiSessionStatements,
}
pub trait Context {
    type Error;
    fn pending(&mut self, owner: i32, query: Pending) -> Result<bool, Self::Error>;
    fn any_multiplayer(&mut self) -> Result<bool, Self::Error>;
    fn network_multiplayer(&mut self) -> Result<bool, Self::Error>;
    fn reactivation_requested(&mut self, owner: i32) -> Result<bool, Self::Error>;
    fn request_unready(&mut self, owner: i32, reason: u32) -> Result<bool, Self::Error>;
    fn clear_complete(&mut self, owner: i32) -> Result<(), Self::Error>;
    fn set_active(&mut self, owner: i32) -> Result<(), Self::Error>;
    fn complete(&mut self, owner: i32) -> Result<bool, Self::Error>;
    fn mark_processed(&mut self, owner: i32) -> Result<(), Self::Error>;
}
/// Cached human/attention are those retained by this SAME unit pass. Completion
/// is reread after RequestUnready/SetTurnActive callbacks before the final store.
pub fn finish<C: Context>(
    c: &mut C,
    owner: i32,
    human: bool,
    attention: bool,
) -> Result<(), C::Error> {
    let pending = c.pending(owner, Pending::SessionStatements)?
        || c.pending(owner, Pending::OpenSession)?
        || c.pending(owner, Pending::SessionStatements)?
        || c.pending(owner, Pending::Action)?
        || c.pending(owner, Pending::Deal)?
        || c.pending(owner, Pending::AiSessionStatements)?;
    let mut reason = 0xb6fa1284;
    let mut force = false;
    if pending {
        let multi = c.any_multiplayer()?;
        force = !multi;
        if !multi {
            reason = 0xfc731e25;
        }
    }
    if !c.network_multiplayer()? && c.reactivation_requested(owner)? {
        force = true;
        if reason == 0xb6fa1284 {
            reason = 0xd60b7e54;
        }
    }
    if (human && attention) || force {
        if human && attention && reason == 0xb6fa1284 {
            reason = 0x05006dda;
        }
        if c.request_unready(owner, reason)? {
            c.clear_complete(owner)?;
            c.set_active(owner)?;
        }
    }
    // Native queries completion even for AI before OR with cached !human.
    if c.complete(owner)? || !human {
        c.mark_processed(owner)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    struct C {
        log: Vec<String>,
        pending: bool,
        network: bool,
        attention_complete: bool,
        fail_active: bool,
    }
    impl Context for C {
        type Error = ();
        fn pending(&mut self, _: i32, q: Pending) -> Result<bool, ()> {
            self.log.push(format!("{q:?}"));
            Ok(self.pending)
        }
        fn any_multiplayer(&mut self) -> Result<bool, ()> {
            self.log.push("multi".into());
            Ok(false)
        }
        fn network_multiplayer(&mut self) -> Result<bool, ()> {
            self.log.push("network".into());
            Ok(self.network)
        }
        fn reactivation_requested(&mut self, _: i32) -> Result<bool, ()> {
            self.log.push("reactivate".into());
            Ok(false)
        }
        fn request_unready(&mut self, _: i32, r: u32) -> Result<bool, ()> {
            self.log.push(format!("unready:{r:x}"));
            Ok(true)
        }
        fn clear_complete(&mut self, _: i32) -> Result<(), ()> {
            self.log.push("clear".into());
            self.attention_complete = false;
            Ok(())
        }
        fn set_active(&mut self, _: i32) -> Result<(), ()> {
            self.log.push("active".into());
            if self.fail_active {
                Err(())
            } else {
                Ok(())
            }
        }
        fn complete(&mut self, _: i32) -> Result<bool, ()> {
            self.log.push("complete".into());
            Ok(self.attention_complete)
        }
        fn mark_processed(&mut self, _: i32) -> Result<(), ()> {
            self.log.push("processed".into());
            Ok(())
        }
    }
    #[test]
    fn no_pending_preserves_repeated_session_query_and_network_short_circuit() {
        let mut c = C {
            log: vec![],
            pending: false,
            network: true,
            attention_complete: true,
            fail_active: false,
        };
        finish(&mut c, 0, true, false).unwrap();
        assert_eq!(
            c.log,
            [
                "SessionStatements",
                "OpenSession",
                "SessionStatements",
                "Action",
                "Deal",
                "AiSessionStatements",
                "network",
                "complete",
                "processed"
            ]
        );
    }
    #[test]
    fn pending_stops_later_queries_and_active_failure_keeps_cleared_flag() {
        let mut c = C {
            log: vec![],
            pending: true,
            network: false,
            attention_complete: true,
            fail_active: true,
        };
        assert_eq!(finish(&mut c, 0, true, false), Err(()));
        assert!(!c.attention_complete);
        assert_eq!(
            c.log,
            [
                "SessionStatements",
                "multi",
                "network",
                "reactivate",
                "unready:fc731e25",
                "clear",
                "active"
            ]
        );
    }
    #[test]
    fn attention_reason_and_current_completion_are_not_cached() {
        let mut c = C {
            log: vec![],
            pending: false,
            network: true,
            attention_complete: true,
            fail_active: false,
        };
        finish(&mut c, 0, true, true).unwrap();
        assert_eq!(
            &c.log[7..],
            ["unready:5006dda", "clear", "active", "complete"]
        );
    }
}
