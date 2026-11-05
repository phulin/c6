//! Current Eras world-turn fields omitted by the bounded query domains.
use super::{AccessError, Error, LoadedState, ObservedSnapshotWitness, WireBinding};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldTurn {
    pub starting_thresholds_calculated: bool,
    pub next_era_countdown: i32,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn world_turn_uses_single_current_manager_and_retains_countdown_writes() {
        let mut state = super::super::super::tests::state();
        let binding = state.binding().clone();
        state
            .initialize_game_eras(
                &binding,
                None,
                super::super::Inputs {
                    golden_ages_raw: None,
                    dark_ages_raw: None,
                    current_era: Some(0),
                    era_start_turn: Some(0),
                },
            )
            .unwrap();
        let input = WorldTurn {
            starting_thresholds_calculated: true,
            next_era_countdown: 3,
        };
        state
            .initialize_game_eras_world_turn(&binding, None, input)
            .unwrap();
        assert!(matches!(
            state.initialize_game_eras_world_turn(&binding, None, input),
            Err(AccessError::State(Error::AlreadyBound))
        ));
        state.set_game_eras_countdown(2).unwrap();
        assert_eq!(state.game_eras_world_turn().unwrap().next_era_countdown, 2);
        assert_eq!(state.game_eras().unwrap().current_era(), Some(0));
    }
}
impl LoadedState {
    pub fn initialize_game_eras_world_turn(
        &mut self,
        binding: &WireBinding,
        source: Option<&ObservedSnapshotWitness>,
        input: WorldTurn,
    ) -> Result<(), AccessError> {
        if !self.is_pristine() {
            return Err(AccessError::State(Error::NotPristine));
        }
        if binding != &self.binding
            || source.is_some_and(|v| self.observed_snapshot.as_ref() != Some(v))
        {
            return Err(AccessError::State(Error::Binding));
        }
        if self.game_eras()?.world_turn.is_some() {
            return Err(AccessError::State(Error::AlreadyBound));
        }
        self.advance_admission_generation()
            .map_err(AccessError::State)?;
        self.game_eras.as_mut().expect("validated Eras").world_turn = Some(input);
        Ok(())
    }
    pub fn game_eras_world_turn(&self) -> Result<WorldTurn, AccessError> {
        self.game_eras()?
            .world_turn
            .ok_or(AccessError::Unavailable("Eras world-turn fields"))
    }
    pub fn set_game_eras_countdown(&mut self, countdown: i32) -> Result<(), AccessError> {
        self.game_eras_world_turn()?;
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(AccessError::State(Error::ResourceLimit))?;
        self.game_eras
            .as_mut()
            .expect("validated Eras")
            .world_turn
            .as_mut()
            .expect("validated fields")
            .next_era_countdown = countdown;
        Ok(())
    }
}
