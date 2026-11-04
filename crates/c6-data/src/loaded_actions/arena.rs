//! Read-only actor footprint and retained action attempts for loaded SkipTurn.
//!
//! This bounded Unit/Formation projection is not the complete affected World.
//! Callback effects outside these records and the gameplay RNG stream inventory
//! remain separate acceptance obligations. A successful diagnostic trace never
//! establishes complete action equivalence.
use super::{Event, Failure};
use c6_core::{
    core_player::initialization::WireBinding,
    formation_initialization::{
        FormationRuntime, LogicalArchive, FORMATION_COMPONENT_TYPE, UNIT_COMPONENT_TYPE,
    },
    loaded_state::{self, LoadedState},
    map::UnitReference,
    unit::{Formation, Unit},
    unit_initialization::UnitRuntime,
};

#[derive(Debug)]
pub enum CaptureError {
    World(c6_core::world::MissingInput),
    State(loaded_state::Error),
    MissingActor,
    MissingFormation,
    Identity,
    Membership,
}

/// Complete owned records for the addressed actor only. No World is cloned.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorSnapshot {
    binding: WireBinding,
    reference: UnitReference,
    resolved_id: u32,
    unit: Unit,
    unit_runtime: UnitRuntime,
    formation: Formation,
    formation_runtime: FormationRuntime,
}
impl ActorSnapshot {
    pub fn capture(state: &LoadedState, reference: UnitReference) -> Result<Self, CaptureError> {
        let actor = state
            .world()
            .unit(reference)
            .map_err(CaptureError::World)?
            .ok_or(CaptureError::MissingActor)?;
        let owner = state
            .world()
            .player(reference.player())
            .map_err(CaptureError::World)?;
        let formation = owner
            .formation(actor.record.formation_id)
            .map_err(CaptureError::World)?
            .ok_or(CaptureError::MissingFormation)?;
        let member = owner.members(formation.record).any(
            |entry| matches!(entry, Ok(Some(found)) if found.resolved_id == actor.resolved_id),
        );
        if !member {
            return Err(CaptureError::Membership);
        }
        Self::checked(
            state.binding().clone(),
            reference,
            actor.resolved_id,
            actor.record.clone(),
            state
                .unit_runtime(reference.player(), reference.id)
                .map_err(CaptureError::State)?
                .clone(),
            formation.record.clone(),
            state
                .formation_runtime(reference.player(), actor.record.formation_id)
                .map_err(CaptureError::State)?
                .clone(),
        )
    }
    // Lower checked constructor for already resolved bounded records. Production
    // capture additionally proves lookup and membership against the actual World.
    fn checked(
        binding: WireBinding,
        reference: UnitReference,
        resolved_id: u32,
        unit: Unit,
        unit_runtime: UnitRuntime,
        formation: Formation,
        formation_runtime: FormationRuntime,
    ) -> Result<Self, CaptureError> {
        let u = unit_runtime.archive.identity();
        let f = formation_runtime.archive.identity();
        let link = unit_runtime
            .formation
            .ok_or(CaptureError::MissingFormation)?;
        if reference.component_id().component_type != UNIT_COMPONENT_TYPE
            || unit.id != resolved_id
            || unit.e_owner != reference.player()
            || u.id != unit.id
            || i32::from(u.owner) != unit.e_owner
            || u.component_type != UNIT_COMPONENT_TYPE
            || f.id != formation.id
            || i32::from(f.owner) != formation.owner
            || f.component_type != FORMATION_COMPONENT_TYPE
            || link.owner != formation.owner
            || link.requested_id != unit.formation_id
            || link.resolved_id != formation.id
            || formation.owner != unit.e_owner
        {
            return Err(CaptureError::Identity);
        }
        Ok(Self {
            binding,
            reference,
            resolved_id,
            unit,
            unit_runtime,
            formation,
            formation_runtime,
        })
    }
    pub fn binding(&self) -> &WireBinding {
        &self.binding
    }
    pub fn reference(&self) -> UnitReference {
        self.reference
    }
    pub fn unit(&self) -> &Unit {
        &self.unit
    }
    pub fn unit_runtime(&self) -> &UnitRuntime {
        &self.unit_runtime
    }
    pub fn formation(&self) -> &Formation {
        &self.formation
    }
    pub fn formation_runtime(&self) -> &FormationRuntime {
        &self.formation_runtime
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    UnitRecord,
    UnitRuntime,
    FormationRecord,
    FormationRuntime,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChangedField {
    pub kind: Kind,
    pub field: &'static str,
    /// Within the source-backed bounded write footprint; not action acceptance.
    pub declared_write: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeltaError {
    Binding,
    ActorIdentity,
    FormationIdentity,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorDelta {
    pub before: ActorSnapshot,
    pub after: ActorSnapshot,
    pub changed_fields: Vec<ChangedField>,
}
impl ActorDelta {
    pub fn between(before: ActorSnapshot, after: ActorSnapshot) -> Result<Self, DeltaError> {
        if before.binding != after.binding {
            return Err(DeltaError::Binding);
        }
        if before.reference != after.reference
            || before.resolved_id != after.resolved_id
            || before.unit.id != after.unit.id
            || before.unit.e_owner != after.unit.e_owner
        {
            return Err(DeltaError::ActorIdentity);
        }
        if before.unit.formation_id != after.unit.formation_id
            || before.formation.id != after.formation.id
            || before.formation.owner != after.formation.owner
            || before.unit_runtime.formation != after.unit_runtime.formation
        {
            return Err(DeltaError::FormationIdentity);
        }
        let mut changed_fields = Vec::new();
        macro_rules! compare {
            ($kind:ident, $a:expr, $b:expr, $($field:ident => $allowed:expr),+ $(,)?) => {{
                $(if $a.$field != $b.$field { changed_fields.push(ChangedField {
                    kind: Kind::$kind, field: stringify!($field), declared_write: $allowed,
                }); })+
            }};
        }
        changed_fields.extend(record_changes(
            &before.unit,
            &after.unit,
            &before.formation,
            &after.formation,
        ));
        let UnitRuntime {
            archive: _,
            definition_index: _,
            domain: _,
            formation: _,
            abilities: _,
            invalid_terrain: _,
            ignore_terrain: _,
            ignore_feature: _,
            bonus_points_per_kill: _,
            bonus_points_per_kill_strength: _,
            operations_disabled: _,
        } = &before.unit_runtime;
        compare!(UnitRuntime, before.unit_runtime, after.unit_runtime,
            definition_index => false, domain => false, formation => false,
            abilities => false, invalid_terrain => false, ignore_terrain => false,
            ignore_feature => false, bonus_points_per_kill => false,
            bonus_points_per_kill_strength => false, operations_disabled => false);
        compare!(FormationRuntime, before.formation_runtime, after.formation_runtime, owner => false);
        if before.formation_runtime.evaluating_operation()
            != after.formation_runtime.evaluating_operation()
        {
            changed_fields.push(ChangedField {
                kind: Kind::FormationRuntime,
                field: "evaluating_operation",
                declared_write: true,
            });
        }
        for (kind, a, b, allowed) in [
            (
                Kind::UnitRuntime,
                &before.unit_runtime.archive,
                &after.unit_runtime.archive,
                &[11][..],
            ),
            (
                Kind::FormationRuntime,
                &before.formation_runtime.archive,
                &after.formation_runtime.archive,
                &[13, 14, 15][..],
            ),
        ] {
            if a != b {
                changed_fields.push(ChangedField {
                    kind,
                    field: "archive",
                    declared_write: archive_extension(a, b, allowed),
                });
            }
        }
        Ok(Self {
            before,
            after,
            changed_fields,
        })
    }
    pub fn unexpected_fields(&self) -> impl Iterator<Item = &ChangedField> {
        self.changed_fields
            .iter()
            .filter(|field| !field.declared_write)
    }
}
fn archive_extension(before: &LogicalArchive, after: &LogicalArchive, allowed: &[u32]) -> bool {
    let Some(calls) = after.dirty_calls().strip_prefix(before.dirty_calls()) else {
        return false;
    };
    let mut expected = before.clone();
    for &field in calls {
        if !allowed.contains(&field) || expected.mark_dirty(field).is_err() {
            return false;
        }
    }
    &expected == after
}

/// External evidence only: stream ownership and completeness must be established
/// independently. No automatic mapping from a descriptor to gameplay is implied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RngState {
    Random(c6_core::rng::RandomState),
    Pcg32(c6_core::random_variants::RandomPcg32),
    Scoped(c6_core::random_variants::ScopedRandom),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RngEvidence {
    pub stream_identity: String,
    pub before: RngState,
    pub after: RngState,
}

/// An attempt retains the original action failure and captures partial effects.
/// Capture failures (for example changed runtime dependencies) are retained next
/// to the outcome; they never erase the original error or restore a cloned World.
/// This is an observation, not action acceptance. A failed action may leave partial
/// state; discard or reconstruct that state before continuing gameplay.
#[must_use = "inspect the action outcome and capture results before continuing gameplay"]
#[derive(Debug)]
pub struct ArenaAttempt {
    pub before: ActorSnapshot,
    pub after: Result<ActorSnapshot, CaptureError>,
    /// Retained even when a runtime dependency change prevents full capture.
    pub after_records: Result<ActorRecords, CaptureError>,
    pub outcome: Result<Vec<Event>, Failure>,
    pub rng_evidence: Option<Vec<RngEvidence>>,
}
impl ArenaAttempt {
    /// Observes the actual caller-supplied action once against this same mutable state.
    /// A pre-capture failure returns before calling the action. The closure may
    /// observe external RNG before/after its action and return that evidence.
    ///
    /// Outer `Ok` means the attempt was recorded, not that a supported action was
    /// accepted. Inspect `outcome`, both after-captures, and the bounded delta.
    /// Any failed partial state must be discarded or reconstructed before gameplay
    /// continues. This observer neither rolls back nor clones the World.
    pub fn observe_in_place(
        state: &mut LoadedState,
        reference: UnitReference,
        action: impl FnOnce(&mut LoadedState) -> (Result<Vec<Event>, Failure>, Option<Vec<RngEvidence>>),
    ) -> Result<Self, CaptureError> {
        let before = ActorSnapshot::capture(state, reference)?;
        let (outcome, rng_evidence) = action(state);
        let after_records = ActorRecords::capture(state, reference);
        let after = ActorSnapshot::capture(state, reference);
        Ok(Self {
            before,
            after,
            after_records,
            outcome,
            rng_evidence,
        })
    }
    pub fn delta(&self) -> Result<ActorDelta, AttemptDeltaError<'_>> {
        let after = self.after.as_ref().map_err(AttemptDeltaError::Capture)?;
        ActorDelta::between(self.before.clone(), after.clone()).map_err(AttemptDeltaError::Identity)
    }
}
#[derive(Debug)]
pub enum AttemptDeltaError<'a> {
    Capture(&'a CaptureError),
    Identity(DeltaError),
}

fn record_changes(
    before_unit: &Unit,
    after_unit: &Unit,
    before_formation: &Formation,
    after_formation: &Formation,
) -> Vec<ChangedField> {
    let mut changed_fields = Vec::new();
    macro_rules! compare {
            ($kind:ident, $a:expr, $b:expr, $($field:ident => $allowed:expr),+ $(,)?) => {{
                $(if $a.$field != $b.$field { changed_fields.push(ChangedField {
                    kind: Kind::$kind, field: stringify!($field), declared_write: $allowed,
                }); })+
            }};
        }
    // Exhaustive pattern forces review when serialized Unit fields grow.
    let Unit {
        wire_version: _,
        id: _,
        e_type_index: _,
        e_ai_type: _,
        ai_controller: _,
        ai_goal: _,
        k_location: _,
        e_facing_direction: _,
        e_owner: _,
        e_original_owner: _,
        b_is_levied: _,
        formation_id: _,
        e_military_formation: _,
        e_automation: _,
        e_activity: _,
        i_damage: _,
        i_fortify_turns: _,
        i_upgrade_discount: _,
        x_moves_remaining: _,
        i_attacks_remaining: _,
        i_operation_timer: _,
        i_build_charges: _,
        i_park_charges: _,
        i_action_charges: _,
        i_air_slot_modifier: _,
        b_dominant_conquer: _,
        i_barbarian_tribe_index: _,
        e_barbarian_player_target: _,
        i_disaster_charges: _,
        i_era_strength_modifier: _,
        b_embarked: _,
        b_delayed_death: _,
        b_immobile: _,
        b_processed_turn: _,
        b_needs_movement_completion: _,
        b_eligible_to_heal: _,
        b_has_hidden_visibility: _,
        b_hidden_visibility_forfeited: _,
        b_ignore_terrain_cost: _,
        b_ignore_zoc: _,
        b_exert_zoc: _,
        b_can_see_through_features: _,
        b_can_see_through_terrain: _,
        b_attack_after_moving: _,
        i_move_after_attacking: _,
        b_needs_attention: _,
        b_ignore_resource_maintenance: _,
        k_health_modifiers: _,
        k_health_religion_modifiers: _,
        k_combat_modifers: _,
        k_movement_modifiers: _,
        k_sight_modifiers: _,
        k_attack_range_modifiers: _,
        k_sight_plots: _,
        ai_abilities_possessed: _,
        ab_invalid_terrain_types: _,
        ab_ignore_terrain_types: _,
        ab_ignore_feature_types: _,
        ai_bonus_points_per_kill: _,
        ai_bonus_points_per_kill_strength: _,
        ai_bypass_walls_promotion_class: _,
        ai_enable_wall_attack_promotion_class: _,
        k_experience: _,
        k_great_person: _,
        k_religion: _,
        k_rock_band: _,
        k_trade: _,
        k_archaeology: _,
        k_espionage: _,
        k_attributes: _,
        k_operations: _,
        k_operations_disabled: _,
        k_hero: _,
        k_properties: _,
    } = before_unit;
    compare!(UnitRecord, before_unit, after_unit,
        wire_version => false,
        id => false,
        e_type_index => false,
        e_ai_type => false,
        ai_controller => false,
        ai_goal => false,
        k_location => false,
        e_facing_direction => false,
        e_owner => false,
        e_original_owner => false,
        b_is_levied => false,
        formation_id => false,
        e_military_formation => false,
        e_automation => false,
        e_activity => true,
        i_damage => false,
        i_fortify_turns => false,
        i_upgrade_discount => false,
        x_moves_remaining => false,
        i_attacks_remaining => false,
        i_operation_timer => true,
        i_build_charges => false,
        i_park_charges => false,
        i_action_charges => false,
        i_air_slot_modifier => false,
        b_dominant_conquer => false,
        i_barbarian_tribe_index => false,
        e_barbarian_player_target => false,
        i_disaster_charges => false,
        i_era_strength_modifier => false,
        b_embarked => false,
        b_delayed_death => false,
        b_immobile => false,
        b_processed_turn => false,
        b_needs_movement_completion => false,
        b_eligible_to_heal => false,
        b_has_hidden_visibility => false,
        b_hidden_visibility_forfeited => false,
        b_ignore_terrain_cost => false,
        b_ignore_zoc => false,
        b_exert_zoc => false,
        b_can_see_through_features => false,
        b_can_see_through_terrain => false,
        b_attack_after_moving => false,
        i_move_after_attacking => false,
        b_needs_attention => false,
        b_ignore_resource_maintenance => false,
        k_health_modifiers => false,
        k_health_religion_modifiers => false,
        k_combat_modifers => false,
        k_movement_modifiers => false,
        k_sight_modifiers => false,
        k_attack_range_modifiers => false,
        k_sight_plots => false,
        ai_abilities_possessed => false,
        ab_invalid_terrain_types => false,
        ab_ignore_terrain_types => false,
        ab_ignore_feature_types => false,
        ai_bonus_points_per_kill => false,
        ai_bonus_points_per_kill_strength => false,
        ai_bypass_walls_promotion_class => false,
        ai_enable_wall_attack_promotion_class => false,
        k_experience => false,
        k_great_person => false,
        k_religion => false,
        k_rock_band => false,
        k_trade => false,
        k_archaeology => false,
        k_espionage => false,
        k_attributes => false,
        k_operations => true,
        k_operations_disabled => false,
        k_hero => false,
        k_properties => false,
    );
    let Formation {
        id: _,
        location: _,
        owner: _,
        moves_remaining_raw: _,
        domain: _,
        is_embarked: _,
        can_embark: _,
        can_attack: _,
        is_ranged_combat: _,
        can_capture: _,
        has_moved: _,
        has_moved_into_zoc: _,
        ignores_zoc: _,
        last_path: _,
        last_path_cache_location: _,
        unit_ids: _,
    } = before_formation;
    compare!(FormationRecord, before_formation, after_formation,
            id => false, location => false, owner => false, moves_remaining_raw => false,
            domain => false, is_embarked => false, can_embark => false, can_attack => false,
            is_ranged_combat => false, can_capture => false, has_moved => false,
            has_moved_into_zoc => false, ignores_zoc => false, last_path => true,
            last_path_cache_location => true, unit_ids => false);
    changed_fields
}

/// Full typed serialized actor records only. Native decoded fixtures can use
/// this projection without fabricating unsaved runtime or gameplay RNG state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorRecords {
    pub reference: UnitReference,
    pub unit: Unit,
    pub formation: Formation,
}
impl ActorRecords {
    pub fn new(
        reference: UnitReference,
        unit: Unit,
        formation: Formation,
    ) -> Result<Self, CaptureError> {
        Self::checked(reference, unit, formation)
    }

    /// Accepts only exact decoded IDs and explicit forward membership. Alias
    /// resolution requires `ActorSnapshot::capture` and the actual World lookup.
    pub fn checked(
        reference: UnitReference,
        unit: Unit,
        formation: Formation,
    ) -> Result<Self, CaptureError> {
        if reference.component_id().component_type != UNIT_COMPONENT_TYPE
            || unit.id != reference.id
            || unit.e_owner != reference.player()
            || unit.formation_id != formation.id
            || unit.e_owner != formation.owner
        {
            return Err(CaptureError::Identity);
        }
        if !formation.unit_ids.contains(&unit.id) {
            return Err(CaptureError::Membership);
        }
        Ok(Self {
            reference,
            unit,
            formation,
        })
    }
    pub fn capture(state: &LoadedState, reference: UnitReference) -> Result<Self, CaptureError> {
        let unit = state
            .world()
            .unit(reference)
            .map_err(CaptureError::World)?
            .ok_or(CaptureError::MissingActor)?;
        let formation = state
            .world()
            .player(reference.player())
            .map_err(CaptureError::World)?
            .formation(unit.record.formation_id)
            .map_err(CaptureError::World)?
            .ok_or(CaptureError::MissingFormation)?;
        Self::checked(reference, unit.record.clone(), formation.record.clone())
    }
}
/// Changes to serialized actor fields only; runtime and broader effects remain open.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordDelta {
    pub before: ActorRecords,
    pub after: ActorRecords,
    pub changed_fields: Vec<ChangedField>,
}
impl RecordDelta {
    pub fn between(before: ActorRecords, after: ActorRecords) -> Result<Self, DeltaError> {
        // Revalidate public record fields, including membership, on every comparison.
        for records in [&before, &after] {
            if records.reference.component_id().component_type != UNIT_COMPONENT_TYPE
                || records.unit.id != records.reference.id
                || records.unit.e_owner != records.reference.player()
            {
                return Err(DeltaError::ActorIdentity);
            }
            if records.unit.formation_id != records.formation.id
                || records.unit.e_owner != records.formation.owner
                || !records.formation.unit_ids.contains(&records.unit.id)
            {
                return Err(DeltaError::FormationIdentity);
            }
        }
        if before.reference != after.reference
            || before.unit.id != after.unit.id
            || before.unit.e_owner != after.unit.e_owner
        {
            return Err(DeltaError::ActorIdentity);
        }
        if before.unit.formation_id != after.unit.formation_id
            || before.formation.id != after.formation.id
            || before.formation.owner != after.formation.owner
        {
            return Err(DeltaError::FormationIdentity);
        }
        let changed_fields = record_changes(
            &before.unit,
            &after.unit,
            &before.formation,
            &after.formation,
        );
        Ok(Self {
            before,
            after,
            changed_fields,
        })
    }
    pub fn unexpected_fields(&self) -> impl Iterator<Item = &ChangedField> {
        self.changed_fields
            .iter()
            .filter(|field| !field.declared_write)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn records() -> ActorRecords {
        let mut unit = super::super::tests::synthetic_unit();
        unit.id = 17;
        unit.formation_id = 19;
        let formation = Formation {
            id: 19,
            owner: 0,
            location: unit.k_location,
            moves_remaining_raw: 0,
            domain: 0,
            is_embarked: false,
            can_embark: false,
            can_attack: false,
            is_ranged_combat: false,
            can_capture: false,
            has_moved: false,
            has_moved_into_zoc: false,
            ignores_zoc: false,
            last_path: vec![],
            last_path_cache_location: unit.k_location,
            unit_ids: vec![17],
        };
        ActorRecords::checked(
            UnitReference {
                owner: 1 << 16,
                id: 17,
            },
            unit,
            formation,
        )
        .unwrap()
    }
    #[test]
    fn checked_records_require_identity_and_explicit_membership() {
        let r = records();
        let mut missing = r.formation.clone();
        missing.unit_ids.clear();
        assert!(matches!(
            ActorRecords::checked(r.reference, r.unit.clone(), missing),
            Err(CaptureError::Membership)
        ));
        let mut other = r.unit.clone();
        other.e_owner = 1;
        assert!(matches!(
            ActorRecords::checked(r.reference, other, r.formation.clone()),
            Err(CaptureError::Identity)
        ));
        let mut after = r.clone();
        after.reference.id += 1;
        assert_eq!(
            RecordDelta::between(r, after).unwrap_err(),
            DeltaError::ActorIdentity
        );
    }
    #[test]
    fn rejects_wrong_component_kind_and_public_record_tampering() {
        let r = records();
        let wrong_kind = UnitReference {
            owner: (i32::from(FORMATION_COMPONENT_TYPE) << 16) | r.reference.player(),
            id: r.reference.id,
        };
        assert_eq!(wrong_kind.player(), r.reference.player());
        assert_eq!(wrong_kind.id, r.reference.id);
        assert!(matches!(
            ActorRecords::checked(wrong_kind, r.unit.clone(), r.formation.clone()),
            Err(CaptureError::Identity)
        ));
        let mut after = r.clone();
        after.reference = wrong_kind;
        assert_eq!(
            RecordDelta::between(r.clone(), after.clone()).unwrap_err(),
            DeltaError::ActorIdentity
        );
        // Both endpoints can be publicly forged identically; equality alone must
        // not bypass reference-kind validation.
        assert_eq!(
            RecordDelta::between(after.clone(), after).unwrap_err(),
            DeltaError::ActorIdentity
        );
        let mut after = r.clone();
        after.formation.unit_ids.clear();
        assert_eq!(
            RecordDelta::between(r.clone(), after).unwrap_err(),
            DeltaError::FormationIdentity
        );
        let mut after = r.clone();
        after.unit.formation_id += 1;
        assert_eq!(
            RecordDelta::between(r, after).unwrap_err(),
            DeltaError::FormationIdentity
        );
    }
    #[test]
    fn retains_every_field_and_exposes_unexpected_partial_effects() {
        let before = records();
        let mut after = before.clone();
        after.unit.e_activity = 23;
        after.unit.i_operation_timer = 57;
        after.unit.e_ai_type = 99;
        after.unit.i_damage = 43;
        after.formation.last_path_cache_location.x = 71;
        after.formation.can_capture = true;
        let delta = RecordDelta::between(before.clone(), after.clone()).unwrap();
        assert_eq!(delta.before, before);
        assert_eq!(delta.after, after);
        assert_eq!(
            delta
                .unexpected_fields()
                .map(|f| f.field)
                .collect::<Vec<_>>(),
            vec!["e_ai_type", "i_damage", "can_capture"]
        );
        assert!(delta
            .changed_fields
            .iter()
            .any(|f| f.field == "i_operation_timer" && f.declared_write));
        let same = RecordDelta::between(delta.after.clone(), delta.after).unwrap();
        assert!(same.changed_fields.is_empty());
    }
    #[test]
    fn archive_guard_preserves_call_order_and_rejects_unsupported_fields() {
        use c6_core::formation_initialization::{DirtyBaseline, UNIT_FIELDS};
        let before = LogicalArchive::new(
            c6_core::unit::ComponentId {
                owner: 0,
                component_type: 1,
                id: 17,
            },
            &UNIT_FIELDS,
            DirtyBaseline::Unavailable,
        );
        let mut after = before.clone();
        after.mark_dirty(11).unwrap();
        after.mark_dirty(11).unwrap();
        assert!(archive_extension(&before, &after, &[11]));
        assert!(!archive_extension(&after, &before, &[11]));
        after.mark_dirty(28).unwrap();
        assert!(!archive_extension(&before, &after, &[11]));
    }
    #[test]
    fn failed_attempt_keeps_original_failure_and_partial_record_effects() {
        use c6_core::{
            core_player::initialization::{DatabaseIdentity, InputIdentities, WireRole},
            formation_initialization::{DirtyBaseline, UNIT_FIELDS},
            unit_initialization::FormationReference,
        };
        let records = records();
        let binding = WireBinding {
            role: WireRole::ReadInputCore,
            core_sha256: [1; 32],
            occurrence_id: b"synthetic".to_vec(),
            output_id: None,
            inputs: InputIdentities {
                configuration_sha256: [2; 32],
                rules_inputs_sha256: [3; 32],
                database: DatabaseIdentity::Snapshot([4; 32]),
                dlc_inputs_sha256: [5; 32],
                mods_inputs_sha256: [6; 32],
                content_inputs_sha256: [7; 32],
            },
        };
        let runtime = UnitRuntime {
            archive: LogicalArchive::new(
                c6_core::unit::ComponentId {
                    owner: 0,
                    component_type: 1,
                    id: 17,
                },
                &UNIT_FIELDS,
                DirtyBaseline::Unavailable,
            ),
            definition_index: 0,
            domain: 0,
            formation: Some(FormationReference {
                owner: 0,
                requested_id: 19,
                resolved_id: 19,
            }),
            abilities: vec![3],
            invalid_terrain: vec![true],
            ignore_terrain: vec![false],
            ignore_feature: vec![true],
            bonus_points_per_kill: vec![4],
            bonus_points_per_kill_strength: vec![5],
            operations_disabled: vec![false],
        };
        let fr = FormationRuntime::from_owning_player_read(&records.formation, 0, 0).unwrap();
        let before = ActorSnapshot::checked(
            binding,
            records.reference,
            17,
            records.unit.clone(),
            runtime,
            records.formation.clone(),
            fr,
        )
        .unwrap();
        let wrong_kind = UnitReference {
            owner: i32::from(FORMATION_COMPONENT_TYPE) << 16,
            id: before.reference.id,
        };
        assert!(matches!(
            ActorSnapshot::checked(
                before.binding.clone(),
                wrong_kind,
                before.resolved_id,
                before.unit.clone(),
                before.unit_runtime.clone(),
                before.formation.clone(),
                before.formation_runtime.clone()
            ),
            Err(CaptureError::Identity)
        ));
        let mut after = before.clone();
        // A synchronous provider can fail after an unsupported write; observation
        // must preserve that write and the original error rather than rolling back.
        after.unit.i_damage = 91;
        after.unit_runtime.abilities.push(27);
        let mut after_records = records;
        after_records.unit = after.unit.clone();
        let attempt = ArenaAttempt {
            before: before.clone(),
            after: Ok(after.clone()),
            after_records: Ok(after_records),
            outcome: Err(Failure {
                error: super::super::Error::Unsupported("synthetic partial provider"),
                trace: vec![Event::Stage(super::super::Stage::Activation)],
            }),
            rng_evidence: None,
        };
        let delta = attempt.delta().unwrap();
        assert_eq!(delta.before, before);
        assert_eq!(delta.after, after);
        assert_eq!(
            delta
                .unexpected_fields()
                .map(|f| (f.kind, f.field))
                .collect::<Vec<_>>(),
            vec![
                (Kind::UnitRecord, "i_damage"),
                (Kind::UnitRuntime, "abilities")
            ]
        );
        let failure = attempt.outcome.unwrap_err();
        assert!(matches!(
            failure.error,
            super::super::Error::Unsupported("synthetic partial provider")
        ));
        assert_eq!(
            failure.trace,
            vec![Event::Stage(super::super::Stage::Activation)]
        );
    }
}
