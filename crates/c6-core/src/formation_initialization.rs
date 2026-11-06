//! Source-backed Formation1 Read application and logical archive ownership.
//!
//! This is the state immediately after Formation/collection Read, not a claim
//! about global load completion. Sync-manager acceptance and gameplay report
//! subscribers are separate live inputs; no subscriber is synthesized here.
#[cfg(test)]
use crate::unit::PlotCoord;
use crate::unit::{ComponentId, Formation};
use std::collections::BTreeSet;

pub const FORMATION_COMPONENT_TYPE: i16 = 44;
pub const UNIT_COMPONENT_TYPE: i16 = 1;
/// Unit constructor wrapper registrations; dirty Read baseline is separate.
pub const UNIT_FIELDS: [&str; 73] = [
    "m_id",
    "m_eTypeIndex",
    "m_eAIType",
    "m_kLocation",
    "m_eFacingDirection",
    "m_eOwner",
    "m_eOriginalOwner",
    "m_kAttributes",
    "m_kProperties",
    "m_bProcessedTurn",
    "m_eAutomation",
    "m_eActivity",
    "m_eDomain",
    "m_bEmbarked",
    "m_bImmobile",
    "m_bDelayedDeath",
    "m_aiController",
    "m_aiGoal",
    "m_bIsLevied",
    "m_aiAbilitiesPossessed",
    "m_abInvalidTerrainTypes",
    "m_abIgnoreTerrainTypes",
    "m_abIgnoreFeatureTypes",
    "m_aiBonusPointsPerKill",
    "m_aiBonusPointsPerKillStrength",
    "m_aiBypassWallsPromotionClass",
    "m_aiEnableWallAttackPromotionClass",
    "m_eMilitaryFormation",
    "m_iDamage",
    "m_iFortifyTurns",
    "m_xMovesRemaining",
    "m_iAttacksRemaining",
    "m_iOperationTimer",
    "m_iBuildCharges",
    "m_iParkCharges",
    "m_iActionCharges",
    "m_iAirSlotModifier",
    "m_bDominantConquer",
    "m_iUpgradeDiscount",
    "m_iDisasterCharges",
    "m_iEraStrengthModifier",
    "m_iBarbarianTribeIndex",
    "m_eBarbarianPlayerTarget",
    "m_bAttackAfterMoving",
    "m_iMoveAfterAttacking",
    "m_bIgnoreTerrainCost",
    "m_bIgnoreZOC",
    "m_bExertZOC",
    "m_bCanSeeThroughFeatures",
    "m_bCanSeeThroughTerrain",
    "m_bNeedsMovementCompletion",
    "m_bEligibleToHeal",
    "m_bHasHiddenVisibility",
    "m_bHiddenVisibilityForfeited",
    "m_bNeedsAttention",
    "m_bIgnoreResourceMaintenance",
    "m_kHealthModifiers",
    "m_kHealthReligionModifiers",
    "m_kCombatModifers",
    "m_kMovementModifiers",
    "m_kSightModifiers",
    "m_kAttackRangeModifiers",
    "m_kArchaeology",
    "m_kEspionage",
    "m_kExperience",
    "m_kGreatPerson",
    "m_kReligion",
    "m_kRockBand",
    "m_kTrade",
    "m_kSightPlots",
    "m_formationID",
    "m_kOperationsDisabled",
    "m_kOperations",
];

/// Constructor registration order; inline wrappers have increasing addresses.
pub const FORMATION_FIELDS: [&str; 17] = [
    "m_id",
    "m_kLocation",
    "m_eOwner",
    "m_xMovesRemaining",
    "m_eDomain",
    "m_bIsEmbarked",
    "m_bCanEmbark",
    "m_bCanAttack",
    "m_bIsRangedCombat",
    "m_bCanCapture",
    "m_bHasMoved",
    "m_bHasMovedIntoZOC",
    "m_bIgnoresZOC",
    "m_bEvaluatingOperation",
    "m_kLastPath",
    "m_kLastPathCacheLocation",
    "m_kUnitIDList",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirtyBaseline {
    /// Only the named local Read boundary has been reconstructed.
    FormationRead,
    /// Attachment is known; preexisting dirty membership still needs a provider.
    Unavailable,
}

/// Logical inline-wrapper ownership, independent of external sync registration.
/// Dirty calls retain invocation order; membership is unique and ordered by
/// inline wrapper address (the constructor index for these objects).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicalArchive {
    identity: ComponentId,
    fields: &'static [&'static str],
    baseline: DirtyBaseline,
    dirty: BTreeSet<u32>,
    dirty_calls: Vec<u32>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    IdentityMismatch,
    UnregisteredField(u32),
    DirtyBaselineUnavailable,
}
impl LogicalArchive {
    /// Does not invent an empty native baseline: callers must state provenance.
    pub fn new(
        identity: ComponentId,
        fields: &'static [&'static str],
        baseline: DirtyBaseline,
    ) -> Self {
        Self {
            identity,
            fields,
            baseline,
            dirty: BTreeSet::new(),
            dirty_calls: Vec::new(),
        }
    }
    pub fn identity(&self) -> ComponentId {
        self.identity
    }
    pub fn identity_bits(&self) -> u64 {
        u64::from(self.identity.owner as u16)
            | (u64::from(self.identity.component_type as u16) << 16)
            | (u64::from(self.identity.id) << 32)
    }
    pub fn registered_fields(&self) -> &'static [&'static str] {
        self.fields
    }
    pub fn contains(&self, index: u32) -> bool {
        (index as usize) < self.fields.len()
    }
    pub fn mark_dirty(&mut self, index: u32) -> Result<(), Error> {
        if !self.contains(index) {
            return Err(Error::UnregisteredField(index));
        }
        self.dirty_calls.push(index);
        self.dirty.insert(index);
        Ok(())
    }
    /// Calls since this sidecar's creation, even if native initial deltas are unknown.
    pub fn dirty_calls(&self) -> &[u32] {
        &self.dirty_calls
    }
    pub fn dirty_indices(&self) -> Result<Vec<u32>, Error> {
        if self.baseline == DirtyBaseline::Unavailable {
            return Err(Error::DirtyBaselineUnavailable);
        }
        Ok(self.dirty.iter().copied().collect())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnerBinding {
    NoOwner,
    /// The actual Globals::GetPlayer lookup result, including a null result.
    Lookup {
        requested: i32,
        resolved: Option<i32>,
    },
}

/// Failure of the narrow enclosing-Player Read lookup proof. Missing facts
/// remain unknown; they are never converted to a native null lookup result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnerLookupError {
    EnclosingIdentityMismatch { requested: i32, record: i32 },
    InvalidRequestedOwner(i32),
    UnknownOwner(i32),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormationRuntime {
    pub archive: LogicalArchive,
    pub owner: OwnerBinding,
    evaluating_operation: bool,
}
impl FormationRuntime {
    /// Formation.Initialize binds the actual parent Player directly. This is
    /// neither a Read postlude nor an invented clean archive baseline.
    pub fn from_created(formation: &Formation) -> Self {
        Self {
            archive: LogicalArchive::new(
                ComponentId {
                    owner: formation.owner as i16,
                    component_type: FORMATION_COMPONENT_TYPE,
                    id: formation.id,
                },
                &FORMATION_FIELDS,
                DirtyBaseline::Unavailable,
            ),
            owner: OwnerBinding::Lookup {
                requested: formation.owner,
                resolved: Some(formation.owner),
            },
            evaluating_operation: false,
        }
    }
    /// Bind only the Player whose Manager entry encloses this Formation Read.
    /// Manager::AllocateInstance publishes that requested owner before Player
    /// Read begins. The decoded entry must preserve requested/header identity.
    /// This proves neither any other owner's slot nor Manager validity/control
    /// mappings. Cross-owner records require the explicit `from_read` provider.
    pub fn from_owning_player_read(
        formation: &Formation,
        requested_owner: i32,
        record_owner: i32,
    ) -> Result<Self, OwnerLookupError> {
        if requested_owner != record_owner {
            return Err(OwnerLookupError::EnclosingIdentityMismatch {
                requested: requested_owner,
                record: record_owner,
            });
        }
        if !(0..65).contains(&requested_owner) {
            return Err(OwnerLookupError::InvalidRequestedOwner(requested_owner));
        }
        Self::from_read(formation, |owner| {
            if owner == requested_owner {
                Ok(Some(record_owner))
            } else {
                Err(OwnerLookupError::UnknownOwner(owner))
            }
        })
    }
    /// Apply the unsaved Read postlude without cloning or changing saved arrays.
    /// `lookup` must represent the admitted Globals player table, not a roster
    /// inferred from saved records. Its error rejects initialization.
    pub fn from_read<E>(
        formation: &Formation,
        mut lookup: impl FnMut(i32) -> Result<Option<i32>, E>,
    ) -> Result<Self, E> {
        let owner = if formation.owner == -1 {
            OwnerBinding::NoOwner
        } else {
            OwnerBinding::Lookup {
                requested: formation.owner,
                resolved: lookup(formation.owner)?,
            }
        };
        let mut archive = LogicalArchive::new(
            ComponentId {
                owner: formation.owner as i16,
                component_type: FORMATION_COMPONENT_TYPE,
                id: formation.id,
            },
            &FORMATION_FIELDS,
            DirtyBaseline::FormationRead,
        );
        // Domain is read through edit(); evaluating resets through unconditional
        // MoveAssignmentDo<bool>, even when its constructor value was false.
        archive.mark_dirty(4).expect("registered domain");
        archive
            .mark_dirty(13)
            .expect("registered evaluating-operation");
        Ok(Self {
            archive,
            owner,
            evaluating_operation: false,
        })
    }
    pub fn evaluating_operation(&self) -> bool {
        self.evaluating_operation
    }
    fn check(&self, formation: &Formation) -> Result<(), Error> {
        let id = self.archive.identity();
        if id.id != formation.id
            || id.owner != formation.owner as i16
            || id.component_type != FORMATION_COMPONENT_TYPE
        {
            return Err(Error::IdentityMismatch);
        }
        Ok(())
    }
    /// Original assignment marks dirty even for an unchanged boolean.
    pub fn set_evaluating_operation(
        &mut self,
        formation: &Formation,
        value: bool,
    ) -> Result<(), Error> {
        self.check(formation)?;
        self.archive.mark_dirty(13)?;
        self.evaluating_operation = value;
        Ok(())
    }
    /// ClearPathCache edits path before cache, including already empty/invalid.
    pub fn clear_path_cache(&mut self, formation: &mut Formation) -> Result<(), Error> {
        self.check(formation)?;
        self.archive.mark_dirty(14)?;
        formation.last_path.clear();
        self.archive.mark_dirty(15)?;
        formation.last_path_cache_location = crate::formation_path_cache::INVALID_CACHE_LOCATION;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn formation() -> Formation {
        Formation {
            id: 0xabcd0007,
            location: PlotCoord { x: 4, y: 6 },
            owner: 2,
            moves_remaining_raw: -257,
            domain: 99,
            is_embarked: false,
            can_embark: true,
            can_attack: true,
            is_ranged_combat: false,
            can_capture: true,
            has_moved: false,
            has_moved_into_zoc: false,
            ignores_zoc: true,
            last_path: vec![crate::unit::PathNode {
                data1_raw: -1,
                data2: 7,
                flags: -3,
                x: -8,
                y: 9,
            }],
            last_path_cache_location: PlotCoord { x: 12, y: 13 },
            unit_ids: vec![0xffff0001, 3, 0xffff0001],
        }
    }
    #[test]
    fn read_keeps_saved_state_and_actual_lookup_result() {
        let f = formation();
        let before = f.clone();
        let mut calls = vec![];
        let r = FormationRuntime::from_read(&f, |owner| {
            calls.push(owner);
            Ok::<_, ()>(None)
        })
        .unwrap();
        assert_eq!(f, before);
        assert_eq!(calls, [2]);
        assert_eq!(
            r.owner,
            OwnerBinding::Lookup {
                requested: 2,
                resolved: None
            }
        );
        assert_eq!(r.archive.dirty_indices(), Ok(vec![4, 13]));
        assert_eq!(r.archive.dirty_calls(), [4, 13]);
        assert!(!r.evaluating_operation());
        assert_eq!(r.archive.identity_bits(), 0xabcd0007_002c0002);
    }
    #[test]
    fn no_owner_skips_lookup_and_preserves_native_narrowing() {
        let mut f = formation();
        f.owner = -1;
        let r =
            FormationRuntime::from_read(&f, |_| -> Result<Option<i32>, ()> { panic!("no lookup") })
                .unwrap();
        assert_eq!(r.owner, OwnerBinding::NoOwner);
        f.owner = 0x10002;
        let r = FormationRuntime::from_read(&f, |x| {
            assert_eq!(x, 0x10002);
            Ok::<_, ()>(Some(5))
        })
        .unwrap();
        assert_eq!(r.archive.identity().owner, 2);
        assert_eq!(
            FormationRuntime::from_read(&f, |_| Err::<Option<i32>, _>(7)),
            Err(7)
        );
    }
    #[test]
    fn edits_keep_call_order_unique_membership_and_reject_wrong_generation() {
        let mut f = formation();
        let members = f.unit_ids.clone();
        let mut r = FormationRuntime::from_read(&f, |_| Ok::<_, ()>(Some(2))).unwrap();
        r.set_evaluating_operation(&f, false).unwrap();
        r.clear_path_cache(&mut f).unwrap();
        r.clear_path_cache(&mut f).unwrap();
        assert_eq!(r.archive.dirty_calls(), [4, 13, 13, 14, 15, 14, 15]);
        assert_eq!(r.archive.dirty_indices(), Ok(vec![4, 13, 14, 15]));
        assert!(f.last_path.is_empty());
        assert_eq!(f.unit_ids, members);
        assert_eq!(f.last_path_cache_location, PlotCoord { x: -9999, y: -9999 });
        f.id ^= 0x10000;
        let before = f.clone();
        let trace = r.archive.dirty_calls().to_vec();
        assert_eq!(r.clear_path_cache(&mut f), Err(Error::IdentityMismatch));
        assert_eq!(f, before);
        assert_eq!(r.archive.dirty_calls(), trace);
    }
    #[test]
    fn enclosing_player_read_proves_only_its_own_published_owner() {
        let mut f = formation();
        let runtime = FormationRuntime::from_owning_player_read(&f, 2, 2).unwrap();
        assert_eq!(
            runtime.owner,
            OwnerBinding::Lookup {
                requested: 2,
                resolved: Some(2)
            }
        );
        assert_eq!(runtime.archive.dirty_calls(), [4, 13]);
        assert_eq!(
            FormationRuntime::from_owning_player_read(&f, 2, 3),
            Err(OwnerLookupError::EnclosingIdentityMismatch {
                requested: 2,
                record: 3
            })
        );
        assert_eq!(
            FormationRuntime::from_owning_player_read(&f, 65, 65),
            Err(OwnerLookupError::InvalidRequestedOwner(65))
        );
        f.owner = 3;
        assert_eq!(
            FormationRuntime::from_owning_player_read(&f, 2, 2),
            Err(OwnerLookupError::UnknownOwner(3))
        );
        // A missing cross-owner proof is distinct from an actual null result.
        assert_eq!(
            FormationRuntime::from_read(&f, |_| Ok::<_, ()>(None))
                .unwrap()
                .owner,
            OwnerBinding::Lookup {
                requested: 3,
                resolved: None
            }
        );
        f.owner = -1;
        assert_eq!(
            FormationRuntime::from_owning_player_read(&f, 2, 2)
                .unwrap()
                .owner,
            OwnerBinding::NoOwner
        );
        f.owner = 64;
        assert_eq!(
            FormationRuntime::from_owning_player_read(&f, 64, 64)
                .unwrap()
                .owner,
            OwnerBinding::Lookup {
                requested: 64,
                resolved: Some(64)
            }
        );
    }

    #[test]
    fn attachment_does_not_invent_unit_dirty_baseline() {
        let mut a = LogicalArchive::new(
            ComponentId {
                owner: 0,
                component_type: 0,
                id: 1,
            },
            &FORMATION_FIELDS,
            DirtyBaseline::Unavailable,
        );
        assert!(a.contains(14));
        assert!(!a.contains(17));
        assert_eq!(a.mark_dirty(17), Err(Error::UnregisteredField(17)));
        assert!(a.dirty_calls().is_empty());
        a.mark_dirty(14).unwrap();
        assert_eq!(a.dirty_indices(), Err(Error::DirtyBaselineUnavailable));
        assert_eq!(a.dirty_calls(), [14]);
    }
}
