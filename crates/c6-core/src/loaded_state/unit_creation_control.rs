//! OnUnitCreated201dc60 after the active/nonhuman guards, on the SAME manager.
use super::LoadedState;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    MissingControlManager(i32),
    /// The source must SetProcessedTurn(true), then FinishMoves, before waking
    /// this manager. Those callbacks remain required for a zero-priority control.
    RecruiterRequiresUnitContinuation,
    RevisionCapacity,
}
impl LoadedState {
    pub fn schedule_created_unit_control(&mut self, owner: i32) -> Result<(), Error> {
        let manager = self
            .retained
            .players
            .iter()
            .find(|p| p.requested_owner == owner)
            .and_then(|p| p.ai_control.as_ref())
            .ok_or(Error::MissingControlManager(owner))?;
        if manager
            .controls
            .instances
            .iter()
            .any(|c| c.recruit_priority == 0)
        {
            return Err(Error::RecruiterRequiresUnitContinuation);
        }
        if manager.priority == -1 {
            self.revision = self
                .revision
                .checked_add(1)
                .ok_or(Error::RevisionCapacity)?;
            self.retained
                .players
                .iter_mut()
                .find(|p| p.requested_owner == owner)
                .and_then(|p| p.ai_control.as_mut())
                .expect("validated current manager")
                .priority = 0;
        }
        Ok(())
    }
}
