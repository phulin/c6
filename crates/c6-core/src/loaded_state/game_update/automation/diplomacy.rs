//! Canonical current diplomacy predicates and DoAutomation completion store.
use super::{index, processed, RequiresDiplomacy};
use crate::{game_update::automation::diplomacy as core, loaded_state::LoadedState};
#[derive(Debug)]
pub enum Error<E> {
    State(crate::loaded_state::Error),
    World(crate::world::MissingInput),
    Prefix(crate::game_update::Error),
    Service(E),
}
/// Admit same-epoch initial pending actions omitted by an earlier partial
/// runtime export. This cannot replace known state or run after gameplay edits.
pub fn initialize_pending_actions(
    state: &mut LoadedState,
    witness: &crate::loaded_state::ObservedSnapshotWitness,
    actions: Vec<crate::diplomacy_manager::PendingAction>,
) -> Result<(), crate::loaded_state::Error> {
    if !state.is_pristine()
        || state.observed_snapshot() != Some(witness)
        || state.diplomacy_manager_source() != Some(witness)
    {
        return Err(crate::loaded_state::Error::Binding);
    }
    if state.diplomacy_manager()?.pending_actions_known().is_some() {
        return Err(crate::loaded_state::Error::DiplomacyManager(
            crate::diplomacy_manager::Error::PendingActionsAlreadyInitialized,
        ));
    }
    state.advance_admission_generation()?;
    state
        .diplomacy_manager
        .as_mut()
        .expect("checked owner")
        .initialize_pending_actions(actions)
        .map_err(crate::loaded_state::Error::DiplomacyManager)
}
pub trait Services {
    type Error;
    fn pending_deal(&mut self, state: &LoadedState, owner: i32) -> Result<bool, Self::Error>;
    fn any_multiplayer(&mut self, state: &LoadedState) -> Result<bool, Self::Error>;
    fn network_multiplayer(&mut self, state: &LoadedState) -> Result<bool, Self::Error>;
    fn reactivation_requested(
        &mut self,
        state: &LoadedState,
        owner: i32,
    ) -> Result<bool, Self::Error>;
    fn request_unready(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        reason: u32,
    ) -> Result<bool, Self::Error>;
    /// Actual SetTurnActive(true), including archive write, log and reporting.
    fn set_active(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Self::Error>;
}
struct Current<'a, S> {
    state: &'a mut LoadedState,
    services: &'a mut S,
}
impl<S: Services> core::Context for Current<'_, S> {
    type Error = Error<S::Error>;
    fn pending(&mut self, owner: i32, q: core::Pending) -> Result<bool, Self::Error> {
        if q == core::Pending::Deal {
            return self
                .services
                .pending_deal(self.state, owner)
                .map_err(Error::Service);
        }
        let manager = self.state.diplomacy_manager().map_err(Error::State)?;
        match q {
            core::Pending::SessionStatements => Ok(manager
                .open_sessions()
                .iter()
                .any(|s| (s.from == owner || s.to == owner) && !s.pending.is_empty())),
            core::Pending::OpenSession => Ok(manager
                .open_sessions()
                .iter()
                .any(|s| s.from == owner || s.to == owner)),
            core::Pending::Action => Ok(manager
                .pending_actions()
                .map_err(|_| {
                    Error::Prefix(crate::game_update::Error::Unsupported(
                        "current pending actions",
                    ))
                })?
                .iter()
                .any(|a| a.from == owner || a.to == owner)),
            core::Pending::AiSessionStatements => {
                for s in manager.open_sessions() {
                    // Native GetControlType is !IsHuman: control1 means AI.
                    for player in [s.from, s.to] {
                        if self
                            .state
                            .world()
                            .player_is_valid(player)
                            .map_err(Error::World)?
                            && self
                                .state
                                .player_slot_status(player)
                                .map_err(Error::State)?
                                != 3
                            && !s.pending.is_empty()
                        {
                            return Ok(true);
                        }
                    }
                }
                Ok(false)
            }
            core::Pending::Deal => unreachable!(),
        }
    }
    fn any_multiplayer(&mut self) -> Result<bool, Self::Error> {
        self.services
            .any_multiplayer(self.state)
            .map_err(Error::Service)
    }
    fn network_multiplayer(&mut self) -> Result<bool, Self::Error> {
        self.services
            .network_multiplayer(self.state)
            .map_err(Error::Service)
    }
    fn reactivation_requested(&mut self, owner: i32) -> Result<bool, Self::Error> {
        self.services
            .reactivation_requested(self.state, owner)
            .map_err(Error::Service)
    }
    fn request_unready(&mut self, owner: i32, reason: u32) -> Result<bool, Self::Error> {
        self.services
            .request_unready(self.state, owner, reason)
            .map_err(Error::Service)
    }
    fn clear_complete(&mut self, owner: i32) -> Result<(), Self::Error> {
        let i = index(owner).map_err(Error::Prefix)?;
        super::revision(self.state).map_err(Error::Prefix)?;
        self.state.retained.core.player_processor.player_complete[i] = false;
        Ok(())
    }
    fn set_active(&mut self, owner: i32) -> Result<(), Self::Error> {
        self.services
            .set_active(self.state, owner)
            .map_err(Error::Service)
    }
    fn complete(&mut self, owner: i32) -> Result<bool, Self::Error> {
        Ok(self.state.retained.core.player_processor.player_complete
            [index(owner).map_err(Error::Prefix)?])
    }
    fn mark_processed(&mut self, owner: i32) -> Result<(), Self::Error> {
        processed(self.state, owner).map_err(Error::Prefix)
    }
}
/// Consumes this exact completed unit pass. Failed callbacks retain prefix
/// mutations and cannot turn a partial Game.Update into whole-update success.
pub fn finish<S: Services>(
    state: &mut LoadedState,
    pending: RequiresDiplomacy,
    services: &mut S,
) -> Result<(), Error<S::Error>> {
    if state.instance_identity != pending.instance || state.action_stamp() != pending.stamp {
        return Err(Error::Prefix(crate::game_update::Error::Unsupported(
            "automation diplomacy belongs to another state",
        )));
    }
    core::finish(
        &mut Current { state, services },
        pending.owner,
        pending.human,
        pending.any_attention,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diplomacy_manager::{EmptyOpenSessions, Manager, PlayerHandle};
    use crate::loaded_state::tests::current_update_player_fixture;
    struct S {
        queries: Vec<&'static str>,
    }
    impl Services for S {
        type Error = ();
        fn pending_deal(&mut self, _: &LoadedState, _: i32) -> Result<bool, ()> {
            self.queries.push("deal");
            Ok(false)
        }
        fn any_multiplayer(&mut self, _: &LoadedState) -> Result<bool, ()> {
            panic!("no pending")
        }
        fn network_multiplayer(&mut self, _: &LoadedState) -> Result<bool, ()> {
            self.queries.push("network");
            Ok(true)
        }
        fn reactivation_requested(&mut self, _: &LoadedState, _: i32) -> Result<bool, ()> {
            panic!("network skips host")
        }
        fn request_unready(&mut self, _: &mut LoadedState, _: i32, _: u32) -> Result<bool, ()> {
            panic!("unreached request")
        }
        fn set_active(&mut self, _: &mut LoadedState, _: i32) -> Result<(), ()> {
            panic!("unreached active")
        }
    }
    #[test]
    fn initial_pending_actions_fill_unknown_once_without_gameplay_revision() {
        let mut state = crate::loaded_state::tests::state();
        let owner = state.world().players()[0].owner();
        let witness = crate::loaded_state::ObservedSnapshotWitness {
            source_role: crate::core_player::initialization::WireRole::SavedOutputCore,
            snapshot_sha256: state.binding().core_sha256,
            snapshot_bytes: state.source_image().len(),
            capture_runtime_sha256: [7; 32],
            capture_facts_sha256: [8; 32],
            capture_configuration_sha256: [9; 32],
        };
        state.observed_snapshot = Some(witness.clone());
        state.diplomacy_manager_source = Some(witness.clone());
        state.diplomacy_manager = Some(
            Manager::from_observed_sessions_with_unknown_pending_actions(
                vec![PlayerHandle { owner }],
                vec![],
                None,
                1,
            )
            .unwrap(),
        );
        let actions = vec![crate::diplomacy_manager::PendingAction {
            id: 42,
            from: owner,
            to: 9,
        }];
        let before = state.action_stamp();
        initialize_pending_actions(&mut state, &witness, actions.clone()).unwrap();
        assert_eq!(state.revision(), 0);
        assert_ne!(state.action_stamp(), before);
        assert_eq!(
            state
                .diplomacy_manager()
                .unwrap()
                .pending_actions()
                .unwrap(),
            actions
        );
        let admitted = state.action_stamp();
        assert!(initialize_pending_actions(&mut state, &witness, vec![]).is_err());
        assert_eq!(state.action_stamp(), admitted);
        super::super::processed(&mut state, owner).unwrap();
        assert!(matches!(
            initialize_pending_actions(&mut state, &witness, vec![]),
            Err(crate::loaded_state::Error::Binding)
        ));
        assert_eq!(
            state
                .diplomacy_manager()
                .unwrap()
                .pending_actions()
                .unwrap(),
            actions
        );
    }
    #[test]
    fn current_empty_manager_and_completed_human_mark_canonical_automation_processed() {
        let (mut state, _, owner) = current_update_player_fixture();
        state.diplomacy_manager = Some(
            Manager::from_observed_empty_sessions(
                EmptyOpenSessions::from_has_pending_session_statements(false).unwrap(),
                vec![PlayerHandle { owner }],
                vec![],
            )
            .unwrap(),
        );
        state.retained.core.player_processor.player_complete[owner as usize] = true;
        let p = RequiresDiplomacy {
            instance: state.instance_identity,
            stamp: state.action_stamp(),
            owner,
            human: true,
            any_attention: false,
        };
        let mut s = S { queries: vec![] };
        finish(&mut state, p, &mut s).unwrap();
        assert!(state.retained.core.player_processor.automation_processed[owner as usize]);
        assert_eq!(s.queries, ["deal", "network"]);
    }
    #[test]
    fn altered_state_rejects_before_any_query() {
        let (mut state, _, owner) = current_update_player_fixture();
        let p = RequiresDiplomacy {
            instance: state.instance_identity,
            stamp: state.action_stamp(),
            owner,
            human: true,
            any_attention: false,
        };
        super::super::processed(&mut state, owner).unwrap();
        let mut s = S { queries: vec![] };
        assert!(matches!(
            finish(&mut state, p, &mut s),
            Err(Error::Prefix(_))
        ));
        assert!(s.queries.is_empty());
    }
}
