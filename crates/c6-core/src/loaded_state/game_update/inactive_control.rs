//! Inactive branch of Linux AI.Control.Manager.DoTurn201eb70.
//! One off-turn signal invocation stores pending BEFORE joining jobs. It never
//! activates a player, drains priorities, changes human roles or declares Ready.
use crate::{game_update::Error, loaded_state::LoadedState};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Complete,
    Pending,
    Yield,
}
/// CollectorOne ORs every nonzero result and stops on status2. Pending alone
/// does not short-circuit later synchronous callbacks.
pub fn collect<E>(statuses: impl IntoIterator<Item = Result<Status, E>>) -> Result<bool, E> {
    let mut pending = false;
    for status in statuses {
        match status? {
            Status::Complete => {}
            Status::Pending => pending = true,
            Status::Yield => return Ok(true),
        }
    }
    Ok(pending)
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Skipped,
    OffTurn { pending: bool },
}
pub trait Effects {
    fn off_turn(&mut self, state: &mut LoadedState, owner: i32) -> Result<bool, Error>;
    fn wait_for_jobs(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Error>;
}
pub fn run<E: Effects>(
    state: &mut LoadedState,
    owner: i32,
    skip_off_turn: bool,
    effects: &mut E,
) -> Result<Outcome, Error> {
    if state
        .turn_active(owner)
        .map_err(|_| Error::Unsupported("inactive control current turn-active"))?
    {
        return Err(Error::Unsupported(
            "active Control.DoTurn requires policy continuation",
        ));
    }
    if skip_off_turn {
        return Ok(Outcome::Skipped);
    }
    let pending = effects.off_turn(state, owner)?;
    let index = state
        .retained
        .players
        .iter()
        .position(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("inactive current Control owner"))?;
    let control = state.retained.players[index]
        .ai_control
        .as_ref()
        .ok_or(Error::Unsupported("inactive current Control.Manager"))?;
    if control.player_id != owner {
        return Err(Error::Unsupported("inactive Control receiver"));
    }
    state.revision = state
        .revision
        .checked_add(1)
        .ok_or(Error::Unsupported("inactive control revision overflow"))?;
    state.retained.players[index]
        .ai_control
        .as_mut()
        .expect("checked")
        .has_out_of_turn_processing = pending;
    effects.wait_for_jobs(state, owner)?;
    Ok(Outcome::OffTurn { pending })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pending_does_not_skip_later_callbacks() {
        let mut seen = 0;
        assert_eq!(
            collect([Status::Pending, Status::Complete].into_iter().map(|s| {
                seen += 1;
                Ok::<_, ()>(s)
            })),
            Ok(true)
        );
        assert_eq!(seen, 2);
    }
    #[test]
    fn yield_stops_before_later_failure() {
        assert_eq!(
            collect([
                Ok(Status::Complete),
                Ok(Status::Yield),
                Err("must not evaluate")
            ]),
            Ok(true)
        );
    }
    #[test]
    fn empty_or_all_complete_is_not_pending() {
        assert_eq!(collect::<()>([]), Ok(false));
        assert_eq!(collect([Ok::<_, ()>(Status::Complete)]), Ok(false));
    }
}
