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
