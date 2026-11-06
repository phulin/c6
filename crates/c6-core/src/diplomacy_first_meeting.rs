//! Initial neutral FirstMeeting processing through the next external decision.
//!
//! Supports an AI sender, local human recipient and the fresh request session.
//! Canonical history/activation/retirement precede the mandatory report callback.
//! The callback owns all reached gameplay/RNG effects; failure retains that
//! completed prefix. Neither response execution nor full diplomacy is covered.
use crate::diplomacy_manager::{Manager, StatementHandle};
use crate::diplomacy_request::{Player, INITIAL_RESPONSE};
use crate::diplomacy_statement::{
    self as statement, Context, SessionHeader, SessionView, StatementInfo, StatementKind,
};

pub const REPORT_EVENT: u32 = 0xfc73_1e25;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Choice {
    /// AddResponse(session, recipient, POSITIVE); subsequent effects are separate.
    Positive,
    /// CloseSession(session), including its actual cleanup/queued continuation.
    Exit,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PendingDecision {
    pub session: SessionHeader,
    pub statement: StatementHandle,
    pub player: i32,
    pub choices: [Choice; 2],
}
/// Semantic projection of FirstMeeting.ToVariants after initial history append.
/// Constructor-bounded inherited fields remain unchanged. The report provider
/// must qualify actual consumers, not infer harmlessness from this projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Report {
    pub statement: StatementInfo,
    pub session: SessionHeader,
    pub first_of_type: bool,
    pub session_statement_index: u32,
}
#[derive(Debug)]
pub enum Error<E> {
    Unsupported(&'static str),
    Statement(statement::Error),
    /// Canonical session processing has committed; do not retry it as fresh work.
    Report(E),
}
#[derive(Debug)]
pub struct Plan {
    owner: u64,
    revision: u64,
    context: Context,
    incoming: StatementInfo,
    ownership: statement::Plan,
    report: Report,
    decision: PendingDecision,
}
impl Manager {
    /// Called at the post-outgoing-removal seam. Selections are the current
    /// loaded rules' ordered keys for AI/NONE/FIRST_MEET_NO_MANS, not defaults
    /// inferred from a saved game. The enclosing owner guards World revision
    /// and actual endpoint/control/rules facts jointly with this manager plan.
    pub fn plan_first_meeting_statement(
        &self,
        handle: StatementHandle,
        context: Context,
        from: Player,
        to: Player,
        selections: &[Choice],
    ) -> Result<Plan, Error<std::convert::Infallible>> {
        let fail = Error::Unsupported;
        let revision = self
            .drain_revision
            .ok_or(Error::Statement(statement::Error::Conflict))?;
        let inventory = self
            .statements
            .as_deref()
            .ok_or(fail("statement inventory"))?;
        let mut matching = inventory.iter().filter(|s| s.handle == handle);
        let incoming = *matching
            .next()
            .ok_or(fail("missing FirstMeeting statement"))?;
        if matching.next().is_some() {
            return Err(fail("duplicate statement identity"));
        }
        if from.owner != incoming.from
            || to.owner != incoming.to
            || !from.major
            || from.human
            || !to.major
            || !to.human
            || incoming.from == incoming.to
            || context.local_player != incoming.to
        {
            return Err(fail("FirstMeeting endpoint/control profile"));
        }
        match incoming.kind {
            StatementKind::FirstMeeting {
                type_hash,
                initiator,
                response_type,
                hospitality_city,
            } if type_hash == crate::diplomacy_meeting::NEUTRAL_MEETING
                && initiator == incoming.from
                && response_type == INITIAL_RESPONSE
                && hospitality_city.x == -9999
                && hospitality_city.y == -9999 => {}
            _ => return Err(fail("initial neutral FirstMeeting payload")),
        }
        if selections != [Choice::Positive, Choice::Exit] {
            return Err(fail("FirstMeeting current rules selections"));
        }
        let routing = self
            .routing
            .as_ref()
            .filter(|r| r.authoritative)
            .ok_or(fail("authoritative session routing"))?;
        if routing.active != context.active_session || context.active_session.is_some() {
            return Err(fail("FirstMeeting inactive fresh session"));
        }
        let mut sessions = routing
            .metadata
            .iter()
            .enumerate()
            .filter(|(_, m)| m.header.id == incoming.session_id);
        let (index, metadata) = sessions
            .next()
            .ok_or(fail("FirstMeeting request session"))?;
        if sessions.next().is_some() || routing.metadata.len() != self.open_sessions.len() {
            return Err(fail("FirstMeeting session inventory"));
        }
        let pending = &self.open_sessions[index];
        if metadata.header.from != incoming.from
            || metadata.header.to != incoming.to
            || !metadata.history.is_empty()
            || !metadata.queued.is_empty()
            || !pending.pending.is_empty()
        {
            return Err(fail("FirstMeeting fresh post-removal session"));
        }
        let ownership = statement::plan_first_meeting(
            context,
            incoming,
            &[SessionView {
                header: metadata.header,
                history: &metadata.history,
                pending: &pending.pending,
                queued: &metadata.queued,
            }],
            inventory,
        )
        .map_err(Error::Statement)?;
        Ok(Plan {
            owner: self.drain_owner,
            revision,
            context,
            incoming,
            ownership,
            report: Report {
                statement: incoming,
                session: metadata.header,
                // The reverse scan sees the statement just appended to history.
                first_of_type: false,
                session_statement_index: 1,
            },
            decision: PendingDecision {
                session: metadata.header,
                statement: handle,
                player: incoming.to,
                choices: [Choice::Positive, Choice::Exit],
            },
        })
    }
    /// Commit bookkeeping, execute the reached report and return the next
    /// semantic decision without choosing a response. Callers must stop their
    /// continuation at this boundary and retain this decision in owned state.
    /// The required callback is source-qualified by the enclosing runtime before
    /// this method; it may not silently omit unclosed gameplay/RNG consumers.
    pub fn apply_first_meeting_statement<E>(
        &mut self,
        plan: Plan,
        report: impl FnOnce(Report) -> Result<(), E>,
    ) -> Result<PendingDecision, Error<E>> {
        if self.drain_owner != plan.owner || self.drain_revision != Some(plan.revision) {
            return Err(Error::Statement(statement::Error::Conflict));
        }
        self.apply_statement(plan.ownership, plan.context, Some(plan.incoming))
            .map_err(Error::Statement)?;
        report(plan.report).map_err(Error::Report)?;
        Ok(plan.decision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diplomacy_manager::ObservedSession;
    use crate::diplomacy_statement::{Logging, SessionHandle, SINGLE_PLAYER_MODE};
    fn fixture() -> (Manager, Context, Player, Player) {
        let info = StatementInfo {
            handle: StatementHandle(3),
            session_id: 1,
            from: 1,
            to: 0,
            kind: StatementKind::FirstMeeting {
                type_hash: crate::diplomacy_meeting::NEUTRAL_MEETING,
                initiator: 1,
                response_type: INITIAL_RESPONSE,
                hospitality_city: crate::unit::PlotCoord { x: -9999, y: -9999 },
            },
        };
        let mut manager = Manager::from_observed_sessions(
            vec![],
            vec![],
            vec![ObservedSession {
                header: SessionHeader {
                    handle: SessionHandle(2),
                    id: 1,
                    from: 1,
                    to: 0,
                },
                history: vec![],
                pending: vec![],
                queued: vec![],
            }],
            None,
            4,
        )
        .unwrap();
        manager.initialize_statement_inventory(vec![info]).unwrap();
        (
            manager,
            Context {
                local_player: 0,
                game_mode: SINGLE_PLAYER_MODE,
                active_session: None,
                logging: Logging::OrdinaryHostDisabled,
            },
            Player {
                owner: 1,
                major: true,
                human: false,
            },
            Player {
                owner: 0,
                major: true,
                human: true,
            },
        )
    }
    #[test]
    fn commits_history_then_reports_and_yields_without_response() {
        let (mut manager, context, from, to) = fixture();
        let plan = manager
            .plan_first_meeting_statement(
                StatementHandle(3),
                context,
                from,
                to,
                &[Choice::Positive, Choice::Exit],
            )
            .unwrap();
        // Generic processing remains unsupported: subtype effects are mandatory.
        assert!(manager
            .plan_statement(
                context,
                Some(manager.statement_inventory().unwrap()[0]),
                &[]
            )
            .is_err());
        let mut reports = vec![];
        let decision = manager
            .apply_first_meeting_statement(plan, |r| {
                reports.push(r);
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(decision.player, 0);
        assert_eq!(decision.choices, [Choice::Positive, Choice::Exit]);
        assert_eq!(manager.active_session().unwrap(), Some(SessionHandle(2)));
        assert_eq!(
            manager.session_metadata().unwrap()[0].history,
            [StatementHandle(3)]
        );
        assert_eq!(manager.statement_inventory().unwrap().len(), 1);
        assert_eq!(reports.len(), 1);
        assert!(!reports[0].first_of_type);
        assert_eq!(reports[0].session_statement_index, 1);
        assert!(manager.open_sessions()[0].pending.is_empty());
    }
    #[test]
    fn report_failure_preserves_committed_prefix_without_returning_decision() {
        let (mut manager, context, from, to) = fixture();
        let plan = manager
            .plan_first_meeting_statement(
                StatementHandle(3),
                context,
                from,
                to,
                &[Choice::Positive, Choice::Exit],
            )
            .unwrap();
        assert!(matches!(
            manager.apply_first_meeting_statement(plan, |_| Err("subscriber")),
            Err(Error::Report("subscriber"))
        ));
        assert_eq!(
            manager.session_metadata().unwrap()[0].history,
            [StatementHandle(3)]
        );
        assert_eq!(manager.active_session().unwrap(), Some(SessionHandle(2)));
    }
    #[test]
    fn unsupported_rules_or_control_leave_state_untouched() {
        let (manager, context, from, mut to) = fixture();
        assert!(manager
            .plan_first_meeting_statement(StatementHandle(3), context, from, to, &[Choice::Exit])
            .is_err());
        to.human = false;
        assert!(manager
            .plan_first_meeting_statement(
                StatementHandle(3),
                context,
                from,
                to,
                &[Choice::Positive, Choice::Exit]
            )
            .is_err());
        assert!(manager.session_metadata().unwrap()[0].history.is_empty());
    }
}
