//! Offline production-projection driver; never executes original game code.
use c6_core::city_district::DomainRebuildInputs;
use c6_core::map::{Limits, Map};
use c6_core::movement_world::{DefinitionTable, TerrainProperties};
use c6_data::city_movement::{DistrictPrefix, MapDomainInputs};
use c6_data::registry::LinuxDefinitionHashRegistry;
use serde::Deserialize;
use serde_json::json;
#[derive(Deserialize)]
struct Input {
    map_path: String,
    district_hashes: Vec<u32>,
    terrain_hashes: Vec<u32>,
    terrain_is_water: Vec<bool>,
    terrain_movement_cost: Vec<i32>,
    district_prefixes: Vec<Vec<u8>>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("input path required")?;
    let input: Input = serde_json::from_slice(&std::fs::read(path)?)?;
    let map = Map::decode(&std::fs::read(input.map_path)?, Limits::default())?;
    let definitions = LinuxDefinitionHashRegistry::from_native_order(input.district_hashes);
    let terrain_definitions = LinuxDefinitionHashRegistry::from_native_order(input.terrain_hashes);
    if input.terrain_is_water.len() != input.terrain_movement_cost.len() {
        return Err("inconsistent actual Terrain properties".into());
    }
    let terrain_properties: Vec<_> = input
        .terrain_is_water
        .into_iter()
        .zip(input.terrain_movement_cost)
        .map(|(is_water, movement_cost)| TerrainProperties {
            is_water,
            movement_cost,
        })
        .collect();
    let terrains = DefinitionTable::new(&terrain_definitions, &terrain_properties)?;
    let map_inputs = MapDomainInputs::new(&map, &terrains)?;
    let mut records = Vec::new();
    for bytes in input.district_prefixes {
        let prefix = DistrictPrefix::decode_prefix(&bytes)?;
        let record = prefix.movement_record(&map, &definitions)?;
        records.push(json!({"id":record.id,"owner":record.owner,
            "x":record.location.x,"y":record.location.y,"parent_city":record.parent_city,
            "type_index":record.type_index,"complete":record.complete}));
    }
    let mut plots = Vec::new();
    for (index, plot) in map.plots.iter().enumerate() {
        let city = plot.city_attributes.as_ref();
        let mut adjacent = Vec::new();
        for direction in 0..6 {
            adjacent.push(map_inputs.adjacent_plot(index, direction)?);
        }
        plots.push(json!({"x": index % map.header.width as usize,
            "y": index / map.header.width as usize,
            "owner": city.map_or(-1, |a| i32::from(a.player)),
            "district": city.map_or(u32::MAX, |a| a.district),
            "has_attributes": city.is_some(),
            "is_water": map_inputs.is_water(index)?,
            "coastal_min_one": map_inputs.is_coastal_land_one(index)?,
            "adjacent": adjacent}));
    }
    println!(
        "{}",
        serde_json::to_string(&json!({"records":records,"plots":plots}))?
    );
    Ok(())
}
