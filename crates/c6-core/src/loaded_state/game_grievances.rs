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
}
