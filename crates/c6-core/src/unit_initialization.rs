//! Source-derived current Linux Unit Read application, not full live loading.
//!
//! Keeps wire hashes unchanged and places derived mutable storage beside the
//! owned Unit. Archive registration, nested component remaps and callbacks have
//! separate contracts; this type never asserts their completion.
use crate::core_player::initialization::InputIdentities;
use crate::formation_initialization::{
    DirtyBaseline, LogicalArchive, UNIT_COMPONENT_TYPE, UNIT_FIELDS,
};
use crate::movement_world::{DefinitionLookup, DefinitionTable};
use crate::unit::{ComponentId, HashValue, Unit};
use crate::world::{MissingInput as WorldError, Player};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Profile,
    Owner,
    Definition,
    DefinitionIndex,
    ArrayStorage,
    ResourceLimit,
    World(WorldError),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Unit Read application: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Actual native definition GetDomainHash(), indexed by collection order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnitDefinition {
    pub domain_hash: u32,
}

/// Caller admits these getter/lookup inputs to this exact saved-input profile.
/// A matching digest alone does not establish that a capture is authentic.
pub struct InitializationInputs<'a> {
    pub identities: &'a InputIdentities,
    pub units: DefinitionTable<'a, UnitDefinition>,
    pub abilities: &'a dyn DefinitionLookup,
    pub terrains: &'a dyn DefinitionLookup,
    pub features: &'a dyn DefinitionLookup,
    pub great_person_classes: &'a dyn DefinitionLookup,
    pub operations: &'a dyn DefinitionLookup,
}

impl InitializationInputs<'_> {
    fn resolve_definition(
        &self,
        wire_hash: u32,
        admitted: &InputIdentities,
    ) -> Result<(i32, i32), Error> {
        if admitted != self.identities {
            return Err(Error::Profile);
        }
        let index = self
            .units
            .lookup
            .find_index(wire_hash)
            .ok_or(Error::Definition)?;
        let index = i32::try_from(index).map_err(|_| Error::DefinitionIndex)?;
        let definition = self.units.at(index).ok_or(Error::DefinitionIndex)?;
        Ok((index, domain_from_hash(definition.domain_hash)))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormationReference {
    pub owner: i32,
    pub requested_id: u32,
    pub resolved_id: u32,
}

/// Native InitializeDomain 0x2c12bc0, including unknown-hash NO_DOMAIN.
/// The input is the admitted definition's domain hash, never a Unit type hash.
pub fn domain_from_hash(hash: u32) -> i32 {
    match hash {
        0x643e_ada3 => 0,
        0x5397_d46f => 1,
        0xb6e1_baac => 2,
        0xc03b_bd2f => 3,
        0x44c2_2cdb => 4,
        _ => -1,
    }
}

/// Generic ReadIndexHashArray overlay. Unknown keys are consumed and ignored;
/// known duplicates overwrite in stream order. Unmentioned destinations retain
/// their supplied values. Resolve each pair once and cache validated indexes
/// before the first write, including when the lookup uses interior mutability.
pub fn overlay<T: Copy>(
    destination: &mut [T],
    pairs: &[HashValue<T>],
    definitions: &dyn DefinitionLookup,
) -> Result<(), Error> {
    if definitions.count() != destination.len() || destination.len() > i32::MAX as usize {
        return Err(Error::ArrayStorage);
    }
    let mut indices = Vec::new();
    indices
        .try_reserve_exact(pairs.len())
        .map_err(|_| Error::ResourceLimit)?;
    for pair in pairs {
        let index = definitions.find_index(pair.definition_hash);
        if index.is_some_and(|i| i >= destination.len()) {
            return Err(Error::DefinitionIndex);
        }
        indices.push(index);
    }
    for (pair, index) in pairs.iter().zip(indices) {
        if let Some(index) = index {
            destination[index] = pair.value;
        }
    }
    Ok(())
}

fn reset_overlay<T: Copy>(
    pairs: &[HashValue<T>],
    definitions: &dyn DefinitionLookup,
    default: T,
) -> Result<Vec<T>, Error> {
    if definitions.count() > i32::MAX as usize {
        return Err(Error::ArrayStorage);
    }
    let mut values = Vec::new();
    values
        .try_reserve_exact(definitions.count())
        .map_err(|_| Error::ResourceLimit)?;
    values.resize(definitions.count(), default);
    overlay(&mut values, pairs, definitions)?;
    Ok(values)
}

/// Derived state for the bounded Unit28 Read boundary. Wire state stays in World.
/// Fields are live storage, not cached answers to action admission predicates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitRuntime {
    /// Inline wrapper attachment is proven; outer-load dirty baseline is separate.
    pub archive: LogicalArchive,
    pub definition_index: i32,
    pub domain: i32,
    pub formation: Option<FormationReference>,
    pub abilities: Vec<i32>,
    pub invalid_terrain: Vec<bool>,
    pub ignore_terrain: Vec<bool>,
    pub ignore_feature: Vec<bool>,
    pub bonus_points_per_kill: Vec<i32>,
    pub bonus_points_per_kill_strength: Vec<i32>,
    pub operations_disabled: Vec<bool>,
}
impl UnitRuntime {
    /// Prepare without mutating or cloning the imported world. The owner must
    /// supply its actual complete Formation lookup, even for a known null result.
    pub fn from_read(
        unit: &Unit,
        admitted_inputs: &InputIdentities,
        inputs: &InitializationInputs<'_>,
        owner: &Player,
    ) -> Result<Self, Error> {
        if unit.e_owner != owner.owner() {
            return Err(Error::Owner);
        }
        let (definition_index, domain) =
            inputs.resolve_definition(unit.e_type_index, admitted_inputs)?;
        let formation = owner
            .formation(unit.formation_id)
            .map_err(Error::World)?
            .map(|found| FormationReference {
                owner: owner.owner(),
                requested_id: found.requested_id,
                resolved_id: found.resolved_id,
            });
        Ok(Self {
            archive: LogicalArchive::new(
                ComponentId {
                    owner: unit.e_owner as i16,
                    component_type: UNIT_COMPONENT_TYPE,
                    id: unit.id,
                },
                &UNIT_FIELDS,
                DirtyBaseline::Unavailable,
            ),
            definition_index,
            domain,
            formation,
            abilities: reset_overlay(&unit.ai_abilities_possessed, inputs.abilities, 0)?,
            invalid_terrain: reset_overlay(&unit.ab_invalid_terrain_types, inputs.terrains, false)?,
            ignore_terrain: reset_overlay(&unit.ab_ignore_terrain_types, inputs.terrains, false)?,
            ignore_feature: reset_overlay(&unit.ab_ignore_feature_types, inputs.features, false)?,
            bonus_points_per_kill: reset_overlay(
                &unit.ai_bonus_points_per_kill,
                inputs.great_person_classes,
                0,
            )?,
            bonus_points_per_kill_strength: reset_overlay(
                &unit.ai_bonus_points_per_kill_strength,
                inputs.great_person_classes,
                0,
            )?,
            operations_disabled: reset_overlay(
                &unit.k_operations_disabled,
                inputs.operations,
                false,
            )?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Lookup;
    impl DefinitionLookup for Lookup {
        fn count(&self) -> usize {
            3
        }
        fn find_index(&self, key: u32) -> Option<usize> {
            match key {
                0..=2 => Some(key as usize),
                99 => Some(1),
                123 => Some(9),
                _ => None,
            }
        }
        fn type_hash(&self, index: usize) -> Option<u32> {
            [40, 99, 60].get(index).copied()
        }
    }
    fn pair<T>(definition_hash: u32, value: T) -> HashValue<T> {
        HashValue {
            definition_hash,
            value,
        }
    }
    #[test]
    fn definition_hash_requires_actual_mapping_and_matching_profile() {
        use crate::core_player::initialization::DatabaseIdentity;
        let identity = InputIdentities {
            configuration_sha256: [1; 32],
            rules_inputs_sha256: [2; 32],
            database: DatabaseIdentity::LiveSnapshotUnavailable,
            dlc_inputs_sha256: [3; 32],
            mods_inputs_sha256: [4; 32],
            content_inputs_sha256: [5; 32],
        };
        let properties = [
            UnitDefinition {
                domain_hash: 0x643eada3,
            },
            UnitDefinition {
                domain_hash: 0xb6e1baac,
            },
            UnitDefinition { domain_hash: 0 },
        ];
        let inputs = InitializationInputs {
            identities: &identity,
            units: DefinitionTable::new(&Lookup, &properties).unwrap(),
            abilities: &Lookup,
            terrains: &Lookup,
            features: &Lookup,
            great_person_classes: &Lookup,
            operations: &Lookup,
        };
        assert_eq!(inputs.resolve_definition(99, &identity), Ok((1, 2)));
        assert_eq!(
            inputs.resolve_definition(777, &identity),
            Err(Error::Definition)
        );
        assert_eq!(
            inputs.resolve_definition(123, &identity),
            Err(Error::DefinitionIndex)
        );
        let mut other = identity.clone();
        other.mods_inputs_sha256[0] ^= 1;
        assert_eq!(inputs.resolve_definition(99, &other), Err(Error::Profile));
    }
    #[test]
    fn overlay_preserves_unsaved_destinations_and_stream_order() {
        let mut prior = [7, 8, 9];
        overlay(
            &mut prior,
            &[pair(99, 4), pair(777, 88), pair(1, -6)],
            &Lookup,
        )
        .unwrap();
        assert_eq!(prior, [7, -6, 9]);
        assert_eq!(
            reset_overlay(&[pair(99, 4), pair(99, -6)], &Lookup, 0),
            Ok(vec![0, -6, 0])
        );
    }
    #[test]
    fn overlay_uses_cached_indexes_when_provider_changes_between_calls() {
        use std::cell::Cell;
        struct Changing(Cell<usize>);
        impl DefinitionLookup for Changing {
            fn count(&self) -> usize {
                3
            }
            fn find_index(&self, _: u32) -> Option<usize> {
                let call = self.0.get();
                self.0.set(call + 1);
                Some(match call {
                    0 => 0,
                    1 => 1,
                    2 => 0,
                    _ => 9,
                })
            }
            fn type_hash(&self, _: usize) -> Option<u32> {
                None
            }
        }
        let lookup = Changing(Cell::new(0));
        let mut prior = [7, 8, 9];
        overlay(&mut prior, &[pair(40, 4), pair(99, 5)], &lookup).unwrap();
        assert_eq!(prior, [4, 5, 9]);
        assert_eq!(lookup.0.get(), 2);
    }
    #[test]
    fn invalid_resolved_index_cannot_partially_mutate() {
        let mut prior = [7, 8, 9];
        assert_eq!(
            overlay(&mut prior, &[pair(99, 4), pair(123, 88)], &Lookup),
            Err(Error::DefinitionIndex)
        );
        assert_eq!(prior, [7, 8, 9]);
        assert_eq!(
            overlay(&mut prior[..2], &[], &Lookup),
            Err(Error::ArrayStorage)
        );
    }
    #[test]
    fn native_domain_branches_include_signed_high_hashes_and_unknown() {
        for (hash, expected) in [
            (0x643eada3, 0),
            (0x5397d46f, 1),
            (0xb6e1baac, 2),
            (0xc03bbd2f, 3),
            (0x44c22cdb, 4),
            (0, -1),
            (u32::MAX, -1),
        ] {
            assert_eq!(domain_from_hash(hash), expected);
        }
    }
}
