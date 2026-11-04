//! Current Player.GetRevealedFeature query used before every sight-count change.
//! This reads the canonical player's hash-indexed feature flags; it does not
//! discover a feature, grant experience, invoke callbacks, or advance RNG.
use crate::registry::LinuxDefinitionRegistry;
use c6_core::{loaded_state::LoadedState, player_prerequisites::IndexedBools};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    MissingPlayer(i32),
    AmbiguousPlayer(i32),
    MissingFeatureFlags(i32),
    MissingFeatureValue { player: i32, index: i32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "current revealed feature: {self:?}")
    }
}
impl std::error::Error for Error {}

/// `features` must be the admitted current native-order collection used to
/// initialize this player's vector. Wire pair order is not native index order.
/// Absent valid-index pairs leave the initializer unknown, rather than false.
pub fn get(
    state: &LoadedState,
    features: &LinuxDefinitionRegistry,
    player: i32,
    feature: i32,
) -> Result<bool, Error> {
    if feature == -1 || (feature as u32 as usize) >= features.entries().len() {
        return Ok(false);
    }
    let mut players = state
        .retained()
        .players
        .iter()
        .filter(|p| p.requested_owner == player);
    let retained = players.next().ok_or(Error::MissingPlayer(player))?;
    if players.next().is_some() {
        return Err(Error::AmbiguousPlayer(player));
    }
    let flags = &retained
        .post_heroes_arrays
        .as_ref()
        .ok_or(Error::MissingFeatureFlags(player))?
        .feature_definitions;
    value(flags, features, player, feature)
}

fn value(
    flags: &IndexedBools,
    features: &LinuxDefinitionRegistry,
    player: i32,
    feature: i32,
) -> Result<bool, Error> {
    // GetRevealedFeature compares size and feature as unsigned u32; -1 has
    // its own early return. Other negative values therefore also miss here.
    if feature == -1 || (feature as u32 as usize) >= features.entries().len() {
        return Ok(false);
    }
    let index = feature as usize;
    // ReadIndexHashArray applies pairs in order, with later writes surviving.
    flags
        .pairs
        .iter()
        .rev()
        .find(|p| features.find_index(p.definition_hash) == Some(index))
        .map(|p| p.value)
        .ok_or(Error::MissingFeatureValue {
            player,
            index: feature,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        registry::{linux_type_hash, DefinitionEntry},
        RulesRowId,
    };
    use c6_core::unit::HashValue;
    fn definitions() -> LinuxDefinitionRegistry {
        LinuxDefinitionRegistry::from_native_order(
            ["A", "B"]
                .into_iter()
                .enumerate()
                .map(|(i, name)| DefinitionEntry {
                    database_rowid: RulesRowId(i as i64 + 9),
                    type_name: name.as_bytes().to_vec(),
                })
                .collect(),
        )
    }
    #[test]
    fn native_indexes_are_not_wire_order_and_last_overlay_survives() {
        let definitions = definitions();
        let flags = IndexedBools {
            pairs: vec![
                HashValue {
                    definition_hash: linux_type_hash(b"B"),
                    value: true,
                },
                HashValue {
                    definition_hash: linux_type_hash(b"A"),
                    value: true,
                },
                HashValue {
                    definition_hash: linux_type_hash(b"A"),
                    value: false,
                },
            ],
        };
        assert_eq!(value(&flags, &definitions, 0, 0), Ok(false));
        assert_eq!(value(&flags, &definitions, 0, 1), Ok(true));
    }
    #[test]
    fn sentinels_and_unsigned_bounds_do_not_require_flag_entries() {
        let definitions = definitions();
        let flags = IndexedBools { pairs: vec![] };
        for feature in [-1, -2, i32::MIN, 2, i32::MAX] {
            assert_eq!(value(&flags, &definitions, 0, feature), Ok(false));
        }
        assert_eq!(
            value(&flags, &definitions, 0, 0),
            Err(Error::MissingFeatureValue {
                player: 0,
                index: 0
            })
        );
    }
    #[test]
    fn native_reader_direct_index_priority_and_unknown_hash_are_preserved() {
        let definitions = definitions();
        let flags = IndexedBools {
            pairs: vec![
                HashValue {
                    definition_hash: 1,
                    value: true,
                },
                HashValue {
                    definition_hash: 0x12345678,
                    value: false,
                },
            ],
        };
        assert_eq!(value(&flags, &definitions, 0, 1), Ok(true));
        assert_eq!(
            value(&flags, &definitions, 0, 0),
            Err(Error::MissingFeatureValue {
                player: 0,
                index: 0
            })
        );
    }
}
