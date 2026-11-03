//! Offline real decoded state exercise; this does not execute original code.
#[path = "loaded_actions_review_load.rs"]
mod fixture;
use c6_core::unit_initialization::InitializationInputs;
use c6_data::{
    loaded_actions::resources::Resources,
    unit_initialization_definitions,
    unit_movement_state::{self, Definitions, Inputs},
    Rules, RulesProfile,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};
fn digest(
    state: &c6_core::loaded_state::LoadedState,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut h = Sha256::new();
    for p in state.world().players() {
        for u in p.unit_records()? {
            h.update(u.value.encode(c6_core::unit::Limits::default())?);
        }
    }
    Ok(h.finalize().to_vec())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().collect();
    if a.len() != 7 {
        return Err("MANIFEST RUNTIME RULES RULES_REPORT PLACEMENT MANAGER".into());
    }
    let mut state = fixture::load_state(Path::new(&a[1]))?;
    let resources = Resources::from_closed_captures(Path::new(&a[5]), Path::new(&a[6]))?;
    let runtime = std::fs::read(&a[2])?;
    let bound = resources.bind(&state, &runtime)?;
    let profile = RulesProfile::LinuxXp2ActivatedDiagnostic;
    let rules = Rules::load_profile(profile, &a[3], &profile.source_identity()?)?;
    let report = std::fs::read(&a[4])?;
    let init = unit_initialization_definitions::Definitions::from_rules(&rules, &bound, &report)?;
    let defs = Definitions::from_rules(&state, &rules, &bound, &report)?;
    let identity = state.binding().inputs.clone();
    state.initialize_rosters(&InitializationInputs {
        identities: &identity,
        units: bound.unit_definitions()?,
        abilities: init.abilities(),
        terrains: init.terrains(),
        features: init.features(),
        great_person_classes: init.great_person_classes(),
        operations: bound.operation_lookup(),
    })?;
    let before = digest(&state)?;
    let inputs = Inputs {
        definitions: &defs,
        alliances: None,
    };
    let (mut count, mut moved, mut bounded) = (0, 0, 0);
    let (mut maxs, mut errors) = (
        BTreeMap::<i32, usize>::new(),
        BTreeMap::<String, usize>::new(),
    );
    let mut selected = None;
    for p in state.world().players() {
        for record in p.unit_records()? {
            let u = &record.value;
            count += 1;
            match unit_movement_state::unit_max_moves(&state, u, &inputs) {
                Ok(max) => {
                    *maxs.entry(max).or_default() += 1;
                    moved += usize::from(unit_movement_state::unit_has_moved(&state, u, &inputs)?);
                    let raw = max.wrapping_shl(8);
                    let mut changed = u.clone();
                    changed.x_moves_remaining = raw;
                    assert!(!unit_movement_state::unit_has_moved(
                        &state, &changed, &inputs
                    )?);
                    if raw > i32::MIN {
                        changed.x_moves_remaining = raw - 1;
                        assert!(unit_movement_state::unit_has_moved(
                            &state, &changed, &inputs
                        )?);
                        bounded += 1;
                    }
                    if raw < i32::MAX {
                        changed.x_moves_remaining = raw + 1;
                        assert!(!unit_movement_state::unit_has_moved(
                            &state, &changed, &inputs
                        )?);
                        bounded += 1;
                    }
                    if u.e_owner == 0 && u.id == 15794187 {
                        selected = Some(
                            serde_json::json!({"owner":u.e_owner,"id":u.id,"max_moves":max,"remaining_q8":u.x_moves_remaining,"has_moved":unit_movement_state::unit_has_moved(&state,u,&inputs)?}),
                        );
                    }
                }
                Err(error) => {
                    if u.e_owner == 0 && u.id == 15794187 {
                        selected = Some(
                            serde_json::json!({"owner":u.e_owner,"id":u.id,"error":format!("{error:?}")}),
                        );
                    }
                    *errors.entry(format!("{error:?}")).or_default() += 1;
                }
            }
        }
    }
    assert_eq!(count, 1262);
    assert!(!maxs.is_empty());
    assert_eq!(before, digest(&state)?);
    println!(
        "{}",
        serde_json::json!({"units":count,"successful_queries":maxs.values().sum::<usize>(),"max_moves_distribution":maxs,"has_moved_true":moved,"remaining_boundary_cases":bounded,"unsupported_queries":errors,"selected":selected,"unit_wire_unchanged":true,"native_unit_max_moves_comparison":false,"original_execution":false})
    );
    Ok(())
}
