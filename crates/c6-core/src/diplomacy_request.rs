//! Constructor-bounded FirstMeeting RequestSession ownership transitions.
//!
//! This is not player policy: a human-contact request queues an outgoing
//! statement. Processing it and reaching a legal decision are later operations.
//! The local-sender broadcast and AI/AI immediate Process remain explicit.
use crate::diplomacy_manager::{Manager, SessionMetadata, StatementHandle};
use crate::diplomacy_meeting::FirstMeetingRequest;
use crate::diplomacy_statement::{
    Logging, SessionHandle, SessionHeader, StatementInfo, StatementKind,
};
use crate::unit_operation_aftermath::{Pending, Session};

pub const DIPLOMACY_CAPABILITY: u32 = 0x6b48_8f66;
pub const INITIAL_RESPONSE: u32 = 0x7e9d_1c4a;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Player {
    pub owner: i32,
    pub major: bool,
    pub human: bool,
}
/// Current rules capabilities and player/control facts, not saved defaults or
/// per-request native answers. Supply complete current entries for endpoints.
#[derive(Clone, Copy, Debug)]
pub struct Context<'a> {
    pub capabilities: &'a [u32],
    pub players: &'a [Player],
    pub local_player: i32,
    pub logging: Logging,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    MissingPlayer(i32),
    DuplicatePlayer(i32),
    LoggingBinding,
    ImmediateStatementProcess,
    LocalSenderBroadcast,
    SessionMetadata,
    StatementInventory,
    Conflict,
    ResourceLimit,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Facts {
    capability: bool,
    from: Option<Player>,
    to: Option<Player>,
    local: i32,
    logging: Logging,
}
fn facts(c: Context<'_>, r: FirstMeetingRequest) -> Result<Facts, Error> {
    let player = |owner| {
        let mut found = c.players.iter().filter(|p| p.owner == owner);
        let first = *found.next().ok_or(Error::MissingPlayer(owner))?;
        if found.next().is_some() {
            return Err(Error::DuplicatePlayer(owner));
        }
        Ok(first)
    };
    let capability = c.capabilities.contains(&DIPLOMACY_CAPABILITY);
    let from = if capability {
        Some(player(r.from)?)
    } else {
        None
    };
    let to = if from.is_some_and(|p| p.major && !p.human) {
        Some(player(r.to)?)
    } else {
        None
    };
    Ok(Facts {
        capability,
        from,
        to,
        local: c.local_player,
        logging: c.logging,
    })
}
/// Native signed comparison chooses the larger endpoint's low16 as low word.
/// Arithmetic truncates exactly, including negative/out-of-range enum values.
pub fn make_id(from: i32, to: i32) -> u32 {
    let (low, high) = if from >= to { (from, to) } else { (to, from) };
    ((high as u32) << 16) | (low as u32 & 0xffff)
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Effect {
    Unconsumed,
    Duplicate,
    Queue(SessionHeader),
}
#[derive(Debug)]
pub struct Plan {
    owner: u64,
    revision: u64,
    request: FirstMeetingRequest,
    handle: StatementHandle,
    facts: Facts,
    effect: Effect,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    /// Native false return does not destroy the incoming object on these early
    /// guards. The caller retains this typed payload; it is not silently retired.
    Unconsumed {
        handle: StatementHandle,
        request: FirstMeetingRequest,
    },
    /// FindOpenSession duplicate destroys the ordinary FirstMeeting object.
    DuplicateDestroyed(StatementHandle),
    Queued {
        session: SessionHandle,
        statement: StatementHandle,
    },
}
impl Manager {
    /// Readonly plan. The enclosing action must additionally guard the common
    /// World revision for current capability/control/local-player facts. The
    /// caller supplies a fresh logical statement identity, never a reused native
    /// pointer label or a handle already retired by another owner.
    pub fn plan_first_meeting_request(
        &self,
        request: FirstMeetingRequest,
        handle: StatementHandle,
        c: Context<'_>,
    ) -> Result<Plan, Error> {
        let f = facts(c, request)?;
        let revision = self.drain_revision.ok_or(Error::Conflict)?;
        let mut effect = Effect::Unconsumed;
        if f.capability && f.from.is_some_and(|p| p.major) {
            if !f.logging.supports_diplomacy_output() {
                return Err(Error::LoggingBinding);
            }
            if !f.from.is_some_and(|p| p.human) && !f.to.is_some_and(|p| p.human) {
                return Err(Error::ImmediateStatementProcess);
            }
            let metadata = self
                .session_metadata()
                .map_err(|_| Error::SessionMetadata)?;
            let inventory = self.statements.as_ref().ok_or(Error::StatementInventory)?;
            if inventory.iter().any(|s| s.handle == handle) {
                return Err(Error::StatementInventory);
            }
            let id = make_id(request.from, request.to);
            effect = if metadata.iter().any(|m| m.header.id == id) {
                Effect::Duplicate
            } else {
                if request.from == f.local {
                    return Err(Error::LocalSenderBroadcast);
                }
                let label = self
                    .next_session_object_label()
                    .map_err(|_| Error::SessionMetadata)?;
                label.checked_add(1).ok_or(Error::ResourceLimit)?;
                Effect::Queue(SessionHeader {
                    handle: SessionHandle(label),
                    id,
                    from: request.from,
                    to: request.to,
                })
            };
        }
        Ok(Plan {
            owner: self.drain_owner,
            revision,
            request,
            handle,
            facts: f,
            effect,
        })
    }
    /// All fallible checks/reserves precede logical writes. This does not replace
    /// the enclosing action's joint World guard or its owned commit protocol.
    pub fn apply_first_meeting_request(
        &mut self,
        p: Plan,
        c: Context<'_>,
    ) -> Result<Outcome, Error> {
        if self.drain_owner != p.owner
            || self.drain_revision != Some(p.revision)
            || facts(c, p.request)? != p.facts
        {
            return Err(Error::Conflict);
        }
        match p.effect {
            Effect::Unconsumed => Ok(Outcome::Unconsumed {
                handle: p.handle,
                request: p.request,
            }),
            Effect::Duplicate => {
                // Constructor-bounded FirstMeeting destructor calls empty base
                // destructor and ordinary free, with no payload/report/RNG effect.
                self.invalidate_drain_plans();
                Ok(Outcome::DuplicateDestroyed(p.handle))
            }
            Effect::Queue(header) => {
                let routing = self
                    .routing
                    .as_mut()
                    .filter(|r| r.authoritative)
                    .ok_or(Error::Conflict)?;
                let statements = self.statements.as_mut().ok_or(Error::Conflict)?;
                let mut session = Session {
                    from: header.from,
                    to: header.to,
                    pending: Vec::new(),
                };
                session
                    .pending
                    .try_reserve(1)
                    .map_err(|_| Error::ResourceLimit)?;
                self.open_sessions
                    .try_reserve(1)
                    .map_err(|_| Error::ResourceLimit)?;
                routing
                    .metadata
                    .try_reserve(1)
                    .map_err(|_| Error::ResourceLimit)?;
                statements
                    .try_reserve(1)
                    .map_err(|_| Error::ResourceLimit)?;
                let next = header.handle.0.checked_add(1).ok_or(Error::ResourceLimit)?;
                // Native append open-session ownership precedes Session.Add.
                self.open_sessions.push(session);
                routing.metadata.push(SessionMetadata {
                    header,
                    history: Vec::new(),
                    queued: Vec::new(),
                });
                routing.next_object_label = next;
                // RequestSession defaults zero response, then Add sets sessionID
                // and missing initiator before appending the outgoing pointer.
                statements.push(StatementInfo {
                    handle: p.handle,
                    session_id: header.id,
                    from: p.request.from,
                    to: p.request.to,
                    kind: StatementKind::FirstMeeting {
                        type_hash: p.request.statement_type,
                        initiator: header.from,
                        response_type: INITIAL_RESPONSE,
                        hospitality_city: p.request.hospitality_city,
                    },
                });
                self.open_sessions
                    .last_mut()
                    .expect("just appended")
                    .pending
                    .push(Pending {
                        from: p.request.from,
                        to: p.request.to,
                        statement: p.handle,
                    });
                // RequestSession never activates this newly created session.
                self.invalidate_drain_plans();
                Ok(Outcome::Queued {
                    session: header.handle,
                    statement: p.handle,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diplomacy_manager::{ObservedSession, PendingAction};
    use crate::unit::PlotCoord;
    const PLAYERS: [Player; 2] = [
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
    ];
    fn context() -> Context<'static> {
        Context {
            capabilities: &[DIPLOMACY_CAPABILITY],
            players: &PLAYERS,
            local_player: 0,
            logging: Logging::OrdinaryHostDisabled,
        }
    }
    fn request() -> FirstMeetingRequest {
        FirstMeetingRequest {
            statement_type: crate::diplomacy_meeting::SECOND_CITY_MEETING,
            from: 1,
            to: 0,
            hospitality_city: PlotCoord { x: 7, y: 9 },
        }
    }
    fn manager() -> Manager {
        let mut m = Manager::from_observed_sessions(
            vec![],
            vec![PendingAction {
                id: 9,
                from: 1,
                to: 0,
            }],
            vec![],
            None,
            20,
        )
        .unwrap();
        m.initialize_statement_inventory(vec![]).unwrap();
        m
    }
    #[test]
    fn queues_owned_payload_without_activation_then_destroys_duplicate() {
        let mut m = manager();
        let plan = m
            .plan_first_meeting_request(request(), StatementHandle(100), context())
            .unwrap();
        assert!(m.open_sessions().is_empty());
        assert_eq!(
            m.apply_first_meeting_request(plan, context()),
            Ok(Outcome::Queued {
                session: SessionHandle(20),
                statement: StatementHandle(100)
            })
        );
        assert_eq!(m.active_session().unwrap(), None);
        assert_eq!(m.next_session_object_label().unwrap(), 21);
        assert_eq!(
            m.open_sessions()[0].pending,
            [Pending {
                from: 1,
                to: 0,
                statement: StatementHandle(100)
            }]
        );
        let info = m.statement_inventory().unwrap()[0];
        assert_eq!(info.session_id, 1);
        assert_eq!(
            info.kind,
            StatementKind::FirstMeeting {
                type_hash: request().statement_type,
                initiator: 1,
                response_type: INITIAL_RESPONSE,
                hospitality_city: request().hospitality_city
            }
        );
        let p = m
            .plan_first_meeting_request(request(), StatementHandle(101), context())
            .unwrap();
        assert_eq!(
            m.apply_first_meeting_request(p, context()),
            Ok(Outcome::DuplicateDestroyed(StatementHandle(101)))
        );
        assert_eq!(m.open_sessions().len(), 1);
        assert_eq!(m.statement_inventory().unwrap().len(), 1);
        assert_eq!(m.pending_actions().unwrap().len(), 1);
    }
    #[test]
    fn duplicate_numeric_id_preserves_existing_identity_order_and_active() {
        let h = SessionHeader {
            handle: SessionHandle(9),
            id: make_id(0, 1),
            from: 0,
            to: 1,
        };
        let mut m = Manager::from_observed_sessions(
            vec![],
            vec![],
            vec![ObservedSession {
                header: h,
                history: vec![],
                pending: vec![],
                queued: vec![],
            }],
            Some(h.handle),
            20,
        )
        .unwrap();
        m.initialize_statement_inventory(vec![]).unwrap();
        let p = m
            .plan_first_meeting_request(request(), StatementHandle(88), context())
            .unwrap();
        assert_eq!(
            m.apply_first_meeting_request(p, context()),
            Ok(Outcome::DuplicateDestroyed(StatementHandle(88)))
        );
        assert_eq!(m.session_metadata().unwrap()[0].header, h);
        assert_eq!(m.active_session().unwrap(), Some(h.handle));
        assert_eq!(m.next_session_object_label().unwrap(), 20);
    }
    #[test]
    fn stale_plan_and_changed_world_inputs_are_rejected_without_mutation() {
        let mut m = manager();
        let stale = m
            .plan_first_meeting_request(request(), StatementHandle(10), context())
            .unwrap();
        let other = m
            .plan_first_meeting_request(request(), StatementHandle(11), context())
            .unwrap();
        m.apply_first_meeting_request(other, context()).unwrap();
        assert_eq!(
            m.apply_first_meeting_request(stale, context()),
            Err(Error::Conflict)
        );
        let p = m
            .plan_first_meeting_request(request(), StatementHandle(12), context())
            .unwrap();
        let mut c = context();
        c.local_player = 1;
        assert_eq!(m.apply_first_meeting_request(p, c), Err(Error::Conflict));
        assert_eq!(m.statement_inventory().unwrap().len(), 1);
    }
    #[test]
    fn local_broadcast_and_ai_immediate_processing_are_not_silently_omitted() {
        let m = manager();
        let mut c = context();
        c.local_player = 1;
        assert!(matches!(
            m.plan_first_meeting_request(request(), StatementHandle(10), c),
            Err(Error::LocalSenderBroadcast)
        ));
        let players = [
            PLAYERS[0],
            Player {
                human: false,
                ..PLAYERS[1]
            },
        ];
        c = context();
        c.players = &players;
        assert!(matches!(
            m.plan_first_meeting_request(request(), StatementHandle(10), c),
            Err(Error::ImmediateStatementProcess)
        ));
        assert!(m.open_sessions().is_empty());
        assert!(m.statement_inventory().unwrap().is_empty());
    }
    #[test]
    fn response_processing_retires_the_same_canonical_first_meeting_payload() {
        use crate::diplomacy_statement as statement;
        let response = StatementInfo {
            handle: StatementHandle(200),
            session_id: 1,
            from: 1,
            to: 0,
            kind: StatementKind::Response,
        };
        let mut m = Manager::from_observed_sessions(vec![], vec![], vec![], None, 20).unwrap();
        m.initialize_statement_inventory(vec![response]).unwrap();
        let p = m
            .plan_first_meeting_request(request(), StatementHandle(100), context())
            .unwrap();
        m.apply_first_meeting_request(p, context()).unwrap();
        let c = statement::Context {
            local_player: 0,
            game_mode: statement::SINGLE_PLAYER_MODE,
            active_session: None,
            logging: Logging::OrdinaryHostDisabled,
        };
        let p = m
            .plan_statement(c, Some(response), m.statement_inventory().unwrap())
            .unwrap();
        let outcome = m.apply_statement(p, c, Some(response)).unwrap();
        assert_eq!(outcome.retired_statement, Some(StatementHandle(100)));
        assert_eq!(m.statement_inventory().unwrap(), [response]);
        assert!(m.open_sessions()[0].pending.is_empty());
        assert_eq!(m.session_metadata().unwrap()[0].history, [response.handle]);
    }
    #[test]
    fn early_refusal_keeps_incoming_ownership_and_signed_id_formula() {
        let mut m = manager();
        let mut c = context();
        c.capabilities = &[];
        c.players = &[]; // capability guard never resolves a player.
        let p = m
            .plan_first_meeting_request(request(), StatementHandle(10), c)
            .unwrap();
        assert_eq!(
            m.apply_first_meeting_request(p, c),
            Ok(Outcome::Unconsumed {
                handle: StatementHandle(10),
                request: request()
            })
        );
        assert!(m.open_sessions().is_empty());
        assert_eq!(make_id(1, 0), 1);
        assert_eq!(make_id(0, 1), 1);
        assert_eq!(make_id(-1, 2), 0xffff0002);
        assert_eq!(make_id(0x123456, 0x123457), 0x34563457);
    }
}
