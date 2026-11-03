//! Offline current City-prefix/Citizens wire candidate; never invokes game code.
use c6_core::city::City;
use c6_core::city_citizens::Citizens;
use c6_core::city_wire::{Header, HeaderDistricts};
use c6_core::unit::{variants, HashValue, Limits};
use serde_json::{json, Value};
use std::io::{Read, Write};
macro_rules! fields {
    ($value:expr; $($field:ident),+ $(,)?) => {{
        let mut object=serde_json::Map::new();
        $(object.insert(stringify!($field).into(),json!(&$value.$field));)+
        Value::Object(object)
    }};
}
fn pairs<T: serde::Serialize>(values: &[HashValue<T>]) -> Value {
    json!(values
        .iter()
        .map(|v| json!([v.definition_hash, &v.value]))
        .collect::<Vec<_>>())
}
fn header(h: &Header) -> Result<Value, Box<dyn std::error::Error>> {
    let mut value = fields!(h; id,location_x,location_y,owner,original_owner,owner_before_occupation,
        liberator,city_ai_id,population,capital,original_capital,last_transfer_type,just_conquered_from,
        just_rebelled_from,last_barbarian_invasion_turn,occupied,production_unit_changes,
        production_building_changes,production_district_changes,wonder_envoy,delayed_death,transferring,
        project_yield_project_hash,housing_per_district,max_unit_level,unit_experience_modifier_q8,
        attacks_per_turn,great_person_point_bonus_percent,settler_consume_pop,espionage_defense_bonus,
        project_production,prevent_bypass_outer_defenses,prevent_melee_attack_outer_defenses,name);
    value["last_warmonger_penalty"] = json!(h.last_warmonger_penalty.as_slice());
    macro_rules! indexed { ($($field:ident),+) => { $(value[stringify!($field)]=pairs(&h.$field);)+ }; }
    indexed!(
        yield_modifiers,
        follower_yield_modifiers,
        yield_modifiers_per_governor_title_q8,
        yield_changes,
        yield_per_district,
        extra_yield_per_population_q8,
        allow_incoming_regional_stacking_counts,
        yield_per_unit_complete,
        yield_per_building_complete,
        yield_per_wonder_complete,
        yield_per_terrain_class_cities,
        project_yield_q8,
        yield_modifier_from_faith_city,
        yield_per_flood
    );
    value["happiness_modifiers"]=json!(h.happiness_modifiers.iter().map(|v|fields!(v;
        happiness_type_index,great_person_class_hash,yield_type_index,yield_amount,great_person_amount)).collect::<Vec<_>>());
    value["terrain_modifiers"] = json!(h
        .terrain_modifiers
        .iter()
        .map(|v| fields!(v;
        terrain_hash,adjacent_improvement_hash,yield_type_index,yield_amount))
        .collect::<Vec<_>>());
    value["improvement_valid_terrain"] = json!(h
        .improvement_valid_terrain
        .iter()
        .map(|v| fields!(v;
        terrain_hash,improvement_hash))
        .collect::<Vec<_>>());
    value["terrain_class_yield_improvement_modifiers"] = json!(h
        .terrain_class_yield_improvement_modifiers
        .iter()
        .map(|v| fields!(v;
        improvement_hash,yield_type_index,amount))
        .collect::<Vec<_>>());
    value["dynamic_bonuses"]=json!(h.dynamic_bonuses.iter().map(|v|fields!(v;
        district_hash,feature_hash,terrain_hash,improvement_hash,river,adjacent_district,appeal,yield_hash,
        tiles_required,amount,description)).collect::<Vec<_>>());
    value["yield_per_terrain_type"] = json!(h
        .yield_per_terrain_type
        .iter()
        .map(|v| fields!(v;
        terrain_hash,yield_hash,amount_q8))
        .collect::<Vec<_>>());
    let (attributes, _) = h.attributes.encode(variants::Limits::default())?;
    value["attributes_wire"] = json!(attributes);
    Ok(value)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: c6-city-wire-candidate prefix|citizens INPUT OUTPUT".into());
    }
    let limits = Limits::default();
    let mut input = Vec::new();
    std::fs::File::open(&args[2])?
        .take(limits.max_bytes as u64 + 1)
        .read_to_end(&mut input)?;
    if input.len() > limits.max_bytes {
        return Err("record input limit".into());
    }
    let (output, consumed, observation) = match args[1].as_str() {
        "city" => {
            let (value, consumed, items) = City::decode_prefix_with_items(&input, limits)?;
            if consumed != input.len() {
                return Err("full City trailing bytes".into());
            }
            let (output, encoded_items) = value.encode_with_items(limits)?;
            if items != encoded_items {
                return Err("City aggregate item charge differs".into());
            }
            let mut carrier = input.clone();
            carrier.extend_from_slice(b"unparsed-tail");
            let (_, prefix_size, prefix_items) = City::decode_prefix_with_items(
                &carrier,
                Limits {
                    max_bytes: input.len(),
                    ..limits
                },
            )?;
            if prefix_size != input.len()
                || prefix_items != items
                || City::decode(&carrier, limits).is_ok()
            {
                return Err("City prefix/trailing contract differs".into());
            }
            if City::decode(
                &input,
                Limits {
                    max_bytes: input.len() - 1,
                    ..limits
                },
            )
            .is_ok()
                || value
                    .encode(Limits {
                        max_bytes: input.len() - 1,
                        ..limits
                    })
                    .is_ok()
            {
                return Err("City global byte budget not enforced".into());
            }
            if items > 0
                && (City::decode(
                    &input,
                    Limits {
                        max_items: items - 1,
                        ..limits
                    },
                )
                .is_ok()
                    || value
                        .encode(Limits {
                            max_items: items - 1,
                            ..limits
                        })
                        .is_ok())
            {
                return Err("City global item budget not enforced".into());
            }
            let mut components = serde_json::Map::new();
            macro_rules! component { ($($field:ident),+) => { $(components.insert(stringify!($field).into(),json!(value.$field.encode(limits)?));)+ }; }
            component!(
                header,
                districts,
                citizens,
                culture,
                gold,
                growth,
                religion,
                resources,
                build_queue,
                buildings,
                combat,
                trade,
                cultural_identity,
                power
            );
            let observation = json!({"id":value.header.id,"owner":value.header.owner,"items":items,"prefix_tail_and_aggregate_limits_checked":true,
                "components":components,"ai_controller":value.ai_controller,"turn_city_founded":value.turn_city_founded});
            (output, consumed, observation)
        }
        "prefix" => {
            let (value, consumed) = HeaderDistricts::decode_prefix(&input, limits)?;
            let output = value.encode(limits)?;
            let observation = json!({"header":header(&value.header)?,"districts":fields!(&value.districts;district_ids,extra_districts)});
            (output, consumed, observation)
        }
        "citizens" => {
            let value = Citizens::decode(&input, limits)?;
            let observation = json!({"working_plot":value.working_plot,"forced_working_plot":value.forced_working_plot,
                "favored_yields":pairs(&value.favored_yields),"disfavored_yields":pairs(&value.disfavored_yields),
                "num_unassigned_citizens":value.num_unassigned_citizens,"num_citizens_working_plots":value.num_citizens_working_plots,
                "force_update_plot_appeal":value.force_update_plot_appeal});
            (value.encode(limits)?, input.len(), observation)
        }
        _ => return Err("unsupported wire component".into()),
    };
    if output != input[..consumed] {
        return Err("wire roundtrip differs".into());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[3])?;
    file.write_all(&output)?;
    println!(
        "{}",
        json!({"consumed":consumed,"observation":observation,"wire_roundtrip_equal":true,
        "complete_city":false,"original_game_execution":false})
    );
    Ok(())
}
