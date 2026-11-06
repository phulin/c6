//! Bounded computed Linux Diplomacy::Manager::ProcessStatement transitions.
//!
//! Reuses the existing pending drain; this is the next statement-processing
//! boundary. Plans cover single-player existing-session queueing and concrete
//! Response processing/destruction. The canonical Manager also owns computed
//! new-session creation with copied numeric IDs and fresh logical object labels.
//! No native AI policy or player decision is supplied. Missing active subtype
//! effects and multiplayer notification/routing remain explicit dependencies.
//! Initial logical identities, statement kinds and ordinary disabled logger
//! binding must come from the actual runtime state; Save alone does not supply
//! these unsaved globals. This module is not full action/native parity support.
use crate::diplomacy_manager::StatementHandle;
use crate::unit_operation_aftermath::Pending;

pub const SINGLE_PLAYER_MODE: u32 = 0xe9685ee7;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionHandle(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatementKind {
    /// Concrete Response Process is empty and its destructor has only ordinary
    /// allocation cleanup. This is a semantic kind, not a native address.
    Response,
    /// Constructor-bounded FirstMeeting payload; other inherited fields retain
    /// their native constructor defaults until a supported transition changes them.
    FirstMeeting {
        type_hash: u32,
        initiator: i32,
        response_type: u32,
        hospitality_city: crate::unit::PlotCoord,
    },
    /// Source-bounded Delegation/ProposeAction payload. Embedded action endpoints
    /// are the initiator and its other session player, always enable=true.
    Delegation {
        initiator: i32,
        response_type: u32,
        subtype: u32,
        action_pending: bool,
    },
    Other {
        type_hash: u32,
        subtype_hash: u32,
    },
}
impl StatementKind {
    pub(crate) fn ordinary_destructor(self) -> bool {
        matches!(
            self,
            Self::Response | Self::FirstMeeting { .. } | Self::Delegation { .. }
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatementInfo {
    pub handle: StatementHandle,
    pub session_id: u32,
    pub from: i32,
    pub to: i32,
    pub kind: StatementKind,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Logging {
    Unknown,
    /// Source-closed ordinary host LoggingManager, initially disabled. Its lookup
    /// returns null before opening a file or calling a log channel. Supported
    /// transitions do not modify this flag/binding; this is not a fabricated
    /// logging-result default or support for arbitrary custom logging callbacks.
    OrdinaryHostDisabled,
    /// Source-closed stock host LoggingManager/GetLogFile and ordinary LogFile
    /// sink. Current adapter facts bind the actual enabled manager, channels,
    /// vtables and platform file-output path. Diplomacy logging reads scalar
    /// gameplay values; only formatting, file/timestamp output and archive dirty
    /// bookkeeping are omitted at the headless boundary. This does not admit
    /// custom log callbacks or supply a fabricated disabled-logger observation.
    OrdinaryHostFileSink,
}
impl Logging {
    pub const fn supports_diplomacy_output(self) -> bool {
        matches!(
            self,
            Self::OrdinaryHostDisabled | Self::OrdinaryHostFileSink
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Context {
    pub local_player: i32,
    /// The actual configuration GameMode scalar; all values other than the
    /// source single-player hash enter the multiplayer branch.
    pub game_mode: u32,
    pub active_session: Option<SessionHandle>,
    pub logging: Logging,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionHeader {
    pub handle: SessionHandle,
    pub id: u32,
    pub from: i32,
    pub to: i32,
}
pub struct SessionView<'a> {
    pub header: SessionHeader,
    pub history: &'a [StatementHandle],
    pub pending: &'a [Pending<StatementHandle>],
    pub queued: &'a [StatementHandle],
}
/// Borrow the canonical owned session lists; do not maintain a second copy of
/// Manager pending state. The owner must guard the whole World revision jointly
/// with all other action deltas before any apply call.
pub struct SessionWrite<'a> {
    pub header: SessionHeader,
    pub history: &'a mut Vec<StatementHandle>,
    pub pending: &'a mut Vec<Pending<StatementHandle>>,
    pub queued: &'a mut Vec<StatementHandle>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    LoggingBinding,
    MultiplayerNotificationAndRouting,
    StatementDestructor(StatementKind),
    StatementProcess(StatementKind),
    PendingDestructor(StatementHandle),
    PendingStatementMetadata(StatementHandle),
    /// Source construction is known, but the owner must allocate a fresh logical
    /// session identity, append it and update active ownership in one commit.
    SessionCreation {
        id: u32,
        from: i32,
        to: i32,
    },
    SessionMetadata,
    InconsistentState,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Unsupported(Unsupported),
    Conflict,
    ResourceLimit,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Null,
    DiscardResponse,
    Queue,
    ProcessResponse,
    /// Session ownership bookkeeping; subtype report/decision continuation is
    /// mandatory through Manager::apply_first_meeting_statement.
    ProcessFirstMeeting,
}
#[derive(Debug)]
pub struct Plan {
    context: Context,
    statement: Option<StatementInfo>,
    target: Option<SessionHeader>,
    remove: Option<(usize, Pending<StatementHandle>)>,
    effect: Effect,
    activate: bool,
    creation: bool,
}
/// Retirement is a required logical ownership delta. The action owner must
/// remove these handles from its typed statement inventory in the same commit;
/// returning them does not implement a generic statement allocator or transport.
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    pub native_result: bool,
    pub retired_statement: Option<StatementHandle>,
}
impl Plan {
    pub fn effect(&self) -> Effect {
        self.effect
    }
    pub fn target(&self) -> Option<SessionHeader> {
        self.target
    }
    pub fn creates_session(&self) -> bool {
        self.creation
    }
    pub(crate) fn activates_session(&self) -> bool {
        self.activate
    }
    pub fn retired_statement(&self) -> Option<StatementHandle> {
        if self.effect == Effect::DiscardResponse {
            self.statement.map(|s| s.handle)
        } else {
            self.remove.as_ref().map(|(_, p)| p.statement)
        }
    }
    pub(crate) fn validate(
        &self,
        context: Context,
        statement: Option<StatementInfo>,
        target: Option<SessionView<'_>>,
    ) -> Result<(), Error> {
        if self.context != context || self.statement != statement {
            return Err(Error::Conflict);
        }
        if matches!(self.effect, Effect::Null | Effect::DiscardResponse) {
            return Ok(());
        }
        let target = target.ok_or(Error::Conflict)?;
        let statement = self.statement.ok_or(Error::Conflict)?;
        if Some(target.header) != self.target
            || target.history.contains(&statement.handle)
            || target.queued.contains(&statement.handle)
            || target
                .pending
                .iter()
                .any(|p| p.statement == statement.handle)
        {
            return Err(Error::Conflict);
        }
        if self.creation
            && (!target.history.is_empty()
                || !target.pending.is_empty()
                || !target.queued.is_empty())
        {
            return Err(Error::Conflict);
        }
        if matches!(
            self.effect,
            Effect::ProcessResponse | Effect::ProcessFirstMeeting
        ) {
            let first = target.pending.iter().position(|p| p.from == statement.from);
            match (&self.remove, first) {
                (None, None) => {}
                (Some((index, pending)), Some(actual))
                    if *index == actual && target.pending[actual] == *pending => {}
                _ => return Err(Error::Conflict),
            }
        }
        Ok(())
    }
    pub(crate) fn reserve(&self, target: &mut SessionWrite<'_>) -> Result<(), Error> {
        match self.effect {
            Effect::Queue => target
                .queued
                .try_reserve(1)
                .map_err(|_| Error::ResourceLimit),
            Effect::ProcessResponse | Effect::ProcessFirstMeeting => target
                .history
                .try_reserve(1)
                .map_err(|_| Error::ResourceLimit),
            _ => Ok(()),
        }
    }
    /// Internal commit requires validate/reserve and the common World guard;
    /// callers perform no callbacks or intervening mutation between them.
    pub(crate) fn commit(self, context: &mut Context, target: Option<SessionWrite<'_>>) -> Outcome {
        match self.effect {
            Effect::Null => Outcome {
                native_result: false,
                retired_statement: None,
            },
            Effect::DiscardResponse => Outcome {
                native_result: false,
                retired_statement: self.statement.map(|s| s.handle),
            },
            Effect::Queue | Effect::ProcessResponse | Effect::ProcessFirstMeeting => {
                let target = target.expect("validated statement target");
                let statement = self.statement.expect("validated nonnull statement");
                if self.effect == Effect::Queue {
                    target.queued.push(statement.handle);
                    return Outcome {
                        native_result: true,
                        retired_statement: None,
                    };
                }
                if self.activate {
                    context.active_session = Some(target.header.handle);
                }
                target.history.push(statement.handle);
                let retired_statement =
                    self.remove.map(|(i, _)| target.pending.remove(i).statement);
                Outcome {
                    native_result: true,
                    retired_statement,
                }
            }
        }
    }
    /// Existing-session leaf apply. Creation is committed through Manager so its
    /// one canonical pending-list owner, ordered metadata and identity allocator
    /// change together. Whole-action atomicity still needs the common World guard.
    pub fn apply(
        self,
        context: &mut Context,
        statement: Option<StatementInfo>,
        mut target: Option<SessionWrite<'_>>,
    ) -> Result<Outcome, Error> {
        if self.creation {
            let h = self.target.expect("creation header");
            return Err(Error::Unsupported(Unsupported::SessionCreation {
                id: h.id,
                from: h.from,
                to: h.to,
            }));
        }
        self.validate(
            *context,
            statement,
            target.as_ref().map(|t| SessionView {
                header: t.header,
                history: t.history,
                pending: t.pending,
                queued: t.queued,
            }),
        )?;
        if let Some(t) = target.as_mut() {
            self.reserve(t)?;
        }
        Ok(self.commit(context, target))
    }
}

/// Source statement routing over the current ordered open-session list. Pending
/// from/to fields are admitted statement header projections, not invented queue
/// metadata. Facts supply concrete payload kinds only for reached destruction.
pub fn plan(
    context: Context,
    statement: Option<StatementInfo>,
    sessions: &[SessionView<'_>],
    facts: &[StatementInfo],
) -> Result<Plan, Error> {
    plan_internal(context, statement, sessions, facts, None, false)
}

/// Creation uses the canonical Manager's next fresh logical object label. It
/// does not generate or replace the incoming statement's numeric session ID.
pub(crate) fn plan_with_creation(
    context: Context,
    statement: Option<StatementInfo>,
    sessions: &[SessionView<'_>],
    facts: &[StatementInfo],
    fresh: SessionHandle,
) -> Result<Plan, Error> {
    plan_internal(context, statement, sessions, facts, Some(fresh), false)
}
/// Called only after a dedicated FirstMeeting or Delegation owner validates its bounded
/// payload, current endpoint facts and required report/decision continuation.
pub(crate) fn plan_first_meeting(
    context: Context,
    statement: StatementInfo,
    sessions: &[SessionView<'_>],
    facts: &[StatementInfo],
) -> Result<Plan, Error> {
    plan_internal(context, Some(statement), sessions, facts, None, true)
}
fn plan_internal(
    context: Context,
    statement: Option<StatementInfo>,
    sessions: &[SessionView<'_>],
    facts: &[StatementInfo],
    fresh: Option<SessionHandle>,
    first_meeting: bool,
) -> Result<Plan, Error> {
    let result = |effect, target, remove| Plan {
        context,
        statement,
        target,
        remove,
        effect,
        activate: matches!(
            effect,
            Effect::ProcessResponse | Effect::ProcessFirstMeeting
        ) && context.active_session.is_none(),
        creation: false,
    };
    let Some(statement) = statement else {
        return Ok(result(Effect::Null, None, None));
    };
    let unsupported = |u| Error::Unsupported(u);
    if !context.logging.supports_diplomacy_output() {
        return Err(unsupported(Unsupported::LoggingBinding));
    }
    if context.game_mode != SINGLE_PLAYER_MODE {
        return Err(unsupported(Unsupported::MultiplayerNotificationAndRouting));
    }
    if statement.from != context.local_player && statement.to != context.local_player {
        return if statement.kind.ordinary_destructor() {
            Ok(result(Effect::DiscardResponse, None, None))
        } else {
            Err(unsupported(Unsupported::StatementDestructor(
                statement.kind,
            )))
        };
    }
    let created;
    let (session, creation) = if let Some(existing) = sessions
        .iter()
        .find(|s| s.header.id == statement.session_id)
    {
        if sessions
            .iter()
            .filter(|s| s.header.handle == existing.header.handle)
            .count()
            != 1
        {
            return Err(unsupported(Unsupported::InconsistentState));
        }
        (existing, false)
    } else {
        let fresh = fresh.ok_or(unsupported(Unsupported::SessionCreation {
            id: statement.session_id,
            from: statement.from,
            to: statement.to,
        }))?;
        if fresh.0.checked_add(1).is_none() {
            return Err(Error::ResourceLimit);
        }
        if context.active_session == Some(fresh)
            || sessions.iter().any(|s| s.header.handle == fresh)
        {
            return Err(unsupported(Unsupported::InconsistentState));
        }
        created = SessionView {
            header: SessionHeader {
                handle: fresh,
                id: statement.session_id,
                from: statement.from,
                to: statement.to,
            },
            history: &[],
            pending: &[],
            queued: &[],
        };
        (&created, true)
    };

    if session.history.contains(&statement.handle)
        || session.queued.contains(&statement.handle)
        || session
            .pending
            .iter()
            .any(|p| p.statement == statement.handle)
    {
        return Err(unsupported(Unsupported::InconsistentState));
    }
    if context
        .active_session
        .is_some_and(|active| active != session.header.handle)
    {
        let mut p = result(Effect::Queue, Some(session.header), None);
        p.creation = creation;
        return Ok(p);
    }
    if statement.kind != StatementKind::Response && !first_meeting {
        return Err(unsupported(Unsupported::StatementProcess(statement.kind)));
    }
    let remove = session
        .pending
        .iter()
        .enumerate()
        .find(|(_, p)| p.from == statement.from)
        .map(|(i, p)| (i, p.clone()));
    if let Some((_, pending)) = &remove {
        let mut matches = facts.iter().filter(|f| f.handle == pending.statement);
        let fact = matches
            .next()
            .ok_or(unsupported(Unsupported::PendingStatementMetadata(
                pending.statement,
            )))?;
        if matches.next().is_some() || fact.from != pending.from || fact.to != pending.to {
            return Err(unsupported(Unsupported::InconsistentState));
        }
        if !fact.kind.ordinary_destructor() {
            return Err(unsupported(Unsupported::PendingDestructor(
                pending.statement,
            )));
        }
    }
    let effect = if first_meeting {
        Effect::ProcessFirstMeeting
    } else {
        Effect::ProcessResponse
    };
    let mut p = result(effect, Some(session.header), remove);
    p.creation = creation;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn context(active: Option<u64>) -> Context {
        Context {
            local_player: 0,
            game_mode: SINGLE_PLAYER_MODE,
            active_session: active.map(SessionHandle),
            logging: Logging::OrdinaryHostDisabled,
        }
    }
    fn statement(handle: u64, from: i32, to: i32, kind: StatementKind) -> StatementInfo {
        StatementInfo {
            handle: StatementHandle(handle),
            session_id: 7,
            from,
            to,
            kind,
        }
    }
    fn header(handle: u64) -> SessionHeader {
        SessionHeader {
            handle: SessionHandle(handle),
            id: 7,
            from: 0,
            to: 1,
        }
    }
    fn pending(handle: u64, from: i32, to: i32) -> Pending<StatementHandle> {
        Pending {
            statement: StatementHandle(handle),
            from,
            to,
        }
    }
    #[test]
    fn inactive_session_queues_without_invoking_unknown_subtype() {
        let info = statement(
            10,
            1,
            0,
            StatementKind::Other {
                type_hash: 2,
                subtype_hash: 3,
            },
        );
        let mut history = vec![StatementHandle(1)];
        let mut outgoing = vec![pending(11, 1, 0)];
        let mut queued = vec![StatementHandle(2)];
        let h = header(8);
        let mut context = context(Some(9));
        let p = plan(
            context,
            Some(info),
            &[SessionView {
                header: h,
                history: &history,
                pending: &outgoing,
                queued: &queued,
            }],
            &[],
        )
        .unwrap();
        assert_eq!(p.effect(), Effect::Queue);
        assert_eq!(
            p.apply(
                &mut context,
                Some(info),
                Some(SessionWrite {
                    header: h,
                    history: &mut history,
                    pending: &mut outgoing,
                    queued: &mut queued
                })
            )
            .unwrap(),
            Outcome {
                native_result: true,
                retired_statement: None
            }
        );
        assert_eq!(queued, vec![StatementHandle(2), StatementHandle(10)]);
        assert_eq!(history, vec![StatementHandle(1)]);
        assert_eq!(outgoing, vec![pending(11, 1, 0)]);
    }
    #[test]
    fn response_activates_appends_and_removes_first_source_without_destination_filter() {
        let info = statement(10, 1, 0, StatementKind::Response);
        let old = statement(20, 1, 3, StatementKind::Response);
        let mut history = vec![];
        let mut outgoing = vec![pending(19, 2, 1), pending(20, 1, 3), pending(21, 1, 0)];
        let mut queued = vec![];
        let h = header(8);
        let mut context = context(None);
        let p = plan(
            context,
            Some(info),
            &[SessionView {
                header: h,
                history: &history,
                pending: &outgoing,
                queued: &queued,
            }],
            &[old],
        )
        .unwrap();
        assert_eq!(p.retired_statement(), Some(StatementHandle(20)));
        let result = p
            .apply(
                &mut context,
                Some(info),
                Some(SessionWrite {
                    header: h,
                    history: &mut history,
                    pending: &mut outgoing,
                    queued: &mut queued,
                }),
            )
            .unwrap();
        assert_eq!(context.active_session, Some(h.handle));
        assert_eq!(history, vec![info.handle]);
        assert_eq!(outgoing, vec![pending(19, 2, 1), pending(21, 1, 0)]);
        assert_eq!(result.retired_statement, Some(old.handle));
    }
    #[test]
    fn duplicate_numeric_ids_select_first_logical_session() {
        let info = statement(10, 1, 0, StatementKind::Response);
        let views = [
            SessionView {
                header: header(8),
                history: &[],
                pending: &[],
                queued: &[],
            },
            SessionView {
                header: header(9),
                history: &[],
                pending: &[],
                queued: &[],
            },
        ];
        let p = plan(context(Some(9)), Some(info), &views, &[]).unwrap();
        assert_eq!(p.target(), Some(header(8)));
        assert_eq!(p.effect(), Effect::Queue);
    }
    #[test]
    fn unsupported_destructor_and_active_process_leave_lists_untouched() {
        let info = statement(10, 1, 0, StatementKind::Response);
        let bad = statement(
            20,
            1,
            0,
            StatementKind::Other {
                type_hash: 2,
                subtype_hash: 3,
            },
        );
        let outgoing = vec![pending(20, 1, 0)];
        let view = SessionView {
            header: header(8),
            history: &[],
            pending: &outgoing,
            queued: &[],
        };
        assert_eq!(
            plan(context(Some(8)), Some(info), &[view], &[bad]).unwrap_err(),
            Error::Unsupported(Unsupported::PendingDestructor(bad.handle))
        );
        assert_eq!(outgoing, vec![pending(20, 1, 0)]);
        let view = SessionView {
            header: header(8),
            history: &[],
            pending: &[],
            queued: &[],
        };
        assert_eq!(
            plan(context(Some(8)), Some(bad), &[view], &[]).unwrap_err(),
            Error::Unsupported(Unsupported::StatementProcess(bad.kind))
        );
    }
    #[test]
    fn stale_first_matching_pending_is_rejected_before_activation_or_history() {
        let info = statement(10, 1, 0, StatementKind::Response);
        let old = statement(20, 1, 0, StatementKind::Response);
        let h = header(8);
        let mut context = context(None);
        let mut outgoing = vec![pending(20, 1, 0)];
        let mut history = vec![];
        let mut queued = vec![];
        let p = plan(
            context,
            Some(info),
            &[SessionView {
                header: h,
                history: &history,
                pending: &outgoing,
                queued: &queued,
            }],
            &[old],
        )
        .unwrap();
        outgoing.insert(0, pending(21, 1, 0));
        assert_eq!(
            p.apply(
                &mut context,
                Some(info),
                Some(SessionWrite {
                    header: h,
                    history: &mut history,
                    pending: &mut outgoing,
                    queued: &mut queued
                })
            ),
            Err(Error::Conflict)
        );
        assert!(history.is_empty());
        assert_eq!(context.active_session, None);
        assert_eq!(outgoing.len(), 2);
    }
    #[test]
    fn null_bypasses_log_and_remote_response_is_retired_without_session_routing() {
        let mut context = context(None);
        context.logging = Logging::Unknown;
        context.game_mode = 0;
        let p = plan(context, None, &[], &[]).unwrap();
        assert_eq!(
            p.apply(&mut context, None, None).unwrap(),
            Outcome {
                native_result: false,
                retired_statement: None
            }
        );
        context = context_for_remote();
        let info = statement(10, 1, 2, StatementKind::Response);
        let p = plan(context, Some(info), &[], &[]).unwrap();
        assert_eq!(
            p.apply(&mut context, Some(info), None).unwrap(),
            Outcome {
                native_result: false,
                retired_statement: Some(info.handle)
            }
        );
        context.game_mode = 0;
        assert_eq!(
            plan(context, Some(info), &[], &[]).unwrap_err(),
            Error::Unsupported(Unsupported::MultiplayerNotificationAndRouting)
        );
    }
    fn context_for_remote() -> Context {
        context(None)
    }
}
