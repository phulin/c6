//! Deferred, bounded outgoing `ProcessPending` composition over canonical state.
//!
//! Copies the ever-alive handles once, samples each supplied current slot status
//! once, and rescans the current open-session head after each statement. Only
//! affected sessions receive overlays. PendingAction entries are not processed.
//! This supports the statement module's bounded effects, not full diplomacy or
//! complete action/turn parity. All facts still require initialized-state admission.
use crate::diplomacy_manager::{
    self as manager, Manager, PlayerHandle, SessionMetadata, StatementHandle,
};
use crate::diplomacy_statement::{
    self as statement, Context, SessionHandle, SessionView, SessionWrite,
};
use crate::unit_operation_aftermath::Session;

#[derive(Clone, Copy, Debug)]
pub struct Inputs<'a> {
    pub context: Option<Context>,
    /// Current slot getter results, in copied ever-alive order. Status 3 is human.
    pub slot_statuses: &'a [(PlayerHandle, i32)],
}
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Manager(manager::Error),
    Statement(statement::Error),
    PlayerStatuses,
    InventoryRequired,
    ExecutionProfileRequired,
    ContextRequired,
    StatementMetadata(StatementHandle),
    Conflict,
    ResourceLimit,
}
impl From<statement::Error> for Error {
    fn from(value: statement::Error) -> Self {
        Self::Statement(value)
    }
}
impl From<manager::Error> for Error {
    fn from(value: manager::Error) -> Self {
        Self::Manager(value)
    }
}
#[derive(Debug)]
struct Overlay {
    index: usize,
    session: Session<StatementHandle>,
    metadata: SessionMetadata,
}
/// Owned affected-session delta. Planning never changes canonical gameplay state.
#[derive(Debug)]
pub struct Plan {
    owner: u64,
    revision: u64,
    context: Option<Context>,
    overlays: Vec<Overlay>,
    retired: Vec<StatementHandle>,
    final_routing: Option<(Option<SessionHandle>, u64)>,
}
fn copy<T: Clone>(source: &[T]) -> Result<Vec<T>, Error> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(source.len())
        .map_err(|_| Error::ResourceLimit)?;
    output.extend_from_slice(source);
    Ok(output)
}
fn views<'a>(manager: &'a Manager, overlays: &'a [Overlay]) -> Result<Vec<SessionView<'a>>, Error> {
    let metadata = manager.session_metadata()?;
    let created = overlays
        .iter()
        .filter(|o| o.index >= manager.open_sessions.len())
        .count();
    let mut result = Vec::new();
    result
        .try_reserve_exact(metadata.len() + created)
        .map_err(|_| Error::ResourceLimit)?;
    for index in 0..metadata.len() + created {
        let (session, metadata) = if let Some(o) = overlays.iter().find(|o| o.index == index) {
            (&o.session, &o.metadata)
        } else {
            (&manager.open_sessions[index], &metadata[index])
        };
        result.push(SessionView {
            header: metadata.header,
            history: &metadata.history,
            pending: &session.pending,
            queued: &metadata.queued,
        });
    }
    Ok(result)
}
fn touch(manager: &Manager, overlays: &mut Vec<Overlay>, index: usize) -> Result<usize, Error> {
    if let Some(i) = overlays.iter().position(|o| o.index == index) {
        return Ok(i);
    }
    let session = &manager.open_sessions[index];
    let metadata = &manager.session_metadata()?[index];
    let overlay = Overlay {
        index,
        session: Session {
            from: session.from,
            to: session.to,
            pending: copy(&session.pending)?,
        },
        metadata: SessionMetadata {
            header: metadata.header,
            history: copy(&metadata.history)?,
            queued: copy(&metadata.queued)?,
        },
    };
    overlays.try_reserve(1).map_err(|_| Error::ResourceLimit)?;
    overlays.push(overlay);
    Ok(overlays.len() - 1)
}
/// Compute sequential statement effects, removing each outgoing entry BEFORE
/// calling the existing post-removal statement planner. A later refusal discards
/// every earlier overlay, including session creation and logical retirements.
pub fn plan(manager: &Manager, inputs: Inputs<'_>) -> Result<Plan, Error> {
    let players = manager.copy_ever_alive()?;
    if players.len() != inputs.slot_statuses.len()
        || players
            .iter()
            .zip(inputs.slot_statuses)
            .any(|(a, (b, _))| a != b)
    {
        return Err(Error::PlayerStatuses);
    }
    let mut plan = Plan {
        owner: manager.drain_owner,
        revision: manager.drain_revision.ok_or(Error::ResourceLimit)?,
        context: inputs.context,
        overlays: Vec::new(),
        retired: Vec::new(),
        final_routing: None,
    };
    let mut context_option = inputs.context;
    let mut cursor = None;
    for (player, (_, status)) in players.iter().zip(inputs.slot_statuses) {
        // IsAI is sampled once for this handle, outside its outgoing loop.
        if *status == 3 {
            continue;
        }
        loop {
            // Empty legacy snapshots have no session metadata dependency.
            if plan.overlays.is_empty() && !manager.has_outgoing(player.owner) {
                break;
            }
            let current = views(manager, &plan.overlays)?;
            let Some((source, pending_index)) = current
                .iter()
                .enumerate()
                .filter(|(_, s)| s.header.from == player.owner || s.header.to == player.owner)
                .find_map(|(i, s)| {
                    s.pending
                        .iter()
                        .position(|p| p.from == player.owner)
                        .map(|j| (i, j))
                })
            else {
                break;
            };
            let outgoing = current[source].pending[pending_index].clone();
            drop(current);
            let mut context = context_option.ok_or(Error::ContextRequired)?;
            let profile = manager
                .execution_profile()
                .ok_or(Error::ExecutionProfileRequired)?;
            if profile != (context.game_mode, context.logging) {
                return Err(Error::Conflict);
            }
            let inventory = manager
                .statement_inventory()
                .ok_or(Error::InventoryRequired)?;
            let incoming = inventory
                .iter()
                .find(|f| f.handle == outgoing.statement)
                .filter(|f| {
                    !plan.retired.contains(&f.handle)
                        && f.from == outgoing.from
                        && f.to == outgoing.to
                })
                .copied()
                .ok_or(Error::StatementMetadata(outgoing.statement))?;
            if cursor.is_none() {
                if manager.active_session()? != context.active_session {
                    return Err(Error::Conflict);
                }
                cursor = Some(manager.next_session_object_label()?);
            }
            let source_overlay = touch(manager, &mut plan.overlays, source)?;
            plan.overlays[source_overlay]
                .session
                .pending
                .remove(pending_index);
            let current = views(manager, &plan.overlays)?;
            let leaf = statement::plan_with_creation(
                context,
                Some(incoming),
                &current,
                inventory,
                SessionHandle(cursor.expect("initialized routing cursor")),
            )?;
            let target = leaf.target();
            let creation = leaf.creates_session();
            drop(current);
            let target_index = if let Some(header) = target {
                if creation {
                    let index = manager.open_sessions.len()
                        + plan
                            .overlays
                            .iter()
                            .filter(|o| o.index >= manager.open_sessions.len())
                            .count();
                    plan.overlays
                        .try_reserve(1)
                        .map_err(|_| Error::ResourceLimit)?;
                    plan.overlays.push(Overlay {
                        index,
                        session: Session {
                            from: header.from,
                            to: header.to,
                            pending: Vec::new(),
                        },
                        metadata: SessionMetadata {
                            header,
                            history: Vec::new(),
                            queued: Vec::new(),
                        },
                    });
                    cursor = Some(header.handle.0.checked_add(1).ok_or(Error::ResourceLimit)?);
                    Some(plan.overlays.len() - 1)
                } else {
                    let current = views(manager, &plan.overlays)?;
                    let index = current
                        .iter()
                        .position(|s| s.header.handle == header.handle)
                        .ok_or(Error::Conflict)?;
                    drop(current);
                    Some(touch(manager, &mut plan.overlays, index)?)
                }
            } else {
                None
            };
            let outcome = if let Some(index) = target_index {
                let overlay = &mut plan.overlays[index];
                leaf.validate(
                    context,
                    Some(incoming),
                    Some(SessionView {
                        header: overlay.metadata.header,
                        history: &overlay.metadata.history,
                        pending: &overlay.session.pending,
                        queued: &overlay.metadata.queued,
                    }),
                )?;
                leaf.reserve(&mut SessionWrite {
                    header: overlay.metadata.header,
                    history: &mut overlay.metadata.history,
                    pending: &mut overlay.session.pending,
                    queued: &mut overlay.metadata.queued,
                })?;
                leaf.commit(
                    &mut context,
                    Some(SessionWrite {
                        header: overlay.metadata.header,
                        history: &mut overlay.metadata.history,
                        pending: &mut overlay.session.pending,
                        queued: &mut overlay.metadata.queued,
                    }),
                )
            } else {
                leaf.validate(context, Some(incoming), None)?;
                leaf.commit(&mut context, None)
            };
            context_option = Some(context);
            if let Some(retired) = outcome.retired_statement {
                // Unknown alias ownership is outside this finite profile. Never
                // retire the payload while a surviving list still refers to it.
                if views(manager, &plan.overlays)?.iter().any(|s| {
                    s.pending.iter().any(|p| p.statement == retired)
                        || s.history.contains(&retired)
                        || s.queued.contains(&retired)
                }) {
                    return Err(Error::Conflict);
                }
                if plan.retired.contains(&retired) {
                    return Err(Error::Conflict);
                }
                plan.retired
                    .try_reserve(1)
                    .map_err(|_| Error::ResourceLimit)?;
                plan.retired.push(retired);
            }
        }
    }
    if let Some(cursor) = cursor {
        plan.final_routing = Some((
            context_option
                .expect("reached statement context")
                .active_session,
            cursor,
        ));
    }
    Ok(plan)
}
impl Plan {
    pub fn context(&self) -> Option<Context> {
        self.context
    }
    /// Validate all canonical read facts through the manager mutation stamp and
    /// reserve append capacity before the enclosing action's common commit guard.
    /// The returned exclusive borrow prevents any intervening manager mutation.
    pub fn prepare(
        self,
        manager: &mut Manager,
        context: Option<Context>,
    ) -> Result<Prepared<'_>, Error> {
        if manager.drain_owner != self.owner
            || manager.drain_revision != Some(self.revision)
            || context != self.context
        {
            return Err(Error::Conflict);
        }
        if self.final_routing.is_some()
            && manager.active_session()? != context.ok_or(Error::ContextRequired)?.active_session
        {
            return Err(Error::Conflict);
        }
        let created = self
            .overlays
            .iter()
            .filter(|o| o.index >= manager.open_sessions.len())
            .count();
        manager
            .open_sessions
            .try_reserve(created)
            .map_err(|_| Error::ResourceLimit)?;
        if created != 0 {
            manager
                .routing
                .as_mut()
                .ok_or(Error::Conflict)?
                .metadata
                .try_reserve(created)
                .map_err(|_| Error::ResourceLimit)?;
        }
        Ok(Prepared {
            manager,
            plan: self,
        })
    }
}
/// A leaf of the enclosing atomic action guard. No fallible work or callbacks
/// remain; inventory retirement and canonical session replacement commit jointly.
pub struct Prepared<'a> {
    manager: &'a mut Manager,
    plan: Plan,
}
impl Prepared<'_> {
    pub fn commit(self) {
        if self.plan.overlays.is_empty() {
            return;
        }
        let manager = self.manager;
        let initial_count = manager.open_sessions.len();
        let routing = manager.routing.as_mut().expect("prepared routing");
        // Move created entries into already reserved canonical positions in
        // ascending creation order.
        // Existing overlays may be touched in arbitrary source/target order.
        for overlay in self.plan.overlays {
            if overlay.index < initial_count {
                manager.open_sessions[overlay.index] = overlay.session;
                routing.metadata[overlay.index] = overlay.metadata;
            } else {
                // Created overlays are appended in creation order in the overlay
                // vector and retain that order even when subsequently touched.
                debug_assert_eq!(overlay.index, manager.open_sessions.len());
                manager.open_sessions.push(overlay.session);
                routing.metadata.push(overlay.metadata);
            }
        }
        if let Some((active, cursor)) = self.plan.final_routing {
            routing.active = active;
            routing.next_object_label = cursor;
        }
        if !self.plan.retired.is_empty() {
            manager
                .statements
                .as_mut()
                .expect("prepared statement inventory")
                .retain(|f| !self.plan.retired.contains(&f.handle));
        }
        manager.invalidate_drain_plans();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit_operation_aftermath::Pending;
    use manager::{ObservedSession, PendingAction};
    use statement::{Logging, SessionHeader, StatementInfo, StatementKind, SINGLE_PLAYER_MODE};

    fn context(active: Option<u64>) -> Context {
        Context {
            local_player: 0,
            game_mode: SINGLE_PLAYER_MODE,
            logging: Logging::OrdinaryHostDisabled,
            active_session: active.map(SessionHandle),
        }
    }
    fn info(handle: u64, id: u32, from: i32, to: i32, kind: StatementKind) -> StatementInfo {
        StatementInfo {
            handle: StatementHandle(handle),
            session_id: id,
            from,
            to,
            kind,
        }
    }
    fn initialized(facts: &[StatementInfo], active: Option<u64>) -> Manager {
        let session = ObservedSession {
            header: SessionHeader {
                handle: SessionHandle(10),
                id: 7,
                from: 1,
                to: 0,
            },
            history: vec![],
            queued: vec![],
            pending: facts
                .iter()
                .map(|f| Pending {
                    from: f.from,
                    to: f.to,
                    statement: f.handle,
                })
                .collect(),
        };
        let mut manager = Manager::from_observed_sessions(
            vec![PlayerHandle { owner: 1 }],
            vec![PendingAction {
                id: 99,
                from: 1,
                to: 0,
            }],
            vec![session],
            active.map(SessionHandle),
            20,
        )
        .unwrap();
        manager
            .initialize_statement_inventory(facts.to_vec())
            .unwrap();
        manager
            .initialize_execution_profile(SINGLE_PLAYER_MODE, Logging::OrdinaryHostDisabled)
            .unwrap();
        manager
    }
    fn inputs(active: Option<u64>) -> Inputs<'static> {
        Inputs {
            context: Some(context(active)),
            slot_statuses: &[(PlayerHandle { owner: 1 }, 1)],
        }
    }
    fn apply(manager: &mut Manager, active: Option<u64>) {
        let inputs = inputs(active);
        let p = plan(manager, inputs).unwrap();
        p.prepare(manager, inputs.context).unwrap().commit();
    }
    #[test]
    fn repeated_rescans_use_post_removal_pending_and_retire_inventory_atomically() {
        let facts: Vec<_> = (1..=4)
            .map(|h| info(h, 7, 1, 0, StatementKind::Response))
            .collect();
        let mut manager = initialized(&facts, None);
        let p = plan(&manager, inputs(None)).unwrap();
        assert_eq!(manager.open_sessions()[0].pending.len(), 4);
        assert_eq!(manager.statement_inventory().unwrap(), facts);
        p.prepare(&mut manager, Some(context(None)))
            .unwrap()
            .commit();
        assert!(manager.open_sessions()[0].pending.is_empty());
        assert_eq!(
            manager.session_metadata().unwrap()[0].history,
            [StatementHandle(1), StatementHandle(3)]
        );
        assert_eq!(manager.statement_inventory().unwrap(), [facts[0], facts[2]]);
        assert_eq!(manager.active_session().unwrap(), Some(SessionHandle(10)));
        assert_eq!(
            manager.pending_actions().unwrap(),
            [PendingAction {
                id: 99,
                from: 1,
                to: 0
            }]
        );
    }
    #[test]
    fn later_refusal_keeps_earlier_discard_and_all_canonical_lists_uncommitted() {
        let other = StatementKind::Other {
            type_hash: 1,
            subtype_hash: 2,
        };
        let facts = [
            info(1, 7, 1, 2, StatementKind::Response),
            info(2, 7, 1, 0, other),
        ];
        let manager = initialized(&facts, None);
        let before = format!("{manager:?}");
        assert_eq!(
            plan(&manager, inputs(None)).unwrap_err(),
            Error::Statement(statement::Error::Unsupported(
                statement::Unsupported::StatementProcess(other)
            ))
        );
        assert_eq!(format!("{manager:?}"), before);
    }
    #[test]
    fn new_response_copies_id_and_later_inactive_unknown_kind_queues() {
        let facts = [
            info(1, u32::MAX, 1, 0, StatementKind::Response),
            info(
                2,
                8,
                1,
                0,
                StatementKind::Other {
                    type_hash: 1,
                    subtype_hash: 2,
                },
            ),
        ];
        let mut manager = initialized(&facts, None);
        apply(&mut manager, None);
        assert_eq!(manager.next_session_object_label().unwrap(), 22);
        assert_eq!(manager.active_session().unwrap(), Some(SessionHandle(20)));
        let metadata = manager.session_metadata().unwrap();
        assert_eq!(
            metadata.iter().map(|m| m.header.id).collect::<Vec<_>>(),
            [7, u32::MAX, 8]
        );
        assert_eq!(metadata[1].history, [facts[0].handle]);
        assert_eq!(metadata[2].queued, [facts[1].handle]);
        assert!(manager.open_sessions().iter().all(|s| s.pending.is_empty()));
        assert_eq!(manager.statement_inventory().unwrap(), facts);
    }
    #[test]
    fn remote_response_discards_without_creating_session_and_human_status_skips() {
        let facts = [info(1, 8, 1, 2, StatementKind::Response)];
        let mut manager = initialized(&facts, None);
        let human = Inputs {
            context: None,
            slot_statuses: &[(PlayerHandle { owner: 1 }, 3)],
        };
        plan(&manager, human)
            .unwrap()
            .prepare(&mut manager, None)
            .unwrap()
            .commit();
        assert_eq!(manager.open_sessions()[0].pending.len(), 1);
        apply(&mut manager, None);
        assert_eq!(manager.open_sessions().len(), 1);
        assert_eq!(manager.next_session_object_label().unwrap(), 20);
        assert!(manager.statement_inventory().unwrap().is_empty());
        assert!(manager.open_sessions()[0].pending.is_empty());
    }
    #[test]
    fn stale_manager_stamp_and_profile_mismatch_refuse_before_commit() {
        let facts = [info(1, 7, 1, 0, StatementKind::Response)];
        let mut manager = initialized(&facts, None);
        let p = plan(&manager, inputs(None)).unwrap();
        manager.pending_actions_mut().unwrap().push(PendingAction {
            id: 4,
            from: 0,
            to: 1,
        });
        assert!(matches!(
            p.prepare(&mut manager, Some(context(None))),
            Err(Error::Conflict)
        ));
        assert_eq!(manager.open_sessions()[0].pending.len(), 1);
        let mut wrong = inputs(None);
        wrong.context.as_mut().unwrap().game_mode = 0;
        assert_eq!(plan(&manager, wrong).unwrap_err(), Error::Conflict);
        assert_eq!(manager.statement_inventory().unwrap(), facts);
    }
    #[test]
    fn unknown_context_is_allowed_only_when_source_drain_reaches_no_statement() {
        let manager = Manager::from_observed_empty_sessions(
            manager::EmptyOpenSessions::from_has_pending_session_statements(false).unwrap(),
            vec![PlayerHandle { owner: 1 }],
            vec![],
        )
        .unwrap();
        assert!(plan(
            &manager,
            Inputs {
                context: None,
                slot_statuses: inputs(None).slot_statuses
            }
        )
        .is_ok());
        let facts = [info(1, 7, 1, 0, StatementKind::Response)];
        let manager = initialized(&facts, None);
        assert_eq!(
            plan(
                &manager,
                Inputs {
                    context: None,
                    slot_statuses: inputs(None).slot_statuses
                }
            )
            .unwrap_err(),
            Error::ContextRequired
        );
    }
    #[test]
    fn manager_identity_rejects_identical_other_owner_and_survives_move() {
        let facts = [info(1, 7, 1, 0, StatementKind::Response)];
        let first = initialized(&facts, None);
        let mut other = initialized(&facts, None);
        let p = plan(&first, inputs(None)).unwrap();
        assert_eq!(first.drain_revision, other.drain_revision);
        assert!(matches!(
            p.prepare(&mut other, Some(context(None))),
            Err(Error::Conflict)
        ));
        assert_eq!(other.open_sessions()[0].pending.len(), 1);
        let p = plan(&first, inputs(None)).unwrap();
        let mut moved = Box::new(first);
        p.prepare(&mut moved, Some(context(None))).unwrap().commit();
        assert!(moved.open_sessions()[0].pending.is_empty());
        assert_eq!(
            moved.session_metadata().unwrap()[0].history,
            [facts[0].handle]
        );
    }
    #[test]
    fn revision_exhaustion_never_revives_old_plans() {
        let facts = [info(1, 7, 1, 0, StatementKind::Response)];
        let mut manager = initialized(&facts, None);
        manager.drain_revision = Some(u64::MAX);
        let p = plan(&manager, inputs(None)).unwrap();
        manager.pending_actions_mut().unwrap();
        assert_eq!(manager.drain_revision, None);
        assert!(matches!(
            p.prepare(&mut manager, Some(context(None))),
            Err(Error::Conflict)
        ));
        assert_eq!(
            plan(&manager, inputs(None)).unwrap_err(),
            Error::ResourceLimit
        );
        manager.pending_actions_mut().unwrap();
        assert_eq!(manager.drain_revision, None);
        assert_eq!(manager.open_sessions()[0].pending.len(), 1);
    }
    #[test]
    fn surviving_alias_refuses_retirement_without_committing_prior_leaf_changes() {
        let facts = [
            info(1, 7, 1, 0, StatementKind::Response),
            info(2, 7, 1, 0, StatementKind::Response),
        ];
        let mut manager = initialized(&facts, None);
        // Simulate an unsupported retained alias beyond the importer's unique
        // list-owner profile. Planning must still refuse instead of dangling it.
        manager.routing.as_mut().unwrap().metadata[0]
            .history
            .push(facts[1].handle);
        let before = format!("{manager:?}");
        assert_eq!(plan(&manager, inputs(None)).unwrap_err(), Error::Conflict);
        assert_eq!(format!("{manager:?}"), before);
    }
}
