//! Owned state for the Linux process-global Diplomacy::Manager pending drain.
//! This owner is distinct from Game.DiplomaticRelations. Statement execution and
//! external player decisions are required providers, never implicit AI policy.
use crate::unit_operation_aftermath::{self as aftermath, Session};

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
    pub open_sessions: EmptyOpenSessions,
    pub pending_actions: Vec<PendingAction>,
}

/// Only state actually required by ProcessPending is modeled. Closed-session
/// history, active-session routing and complete Statement payloads remain
/// separate required state for other diplomacy methods.
#[derive(Debug)]
pub struct Manager {
    ever_alive: Vec<PlayerHandle>,
    open_sessions: Vec<Session<StatementHandle>>,
    pending_actions: Vec<PendingAction>,
}
impl Manager {
    /// Admit supplied state facts after the caller checks their actual capture
    /// and initialized object identities. This performs consistency checking,
    /// not native-execution authentication. No default empty constructor exists.
    pub fn from_observed_empty_sessions(
        _: EmptyOpenSessions,
        ever_alive: Vec<PlayerHandle>,
        pending_actions: Vec<PendingAction>,
    ) -> Result<Self, Error> {
        if ever_alive.len() > 64
            || ever_alive.iter().enumerate().any(|(i, p)| {
                !(0..64).contains(&p.owner)
                    || ever_alive[..i].iter().any(|other| other.owner == p.owner)
            })
        {
            return Err(Error::InvalidPlayerInventory);
        }
        Ok(Self {
            ever_alive,
            open_sessions: Vec::new(),
            pending_actions,
        })
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
        &mut self.open_sessions
    }
    pub fn pending_actions(&self) -> &[PendingAction] {
        &self.pending_actions
    }
    pub fn pending_actions_mut(&mut self) -> &mut Vec<PendingAction> {
        &mut self.pending_actions
    }
    pub fn remove_next_outgoing(&mut self, player: i32) -> Option<StatementHandle> {
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
