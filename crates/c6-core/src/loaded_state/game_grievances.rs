//! Same-before refinement of the canonical current diplomatic owner.
use super::*;
impl LoadedState {
    pub fn initialize_current_grievances(
        &mut self,
        witness: &ObservedSnapshotWitness,
        amounts: Vec<(i32, i32)>,
        holders: Vec<(i32, i32)>,
    ) -> Result<(), Error> {
        if !self.is_pristine() {
            return Err(Error::NotPristine);
        }
        if self.observed_snapshot.as_ref() != Some(witness) {
            return Err(Error::Binding);
        }
        let owner = self.game_relations.as_mut().ok_or(Error::Binding)?;
        owner
            .initialize_current_grievances(amounts, holders)
            .map_err(|_| Error::Binding)?;
        self.advance_admission_generation()
    }
    /// Install the same observed initial tension and grievance log without
    /// treating fixture initialization as a gameplay action.
    pub fn initialize_current_war_consequences(
        &mut self,
        witness: &ObservedSnapshotWitness,
        tension: Vec<(i32, i32)>,
        log: Vec<crate::game_diplomatic_relations::war::GrievanceLogItem>,
    ) -> Result<(), Error> {
        if !self.is_pristine() {
            return Err(Error::NotPristine);
        }
        if self.observed_snapshot.as_ref() != Some(witness) {
            return Err(Error::Binding);
        }
        self.game_relations.as_mut().ok_or(Error::Binding)?
            .initialize_current_war_consequences(tension, log)
            .map_err(|_| Error::Binding)?;
        self.advance_admission_generation()
    }

}
