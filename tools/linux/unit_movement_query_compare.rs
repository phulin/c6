//! Independent candidate projection. No native answer file is opened here.
#[path = "loaded_actions_review_load.rs"]
mod fixture;
use c6_core::{unit::Limits, unit_initialization::InitializationInputs};
use c6_data::{
    game_relations, loaded_actions::resources::Resources, registry::linux_type_hash,
    unit_initialization_definitions, unit_movement_state, Rules, RulesProfile,
};
use serde_json::{json, Value};
use std::path::Path;
fn parameter(rules: &Rules, name: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let table = rules
        .gameplay()
        .table("GlobalParameters")
        .ok_or("parameters")?;
    let row = table
        .rows()
        .find(|r| r.text("Name").ok() == Some(name))
        .ok_or("parameter")?;
    let value = match row.get("Value")? {
        c6_data::SqlValue::Integer(v) => i32::try_from(*v)?,
        c6_data::SqlValue::Text(v) => std::str::from_utf8(v)?.parse()?,
        _ => return Err("parameter type".into()),
    };
    Ok(value)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().collect();
    if a.len() != 8 {
        return Err("MANIFEST RUNTIME RULES RULES_REPORT PLACEMENT MANAGER OWNER_TRACE".into());
    }
    let mut state = fixture::load_state(Path::new(&a[1]))?;
    let resources = Resources::from_closed_captures(Path::new(&a[5]), Path::new(&a[6]))?;
    let runtime = std::fs::read(&a[2])?;
    let bound = resources.bind(&state, &runtime)?;
    // Historical controls are a reference projection only. Comparison MUST match
    // the new native input projection; this does not relabel old live facts.
    let legacy = bound.admit_observed_snapshot(&mut state)?;
    let profile = RulesProfile::LinuxXp2ActivatedDiagnostic;
    let rules = Rules::load_profile(profile, &a[3], &profile.source_identity()?)?;
    let report = std::fs::read(&a[4])?;
    let init = unit_initialization_definitions::Definitions::from_rules(&rules, &bound, &report)?;
    let movement = unit_movement_state::Definitions::from_rules(&state, &rules, &bound, &report)?;
    let relations = game_relations::Definitions::from_rules(&state, &rules, &bound, &report)?;
    relations.install_attributed_snapshot(&mut state, &std::fs::read(&a[7])?, Limits::default())?;
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
    let game = state.game_relations()?;
    let alliances = game.alliances().ok_or("initialized alliances")?;
    let pairs: Vec<_> = alliances
        .values()
        .iter()
        .zip(alliances.changed_turns().ok_or("alliance changed turns")?)
        .map(|(v, t)| [*v, *t])
        .collect();
    println!(
        "{}",
        json!({"kind":"global", "turn":state.retained().core.current_turn,
        "embarked_base":parameter(&rules,"MOVEMENT_WHILE_EMBARKED_BASE")?,
        "city_cost":parameter(&rules,"CITY_MOVEMENT_COST")?,
        "alliance_player_count":game.player_count(),"alliances":pairs})
    );
    let world = state.world();
    let mapping = world.player_mapping.ok_or("mapping")?;
    let slots = world.player_slot_present.ok_or("slots")?;
    for query in 0..65i32 {
        let valid = world.player_is_valid(query)?;
        let p = if valid && slots[query as usize] {
            Some(state.player_controls(query)?)
        } else {
            None
        };
        let mut row = json!({"kind":"player","query":query,"mapped_index":mapping[query as usize],
            "valid":valid,"stored_nonnull":slots[query as usize],"resolved":p.is_some()});
        if let Some(p) = p {
            let retained = state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == query)
                .ok_or("retained player")?;
            let modifiers: Vec<_> = retained
                .diplomacy
                .movement_modifiers
                .iter()
                .map(|m| [m.source, m.turns_active, m.extra_moves])
                .collect();
            row.as_object_mut().unwrap().extend(json!({"owner":query,"slot_status":p.slot_status,"team":p.team,
                "alive":p.is_alive,"turn_active":p.turn_active,"embarked_modifier":retained.units.embarked_movement_modifier,
                "suzerain":retained.influence.suzerain,"timing":retained.diplomacy.last_war_and_city_turns,
                "movement_modifiers":modifiers}).as_object().unwrap().clone());
        }
        println!("{row}");
    }
    let inputs = unit_movement_state::Inputs {
        definitions: &movement,
    };
    let mut count = 0;
    for p in world.players() {
        for (ordinal, record) in p.unit_records()?.iter().enumerate() {
            let u = &record.value;
            let runtime = state.unit_runtime(u.e_owner, u.id)?;
            let definition = rules
                .units()
                .iter()
                .find(|d| linux_type_hash(d.unit_type.as_bytes()) == u.e_type_index)
                .ok_or("unit type hash")?;
            let map = world.map();
            let plot = if u.k_location.x == -9999 || u.k_location.y == -9999 {
                None
            } else {
                let x = c6_core::movement_world::coord_range(
                    u.k_location.x,
                    map.header.width,
                    map.header.wrap_x,
                );
                let y = c6_core::movement_world::coord_range(
                    u.k_location.y,
                    map.header.height,
                    map.header.wrap_y,
                );
                map.plot_at(x, y)
            };
            let plot: Value = match plot {
                Some(p) => {
                    json!({"owner":p.city_attributes.as_ref().map_or(-1,|c|i32::from(c.player)),
                "movement_cost":unit_movement_state::plot_movement_cost(&state,p,&movement)?})
                }
                None => Value::Null,
            };
            let m = &u.k_movement_modifiers;
            println!(
                "{}",
                json!({"kind":"unit","owner":u.e_owner,"ordinal":ordinal,"id":u.id,
                "remaining_q8":u.x_moves_remaining,"x":u.k_location.x,"y":u.k_location.y,
                "embarked":u.b_embarked,"activity":u.e_activity,"effective_embarked":unit_movement_state::unit_is_embarked(u),"delayed_death":u.b_delayed_death,"domain":runtime.domain,
                "runtime_type":runtime.definition_index,"fight_embarked":u.k_combat_modifers.b_fight_while_embarked,
                "modifiers":[m.i_extra_moves,m.i_clear_terrain_start_movement,m.i_friendly_territory_start_movement,m.i_enemy_territory_start_movement],
                "definition":{"index":runtime.definition_index,"hash":u.e_type_index,"base_moves":definition.base_moves},"plot":plot})
            );
            println!(
                "{}",
                json!({"kind":"answer","owner":u.e_owner,"ordinal":ordinal,"id":u.id,
                "max_moves":unit_movement_state::unit_max_moves(&state,u,&inputs)?,
                "has_moved":unit_movement_state::unit_has_moved(&state,u,&inputs)?})
            );
            count += 1;
        }
    }
    if count != 1262 {
        return Err(format!("expected1262 Units, got{count}").into());
    }
    eprintln!("candidate_units={count} original_execution=false");
    Ok(())
}
