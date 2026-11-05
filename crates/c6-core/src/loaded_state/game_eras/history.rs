//! Current Game.Eras score cells shared by historic moments and other credits.
use super::{AccessError, Error, LoadedState, ObservedSnapshotWitness, WireBinding};
impl LoadedState {
    pub fn initialize_game_era_scores(
        &mut self,
        binding: &WireBinding,
        source: Option<&ObservedSnapshotWitness>,
        scores: Vec<Vec<i32>>,
    ) -> Result<(), AccessError> {
        if !self.is_pristine() {
            return Err(AccessError::State(Error::NotPristine));
        }
        if binding != &self.binding
            || source.is_some_and(|v| self.observed_snapshot.as_ref() != Some(v))
        {
            return Err(AccessError::State(Error::Binding));
        }
        let runtime = self
            .game_eras
            .as_ref()
            .ok_or(AccessError::Unavailable("Game.Eras"))?;
        if runtime.current_scores.is_some() {
            return Err(AccessError::State(Error::AlreadyBound));
        }
        let count = scores
            .iter()
            .try_fold(scores.len(), |n, row| n.checked_add(row.len()))
            .ok_or(AccessError::State(Error::ResourceLimit))?;
        if count > self.limits.max_runtime_array_items {
            return Err(AccessError::State(Error::ResourceLimit));
        }
        self.advance_admission_generation()
            .map_err(AccessError::State)?;
        self.game_eras
            .as_mut()
            .expect("validated current owner")
            .current_scores = Some(scores);
        Ok(())
    }
    pub fn initialize_game_era_pride(
        &mut self,
        binding: &WireBinding,
        source: Option<&ObservedSnapshotWitness>,
        extra: Vec<i32>,
        minimum: Vec<i32>,
    ) -> Result<(), AccessError> {
        if !self.is_pristine() {
            return Err(AccessError::State(Error::NotPristine));
        }
        if binding != &self.binding
            || source.is_some_and(|v| self.observed_snapshot.as_ref() != Some(v))
        {
            return Err(AccessError::State(Error::Binding));
        }
        let runtime = self
            .game_eras
            .as_ref()
            .ok_or(AccessError::Unavailable("Game.Eras"))?;
        if runtime.pride.is_some() {
            return Err(AccessError::State(Error::AlreadyBound));
        }
        if extra
            .len()
            .checked_add(minimum.len())
            .is_none_or(|n| n > self.limits.max_runtime_array_items)
        {
            return Err(AccessError::State(Error::ResourceLimit));
        }
        self.advance_admission_generation()
            .map_err(AccessError::State)?;
        self.game_eras
            .as_mut()
            .expect("validated current owner")
            .pride = Some((extra, minimum));
        Ok(())
    }
    /// History.GetEraScore29da5d0: optional base presence is handled by caller.
    /// Minimum is queried only for a positive extra; missing domains stay unknown.
    pub fn game_era_pride_score(&self, owner: i32, base: i32) -> Result<i32, AccessError> {
        if owner == -1 {
            return Ok(base);
        }
        let owner = usize::try_from(owner).map_err(|_| AccessError::InvalidPlayer(owner))?;
        let (extra, minimum) = self
            .game_eras
            .as_ref()
            .and_then(|r| r.pride.as_ref())
            .ok_or(AccessError::Unavailable("Era pride score vectors"))?;
        let extra = extra.get(owner).copied().unwrap_or(0);
        if extra <= 0 {
            return Ok(base);
        }
        let minimum = minimum.get(owner).copied().unwrap_or(0);
        Ok(if base >= minimum {
            base.wrapping_add(extra)
        } else {
            base
        })
    }
    /// Complete current PlayerScores rates at native row+54/+58/+5c/+60,
    /// in the order used by the BuildingConstructed callback299ca00.
    pub fn initialize_game_era_construction_rates(
        &mut self,
        binding: &WireBinding,
        source: Option<&ObservedSnapshotWitness>,
        values: Vec<[i32; 4]>,
    ) -> Result<(), AccessError> {
        if !self.is_pristine() {
            return Err(AccessError::State(Error::NotPristine));
        }
        if binding != &self.binding
            || source.is_some_and(|v| self.observed_snapshot.as_ref() != Some(v))
        {
            return Err(AccessError::State(Error::Binding));
        }
        let runtime = self
            .game_eras
            .as_ref()
            .ok_or(AccessError::Unavailable("Game.Eras"))?;
        if runtime.construction_rates.is_some() {
            return Err(AccessError::State(Error::AlreadyBound));
        }
        if runtime
            .current_scores
            .as_ref()
            .is_some_and(|rows| rows.len() != values.len())
        {
            return Err(AccessError::State(Error::Binding));
        }
        if values
            .len()
            .checked_mul(4)
            .is_none_or(|n| n > self.limits.max_runtime_array_items)
        {
            return Err(AccessError::State(Error::ResourceLimit));
        }
        self.advance_admission_generation()
            .map_err(AccessError::State)?;
        self.game_eras
            .as_mut()
            .expect("validated")
            .construction_rates = Some(values);
        Ok(())
    }
    pub fn game_era_construction_rates(
        &self,
        owner: i32,
    ) -> Result<Option<&[i32; 4]>, AccessError> {
        if owner == -1 {
            return Ok(None);
        }
        let owner = usize::try_from(owner).map_err(|_| AccessError::InvalidPlayer(owner))?;
        self.game_eras()?
            .construction_rates
            .as_ref()
            .ok_or(AccessError::Unavailable("current Era construction rates"))
            .map(|rows| rows.get(owner))
    }
    /// Complete current PlayerScores rates at native row+48/+4c,
    /// in the order used by the continent/wonder discovery callbacks299c7a0/299c8c0.
    pub fn initialize_game_era_discovery_rates(
        &mut self,
        binding: &WireBinding,
        source: Option<&ObservedSnapshotWitness>,
        values: Vec<[i32; 2]>,
    ) -> Result<(), AccessError> {
        if !self.is_pristine() {
            return Err(AccessError::State(Error::NotPristine));
        }
        if binding != &self.binding
            || source.is_some_and(|v| self.observed_snapshot.as_ref() != Some(v))
        {
            return Err(AccessError::State(Error::Binding));
        }
        let runtime = self
            .game_eras
            .as_ref()
            .ok_or(AccessError::Unavailable("Game.Eras"))?;
        if runtime.discovery_rates.is_some() {
            return Err(AccessError::State(Error::AlreadyBound));
        }
        if runtime
            .current_scores
            .as_ref()
            .is_some_and(|rows| rows.len() != values.len())
        {
            return Err(AccessError::State(Error::Binding));
        }
        if values
            .len()
            .checked_mul(2)
            .is_none_or(|n| n > self.limits.max_runtime_array_items)
        {
            return Err(AccessError::State(Error::ResourceLimit));
        }
        self.advance_admission_generation()
            .map_err(AccessError::State)?;
        self.game_eras.as_mut().expect("validated").discovery_rates = Some(values);
        Ok(())
    }
    pub fn game_era_discovery_rates(&self, owner: i32) -> Result<Option<&[i32; 2]>, AccessError> {
        if owner == -1 {
            return Ok(None);
        }
        let owner = usize::try_from(owner).map_err(|_| AccessError::InvalidPlayer(owner))?;
        self.game_eras()?
            .discovery_rates
            .as_ref()
            .ok_or(AccessError::Unavailable("current Era discovery rates"))
            .map(|rows| rows.get(owner))
    }
    /// Current PlayerScores tech/civic boost rates at native +20/+24.
    pub fn initialize_game_era_boost_rates(
        &mut self,
        binding: &WireBinding,
        source: Option<&ObservedSnapshotWitness>,
        values: Vec<[i32; 2]>,
    ) -> Result<(), AccessError> {
        if !self.is_pristine() {
            return Err(AccessError::State(Error::NotPristine));
        }
        if binding != &self.binding
            || source.is_some_and(|v| self.observed_snapshot.as_ref() != Some(v))
        {
            return Err(AccessError::State(Error::Binding));
        }
        let runtime = self
            .game_eras
            .as_ref()
            .ok_or(AccessError::Unavailable("Game.Eras"))?;
        if runtime.boost_rates.is_some() {
            return Err(AccessError::State(Error::AlreadyBound));
        }
        if runtime
            .current_scores
            .as_ref()
            .is_some_and(|rows| rows.len() != values.len())
        {
            return Err(AccessError::State(Error::Binding));
        }
        if values
            .len()
            .checked_mul(2)
            .is_none_or(|n| n > self.limits.max_runtime_array_items)
        {
            return Err(AccessError::State(Error::ResourceLimit));
        }
        self.advance_admission_generation()
            .map_err(AccessError::State)?;
        self.game_eras.as_mut().expect("validated").boost_rates = Some(values);
        Ok(())
    }
    pub fn game_era_boost_rates(&self, owner: i32) -> Result<Option<&[i32; 2]>, AccessError> {
        if owner == -1 {
            return Ok(None);
        }
        let owner = usize::try_from(owner).map_err(|_| AccessError::InvalidPlayer(owner))?;
        self.game_eras()?
            .boost_rates
            .as_ref()
            .ok_or(AccessError::Unavailable("current Era boost rates"))
            .map(|rows| rows.get(owner))
    }
    /// Current PlayerScores district completion rate at native +34.
    pub fn initialize_game_era_district_construction_rates(
        &mut self,
        binding: &WireBinding,
        source: Option<&ObservedSnapshotWitness>,
        values: Vec<i32>,
    ) -> Result<(), AccessError> {
        if !self.is_pristine() {
            return Err(AccessError::State(Error::NotPristine));
        }
        if binding != &self.binding
            || source.is_some_and(|v| self.observed_snapshot.as_ref() != Some(v))
        {
            return Err(AccessError::State(Error::Binding));
        }
        let runtime = self
            .game_eras
            .as_ref()
            .ok_or(AccessError::Unavailable("Game.Eras"))?;
        if runtime.district_construction_rates.is_some() {
            return Err(AccessError::State(Error::AlreadyBound));
        }
        if runtime
            .current_scores
            .as_ref()
            .is_some_and(|rows| rows.len() != values.len())
        {
            return Err(AccessError::State(Error::Binding));
        }
        if values
            .len()
            .checked_mul(1)
            .is_none_or(|n| n > self.limits.max_runtime_array_items)
        {
            return Err(AccessError::State(Error::ResourceLimit));
        }
        self.advance_admission_generation()
            .map_err(AccessError::State)?;
        self.game_eras
            .as_mut()
            .expect("validated")
            .district_construction_rates = Some(values);
        Ok(())
    }
    pub fn game_era_district_construction_rate(
        &self,
        owner: i32,
    ) -> Result<Option<i32>, AccessError> {
        if owner == -1 {
            return Ok(None);
        }
        let owner = usize::try_from(owner).map_err(|_| AccessError::InvalidPlayer(owner))?;
        self.game_eras()?
            .district_construction_rates
            .as_ref()
            .ok_or(AccessError::Unavailable(
                "current Era district construction rates",
            ))
            .map(|rows| rows.get(owner).copied())
    }
    pub fn game_era_scores(&self) -> Result<&[Vec<i32>], AccessError> {
        self.game_eras
            .as_ref()
            .and_then(|r| r.current_scores.as_deref())
            .ok_or(AccessError::Unavailable("current Game.Eras score vectors"))
    }
    /// ChangeEraScore2998080 through the native cell write. The caller must run
    /// report788b7534 and any reached notification continuation after success.
    /// Invalid positive indices follow the source early return without mutation.
    pub fn change_game_era_score(
        &mut self,
        owner: i32,
        amount: i32,
        reason: i32,
    ) -> Result<bool, AccessError> {
        if owner == -1 {
            return Ok(false);
        }
        let owner = usize::try_from(owner).map_err(|_| AccessError::InvalidPlayer(owner))?;
        let reason = usize::try_from(reason)
            .map_err(|_| AccessError::Unavailable("negative EraScore type"))?;
        let scores = self.game_era_scores()?;
        let Some(cell) = scores.get(owner).and_then(|r| r.get(reason)) else {
            return Ok(false);
        };
        let value = cell.wrapping_add(amount);
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(AccessError::State(Error::ResourceLimit))?;
        self.game_eras
            .as_mut()
            .expect("validated")
            .current_scores
            .as_mut()
            .expect("validated")[owner][reason] = value;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_domains_and_native_invalid_indices_do_not_become_zero_cells() {
        let mut state = super::super::super::tests::state();
        assert!(!state.change_game_era_score(-1, 3, 0).unwrap());
        assert!(matches!(
            state.change_game_era_score(0, 3, 0),
            Err(AccessError::Unavailable(_))
        ));
        let binding = state.binding().clone();
        state
            .initialize_game_eras(
                &binding,
                None,
                super::super::Inputs {
                    golden_ages_raw: Some(vec![2]),
                    dark_ages_raw: None,
                    current_era: Some(0),
                    era_start_turn: None,
                },
            )
            .unwrap();
        state
            .initialize_game_era_scores(&binding, None, vec![vec![i32::MAX, 0], vec![]])
            .unwrap();
        state
            .initialize_game_era_pride(&binding, None, vec![2, -1, 5], vec![3, 0, 1])
            .unwrap();
        assert_eq!(state.game_era_pride_score(0, 2).unwrap(), 2);
        assert_eq!(state.game_era_pride_score(0, 3).unwrap(), 5);
        assert_eq!(state.game_era_pride_score(1, 3).unwrap(), 3);
        assert_eq!(
            state.game_era_pride_score(2, i32::MAX).unwrap(),
            i32::MIN + 4
        );
        assert_eq!(state.game_era_pride_score(9, 3).unwrap(), 3);
        assert!(!state.change_game_era_score(1, 3, 0).unwrap());
        assert!(!state.change_game_era_score(2, 3, 0).unwrap());
        assert!(!state.change_game_era_score(0, 3, 2).unwrap());
        assert_eq!(
            state.game_era_scores().unwrap(),
            &[vec![i32::MAX, 0], vec![]]
        );
        assert!(state.change_game_era_score(0, 3, 0).unwrap());
        assert_eq!(state.game_era_scores().unwrap()[0], [i32::MIN + 2, 0]);
        assert!(state.player_has_golden_age(0).unwrap());
        assert!(matches!(
            state.player_has_dark_age(0),
            Err(AccessError::Unavailable(_))
        ));
    }
}
