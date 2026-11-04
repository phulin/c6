//! Owned state for the Linux process-global Diplomacy::Manager pending drain.
//! This owner is distinct from Game.DiplomaticRelations. Statement execution and
//! external player decisions are required providers, never implicit AI policy.
use crate::diplomacy_statement::{
    self as statement, Context, SessionHandle, SessionHeader, SessionView, SessionWrite,
    StatementInfo,
};
use crate::unit_operation_aftermath::{self as aftermath, Session};
use std::sync::atomic::{AtomicU64, Ordering};

// Representation-only owner token for rejecting cross-manager deferred plans.
static NEXT_DRAIN_OWNER: AtomicU64 = AtomicU64::new(0);
fn drain_owner() -> Result<u64, Error> {
    NEXT_DRAIN_OWNER
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
        .map_err(|_| Error::ResourceLimit)
}

/// Stable logical identity of an owned Player object. Native address bits are
/// capture evidence, not the Rust object's address. Player replacement is not
/// supported by the current LoadedState API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlayerHandle {
    pub owner: i32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StatementHandle(pub u64);
/// Exact 12-byte native PendingAction fields; processing these actions is a
/// separate dependency from ProcessPending's outgoing session-statement drain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PendingAction {
    pub id: u32,
    pub from: i32,
    pub to: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    InvalidPlayerInventory,
    SessionsRequired,
    ResourceLimit,
    SessionMetadataRequired,
    InvalidSessionMetadata,
    InvalidStatementInventory,
    StatementInventoryAlreadyInitialized,
    ExecutionProfileAlreadyInitialized,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "diplomacy manager: {self:?}")
    }
}
impl std::error::Error for Error {}

/// An actual source getter result, not an initialized-save default. The target
/// getter tests the SAME list count used by ProcessPendingSessionStatement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EmptyOpenSessions(());
impl EmptyOpenSessions {
    pub fn from_has_pending_session_statements(nonempty: bool) -> Result<Self, Error> {
        if nonempty {
            Err(Error::SessionsRequired)
        } else {
            Ok(Self(()))
        }
    }
    /// Stronger passive header witness: libc++ list {prev,next,size}; empty
    /// links must both point at the actual global sentinel.
    pub fn from_header(sentinel: u64, prev: u64, next: u64, count: u64) -> Result<Self, Error> {
        if sentinel == 0 || prev != sentinel || next != sentinel || count != 0 {
            return Err(Error::SessionsRequired);
        }
        Ok(Self(()))
    }
}

/// Supplied, explicitly attributed pending-state export. Capture preimages and
/// phase attribution are checked by the producer adapter, not authenticated here.
/// LoadedState requires exact equality with its admitted dynamic witness.
#[derive(Debug)]
pub struct PendingStateSnapshot {
    pub witness: crate::loaded_state::ObservedSnapshotWitness,
    pub ever_alive: Vec<PlayerHandle>,
    /// Complete actual 65-entry allocation-slot table, distinct from the ID
    /// table. InitializeLists visits only its first64 entries, in this order.
    pub allocation_slots: Vec<Option<PlayerHandle>>,
    pub open_sessions: EmptyOpenSessions,
    pub pending_actions: Vec<PendingAction>,
}

/// Complete typed session initialization, supplied from actual runtime state.
/// Logical object labels are separate from incoming numeric session IDs.
#[derive(Debug)]
pub struct ObservedSession {
    pub header: SessionHeader,
    pub history: Vec<StatementHandle>,
    pub pending: Vec<aftermath::Pending<StatementHandle>>,
    pub queued: Vec<StatementHandle>,
}
/// Only scalar/other-list metadata is aligned with the canonical pending vector;
/// no second pending-list or statement payload owner is maintained.
#[derive(Debug)]
pub struct SessionMetadata {
    pub header: SessionHeader,
    pub history: Vec<StatementHandle>,
    pub queued: Vec<StatementHandle>,
}
#[derive(Debug)]
pub(crate) struct Routing {
    pub(crate) authoritative: bool,
    pub(crate) metadata: Vec<SessionMetadata>,
    pub(crate) active: Option<SessionHandle>,
    // Representation-only fresh object labels, never native numeric SessionID.
    pub(crate) next_object_label: u64,
}
/// Owned pending state plus optional authoritative session routing metadata.
/// Complete statement payloads and other active subtype effects remain required
/// separately; this owner does not execute native policy or invent decisions.
#[derive(Debug)]
pub struct Manager {
    ever_alive: Vec<PlayerHandle>,
    pub(crate) open_sessions: Vec<Session<StatementHandle>>,
    pending_actions: Vec<PendingAction>,
    pub(crate) routing: Option<Routing>,
    pub(crate) statements: Option<Vec<StatementInfo>>,
    execution_profile: Option<(u32, statement::Logging)>,
    // Exhaustion permanently disables deferred planning instead of aliasing an old stamp.
    pub(crate) drain_revision: Option<u64>,
    pub(crate) drain_owner: u64,
}
impl Manager {
    pub(crate) fn invalidate_drain_plans(&mut self) {
        self.drain_revision = self.drain_revision.and_then(|r| r.checked_add(1));
    }
    /// Install the complete typed statement owner once, before action admission.
    /// Payload kinds are supplied runtime facts, not inferred from numeric IDs.
    pub fn initialize_statement_inventory(
        &mut self,
        facts: Vec<StatementInfo>,
    ) -> Result<(), Error> {
        if self.statements.is_some() {
            return Err(Error::StatementInventoryAlreadyInitialized);
        }
        let metadata = self.session_metadata()?;
        if facts
            .iter()
            .enumerate()
            .any(|(i, f)| facts[..i].iter().any(|g| g.handle == f.handle))
        {
            return Err(Error::InvalidStatementInventory);
        }
        for (session, meta) in self.open_sessions.iter().zip(metadata) {
            for pending in &session.pending {
                if !facts.iter().any(|f| {
                    f.handle == pending.statement && f.from == pending.from && f.to == pending.to
                }) {
                    return Err(Error::InvalidStatementInventory);
                }
            }
            if meta
                .history
                .iter()
                .chain(&meta.queued)
                .any(|h| !facts.iter().any(|f| f.handle == *h))
            {
                return Err(Error::InvalidStatementInventory);
            }
        }
        // A handle cannot have two simultaneous owned list positions.
        let mut handles = Vec::new();
        for (session, meta) in self.open_sessions.iter().zip(metadata) {
            for h in session
                .pending
                .iter()
                .map(|p| p.statement)
                .chain(meta.history.iter().copied())
                .chain(meta.queued.iter().copied())
            {
                if handles.contains(&h) {
                    return Err(Error::InvalidStatementInventory);
                }
                handles.try_reserve(1).map_err(|_| Error::ResourceLimit)?;
                handles.push(h);
            }
        }
        self.invalidate_drain_plans();
        self.statements = Some(facts);
        Ok(())
    }
    pub fn statement_inventory(&self) -> Option<&[StatementInfo]> {
        self.statements.as_deref()
    }
    /// Bind the actual configuration and source-closed logging host once.
    pub fn initialize_execution_profile(
        &mut self,
        game_mode: u32,
        logging: statement::Logging,
    ) -> Result<(), Error> {
        if self.execution_profile.is_some() {
            return Err(Error::ExecutionProfileAlreadyInitialized);
        }
        self.invalidate_drain_plans();
        self.execution_profile = Some((game_mode, logging));
        Ok(())
    }
    pub fn execution_profile(&self) -> Option<(u32, statement::Logging)> {
        self.execution_profile
    }

    fn validate_players(ever_alive: &[PlayerHandle]) -> Result<(), Error> {
        if ever_alive.len() > 64
            || ever_alive.iter().enumerate().any(|(i, p)| {
                !(0..65).contains(&p.owner)
                    || ever_alive[..i].iter().any(|other| other.owner == p.owner)
            })
        {
            return Err(Error::InvalidPlayerInventory);
        }
        Ok(())
    }

    /// Admit supplied state facts after the caller checks their actual capture
    /// and initialized object identities. This performs consistency checking,
    /// not native-execution authentication. No default empty constructor exists.
    pub fn from_observed_empty_sessions(
        _: EmptyOpenSessions,
        ever_alive: Vec<PlayerHandle>,
        pending_actions: Vec<PendingAction>,
    ) -> Result<Self, Error> {
        Self::validate_players(&ever_alive)?;
        Ok(Self {
            ever_alive,
            open_sessions: Vec::new(),
            pending_actions,
            routing: None,
            statements: None,
            execution_profile: None,
            drain_revision: Some(0),
            drain_owner: drain_owner()?,
        })
    }
    /// Initialize authoritative routing from an explicitly supplied runtime
    /// snapshot. Active may identify a retained object outside the open list;
    /// the supported single-player branch compares that identity without
    /// dereferencing its unmodeled payload. No empty/none globals are inferred.
    /// next_object_label is the importer's fresh logical-label cursor above all
    /// reserved labels, not a native session ID counter or RNG input.
    pub fn from_observed_sessions(
        ever_alive: Vec<PlayerHandle>,
        pending_actions: Vec<PendingAction>,
        sessions: Vec<ObservedSession>,
        active: Option<SessionHandle>,
        next_object_label: u64,
    ) -> Result<Self, Error> {
        Self::validate_players(&ever_alive)?;
        let mut manager = Self {
            ever_alive,
            open_sessions: Vec::new(),
            pending_actions,
            routing: None,
            statements: None,
            execution_profile: None,
            drain_revision: Some(0),
            drain_owner: drain_owner()?,
        };
        let mut metadata = Vec::new();
        metadata
            .try_reserve_exact(sessions.len())
            .map_err(|_| Error::ResourceLimit)?;
        manager
            .open_sessions
            .try_reserve_exact(sessions.len())
            .map_err(|_| Error::ResourceLimit)?;
        if active.is_some_and(|h| h.0 >= next_object_label) {
            return Err(Error::InvalidSessionMetadata);
        }
        for session in sessions {
            if session.header.handle.0 >= next_object_label
                || metadata
                    .iter()
                    .any(|m: &SessionMetadata| m.header.handle == session.header.handle)
            {
                return Err(Error::InvalidSessionMetadata);
            }
            manager.open_sessions.push(Session {
                from: session.header.from,
                to: session.header.to,
                pending: session.pending,
            });
            metadata.push(SessionMetadata {
                header: session.header,
                history: session.history,
                queued: session.queued,
            });
        }
        manager.routing = Some(Routing {
            authoritative: true,
            metadata,
            active,
            next_object_label,
        });
        Ok(manager)
    }
    /// Retained pre-mutation facts for diagnostics only. After unstructured
    /// legacy access these cannot authorize routing or be treated as current.
    pub fn retained_session_metadata(&self) -> Option<&[SessionMetadata]> {
        self.routing.as_ref().map(|r| r.metadata.as_slice())
    }
    pub fn session_metadata(&self) -> Result<&[SessionMetadata], Error> {
        self.routing
            .as_ref()
            .filter(|r| r.authoritative)
            .map(|r| r.metadata.as_slice())
            .ok_or(Error::SessionMetadataRequired)
    }
    pub fn active_session(&self) -> Result<Option<SessionHandle>, Error> {
        self.routing
            .as_ref()
            .filter(|r| r.authoritative)
            .map(|r| r.active)
            .ok_or(Error::SessionMetadataRequired)
    }
    pub fn next_session_object_label(&self) -> Result<u64, Error> {
        self.routing
            .as_ref()
            .filter(|r| r.authoritative)
            .map(|r| r.next_object_label)
            .ok_or(Error::SessionMetadataRequired)
    }
    fn validate_owned_statement(
        &self,
        context: Context,
        incoming: Option<StatementInfo>,
        retired: Option<StatementHandle>,
    ) -> Result<(), statement::Error> {
        if incoming.is_some()
            && self
                .execution_profile
                .is_some_and(|p| p != (context.game_mode, context.logging))
        {
            return Err(statement::Error::Conflict);
        }
        let Some(facts) = &self.statements else {
            return Ok(());
        };
        if incoming.is_some_and(|incoming| !facts.contains(&incoming)) {
            return Err(statement::Error::Conflict);
        }
        if let Some(retired) = retired {
            let fact = facts
                .iter()
                .find(|f| f.handle == retired)
                .ok_or(statement::Error::Conflict)?;
            if fact.kind != statement::StatementKind::Response {
                return Err(statement::Error::Unsupported(
                    statement::Unsupported::PendingDestructor(retired),
                ));
            }
        }
        Ok(())
    }
    fn retire_owned_statement(&mut self, retired: Option<StatementHandle>) {
        if let (Some(facts), Some(retired)) = (&mut self.statements, retired) {
            facts.retain(|f| f.handle != retired);
        }
    }
    /// Readonly computed routing/creation plan at the POSToutgoing-removal seam.
    /// Context configuration/logging and payload facts still require the same
    /// actual World revision and effect-closed binding as the statement module.
    pub fn plan_statement(
        &self,
        context: Context,
        incoming: Option<StatementInfo>,
        facts: &[StatementInfo],
    ) -> Result<statement::Plan, statement::Error> {
        self.validate_owned_statement(context, incoming, None)?;
        let facts = self.statements.as_deref().unwrap_or(facts);
        let no_session_read = incoming.is_none()
            || context.game_mode != statement::SINGLE_PLAYER_MODE
            || incoming
                .is_some_and(|s| s.from != context.local_player && s.to != context.local_player);
        if no_session_read {
            return statement::plan(context, incoming, &[], facts);
        }
        let routing = self.routing.as_ref().filter(|r| r.authoritative).ok_or(
            statement::Error::Unsupported(statement::Unsupported::SessionMetadata),
        )?;
        if context.active_session != routing.active {
            return Err(statement::Error::Conflict);
        }
        let mut views = Vec::new();
        views
            .try_reserve_exact(self.open_sessions.len())
            .map_err(|_| statement::Error::ResourceLimit)?;
        for (pending, metadata) in self.open_sessions.iter().zip(&routing.metadata) {
            views.push(SessionView {
                header: metadata.header,
                history: &metadata.history,
                pending: &pending.pending,
                queued: &metadata.queued,
            });
        }
        statement::plan_with_creation(
            context,
            incoming,
            &views,
            facts,
            SessionHandle(routing.next_object_label),
        )
    }
    /// Apply only after the common World revision guard and all other action
    /// preflights pass. Open-order/identity/current pending facts are checked
    /// again here. An initialized canonical statement inventory retires ownership
    /// in the same commit; legacy external owners must apply returned retirement.
    /// This alone is not a complete action transaction.
    pub fn apply_statement(
        &mut self,
        plan: statement::Plan,
        mut context: Context,
        incoming: Option<StatementInfo>,
    ) -> Result<statement::Outcome, statement::Error> {
        self.validate_owned_statement(context, incoming, plan.retired_statement())?;
        self.invalidate_drain_plans();
        let Some(target) = plan.target() else {
            plan.validate(context, incoming, None)?;
            let outcome = plan.commit(&mut context, None);
            self.retire_owned_statement(outcome.retired_statement);
            return Ok(outcome);
        };
        let routing = self.routing.as_mut().filter(|r| r.authoritative).ok_or(
            statement::Error::Unsupported(statement::Unsupported::SessionMetadata),
        )?;
        if context.active_session != routing.active {
            return Err(statement::Error::Conflict);
        }
        let existing = routing
            .metadata
            .iter()
            .position(|m| m.header.id == target.id);
        if plan.creates_session() {
            if existing.is_some()
                || routing.next_object_label != target.handle.0
                || routing.active == Some(target.handle)
                || routing
                    .metadata
                    .iter()
                    .any(|m| m.header.handle == target.handle)
            {
                return Err(statement::Error::Conflict);
            }
            let next = routing
                .next_object_label
                .checked_add(1)
                .ok_or(statement::Error::ResourceLimit)?;
            let mut pending = Session {
                from: target.from,
                to: target.to,
                pending: Vec::new(),
            };
            let mut metadata = SessionMetadata {
                header: target,
                history: Vec::new(),
                queued: Vec::new(),
            };
            plan.validate(
                context,
                incoming,
                Some(SessionView {
                    header: target,
                    history: &metadata.history,
                    pending: &pending.pending,
                    queued: &metadata.queued,
                }),
            )?;
            plan.reserve(&mut SessionWrite {
                header: target,
                history: &mut metadata.history,
                pending: &mut pending.pending,
                queued: &mut metadata.queued,
            })?;
            self.open_sessions
                .try_reserve(1)
                .map_err(|_| statement::Error::ResourceLimit)?;
            routing
                .metadata
                .try_reserve(1)
                .map_err(|_| statement::Error::ResourceLimit)?;
            // All failure paths are above. Native creation sets copied scalar
            // fields and appends ownership before activation/history/queue.
            routing.next_object_label = next;
            self.open_sessions.push(pending);
            routing.metadata.push(metadata);
        } else if existing.is_none_or(|i| routing.metadata[i].header != target) {
            return Err(statement::Error::Conflict);
        }
        let index = if plan.creates_session() {
            self.open_sessions.len() - 1
        } else {
            existing.expect("validated existing session")
        };
        let pending = &mut self.open_sessions[index];
        let metadata = &mut routing.metadata[index];
        if !plan.creates_session() {
            plan.validate(
                context,
                incoming,
                Some(SessionView {
                    header: metadata.header,
                    history: &metadata.history,
                    pending: &pending.pending,
                    queued: &metadata.queued,
                }),
            )?;
            plan.reserve(&mut SessionWrite {
                header: metadata.header,
                history: &mut metadata.history,
                pending: &mut pending.pending,
                queued: &mut metadata.queued,
            })?;
        }
        if plan.activates_session() {
            routing.active = Some(target.handle);
        }
        let outcome = plan.commit(
            &mut context,
            Some(SessionWrite {
                header: metadata.header,
                history: &mut metadata.history,
                pending: &mut pending.pending,
                queued: &mut metadata.queued,
            }),
        );
        self.retire_owned_statement(outcome.retired_statement);
        Ok(outcome)
    }
    pub fn ever_alive(&self) -> &[PlayerHandle] {
        &self.ever_alive
    }
    pub fn copy_ever_alive(&self) -> Result<Vec<PlayerHandle>, Error> {
        let mut out = Vec::new();
        out.try_reserve_exact(self.ever_alive.len())
            .map_err(|_| Error::ResourceLimit)?;
        out.extend_from_slice(&self.ever_alive);
        Ok(out)
    }
    pub fn open_sessions(&self) -> &[Session<StatementHandle>] {
        &self.open_sessions
    }
    /// A real statement provider owns each handle's typed payload and full
    /// routing/history/effects. Merely inserting a handle does not implement it.
    pub fn open_sessions_mut(&mut self) -> &mut Vec<Session<StatementHandle>> {
        self.invalidate_drain_plans();
        // Unstructured mutation can change identity/order/endpoints as well as
        // pending lists. Never silently retain stale routing authority.
        if let Some(routing) = self.routing.as_mut() {
            routing.authoritative = false;
        }
        &mut self.open_sessions
    }
    pub fn pending_actions(&self) -> &[PendingAction] {
        &self.pending_actions
    }
    pub fn pending_actions_mut(&mut self) -> &mut Vec<PendingAction> {
        self.invalidate_drain_plans();
        &mut self.pending_actions
    }
    pub fn has_outgoing(&self, player: i32) -> bool {
        self.open_sessions.iter().any(|session| {
            (session.from == player || session.to == player)
                && session.pending.iter().any(|pending| pending.from == player)
        })
    }
    pub fn remove_next_outgoing(&mut self, player: i32) -> Option<StatementHandle> {
        self.invalidate_drain_plans();
        self.open_sessions
            .iter_mut()
            .filter(|session| session.from == player || session.to == player)
            .find_map(|session| aftermath::remove_next(session, player, -1))
    }
}

/// Exact ProcessStatement is intentionally mandatory, including actual session
/// routing/history, subtype rules, effects/RNG and external decisions. A missing
/// implementation must return an explicit unsupported/decision dependency.
pub trait ProcessStatement<W> {
    fn process_statement(
        &mut self,
        world: &mut W,
        statement: StatementHandle,
    ) -> Result<(), aftermath::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use aftermath::Pending;
    #[test]
    fn unavailable_or_nonempty_observation_never_becomes_empty() {
        assert_eq!(
            EmptyOpenSessions::from_has_pending_session_statements(true),
            Err(Error::SessionsRequired)
        );
        for (prev, next, count) in [(10, 10, 1), (0, 10, 0), (10, 0, 0)] {
            assert!(EmptyOpenSessions::from_header(10, prev, next, count).is_err());
        }
        assert!(EmptyOpenSessions::from_header(10, 10, 10, 0).is_ok());
    }
    #[test]
    fn ordered_handles_and_actions_are_owned_without_defaults() {
        let players = vec![PlayerHandle { owner: 63 }, PlayerHandle { owner: 0 }];
        let actions = vec![PendingAction {
            id: 1,
            from: 0,
            to: 63,
        }];
        let state = Manager::from_observed_empty_sessions(
            EmptyOpenSessions::from_has_pending_session_statements(false).unwrap(),
            players.clone(),
            actions.clone(),
        )
        .unwrap();
        assert_eq!(state.copy_ever_alive().unwrap(), players);
        assert_eq!(state.pending_actions(), actions);
        assert!(state.open_sessions().is_empty());
    }
    #[test]
    fn removal_uses_current_session_head_and_keeps_other_players() {
        let mut state = Manager::from_observed_empty_sessions(
            EmptyOpenSessions::from_has_pending_session_statements(false).unwrap(),
            vec![],
            vec![],
        )
        .unwrap();
        state.open_sessions_mut().extend([
            Session {
                from: 2,
                to: 1,
                pending: vec![
                    Pending {
                        from: 2,
                        to: 1,
                        statement: StatementHandle(9),
                    },
                    Pending {
                        from: 1,
                        to: 2,
                        statement: StatementHandle(10),
                    },
                ],
            },
            Session {
                from: 1,
                to: 3,
                pending: vec![Pending {
                    from: 1,
                    to: 3,
                    statement: StatementHandle(11),
                }],
            },
        ]);
        assert_eq!(state.remove_next_outgoing(1), Some(StatementHandle(10)));
        assert_eq!(state.remove_next_outgoing(1), Some(StatementHandle(11)));
        assert_eq!(state.remove_next_outgoing(1), None);
        assert_eq!(state.remove_next_outgoing(2), Some(StatementHandle(9)));
    }
}

#[cfg(test)]
mod routing_tests {
    use super::*;
    use statement::{Effect, Logging, StatementKind, SINGLE_PLAYER_MODE};
    fn context(active: Option<u64>) -> Context {
        Context {
            local_player: 0,
            game_mode: SINGLE_PLAYER_MODE,
            active_session: active.map(SessionHandle),
            logging: Logging::OrdinaryHostDisabled,
        }
    }
    fn info(handle: u64, id: u32, kind: StatementKind) -> StatementInfo {
        StatementInfo {
            handle: StatementHandle(handle),
            session_id: id,
            from: 1,
            to: 0,
            kind,
        }
    }
    fn observed(handle: u64, id: u32) -> ObservedSession {
        ObservedSession {
            header: SessionHeader {
                handle: SessionHandle(handle),
                id,
                from: 1,
                to: 0,
            },
            history: vec![],
            pending: vec![],
            queued: vec![],
        }
    }
    #[test]
    fn creation_copies_numeric_id_and_repeated_id_reuses_object_without_allocating() {
        let mut manager =
            Manager::from_observed_sessions(vec![], vec![], vec![], None, 100).unwrap();
        let incoming = info(900, u32::MAX, StatementKind::Response);
        let p = manager
            .plan_statement(context(None), Some(incoming), &[])
            .unwrap();
        assert!(p.creates_session());
        assert_eq!(p.target().unwrap().id, u32::MAX);
        assert!(manager.open_sessions().is_empty());
        assert_eq!(manager.next_session_object_label().unwrap(), 100);
        let out = manager
            .apply_statement(p, context(None), Some(incoming))
            .unwrap();
        assert!(out.native_result);
        assert_eq!(out.retired_statement, None);
        assert_eq!(manager.active_session().unwrap(), Some(SessionHandle(100)));
        assert_eq!(manager.next_session_object_label().unwrap(), 101);
        let metadata = manager.session_metadata().unwrap();
        assert_eq!(
            metadata[0].header,
            SessionHeader {
                handle: SessionHandle(100),
                id: u32::MAX,
                from: 1,
                to: 0
            }
        );
        assert_eq!(metadata[0].history, vec![incoming.handle]);
        assert!(manager.open_sessions()[0].pending.is_empty());
        assert!(metadata[0].queued.is_empty());
        let next = info(901, u32::MAX, StatementKind::Response);
        let p = manager
            .plan_statement(context(Some(100)), Some(next), &[])
            .unwrap();
        assert!(!p.creates_session());
        manager
            .apply_statement(p, context(Some(100)), Some(next))
            .unwrap();
        assert_eq!(manager.open_sessions().len(), 1);
        assert_eq!(manager.next_session_object_label().unwrap(), 101);
        assert_eq!(
            manager.session_metadata().unwrap()[0].history,
            vec![incoming.handle, next.handle]
        );
    }
    #[test]
    fn new_inactive_session_appends_and_queues_unknown_owned_subtype() {
        let mut manager = Manager::from_observed_sessions(
            vec![],
            vec![],
            vec![observed(10, 1)],
            Some(SessionHandle(10)),
            20,
        )
        .unwrap();
        let incoming = info(
            90,
            8,
            StatementKind::Other {
                type_hash: 3,
                subtype_hash: 4,
            },
        );
        let p = manager
            .plan_statement(context(Some(10)), Some(incoming), &[])
            .unwrap();
        assert!(p.creates_session());
        assert_eq!(p.effect(), Effect::Queue);
        manager
            .apply_statement(p, context(Some(10)), Some(incoming))
            .unwrap();
        let meta = manager.session_metadata().unwrap();
        assert_eq!(
            meta.iter().map(|m| m.header.id).collect::<Vec<_>>(),
            vec![1, 8]
        );
        assert!(meta[1].history.is_empty());
        assert_eq!(meta[1].queued, vec![incoming.handle]);
        assert!(manager.open_sessions()[1].pending.is_empty());
        assert_eq!(manager.active_session().unwrap(), Some(SessionHandle(10)));
        assert_eq!(manager.next_session_object_label().unwrap(), 21);
    }
    #[test]
    fn unknown_active_creation_effect_refuses_before_identity_or_owner_mutation() {
        let mut manager =
            Manager::from_observed_sessions(vec![], vec![], vec![], None, 20).unwrap();
        let incoming = info(
            90,
            8,
            StatementKind::Other {
                type_hash: 3,
                subtype_hash: 4,
            },
        );
        assert_eq!(
            manager
                .plan_statement(context(None), Some(incoming), &[])
                .unwrap_err(),
            statement::Error::Unsupported(statement::Unsupported::StatementProcess(incoming.kind))
        );
        assert!(manager.open_sessions().is_empty());
        assert!(manager.session_metadata().unwrap().is_empty());
        assert_eq!(manager.active_session().unwrap(), None);
        assert_eq!(manager.next_session_object_label().unwrap(), 20);
        // Legacy arbitrary mutable access invalidates metadata even if this caller
        // does not edit; it is never an implicit fresh empty routing snapshot.
        manager.open_sessions_mut();
        assert!(matches!(
            manager.session_metadata(),
            Err(Error::SessionMetadataRequired)
        ));
        assert_eq!(
            manager
                .plan_statement(
                    context(None),
                    Some(info(91, 8, StatementKind::Response)),
                    &[]
                )
                .unwrap_err(),
            statement::Error::Unsupported(statement::Unsupported::SessionMetadata)
        );
    }
    #[test]
    fn stale_creation_plan_cannot_append_twice_or_consume_another_label() {
        let mut manager =
            Manager::from_observed_sessions(vec![], vec![], vec![], None, 20).unwrap();
        let first = info(90, 8, StatementKind::Response);
        let second = info(91, 9, StatementKind::Response);
        let stale = manager
            .plan_statement(context(None), Some(first), &[])
            .unwrap();
        let p = manager
            .plan_statement(context(None), Some(second), &[])
            .unwrap();
        manager
            .apply_statement(p, context(None), Some(second))
            .unwrap();
        assert_eq!(
            manager.apply_statement(stale, context(Some(20)), Some(first)),
            Err(statement::Error::Conflict)
        );
        assert_eq!(manager.open_sessions().len(), 1);
        assert_eq!(manager.session_metadata().unwrap()[0].header.id, 9);
        assert_eq!(manager.next_session_object_label().unwrap(), 21);
        assert_eq!(
            manager.session_metadata().unwrap()[0].history,
            vec![second.handle]
        );
    }
    #[test]
    fn stale_creation_cursor_is_checked_when_active_identity_does_not_change() {
        let mut manager = Manager::from_observed_sessions(
            vec![],
            vec![],
            vec![observed(10, 1)],
            Some(SessionHandle(10)),
            20,
        )
        .unwrap();
        let first = info(90, 8, StatementKind::Response);
        let second = info(91, 9, StatementKind::Response);
        let stale = manager
            .plan_statement(context(Some(10)), Some(first), &[])
            .unwrap();
        let p = manager
            .plan_statement(context(Some(10)), Some(second), &[])
            .unwrap();
        manager
            .apply_statement(p, context(Some(10)), Some(second))
            .unwrap();
        assert_eq!(manager.active_session().unwrap(), Some(SessionHandle(10)));
        assert_eq!(
            manager.apply_statement(stale, context(Some(10)), Some(first)),
            Err(statement::Error::Conflict)
        );
        assert_eq!(manager.open_sessions().len(), 2);
        assert_eq!(manager.next_session_object_label().unwrap(), 21);
        assert_eq!(
            manager.session_metadata().unwrap()[1].queued,
            vec![second.handle]
        );
    }
    #[test]
    fn owned_pending_drain_preserves_authoritative_routing_and_one_pending_owner() {
        let first = info(90, 8, StatementKind::Response);
        let second = StatementInfo {
            handle: StatementHandle(91),
            session_id: 8,
            from: 0,
            to: 1,
            kind: StatementKind::Response,
        };
        let mut initial = observed(10, 8);
        initial.pending = vec![
            aftermath::Pending {
                from: 1,
                to: 0,
                statement: first.handle,
            },
            aftermath::Pending {
                from: 0,
                to: 1,
                statement: second.handle,
            },
        ];
        let mut manager =
            Manager::from_observed_sessions(vec![], vec![], vec![initial], None, 20).unwrap();
        assert_eq!(manager.remove_next_outgoing(1), Some(first.handle));
        assert!(manager.session_metadata().is_ok());
        let p = manager
            .plan_statement(context(None), Some(first), &[])
            .unwrap();
        manager
            .apply_statement(p, context(None), Some(first))
            .unwrap();
        assert_eq!(manager.remove_next_outgoing(0), Some(second.handle));
        let p = manager
            .plan_statement(context(Some(10)), Some(second), &[])
            .unwrap();
        manager
            .apply_statement(p, context(Some(10)), Some(second))
            .unwrap();
        assert!(manager.open_sessions()[0].pending.is_empty());
        assert_eq!(
            manager.session_metadata().unwrap()[0].history,
            vec![first.handle, second.handle]
        );
        assert_eq!(manager.next_session_object_label().unwrap(), 20);
    }
    #[test]
    fn legacy_invalidation_retains_history_and_duplicate_numeric_ids_keep_first_match() {
        let mut first = observed(10, 8);
        first.history.push(StatementHandle(80));
        let mut manager = Manager::from_observed_sessions(
            vec![],
            vec![],
            vec![first, observed(11, 8)],
            Some(SessionHandle(11)),
            20,
        )
        .unwrap();
        let incoming = info(90, 8, StatementKind::Response);
        let p = manager
            .plan_statement(context(Some(11)), Some(incoming), &[])
            .unwrap();
        assert!(!p.creates_session());
        assert_eq!(p.target().unwrap().handle, SessionHandle(10));
        assert_eq!(p.effect(), Effect::Queue);
        manager
            .apply_statement(p, context(Some(11)), Some(incoming))
            .unwrap();
        assert_eq!(manager.next_session_object_label().unwrap(), 20);
        assert_eq!(
            manager.session_metadata().unwrap()[0].queued,
            vec![incoming.handle]
        );
        manager.open_sessions_mut();
        assert!(matches!(
            manager.session_metadata(),
            Err(Error::SessionMetadataRequired)
        ));
        let retained = manager.retained_session_metadata().unwrap();
        assert_eq!(retained[0].history, vec![StatementHandle(80)]);
        assert_eq!(retained[0].queued, vec![incoming.handle]);
    }
    #[test]
    fn exhausted_object_labels_only_refuse_actual_creation() {
        let manager =
            Manager::from_observed_sessions(vec![], vec![], vec![], None, u64::MAX).unwrap();
        assert_eq!(
            manager
                .plan_statement(
                    context(None),
                    Some(info(90, 8, StatementKind::Response)),
                    &[]
                )
                .unwrap_err(),
            statement::Error::ResourceLimit
        );
        assert!(manager.open_sessions().is_empty());
        let mut manager = Manager::from_observed_sessions(
            vec![],
            vec![],
            vec![observed(10, 8)],
            Some(SessionHandle(10)),
            u64::MAX,
        )
        .unwrap();
        let incoming = info(90, 8, StatementKind::Response);
        let p = manager
            .plan_statement(context(Some(10)), Some(incoming), &[])
            .unwrap();
        manager
            .apply_statement(p, context(Some(10)), Some(incoming))
            .unwrap();
        assert_eq!(manager.next_session_object_label().unwrap(), u64::MAX);
    }
}

#[cfg(test)]
mod inventory_tests {
    use super::*;
    #[test]
    fn inventory_import_is_complete_unique_and_once_only_before_writes() {
        let fact = StatementInfo {
            handle: StatementHandle(1),
            session_id: 7,
            from: 1,
            to: 0,
            kind: statement::StatementKind::Response,
        };
        let observed = ObservedSession {
            header: SessionHeader {
                handle: SessionHandle(10),
                id: 7,
                from: 1,
                to: 0,
            },
            history: vec![],
            queued: vec![],
            pending: vec![aftermath::Pending {
                statement: fact.handle,
                from: 1,
                to: 0,
            }],
        };
        let mut manager =
            Manager::from_observed_sessions(vec![], vec![], vec![observed], None, 20).unwrap();
        let revision = manager.drain_revision;
        assert_eq!(
            manager.initialize_statement_inventory(vec![]),
            Err(Error::InvalidStatementInventory)
        );
        assert_eq!(
            manager.initialize_statement_inventory(vec![fact, fact]),
            Err(Error::InvalidStatementInventory)
        );
        let wrong = StatementInfo { from: 2, ..fact };
        assert_eq!(
            manager.initialize_statement_inventory(vec![wrong]),
            Err(Error::InvalidStatementInventory)
        );
        assert_eq!(manager.drain_revision, revision);
        assert_eq!(manager.statement_inventory(), None);
        manager.initialize_statement_inventory(vec![fact]).unwrap();
        assert_eq!(
            manager.initialize_statement_inventory(vec![]),
            Err(Error::StatementInventoryAlreadyInitialized)
        );
        assert_eq!(manager.statement_inventory().unwrap(), [fact]);
    }
    #[test]
    fn legacy_statement_apply_uses_owned_facts_and_retires_owned_payload() {
        let first = StatementInfo {
            handle: StatementHandle(1),
            session_id: 7,
            from: 1,
            to: 0,
            kind: statement::StatementKind::Response,
        };
        let second = StatementInfo {
            handle: StatementHandle(2),
            ..first
        };
        let observed = ObservedSession {
            header: SessionHeader {
                handle: SessionHandle(10),
                id: 7,
                from: 1,
                to: 0,
            },
            history: vec![],
            queued: vec![],
            pending: vec![aftermath::Pending {
                statement: second.handle,
                from: 1,
                to: 0,
            }],
        };
        let mut manager =
            Manager::from_observed_sessions(vec![], vec![], vec![observed], None, 20).unwrap();
        manager
            .initialize_statement_inventory(vec![first, second])
            .unwrap();
        let context = Context {
            local_player: 0,
            game_mode: statement::SINGLE_PLAYER_MODE,
            active_session: None,
            logging: statement::Logging::OrdinaryHostDisabled,
        };
        let plan = manager.plan_statement(context, Some(first), &[]).unwrap();
        let result = manager.apply_statement(plan, context, Some(first)).unwrap();
        assert_eq!(result.retired_statement, Some(second.handle));
        assert_eq!(manager.statement_inventory().unwrap(), [first]);
        assert_eq!(
            manager.session_metadata().unwrap()[0].history,
            [first.handle]
        );
    }
}
