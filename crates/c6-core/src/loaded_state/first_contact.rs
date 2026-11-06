//! Canonical semantic diplomatic-state activation during a supported meeting.
use super::{Error as StateError, LoadedState, MissingInput};
#[derive(Debug)]
pub enum Error {
    State(StateError),
    MissingDiplomatic(i32),
    MissingState { owner: i32, other: i32 },
    Binding,
}
impl LoadedState {
    /// Preserve OnHasMet's StateVector[other.ID].Activate(true). Its evaluator
    /// prefix must already be executed or qualified as pure native planning by
    /// the enclosing current callback provider; this helper does not replace it.
    /// Lookup uses the player ordinal directly, not PlayerToIndex.
    pub fn activate_contact_diplomatic_state(
        &mut self,
        receiver: i32,
        first: i32,
        second: i32,
    ) -> Result<bool, Error> {
        let other = if receiver == first {
            second
        } else if receiver == second {
            first
        } else {
            return Ok(false);
        };
        let index = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == receiver)
            .ok_or(Error::State(StateError::Missing(MissingInput::Player(
                receiver,
            ))))?;
        let target = usize::try_from(other).map_err(|_| Error::MissingState {
            owner: receiver,
            other,
        })?;
        let diplomatic = self.retained.players[index]
            .ai_diplomatic
            .as_ref()
            .ok_or(Error::MissingDiplomatic(receiver))?;
        if diplomatic.player_id != receiver {
            return Err(Error::Binding);
        }
        let state = diplomatic
            .diplomatic_states
            .states
            .get(target)
            .ok_or(Error::MissingState {
                owner: receiver,
                other,
            })?;
        if state.owner != receiver || state.opponent != other {
            return Err(Error::Binding);
        }
        if state.is_active.0 != 0 {
            return Ok(false);
        }
        let next = self
            .revision
            .checked_add(1)
            .ok_or(Error::State(StateError::ResourceLimit))?;
        self.retained.players[index]
            .ai_diplomatic
            .as_mut()
            .expect("checked diplomatic owner")
            .diplomatic_states
            .states[target]
            .is_active
            .0 = 1;
        self.revision = next;
        Ok(true)
    }
}
