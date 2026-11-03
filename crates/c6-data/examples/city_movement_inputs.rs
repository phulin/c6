//! Offline production-projection driver; never executes original game code.
use c6_core::map::{Limits, Map};
use c6_data::city_movement::DistrictPrefix;
use c6_data::registry::LinuxDefinitionHashRegistry;
use serde::Deserialize;
use serde_json::json;
#[derive(Deserialize)]
struct Input {
    map_path: String,
    district_hashes: Vec<u32>,
    district_prefixes: Vec<Vec<u8>>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("input path required")?;
    let input: Input = serde_json::from_slice(&std::fs::read(path)?)?;
    let map = Map::decode(&std::fs::read(input.map_path)?, Limits::default())?;
    let definitions = LinuxDefinitionHashRegistry::from_native_order(input.district_hashes);
    let mut records = Vec::new();
    for bytes in input.district_prefixes {
        let prefix = DistrictPrefix::decode_prefix(&bytes)?;
        let record = prefix.movement_record(&map, &definitions)?;
        records.push(json!({"id":record.id,"owner":record.owner,
            "x":record.location.x,"y":record.location.y,"parent_city":record.parent_city,
            "type_index":record.type_index,"complete":record.complete}));
    }
    println!("{}", serde_json::to_string(&records)?);
    Ok(())
}
