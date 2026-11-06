//! Deferred Religious AI pantheon request over the persistent gameplay owners.
use super::Runtime;
use crate::loaded_actions::pantheon_actions as action;
use c6_core::{game_update::Error, loaded_state::LoadedState};

impl<P> Runtime<'_, P> {
    /// Bind the same rules definitions used to execute external founding.
    pub fn bind_pantheon_decisions(
        &mut self,
        state: &LoadedState,
        definitions: action::Definitions,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("pantheon decision binding"))?;
        if self.pantheon_definitions.is_some() {
            return Err(Error::Unsupported("pantheon definitions already bound"));
        }
        self.pantheon_definitions = Some(definitions);
        Ok(())
    }
    pub(crate) fn queue_pantheon(&mut self, state: &LoadedState, owner: i32) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("pantheon queue binding"))?;
        if let Some((_, pending)) = self.pending_pantheons.iter_mut().find(|p| p.0 == owner) {
            *pending = true;
        } else {
            self.pending_pantheons.push((owner, true));
        }
        Ok(())
    }
    pub(crate) fn pending_pantheon(
        &mut self,
        owner: i32,
        initial: Option<bool>,
    ) -> Result<bool, Error> {
        if let Some((_, pending)) = self.pending_pantheons.iter().find(|p| p.0 == owner) {
            return Ok(*pending);
        }
        let pending = initial.ok_or(Error::Unsupported("NPC pending pantheon input"))?;
        self.pending_pantheons.push((owner, pending));
        Ok(pending)
    }
    pub(crate) fn pantheon_options(
        &self,
        state: &LoadedState,
        owner: i32,
    ) -> Result<Vec<i32>, Error> {
        let definitions = self
            .pantheon_definitions
            .as_ref()
            .ok_or(Error::Unsupported("NPC pantheon definitions"))?;
        let current = action::Current {
            capabilities: self
                .social_capabilities
                .as_ref()
                .ok_or(Error::Unsupported("NPC pantheon capabilities"))?,
            players: self
                .player_services
                .as_ref()
                .ok_or(Error::Unsupported("NPC pantheon Player.Manager"))?,
        };
        action::available::<std::convert::Infallible>(state, definitions, &current, owner)
            .map(|choices| choices.into_iter().map(|c| c.native_type).collect())
            .map_err(|_| Error::Unsupported("NPC current legal pantheons"))
    }
    pub(crate) fn selected_pantheon(
        &self,
        state: &LoadedState,
        owner: i32,
    ) -> Result<Option<i32>, Error> {
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Unsupported("NPC current pantheon player"))?;
        if player.religion.pantheon_hash == u32::MAX {
            return Ok(None);
        }
        self.pantheon_definitions
            .as_ref()
            .and_then(|d| d.belief_index(player.religion.pantheon_hash))
            .map(Some)
            .ok_or(Error::Unsupported("NPC selected pantheon identity"))
    }
    pub(crate) fn complete_pantheon(&mut self, owner: i32) -> Result<(), Error> {
        let (_, pending) = self
            .pending_pantheons
            .iter_mut()
            .find(|p| p.0 == owner)
            .ok_or(Error::Unsupported("NPC pantheon request owner"))?;
        if !*pending {
            return Err(Error::Unsupported("NPC pantheon request already completed"));
        }
        *pending = false;
        Ok(())
    }
}
