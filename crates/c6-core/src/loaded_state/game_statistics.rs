//! Canonical current global Game.Stats domains. This begins with the actual
//! building-count array; other global counters remain unavailable, not zero.
//! Saved Stats overlays are not substituted for a current dense snapshot.
use super::{Error, LoadedState, ObservedSnapshotWitness, WireBinding};
use crate::unit::HashValue;

#[derive(Debug)]
pub struct Statistics {
    buildings: Vec<HashValue<i32>>,
    source: Option<ObservedSnapshotWitness>,
}
impl Statistics {
    pub fn building_counts(&self) -> &[HashValue<i32>] {
        &self.buildings
    }
    pub fn source(&self) -> Option<&ObservedSnapshotWitness> {
        self.source.as_ref()
    }
}
#[derive(Debug)]
pub enum AccessError {
    State(Error),
    Unavailable,
    DuplicateBuilding(u32),
    UnknownBuilding(u32),
}
impl std::fmt::Display for AccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "global Game.Stats: {self:?}")
    }
}
impl std::error::Error for AccessError {}
impl LoadedState {
    /// Supplied complete current building-definition domain, in native order.
    /// The input adapter joins the actual source epoch and definition mapping;
    /// None source denotes model-origin construction, never a native witness.
    pub fn install_game_building_counts(
        &mut self,
        binding: &WireBinding,
        buildings: Vec<HashValue<i32>>,
        source: Option<ObservedSnapshotWitness>,
    ) -> Result<(), AccessError> {
        if binding != &self.binding
            || source.as_ref().is_some_and(|s| {
                s.capture_configuration_sha256 != binding.inputs.configuration_sha256
            })
        {
            return Err(AccessError::State(Error::Binding));
        }
        if self.game_statistics.is_some() {
            return Err(AccessError::State(Error::AlreadyBound));
        }
        if buildings.len() > self.limits.max_runtime_array_items {
            return Err(AccessError::State(Error::ResourceLimit));
        }
        for (i, v) in buildings.iter().enumerate() {
            if buildings[..i]
                .iter()
                .any(|p| p.definition_hash == v.definition_hash)
            {
                return Err(AccessError::DuplicateBuilding(v.definition_hash));
            }
        }
        let next = self
            .revision
            .checked_add(1)
            .ok_or(AccessError::State(Error::ResourceLimit))?;
        self.game_statistics = Some(Statistics { buildings, source });
        self.revision = next;
        Ok(())
    }
    pub fn game_statistics(&self) -> Result<&Statistics, AccessError> {
        self.game_statistics
            .as_ref()
            .ok_or(AccessError::Unavailable)
    }
    pub fn game_building_count(&self, building: u32) -> Result<i32, AccessError> {
        self.game_statistics()?
            .buildings
            .iter()
            .find(|v| v.definition_hash == building)
            .map(|v| v.value)
            .ok_or(AccessError::UnknownBuilding(building))
    }
    /// Game.Stats.OnBuildingConstructed2a1be80: valid plot/building and a true
    /// completion flag increment the GLOBAL counter once, including nonwonders.
    /// The source invokes no gameplay callback or RNG. Player stats are separate.
    pub fn record_game_building_constructed(
        &mut self,
        building: Option<u32>,
        plot: i32,
        completed: bool,
    ) -> Result<bool, AccessError> {
        let Some(building) = building.filter(|_| plot >= 0 && completed) else {
            return Ok(false);
        };
        let stats = self
            .game_statistics
            .as_ref()
            .ok_or(AccessError::Unavailable)?;
        let index = stats
            .buildings
            .iter()
            .position(|v| v.definition_hash == building)
            .ok_or(AccessError::UnknownBuilding(building))?;
        let next = self
            .revision
            .checked_add(1)
            .ok_or(AccessError::State(Error::ResourceLimit))?;
        let stats = self.game_statistics.as_mut().expect("validated domain");
        stats.buildings[index].value = stats.buildings[index].value.wrapping_add(1);
        self.revision = next;
        Ok(true)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn global_counter_is_explicit_and_source_guards_do_not_require_it() {
        let mut state = super::super::tests::state();
        let stamp = state.revision();
        assert!(!state
            .record_game_building_constructed(None, 0, true)
            .unwrap());
        assert!(!state
            .record_game_building_constructed(Some(7), -1, true)
            .unwrap());
        assert!(!state
            .record_game_building_constructed(Some(7), 0, false)
            .unwrap());
        assert_eq!(state.revision(), stamp);
        assert!(matches!(
            state.game_building_count(7),
            Err(AccessError::Unavailable)
        ));
        let binding = state.binding().clone();
        state
            .install_game_building_counts(
                &binding,
                vec![HashValue {
                    definition_hash: 7,
                    value: i32::MAX,
                }],
                None,
            )
            .unwrap();
        assert!(state
            .record_game_building_constructed(Some(7), 0, true)
            .unwrap());
        assert_eq!(state.game_building_count(7).unwrap(), i32::MIN);
        let stamp = state.revision();
        assert!(matches!(
            state.record_game_building_constructed(Some(8), 0, true),
            Err(AccessError::UnknownBuilding(8))
        ));
        assert_eq!(state.revision(), stamp);
    }
    #[test]
    fn duplicate_or_foreign_inputs_reject_before_installing() {
        let mut state = super::super::tests::state();
        let binding = state.binding().clone();
        let value = HashValue {
            definition_hash: 7,
            value: 0,
        };
        assert!(matches!(
            state.install_game_building_counts(&binding, vec![value.clone(), value], None),
            Err(AccessError::DuplicateBuilding(7))
        ));
        assert!(matches!(
            state.game_statistics(),
            Err(AccessError::Unavailable)
        ));
        let mut wrong = binding;
        wrong.core_sha256[0] ^= 1;
        assert!(matches!(
            state.install_game_building_counts(&wrong, vec![], None),
            Err(AccessError::State(Error::Binding))
        ));
        assert!(matches!(
            state.game_statistics(),
            Err(AccessError::Unavailable)
        ));
    }
}
