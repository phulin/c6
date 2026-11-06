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
    State(crate::loaded_state::Error),
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
        let (payload, decision) = self.commit_first_meeting_statement(plan)?;
        report(payload).map_err(Error::Report)?;
        Ok(decision)
    }
    fn commit_first_meeting_statement<E>(
        &mut self,
        plan: Plan,
    ) -> Result<(Report, PendingDecision), Error<E>> {
        if self.drain_owner != plan.owner || self.drain_revision != Some(plan.revision) {
            return Err(Error::Statement(statement::Error::Conflict));
        }
        self.apply_statement(plan.ownership, plan.context, Some(plan.incoming))
            .map_err(Error::Statement)?;
        Ok((plan.report, plan.decision))
    }
}
impl crate::loaded_state::LoadedState {
    /// Commit canonical session processing before invoking a report consumer
    /// against this same LoadedState. The manager borrow ends before callbacks,
    /// so gameplay/RNG effects can read or mutate current canonical state.
    /// As with Manager apply, the owner must guard the common World revision
    /// and preflight the concrete report provider before entering this method.
    pub fn apply_first_meeting_statement<E>(
        &mut self,
        plan: Plan,
        report: impl FnOnce(&mut Self, Report) -> Result<(), E>,
    ) -> Result<PendingDecision, Error<E>> {
        let (payload, decision) = self
            .diplomacy_manager_mut()
            .map_err(Error::State)?
            .commit_first_meeting_statement(plan)?;
        report(self, payload).map_err(Error::Report)?;
        Ok(decision)
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
    #[test]
    fn actual_enabled_stock_file_sink_keeps_canonical_processing_and_policy_yield() {
        let (mut manager, mut context, ai, human) = fixture();
        context.logging = Logging::OrdinaryHostFileSink;
        manager
            .initialize_execution_profile(context.game_mode, context.logging)
            .unwrap();
        let plan = manager
            .plan_first_meeting_statement(
                StatementHandle(3),
                context,
                ai,
                human,
                &[Choice::Positive, Choice::Exit],
            )
            .unwrap();
        let decision = manager
            .apply_first_meeting_statement(plan, |_| Ok::<_, ()>(()))
            .unwrap();
        assert_eq!(decision.player, human.owner);
        assert_eq!(
            manager.session_metadata().unwrap()[0].history,
            [StatementHandle(3)]
        );
        assert_eq!(
            manager.execution_profile(),
            Some((context.game_mode, Logging::OrdinaryHostFileSink))
        );
    }
    fn ready_choice() -> (Manager, PendingDecision, ChoiceContext) {
        let (mut manager, context, ai, human) = fixture();
        manager
            .initialize_first_meeting_empty_closed_sessions(0)
            .unwrap();
        let plan = manager
            .plan_first_meeting_statement(
                StatementHandle(3),
                context,
                ai,
                human,
                &[Choice::Positive, Choice::Exit],
            )
            .unwrap();
        let decision = manager
            .apply_first_meeting_statement(plan, |_| Ok::<_, ()>(()))
            .unwrap();
        let context = ChoiceContext {
            statement: Context {
                active_session: Some(decision.session.handle),
                ..context
            },
            ai,
            human,
            ai_has_capital: false,
            human_has_capital: false,
            active_human_count: 1,
            local_turn_active: true,
        };
        (manager, decision, context)
    }
    #[test]
    fn founded_capitals_allow_external_positive_and_exit() {
        for choice in [Choice::Positive, Choice::Exit] {
            let (manager, decision, mut context) = ready_choice();
            context.ai_has_capital = true;
            context.human_has_capital = true;
            assert!(manager
                .plan_first_meeting_choice(
                    decision,
                    choice,
                    context,
                    [StatementHandle(4), StatementHandle(5), StatementHandle(6)]
                )
                .is_ok());
        }
    }
    #[test]
    fn positive_requires_second_choice_then_keeps_complete_closed_history() {
        let (mut manager, decision, context) = ready_choice();
        let first = manager
            .plan_first_meeting_choice(
                decision,
                Choice::Positive,
                context,
                [StatementHandle(4), StatementHandle(5), StatementHandle(6)],
            )
            .unwrap();
        let report = manager
            .begin_first_meeting_choice::<()>(&first)
            .unwrap()
            .unwrap();
        assert_eq!(
            report.statement.kind,
            StatementKind::FirstMeeting {
                type_hash: INFO_EXCHANGE,
                initiator: 1,
                response_type: 0,
                hospitality_city: crate::unit::PlotCoord { x: -9999, y: -9999 }
            }
        );
        assert_eq!(report.session_statement_index, 3);
        assert!(!report.first_of_type);
        assert!(!manager
            .statement_inventory()
            .unwrap()
            .iter()
            .any(|s| s.handle == StatementHandle(4)));
        assert_eq!(
            manager.session_metadata().unwrap()[0].history,
            [StatementHandle(3), StatementHandle(5), StatementHandle(6)]
        );
        let decision = PendingDecision {
            statement: report.statement.handle,
            ..decision
        };
        assert!(manager
            .plan_first_meeting_choice(
                decision,
                Choice::Positive,
                ChoiceContext {
                    local_turn_active: false,
                    ..context
                },
                [StatementHandle(7), StatementHandle(8), StatementHandle(9)],
            )
            .is_err());
        let matching = crate::diplomacy_manager::PendingAction {
            id: STATEMENT_RESULT_ACTION,
            from: context.human.owner,
            to: context.ai.owner,
        };
        let unrelated = crate::diplomacy_manager::PendingAction {
            id: 123,
            ..matching
        };
        manager.pending_actions = Some(vec![unrelated, matching, matching]);
        let second = manager
            .plan_first_meeting_choice(
                decision,
                Choice::Positive,
                context,
                [StatementHandle(7), StatementHandle(8), StatementHandle(9)],
            )
            .unwrap();
        assert!(manager
            .begin_first_meeting_choice::<()>(&second)
            .unwrap()
            .is_none());
        assert_eq!(manager.pending_actions().unwrap(), [unrelated, matching]);
        manager.close_first_meeting_choice::<()>(&second).unwrap();
        assert!(manager.open_sessions().is_empty());
        assert_eq!(manager.active_session().unwrap(), None);
        assert_eq!(
            manager.first_meeting_closed_sessions().unwrap()[0].history,
            [
                StatementHandle(3),
                StatementHandle(5),
                StatementHandle(6),
                StatementHandle(8)
            ]
        );
        assert_eq!(manager.statement_inventory().unwrap().len(), 4);
    }
    #[test]
    fn exit_retains_initial_history_without_response_and_unknown_closed_refuses() {
        let (mut manager, decision, context) = ready_choice();
        let exit = manager
            .plan_first_meeting_choice(decision, Choice::Exit, context, [StatementHandle(0); 3])
            .unwrap();
        assert!(manager
            .begin_first_meeting_choice::<()>(&exit)
            .unwrap()
            .is_none());
        manager.close_first_meeting_choice::<()>(&exit).unwrap();
        assert_eq!(
            manager.first_meeting_closed_sessions().unwrap()[0].history,
            [StatementHandle(3)]
        );
        assert_eq!(manager.statement_inventory().unwrap().len(), 1);
        let (mut manager, decision, context) = ready_choice();
        manager.routing.as_mut().unwrap().closed = None;
        assert!(manager
            .plan_first_meeting_choice(decision, Choice::Exit, context, [StatementHandle(0); 3])
            .is_err());
        assert_eq!(manager.open_sessions().len(), 1);
    }
}

pub const INFO_EXCHANGE: u32 = 0xd6b6_fe21;
pub const POSITIVE_RESPONSE: u32 = 0x7332_f7d6;
pub const CLOSED_REPORT_EVENT: u32 = 0x1441_0052;
pub const STATEMENT_RESULT_ACTION: u32 = 0x6ecb_4120;

/// Current control/capital facts for the bounded, synchronous local single-player
/// message continuation. The visibility provider owns present-capital effects.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChoiceContext {
    pub statement: Context,
    pub ai: Player,
    pub human: Player,
    pub ai_has_capital: bool,
    pub human_has_capital: bool,
    pub active_human_count: u32,
    /// Actual current local human IsTurnActive. Result-action messages unready
    /// turns; true takes CanUnreadyTurn's source early false return.
    pub local_turn_active: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChoiceEffect {
    Statement(Report),
    /// Native SetHadFriendlyMeeting(player, other, true), including the archive
    /// dirty boundary. The required owner performs this actual gameplay write.
    FriendlyMeeting {
        player: i32,
        other: i32,
    },
    /// Query the current capital and permanently reveal its radius-one neighborhood.
    /// Missing capitals skip; present capitals require the canonical visibility owner.
    RevealCapital {
        owner: i32,
        recipient: i32,
    },
    /// Delivered Messages.DiplomacyAction::Execute unconditionally enters
    /// Game.Update AFTER the reconstructed result statement's gameplay effects.
    /// The canonical Session owns this continuation, including reached policy
    /// boundaries/RNG; a report-only or successful no-op provider is invalid.
    /// The generated CloseSession command remains ordered after this action.
    ResultActionUpdate {
        action_type: u32,
        from: i32,
        to: i32,
    },
    SessionClosed {
        session_id: u32,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChoiceOutcome {
    Pending(PendingDecision),
    Closed(SessionHeader),
}
#[derive(Debug)]
pub struct ChoicePlan {
    owner: u64,
    revision: u64,
    decision: PendingDecision,
    choice: Choice,
    info_exchange: bool,
    /// Source identities: local queued response (destroyed on delivery),
    /// delivered response and optional AI followup. They must all be fresh.
    handles: [StatementHandle; 3],
}
impl Manager {
    /// Bind an actual observed empty closed list once; an unavailable or
    /// nonempty observation is never silently replaced with an empty owner.
    pub fn initialize_first_meeting_empty_closed_sessions(
        &mut self,
        observed_count: usize,
    ) -> Result<(), Error<std::convert::Infallible>> {
        let routing = self
            .routing
            .as_mut()
            .filter(|r| r.authoritative)
            .ok_or(Error::Unsupported("closed session routing"))?;
        if observed_count != 0 || routing.closed.is_some() {
            return Err(Error::Unsupported("initial closed session inventory"));
        }
        routing.closed = Some(Vec::new());
        self.invalidate_drain_plans();
        Ok(())
    }
    pub fn first_meeting_closed_sessions(
        &self,
    ) -> Option<&[crate::diplomacy_manager::ObservedSession]> {
        self.routing
            .as_ref()
            .filter(|r| r.authoritative)?
            .closed
            .as_deref()
    }
    /// Plan one explicit player choice. The owner admits the existing local
    /// synchronized message delivery boundary and guards its common World stamp.
    /// Only this single open session is supported; no queued/other-session work
    /// can be hidden by collapsing the synchronous message roundtrip.
    pub fn plan_first_meeting_choice(
        &self,
        decision: PendingDecision,
        choice: Choice,
        context: ChoiceContext,
        handles: [StatementHandle; 3],
    ) -> Result<ChoicePlan, Error<std::convert::Infallible>> {
        let fail = Error::Unsupported;
        let revision = self
            .drain_revision
            .ok_or(Error::Statement(statement::Error::Conflict))?;
        let routing = self
            .routing
            .as_ref()
            .filter(|r| r.authoritative)
            .ok_or(fail("first meeting routing"))?;
        let inventory = self
            .statements
            .as_deref()
            .ok_or(fail("first meeting inventory"))?;
        if self.open_sessions.len() != 1
            || routing.metadata.len() != 1
            || routing.closed.is_none()
            || routing.active != Some(decision.session.handle)
            || context.statement.active_session != routing.active
            || context.statement.local_player != decision.player
            || context.statement.game_mode != statement::SINGLE_PLAYER_MODE
            || !context.statement.logging.supports_diplomacy_output()
            || self
                .execution_profile()
                .is_some_and(|p| p != (context.statement.game_mode, context.statement.logging))
            || context.ai.owner != decision.session.from
            || context.human.owner != decision.session.to
            || !context.ai.major
            || context.ai.human
            || !context.human.major
            || !context.human.human
            || context.human.owner != decision.player
            || context.active_human_count == 0
        {
            return Err(fail("local first meeting choice profile"));
        }
        let metadata = &routing.metadata[0];
        if metadata.header != decision.session
            || !self.open_sessions[0].pending.is_empty()
            || !metadata.queued.is_empty()
            || metadata.history.last() != Some(&decision.statement)
            || decision.choices != [Choice::Positive, Choice::Exit]
        {
            return Err(fail("first meeting decision ownership"));
        }
        let selected = inventory
            .iter()
            .find(|s| s.handle == decision.statement)
            .ok_or(fail("first meeting selected statement"))?;
        let info_exchange = match selected.kind {
            StatementKind::FirstMeeting {
                type_hash,
                initiator,
                response_type,
                hospitality_city,
            } if initiator == context.ai.owner
                && hospitality_city.x == -9999
                && hospitality_city.y == -9999
                && selected.from == context.ai.owner
                && selected.to == context.human.owner
                && selected.session_id == decision.session.id =>
            {
                if type_hash == crate::diplomacy_meeting::NEUTRAL_MEETING
                    && response_type == INITIAL_RESPONSE
                    && metadata.history.len() == 1
                {
                    false
                } else if type_hash == INFO_EXCHANGE
                    && response_type == 0
                    && metadata.history.len() == 3
                {
                    true
                } else {
                    return Err(fail("first meeting response phase"));
                }
            }
            _ => return Err(fail("first meeting choice payload")),
        };
        // All retained history must belong to this source-bounded conversation.
        if metadata
            .history
            .iter()
            .any(|h| inventory.iter().filter(|s| s.handle == *h).count() != 1)
        {
            return Err(fail("first meeting history inventory"));
        }
        if info_exchange {
            for (index, from, to, response) in [
                (0, context.ai.owner, context.human.owner, INITIAL_RESPONSE),
                (1, context.human.owner, context.ai.owner, POSITIVE_RESPONSE),
            ] {
                let previous = inventory
                    .iter()
                    .find(|s| s.handle == metadata.history[index])
                    .ok_or(fail("first meeting prior history"))?;
                if previous.from != from
                    || previous.to != to
                    || previous.session_id != decision.session.id
                    || !matches!(previous.kind, StatementKind::FirstMeeting {
                        type_hash: crate::diplomacy_meeting::NEUTRAL_MEETING,
                        initiator, response_type, hospitality_city,
                    } if initiator == context.ai.owner && response_type == response
                        && hospitality_city.x == -9999 && hospitality_city.y == -9999)
                {
                    return Err(fail("first meeting prior history payload"));
                }
            }
        }
        if choice == Choice::Positive && info_exchange && !context.local_turn_active {
            return Err(fail(
                "first meeting result-action local unready continuation",
            ));
        }
        if choice == Choice::Positive && info_exchange && self.pending_actions.is_none() {
            return Err(fail("first meeting result-action pending inventory"));
        }
        if choice == Choice::Positive
            && handles
                .iter()
                .enumerate()
                .any(|(i, h)| handles[..i].contains(h) || inventory.iter().any(|s| s.handle == *h))
        {
            return Err(fail("fresh first meeting response identities"));
        }
        Ok(ChoicePlan {
            owner: self.drain_owner,
            revision,
            decision,
            choice,
            info_exchange,
            handles,
        })
    }
    /// Commit the source local response queue/broadcast/delivery roundtrip.
    /// The transient queued response is destroyed by RemoveMatchingPending;
    /// only its separately reconstructed delivered object reaches history.
    fn begin_first_meeting_choice<E>(
        &mut self,
        plan: &ChoicePlan,
    ) -> Result<Option<Report>, Error<E>> {
        if self.drain_owner != plan.owner || self.drain_revision != Some(plan.revision) {
            return Err(Error::Statement(statement::Error::Conflict));
        }
        let routing = self
            .routing
            .as_mut()
            .filter(|r| r.authoritative)
            .ok_or(Error::Unsupported("first meeting routing"))?;
        let inventory = self
            .statements
            .as_mut()
            .ok_or(Error::Unsupported("first meeting inventory"))?;
        let metadata = &mut routing.metadata[0];
        if plan.choice == Choice::Exit {
            return Ok(None);
        }
        let selected = *inventory
            .iter()
            .find(|s| s.handle == plan.decision.statement)
            .ok_or(Error::Unsupported("first meeting selected statement"))?;
        let StatementKind::FirstMeeting {
            type_hash,
            initiator,
            hospitality_city,
            ..
        } = selected.kind
        else {
            return Err(Error::Unsupported("first meeting payload"));
        };
        inventory
            .try_reserve(2)
            .map_err(|_| Error::Statement(statement::Error::ResourceLimit))?;
        metadata
            .history
            .try_reserve(2)
            .map_err(|_| Error::Statement(statement::Error::ResourceLimit))?;
        let delivered = StatementInfo {
            handle: plan.handles[1],
            session_id: selected.session_id,
            from: selected.to,
            to: selected.from,
            kind: StatementKind::FirstMeeting {
                type_hash,
                initiator,
                response_type: POSITIVE_RESPONSE,
                hospitality_city,
            },
        };
        // plan.handles[0] names the queued/local object whose ordinary destructor
        // has completed during delivery; do not retain an extra payload owner.
        inventory.push(delivered);
        metadata.history.push(delivered.handle);
        if plan.info_exchange {
            // ProcessAction removes only the FIRST pending entry matching action
            // TYPE/from/to before invoking the reconstructed result statement.
            // PendingAction.id is the native type hash, not Action.Instance ID.
            let pending = self.pending_actions.as_mut().ok_or(Error::Unsupported(
                "first meeting result-action pending inventory",
            ))?;
            if let Some(index) = pending.iter().position(|a| {
                a.id == STATEMENT_RESULT_ACTION && a.from == delivered.from && a.to == delivered.to
            }) {
                pending.remove(index);
            }
            self.invalidate_drain_plans();
            return Ok(None);
        }
        // NO_MANS positive creates a fresh AI-originating InfoExchange. Create
        // leaves response zero; Session.Add sets initiator from the session.
        let followup = StatementInfo {
            handle: plan.handles[2],
            session_id: selected.session_id,
            from: selected.from,
            to: selected.to,
            kind: StatementKind::FirstMeeting {
                type_hash: INFO_EXCHANGE,
                initiator: metadata.header.from,
                response_type: 0,
                hospitality_city,
            },
        };
        inventory.push(followup);
        metadata.history.push(followup.handle);
        let report = Report {
            statement: followup,
            session: metadata.header,
            first_of_type: false,
            session_statement_index: metadata.history.len() as u32,
        };
        self.invalidate_drain_plans();
        Ok(Some(report))
    }
    fn close_first_meeting_choice<E>(&mut self, plan: &ChoicePlan) -> Result<(), Error<E>> {
        let routing = self
            .routing
            .as_mut()
            .filter(|r| r.authoritative)
            .ok_or(Error::Unsupported("first meeting close routing"))?;
        let expected_history = if plan.info_exchange {
            if plan.choice == Choice::Positive {
                4
            } else {
                3
            }
        } else {
            1
        };
        let expected_tail = if plan.choice == Choice::Positive {
            plan.handles[1]
        } else {
            plan.decision.statement
        };
        if self.drain_owner != plan.owner
            || routing.metadata.len() != 1
            || self.open_sessions.len() != 1
            || routing.metadata[0].header != plan.decision.session
            || routing.active != Some(plan.decision.session.handle)
            || !self.open_sessions[0].pending.is_empty()
            || !routing.metadata[0].queued.is_empty()
            || routing.metadata[0].history.len() != expected_history
            || routing.metadata[0].history.last() != Some(&expected_tail)
        {
            return Err(Error::Statement(statement::Error::Conflict));
        }
        let closed = routing
            .closed
            .as_mut()
            .ok_or(Error::Unsupported("closed session inventory"))?;
        closed
            .try_reserve(1)
            .map_err(|_| Error::Statement(statement::Error::ResourceLimit))?;
        let metadata = routing.metadata.remove(0);
        let session = self.open_sessions.remove(0);
        closed.push(crate::diplomacy_manager::ObservedSession {
            header: metadata.header,
            history: metadata.history,
            pending: session.pending,
            queued: metadata.queued,
        });
        routing.active = None;
        self.invalidate_drain_plans();
        // Native HandleCloseSession calls Manager.Process after the report. This
        // profile has no open sessions, so that continuation has no subtype work.
        Ok(())
    }
}
/// Owned close-command continuation after a delivered StatementResult's
/// deterministic gameplay effects, before its mandatory Game.Update completes.
/// Retain it while the canonical Session runs Update or yields a player choice.
#[derive(Debug)]
pub struct ResultActionContinuation {
    plan: ChoicePlan,
}
impl ResultActionContinuation {
    pub fn action(&self) -> ChoiceEffect {
        ChoiceEffect::ResultActionUpdate {
            action_type: STATEMENT_RESULT_ACTION,
            from: self.plan.decision.session.to,
            to: self.plan.decision.session.from,
        }
    }
    pub fn session(&self) -> SessionHeader {
        self.plan.decision.session
    }
}
#[derive(Debug)]
pub enum ChoiceProgress {
    Pending(PendingDecision),
    RequiresResultActionUpdate(ResultActionContinuation),
    Closed(SessionHeader),
}
impl crate::loaded_state::LoadedState {
    /// Resolve one external choice through the bounded current local transport.
    /// This convenience wrapper requires its callback to complete the actual
    /// result-action Update. Session owners that must loan shared update/research
    /// services use begin/resume below and retain the explicit continuation.
    pub fn apply_first_meeting_choice<E>(
        &mut self,
        plan: ChoicePlan,
        mut effect: impl FnMut(&mut Self, ChoiceEffect) -> Result<(), E>,
    ) -> Result<ChoiceOutcome, Error<E>> {
        match self.begin_first_meeting_choice(plan, &mut effect)? {
            ChoiceProgress::Pending(decision) => Ok(ChoiceOutcome::Pending(decision)),
            ChoiceProgress::Closed(session) => Ok(ChoiceOutcome::Closed(session)),
            ChoiceProgress::RequiresResultActionUpdate(continuation) => {
                effect(self, continuation.action()).map_err(Error::Report)?;
                self.resume_first_meeting_after_result_update(continuation, effect)
            }
        }
    }
    /// Commit response ownership and reached deterministic result gameplay,
    /// then explicitly return the mandatory Update continuation before closing.
    /// No callback is asked to borrow Session-owned update/research state while
    /// a backend or this LoadedState already has an incompatible loan.
    pub fn begin_first_meeting_choice<E>(
        &mut self,
        plan: ChoicePlan,
        mut effect: impl FnMut(&mut Self, ChoiceEffect) -> Result<(), E>,
    ) -> Result<ChoiceProgress, Error<E>> {
        let report = self
            .diplomacy_manager_mut()
            .map_err(Error::State)?
            .begin_first_meeting_choice(&plan)?;
        if let Some(report) = report {
            effect(self, ChoiceEffect::Statement(report)).map_err(Error::Report)?;
            return Ok(ChoiceProgress::Pending(PendingDecision {
                session: report.session,
                statement: report.statement.handle,
                player: report.statement.to,
                choices: [Choice::Positive, Choice::Exit],
            }));
        }
        if plan.choice == Choice::Positive {
            for (player, other) in [
                (plan.decision.session.from, plan.decision.session.to),
                (plan.decision.session.to, plan.decision.session.from),
            ] {
                effect(self, ChoiceEffect::FriendlyMeeting { player, other })
                    .map_err(Error::Report)?;
            }
            // The delivered response reverses the initial statement endpoints.
            for (owner, recipient) in [
                (plan.decision.session.to, plan.decision.session.from),
                (plan.decision.session.from, plan.decision.session.to),
            ] {
                effect(self, ChoiceEffect::RevealCapital { owner, recipient })
                    .map_err(Error::Report)?;
            }
            return Ok(ChoiceProgress::RequiresResultActionUpdate(
                ResultActionContinuation { plan },
            ));
        }
        self.finish_first_meeting_close(plan, effect)
            .map(|outcome| match outcome {
                ChoiceOutcome::Closed(session) => ChoiceProgress::Closed(session),
                ChoiceOutcome::Pending(_) => unreachable!("close returns only Closed"),
            })
    }
    /// Resume only after the same Session has completed the delivered action's
    /// mandatory Game.Update, including its pending-diplomacy/profile tail. A
    /// genuine player decision suspends this continuation; it is not completion.
    /// Validate current logical ownership/history again after Update mutations.
    pub fn resume_first_meeting_after_result_update<E>(
        &mut self,
        continuation: ResultActionContinuation,
        effect: impl FnMut(&mut Self, ChoiceEffect) -> Result<(), E>,
    ) -> Result<ChoiceOutcome, Error<E>> {
        self.finish_first_meeting_close(continuation.plan, effect)
    }
    fn finish_first_meeting_close<E>(
        &mut self,
        plan: ChoicePlan,
        mut effect: impl FnMut(&mut Self, ChoiceEffect) -> Result<(), E>,
    ) -> Result<ChoiceOutcome, Error<E>> {
        self.diplomacy_manager_mut()
            .map_err(Error::State)?
            .close_first_meeting_choice(&plan)?;
        effect(
            self,
            ChoiceEffect::SessionClosed {
                session_id: plan.decision.session.id,
            },
        )
        .map_err(Error::Report)?;
        Ok(ChoiceOutcome::Closed(plan.decision.session))
    }
}
