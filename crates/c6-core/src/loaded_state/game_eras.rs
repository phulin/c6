//! Current global Game.Eras domains shared by gameplay queries. Player.Eras
//! hashes are separate. Absent domains remain unknown rather than empty arrays.
pub mod history;
use super::{Error, LoadedState, ObservedSnapshotWitness, WireBinding};

#[derive(Debug)]
pub struct Inputs {
    pub golden_ages_raw: Option<Vec<u8>>,
    pub dark_ages_raw: Option<Vec<u8>>,
    pub current_era: Option<i32>,
    pub era_start_turn: Option<i32>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CombatBonuses {
    pub defeated_corps: i32,
    pub defeated_army: i32,
    pub defeated_naval: i32,
    pub defeated_giant_death_robot: i32,
}
#[derive(Debug)]
pub struct Runtime {
    inputs: Inputs,
    combat_bonuses: Option<Vec<CombatBonuses>>,
    current_scores: Option<Vec<Vec<i32>>>,
    pride: Option<(Vec<i32>, Vec<i32>)>,
    construction_rates: Option<Vec<[i32; 4]>>,
}
impl Runtime {
    pub fn golden_ages_raw(&self) -> Option<&[u8]> {
        self.inputs.golden_ages_raw.as_deref()
    }
    pub fn dark_ages_raw(&self) -> Option<&[u8]> {
        self.inputs.dark_ages_raw.as_deref()
    }
    pub fn current_era(&self) -> Option<i32> {
        self.inputs.current_era
    }
    pub fn era_start_turn(&self) -> Option<i32> {
        self.inputs.era_start_turn
    }
}
#[derive(Debug)]
pub enum AccessError {
    State(Error),
    Unavailable(&'static str),
    InvalidPlayer(i32),
}
impl std::fmt::Display for AccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "current Game.Eras: {self:?}")
    }
}
impl std::error::Error for AccessError {}
fn age(values: Option<&[u8]>, owner: i32, name: &'static str) -> Result<bool, AccessError> {
    if owner == -1 {
        return Ok(false);
    }
    let index = usize::try_from(owner).map_err(|_| AccessError::InvalidPlayer(owner))?;
    let values = values.ok_or(AccessError::Unavailable(name))?;
    // 29966f0/29967a0: out-of-range player returns false; stored bytes use !=0.
    Ok(values.get(index).is_some_and(|&raw| raw != 0))
}
impl LoadedState {
    /// Initial current-state import only. A native source must be this state's
    /// observed epoch; its capture configuration need not equal the save's older
    /// configuration. None denotes model construction, never native evidence.
    pub fn initialize_game_eras(
        &mut self,
        binding: &WireBinding,
        source: Option<&ObservedSnapshotWitness>,
        inputs: Inputs,
    ) -> Result<(), AccessError> {
        if !self.is_pristine() {
            return Err(AccessError::State(Error::NotPristine));
        }
        if binding != &self.binding
            || source.is_some_and(|v| self.observed_snapshot.as_ref() != Some(v))
        {
            return Err(AccessError::State(Error::Binding));
        }
        if self.game_eras.is_some() {
            return Err(AccessError::State(Error::AlreadyBound));
        }
        if inputs
            .golden_ages_raw
            .as_ref()
            .is_some_and(|v| v.len() > self.limits.max_runtime_array_items)
            || inputs
                .dark_ages_raw
                .as_ref()
                .is_some_and(|v| v.len() > self.limits.max_runtime_array_items)
        {
            return Err(AccessError::State(Error::ResourceLimit));
        }
        self.advance_admission_generation()
            .map_err(AccessError::State)?;
        self.game_eras = Some(Runtime {
            inputs,
            combat_bonuses: None,
            current_scores: None,
            pride: None,
            construction_rates: None,
        });
        Ok(())
    }
    pub fn initialize_game_eras_combat_bonuses(
        &mut self,
        binding: &WireBinding,
        source: Option<&ObservedSnapshotWitness>,
        values: Vec<CombatBonuses>,
    ) -> Result<(), AccessError> {
        if !self.is_pristine() {
            return Err(AccessError::State(Error::NotPristine));
        }
        if binding != &self.binding
            || source.is_some_and(|v| self.observed_snapshot.as_ref() != Some(v))
        {
            return Err(AccessError::State(Error::Binding));
        }
        if values.len() > self.limits.max_runtime_array_items {
            return Err(AccessError::State(Error::ResourceLimit));
        }
        let runtime = self
            .game_eras
            .as_ref()
            .ok_or(AccessError::Unavailable("Game.Eras"))?;
        if runtime.combat_bonuses.is_some() {
            return Err(AccessError::State(Error::AlreadyBound));
        }
        self.advance_admission_generation()
            .map_err(AccessError::State)?;
        self.game_eras
            .as_mut()
            .expect("validated current Eras")
            .combat_bonuses = Some(values);
        Ok(())
    }
    /// Native owner==-1/out-of-range guards precede all score-cell reads.
    pub fn game_eras_combat_bonuses(
        &self,
        owner: i32,
    ) -> Result<Option<CombatBonuses>, AccessError> {
        if owner == -1 {
            return Ok(None);
        }
        let index = usize::try_from(owner).map_err(|_| AccessError::InvalidPlayer(owner))?;
        let values = self
            .game_eras()?
            .combat_bonuses
            .as_ref()
            .ok_or(AccessError::Unavailable("Eras combat bonuses"))?;
        Ok(values.get(index).copied())
    }
    pub fn game_eras(&self) -> Result<&Runtime, AccessError> {
        self.game_eras
            .as_ref()
            .ok_or(AccessError::Unavailable("Game.Eras"))
    }
    pub fn player_has_golden_age(&self, owner: i32) -> Result<bool, AccessError> {
        if owner == -1 {
            return Ok(false);
        }
        age(self.game_eras()?.golden_ages_raw(), owner, "golden ages")
    }
    pub fn player_has_dark_age(&self, owner: i32) -> Result<bool, AccessError> {
        if owner == -1 {
            return Ok(false);
        }
        age(self.game_eras()?.dark_ages_raw(), owner, "dark ages")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raw_nonzero_and_native_bounds_remain_distinct_from_unknown() {
        assert!(!age(None, -1, "golden").unwrap());
        assert!(matches!(
            age(None, 0, "golden"),
            Err(AccessError::Unavailable(_))
        ));
        assert!(age(Some(&[0, 2, 255]), 1, "golden").unwrap());
        assert!(age(Some(&[0, 2, 255]), 2, "golden").unwrap());
        assert!(!age(Some(&[0]), 1, "golden").unwrap());
        assert!(matches!(
            age(Some(&[0]), -2, "golden"),
            Err(AccessError::InvalidPlayer(-2))
        ));
    }
    #[test]
    fn canonical_initial_domain_is_single_bind_and_other_domains_stay_missing() {
        let mut state = super::super::tests::state();
        let binding = state.binding().clone();
        let inputs = || Inputs {
            golden_ages_raw: Some(vec![0, 1]),
            dark_ages_raw: None,
            current_era: None,
            era_start_turn: None,
        };
        state
            .initialize_game_eras(&binding, None, inputs())
            .unwrap();
        assert!(state.player_has_golden_age(1).unwrap());
        assert!(matches!(
            state.player_has_dark_age(1),
            Err(AccessError::Unavailable(_))
        ));
        assert!(matches!(
            state.initialize_game_eras(&binding, None, inputs()),
            Err(AccessError::State(Error::AlreadyBound))
        ));
    }
}
