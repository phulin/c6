//! Canonical owner for the initialized delegation response's deterministic tail.
use super::{Error as StateError, LoadedState, MissingInput};
use crate::diplomatic_delegation_policy as policy;

#[derive(Debug)]
pub enum Error {
    State(StateError),
    MissingDiplomatic(i32),
    Policy(policy::Error),
}

impl LoadedState {
    /// Validate all reached policy-adjacent writes before exposing the response.
    pub fn validate_delegation_response_policy(
        &self,
        target: i32,
        sender: i32,
        definition_hash: u32,
    ) -> Result<(), Error> {
        let player = self
            .retained
            .players
            .iter()
            .find(|p| p.header.owner == target)
            .ok_or(Error::State(StateError::Missing(MissingInput::Player(
                target,
            ))))?;
        let state = player
            .ai_diplomatic
            .as_ref()
            .ok_or(Error::MissingDiplomatic(target))?;
        policy::validate_initialized_response(state, target, sender, definition_hash)
            .map_err(Error::Policy)?;
        self.revision
            .checked_add(1)
            .ok_or(Error::State(StateError::ResourceLimit))?;
        Ok(())
    }

    /// Preserve ChangePartnerActionClosed(sender, delegation, false), reached
    /// after either external accept or refuse, before action gameplay continues.
    pub fn finish_delegation_response_policy(
        &mut self,
        target: i32,
        sender: i32,
        definition_hash: u32,
    ) -> Result<(), Error> {
        self.validate_delegation_response_policy(target, sender, definition_hash)?;
        let player = self
            .retained
            .players
            .iter_mut()
            .find(|p| p.header.owner == target)
            .expect("validated player");
        policy::finish_initialized_response(
            player.ai_diplomatic.as_mut().expect("validated diplomacy"),
            target,
            sender,
            definition_hash,
        )
        .map_err(Error::Policy)?;
        self.revision += 1;
        Ok(())
    }
}

impl LoadedState {
    pub fn validate_delegation_sender_result(
        &self,
        sender: i32,
        target: i32,
        hash: u32,
    ) -> Result<(), Error> {
        let p = self
            .retained
            .players
            .iter()
            .find(|p| p.header.owner == sender)
            .ok_or(Error::MissingDiplomatic(sender))?;
        let ai = p
            .ai_diplomatic
            .as_ref()
            .ok_or(Error::MissingDiplomatic(sender))?;
        policy::validate_sender_result(ai, sender, target, hash).map_err(Error::Policy)?;
        self.revision
            .checked_add(1)
            .ok_or(Error::State(StateError::ResourceLimit))?;
        Ok(())
    }
    pub fn finish_delegation_sender_result(
        &mut self,
        sender: i32,
        target: i32,
        hash: u32,
    ) -> Result<(), Error> {
        self.validate_delegation_sender_result(sender, target, hash)?;
        let p = self
            .retained
            .players
            .iter_mut()
            .find(|p| p.header.owner == sender)
            .expect("validated player");
        policy::finish_sender_result(
            p.ai_diplomatic.as_mut().expect("validated AI"),
            sender,
            target,
            hash,
        )
        .map_err(Error::Policy)?;
        self.revision += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player_ai_diplomatic::{BoolByte, HashEntry};

    fn ready_state() -> LoadedState {
        let mut state = super::super::tests::state();
        let player = &mut state.retained.players[0];
        assert_eq!(player.header.owner, 0);
        let ai = player.ai_diplomatic.as_mut().unwrap();
        ai.player_id = 0;
        let mut handler = ai.diplo_action_evaluator.handlers[0].clone();
        handler.owner = 0;
        handler.opponent = 1;
        ai.diplo_action_evaluator
            .handlers
            .resize(2, handler.clone());
        ai.diplo_action_evaluator.handlers[1] = handler;
        ai.closed_partner_action.resize(2, vec![]);
        ai.closed_partner_action[1] = vec![HashEntry {
            hash: 17,
            value: BoolByte(1),
        }];
        state
    }

    #[test]
    fn canonical_policy_tail_updates_only_target_cell_and_revision() {
        let mut state = ready_state();
        let before_stamp = state.action_stamp();
        let mut expected = state.retained.players[0].ai_diplomatic.clone().unwrap();
        state.validate_delegation_response_policy(0, 1, 17).unwrap();
        assert_eq!(state.action_stamp(), before_stamp);
        assert_eq!(
            state.retained.players[0].ai_diplomatic.as_ref(),
            Some(&expected)
        );
        expected.closed_partner_action[1][0].value = BoolByte(0);
        state.finish_delegation_response_policy(0, 1, 17).unwrap();
        assert_eq!(
            state.retained.players[0].ai_diplomatic.as_ref(),
            Some(&expected)
        );
        assert_eq!(state.revision(), 1);
        assert_ne!(state.action_stamp(), before_stamp);
        // Accept and refuse share this tail. The native store is unconditional,
        // so repeating the false store still advances canonical action revision.
        state.finish_delegation_response_policy(0, 1, 17).unwrap();
        assert_eq!(
            state.retained.players[0].ai_diplomatic.as_ref(),
            Some(&expected)
        );
        assert_eq!(state.revision(), 2);
    }

    #[test]
    fn rejection_and_revision_exhaustion_preserve_canonical_cell_and_stamp() {
        let mut state = ready_state();
        let before = state.retained.players[0].ai_diplomatic.clone();
        let stamp = state.action_stamp();
        assert!(matches!(
            state.finish_delegation_response_policy(0, 1, 18),
            Err(Error::Policy(policy::Error::MissingDefinition))
        ));
        assert_eq!(state.retained.players[0].ai_diplomatic, before);
        assert_eq!(state.action_stamp(), stamp);
        state.revision = u64::MAX;
        let stamp = state.action_stamp();
        assert!(matches!(
            state.finish_delegation_response_policy(0, 1, 17),
            Err(Error::State(StateError::ResourceLimit))
        ));
        assert_eq!(state.retained.players[0].ai_diplomatic, before);
        assert_eq!(state.action_stamp(), stamp);
    }
}
