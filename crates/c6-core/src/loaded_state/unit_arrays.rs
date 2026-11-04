//! Targeted writes to current native-index arrays and their hash projection.
//! Other runtime storage and dependency guards are preserved.
use super::{Error, LoadedState};
use crate::unit::HashValue;

fn store<T: Copy>(
    pairs: &mut Vec<HashValue<T>>,
    dependencies: &mut Vec<HashValue<T>>,
    live: &mut [T],
    index: usize,
    hash: u32,
    value: T,
) -> Result<(), Error> {
    // Multiple projected occurrences need their original index mapping; do not
    // overwrite another native cell merely because its definition hash aliases.
    if pairs.iter().filter(|p| p.definition_hash == hash).count() > 1
        || dependencies
            .iter()
            .filter(|p| p.definition_hash == hash)
            .count()
            > 1
    {
        return Err(Error::UnitInitialization(
            crate::unit_initialization::Error::ArrayStorage,
        ));
    }
    let cell = live.get_mut(index).ok_or(Error::UnitInitialization(
        crate::unit_initialization::Error::ArrayStorage,
    ))?;
    // Read may omit a default cell. Reserve both projections before writing;
    // unknown hashes and other indexes remain untouched.
    if !pairs.iter().any(|p| p.definition_hash == hash) {
        pairs.try_reserve(1).map_err(|_| Error::ResourceLimit)?;
    }
    if !dependencies.iter().any(|p| p.definition_hash == hash) {
        dependencies
            .try_reserve(1)
            .map_err(|_| Error::ResourceLimit)?;
    }
    fn project<T: Copy>(pairs: &mut Vec<HashValue<T>>, hash: u32, value: T) {
        let mut found = false;
        for pair in pairs.iter_mut().filter(|p| p.definition_hash == hash) {
            pair.value = value;
            found = true;
        }
        if !found {
            pairs.push(HashValue {
                definition_hash: hash,
                value,
            });
        }
    }
    project(pairs, hash, value);
    project(dependencies, hash, value);
    *cell = value;
    Ok(())
}

impl LoadedState {
    /// `index` and `definition_hash` come from the admitted current ability
    /// registry. This is a value setter; wrapping/clamping and signals belong to
    /// ChangeAbilityCount. Both representations are current before callbacks.
    pub fn set_unit_ability_count(
        &mut self,
        owner: i32,
        id: u32,
        index: usize,
        definition_hash: u32,
        value: i32,
    ) -> Result<(), Error> {
        self.unit_runtime(owner, id)?;
        let next = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        let row = self
            .runtime
            .as_mut()
            .expect("validated runtime")
            .iter_mut()
            .find(|p| p.owner == owner)
            .expect("validated owner");
        let at = row.unit_index[(id & 0xffff) as usize].expect("validated slot");
        let unit = self
            .world
            .player_mut(owner)
            .map_err(Error::World)?
            .unit_mut(id)
            .map_err(Error::World)?
            .expect("validated Unit");
        let runtime = &mut row.units[at];
        store(
            &mut unit.ai_abilities_possessed,
            &mut runtime.dependencies.abilities,
            &mut runtime.value.abilities,
            index,
            definition_hash,
            value,
        )?;
        self.revision = next;
        Ok(())
    }
    /// Current SetInvalidTerrainType cell, preserving every other array/index.
    pub fn set_unit_invalid_terrain(
        &mut self,
        owner: i32,
        id: u32,
        index: usize,
        definition_hash: u32,
        value: bool,
    ) -> Result<(), Error> {
        self.unit_runtime(owner, id)?;
        let next = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        let row = self
            .runtime
            .as_mut()
            .expect("validated runtime")
            .iter_mut()
            .find(|p| p.owner == owner)
            .expect("validated owner");
        let at = row.unit_index[(id & 0xffff) as usize].expect("validated slot");
        let unit = self
            .world
            .player_mut(owner)
            .map_err(Error::World)?
            .unit_mut(id)
            .map_err(Error::World)?
            .expect("validated Unit");
        let runtime = &mut row.units[at];
        store(
            &mut unit.ab_invalid_terrain_types,
            &mut runtime.dependencies.invalid_terrain,
            &mut runtime.value.invalid_terrain,
            index,
            definition_hash,
            value,
        )?;
        self.revision = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state() -> (LoadedState, i32, u32) {
        let (mut state, actor) = super::super::tests::actor_commit_state();
        let owner = actor.player();
        let row = state
            .runtime
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|p| p.owner == owner)
            .unwrap();
        let runtime = &mut row.units[0];
        runtime.value.abilities = vec![4, 1];
        runtime.value.invalid_terrain = vec![false, true];
        runtime.value.ignore_feature = vec![true];
        let unit = state
            .world
            .player_mut(owner)
            .unwrap()
            .unit_mut(actor.id)
            .unwrap()
            .unwrap();
        unit.ai_abilities_possessed = vec![
            HashValue {
                definition_hash: 10,
                value: 4,
            },
            HashValue {
                definition_hash: 11,
                value: 1,
            },
        ];
        unit.ab_invalid_terrain_types = vec![
            HashValue {
                definition_hash: 20,
                value: false,
            },
            HashValue {
                definition_hash: 21,
                value: true,
            },
        ];
        runtime.dependencies = super::super::UnitDependencies::capture(unit).unwrap();
        (state, owner, actor.id)
    }
    #[test]
    fn removal_then_current_query_reads_both_updated_arrays_and_preserves_other_storage() {
        let (mut s, o, id) = state();
        let stamp = s.action_stamp();
        s.set_unit_ability_count(o, id, 1, 11, 0).unwrap();
        s.set_unit_invalid_terrain(o, id, 0, 20, true).unwrap();
        let runtime = s.unit_runtime(o, id).unwrap();
        assert_eq!(runtime.abilities, [4, 0]);
        assert_eq!(runtime.invalid_terrain, [true, true]);
        assert_eq!(runtime.ignore_feature, [true]);
        let unit = s.world.player(o).unwrap().unit(id).unwrap().unwrap().record;
        assert_eq!(unit.ai_abilities_possessed[1].value, 0);
        assert!(unit.ab_invalid_terrain_types[0].value);
        assert_ne!(stamp, s.action_stamp());
    }
    #[test]
    fn unrelated_raw_change_is_not_admitted_by_targeted_setter() {
        let (mut s, o, id) = state();
        s.unit_mut(o, id)
            .unwrap()
            .ab_ignore_feature_types
            .push(HashValue {
                definition_hash: 99,
                value: true,
            });
        assert!(matches!(
            s.set_unit_ability_count(o, id, 1, 11, 0),
            Err(Error::RuntimeDependenciesChanged)
        ));
        assert_eq!(
            s.runtime.as_ref().unwrap()[0].units[0].value.abilities,
            [4, 1]
        );
    }
    #[test]
    fn missing_projected_default_is_added_without_resetting_other_cells() {
        let (mut s, o, id) = state();
        s.world
            .player_mut(o)
            .unwrap()
            .unit_mut(id)
            .unwrap()
            .unwrap()
            .ai_abilities_possessed
            .remove(0);
        s.runtime.as_mut().unwrap()[0].units[0]
            .dependencies
            .abilities
            .remove(0);
        s.set_unit_ability_count(o, id, 0, 10, 7).unwrap();
        assert_eq!(s.unit_runtime(o, id).unwrap().abilities, [7, 1]);
        assert_eq!(
            s.world
                .player(o)
                .unwrap()
                .unit(id)
                .unwrap()
                .unwrap()
                .record
                .ai_abilities_possessed
                .last()
                .unwrap()
                .definition_hash,
            10
        );
    }
}
