//! Offline production-projection driver; never executes original game code.
use c6_core::city_district::DomainRebuildInputs;
use c6_core::map::{Limits, Map};
use c6_core::movement_world::{DefinitionTable, TerrainProperties};
use c6_data::city_movement::{
    CityBuildings, DistrictPrefix, DomainRegistryExport, FreshSingleMapReadDefaults,
    MapDomainInputs, NativeDomainDefinitions, NativeTerrainDefinitions, PlayerMetadata,
    TerrainRegistryExport, TypedDomainInputs, WorldInstances,
};
use c6_data::registry::LinuxDefinitionHashRegistry;
use serde::Deserialize;
use serde_json::json;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    map_path: String,
    #[serde(default)]
    fresh_single_map_read_defaults: bool,
    district_hashes: Vec<u32>,
    terrain_hashes: Vec<u32>,
    terrain_is_water: Vec<bool>,
    terrain_movement_cost: Vec<i32>,
    district_prefixes: Vec<Vec<u8>>,
    #[serde(default)]
    domain_metadata: Option<DomainInputs>,
    #[serde(default)]
    require_domain_metadata: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DomainInputs {
    registries: Vec<DomainRegistryExport>,
    terrain: TerrainRegistryExport,
    players: Vec<PlayerMetadata>,
    city_records: Vec<Vec<u8>>,
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
    let mut prefixes = Vec::new();
    for bytes in input.district_prefixes {
        let prefix = DistrictPrefix::decode_prefix(&bytes)?;
        let record = prefix.movement_record(&map, &definitions)?;
        prefixes.push(prefix);
        records.push(json!({"id":record.id,"owner":record.owner,
            "x":record.location.x,"y":record.location.y,"parent_city":record.parent_city,
            "type_index":record.type_index,"complete":record.complete}));
    }
    let domain_reconstruction = if let Some(metadata) = input.domain_metadata {
        let domain_definitions = NativeDomainDefinitions::from_exports(&metadata.registries)?;
        let actual_terrain = NativeTerrainDefinitions::from_export(&metadata.terrain)?;
        let actual_terrains = actual_terrain.as_core()?;
        if (0..terrain_definitions.hashes().len()).any(|index| {
            actual_terrains.lookup.type_hash(index) != Some(terrain_definitions.hashes()[index])
                || actual_terrains.at(index as i32) != terrains.at(index as i32)
        }) || actual_terrains.lookup.count() != terrain_definitions.hashes().len()
        {
            return Err("Terrain definition packets disagree".into());
        }
        // The repeated District hashes are actual independent native packets;
        // require exact identity instead of silently choosing one.
        if (0..definitions.hashes().len()).any(|index| {
            domain_definitions.district_lookup().type_hash(index)
                != Some(definitions.hashes()[index])
        }) || domain_definitions.district_lookup().count() != definitions.hashes().len()
        {
            return Err("District definition packets disagree".into());
        }
        let mut cities = Vec::new();
        for bytes in metadata.city_records {
            cities.push(c6_core::city::City::decode(
                &bytes,
                c6_core::unit::Limits::default(),
            )?);
        }
        let instances = WorldInstances::from_metadata(
            &map,
            &cities,
            &prefixes,
            &metadata.players,
            &domain_definitions,
            1_000_000,
        )?;
        let registry = instances.registry()?;
        let city_buildings = CityBuildings::new(&cities, 1_000_000)?;
        let inputs = TypedDomainInputs {
            map: MapDomainInputs::new(&map, &actual_terrains)?,
            registry: &registry,
            cities: &city_buildings,
            building_definitions: domain_definitions.building_lookup(),
        };
        let masks = c6_core::city_district::rebuild_valid_domains(
            &map,
            &registry,
            &domain_definitions.as_core()?,
            &inputs,
            1_000_000,
        )?;
        let mut wonder_complete = Vec::new();
        for index in 0..map.plots.len() {
            wonder_complete.push(if map.plots[index].city_attributes.is_some() {
                Some(inputs.is_wonder_complete(index)?)
            } else {
                None
            });
        }
        let effective = if input.fresh_single_map_read_defaults {
            let defaults =
                FreshSingleMapReadDefaults::initialize_for_fresh_single_canonical_read(&map)?;
            let mut effective_masks = Vec::new();
            let mut effective_wonders = Vec::new();
            for index in 0..map.plots.len() {
                effective_masks.push(defaults.effective_mask(index, masks[index])?.0);
                effective_wonders
                    .push(defaults.effective_wonder_complete(index, wonder_complete[index])?);
            }
            Some(json!({"phase":"fresh_single_canonical_map_read",
                "highest_allocated_index":defaults.highest_allocated_index(),
                "masks":effective_masks,"wonder_complete":effective_wonders}))
        } else {
            None
        };
        Some(
            json!({"effective_initialized":effective,"masks":masks.iter().map(|mask|mask.map(|mask|mask.0)).collect::<Vec<_>>(),
            "wonder_complete":wonder_complete}),
        )
    } else {
        if input.require_domain_metadata || input.fresh_single_map_read_defaults {
            return Err("complete genuine domain metadata unavailable".into());
        }
        None
    };
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
        serde_json::to_string(
            &json!({"records":records,"plots":plots,"domain_reconstruction":domain_reconstruction})
        )?
    );
    Ok(())
}
