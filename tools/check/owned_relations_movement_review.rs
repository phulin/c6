//! Offline owned relations + full loaded Unit query inputs, never original code.
#[path = "../linux/loaded_actions_review_load.rs"]
mod fixture;
use c6_core::{unit::Limits, unit_initialization::InitializationInputs};
use c6_data::{
    game_relations, loaded_actions::resources::Resources, unit_initialization_definitions,
    unit_movement_state, Rules, RulesProfile,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};
fn digest(
    state: &c6_core::loaded_state::LoadedState,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut h = Sha256::new();
    for p in state.world().players() {
        for u in p.unit_records()? {
            h.update(u.value.encode(Limits::default())?);
        }
    }
    Ok(h.finalize().to_vec())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().collect();
    if a.len() != 8 {
        return Err("MANIFEST RUNTIME RULES RULES_REPORT PLACEMENT MANAGER OWNER_TRACE".into());
    }
    let started = std::time::Instant::now();
    let mut state = fixture::load_state(Path::new(&a[1]))?;
    eprintln!("phase decode_seconds={}", started.elapsed().as_secs_f64());
    let resources = Resources::from_closed_captures(Path::new(&a[5]), Path::new(&a[6]))?;
    let runtime = std::fs::read(&a[2])?;
    let bound = resources.bind(&state, &runtime)?;
    // This observed control export is admitted only to its exact associated
    // complete SavedOutput preimage, even if separately framed by native Read.
    let legacy = bound.admit_observed_snapshot(&mut state)?;
    let profile = RulesProfile::LinuxXp2ActivatedDiagnostic;
    let rules = Rules::load_profile(profile, &a[3], &profile.source_identity()?)?;
    let report = std::fs::read(&a[4])?;
    let mut bad_report = report.clone();
    bad_report.push(b' ');
    assert!(game_relations::Definitions::from_rules(&state, &rules, &bound, &bad_report).is_err());
    assert!(unit_movement_state::Definitions::from_rules(&state, &rules, &bound, &bad_report).is_err());
    assert!(state.game_relations().is_err());
    let init = unit_initialization_definitions::Definitions::from_rules(&rules, &bound, &report)?;
    let movement = unit_movement_state::Definitions::from_rules(&state, &rules, &bound, &report)?;
    let relations = game_relations::Definitions::from_rules(&state, &rules, &bound, &report)?;
    let trace = std::fs::read(&a[7])?;
    let mut bad = trace.clone();
    bad.push(b' ');
    assert!(relations
        .install_attributed_snapshot(&mut state, &bad, Limits::default())
        .is_err());
    assert!(state.game_relations().is_err());
    relations.install_attributed_snapshot(&mut state, &trace, Limits::default())?;
    let source = state.game_relations_source().unwrap().clone();
    let game = state.game_relations()?;
    assert_eq!(game.player_count(), 64);
    assert_eq!(game.war().unwrap().values(), legacy.relations().war_types);
    let native_war_values_compared = game.war().unwrap().values().len();
    let active_alliances = game
        .alliances()
        .unwrap()
        .values()
        .iter()
        .filter(|v| **v != -1)
        .count();
    assert_eq!(active_alliances, 26);
    let prefix = game.read_prefix().unwrap();
    assert_eq!(
        game.war().unwrap().changed_turns(),
        prefix.saved_war.changed_turns()
    );
    assert_eq!(
        game.alliances().unwrap().changed_turns(),
        prefix.saved_alliances.changed_turns()
    );
    let logical_relations = game.clone();
    let mut detached = game.clone();
    assert!(detached.initialize_definitions(&[], &[]).is_err());
    assert_eq!(state.game_relations()?, &logical_relations);
    assert!(relations
        .install_attributed_snapshot(&mut state, &trace, Limits::default())
        .is_err());
    assert_eq!(state.game_relations()?, &logical_relations);
    // Capture is validation/provenance only once owned state exists. It is not
    // kept as another independently mutable gameplay relationship authority.
    drop(legacy);
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
    eprintln!("phase admission_and_initialization_seconds={}", started.elapsed().as_secs_f64());
    let before = digest(&state)?;
    let query_started = std::time::Instant::now();
    let inputs = unit_movement_state::Inputs {
        definitions: &movement,
    };
    let (mut count, mut moved, mut boundaries) = (0, 0, 0);
    let (mut maxs, mut errors) = (
        BTreeMap::<i32, usize>::new(),
        BTreeMap::<String, usize>::new(),
    );
    let mut selected = None;
    let mut any = None;
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
                        boundaries += 1;
                    }
                    if raw < i32::MAX {
                        changed.x_moves_remaining = raw + 1;
                        assert!(!unit_movement_state::unit_has_moved(
                            &state, &changed, &inputs
                        )?);
                        boundaries += 1;
                    }
                    any = Some((u.e_owner, u.id, max));
                    if u.e_owner == 0 && u.id == 15794187 {
                        selected = Some(
                            serde_json::json!({"owner":u.e_owner,"id":u.id,"max_moves":max,"remaining_q8":u.x_moves_remaining,"has_moved":unit_movement_state::unit_has_moved(&state,u,&inputs)?}),
                        );
                    }
                }
                Err(error) => {
                    *errors.entry(format!("{error:?}")).or_default() += 1;
                }
            }
        }
    }
    eprintln!("phase queries_and_boundary_probes_seconds={}", query_started.elapsed().as_secs_f64());
    assert_eq!(count, 1262);
    assert!(errors.is_empty(), "{errors:?}");
    let (owner, id, max) = any.unwrap();
    let revision = state.revision();
    let unit = state.unit_mut(owner, id)?;
    // A real unrelated Unit write boundary increments the state revision.
    unit.i_operation_timer = unit.i_operation_timer.wrapping_add(1);
    unit.i_operation_timer = unit.i_operation_timer.wrapping_sub(1);
    assert!(state.revision() > revision);
    assert_eq!(state.game_relations()?, &logical_relations);
    let actor = state.world().player(owner)?.unit(id)?.unwrap().record;
    assert_eq!(
        unit_movement_state::unit_max_moves(&state, actor, &inputs)?,
        max
    );
    assert!(relations
        .install_attributed_snapshot(&mut state, &trace, Limits::default())
        .is_err());
    assert!(bound.admit_observed_snapshot(&mut state).is_err());
    assert_eq!(state.game_relations()?, &logical_relations);
    assert_eq!(before, digest(&state)?);
    eprintln!("phase total_seconds={}", started.elapsed().as_secs_f64());
    println!(
        "{}",
        serde_json::json!({"units":count,"successful_queries":maxs.values().sum::<usize>(),"max_moves_distribution":maxs,"has_moved_true":moved,"remaining_boundary_cases":boundaries,"unsupported_queries":errors,"selected":selected,"unit_wire_unchanged":true,"native_war_values_compared":native_war_values_compared,"active_initialized_alliances":active_alliances,"war_definition_count":relations.war_hashes().len(),"alliance_definition_count":relations.alliance_entries().len(),"relation_changed_turns_preserved":true,"relations_remain_authority_across_unit_writes":true,"source_owner_range":source.owner_range,"prefix_bytes":source.decoded_prefix_bytes,"owner_suffix_unclassified_bytes":source.owner_range[1]-source.owner_range[0]-source.decoded_prefix_bytes,"native_unit_max_moves_comparison":false,"native_alliance_query_comparison":false,"original_execution":false})
    );
    Ok(())
}
