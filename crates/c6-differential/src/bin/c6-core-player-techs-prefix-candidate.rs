//! Offline raw Core to first Player Techs validation; never executes the engine.
use c6_core::{core_stream::ThroughFirstPlayer, map, unit};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|v| format!("{v:02x}")).collect()
}
fn digest(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
fn container<T>(v: &map::Recycled<T>) -> Value {
    json!({"capacity":v.capacity,"current_id_bits":v.current_id_bits,
        "free_head":v.free_head,"free_tail":v.free_tail,"free_links":v.free_links,
        "slots":v.entries.iter().map(|e|e.index).collect::<Vec<_>>()})
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: c6-core-player-techs-prefix-candidate RAW_CORE POST_MAP_OUTPUT".into());
    }
    let mut bytes = Vec::new();
    const MAX: u64 = 256 * 1024 * 1024;
    std::fs::File::open(&args[1])?
        .take(MAX + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX {
        return Err("raw input resource bound".into());
    }
    let v = ThroughFirstPlayer::decode(&bytes, map::Limits::default(), unit::Limits::default())?;
    if !std::ptr::eq(v.tail.as_ptr(), bytes[v.consumed..].as_ptr())
        || v.tail != &bytes[v.consumed..]
    {
        return Err("tail borrow differs".into());
    }
    let (encoded, items) = v
        .player_prefix
        .encode_prefix_with_items(unit::Limits::default())?;
    if encoded != bytes[v.core.consumed..v.consumed] || items + v.core.items_used != v.items_used {
        return Err("post-Map typed wire or item charge differs".into());
    }
    let map_bytes = v.core.map.encode(map::Limits::default())?;
    if map_bytes != bytes[v.core.map_start..v.core.map_end] {
        return Err("typed Map wire differs".into());
    }
    std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&args[2])?
        .write_all(&encoded)?;
    // Native acceptance includes exact shared/Player byte+item ceilings, not
    // only default-limit roundtrips. No native process or state is mutated.
    let exact = map::Limits {
        max_bytes: v.consumed,
        max_items: v.items_used,
        ..map::Limits::default()
    };
    let player_exact = unit::Limits {
        max_bytes: encoded.len(),
        max_items: items,
    };
    let again = ThroughFirstPlayer::decode(&bytes, exact, player_exact)?;
    if again.consumed != v.consumed || again.items_used != v.items_used {
        return Err("exact shared/Player cap differs".into());
    }
    for cap in [
        map::Limits {
            max_bytes: v.consumed - 1,
            ..exact
        },
        map::Limits {
            max_items: v.items_used - 1,
            ..exact
        },
    ] {
        if ThroughFirstPlayer::decode(&bytes, cap, player_exact).is_ok() {
            return Err("one-short shared cap accepted".into());
        }
    }
    let mut short_player_caps = vec![unit::Limits {
        max_bytes: encoded.len() - 1,
        ..player_exact
    }];
    if items > 0 {
        short_player_caps.push(unit::Limits {
            max_items: items - 1,
            ..player_exact
        });
    }
    for cap in short_player_caps {
        if ThroughFirstPlayer::decode(&bytes, exact, cap).is_ok()
            || v.player_prefix.encode_prefix(cap).is_ok()
        {
            return Err("one-short Player cap accepted".into());
        }
    }
    if v.player_prefix.encode_prefix(player_exact)? != encoded {
        return Err("exact encode cap differs".into());
    }
    let r = &v.player_prefix.religion;
    let religion = json!({"minimum_faith_for_next_pantheon":r.minimum_faith_for_next_pantheon,
        "next_pantheon_founder":r.next_pantheon_founder,
        "records":r.records.iter().map(|r|json!({"religion_hash":r.religion_hash,"founder":r.founder,
            "founded_turn":r.founded_turn,"holy_city":[r.holy_city.x,r.holy_city.y],
            "custom_name":hex(&r.custom_name),"belief_hashes":r.belief_hashes,
            "building_hashes":r.building_hashes,"unit_hashes":r.unit_hashes,
            "anyone_condemns_favor":r.anyone_condemns_favor,"condemn_prohibited":r.condemn_prohibited,
            "auto_spreads_to_new_cities":r.auto_spreads_to_new_cities,"awards_envoy":r.awards_envoy,
            "awards_envoy_religious_pressure":r.awards_envoy_religious_pressure})).collect::<Vec<_>>()});
    let first = if let Some(p) = &v.player_prefix.first_player {
        let h = &p.header;
        let c = &p.cities;
        let formations: Result<Vec<_>, unit::Error> = p
            .formations
            .entries
            .iter()
            .map(|e| {
                let encoded = e.value.encode(unit::Limits::default())?;
                Ok(json!({"slot":e.index,"id":e.value.id,"owner":e.value.owner,
                "bytes":encoded.len(),"sha256":digest(&encoded)}))
            })
            .collect();
        let cities: Result<Vec<_>, c6_core::city::Error> = c
            .records
            .entries
            .iter()
            .map(|e| {
                let b = e.value.encode(unit::Limits::default())?;
                let h = &e.value.header;
                Ok(json!({"slot":e.index,"id":h.id,"owner":h.owner,
                "location":[h.location_x,h.location_y],"population":h.population,
                "bytes":b.len(),"sha256":digest(&b)}))
            })
            .collect();
        let espionage = p.espionage.encode(unit::Limits::default())?;
        let religion = p.religion.encode(unit::Limits::default())?;
        let resources = p.resources.encode(unit::Limits::default())?;
        let stats = p.stats.encode(unit::Limits::default())?;
        let techs = p.techs.encode(unit::Limits::default())?;
        let name_manager = p.name_manager.encode(unit::Limits::default())?;
        let culture = p.culture.encode(unit::Limits::default())?;
        let diplomacy = p.diplomacy.encode(unit::Limits::default())?;
        let districts = p.districts.encode(unit::Limits::default())?;
        let district_records: Result<Vec<_>, c6_core::player_districts::Error> = p
            .districts
            .records
            .entries
            .iter()
            .map(|e| {
                let b = e.value.encode(unit::Limits::default())?;
                let d = &e.value;
                Ok(
                    json!({"slot":e.index,"id":d.id,"location":[d.location.0,d.location.1],
                "city_id":d.city_id,"district_hash":d.district_hash,"is_complete":d.is_complete,
                "bytes":b.len(),"sha256":digest(&b)}),
                )
            })
            .collect();
        json!({"espionage":{"bytes":espionage.len(),"sha256":digest(&espionage)},
            "religion":{"bytes":religion.len(),"sha256":digest(&religion)},
            "resources":{"bytes":resources.len(),"sha256":digest(&resources)},
            "stats":{"bytes":stats.len(),"sha256":digest(&stats)},
            "techs":{"bytes":techs.len(),"sha256":digest(&techs),
                "has_tech":p.techs.has_tech.pairs.iter().map(|x|json!([x.definition_hash,x.value])).collect::<Vec<_>>()},
            "name_manager":{"bytes":name_manager.len(),"sha256":digest(&name_manager),
                "citizen_name_usage_counts":p.name_manager.citizen_name_usage_counts},
            "culture":{"bytes":culture.len(),"sha256":digest(&culture),
                "has_civic":p.culture.has_civic.pairs.iter().map(|x|json!([x.definition_hash,x.value])).collect::<Vec<_>>()},
            "diplomacy":{"bytes":diplomacy.len(),"sha256":digest(&diplomacy)},
            "districts":{"bytes":districts.len(),"sha256":digest(&districts),
                "container":container(&p.districts.records),"records":district_records?},
            "requested_owner":p.requested_owner,"header":{"owner":h.owner,
            "starting_plot":[h.starting_plot.x,h.starting_plot.y],"ordered_scalars":h.ordered_scalars,
            "ordered_flags":h.ordered_flags,"saved_is_alive":h.saved_is_alive(),"handicap_hash":h.handicap_hash,
            "district_hash":h.district_hash,"project_hash":h.project_hash,"final_scalar":h.final_scalar},
            "formation_container":container(&p.formations),"formations":formations?,
            "city_container":container(&c.records),"cities":cities?,
            "city_tail":{"fresh_water_housing_bonus":c.fresh_water_housing_bonus,
                "urban_defenses":c.urban_defenses,"home_continent_hash":c.home_continent_hash,
                "settled_continent_hashes":c.settled_continent_hashes,"peak_cities":c.peak_cities,
                "extra_starting_population_off_home_continent":c.extra_starting_population_off_home_continent,
                "population_after_conquest":c.population_after_conquest,
                "banned_building_hashes":c.banned_building_hashes,"banned_district_hashes":c.banned_district_hashes,
                "banned_district_building_hashes":c.banned_district_building_hashes,
                "banned_project_hashes":c.banned_project_hashes,"building_build_overrides":c.building_build_overrides}})
    } else {
        Value::Null
    };
    println!(
        "{}",
        json!({"map_start":v.core.map_start,"map_end":v.core.map_end,
        "map_sha256":digest(&map_bytes),"consumed":v.consumed,"items_used":v.items_used,
        "through_map_items":v.core.items_used,"through_map_logical_allocation":v.core.logical_allocation_used,
        "post_map_bytes":encoded.len(),"post_map_items":items,"post_map_sha256":digest(&encoded),
        "exact_and_one_short_caps":true,"tail_borrowed":true,"tail_bytes":v.tail.len(),"tail_sha256":digest(v.tail),
        "religion":religion,"player_count":v.player_prefix.player_count,"first_player":first,
        "boundary":format!("{:?}",v.player_prefix.boundary),
        "unfinished_player_entries":v.player_prefix.unfinished_player_entries,
        "complete_player_import":false,"complete_core_import":false,"gameplay_parity":false,
        "metadata_compatibility_checked":false,"rng_reconstructed":false,"original_game_execution":false})
    );
    Ok(())
}
