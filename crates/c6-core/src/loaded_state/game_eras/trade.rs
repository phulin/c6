//! Canonical current PlayerScores route-completion rate at native +0x44.
use super::{AccessError, Error, LoadedState, ObservedSnapshotWitness, WireBinding};
impl LoadedState {
    pub fn initialize_game_era_trade_completion_rates(
        &mut self,
        binding: &WireBinding,
        source: Option<&ObservedSnapshotWitness>,
        values: Vec<i32>,
    ) -> Result<(), AccessError> {
        if !self.is_pristine() {
            return Err(AccessError::State(Error::NotPristine));
        }
        if binding != &self.binding
            || source.is_some_and(|s| self.observed_snapshot.as_ref() != Some(s))
        {
            return Err(AccessError::State(Error::Binding));
        }
        let runtime = self.game_eras()?;
        if runtime.trade_completion_rates.is_some() {
            return Err(AccessError::State(Error::AlreadyBound));
        }
        if runtime
            .current_scores
            .as_ref()
            .is_some_and(|rows| rows.len() != values.len())
        {
            return Err(AccessError::State(Error::Binding));
        }
        if values.len() > self.limits.max_runtime_array_items {
            return Err(AccessError::State(Error::ResourceLimit));
        }
        self.advance_admission_generation()
            .map_err(AccessError::State)?;
        self.game_eras
            .as_mut()
            .expect("validated Eras")
            .trade_completion_rates = Some(values);
        Ok(())
    }
    pub fn game_era_trade_completion_rate(&self, owner: i32) -> Result<Option<i32>, AccessError> {
        if owner == -1 {
            return Ok(None);
        }
        let owner = usize::try_from(owner).map_err(|_| AccessError::InvalidPlayer(owner))?;
        self.game_eras()?
            .trade_completion_rates
            .as_ref()
            .ok_or(AccessError::Unavailable(
                "current Era trade completion rates",
            ))
            .map(|values| values.get(owner).copied())
    }
}
