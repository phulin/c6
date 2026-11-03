//! Linux XP2 improvement technology/civic bonus helpers over supplied native inputs.
//!
//! Static source contracts only; initialized native differential acceptance is
//! outstanding. This is not Rules.GetYield, an action executor, or turn support.

use crate::movement_world::DefinitionLookup;

/// Four-byte signed FixedPointT<8> storage, returned by native hidden sret.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NativeFixedPoint8(pub i32);

impl NativeFixedPoint8 {
    pub const fn integer(value: i32) -> Self {
        Self(value.wrapping_shl(8))
    }

    fn add(self, value: Self) -> Self {
        Self(self.0.wrapping_add(value.0))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrerequisiteKind {
    Technology,
    Civic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MissingInput {
    PlayerRegistry,
    PlayerMask { player: i32, kind: PrerequisiteKind },
    GlobalAvailability { kind: PrerequisiteKind, index: i32 },
    ImprovementDefinitions,
    DefinitionRegistryShape,
    DefinitionIndex,
    BonusReferences { improvement: u32 },
}

/// An actual resolved definition's collection index, with its native count.
/// Construct from native ordered definition exports, never SQL rowids or hashes.
/// Keeping the bound proves Civic Types.GetIndex is identity in these helpers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeDefinitionIndex(u32);

impl NativeDefinitionIndex {
    pub fn new(index: u32, native_definition_count: u32) -> Option<Self> {
        (index < native_definition_count && index <= i32::MAX as u32).then_some(Self(index))
    }

    pub const fn get(self) -> i32 {
        self.0 as i32
    }
}

/// Nullable references have native null semantics; None never means uncaptured.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BonusYieldChange {
    pub yield_index: Option<NativeDefinitionIndex>,
    pub prerequisite_technology: Option<NativeDefinitionIndex>,
    pub prerequisite_civic: Option<NativeDefinitionIndex>,
    pub bonus_yield_change: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct ImprovementDefinition<'a> {
    pub type_hash: u32,
    /// Original GetBonusYieldChanges reference order; None entries are null refs.
    /// An absent slice is unavailable input, distinct from a known empty list.
    pub bonus_yield_changes: Option<&'a [Option<BonusYieldChange>]>,
}

#[derive(Clone, Copy)]
pub struct OrderedImprovementDefinitions<'a> {
    lookup: Option<&'a dyn DefinitionLookup>,
    definitions: &'a [ImprovementDefinition<'a>],
}

impl<'a> OrderedImprovementDefinitions<'a> {
    /// Validate the native indexed lookup and property alignment once on import.
    pub fn new(
        lookup: &'a dyn DefinitionLookup,
        definitions: &'a [ImprovementDefinition<'a>],
    ) -> Result<Self, MissingInput> {
        if lookup.count() != definitions.len() || lookup.count() > i32::MAX as usize {
            return Err(MissingInput::DefinitionRegistryShape);
        }
        for (index, definition) in definitions.iter().enumerate() {
            if lookup.type_hash(index) != Some(definition.type_hash) {
                return Err(MissingInput::DefinitionRegistryShape);
            }
        }
        Ok(Self {
            lookup: Some(lookup),
            definitions,
        })
    }

    pub const fn unavailable() -> Self {
        Self {
            lookup: None,
            definitions: &[],
        }
    }

    fn find(&self, index_or_hash: u32) -> Result<Option<&ImprovementDefinition<'a>>, MissingInput> {
        let lookup = self.lookup.ok_or(MissingInput::ImprovementDefinitions)?;
        match lookup.find_index(index_or_hash) {
            Some(index) => self
                .definitions
                .get(index)
                .map(Some)
                .ok_or(MissingInput::DefinitionIndex),
            None => Ok(None),
        }
    }
}

/// Read-only typed world boundary. Global availability means actual node presence
/// AND IsActive, distinct from research eligibility/CanStart. Known node absence
/// returns false; unavailable node/provider state must return MissingInput.
pub trait PrerequisiteState {
    /// Original Manager.IsValid, including the manager's player mapping.
    fn is_valid_player(&self, player: i32) -> Result<bool, MissingInput>;
    /// Complete original acquired bool vector; no missing-mask defaults.
    fn acquired_mask(&self, player: i32, kind: PrerequisiteKind) -> Result<&[bool], MissingInput>;
    fn globally_available(&self, kind: PrerequisiteKind, index: i32) -> Result<bool, MissingInput>;
}

// Native HasTech/HasCivic reject -1 then perform an unsigned size comparison.
// Civic references here are resolved indices below actual definition count, so
// its later Types.GetIndex conversion is identity (not a hash lookup).
fn has_prerequisite(
    state: &impl PrerequisiteState,
    player: i32,
    kind: PrerequisiteKind,
    index: i32,
) -> Result<bool, MissingInput> {
    if index == -1 {
        return Ok(false);
    }
    let acquired = state.acquired_mask(player, kind)?;
    let index_unsigned = index as u32 as usize;
    if index_unsigned >= acquired.len() {
        return Ok(false);
    }
    // Check availability before reading the player's bit, even when it is false.
    if !state.globally_available(kind, index)? {
        return Ok(false);
    }
    Ok(acquired[index_unsigned])
}

fn bonus_yield(
    rules: OrderedImprovementDefinitions<'_>,
    state: &impl PrerequisiteState,
    player: i32,
    improvement: u32,
    requested_yield: i32,
    kind: PrerequisiteKind,
) -> Result<NativeFixedPoint8, MissingInput> {
    let mut result = NativeFixedPoint8::default();
    if !state.is_valid_player(player)? {
        return Ok(result);
    }
    let Some(definition) = rules.find(improvement)? else {
        return Ok(result);
    };
    let references = definition
        .bonus_yield_changes
        .ok_or(MissingInput::BonusReferences { improvement })?;
    for reference in references.iter().flatten() {
        if reference.yield_index.map(NativeDefinitionIndex::get) != Some(requested_yield) {
            continue;
        }
        let prerequisite = match kind {
            PrerequisiteKind::Technology => reference.prerequisite_technology,
            PrerequisiteKind::Civic => reference.prerequisite_civic,
        };
        let Some(prerequisite) = prerequisite else {
            continue;
        };
        if has_prerequisite(state, player, kind, prerequisite.get())? {
            result = result.add(NativeFixedPoint8::integer(reference.bonus_yield_change));
        }
    }
    Ok(result)
}

/// Complete 0x2a3fd30 helper for supplied original typed rules/world inputs.
/// Does not compute other improvement or plot contributions.
pub fn improvement_bonus_tech_yield(
    rules: OrderedImprovementDefinitions<'_>,
    state: &impl PrerequisiteState,
    player: i32,
    improvement: u32,
    requested_yield: i32,
) -> Result<NativeFixedPoint8, MissingInput> {
    bonus_yield(
        rules,
        state,
        player,
        improvement,
        requested_yield,
        PrerequisiteKind::Technology,
    )
}

/// Complete 0x2a3fe40 helper under the same resolved-definition input contract.
pub fn improvement_bonus_civic_yield(
    rules: OrderedImprovementDefinitions<'_>,
    state: &impl PrerequisiteState,
    player: i32,
    improvement: u32,
    requested_yield: i32,
) -> Result<NativeFixedPoint8, MissingInput> {
    bonus_yield(
        rules,
        state,
        player,
        improvement,
        requested_yield,
        PrerequisiteKind::Civic,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Lookup<'a>(&'a [u32]);
    impl DefinitionLookup for Lookup<'_> {
        fn count(&self) -> usize {
            self.0.len()
        }
        fn type_hash(&self, index: usize) -> Option<u32> {
            self.0.get(index).copied()
        }
        fn find_index(&self, key: u32) -> Option<usize> {
            if (key as usize) < self.count() {
                Some(key as usize)
            } else {
                self.0.iter().rposition(|&hash| hash == key)
            }
        }
    }

    struct State {
        valid: bool,
        mask: Option<Vec<bool>>,
        global: Option<Vec<bool>>,
        queried: RefCell<Vec<(PrerequisiteKind, i32)>>,
    }
    impl PrerequisiteState for State {
        fn is_valid_player(&self, _: i32) -> Result<bool, MissingInput> {
            Ok(self.valid)
        }
        fn acquired_mask(
            &self,
            player: i32,
            kind: PrerequisiteKind,
        ) -> Result<&[bool], MissingInput> {
            self.mask
                .as_deref()
                .ok_or(MissingInput::PlayerMask { player, kind })
        }
        fn globally_available(
            &self,
            kind: PrerequisiteKind,
            index: i32,
        ) -> Result<bool, MissingInput> {
            self.queried.borrow_mut().push((kind, index));
            self.global
                .as_ref()
                .and_then(|mask| mask.get(index as usize))
                .copied()
                .ok_or(MissingInput::GlobalAvailability { kind, index })
        }
    }
    fn state() -> State {
        State {
            valid: true,
            mask: Some(vec![true; 3]),
            global: Some(vec![true; 3]),
            queried: RefCell::default(),
        }
    }
    fn index(value: u32) -> NativeDefinitionIndex {
        NativeDefinitionIndex::new(value, 3).unwrap()
    }
    fn row(value: i32, prerequisite: u32) -> Option<BonusYieldChange> {
        Some(BonusYieldChange {
            yield_index: Some(index(1)),
            prerequisite_technology: Some(index(prerequisite)),
            prerequisite_civic: Some(index(prerequisite)),
            bonus_yield_change: value,
        })
    }
    fn run(
        state: &State,
        rows: &[Option<BonusYieldChange>],
        kind: PrerequisiteKind,
    ) -> Result<NativeFixedPoint8, MissingInput> {
        let entries = [ImprovementDefinition {
            type_hash: 77,
            bonus_yield_changes: Some(rows),
        }];
        bonus_yield(
            OrderedImprovementDefinitions::new(&Lookup(&[77]), &entries).unwrap(),
            state,
            4,
            77,
            1,
            kind,
        )
    }
    #[test]
    fn both_helpers_preserve_reference_order_and_wrapping_negative_arithmetic() {
        for kind in [PrerequisiteKind::Technology, PrerequisiteKind::Civic] {
            let state = state();
            let rows = [
                row(0x7f_ffff, 2),
                None,
                row(1, 0),
                row(-2, 1),
                row(i32::MIN, 2),
            ];
            assert_eq!(run(&state, &rows, kind), Ok(NativeFixedPoint8(0x7fff_fe00)));
            assert_eq!(
                *state.queried.borrow(),
                [(kind, 2), (kind, 0), (kind, 1), (kind, 2)]
            );
        }
        assert_eq!(
            NativeFixedPoint8::integer(i32::MAX),
            NativeFixedPoint8(-256)
        );
        assert_eq!(NativeFixedPoint8::integer(-1), NativeFixedPoint8(-256));
    }
    #[test]
    fn global_availability_and_acquired_mask_are_both_required() {
        let mut state = state();
        state.global = Some(vec![false, true, true]);
        state.mask = Some(vec![true, false, true]);
        assert_eq!(
            run(
                &state,
                &[row(3, 0), row(5, 1), row(7, 2)],
                PrerequisiteKind::Technology
            ),
            Ok(NativeFixedPoint8::integer(7))
        );
        state.global = None;
        assert!(matches!(
            run(&state, &[row(5, 1)], PrerequisiteKind::Civic),
            Err(MissingInput::GlobalAvailability { .. })
        ));
        state.mask = None;
        assert!(matches!(
            run(&state, &[row(5, 1)], PrerequisiteKind::Technology),
            Err(MissingInput::PlayerMask { .. })
        ));
    }
    #[test]
    fn native_index_guards_short_circuit_before_global_provider() {
        let mut state = state();
        state.global = None;
        for kind in [PrerequisiteKind::Technology, PrerequisiteKind::Civic] {
            for raw_index in [-1, -2, 3, i32::MAX] {
                assert_eq!(has_prerequisite(&state, 4, kind, raw_index), Ok(false));
            }
        }
        assert!(state.queried.borrow().is_empty());
        state.mask = None;
        assert_eq!(
            has_prerequisite(&state, 4, PrerequisiteKind::Civic, -1),
            Ok(false)
        );
    }
    #[test]
    fn nullable_and_nonmatching_references_do_not_query_missing_masks() {
        let mut state = state();
        state.mask = None;
        let mut null_yield = row(9, 0).unwrap();
        null_yield.yield_index = None;
        let mut other_yield = null_yield;
        other_yield.yield_index = Some(index(2));
        let mut null_prereq = row(9, 0).unwrap();
        null_prereq.prerequisite_technology = None;
        null_prereq.prerequisite_civic = None;
        let rows = [None, Some(null_yield), Some(other_yield), Some(null_prereq)];
        assert_eq!(
            run(&state, &rows, PrerequisiteKind::Technology),
            Ok(NativeFixedPoint8(0))
        );
        assert_eq!(
            run(&state, &rows, PrerequisiteKind::Civic),
            Ok(NativeFixedPoint8(0))
        );
    }
    #[test]
    fn invalid_player_and_missing_definition_have_proven_zero_semantics() {
        let mut state = state();
        state.valid = false;
        let unavailable = OrderedImprovementDefinitions::unavailable();
        assert_eq!(
            improvement_bonus_tech_yield(unavailable, &state, -1, 77, 1),
            Ok(NativeFixedPoint8(0))
        );
        state.valid = true;
        assert_eq!(
            improvement_bonus_civic_yield(unavailable, &state, 4, 77, 1),
            Err(MissingInput::ImprovementDefinitions)
        );
        state.mask = None;
        assert_eq!(
            improvement_bonus_civic_yield(
                OrderedImprovementDefinitions::new(&Lookup(&[]), &[]).unwrap(),
                &state,
                4,
                77,
                1
            ),
            Ok(NativeFixedPoint8(0))
        );
        let entries = [ImprovementDefinition {
            type_hash: 77,
            bonus_yield_changes: None,
        }];
        assert_eq!(
            improvement_bonus_tech_yield(
                OrderedImprovementDefinitions::new(&Lookup(&[77]), &entries).unwrap(),
                &state,
                4,
                77,
                1
            ),
            Err(MissingInput::BonusReferences { improvement: 77 })
        );
    }
    #[test]
    fn registry_lookup_uses_direct_index_and_duplicate_hash_resolution() {
        let state = state();
        let first = [row(2, 0)];
        let last = [row(7, 0)];
        let entries = [
            ImprovementDefinition {
                type_hash: 77,
                bonus_yield_changes: Some(&first),
            },
            ImprovementDefinition {
                type_hash: 77,
                bonus_yield_changes: Some(&last),
            },
        ];
        let lookup = Lookup(&[77, 77]);
        let rules = OrderedImprovementDefinitions::new(&lookup, &entries).unwrap();
        assert_eq!(
            improvement_bonus_tech_yield(rules, &state, 4, 0, 1),
            Ok(NativeFixedPoint8::integer(2))
        );
        assert_eq!(
            improvement_bonus_tech_yield(rules, &state, 4, 77, 1),
            Ok(NativeFixedPoint8::integer(7))
        );
        assert!(matches!(
            OrderedImprovementDefinitions::new(&Lookup(&[76, 77]), &entries),
            Err(MissingInput::DefinitionRegistryShape)
        ));
        assert!(matches!(
            OrderedImprovementDefinitions::new(&Lookup(&[77]), &entries),
            Err(MissingInput::DefinitionRegistryShape)
        ));
    }
    #[test]
    fn definition_indices_require_actual_bounds() {
        assert_eq!(NativeDefinitionIndex::new(3, 3), None);
        assert_eq!(NativeDefinitionIndex::new(u32::MAX, u32::MAX), None);
        assert_eq!(NativeDefinitionIndex::new(2, 3).unwrap().get(), 2);
    }
}
