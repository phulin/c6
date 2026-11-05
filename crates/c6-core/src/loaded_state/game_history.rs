//! Current Game.History owner. Fixture adapters may supply a typed current
//! manager directly; unavailable history is never replaced by an empty list.
use super::{Error, LoadedState, ObservedSnapshotWitness, WireBinding};
use crate::game_history::{Manager, Moment};

#[derive(Debug)]
pub enum AccessError {
    State(Error),
    Unavailable,
}
impl std::fmt::Display for AccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "current Game.History: {self:?}")
    }
}
impl std::error::Error for AccessError {}
impl LoadedState {
    pub fn initialize_game_history(
        &mut self,
        binding: &WireBinding,
        witness: &ObservedSnapshotWitness,
        history: Manager,
    ) -> Result<(), AccessError> {
        if !self.is_pristine() {
            return Err(AccessError::State(Error::NotPristine));
        }
        if binding != &self.binding || self.observed_snapshot.as_ref() != Some(witness) {
            return Err(AccessError::State(Error::Binding));
        }
        if self.game_history.is_some() {
            return Err(AccessError::State(Error::AlreadyBound));
        }
        if history.moments.len() > self.limits.max_runtime_array_items {
            return Err(AccessError::State(Error::ResourceLimit));
        }
        self.advance_admission_generation()
            .map_err(AccessError::State)?;
        self.game_history = Some(history);
        Ok(())
    }
    pub fn game_history(&self) -> Result<&Manager, AccessError> {
        self.game_history.as_ref().ok_or(AccessError::Unavailable)
    }
    /// HandleMoment assigns the old independent counter as ID, increments it
    /// with native wrapping, and appends before dispatching MomentRecorded.
    /// The caller must first compute admission and all current Moment fields.
    pub fn record_historic_moment(&mut self, mut moment: Moment) -> Result<i32, AccessError> {
        let history = self.game_history()?;
        if history.moments.len() >= self.limits.max_runtime_array_items {
            return Err(AccessError::State(Error::ResourceLimit));
        }
        let id = history.total_moments_recorded;
        let next = self
            .revision
            .checked_add(1)
            .ok_or(AccessError::State(Error::ResourceLimit))?;
        moment.id = id;
        let history = self.game_history.as_mut().expect("checked current History");
        history.total_moments_recorded = id.wrapping_add(1);
        history.moments.push(moment);
        self.revision = next;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn moment(id: i32) -> Moment {
        Moment {
            wire_version: crate::game_history::MOMENT_VERSION,
            type_hash: 7,
            turn: 4,
            action_plot: crate::unit::PlotCoord { x: 1, y: 2 },
            acting_player_ordinal: 0,
            id,
            has_ever_been_commemorated_raw: 0,
            game_era_hash: u32::MAX,
            extra_data: vec![],
            city_name_bytes: vec![],
            unit_citizen_name_bytes: vec![],
            other_city_name_bytes: vec![],
            era_score: 0,
        }
    }
    #[test]
    fn independent_counter_assigns_old_id_then_wraps_without_inferring_list_length() {
        let mut state = crate::loaded_state::tests::state();
        state.game_history = Some(Manager {
            wire_version: crate::game_history::VERSION,
            moments: vec![moment(19)],
            total_moments_recorded: i32::MAX,
        });
        assert_eq!(state.record_historic_moment(moment(-1)).unwrap(), i32::MAX);
        assert_eq!(state.record_historic_moment(moment(-1)).unwrap(), i32::MIN);
        let current = state.game_history().unwrap();
        assert_eq!(current.total_moments_recorded, i32::MIN + 1);
        assert_eq!(
            current.moments.iter().map(|m| m.id).collect::<Vec<_>>(),
            [19, i32::MAX, i32::MIN]
        );
    }
}
