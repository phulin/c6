//! Offline Map codec candidate; JSON diagnostics are outside measured operations.
use c6_core::map::*;
use c6_core::unit::variants::{
    ArrayVariant, KeyedHeader, TypedVariant, TypedVariantMap, VariantPayload,
};
use serde_json::{json, Value};
use std::{env, fs, io::BufWriter, time::Instant};
fn hex(b: &[u8]) -> String {
    b.iter().map(|v| format!("{v:02x}")).collect()
}
fn array(v: &ArrayVariant) -> Value {
    let payload = match &v.payload {
        VariantPayload::Inline32(x) => json!(hex(&x.to_le_bytes())),
        VariantPayload::Inline64(x) => json!(hex(&x.to_le_bytes())),
        VariantPayload::Char8(b) => json!(hex(b)),
        VariantPayload::TypedVariantMap(m) => typed_map(m),
        VariantPayload::TypedVariantVectorMap(m) => keyed(m.header, &m.entries),
        VariantPayload::TypedVariantVector(v) => json!(v.iter().map(typed).collect::<Vec<_>>()),
        VariantPayload::EmptyVariantVector => json!([]),
        VariantPayload::VariantMap(m) => {
            json!({"scalars":m.scalars,"arrays":m.arrays.iter().map(|(k,v)|json!([k,array(v)])).collect::<Vec<_>>()})
        }
    };
    json!({"flags":v.count_flags,"element_size":v.element_size,"payload":payload})
}
fn typed(v: &TypedVariant) -> Value {
    json!({"kind":v.type_bits,"value":array(&v.value)})
}
fn keyed(h: KeyedHeader, e: &[c6_core::unit::variants::TypedEntry]) -> Value {
    let header = match h {
        KeyedHeader::Count => e.len() as u32,
        KeyedHeader::Prefix(h) => h,
    };
    json!({"header":header,"entries":e.iter().map(|e|json!({"key":e.key,"name":e.name.as_ref().map(|v|hex(v)),"value":typed(&e.value)})).collect::<Vec<_>>()})
}
fn typed_map(m: &TypedVariantMap) -> Value {
    keyed(m.header, &m.entries)
}
fn hashes(v: &[DefinitionValue]) -> Value {
    json!(v.iter().map(|v| (v.hash, v.value)).collect::<Vec<_>>())
}
fn recycled<T>(v: &Recycled<T>, f: impl Fn(&T) -> Value) -> Value {
    json!({"bits":16,"capacity":v.capacity,"entries":v.entries.iter().map(|s|json!([s.index,f(&s.value)])).collect::<Vec<_>>(),"current_id_bits":v.current_id_bits,"free_head":v.free_head,"free_tail":v.free_tail,"free":v.free_links})
}
fn area(v: &Area) -> Value {
    json!({"scalars":[v.id,v.plot_count,v.coastal_plot_count,v.coastal_hill_mountain_count,v.owned_plot_count,v.river_edge_count,v.unit_count,v.city_count,v.total_population,v.starting_plot_count,v.natural_wonder_count],"players":[&v.units_per_player,&v.cities_per_player,&v.population_per_player,&v.free_specialist,&v.revealed_plot_count],"yields":v.yields_per_player.iter().map(|v|hashes(v)).collect::<Vec<_>>(),"resources":hashes(&v.resource_counts),"improvements":hashes(&v.improvement_counts),"bounds":[v.east,v.north,v.south,v.west],"flags":[u8::from(v.water),u8::from(v.mountains),u8::from(v.canyons)]})
}
fn visibility(v: &Visibility) -> Value {
    json!({"player":v.player_id,"cached":v.cached_num_revealed_hexes,"stale":u8::from(v.cached_old),"revealed":v.revealed,"layers":v.layers.iter().map(|l|json!({"id":l.type_hash,"index":l.index,"count":l.count,"width":l.grid_width,"values":l.values})).collect::<Vec<_>>(),"shared":[v.outgoing_visibility.iter().map(|v|(v.player_id,v.count)).collect::<Vec<_>>(),v.outgoing_visibility_blockers.iter().map(|v|(v.player_id,v.count)).collect::<Vec<_>>() ]})
}
fn plot(v: &Plot) -> Value {
    json!({"area_id":v.area_id,"landmass_id":v.landmass_id,"region_id":v.region_id,"terrain_hash":v.terrain_hash,"feature_hash":v.feature_hash,"feature_layout":v.feature_layout,"continent_hash":v.continent_hash,"unit_count":v.unit_count,"resource_hash":v.resource_hash,"resource_count":v.resource_count,"improvement_hash":v.improvement_hash,"improvement_owner":v.improvement_owner,"route_type":v.route_type,"route_era":v.route_era,"appeal":v.appeal,"river_e_flow":v.river_east_flow,"river_se_flow":v.river_south_east_flow,"river_sw_flow":v.river_south_west_flow,"river_crossing_count":v.river_crossing_count,"river_crossing_mask":v.river_crossing_mask,"cliff_crossing_mask":v.cliff_crossing_mask,"flags":v.flags.bits(),"properties":v.properties.as_ref().map(typed_map),"city":v.city_attributes.as_ref().map(|c|json!({"working":c.working_city,"owning":c.owning_city,"district":c.district,"owner":c.player,"wonder":c.wonder_building_hash}))})
}
fn sparse<T>(s: &SparsePlotData<T>, f: impl Fn(&T) -> Value) -> Value {
    json!({"default":f(&s.default),"indices":s.indices,"values":s.values.iter().map(f).collect::<Vec<_>>()})
}
fn eruption(v: &Eruption) -> Value {
    json!({"turn":v.turn,"duration":v.duration,"severity":v.severity})
}
fn localized(v: &LocalizedText) -> Value {
    json!({"base":hex(&v.base),"args":v.arguments.iter().map(|v|match v {LocalizedArgument::None=>json!({"kind":0,"value":null}),LocalizedArgument::Int32(v)=>json!({"kind":1,"value":v}),LocalizedArgument::Long64(v)=>json!({"kind":2,"value":v}),LocalizedArgument::Float32Bits(v)=>json!({"kind":3,"value":v}),LocalizedArgument::Float64Bits(v)=>json!({"kind":4,"value":v}),LocalizedArgument::Text(v)=>json!({"kind":5,"value":localized(v)})}).collect::<Vec<_>>(),"names":v.named_arguments.iter().map(|v|json!({"index":v.argument_index,"name":hex(&v.name)})).collect::<Vec<_>>()})
}
fn territory(v: &Territory) -> Value {
    json!({"id":v.id,"terrain_class_hash":v.terrain_class_hash,"counts":[v.plot_count,v.coastal_plot_count,v.coastal_hill_mountain_count,v.river_edge_count,v.natural_wonder_count,v.feature_count],"is_lake":u8::from(v.is_lake),"is_sea":u8::from(v.is_sea),"bounds":[v.east,v.north,v.south,v.west],"centroid_delta":v.centroid_delta,"feature_counts":hashes(&v.feature_counts),"name":localized(&v.name)})
}
fn component(c: &Component) -> Value {
    let value = match c {
        Component::Route(v) => {
            json!({"width":v.width,"height":v.height,"portals":v.portals.iter().map(|v|json!({"entrance_plot":v.entrance_plot_index,"exit_plots":v.exit_plot_indices,"source_plots":v.source_plot_indices})).collect::<Vec<_>>()})
        }
        Component::Feature(v) => {
            json!({"extra_yields":sparse(&v.extra_yields,|v|hashes(v)),"volcanoes":v.volcanoes.iter().map(|v|json!({"name_hash":v.name_hash,"plot":v.plot,"discovering_civ_hash":v.discovering_civilization_hash,"turn_active":v.turn_active,"custom_name":hex(&v.custom_name),"eruptions":v.eruptions.iter().map(eruption).collect::<Vec<_>>(),"affected_plots":v.affected_plots,"periods_raw":v.periods_raw,"chances_raw":v.chances_raw})).collect::<Vec<_>>(),"natural_wonders":v.natural_wonder_volcanoes.iter().map(|v|json!({"feature_hash":v.feature_hash,"plots":v.plots,"eruptions":v.eruptions.iter().map(eruption).collect::<Vec<_>>()})).collect::<Vec<_>>(),"plot_volcanoes":sparse(&v.plot_volcanoes,|v|json!(v)),"ice":v.ice.iter().map(|v|json!({"plot":v.plot,"random_event_hash":v.random_event_hash})).collect::<Vec<_>>()})
        }
        Component::River(v) => {
            json!({"next_id":v.next_id,"rivers":v.rivers.iter().map(|v|json!({"name_hash":v.name_hash,"id":v.id,"plots":v.plots,"floodplains":v.floodplains,"discovering_civ_hash":v.discovering_civilization_hash,"custom_name":hex(&v.custom_name),"floods":v.floods.iter().map(|f|json!({"turn":f.turn,"duration":f.duration,"severity":f.severity,"mitigated":u8::from(f.mitigated)})).collect::<Vec<_>>(),"edges":v.edges,"periods_raw":v.periods_raw,"chances_raw":v.chances_raw})).collect::<Vec<_>>(),"plot_rivers":sparse(&v.plot_rivers,|v|json!(v)),"floodplain_rivers":sparse(&v.floodplain_rivers,|v|json!(v))})
        }
        Component::Terrain(v) => {
            json!({"coastal_lowlands":v.coastal_lowlands.iter().map(|v|json!({"lowland_hash":v.lowland_type_hash,"plot":v.plot_index,"starting_terrain_hash":v.original_terrain_hash,"river_flow":[v.river_east_flow,v.river_south_east_flow,v.river_south_west_flow,v.river_west_flow,v.river_north_west_flow,v.river_north_east_flow],"river_crossing_count":v.river_crossing_count,"river_crossing_mask":v.river_crossing_mask,"river_edges":[u8::from(v.river_west),u8::from(v.river_north_west),u8::from(v.river_north_east),u8::from(v.river_east),u8::from(v.river_south_east),u8::from(v.river_south_west)],"flooded":u8::from(v.flooded),"submerged":u8::from(v.submerged),"protected":u8::from(v.protected)})).collect::<Vec<_>>(),"deserts":v.deserts.iter().map(|v|json!({"type_hash":v.type_hash,"discovering_civ_hash":v.discovering_civilization_hash,"territory":v.territory_id,"custom_name":hex(&v.custom_name)})).collect::<Vec<_>>(),"mountain_ranges":v.mountain_ranges.iter().map(|v|json!({"type_hash":v.type_hash,"discovering_civ_hash":v.discovering_civilization_hash,"territory":v.territory_id,"custom_name":hex(&v.custom_name)})).collect::<Vec<_>>(),"named_lakes":v.named_lakes.iter().map(|v|json!({"type_hash":v.kind,"discovering_civ_hash":v.discovering_civilization_hash,"territory":v.territory_id,"custom_name":hex(&v.custom_name)})).collect::<Vec<_>>(),"named_oceans":v.named_oceans.iter().map(|v|json!({"type_hash":v.kind,"discovering_civ_hash":v.discovering_civilization_hash,"territory":v.territory_id,"custom_name":hex(&v.custom_name)})).collect::<Vec<_>>(),"named_seas":v.named_seas.iter().map(|v|json!({"type_hash":v.kind,"discovering_civ_hash":v.discovering_civilization_hash,"territory":v.territory_id,"custom_name":hex(&v.custom_name)})).collect::<Vec<_>>()})
        }
        Component::Territories {
            container,
            plot_territory_ids,
        } => {
            json!({"territories":recycled(container,territory),"plot_territories":plot_territory_ids})
        }
        Component::TerritoryBuilder => Value::Null,
    };
    json!({"name":std::str::from_utf8(c.name()).unwrap(),"value":value})
}
fn diagnostic(m: &Map) -> Value {
    let h = &m.header;
    json!({"header":{"width":h.width,"height":h.height,"top_latitude":h.top_latitude,"bottom_latitude":h.bottom_latitude,"wrap_x":u8::from(h.wrap_x),"wrap_y":u8::from(h.wrap_y),"size_hash":h.map_size_hash,"land_plots":h.land_plots,"owned_plots":h.owned_plots,"natural_wonders":h.natural_wonders,"resources":hashes(&h.resource_counts),"land_resources":hashes(&h.land_resource_counts)},"visibility":m.visibility.iter().map(|v|json!([v.player,visibility(&v.visibility)])).collect::<Vec<_>>(),"areas":recycled(&m.areas,area),"landmasses":recycled(&m.landmasses,|v|json!({"id":v.id,"count":v.plot_count,"water":u8::from(v.water),"centroid_delta":v.centroid_delta})),"regions":{"regions":recycled(&m.regions,|v|json!({"id":v.id,"count":v.plot_count,"chokepoint":u8::from(v.chokepoint),"area":v.area_id,"joined":v.joined_region_ids})),"chokepoints":recycled(&m.chokepoints,|v|json!({"id":v.id,"initialized":u8::from(v.initialized),"endpoints":v.endpoints,"points":v.points,"joined_areas":v.joined_area_ids}))},"continents":m.continents.iter().map(|v|json!({"id":v.id,"type_hash":v.type_hash,"count":v.plot_count,"fertility":v.fertility})).collect::<Vec<_>>(),"features":recycled(&m.feature_layouts,|v|json!({"lowest_plot":v.lowest_plot,"plots":v.plot_indices})),"plots":m.plots.iter().map(plot).collect::<Vec<_>>(),"units":{"width":m.units.width,"height":m.units.height,"layers":m.units.layers.iter().map(|v|json!({"id":v.layer_id,"maximum":v.maximum,"cells":v.cells.iter().map(|c|json!([c.plot_index,c.units.iter().map(|u|(u.owner,u.id)).collect::<Vec<_>>()])).collect::<Vec<_>>()})).collect::<Vec<_>>()},"components":m.components.iter().map(component).collect::<Vec<_>>()})
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 6 {
        return Err(
            "usage: c6-map-codec-candidate INPUT OUTPUT DIAGNOSTIC_JSON RANGES_JSON ITERATIONS"
                .into(),
        );
    }
    let bytes = fs::read(&args[1])?;
    let (m, ranges) = Map::decode_with_ranges(&bytes, Limits::default())?;
    let output = m.encode(Limits::default())?;
    assert_eq!(output, bytes);
    fs::write(&args[2], output)?;
    serde_json::to_writer(BufWriter::new(fs::File::create(&args[3])?), &diagnostic(&m))?;
    serde_json::to_writer(
        BufWriter::new(fs::File::create(&args[4])?),
        &ranges
            .iter()
            .map(|r| json!({"name":r.name,"start":r.start,"end":r.end}))
            .collect::<Vec<_>>(),
    )?;
    for index in 0..m.plots.len() {
        let coords = m.coordinates(index as u32).unwrap();
        assert!(std::ptr::eq(
            m.plot_at(coords.0, coords.1).unwrap(),
            m.plot_by_index(index as u32).unwrap()
        ));
    }
    assert!(m.plot_at(-1, 0).is_none());
    assert!(m.plot_at(m.header.width, 0).is_none());
    assert!(m.plot_by_index(u32::MAX).is_none());
    assert!(Map::decode(
        &bytes,
        Limits {
            max_bytes: bytes.len() - 1,
            ..Limits::default()
        }
    )
    .is_err());
    assert!(Map::decode(
        &bytes,
        Limits {
            max_items: 0,
            ..Limits::default()
        }
    )
    .is_err());
    assert!(Map::decode(
        &bytes,
        Limits {
            max_allocation: 0,
            ..Limits::default()
        }
    )
    .is_err());
    assert!(Map::decode(
        &bytes,
        Limits {
            max_depth: 0,
            ..Limits::default()
        }
    )
    .is_err());
    let tight = Limits {
        max_bytes: bytes.len(),
        ..Limits::default()
    };
    assert_eq!(Map::decode(&bytes, tight)?.encode(tight)?, bytes);
    let iterations: u32 = args[5].parse()?;
    if iterations == 0 || iterations > 10000 {
        return Err("iteration resource bound".into());
    }
    let time = Instant::now();
    for _ in 0..iterations {
        std::hint::black_box(Map::decode(
            std::hint::black_box(&bytes),
            Limits::default(),
        )?);
    }
    let decode_ns = time.elapsed().as_nanos();
    let time = Instant::now();
    for _ in 0..iterations {
        std::hint::black_box(m.encode(Limits::default())?);
    }
    let encode_ns = time.elapsed().as_nanos();
    let keys: Vec<_> = (0..65536usize)
        .map(|n| {
            let i = n.wrapping_mul(104729) % m.plots.len();
            let (x, y) = m.coordinates(i as u32).unwrap();
            (x, y, i as u32)
        })
        .collect();
    let time = Instant::now();
    for _ in 0..iterations {
        for &(x, y, i) in &keys {
            std::hint::black_box(m.plot_at(std::hint::black_box(x), std::hint::black_box(y)));
            std::hint::black_box(m.plot_by_index(std::hint::black_box(i)));
        }
    }
    let lookup_ns = time.elapsed().as_nanos();
    println!(
        "{}",
        json!({"bytes":bytes.len(),"plots":m.plots.len(),"width":m.header.width,"height":m.header.height,"components":m.components.len(),"iterations":iterations,"decode_ns":decode_ns,"encode_ns":encode_ns,"lookup_operations":u64::from(iterations)*keys.len()as u64*2,"lookup_ns":lookup_ns})
    );
    Ok(())
}
