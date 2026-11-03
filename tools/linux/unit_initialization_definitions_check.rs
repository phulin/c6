//! Offline strict-capture Unit initialization; never executes original game code.
#[path = "loaded_actions_review_load.rs"]
mod fixture;
use c6_core::{
    formation_initialization::{OwnerBinding, FORMATION_COMPONENT_TYPE, FORMATION_FIELDS},
    movement_world::DefinitionLookup,
    unit::{ComponentId, HashValue},
    unit_initialization::{domain_from_hash, FormationReference, InitializationInputs},
};
use c6_data::{
    loaded_actions::resources::Resources, unit_initialization_definitions::Definitions, Rules,
    RulesProfile,
};
use sha2::{Digest, Sha256};
use std::path::Path;

fn order<T>(pairs: &[HashValue<T>], lookup: &dyn DefinitionLookup) {
    assert_eq!(pairs.len(), lookup.count());
    for (i, pair) in pairs.iter().enumerate() {
        assert_eq!(lookup.type_hash(i), Some(pair.definition_hash));
    }
}
fn wire_digest(
    state: &c6_core::loaded_state::LoadedState,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut h = Sha256::new();
    for player in state.world().players() {
        for unit in player.unit_records()? {
            h.update(unit.value.encode(c6_core::unit::Limits::default())?);
        }
        for formation in player.formation_records()? {
            h.update(formation.value.encode(c6_core::unit::Limits::default())?);
        }
    }
    Ok(h.finalize().to_vec())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 7 {
        return Err(
            "usage: check MANIFEST TARGET_RUNTIME RULES_DIR RULES_REPORT PLACEMENT_DIR MANAGER_DIR"
                .into(),
        );
    }
    let mut state = fixture::load_state(Path::new(&args[1]))?;
    let resources = Resources::from_closed_captures(Path::new(&args[5]), Path::new(&args[6]))?;
    let runtime = std::fs::read(&args[2])?;
    let bound = resources.bind(&state, &runtime)?;
    let profile = RulesProfile::LinuxXp2ActivatedDiagnostic;
    let rules = Rules::load_profile(profile, &args[3], &profile.source_identity()?)?;
    let report = std::fs::read(&args[4])?;
    let definitions = Definitions::from_rules(&rules, &bound, &report)?;
    let mut changed = report.clone();
    changed.push(b' ');
    assert!(Definitions::from_rules(&rules, &bound, &changed).is_err());
    let counts = [
        definitions.abilities().count(),
        definitions.terrains().count(),
        definitions.features().count(),
        definitions.great_person_classes().count(),
    ];
    let mut checked = 0;
    for player in state.world().players() {
        for record in player.unit_records()? {
            let u = &record.value;
            order(&u.ai_abilities_possessed, definitions.abilities());
            order(&u.ab_invalid_terrain_types, definitions.terrains());
            order(&u.ab_ignore_terrain_types, definitions.terrains());
            order(&u.ab_ignore_feature_types, definitions.features());
            order(
                &u.ai_bonus_points_per_kill,
                definitions.great_person_classes(),
            );
            order(
                &u.ai_bonus_points_per_kill_strength,
                definitions.great_person_classes(),
            );
            order(&u.k_operations_disabled, bound.operation_lookup());
            checked += 1;
        }
    }
    assert_eq!(checked, 1262);
    let before = wire_digest(&state)?;
    let identities = state.binding().inputs.clone();
    let inputs = InitializationInputs {
        identities: &identities,
        units: bound.unit_definitions()?,
        abilities: definitions.abilities(),
        terrains: definitions.terrains(),
        features: definitions.features(),
        great_person_classes: definitions.great_person_classes(),
        operations: bound.operation_lookup(),
    };
    state.initialize_rosters(&inputs)?;
    state.initialize_formations()?;
    assert_eq!(before, wire_digest(&state)?);
    let mut formations_checked = 0;
    for player in state.world().players() {
        for record in player.unit_records()? {
            let u = &record.value;
            let actual = state.unit_runtime(player.owner(), u.id)?;
            assert_eq!(
                actual.archive.identity(),
                ComponentId {
                    owner: u.e_owner as i16,
                    component_type: 1,
                    id: u.id
                }
            );
            let index = inputs
                .units
                .lookup
                .find_index(u.e_type_index)
                .ok_or("missing Unit definition")?;
            assert_eq!(actual.definition_index, index as i32);
            assert_eq!(
                actual.domain,
                domain_from_hash(
                    inputs
                        .units
                        .at(index as i32)
                        .ok_or("missing Unit properties")?
                        .domain_hash
                )
            );
            let formation = player
                .formation(u.formation_id)?
                .map(|found| FormationReference {
                    owner: player.owner(),
                    requested_id: found.requested_id,
                    resolved_id: found.resolved_id,
                });
            assert_eq!(actual.formation, formation);
            assert_eq!(
                actual.abilities,
                u.ai_abilities_possessed
                    .iter()
                    .map(|p| p.value)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                actual.invalid_terrain,
                u.ab_invalid_terrain_types
                    .iter()
                    .map(|p| p.value)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                actual.ignore_terrain,
                u.ab_ignore_terrain_types
                    .iter()
                    .map(|p| p.value)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                actual.ignore_feature,
                u.ab_ignore_feature_types
                    .iter()
                    .map(|p| p.value)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                actual.bonus_points_per_kill,
                u.ai_bonus_points_per_kill
                    .iter()
                    .map(|p| p.value)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                actual.bonus_points_per_kill_strength,
                u.ai_bonus_points_per_kill_strength
                    .iter()
                    .map(|p| p.value)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                actual.operations_disabled,
                u.k_operations_disabled
                    .iter()
                    .map(|p| p.value)
                    .collect::<Vec<_>>()
            );
        }
        for record in player.formation_records()? {
            let f = &record.value;
            let actual = state.formation_runtime(player.owner(), f.id)?;
            assert_eq!(
                actual.archive.identity(),
                ComponentId {
                    owner: f.owner as i16,
                    component_type: FORMATION_COMPONENT_TYPE,
                    id: f.id
                }
            );
            assert_eq!(actual.archive.registered_fields(), &FORMATION_FIELDS);
            let expected_owner = if f.owner == -1 {
                OwnerBinding::NoOwner
            } else {
                OwnerBinding::Lookup {
                    requested: f.owner,
                    resolved: Some(player.owner()),
                }
            };
            assert_eq!(actual.owner, expected_owner);
            assert!(!actual.evaluating_operation());
            assert_eq!(actual.archive.dirty_calls(), [4, 13]);
            assert_eq!(actual.archive.dirty_indices(), Ok(vec![4, 13]));
            formations_checked += 1;
        }
    }
    assert_eq!(formations_checked, 1250);
    assert!(matches!(
        state.initialize_rosters(&inputs),
        Err(c6_core::loaded_state::Error::AlreadyInitialized)
    ));
    assert!(matches!(
        state.initialize_formations(),
        Err(c6_core::loaded_state::Error::AlreadyInitialized)
    ));
    println!(
        "{}",
        serde_json::json!({"units":checked,"formations":formations_checked,"runtime_identities_checked":true,"formation_read_postlude_checked":true,"array_collection_counts":counts,"native_save_array_orders_compared":true,"wire_unit_diagnostic_unchanged":true,"wire_formation_diagnostic_unchanged":true,"complete_unit_initialization":false,"native_initialized_state_comparison":false,"original_execution":false})
    );
    Ok(())
}
