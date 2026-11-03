//! Linux XP2 improvement yield arithmetic with null contribution output.
//!
//! Original ordered definitions and typed adjacency inputs are required. This
//! component is not a complete plot yield, game state, action, or turn engine.

use crate::movement_world::DefinitionLookup;
use crate::yields::{
    improvement_bonus_civic_yield, improvement_bonus_tech_yield, ImprovementDefinition,
    MissingInput as BonusMissingInput, NativeDefinitionIndex, NativeFixedPoint8,
    OrderedImprovementDefinitions, PrerequisiteState,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImprovementYieldError {
    DefinitionRegistryShape,
    DefinitionIndex,
    BaseReferences { improvement: u32 },
    AdjacencyReferences { improvement: u32 },
    AdjacencyState { definition: i32 },
    Normalization { yield_index: i32 },
    Bonus(BonusMissingInput),
}

impl From<BonusMissingInput> for ImprovementYieldError {
    fn from(value: BonusMissingInput) -> Self {
        Self::Bonus(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImprovementBaseYield {
    pub yield_index: NativeDefinitionIndex,
    pub change: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct ImprovementYieldDefinition<'a> {
    pub type_hash: u32,
    /// Original nonnull row references, in definition order. None is uncaptured.
    pub base: Option<&'a [ImprovementBaseYield]>,
    /// Resolved Adjacency_YieldChange collection indices, in reference order.
    /// Duplicate references must remain duplicated. None is uncaptured.
    pub adjacency: Option<&'a [NativeDefinitionIndex]>,
}

#[derive(Clone, Copy)]
pub struct OrderedImprovementYieldDefinitions<'a> {
    definitions: &'a [ImprovementYieldDefinition<'a>],
    bonuses: OrderedImprovementDefinitions<'a>,
}

impl<'a> OrderedImprovementYieldDefinitions<'a> {
    /// Both property arrays must align with the same actual native registry.
    pub fn new(
        lookup: &'a dyn DefinitionLookup,
        definitions: &'a [ImprovementYieldDefinition<'a>],
        bonuses: &'a [ImprovementDefinition<'a>],
    ) -> Result<Self, ImprovementYieldError> {
        if lookup.count() != definitions.len() || lookup.count() > i32::MAX as usize {
            return Err(ImprovementYieldError::DefinitionRegistryShape);
        }
        for (index, definition) in definitions.iter().enumerate() {
            if lookup.type_hash(index) != Some(definition.type_hash) {
                return Err(ImprovementYieldError::DefinitionRegistryShape);
            }
        }
        Ok(Self {
            definitions,
            bonuses: OrderedImprovementDefinitions::new(lookup, bonuses)?,
        })
    }

    /// Check that another native registry has the same ordered type hashes.
    /// This compares registry alignment, not rules/configuration identity or
    /// equivalence of the provider's lookup implementation.
    pub fn matches_lookup(self, lookup: &dyn DefinitionLookup) -> bool {
        lookup.count() == self.definitions.len()
            && self
                .definitions
                .iter()
                .enumerate()
                .all(|(index, definition)| lookup.type_hash(index) == Some(definition.type_hash))
    }

    /// Inspect an actual native collection index without hash resolution.
    pub fn at(&self, index: i32) -> Option<&'a ImprovementYieldDefinition<'a>> {
        self.definitions.get(index as u32 as usize)
    }
}

/// Typed dependency for the original adjacency overload (0x2a3f150).
/// Implement using actual adjacency definitions/world state, preserving its
/// gates and arithmetic. Neither hypothetical map changes nor tile outputs exist
/// in this interface. Missing input must be an error, never a default zero.
pub trait ImprovementAdjacency {
    fn player_adjacency_yield(
        &self,
        player: i32,
        definition: NativeDefinitionIndex,
        requested_yield: i32,
        plot: Option<u32>,
    ) -> Result<i32, ImprovementYieldError>;

    /// Original NormalizeYields(value, yield, null player), only for yield -1.
    /// Specific-yield callers do not need the separate normalization policy.
    fn normalize_base_yield(
        &self,
        _value: NativeFixedPoint8,
        yield_index: i32,
    ) -> Result<NativeFixedPoint8, ImprovementYieldError> {
        Err(ImprovementYieldError::Normalization { yield_index })
    }
}

/// Plot identity plus the political owner read through CityAttributes.GetPlayer.
/// The builder's owner is not used by the original wrapper.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImprovementYieldPlot {
    pub id: u32,
    pub political_owner: i32,
}

/// Original 0x2a3ee50 arithmetic for a null contribution-list argument.
/// Improvement is a direct native collection index, never a type hash.
/// Base uses the first matching row; adjacency and bonus helpers retain all rows.
/// Modifiers are deliberately absent: native calls that helper only with a
/// nonnull contribution list. Invalid-player handling belongs to each callee;
/// it must not suppress base yield here.
pub fn improvement_player_yield(
    rules: OrderedImprovementYieldDefinitions<'_>,
    prerequisites: &impl PrerequisiteState,
    adjacency: &impl ImprovementAdjacency,
    player: i32,
    improvement: u32,
    requested_yield: i32,
    plot: Option<u32>,
) -> Result<i32, ImprovementYieldError> {
    let Some(definition) = rules.at(improvement as i32) else {
        return Ok(0);
    };
    let mut raw = 0i32;
    for row in definition
        .base
        .ok_or(ImprovementYieldError::BaseReferences { improvement })?
    {
        let value = NativeFixedPoint8::integer(row.change);
        if requested_yield == -1 {
            raw = raw.wrapping_add(
                adjacency
                    .normalize_base_yield(value, row.yield_index.get())?
                    .0,
            );
        } else if row.yield_index.get() == requested_yield {
            raw = raw.wrapping_add(value.0);
            break;
        }
    }
    for &reference in definition
        .adjacency
        .ok_or(ImprovementYieldError::AdjacencyReferences { improvement })?
    {
        let value = adjacency.player_adjacency_yield(player, reference, requested_yield, plot)?;
        raw = raw.wrapping_add(NativeFixedPoint8::integer(value).0);
    }
    raw = raw.wrapping_add(
        improvement_bonus_tech_yield(
            rules.bonuses,
            prerequisites,
            player,
            improvement,
            requested_yield,
        )?
        .0,
    );
    raw = raw.wrapping_add(
        improvement_bonus_civic_yield(
            rules.bonuses,
            prerequisites,
            player,
            improvement,
            requested_yield,
        )?
        .0,
    );
    // 0x1fccdb0: signed arithmetic shift, including negative fractions.
    Ok(raw >> 8)
}

/// Original 0x2a3ed90 wrapper with null contribution output.
pub fn improvement_yield(
    rules: OrderedImprovementYieldDefinitions<'_>,
    prerequisites: &impl PrerequisiteState,
    adjacency: &impl ImprovementAdjacency,
    improvement: u32,
    requested_yield: i32,
    plot: Option<ImprovementYieldPlot>,
) -> Result<i32, ImprovementYieldError> {
    if improvement == u32::MAX {
        return Ok(0);
    }
    let Some(plot) = plot else { return Ok(0) };
    if plot.political_owner == -1 {
        return Ok(0);
    }
    improvement_player_yield(
        rules,
        prerequisites,
        adjacency,
        plot.political_owner,
        improvement,
        requested_yield,
        Some(plot.id),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::yields::{BonusYieldChange, PrerequisiteKind};
    use std::cell::RefCell;

    struct Lookup;
    impl DefinitionLookup for Lookup {
        fn count(&self) -> usize {
            2
        }
        fn type_hash(&self, index: usize) -> Option<u32> {
            (index < 2).then_some(77)
        }
        fn find_index(&self, key: u32) -> Option<usize> {
            match key {
                0 | 1 => Some(key as usize),
                77 => Some(1),
                _ => None,
            }
        }
    }
    struct State;
    impl PrerequisiteState for State {
        fn is_valid_player(&self, player: i32) -> Result<bool, BonusMissingInput> {
            Ok(player != -1)
        }
        fn acquired_mask(&self, _: i32, _: PrerequisiteKind) -> Result<&[bool], BonusMissingInput> {
            Ok(&[true])
        }
        fn globally_available(
            &self,
            _: PrerequisiteKind,
            _: i32,
        ) -> Result<bool, BonusMissingInput> {
            Ok(true)
        }
    }
    type AdjacencyCall = (i32, i32, i32, Option<u32>);

    #[derive(Default)]
    struct Adj {
        calls: RefCell<Vec<AdjacencyCall>>,
        normalize: bool,
        missing: bool,
    }
    impl ImprovementAdjacency for Adj {
        fn player_adjacency_yield(
            &self,
            player: i32,
            definition: NativeDefinitionIndex,
            y: i32,
            plot: Option<u32>,
        ) -> Result<i32, ImprovementYieldError> {
            self.calls
                .borrow_mut()
                .push((player, definition.get(), y, plot));
            if self.missing {
                return Err(ImprovementYieldError::AdjacencyState {
                    definition: definition.get(),
                });
            }
            Ok(if definition.get() == 0 { 3 } else { -2 })
        }
        fn normalize_base_yield(
            &self,
            value: NativeFixedPoint8,
            y: i32,
        ) -> Result<NativeFixedPoint8, ImprovementYieldError> {
            if !self.normalize {
                return Err(ImprovementYieldError::Normalization { yield_index: y });
            }
            Ok(NativeFixedPoint8(value.0 / 2))
        }
    }
    fn idx(value: u32) -> NativeDefinitionIndex {
        NativeDefinitionIndex::new(value, 3).unwrap()
    }
    fn base(change: i32, y: u32) -> ImprovementBaseYield {
        ImprovementBaseYield {
            yield_index: idx(y),
            change,
        }
    }
    fn with_rules<T>(
        base: &[ImprovementBaseYield],
        refs: &[NativeDefinitionIndex],
        f: impl FnOnce(OrderedImprovementYieldDefinitions<'_>) -> T,
    ) -> T {
        let entries = [
            ImprovementYieldDefinition {
                type_hash: 77,
                base: Some(&[]),
                adjacency: Some(&[]),
            },
            ImprovementYieldDefinition {
                type_hash: 77,
                base: Some(base),
                adjacency: Some(refs),
            },
        ];
        let rows = [Some(BonusYieldChange {
            yield_index: Some(idx(1)),
            prerequisite_technology: Some(idx(0)),
            prerequisite_civic: Some(idx(0)),
            bonus_yield_change: 5,
        })];
        let bonus = [
            ImprovementDefinition {
                type_hash: 77,
                bonus_yield_changes: Some(&[]),
            },
            ImprovementDefinition {
                type_hash: 77,
                bonus_yield_changes: Some(&rows),
            },
        ];
        f(OrderedImprovementYieldDefinitions::new(&Lookup, &entries, &bonus).unwrap())
    }
    #[test]
    fn registry_alignment_checks_count_and_every_hash() {
        struct OtherLookup(&'static [u32]);
        impl DefinitionLookup for OtherLookup {
            fn count(&self) -> usize {
                self.0.len()
            }
            fn type_hash(&self, index: usize) -> Option<u32> {
                self.0.get(index).copied()
            }
            fn find_index(&self, _: u32) -> Option<usize> {
                None
            }
        }
        with_rules(&[], &[], |rules| {
            assert!(rules.matches_lookup(&Lookup));
            assert!(rules.matches_lookup(&OtherLookup(&[77, 77])));
            assert!(!rules.matches_lookup(&OtherLookup(&[77])));
            assert!(!rules.matches_lookup(&OtherLookup(&[77, 77, 77])));
            assert!(!rules.matches_lookup(&OtherLookup(&[76, 77])));
            assert!(!rules.matches_lookup(&OtherLookup(&[77, 76])));
        });
    }

    #[test]
    fn first_base_all_adjacency_both_bonuses_and_native_index_lookup() {
        let adj = Adj::default();
        with_rules(
            &[base(90, 0), base(7, 1), base(999, 1)],
            &[idx(0), idx(1), idx(0)],
            |rules| {
                assert_eq!(
                    improvement_player_yield(rules, &State, &adj, 4, 1, 1, Some(8)),
                    Ok(21)
                );
                assert_eq!(
                    *adj.calls.borrow(),
                    [(4, 0, 1, Some(8)), (4, 1, 1, Some(8)), (4, 0, 1, Some(8))]
                );
                assert_eq!(
                    improvement_player_yield(rules, &State, &adj, 4, 0, 1, Some(8)),
                    Ok(0)
                );
                assert_eq!(
                    improvement_player_yield(rules, &State, &adj, 4, 76, 1, Some(8)),
                    Ok(0)
                );
            },
        );
    }
    #[test]
    fn out_of_range_hash_returns_zero_before_rows_or_providers() {
        struct Forbidden;
        impl PrerequisiteState for Forbidden {
            fn is_valid_player(&self, _: i32) -> Result<bool, BonusMissingInput> {
                panic!("unexpected player query")
            }
            fn acquired_mask(
                &self,
                _: i32,
                _: PrerequisiteKind,
            ) -> Result<&[bool], BonusMissingInput> {
                panic!("unexpected mask query")
            }
            fn globally_available(
                &self,
                _: PrerequisiteKind,
                _: i32,
            ) -> Result<bool, BonusMissingInput> {
                panic!("unexpected availability query")
            }
        }
        impl ImprovementAdjacency for Forbidden {
            fn player_adjacency_yield(
                &self,
                _: i32,
                _: NativeDefinitionIndex,
                _: i32,
                _: Option<u32>,
            ) -> Result<i32, ImprovementYieldError> {
                panic!("unexpected adjacency query")
            }
        }
        let entries = [ImprovementYieldDefinition {
            type_hash: 77,
            base: None,
            adjacency: None,
        }; 2];
        let bonus = [ImprovementDefinition {
            type_hash: 77,
            bonus_yield_changes: None,
        }; 2];
        let rules = OrderedImprovementYieldDefinitions::new(&Lookup, &entries, &bonus).unwrap();
        // Lookup.find_index(77) would select index 1, but native uses At.
        for key in [2, 77, u32::MAX] {
            assert_eq!(
                improvement_player_yield(rules, &Forbidden, &Forbidden, 4, key, 1, None),
                Ok(0)
            );
        }
    }

    #[test]
    fn wrapper_guards_and_political_owner() {
        let adj = Adj::default();
        with_rules(&[base(7, 1)], &[idx(0)], |rules| {
            for (improvement, plot) in [
                (
                    u32::MAX,
                    Some(ImprovementYieldPlot {
                        id: 8,
                        political_owner: 4,
                    }),
                ),
                (77, None),
                (
                    77,
                    Some(ImprovementYieldPlot {
                        id: 8,
                        political_owner: -1,
                    }),
                ),
            ] {
                assert_eq!(
                    improvement_yield(rules, &State, &adj, improvement, 1, plot),
                    Ok(0)
                );
            }
            assert!(adj.calls.borrow().is_empty());
            assert_eq!(
                improvement_yield(
                    rules,
                    &State,
                    &adj,
                    1,
                    1,
                    Some(ImprovementYieldPlot {
                        id: 8,
                        political_owner: 6
                    })
                ),
                Ok(20)
            );
            assert_eq!(adj.calls.borrow()[0].0, 6);
            // No blanket invalid-player guard suppresses base/adjacency.
            assert_eq!(
                improvement_player_yield(rules, &State, &adj, -1, 1, 1, None),
                Ok(10)
            );
        });
    }
    #[test]
    fn wrapping_and_signed_shift_after_full_sum() {
        with_rules(&[base(0x7f_ffff, 1)], &[], |rules| {
            assert_eq!(
                improvement_player_yield(rules, &State, &Adj::default(), 4, 1, 1, None),
                Ok(-8_388_599)
            );
        });
        let adj = Adj {
            normalize: true,
            ..Adj::default()
        };
        with_rules(&[base(-1, 0), base(0, 1)], &[], |rules| {
            assert_eq!(
                improvement_player_yield(rules, &State, &adj, 4, 1, -1, None),
                Ok(-1)
            );
            assert_eq!(
                improvement_player_yield(rules, &State, &Adj::default(), 4, 1, -1, None),
                Err(ImprovementYieldError::Normalization { yield_index: 0 })
            );
        });
    }
    #[test]
    fn missing_adjacency_and_properties_are_explicit() {
        let adj = Adj {
            missing: true,
            ..Adj::default()
        };
        with_rules(&[], &[idx(0)], |rules| {
            assert_eq!(
                improvement_player_yield(rules, &State, &adj, 4, 1, 1, None),
                Err(ImprovementYieldError::AdjacencyState { definition: 0 })
            );
        });
        let bonus = [ImprovementDefinition {
            type_hash: 77,
            bonus_yield_changes: Some(&[]),
        }; 2];
        let entries = [ImprovementYieldDefinition {
            type_hash: 77,
            base: None,
            adjacency: None,
        }; 2];
        let rules = OrderedImprovementYieldDefinitions::new(&Lookup, &entries, &bonus).unwrap();
        assert_eq!(
            improvement_player_yield(rules, &State, &adj, 4, 1, 1, None),
            Err(ImprovementYieldError::BaseReferences { improvement: 1 })
        );
        let entries = [ImprovementYieldDefinition {
            type_hash: 77,
            base: Some(&[]),
            adjacency: None,
        }; 2];
        let rules = OrderedImprovementYieldDefinitions::new(&Lookup, &entries, &bonus).unwrap();
        assert_eq!(
            improvement_player_yield(rules, &State, &adj, 4, 1, 1, None),
            Err(ImprovementYieldError::AdjacencyReferences { improvement: 1 })
        );
    }
}
