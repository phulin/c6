//! Bounded Linux delegation statement ownership, with external human choices.
//! Uses the canonical Manager lists and statement routing; no policy runs here.
use crate::{
    diplomacy_manager::{Manager, ObservedSession, SessionMetadata, StatementHandle},
    diplomacy_request::{make_id, INITIAL_RESPONSE},
    diplomacy_statement::{
        self as statement, Context, SessionHandle, SessionHeader, SessionView, StatementInfo,
        StatementKind,
    },
    unit_operation_aftermath::{Pending, Session},
};
pub const TYPE: u32 = 0xcb57_0780;
pub const NONE: u32 = 0xb6fa_1284;
pub const POSITIVE: u32 = 0x7332_f7d6;
pub const NEGATIVE: u32 = 0x1644_80cd;
pub const HUMAN_ACCEPT: u32 = 0xc20e_f7ff;
pub const HUMAN_REFUSE: u32 = 0x374a_9ab5;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Choice {
    Positive,
    Negative,
    Exit,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PendingDecision {
    pub session: SessionHeader,
    pub statement: StatementHandle,
    pub player: i32,
    pub choices: [Choice; 3],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Report {
    pub statement: StatementInfo,
    pub session: SessionHeader,
    pub first_of_type: bool,
    pub session_statement_index: u32,
}
#[derive(Debug)]
pub enum Error {
    Unsupported(&'static str),
    Statement(statement::Error),
    ResourceLimit,
}
fn need(ok: bool, reason: &'static str) -> Result<(), Error> {
    if ok {
        Ok(())
    } else {
        Err(Error::Unsupported(reason))
    }
}
fn initial(from: i32) -> StatementKind {
    StatementKind::Delegation {
        initiator: from,
        response_type: INITIAL_RESPONSE,
        subtype: NONE,
        action_pending: true,
    }
}
impl Manager {
    /// Delivered ProcessAction removes only the first matching pending entry.
    /// The message's authority flags do not change these gameplay endpoints.
    pub fn consume_delegation_action(&mut self, from: i32, to: i32) -> Result<(), Error> {
        let pending = self
            .pending_actions
            .as_mut()
            .ok_or(Error::Unsupported("delegation pending action inventory"))?;
        if let Some(index) = pending
            .iter()
            .position(|a| a.id == 0xc08d_3473 && a.from == from && a.to == to)
        {
            pending.remove(index);
        }
        self.invalidate_drain_plans();
        Ok(())
    }
    /// RequestSession after the caller validates current capability and player
    /// facts. Only the nonlocal AI sender/local human recipient route is admitted.
    /// Duplicate open sessions retire the new ordinary delegation object.
    pub fn request_delegation(
        &mut self,
        from: i32,
        to: i32,
        handle: StatementHandle,
        c: Context,
    ) -> Result<bool, Error> {
        need(
            c.game_mode == statement::SINGLE_PLAYER_MODE
                && c.local_player == to
                && from != to
                && c.logging.supports_diplomacy_output(),
            "delegation request execution profile",
        )?;
        let routing = self
            .routing
            .as_mut()
            .filter(|r| r.authoritative)
            .ok_or(Error::Unsupported("delegation routing"))?;
        let inventory = self
            .statements
            .as_mut()
            .ok_or(Error::Unsupported("delegation inventory"))?;
        need(
            !inventory.iter().any(|s| s.handle == handle),
            "fresh delegation identity",
        )?;
        let id = make_id(from, to);
        if routing.metadata.iter().any(|m| m.header.id == id) {
            self.invalidate_drain_plans();
            return Ok(false);
        }
        need(
            routing.active == c.active_session && routing.closed.is_some(),
            "delegation active/closed ownership",
        )?;
        let next = routing
            .next_object_label
            .checked_add(1)
            .ok_or(Error::ResourceLimit)?;
        let header = SessionHeader {
            handle: SessionHandle(routing.next_object_label),
            id,
            from,
            to,
        };
        self.open_sessions
            .try_reserve(1)
            .map_err(|_| Error::ResourceLimit)?;
        routing
            .metadata
            .try_reserve(1)
            .map_err(|_| Error::ResourceLimit)?;
        inventory.try_reserve(1).map_err(|_| Error::ResourceLimit)?;
        let mut pending = Vec::new();
        pending.try_reserve(1).map_err(|_| Error::ResourceLimit)?;
        pending.push(Pending {
            from,
            to,
            statement: handle,
        });
        self.open_sessions.push(Session { from, to, pending });
        routing.metadata.push(SessionMetadata {
            header,
            history: Vec::new(),
            queued: Vec::new(),
        });
        routing.next_object_label = next;
        inventory.push(StatementInfo {
            handle,
            session_id: id,
            from,
            to,
            kind: initial(from),
        });
        self.invalidate_drain_plans();
        Ok(true)
    }
    /// Called at the same post-outgoing-removal seam as FirstMeeting. Report
    /// delivery is mandatory after this committed ownership prefix.
    pub fn process_delegation_initial(
        &mut self,
        handle: StatementHandle,
        c: Context,
    ) -> Result<(Report, PendingDecision), Error> {
        let incoming = *self
            .statements
            .as_ref()
            .and_then(|v| v.iter().find(|s| s.handle == handle))
            .ok_or(Error::Unsupported("delegation statement"))?;
        need(
            incoming.kind == initial(incoming.from) && incoming.to == c.local_player,
            "initial delegation payload",
        )?;
        let metadata = self
            .session_metadata()
            .map_err(|_| Error::Unsupported("delegation session metadata"))?;
        let views: Vec<_> = self
            .open_sessions
            .iter()
            .zip(metadata)
            .map(|(s, m)| SessionView {
                header: m.header,
                history: &m.history,
                pending: &s.pending,
                queued: &m.queued,
            })
            .collect();
        let plan =
            statement::plan_first_meeting(c, incoming, &views, self.statements.as_deref().unwrap())
                .map_err(Error::Statement)?;
        let header = views
            .iter()
            .find(|s| s.header.id == incoming.session_id)
            .ok_or(Error::Unsupported("delegation request session"))?
            .header;
        need(
            views.len() == 1
                && views[0].history.is_empty()
                && views[0].pending.is_empty()
                && views[0].queued.is_empty()
                && c.active_session.is_none(),
            "fresh delegation initial session",
        )?;
        self.apply_statement(plan, c, Some(incoming))
            .map_err(Error::Statement)?;
        Ok((
            Report {
                statement: incoming,
                session: header,
                first_of_type: false,
                session_statement_index: 1,
            },
            PendingDecision {
                session: header,
                statement: handle,
                player: incoming.to,
                choices: [Choice::Positive, Choice::Negative, Choice::Exit],
            },
        ))
    }
    /// Validate before invoking any callback or changing logical identity.
    pub fn validate_delegation_choice(
        &self,
        decision: PendingDecision,
        c: Context,
    ) -> Result<(), Error> {
        let r = self
            .routing
            .as_ref()
            .filter(|r| r.authoritative)
            .ok_or(Error::Unsupported("delegation routing"))?;
        need(
            self.open_sessions.len() == 1
                && r.metadata.len() == 1
                && r.active == Some(decision.session.handle)
                && c.active_session == r.active
                && c.local_player == decision.player
                && c.game_mode == statement::SINGLE_PLAYER_MODE
                && c.logging.supports_diplomacy_output(),
            "delegation choice profile",
        )?;
        let m = &r.metadata[0];
        need(
            m.header == decision.session
                && m.history == [decision.statement]
                && m.queued.is_empty()
                && self.open_sessions[0].pending.is_empty()
                && r.closed.is_some(),
            "delegation decision lists",
        )?;
        need(
            decision.player == decision.session.to
                && decision.choices == [Choice::Positive, Choice::Negative, Choice::Exit]
                && self.statements.as_ref().is_some_and(|v| {
                    v.iter().any(|s| {
                        s.handle == decision.statement && s.kind == initial(decision.session.from)
                    })
                }),
            "delegation decision payload",
        )
    }
    /// Local MakeResponse/Add/broadcast/delivery, then ProposeAction's generated
    /// acknowledgment. The transient outgoing response is separately retired;
    /// its reconstructed delivered object stays in canonical history.
    pub fn respond_delegation(
        &mut self,
        decision: PendingDecision,
        choice: Choice,
        c: Context,
        handles: [StatementHandle; 3],
    ) -> Result<Option<Report>, Error> {
        self.validate_delegation_choice(decision, c)?;
        if choice == Choice::Exit {
            return Ok(None);
        }
        let inventory = self.statements.as_mut().unwrap();
        need(
            handles.iter().enumerate().all(|(i, h)| {
                !handles[..i].contains(h) && !inventory.iter().any(|s| s.handle == *h)
            }),
            "fresh delegation response identities",
        )?;
        inventory.try_reserve(2).map_err(|_| Error::ResourceLimit)?;
        let r = self.routing.as_mut().unwrap();
        let m = &mut r.metadata[0];
        m.history.try_reserve(2).map_err(|_| Error::ResourceLimit)?;
        let response = if choice == Choice::Positive {
            POSITIVE
        } else {
            NEGATIVE
        };
        let delivered = StatementInfo {
            handle: handles[1],
            session_id: m.header.id,
            from: decision.player,
            to: decision.session.from,
            kind: StatementKind::Delegation {
                initiator: decision.session.from,
                response_type: response,
                subtype: NONE,
                action_pending: false,
            },
        };
        inventory.push(delivered);
        m.history.push(delivered.handle);
        let followup = StatementInfo {
            handle: handles[2],
            session_id: m.header.id,
            from: decision.session.from,
            to: decision.player,
            kind: StatementKind::Delegation {
                initiator: decision.session.from,
                response_type: response,
                subtype: if choice == Choice::Positive {
                    HUMAN_ACCEPT
                } else {
                    HUMAN_REFUSE
                },
                action_pending: false,
            },
        };
        inventory.push(followup);
        m.history.push(followup.handle);
        let report = Report {
            statement: followup,
            session: m.header,
            first_of_type: false,
            session_statement_index: 3,
        };
        self.invalidate_drain_plans();
        Ok(Some(report))
    }
    /// Exact synchronized close for this conversation. Its report is delivered
    /// by the caller after ownership moves; Manager.Process sees no open work.
    pub fn close_delegation(&mut self, header: SessionHeader) -> Result<(), Error> {
        let r = self
            .routing
            .as_mut()
            .filter(|r| r.authoritative)
            .ok_or(Error::Unsupported("delegation routing"))?;
        need(
            self.open_sessions.len() == 1
                && r.metadata.len() == 1
                && r.metadata[0].header == header
                && r.active == Some(header.handle)
                && self.open_sessions[0].pending.is_empty()
                && r.metadata[0].queued.is_empty(),
            "delegation close lists",
        )?;
        let closed = r
            .closed
            .as_mut()
            .ok_or(Error::Unsupported("delegation closed list"))?;
        closed.try_reserve(1).map_err(|_| Error::ResourceLimit)?;
        let m = r.metadata.remove(0);
        let s = self.open_sessions.remove(0);
        closed.push(ObservedSession {
            header: m.header,
            history: m.history,
            pending: s.pending,
            queued: m.queued,
        });
        r.active = None;
        self.invalidate_drain_plans();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fresh() -> (Manager, Context) {
        let mut m = Manager::from_observed_sessions(vec![], vec![], vec![], None, 1).unwrap();
        m.initialize_statement_inventory(vec![]).unwrap();
        m.initialize_first_meeting_empty_closed_sessions(0).unwrap();
        let c = Context {
            local_player: 0,
            game_mode: statement::SINGLE_PLAYER_MODE,
            active_session: None,
            logging: statement::Logging::OrdinaryHostDisabled,
        };
        m.initialize_execution_profile(c.game_mode, c.logging)
            .unwrap();
        (m, c)
    }
    fn requested() -> (Manager, Context, PendingDecision) {
        let (mut m, mut c) = fresh();
        assert!(m.request_delegation(1, 0, StatementHandle(1), c).unwrap());
        assert_eq!(m.open_sessions[0].pending.len(), 1);
        // Exact post-outgoing-removal seam supplied by ProcessPending.
        m.open_sessions[0].pending.remove(0);
        let (report, d) = m.process_delegation_initial(StatementHandle(1), c).unwrap();
        assert_eq!(report.statement.kind, initial(1));
        c.active_session = m.active_session().unwrap();
        (m, c, d)
    }
    #[test]
    fn both_responses_preserve_three_history_objects_and_distinct_acknowledgments() {
        for (choice, response, subtype) in [
            (Choice::Positive, POSITIVE, HUMAN_ACCEPT),
            (Choice::Negative, NEGATIVE, HUMAN_REFUSE),
        ] {
            let (mut m, c, d) = requested();
            let report = m
                .respond_delegation(
                    d,
                    choice,
                    c,
                    [StatementHandle(2), StatementHandle(3), StatementHandle(4)],
                )
                .unwrap()
                .unwrap();
            assert_eq!(
                report.statement.kind,
                StatementKind::Delegation {
                    initiator: 1,
                    response_type: response,
                    subtype,
                    action_pending: false
                }
            );
            assert_eq!(m.statement_inventory().unwrap().len(), 3);
            assert!(!m
                .statement_inventory()
                .unwrap()
                .iter()
                .any(|s| s.handle == StatementHandle(2)));
            assert_eq!(
                m.session_metadata().unwrap()[0].history,
                [StatementHandle(1), StatementHandle(3), StatementHandle(4)]
            );
            m.close_delegation(d.session).unwrap();
            assert_eq!(m.active_session().unwrap(), None);
            assert_eq!(
                m.first_meeting_closed_sessions().unwrap()[0].history.len(),
                3
            );
        }
    }
    #[test]
    fn exit_closes_without_response_and_duplicate_request_keeps_original() {
        let (mut m, c, d) = requested();
        assert!(!m.request_delegation(1, 0, StatementHandle(9), c).unwrap());
        assert_eq!(m.statement_inventory().unwrap().len(), 1);
        assert!(m
            .respond_delegation(d, Choice::Exit, c, [StatementHandle(0); 3])
            .unwrap()
            .is_none());
        m.close_delegation(d.session).unwrap();
        assert_eq!(
            m.first_meeting_closed_sessions().unwrap()[0].history,
            [d.statement]
        );
    }
    #[test]
    fn pending_action_consumes_first_matching_gameplay_endpoints_only() {
        use crate::diplomacy_manager::PendingAction;
        let (mut m, _) = fresh();
        let unrelated = PendingAction {
            id: 0xc08d_3473,
            from: 0,
            to: 1,
        };
        let matching = PendingAction {
            id: 0xc08d_3473,
            from: 1,
            to: 0,
        };
        *m.pending_actions_mut().unwrap() = vec![unrelated, matching, matching];
        m.consume_delegation_action(1, 0).unwrap();
        assert_eq!(m.pending_actions().unwrap(), [unrelated, matching]);
    }
}
